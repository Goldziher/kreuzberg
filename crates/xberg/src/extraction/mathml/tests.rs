use super::*;

/// Helper: parse a MathML XML fragment and return rendered LaTeX.
fn mathml_to_latex(inner: &str) -> String {
    let xml = format!(r#"<math xmlns="http://www.w3.org/1998/Math/MathML">{}</math>"#, inner);
    let mut budget = SecurityBudget::with_defaults();
    convert_mathml_str_to_latex(&xml, &mut budget).expect("conversion ok")
}

/// OpenOffice writes its formula objects with a prefixed namespace and its
/// own DTD, which a real ODF document embeds verbatim.
/// OpenOffice writes its stretchy fences as private use codepoints, which
/// no renderer can display. One real document carried them into its LaTeX.
#[test]
fn test_private_use_characters_are_dropped() {
    let xml = "<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mrow>\
<mi>F</mi><mo>\u{E09E}</mo><mn>1</mn><mo>\u{E09F}</mo>\
<mfenced open=\"\u{E09E}\" close=\"\u{E09F}\"><mn>2</mn></mfenced></mrow></math>";

    let mut budget = SecurityBudget::with_defaults();
    let latex = convert_mathml_str_to_latex(xml, &mut budget).expect("converts");

    assert!(
        !latex.chars().any(is_private_use),
        "no private use character survives: {latex:?}"
    );
    assert!(
        latex.contains('F') && latex.contains('1'),
        "the real content stays: {latex}"
    );
}

#[test]
fn test_prefixed_mathml_with_the_openoffice_doctype() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE math:math PUBLIC "-//OpenOffice.org//DTD Modified W3C MathML 1.01//EN" "math.dtd">
<math:math xmlns:math="http://www.w3.org/1998/Math/MathML">
 <math:semantics>
  <math:mrow>
   <math:mn>1</math:mn>
   <math:mo math:stretchy="false">+</math:mo>
   <math:mn>2</math:mn>
  </math:mrow>
 </math:semantics>
</math:math>"#;

    let mut budget = SecurityBudget::with_defaults();
    let latex = convert_mathml_str_to_latex(xml, &mut budget).expect("converts");
    assert!(
        !latex.trim().is_empty(),
        "a formula with a DOCTYPE must convert, got {latex:?}"
    );
    assert!(latex.contains('1') && latex.contains('2'), "operands survive: {latex}");
}

#[test]
fn test_mi_plain_text() {
    assert_eq!(mathml_to_latex("<mi>x</mi>"), "x");
}

#[test]
fn test_mn_number() {
    assert_eq!(mathml_to_latex("<mn>42</mn>"), "42");
}

#[test]
fn test_mo_unicode_operator() {
    assert_eq!(mathml_to_latex("<mo>\u{00D7}</mo>"), "\\times ");
}

#[test]
fn test_numeric_char_ref_with_trailing_comment_is_not_duplicated() {
    // Real-world MathML (e.g. EPUB accessibility test suites) commonly
    // annotates a numeric character reference with a same-content XML
    // comment: `<mo>&#x222B;<!-- ∫ --></mo>`. The comment must not be
    // rendered a second time alongside the decoded entity. ~keep
    assert_eq!(mathml_to_latex("<mo>&#x222B;<!-- \u{222B} --></mo>"), "\\int ");
    assert_eq!(
        mathml_to_latex("<mi mathvariant=\"normal\">&#x221E;<!-- \u{221E} --></mi>"),
        "\\infty "
    );
}

#[test]
fn test_mtext_wraps_in_text_command() {
    assert_eq!(mathml_to_latex("<mtext>hello world</mtext>"), "\\text{hello world}");
}

#[test]
fn test_ms_string_literal() {
    assert_eq!(mathml_to_latex("<ms>abc</ms>"), "abc");
}

#[test]
fn test_mrow_concatenates_children() {
    assert_eq!(mathml_to_latex("<mrow><mi>x</mi><mo>+</mo><mi>y</mi></mrow>"), "x+y");
}

#[test]
fn test_mfrac() {
    assert_eq!(mathml_to_latex("<mfrac><mn>1</mn><mn>2</mn></mfrac>"), "\\frac{1}{2}");
}

#[test]
fn test_msup() {
    assert_eq!(mathml_to_latex("<msup><mi>x</mi><mn>2</mn></msup>"), "x^{2}");
}

#[test]
fn test_msub() {
    assert_eq!(mathml_to_latex("<msub><mi>a</mi><mi>n</mi></msub>"), "a_{n}");
}

#[test]
fn test_msubsup() {
    assert_eq!(
        mathml_to_latex("<msubsup><mi>x</mi><mi>i</mi><mn>2</mn></msubsup>"),
        "x_{i}^{2}"
    );
}

#[test]
fn test_msqrt_no_degree() {
    assert_eq!(mathml_to_latex("<msqrt><mi>x</mi></msqrt>"), "\\sqrt{x}");
}

#[test]
fn test_mroot_with_degree() {
    assert_eq!(mathml_to_latex("<mroot><mi>x</mi><mn>3</mn></mroot>"), "\\sqrt[3]{x}");
}

#[test]
fn test_mfenced_default_parens() {
    assert_eq!(mathml_to_latex("<mfenced><mi>x</mi></mfenced>"), "\\left(x\\right)");
}

#[test]
fn test_mfenced_brackets_multiple_elements() {
    assert_eq!(
        mathml_to_latex(r#"<mfenced open="[" close="]"><mi>a</mi><mi>b</mi></mfenced>"#),
        "\\left[a,b\\right]"
    );
}

#[test]
fn test_munder() {
    assert_eq!(
        mathml_to_latex("<munder><mi>x</mi><mi>n</mi></munder>"),
        "\\underset{n}{x}"
    );
}

#[test]
fn test_munder_lim_takes_limits() {
    assert_eq!(mathml_to_latex("<munder><mi>lim</mi><mi>n</mi></munder>"), "\\lim_{n}");
}

#[test]
fn test_munder_sum_takes_limits() {
    assert_eq!(
        mathml_to_latex("<munder><mo>\u{2211}</mo><mrow><mi>i</mi><mo>=</mo><mn>1</mn></mrow></munder>"),
        "\\sum_{i=1}"
    );
}

#[test]
fn test_munder_named_operators_take_limits() {
    for name in ["max", "min", "sup", "inf", "det", "gcd", "Pr"] {
        assert_eq!(
            mathml_to_latex(&format!("<munder><mi>{name}</mi><mi>x</mi></munder>")),
            format!("\\{name}_{{x}}")
        );
    }
}

#[test]
fn test_munder_spaced_liminf_takes_limits() {
    assert_eq!(
        mathml_to_latex("<munder><mi>lim inf</mi><mi>n</mi></munder>"),
        "\\liminf_{n}"
    );
    assert_eq!(
        mathml_to_latex("<munder><mo>lim\u{2006}sup</mo><mi>n</mi></munder>"),
        "\\limsup_{n}"
    );
}

#[test]
fn test_munder_function_without_limits_keeps_underset() {
    assert_eq!(
        mathml_to_latex("<munder><mi>sin</mi><mi>x</mi></munder>"),
        "\\underset{x}{sin}"
    );
}

#[test]
fn test_munder_grouped_base_takes_limits() {
    assert_eq!(
        mathml_to_latex("<munder><mrow><mo>\u{2211}</mo></mrow><mi>i</mi></munder>"),
        "\\sum_{i}"
    );
    assert_eq!(
        mathml_to_latex("<munder><mstyle><mi>lim</mi></mstyle><mi>n</mi></munder>"),
        "\\lim_{n}"
    );
}

#[test]
fn test_munderover_integral_keeps_stacked_limits() {
    assert_eq!(
        mathml_to_latex("<munderover><mo>\u{222B}</mo><mi>a</mi><mi>b</mi></munderover>"),
        "\\int\\limits_{a}^{b}"
    );
}

#[test]
fn test_munder_contour_integral_keeps_stacked_limit() {
    assert_eq!(
        mathml_to_latex("<munder><mo>\u{222E}</mo><mi>C</mi></munder>"),
        "\\oint\\limits_{C}"
    );
}

#[test]
fn test_msubsup_integral_keeps_side_scripts() {
    assert_eq!(
        mathml_to_latex("<msubsup><mo>\u{222B}</mo><mi>a</mi><mi>b</mi></msubsup>"),
        "\\int _{a}^{b}"
    );
}

/// An accent over a limit-taking base is still an accent: `\overline{lim}` is the upper limit.
#[test]
fn test_mover_bar_on_lim_is_overline() {
    assert_eq!(
        mathml_to_latex("<mover><mi>lim</mi><mo>\u{00AF}</mo></mover>"),
        "\\overline{lim}"
    );
}

#[test]
fn test_munder_low_line_on_lim_is_underline() {
    assert_eq!(
        mathml_to_latex("<munder><mi>lim</mi><mo>_</mo></munder>"),
        "\\underline{lim}"
    );
}

#[test]
fn test_mover_hat_on_sum_is_accent() {
    assert_eq!(
        mathml_to_latex("<mover><mo>\u{2211}</mo><mo>^</mo></mover>"),
        "\\hat{\\sum }"
    );
}

#[test]
fn test_munderover_accent_on_sum_is_not_limits() {
    assert_eq!(
        mathml_to_latex("<munderover><mo>\u{2211}</mo><mi>i</mi><mo>^</mo></munderover>"),
        "\\hat{\\underset{i}{\\sum }}"
    );
}

#[test]
fn test_munderover_bar_over_stack_is_overline() {
    assert_eq!(
        mathml_to_latex("<munderover><mi>x</mi><mi>a</mi><mo>\u{00AF}</mo></munderover>"),
        "\\overline{\\underset{a}{x}}"
    );
}

#[test]
fn test_mover_hat_accent() {
    // A bare `^` inside `\overset` is unparseable LaTeX ("expected group
    // after ^"); accent characters must map to accent macros.
    assert_eq!(mathml_to_latex("<mover><mi>x</mi><mo>^</mo></mover>"), "\\hat{x}");
}

#[test]
fn test_mover_accent_family() {
    assert_eq!(
        mathml_to_latex("<mover><mi>x</mi><mo>\u{02DC}</mo></mover>"),
        "\\tilde{x}"
    );
    assert_eq!(
        mathml_to_latex("<mover><mi>q</mi><mo>\u{02D9}</mo></mover>"),
        "\\dot{q}"
    );
    assert_eq!(
        mathml_to_latex("<mover><mi>y</mi><mo>\u{00AF}</mo></mover>"),
        "\\bar{y}"
    );
    assert_eq!(
        mathml_to_latex("<mover><mi>v</mi><mo>\u{2192}</mo></mover>"),
        "\\vec{v}"
    );
    // Multi-glyph base widens to the stretched forms.
    assert_eq!(
        mathml_to_latex("<mover><mrow><mi>a</mi><mi>b</mi></mrow><mo>\u{00AF}</mo></mover>"),
        "\\overline{ab}"
    );
}

#[test]
fn test_munder_low_line_is_underline() {
    // Authors write lower bounds as `munder` with a low-line char.
    assert_eq!(
        mathml_to_latex("<munder><mi>m</mi><mo>_</mo></munder>"),
        "\\underline{m}"
    );
}

#[test]
fn test_mover_with_content_script_keeps_overset() {
    assert_eq!(
        mathml_to_latex("<mover><mi>x</mi><mi>n</mi></mover>"),
        "\\overset{n}{x}"
    );
}

/// An explicit `accent="false"` overrides the script-character heuristic, so the arrow is a limit.
#[test]
fn test_munderover_accent_false_keeps_limits() {
    assert_eq!(
        mathml_to_latex("<munderover accent=\"false\"><mo>\u{2211}</mo><mi>i</mi><mo>\u{2192}</mo></munderover>"),
        "\\sum_{i}^{\\rightarrow }"
    );
}

#[test]
fn test_munderover_accentunder_false_keeps_limits() {
    assert_eq!(
        mathml_to_latex("<munderover accentunder=\"false\"><mo>\u{2211}</mo><mo>_</mo><mi>n</mi></munderover>"),
        "\\sum_{\\_}^{n}"
    );
}

#[test]
fn test_mover_accent_false_keeps_overset() {
    assert_eq!(
        mathml_to_latex("<mover accent=\"false\"><mi>x</mi><mo>\u{2192}</mo></mover>"),
        "\\overset{\\rightarrow }{x}"
    );
}

#[test]
fn test_munder_accentunder_false_keeps_underset() {
    assert_eq!(
        mathml_to_latex("<munder accentunder=\"false\"><mi>x</mi><mo>_</mo></munder>"),
        "\\underset{\\_}{x}"
    );
}

/// `\overset{^}{x}` does not parse, so a caret that is not an accent is written as a hat over nothing.
#[test]
fn test_mover_accent_false_caret_is_a_visible_script() {
    assert_eq!(
        mathml_to_latex("<mover accent=\"false\"><mi>x</mi><mo>^</mo></mover>"),
        "\\overset{\\hat{}}{x}"
    );
}

/// Each accent-like script character: its accent over a base, its accent under one, and the mark written for it
/// as a plain script.
const SCRIPT_MARKS: &[(&str, Option<&str>, Option<&str>, &str)] = &[
    ("^", Some("\\hat"), None, "\\hat{}"),
    ("\u{02C6}", Some("\\hat"), None, "\\hat{}"),
    ("\u{0302}", Some("\\hat"), None, "\\hat{}"),
    ("~", Some("\\tilde"), None, "\\sim "),
    ("\u{02DC}", Some("\\tilde"), None, "\\tilde{}"),
    ("\u{0303}", Some("\\tilde"), None, "\\tilde{}"),
    ("\u{223C}", Some("\\tilde"), None, "\\sim "),
    ("\u{02D9}", Some("\\dot"), None, "\\dot{}"),
    ("\u{0307}", Some("\\dot"), None, "\\dot{}"),
    ("\u{00A8}", Some("\\ddot"), None, "\\ddot{}"),
    ("\u{0308}", Some("\\ddot"), None, "\\ddot{}"),
    ("\u{00AF}", Some("\\bar"), Some("\\underline"), "\\bar{}"),
    ("\u{203E}", Some("\\bar"), Some("\\underline"), "\\bar{}"),
    ("\u{0304}", Some("\\bar"), None, "\\bar{}"),
    ("\u{0305}", Some("\\bar"), None, "\\bar{}"),
    ("\u{2192}", Some("\\vec"), None, "\\rightarrow "),
    ("\u{20D7}", Some("\\vec"), None, "\\vec{}"),
    ("\u{02C7}", Some("\\check"), None, "\\check{}"),
    ("\u{030C}", Some("\\check"), None, "\\check{}"),
    ("\u{02D8}", Some("\\breve"), None, "\\breve{}"),
    ("\u{0306}", Some("\\breve"), None, "\\breve{}"),
    ("\u{00B4}", Some("\\acute"), None, "\\acute{}"),
    ("\u{0301}", Some("\\acute"), None, "\\acute{}"),
    ("`", Some("\\grave"), None, "\\grave{}"),
    ("\u{0300}", Some("\\grave"), None, "\\grave{}"),
    ("\u{02DA}", Some("\\mathring"), None, "\\mathring{}"),
    ("\u{030A}", Some("\\mathring"), None, "\\mathring{}"),
    ("\u{23DE}", Some("\\overbrace"), None, "\\overbrace{\\qquad}"),
    ("_", None, Some("\\underline"), "\\_"),
    ("\u{0332}", None, Some("\\underline"), "\\_"),
    ("\u{02CD}", None, Some("\\underline"), "\\_"),
    ("\u{23DF}", None, Some("\\underbrace"), "\\underbrace{\\qquad}"),
];

const SUM: &str = "<mo>\u{2211}</mo>";

/// Convert `xml`, check it against `expected`, and check it is LaTeX a renderer accepts: ASCII, balanced
/// braces, every script a group, and no script left empty.
fn assert_script_case(xml: &str, expected: &str) {
    let latex = mathml_to_latex(xml);
    assert_eq!(latex, expected, "{xml}");
    assert!(latex.is_ascii(), "{xml} -> {latex}");
    let mut depth = 0i32;
    let mut chars = latex.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                chars.next();
            }
            '{' => depth += 1,
            '}' => depth -= 1,
            '^' | '_' => assert_eq!(chars.peek(), Some(&'{'), "{xml} -> {latex}"),
            '~' => panic!("{xml} -> {latex}"),
            _ => {}
        }
        assert!(depth >= 0, "{xml} -> {latex}");
    }
    assert_eq!(depth, 0, "{xml} -> {latex}");
    for empty in ["^{}", "_{}", "set{}"] {
        assert!(!latex.contains(empty), "{xml} -> {latex}");
    }
}

/// The wide form of an over accent, taken over more than one glyph.
fn wide(cmd: &str) -> &str {
    match cmd {
        "\\bar" => "\\overline",
        "\\vec" => "\\overrightarrow",
        cmd => cmd,
    }
}

/// The element's own `accent` decides, else the script's `mo`, else the character: an accent where one is
/// known, and its visible mark wherever the script is not one.
#[test]
fn test_mover_accent_resolution_for_every_script_mark() {
    for &(ch, over, _, mark) in SCRIPT_MARKS {
        let accent_x = over.map_or(format!("\\overset{{{mark}}}{{x}}"), |c| format!("{c}{{x}}"));
        let accent_sum = over.map_or(format!("\\sum^{{{mark}}}"), |c| format!("{c}{{\\sum }}"));
        let plain_x = format!("\\overset{{{mark}}}{{x}}");
        let plain_sum = format!("\\sum^{{{mark}}}");
        for (parent, script, base, expected) in [
            ("", "", "<mi>x</mi>", &accent_x),
            ("", "", SUM, &accent_sum),
            (" accent=\"true\"", "", "<mi>x</mi>", &accent_x),
            ("", " accent=\"true\"", SUM, &accent_sum),
            (" accent=\"false\"", "", "<mi>x</mi>", &plain_x),
            (" accent=\"false\"", " accent=\"true\"", SUM, &plain_sum),
            ("", " accent=\"false\"", "<mi>x</mi>", &plain_x),
            ("", " accent=\"false\"", SUM, &plain_sum),
            (" accent=\"true\"", " accent=\"false\"", SUM, &accent_sum),
        ] {
            let xml = format!("<mover{parent}>{base}<mo{script}>{ch}</mo></mover>");
            assert_script_case(&xml, expected);
        }
    }
}

#[test]
fn test_munder_accent_resolution_for_every_script_mark() {
    for &(ch, _, under, mark) in SCRIPT_MARKS {
        let accent_x = under.map_or(format!("\\underset{{{mark}}}{{x}}"), |c| format!("{c}{{x}}"));
        let accent_sum = under.map_or(format!("\\sum_{{{mark}}}"), |c| format!("{c}{{\\sum }}"));
        let plain_x = format!("\\underset{{{mark}}}{{x}}");
        let plain_sum = format!("\\sum_{{{mark}}}");
        for (parent, script, base, expected) in [
            ("", "", "<mi>x</mi>", &accent_x),
            ("", "", SUM, &accent_sum),
            (" accentunder=\"true\"", "", "<mi>x</mi>", &accent_x),
            ("", " accent=\"true\"", SUM, &accent_sum),
            (" accentunder=\"false\"", "", SUM, &plain_sum),
            (" accentunder=\"false\"", " accent=\"true\"", "<mi>x</mi>", &plain_x),
            ("", " accent=\"false\"", "<mi>x</mi>", &plain_x),
            ("", " accent=\"false\"", SUM, &plain_sum),
            (" accentunder=\"true\"", " accent=\"false\"", SUM, &accent_sum),
        ] {
            let xml = format!("<munder{parent}>{base}<mo{script}>{ch}</mo></munder>");
            assert_script_case(&xml, expected);
        }
    }
}

#[test]
fn test_munderover_accent_resolution_for_every_script_mark() {
    for &(ch, over, under, mark) in SCRIPT_MARKS {
        let over_accent_x = over.map_or(format!("\\overset{{{mark}}}{{\\underset{{i}}{{x}}}}"), |c| {
            format!("{}{{\\underset{{i}}{{x}}}}", wide(c))
        });
        let over_accent_sum = over.map_or(format!("\\sum_{{i}}^{{{mark}}}"), |c| {
            format!("{}{{\\underset{{i}}{{\\sum }}}}", wide(c))
        });
        let over_plain_x = format!("\\overset{{{mark}}}{{\\underset{{i}}{{x}}}}");
        let over_plain_sum = format!("\\sum_{{i}}^{{{mark}}}");
        for (parent, script, base, expected) in [
            ("", "", SUM, &over_accent_sum),
            (" accent=\"true\"", " accent=\"false\"", "<mi>x</mi>", &over_accent_x),
            (" accent=\"false\"", "", SUM, &over_plain_sum),
            (" accent=\"false\"", " accent=\"true\"", "<mi>x</mi>", &over_plain_x),
            ("", " accent=\"false\"", SUM, &over_plain_sum),
            ("", " accent=\"false\"", "<mi>x</mi>", &over_plain_x),
        ] {
            let xml = format!("<munderover{parent}>{base}<mi>i</mi><mo{script}>{ch}</mo></munderover>");
            assert_script_case(&xml, expected);
        }

        let under_accent_x = under.map_or(format!("\\overset{{n}}{{\\underset{{{mark}}}{{x}}}}"), |c| {
            format!("\\overset{{n}}{{{c}{{x}}}}")
        });
        let under_accent_sum = under.map_or(format!("\\sum_{{{mark}}}^{{n}}"), |c| {
            format!("\\overset{{n}}{{{c}{{\\sum }}}}")
        });
        let under_plain_x = format!("\\overset{{n}}{{\\underset{{{mark}}}{{x}}}}");
        let under_plain_sum = format!("\\sum_{{{mark}}}^{{n}}");
        for (parent, script, base, expected) in [
            ("", "", SUM, &under_accent_sum),
            (
                " accentunder=\"true\"",
                " accent=\"false\"",
                "<mi>x</mi>",
                &under_accent_x,
            ),
            (" accentunder=\"false\"", "", SUM, &under_plain_sum),
            (
                " accentunder=\"false\"",
                " accent=\"true\"",
                "<mi>x</mi>",
                &under_plain_x,
            ),
            ("", " accent=\"false\"", SUM, &under_plain_sum),
            ("", " accent=\"false\"", "<mi>x</mi>", &under_plain_x),
        ] {
            let xml = format!("<munderover{parent}>{base}<mo{script}>{ch}</mo><mi>n</mi></munderover>");
            assert_script_case(&xml, expected);
        }
    }
}

/// An `mo` script's own `accent="false"` makes it a limit, as the element's attribute does.
#[test]
fn test_munderover_script_accent_false_keeps_limits() {
    assert_script_case(
        "<munderover><mo>&#x2211;</mo><mi>i</mi><mo accent=\"false\">&#x2192;</mo></munderover>",
        "\\sum_{i}^{\\rightarrow }",
    );
}

/// The script's `accent` is read off its core `mo`, through grouping.
#[test]
fn test_script_accent_is_read_through_grouping() {
    assert_script_case(
        "<mover><mi>x</mi><mrow><mo accent=\"false\">^</mo></mrow></mover>",
        "\\overset{\\hat{}}{x}",
    );
    assert_script_case(
        "<munder><mo>&#x2211;</mo><semantics><mo accent=\"false\">_</mo><annotation>x</annotation></semantics></munder>",
        "\\sum_{\\_}",
    );
}

/// A `^` or `~` from element text sets a superscript or a space in TeX, so every path writes its glyph instead.
#[test]
fn test_caret_and_tilde_text_is_escaped_on_every_path() {
    for (xml, expected) in [
        ("<msup><mi>x</mi><mo>^</mo></msup>", "x^{\\hat{}}"),
        ("<msub><mi>f</mi><mo>~</mo></msub>", "f_{\\sim }"),
        (
            "<mmultiscripts><mi>x</mi><mo>^</mo><none/></mmultiscripts>",
            "x_{\\hat{}}",
        ),
        (
            "<mover><mi>x</mi><mrow><mo>^</mo><mspace/></mrow></mover>",
            "\\overset{\\hat{} }{x}",
        ),
        ("<mover><mi>x</mi><mo>^^</mo></mover>", "\\overset{\\hat{}\\hat{}}{x}"),
        ("<mi>a</mi><mo>~</mo><mi>b</mi>", "a\\sim b"),
        ("<mtext>a~b^c</mtext>", "\\text{a\\textasciitilde b\\textasciicircum c}"),
    ] {
        assert_script_case(xml, expected);
    }
}

/// The collector unwraps a `semantics` with one presentation child, so the base's `movablelimits` counts there too.
#[test]
fn test_semantics_base_keeps_unmovable_limits() {
    assert_eq!(
        mathml_to_latex("<munder><semantics><mo movablelimits=\"false\">&#x2211;</mo></semantics><mi>i</mi></munder>"),
        "\\sum\\limits_{i}"
    );
    assert_eq!(
        mathml_to_latex(
            "<munder><semantics><mo movablelimits=\"false\">&#x2211;</mo>\
             <annotation encoding=\"StarMath 5.0\">sum</annotation></semantics><mi>i</mi></munder>"
        ),
        "\\sum\\limits_{i}"
    );
}

/// Each level's accent choice reads its base's rendering; rendering the base again for it cost 2^depth.
#[test]
fn test_deeply_nested_accents_render_in_linear_time() {
    let depth = 40;
    let mut xml = "<mi>x</mi>".to_string();
    for _ in 0..depth {
        xml = format!("<mover>{xml}<mo>\u{2192}</mo></mover>");
    }
    let expected = format!(
        "{}\\vec{{x}}{}",
        "\\overrightarrow{".repeat(depth - 1),
        "}".repeat(depth - 1)
    );
    assert_eq!(mathml_to_latex(&xml), expected);
}

#[test]
fn test_literal_stretchy_brace_is_escaped() {
    // A `<mo>{</mo>` cases brace passed through raw changes LaTeX grouping
    // structure and leaves the formula unbalanced when its mate sits in
    // another table row.
    assert_eq!(mathml_to_latex("<mo>{</mo><mi>x</mi>"), "\\{x");
}

#[test]
fn test_literal_backslash_is_escaped() {
    // Set difference written as a raw backslash: `A\B` must not fuse into
    // an undefined control sequence `\B`.
    assert_eq!(mathml_to_latex("<mi>A</mi><mo>\\</mo><mi>B</mi>"), "A\\backslash B");
}

#[test]
fn test_mfenced_norm_delimiters() {
    assert_eq!(
        mathml_to_latex(r#"<mfenced open="&#x2225;" close="&#x2225;"><mi>x</mi></mfenced>"#),
        "\\left\\|x\\right\\|"
    );
}

#[test]
fn test_mfenced_angle_delimiters_do_not_glue() {
    assert_eq!(
        mathml_to_latex(r#"<mfenced open="&#x27E8;" close="&#x27E9;"><mi>A</mi></mfenced>"#),
        "\\left\\langle A\\right\\rangle "
    );
}

#[test]
fn test_mfenced_with_operator_children_drops_separators() {
    // `mfenced` abused as grouping: `(1 - x)` must not become `(1,-,x)`.
    assert_eq!(
        mathml_to_latex(r#"<mfenced><mn>1</mn><mo>-</mo><mi>x</mi></mfenced>"#),
        "\\left(1-x\\right)"
    );
}

#[test]
fn test_mtext_greek_moves_outside_text_group() {
    // `\Delta` is math-mode-only; inside `\text{}` it is undefined.
    assert_eq!(
        mathml_to_latex("<mtext>rate \u{0394}x</mtext>"),
        "\\text{rate }\\Delta \\text{x}"
    );
}

#[test]
fn test_mtext_escapes_structural_chars() {
    assert_eq!(mathml_to_latex("<mtext>m_{0} 50%</mtext>"), "\\text{m\\_\\{0\\} 50\\%}");
}

#[test]
fn test_braced_base_with_script_still_wraps() {
    // `{S_{\sigma }}_{1}` starts and ends with braces but is two atoms;
    // scripting it again without a wrap is a double subscript.
    assert_eq!(
        mathml_to_latex("<msub><msub><mrow><mi>S</mi><mi>b</mi></mrow><mn>1</mn></msub><mn>2</mn></msub>"),
        "{{Sb}_{1}}_{2}"
    );
}

#[test]
fn test_scripted_base_wraps_before_outer_script() {
    // `\lambda _{1}^{'}` scripted again must brace-wrap, or the outer
    // script produces a double superscript.
    assert_eq!(
        mathml_to_latex("<msup><msup><mi>\u{03BB}</mi><mn>1</mn></msup><mn>2</mn></msup>"),
        "{\\lambda ^{1}}^{2}"
    );
}

#[test]
fn test_empty_script_base_renders_as_empty_group() {
    // Tensor prescript markup: an empty base must yield `{}` so the script
    // cannot fuse onto the preceding atom as a double subscript.
    assert_eq!(
        mathml_to_latex("<msup><mi>T</mi><mi>\u{03BD}</mi></msup><msub><mrow/><mi>\u{03BD}</mi></msub>"),
        "T^{\\nu }{}_{\\nu }"
    );
}

#[test]
fn test_combining_overline_folds_into_bar() {
    // Identifiers carry combining marks (`U̅`); the raw mark is not a
    // KaTeX-valid accent.
    assert_eq!(mathml_to_latex("<mi>U\u{0305}</mi>"), "\\bar{U}");
    // A mark split into its own element applies to the previous atom.
    assert_eq!(mathml_to_latex("<mi>\u{03A3}</mi><mo>\u{0305}</mo>"), "\\bar{\\Sigma} ");
}

/// Tensor indices alternate between positions, each pair after the last.
#[test]
fn test_mmultiscripts_chains_postscript_pairs() {
    assert_eq!(
        mathml_to_latex("<mmultiscripts><mi>R</mi><mi>i</mi><none/><none/><mi>j</mi></mmultiscripts>"),
        "R_{i}{}^{j}"
    );
}

#[test]
fn test_mmultiscripts_keeps_prescript_order() {
    assert_eq!(
        mathml_to_latex(
            "<mmultiscripts><mi>X</mi><mprescripts/><mi>a</mi><mi>b</mi><mi>c</mi><mi>d</mi></mmultiscripts>"
        ),
        "{}_{a}^{b}{}_{c}^{d}X"
    );
}

#[test]
fn test_mmultiscripts_with_prescripts_and_postscripts() {
    assert_eq!(
        mathml_to_latex(
            "<mmultiscripts><mi>C</mi><mi>a</mi><mi>b</mi><mprescripts/><mn>6</mn><mn>14</mn></mmultiscripts>"
        ),
        "{}_{6}^{14}C_{a}^{b}"
    );
}

#[test]
fn test_mmultiscripts_none_in_prescripts() {
    assert_eq!(
        mathml_to_latex("<mmultiscripts><mi>C</mi><mprescripts/><none/><mn>14</mn></mmultiscripts>"),
        "{}^{14}C"
    );
}

/// A trailing script with no partner is a subscript.
#[test]
fn test_mmultiscripts_odd_script_count() {
    assert_eq!(
        mathml_to_latex("<mmultiscripts><mi>R</mi><mi>i</mi><mi>j</mi><mi>k</mi></mmultiscripts>"),
        "R_{i}^{j}{}_{k}"
    );
}

#[test]
fn test_mmultiscripts_empty_base() {
    assert_eq!(
        mathml_to_latex("<mmultiscripts><none/><mi>i</mi><none/></mmultiscripts>"),
        "{}_{i}"
    );
    assert_eq!(mathml_to_latex("<mmultiscripts></mmultiscripts>"), "");
}

#[test]
fn test_mmultiscripts_blank_script_is_absent() {
    assert_eq!(
        mathml_to_latex("<mmultiscripts><mi>R</mi><mi>i</mi><mrow/></mmultiscripts>"),
        "R_{i}"
    );
    assert_eq!(
        mathml_to_latex("<mmultiscripts><mi>R</mi><mi></mi><mi>j</mi></mmultiscripts>"),
        "R^{j}"
    );
    assert_eq!(
        mathml_to_latex("<mmultiscripts><mi>R</mi><mi> </mi><mi>j</mi></mmultiscripts>"),
        "R^{j}"
    );
    assert_eq!(
        mathml_to_latex("<mmultiscripts><mi>R</mi><mi>i</mi><mrow><mrow/></mrow></mmultiscripts>"),
        "R_{i}"
    );
    assert_eq!(
        mathml_to_latex("<mmultiscripts><mi>R</mi><mi>i</mi><mtext></mtext></mmultiscripts>"),
        "R_{i}"
    );
}

/// `none` and `mprescripts` are never collected, so the loop itself must charge them.
#[test]
fn test_mmultiscripts_empty_slots_charge_iteration_budget() {
    let limits = crate::extractors::security::SecurityLimits {
        max_iterations: 10,
        ..Default::default()
    };
    let mut budget = SecurityBudget::from_limits(&limits);
    let xml = format!(
        "<math><mmultiscripts><mi>R</mi>{}</mmultiscripts></math>",
        "<none/>".repeat(20)
    );
    assert!(convert_mathml_str_to_latex(&xml, &mut budget).is_err());
}

#[test]
fn test_mmultiscripts_as_msup_base() {
    assert_eq!(
        mathml_to_latex("<msup><mmultiscripts><mi>R</mi><mi>i</mi><none/></mmultiscripts><mn>2</mn></msup>"),
        "{R_{i}}^{2}"
    );
}

#[test]
fn test_invisible_operators_become_spaces() {
    assert_eq!(
        mathml_to_latex("<mn>2</mn><mo>\u{2062}</mo><msup><mi>sin</mi><mn>2</mn></msup><mo>\u{2061}</mo><mi>x</mi>"),
        "2 {sin}^{2} x"
    );
}

/// Removed rather than spaced, function application would merge `sin` and `x` into `sinx`.
#[test]
fn test_function_application_keeps_tokens_apart() {
    assert_eq!(mathml_to_latex("<mi>sin</mi><mo>\u{2061}</mo><mi>x</mi>"), "sin x");
}

#[test]
fn test_invisible_separator_keeps_digits_apart() {
    assert_eq!(
        mathml_to_latex("<msub><mi>a</mi><mrow><mn>1</mn><mo>\u{2063}</mo><mn>2</mn></mrow></msub>"),
        "a_{1 2}"
    );
}

#[test]
fn test_invisible_separator_inside_element_text() {
    assert_eq!(mathml_to_latex("<msub><mi>a</mi><mn>1\u{2063}2</mn></msub>"), "a_{1 2}");
}

#[test]
fn test_invisible_plus_becomes_space() {
    assert_eq!(
        mathml_to_latex("<mn>2</mn><mo>\u{2064}</mo><mfrac><mn>1</mn><mn>2</mn></mfrac>"),
        "2 \\frac{1}{2}"
    );
}

#[test]
fn test_invisible_operator_in_direct_text() {
    assert_eq!(mathml_to_latex("<mrow>sin\u{2061}x</mrow>"), "sin x");
}

/// Matches `separators="\u{2063}"` below: an invisible separator shows no comma.
#[test]
fn test_mfenced_invisible_separator_element_is_grouping() {
    assert_eq!(
        mathml_to_latex("<mfenced><mi>a</mi><mo>\u{2063}</mo><mi>b</mi></mfenced>"),
        "\\left(a b\\right)"
    );
}

/// A child that cleans to nothing is still an argument, so its separators stay.
#[test]
fn test_mfenced_blank_child_keeps_separators() {
    assert_eq!(
        mathml_to_latex("<mfenced><mi>a</mi><mi>\u{E001}</mi><mi>b</mi></mfenced>"),
        "\\left(a,,b\\right)"
    );
    assert_eq!(
        mathml_to_latex("<mfenced><mi>a</mi><mi></mi><mi>b</mi></mfenced>"),
        "\\left(a,,b\\right)"
    );
}

/// A space inside `\text{...}` shows, so text content drops an invisible operator.
#[test]
fn test_text_content_drops_invisible_operators() {
    assert_eq!(mathml_to_latex("<mtext>a\u{2062}b</mtext>"), "\\text{ab}");
    assert_eq!(mathml_to_latex("<ms>a\u{2061}b</ms>"), "ab");
}

/// Invisible times inside `mfenced` is a product, not the pair `(2, x)`.
#[test]
fn test_mfenced_invisible_times_is_grouping() {
    assert_eq!(
        mathml_to_latex("<mfenced><mn>2</mn><mo>\u{2062}</mo><mi>x</mi></mfenced>"),
        "\\left(2 x\\right)"
    );
}

#[test]
fn test_mfenced_function_application_is_grouping() {
    assert_eq!(
        mathml_to_latex("<mfenced><mi>f</mi><mo>\u{2061}</mo><mi>x</mi></mfenced>"),
        "\\left(f x\\right)"
    );
}

#[test]
fn test_mfenced_invisible_separators_attribute() {
    assert_eq!(
        mathml_to_latex("<mfenced separators=\"\u{2063}\"><mi>a</mi><mi>b</mi></mfenced>"),
        "\\left(a b\\right)"
    );
}

#[test]
fn test_munderover() {
    assert_eq!(
        mathml_to_latex("<munderover><mo>\u{2211}</mo><mi>i</mi><mi>n</mi></munderover>"),
        "\\sum_{i}^{n}"
    );
    assert_eq!(
        mathml_to_latex("<mover><mo>\u{220F}</mo><mi>n</mi></mover>"),
        "\\prod^{n}"
    );
    assert_eq!(
        mathml_to_latex("<munderover><mi>x</mi><mi>a</mi><mi>b</mi></munderover>"),
        "\\overset{b}{\\underset{a}{x}}"
    );
}

#[test]
fn test_munderover_unmovable_limits_stay_stacked() {
    assert_eq!(
        mathml_to_latex("<munderover><mo movablelimits=\"false\">\u{2211}</mo><mi>i</mi><mi>n</mi></munderover>"),
        "\\sum\\limits_{i}^{n}"
    );
    assert_eq!(
        mathml_to_latex("<munder><mrow><mo movablelimits=\"false\">lim</mo></mrow><mi>n</mi></munder>"),
        "\\lim\\limits_{n}"
    );
    assert_eq!(
        mathml_to_latex("<mover><mo movablelimits=\"false\">\u{220F}</mo><mi>n</mi></mover>"),
        "\\prod\\limits^{n}"
    );
    assert_eq!(
        mathml_to_latex("<munderover><mo movablelimits=\"false\">\u{222B}</mo><mi>a</mi><mi>b</mi></munderover>"),
        "\\int\\limits_{a}^{b}"
    );
    assert_eq!(
        mathml_to_latex("<munderover><mo movablelimits=\"true\">\u{2211}</mo><mi>i</mi><mi>n</mi></munderover>"),
        "\\sum_{i}^{n}"
    );
}

#[test]
fn test_mspace_renders_as_space() {
    assert_eq!(mathml_to_latex("<mrow><mi>a</mi><mspace/><mi>b</mi></mrow>"), "a b");
}

#[test]
fn test_mphantom() {
    assert_eq!(mathml_to_latex("<mphantom><mi>x</mi></mphantom>"), "\\phantom{x}");
}

#[test]
fn test_mtable_matrix() {
    let latex = mathml_to_latex(
        r#"<mtable>
            <mtr><mtd><mn>1</mn></mtd><mtd><mn>2</mn></mtd></mtr>
            <mtr><mtd><mn>3</mn></mtd><mtd><mn>4</mn></mtd></mtr>
        </mtable>"#,
    );
    assert_eq!(latex, "\\begin{matrix}1 & 2 \\\\ 3 & 4\\end{matrix}");
}

#[test]
fn test_unknown_element_degrades_to_text_content() {
    assert_eq!(mathml_to_latex("<mlongdiv><mn>42</mn></mlongdiv>"), "42");
}

/// A document that ships the author's TeX states the formula exactly, so it
/// beats reconstructing LaTeX from the presentation tree.
#[test]
fn test_tex_annotation_wins_over_the_presentation_tree() {
    let latex = mathml_to_latex(
        r#"<semantics>
            <mrow><mi>E</mi><mo>=</mo><mi>m</mi><msup><mi>c</mi><mn>2</mn></msup></mrow>
            <annotation encoding="application/x-tex">E = mc^2</annotation>
        </semantics>"#,
    );
    assert_eq!(latex, "E = mc^2");
}

/// Renderers wrap the expression in the style the surrounding document set.
#[test]
fn test_display_style_wrapper_comes_off() {
    let latex = mathml_to_latex(
        r#"<semantics>
            <mrow><mi>x</mi></mrow>
            <annotation encoding="application/x-tex">{\displaystyle x^{2}+1}</annotation>
        </semantics>"#,
    );
    assert_eq!(latex, "x^{2}+1");
}

/// A brace that closes before the end is part of the formula, so the
/// wrapper stays.
#[test]
fn test_partial_brace_group_keeps_the_wrapper() {
    assert_eq!(strip_style_wrapper("{\\displaystyle a} + b"), "{\\displaystyle a} + b");
}

/// An annotation in another notation is not TeX and must not leak.
#[test]
fn test_non_tex_annotation_still_renders_the_presentation_branch() {
    let latex = mathml_to_latex(
        r#"<semantics>
            <mrow><mi>E</mi><mo>=</mo><mi>m</mi></mrow>
            <annotation encoding="StarMath 5.0">E = m</annotation>
        </semantics>"#,
    );
    assert_eq!(latex, "E=m");
}

/// An empty annotation carries nothing, so the presentation tree stands.
#[test]
fn test_empty_tex_annotation_falls_back() {
    let latex = mathml_to_latex(
        r#"<semantics>
            <mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow>
            <annotation encoding="application/x-tex">   </annotation>
        </semantics>"#,
    );
    assert_eq!(latex, "a+b");
}

/// Content MathML states meaning rather than layout, so an `apply` tree
/// converts by operator.
#[test]
fn test_content_mathml_apply_converts_by_operator() {
    assert_eq!(mathml_to_latex("<apply><plus/><ci>a</ci><ci>b</ci></apply>"), "a+b");
    assert_eq!(mathml_to_latex("<apply><power/><ci>x</ci><cn>2</cn></apply>"), "x^{2}");
    assert_eq!(
        mathml_to_latex("<apply><root/><degree><cn>3</cn></degree><ci>x</ci></apply>"),
        "\\sqrt[3]{x}"
    );
}

#[test]
fn test_content_mathml_functions_and_relations() {
    assert_eq!(
        mathml_to_latex("<apply><eq/><ci>y</ci><apply><sin/><ci>x</ci></apply></apply>"),
        "y=\\sin\\left(x\\right)"
    );
}

#[test]
fn test_content_mathml_arg_is_named_operator() {
    assert_eq!(
        mathml_to_latex("<apply><arg/><ci>z</ci></apply>"),
        "\\arg\\left(z\\right)"
    );
}

#[test]
fn test_content_mathml_coth_is_named_operator() {
    assert_eq!(
        mathml_to_latex("<apply><coth/><ci>x</ci></apply>"),
        "\\coth\\left(x\\right)"
    );
}

/// A sum carries its bound variable and limits.
#[test]
fn test_content_mathml_sum_carries_limits() {
    let latex = mathml_to_latex(
        "<apply><sum/><bvar><ci>i</ci></bvar><lowlimit><cn>1</cn></lowlimit>             <uplimit><ci>n</ci></uplimit><ci>i</ci></apply>",
    );
    assert_eq!(latex, "\\sum_{i=1}^{n} i");
}

#[test]
fn test_content_mathml_matrix_and_piecewise() {
    let matrix = mathml_to_latex(
        "<matrix><matrixrow><cn>1</cn><cn>0</cn></matrixrow><matrixrow><cn>0</cn><cn>1</cn></matrixrow></matrix>",
    );
    assert_eq!(matrix, "\\begin{pmatrix}1 & 0 \\\\ 0 & 1\\end{pmatrix}");

    let cases = mathml_to_latex(
        "<piecewise><piece><cn>0</cn><apply><lt/><ci>x</ci><cn>0</cn></apply></piece>             <otherwise><ci>x</ci></otherwise></piecewise>",
    );
    assert!(cases.starts_with("\\begin{cases}"), "got: {cases}");
    assert!(cases.contains("\\text{otherwise}"), "got: {cases}");
}

/// An operator the mapping does not name still parses and still says what
/// the source said.
#[test]
fn test_unknown_content_operator_degrades() {
    assert_eq!(
        mathml_to_latex("<apply><wibble/><ci>a</ci></apply>"),
        "\\operatorname{wibble}\\left(a\\right)"
    );
}

/// A document that carries only the content branch has its meaning read from
/// `annotation-xml`, since the presentation side renders to nothing.
#[test]
fn test_content_annotation_is_used_when_presentation_is_empty() {
    let latex = mathml_to_latex(
        r#"<semantics><mrow/><annotation-xml encoding="MathML-Content">
            <apply><plus/><ci>a</ci><ci>b</ci></apply>
        </annotation-xml></semantics>"#,
    );
    assert_eq!(latex, "a+b");
}

/// A document with a working presentation branch keeps using it, so nothing
/// that already converted changes.
#[test]
fn test_presentation_branch_still_wins_over_content_annotation() {
    let latex = mathml_to_latex(
        r#"<semantics><mrow><mi>E</mi><mo>=</mo><mi>m</mi></mrow>
            <annotation-xml encoding="MathML-Content"><apply><plus/><ci>q</ci><ci>r</ci></apply></annotation-xml>
        </semantics>"#,
    );
    assert_eq!(latex, "E=m");
}

#[test]
fn test_semantics_renders_presentation_branch_only() {
    let latex = mathml_to_latex(
        r#"<semantics>
            <mrow><mi>E</mi><mo>=</mo><mi>m</mi></mrow>
            <annotation encoding="StarMath 5.0">E = m</annotation>
        </semantics>"#,
    );
    assert_eq!(latex, "E=m");
}

#[test]
fn test_nested_quadratic_formula() {
    // Uses a literal '±' (U+00B1) character, matching how the OMML test
    // suite embeds Unicode math symbols directly rather than as escapes
    // (escape sequences are not processed inside raw strings).
    let latex = mathml_to_latex(
        r#"<mi>x</mi><mo>=</mo>
        <mfrac>
            <mrow>
                <mo>-</mo><mi>b</mi><mo>±</mo>
                <msqrt>
                    <msup><mi>b</mi><mn>2</mn></msup>
                    <mo>-</mo><mn>4</mn><mi>a</mi><mi>c</mi>
                </msqrt>
            </mrow>
            <mrow><mn>2</mn><mi>a</mi></mrow>
        </mfrac>"#,
    );
    assert_eq!(latex, "x=\\frac{-b\\pm \\sqrt{b^{2}-4ac}}{2a}");
}

#[test]
fn test_formula_odt_fixture_shape() {
    // Mirrors the real embedded formula object in test_documents/odt/formula.odt:
    // E = m * c^2, wrapped in <semantics>/<annotation> with a StarMath fallback.
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <math xmlns="http://www.w3.org/1998/Math/MathML">
            <semantics>
                <mrow><mrow><mi>E</mi><mo stretchy="false">=</mo>
                <mrow><mi>m</mi><mo stretchy="false">⋅</mo>
                <msup><mi>c</mi><mn>2</mn></msup></mrow></mrow></mrow>
                <annotation encoding="StarMath 5.0">E = m cdot c^2</annotation>
            </semantics>
        </math>"#;
    let mut budget = SecurityBudget::with_defaults();
    let latex = convert_mathml_str_to_latex(xml, &mut budget).expect("conversion ok");
    assert_eq!(latex, "E=m\\cdot c^{2}");
}

#[test]
fn test_convert_mathml_node_to_latex_from_pre_parsed_node() {
    let xml = r#"<math xmlns="http://www.w3.org/1998/Math/MathML"><mi>x</mi></math>"#;
    let doc = roxmltree::Document::parse(xml).expect("parses");
    let mut budget = SecurityBudget::with_defaults();
    let latex = convert_mathml_node_to_latex(doc.root_element(), &mut budget).expect("conversion ok");
    assert_eq!(latex, "x");
}

#[test]
fn test_depth_failure_does_not_leak_mathml_budget_depth() {
    let limits = crate::extractors::security::SecurityLimits {
        max_nesting_depth: 1,
        max_xml_depth: 1,
        ..Default::default()
    };
    let mut budget = SecurityBudget::from_limits(&limits);
    let nested = roxmltree::Document::parse("<math><mrow><mi>x</mi></mrow></math>").expect("parse nested MathML");

    assert!(convert_mathml_node_to_latex(nested.root_element(), &mut budget).is_err());

    let shallow = roxmltree::Document::parse("<mi>x</mi>").expect("parse shallow MathML");
    assert_eq!(
        convert_mathml_node_to_latex(shallow.root_element(), &mut budget).expect("depth should be restored"),
        "x"
    );
}
