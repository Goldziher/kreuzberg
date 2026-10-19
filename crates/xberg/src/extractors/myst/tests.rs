use super::*;
use crate::SecurityLimits;

fn test_budget() -> SecurityBudget {
    SecurityBudget::from_limits(&SecurityLimits::default())
}

#[test]
fn should_reject_generated_myst_content_beyond_growth_limit() {
    let limits = SecurityLimits {
        max_content_size: 8,
        ..Default::default()
    };
    let mut budget = SecurityBudget::from_limits(&limits);
    let error = preprocess_myst(":::{note}\nexpanded\n:::", &mut budget)
        .expect_err("rendered admonition must respect the caller's growth limit");
    assert!(matches!(error, crate::XbergError::Security { .. }));
}

#[test]
fn should_preprocess_supported_myst_constructs() {
    let content = ":::{warning}\nTake care.\n:::\n\n```{math}\n:label: eq-one\nx = 1\n```\nSee {ref}`eq-one`.";
    let rendered = preprocess_myst(content, &mut test_budget()).expect("supported MyST should preprocess");
    assert_eq!(
        ["Warning", "Take care.", "x = 1", "eq-one"].map(|expected| rendered.contains(expected)),
        [true; 4]
    );
    assert_eq!(
        [":::", ":label:", "```{math}", "{ref}"].map(|syntax| rendered.contains(syntax)),
        [false; 4]
    );
}

#[test]
fn should_require_strong_myst_notebook_detection() {
    let ordinary = "# Report\n\n```{code-cell} python\nprint(1)\n```";
    assert_eq!(
        parse_myst_text_notebook(ordinary, &mut test_budget()).expect("detection should succeed"),
        None
    );
}

#[test]
fn should_parse_myst_notebook_cells_and_metadata() {
    let content = "---\ntitle: Report\nkernelspec:\n  language: python\n---\n# Analysis\n\n```{code-cell} python\n:tags: [parameters]\nx = 1\n```";
    let notebook = parse_myst_text_notebook(content, &mut test_budget())
        .expect("parsing should succeed")
        .expect("frontmatter and code-cell identify a MyST notebook");
    assert_eq!(notebook.metadata["title"], "Report");
    assert_eq!(notebook.cells.len(), 2);
    assert_eq!(notebook.cells[0].cell_type, TextNotebookCellType::Markdown);
    assert_eq!(notebook.cells[1].cell_type, TextNotebookCellType::Code);
    assert_eq!(notebook.cells[1].tags, ["parameters"]);
}

#[test]
#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn should_parse_percent_cells_without_header() {
    let content = "# %% [markdown]\n# # Report\n# %% tags=[parameters]\nx = 1\n";
    let notebook = parse_text_notebook(content, &mut test_budget())
        .expect("parsing should succeed")
        .expect("percent markers identify a notebook");
    assert_eq!(notebook.cells.len(), 2);
    assert_eq!(notebook.cells[0].source, "# Report");
    assert_eq!(notebook.cells[1].tags, ["parameters"]);
}

#[test]
#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn should_not_treat_marker_free_source_as_notebook() {
    assert_eq!(
        parse_text_notebook("# ordinary source\nx = 1\n", &mut test_budget()).expect("detection should succeed"),
        None
    );
}

#[test]
#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn should_require_light_header() {
    let content = "# + [markdown]\n# Ordinary comment\n# -\nx = 1\n";
    assert_eq!(
        parse_text_notebook(content, &mut test_budget()).expect("detection should succeed"),
        None
    );
}

#[test]
#[cfg(all(feature = "notebook", feature = "tree-sitter"))]
fn should_parse_light_cells_with_header() {
    let content = "# ---\n# jupyter:\n#   jupytext:\n#     text_representation:\n#       format_name: light\n# kernelspec:\n#   language: python\n# ---\n# + [markdown]\n# # Report\n# -\n# +\nx = 1\n# -\n";
    let notebook = parse_text_notebook(content, &mut test_budget())
        .expect("parsing should succeed")
        .expect("light header identifies a notebook");
    assert_eq!(notebook.cells.len(), 2);
    assert_eq!(notebook.cells[0].source, "# Report");
    assert_eq!(notebook.cells[1].source, "x = 1");
    assert_eq!(notebook.metadata["kernelspec"]["language"], "python");
}

#[test]
fn should_bound_deeply_nested_directive_preprocessing() {
    let deepest = MAX_MYST_DIRECTIVE_DEPTH + 16;
    let mut content = String::new();
    for fence_length in (3..deepest + 3).rev() {
        content.push_str(&format!("{}{{note}}\n", ":".repeat(fence_length)));
    }
    content.push_str("bounded body\n");
    for fence_length in 3..deepest + 3 {
        content.push_str(&format!("{}\n", ":".repeat(fence_length)));
    }
    let error = preprocess_myst(&content, &mut test_budget())
        .expect_err("directives beyond the named recursion cap must be rejected");
    assert!(matches!(error, crate::XbergError::Security { .. }));
}

#[test]
fn should_allow_exact_named_directive_depth_limit() {
    let deepest = MAX_MYST_DIRECTIVE_DEPTH;
    let mut content = String::new();
    for fence_length in (3..deepest + 3).rev() {
        content.push_str(&format!("{}{{note}}\n", ":".repeat(fence_length)));
    }
    content.push_str("boundary body\n");
    for fence_length in 3..deepest + 3 {
        content.push_str(&format!("{}\n", ":".repeat(fence_length)));
    }
    let rendered =
        preprocess_myst(&content, &mut test_budget()).expect("the named depth cap itself should remain valid");
    assert!(rendered.contains("boundary body"));
}

#[test]
fn should_reject_unsafe_target_marker_values() {
    let mut rendered = String::new();
    render_target_marker("unsafe--><script>", &mut rendered, &mut test_budget())
        .expect("invalid target should be ignored safely");
    assert_eq!(rendered, "");
    assert_eq!(myst_target_marker("<!--xberg-myst-target:unsafe--><script>-->"), None);
}

#[test]
fn should_place_table_target_after_caption() {
    let content = "```{table} Results\n:name: table-results\n| A |\n| - |\n| 1 |\n```";
    let rendered = preprocess_myst(content, &mut test_budget()).expect("table should preprocess");
    let caption = rendered.find("Results").expect("table caption");
    let target = rendered.find(TARGET_METADATA_PREFIX).expect("table target");
    let table = rendered.find("| A |").expect("table body");
    assert!(caption < target && target < table);
}

#[test]
fn should_preserve_named_code_and_admonition_targets() {
    let content = ":::{note}\n:name: guidance\nBody.\n:::\n\n```{code-block} rust\n:label: example\nfn main() {}\n```";
    let rendered = preprocess_myst(content, &mut test_budget()).expect("named directives should preprocess");
    let guidance = rendered
        .find("<!--xberg-myst-target:guidance-->")
        .expect("admonition target marker");
    let admonition = rendered.find("> [!NOTE]").expect("rendered admonition");
    let example = rendered
        .find("<!--xberg-myst-target:example-->")
        .expect("code target marker");
    let code = rendered.find("```rust").expect("rendered code block");
    assert!(guidance < admonition && example < code);
}
