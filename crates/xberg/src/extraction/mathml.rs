//! Presentation MathML to LaTeX converter.
//!
//! Converts `<math>` (MathML) subtrees found in ODT/ODP embedded formula objects
//! and EPUB XHTML content to LaTeX notation. Modeled on the OMML converter at
//! `docx::math`: the subtree is collected into an `MmlNode` tree, then recursively
//! rendered to LaTeX. Unknown/unhandled elements degrade to their text content
//! instead of failing the whole document.
//!
//! Unlike OMML (which streams off a `quick_xml::Reader` while parsing DOCX part
//! XML), callers here already hold a parsed `roxmltree::Document` — ODT parses an
//! embedded object's `content.xml` on its own, and EPUB walks XHTML with
//! `roxmltree` already. So the converter operates directly on a `roxmltree::Node`,
//! with a `&str`-in convenience wrapper for callers that only have raw XML text.

use crate::extractors::security::{SecurityBudget, SecurityError};
use roxmltree::Node;

/// Names of MathML elements that hold no rendered content and whose text (an
/// alternate-encoding annotation, e.g. `StarMath` or content-MathML) must never
/// leak into the LaTeX output.
const ANNOTATION_ELEMENTS: &[&str] = &["annotation", "annotation-xml"];

/// `annotation` encodings whose text is the LaTeX the author wrote.
///
/// A document that ships one states the formula exactly, so it beats
/// reconstructing LaTeX from the presentation tree, which can only approximate
/// the author's spelling.
const TEX_ANNOTATION_ENCODINGS: &[&str] = &["application/x-tex", "text/x-tex", "tex", "latex"];

/// Names of MathML elements that are pure grouping/styling wrappers: their
/// children are rendered in sequence with no LaTeX markup of their own.
const TRANSPARENT_ELEMENTS: &[&str] = &["math", "mrow", "mstyle", "mpadded", "merror"];

#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
enum MmlNode {
    /// LaTeX taken verbatim from a TeX `annotation`.
    Verbatim(String),
    /// Plain text from `mi`/`mn`/`mo`/`ms`.
    Run(String),
    /// Literal text from `mtext`: rendered as `\text{...}`.
    Text(String),
    /// A single blank space from `mspace`.
    Space,
    /// Fraction: `\frac{num}{den}`.
    Frac { num: Box<MmlNode>, den: Box<MmlNode> },
    /// Superscript: `base^{sup}`.
    Sup { base: Box<MmlNode>, sup: Box<MmlNode> },
    /// Subscript: `base_{sub}`.
    Sub { base: Box<MmlNode>, sub: Box<MmlNode> },
    /// Sub-superscript: `base_{sub}^{sup}`.
    SubSup {
        base: Box<MmlNode>,
        sub: Box<MmlNode>,
        sup: Box<MmlNode>,
    },
    /// Square root: `\sqrt{body}`.
    Sqrt { body: Box<MmlNode> },
    /// N-th root: `\sqrt[index]{body}`.
    Root { body: Box<MmlNode>, index: Box<MmlNode> },
    /// Fenced group: `\left<open> a, b, ...\right<close>`.
    Fenced {
        open: String,
        close: String,
        sep: String,
        elements: Vec<MmlNode>,
    },
    /// Underscript: an under-accent (`\underline{base}`), a limit (`\lim_{under}`), or
    /// `\underset{under}{base}`. `stacked` is set when the base operator has `movablelimits="false"`, and
    /// `no_accentunder` when `accentunder` resolves to false, which keeps an accent-like script a script.
    Under {
        base: Box<MmlNode>,
        under: Box<MmlNode>,
        stacked: bool,
        no_accentunder: bool,
    },
    /// Overscript: an accent (`\hat{base}`), a limit (`\sum^{over}`), or `\overset{over}{base}`.
    /// `no_accent` is set when `accent` resolves to false.
    Over {
        base: Box<MmlNode>,
        over: Box<MmlNode>,
        stacked: bool,
        no_accent: bool,
    },
    /// Under+overscript: limits (`\sum_{under}^{over}`) or `\overset{over}{\underset{under}{base}}`.
    UnderOver {
        base: Box<MmlNode>,
        under: Box<MmlNode>,
        over: Box<MmlNode>,
        stacked: bool,
        no_accent: bool,
        no_accentunder: bool,
    },
    /// Phantom (invisible but space-occupying): `\phantom{body}`.
    Phantom { body: Box<MmlNode> },
    /// Table: `\begin{matrix}...\end{matrix}`.
    Table { rows: Vec<Vec<MmlNode>> },
    /// Grouping container (`math`, `mrow`, `semantics` presentation branch,
    /// unknown elements) — renders its children in sequence.
    Group { children: Vec<MmlNode> },
}

/// Convert a MathML XML fragment (a full document whose root is `<math>`, or
/// `<math>` nested anywhere in the fragment) to LaTeX.
///
/// Used by callers (e.g. ODT's embedded-object formula extraction) that only
/// have the raw XML text of a formula and have not already parsed it.
pub(crate) fn convert_mathml_str_to_latex(xml: &str, budget: &mut SecurityBudget) -> Result<String, SecurityError> {
    // An embedded formula object carries its own DOCTYPE: OpenOffice writes
    // `<!DOCTYPE math:math PUBLIC "-//OpenOffice.org//DTD Modified W3C MathML
    // 1.01//EN">` into every one, and `roxmltree` rejects a DTD outright, so the
    // formula was dropped. The declaration is removed instead of allowed: a
    // formula needs none of it, and the rejection still guards the entity
    // expansion that a hostile document would declare inside one.
    let xml = crate::utils::xml_utils::strip_doctype(xml);
    let Ok(doc) = roxmltree::Document::parse(&xml) else {
        return Ok(String::new());
    };

    let root = doc.root_element();
    let math_node = if root.tag_name().name().eq_ignore_ascii_case("math") {
        root
    } else {
        match root
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("math"))
        {
            Some(node) => node,
            None => root,
        }
    };

    convert_mathml_node_to_latex(math_node, budget)
}

/// Convert an already-parsed MathML `<math>` (or presentation) node to LaTeX.
///
/// Used by callers (e.g. the EPUB XHTML walker) that already hold a
/// `roxmltree::Node` positioned at the `<math>` element.
pub(crate) fn convert_mathml_node_to_latex(node: Node, budget: &mut SecurityBudget) -> Result<String, SecurityError> {
    let collected = collect_node(node, budget)?;
    let mut out = String::new();
    render_node(&collected, &mut out);
    Ok(out)
}

/// Collect an element's children into a sequence of `MmlNode`s, dispatching each
/// child element through [`collect_node`] and each direct text node into a
/// [`MmlNode::Run`]. Whitespace-only text nodes are dropped.
fn collect_children(parent: Node, budget: &mut SecurityBudget) -> Result<Vec<MmlNode>, SecurityError> {
    let mut nodes = Vec::new();
    for child in parent.children() {
        budget.step()?;
        if child.is_element() {
            nodes.push(collect_node(child, budget)?);
        } else if child.is_text() {
            let text = child.text().unwrap_or("");
            if !text.trim().is_empty() {
                budget.check_entity(text)?;
                budget.account_text(text.len())?;
                nodes.push(MmlNode::Run(text.to_string()));
            }
        }
    }
    Ok(nodes)
}

/// Collect the Nth element child of `parent` (skipping non-element nodes) into a
/// single `MmlNode`, or an empty `Group` if fewer than `index + 1` element
/// children exist.
fn collect_nth_child(parent: Node, index: usize, budget: &mut SecurityBudget) -> Result<MmlNode, SecurityError> {
    match parent.children().filter(|c| c.is_element()).nth(index) {
        Some(child) => collect_node(child, budget),
        None => Ok(MmlNode::Group { children: Vec::new() }),
    }
}

/// The value of a boolean attribute, or `None` when it is absent or neither `true` nor `false`.
fn boolean_attribute(node: Node, name: &str) -> Option<bool> {
    let value = node.attribute(name)?.trim();
    if value.eq_ignore_ascii_case("true") {
        Some(true)
    } else if value.eq_ignore_ascii_case("false") {
        Some(false)
    } else {
        None
    }
}

/// Report whether an under/over element turns off the accent of its script at `script_index`. Its own `attr`
/// (`accent` or `accentunder`) decides when given, else the `accent` of the script's core `mo` (MathML 3
/// sections 3.4.4.2, 3.4.5.2, 3.4.6.2). Left to neither, the script-character heuristic stands in for the operator
/// dictionary, so an explicit `true` changes nothing.
fn accent_disabled(node: Node, attr: &str, script_index: usize) -> bool {
    let resolved = boolean_attribute(node, attr).or_else(|| {
        let script = node.children().filter(|c| c.is_element()).nth(script_index)?;
        boolean_attribute(core_mo(script)?, "accent")
    });
    resolved == Some(false)
}

/// Report whether the base of an under/over element is an `mo` with `movablelimits="false"`.
fn base_limits_unmovable(node: Node) -> bool {
    let base = node.children().find(|c| c.is_element());
    base.and_then(core_mo)
        .and_then(|mo| boolean_attribute(mo, "movablelimits"))
        == Some(false)
}

/// The `mo` at the core of `element`, looking through single-child grouping and a `semantics` with one
/// presentation child, as the collector and renderer do.
fn core_mo<'a, 'input>(element: Node<'a, 'input>) -> Option<Node<'a, 'input>> {
    let mut current = element;
    loop {
        let tag = current.tag_name().name();
        if tag.eq_ignore_ascii_case("mo") {
            return Some(current);
        }
        let semantics = tag.eq_ignore_ascii_case("semantics");
        if !semantics && !TRANSPARENT_ELEMENTS.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            return None;
        }
        let mut children = current
            .children()
            .filter(|c| c.is_element() && !(semantics && is_annotation(*c)));
        current = match (children.next(), children.next()) {
            (Some(only), None) => only,
            _ => return None,
        };
    }
}

/// Report whether `node` is an `annotation` or `annotation-xml` element.
fn is_annotation(node: Node) -> bool {
    ANNOTATION_ELEMENTS
        .iter()
        .any(|&s| s.eq_ignore_ascii_case(node.tag_name().name()))
}

/// Collect a single MathML element into an `MmlNode`, dispatching on tag name.
fn collect_node(node: Node, budget: &mut SecurityBudget) -> Result<MmlNode, SecurityError> {
    budget.step()?;
    if let Err(error) = budget.enter() {
        budget.leave();
        return Err(error);
    }
    let result = collect_node_inner(node, budget);
    budget.leave();
    result
}

/// The LaTeX of a `semantics` child annotation that carries TeX, if any.
///
/// Renderers wrap the whole expression in `{\displaystyle ...}` or
/// `{\textstyle ...}` to state the style the surrounding document set. That
/// wrapper is presentation, not the formula, so it comes off; `$` delimiters
/// come off for the same reason the projection strips them.
fn tex_annotation(node: Node, budget: &mut SecurityBudget) -> Result<Option<String>, SecurityError> {
    for child in node.children().filter(|c| c.is_element()) {
        if !child.tag_name().name().eq_ignore_ascii_case("annotation") {
            continue;
        }
        let Some(encoding) = child.attribute("encoding") else {
            continue;
        };
        if !TEX_ANNOTATION_ENCODINGS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(encoding.trim()))
        {
            continue;
        }
        let text = collect_text(child, budget)?;
        let latex = strip_style_wrapper(crate::extraction::derive::strip_math_delimiters(text.trim()));
        if !latex.is_empty() {
            return Ok(Some(latex.to_string()));
        }
    }
    Ok(None)
}

/// Remove a `{\displaystyle ...}` or `{\textstyle ...}` wrapper around the
/// whole expression.
pub(crate) fn strip_style_wrapper(latex: &str) -> &str {
    for prefix in ["{\\displaystyle", "{\\textstyle", "{\\scriptstyle"] {
        let Some(rest) = latex.strip_prefix(prefix) else {
            continue;
        };
        let Some(inner) = rest.strip_suffix('}') else {
            continue;
        };
        // The wrapper must enclose everything: a brace that closes early means
        // the tail belongs to the formula.
        let mut depth = 1i32;
        for ch in inner.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                return latex;
            }
        }
        return inner.trim();
    }
    latex
}

/// The LaTeX of a content-MathML `annotation-xml` child, if the element has one.
fn content_annotation(node: Node, budget: &mut SecurityBudget) -> Result<Option<String>, SecurityError> {
    for child in node.children().filter(|c| c.is_element()) {
        if !child.tag_name().name().eq_ignore_ascii_case("annotation-xml") {
            continue;
        }
        let is_content = child
            .attribute("encoding")
            .is_some_and(|e| e.trim().eq_ignore_ascii_case("MathML-Content"));
        if !is_content {
            continue;
        }
        for inner in child.children().filter(|c| c.is_element()) {
            let latex = convert_content_node(inner, budget)?;
            if !latex.trim().is_empty() {
                return Ok(Some(latex.trim().to_string()));
            }
        }
    }
    Ok(None)
}

/// Content MathML operators that render as an infix chain.
const INFIX_OPERATORS: &[(&str, &str)] = &[
    ("plus", "+"),
    ("minus", "-"),
    ("times", " \\times "),
    ("divide", " \\div "),
    ("eq", "="),
    ("neq", " \\ne "),
    ("lt", " < "),
    ("gt", " > "),
    ("leq", " \\le "),
    ("geq", " \\ge "),
    ("equivalent", " \\equiv "),
    ("approx", " \\approx "),
    ("and", " \\land "),
    ("or", " \\lor "),
    ("implies", " \\implies "),
    ("in", " \\in "),
    ("notin", " \\notin "),
    ("subset", " \\subset "),
    ("prsubset", " \\subsetneq "),
    ("union", " \\cup "),
    ("intersect", " \\cap "),
    ("setdiff", " \\setminus "),
    ("cartesianproduct", " \\times "),
    ("compose", " \\circ "),
];

/// Convert a content-MathML `apply` subtree to LaTeX.
///
/// Content MathML states what a formula *means* (`<apply><plus/><ci>a</ci>…`)
/// rather than how it looks, so it converts by operator rather than by layout.
/// An operator with no LaTeX spelling becomes `\operatorname{name}(args)`, which
/// parses and still names what the source said.
fn convert_apply(node: Node, budget: &mut SecurityBudget) -> Result<String, SecurityError> {
    budget.step()?;
    let mut children = node.children().filter(|c| c.is_element());
    let Some(operator) = children.next() else {
        return Ok(String::new());
    };
    let name = operator.tag_name().name().to_ascii_lowercase();

    // `bvar`, `lowlimit`, `uplimit`, `degree`, and `condition` qualify the
    // operator; everything else is an operand.
    let mut operands: Vec<Node> = Vec::new();
    let (mut bvar, mut lower, mut upper, mut degree) = (None, None, None, None);
    for child in children {
        match child.tag_name().name().to_ascii_lowercase().as_str() {
            "bvar" => bvar = Some(child),
            "lowlimit" | "condition" => lower = Some(child),
            "uplimit" => upper = Some(child),
            "degree" => degree = Some(child),
            _ => operands.push(child),
        }
    }

    let rendered: Vec<String> = operands
        .iter()
        .map(|c| convert_content_node(*c, budget))
        .collect::<Result<_, _>>()?;

    if let Some((_, latex)) = INFIX_OPERATORS.iter().find(|(op, _)| *op == name) {
        // Unary minus reads as negation rather than subtraction.
        if name == "minus" && rendered.len() == 1 {
            return Ok(format!("-{}", rendered[0]));
        }
        return Ok(rendered.join(latex));
    }
    if let Some((latex, _)) = crate::extraction::math_symbols::operator_name_to_latex(&name) {
        return Ok(format!("{latex}\\left({}\\right)", rendered.join(", ")));
    }

    match name.as_str() {
        "power" if rendered.len() == 2 => Ok(format!("{}^{{{}}}", rendered[0], rendered[1])),
        "root" => {
            let index = apply_qualifier(degree, budget)?;
            let radicand = rendered.first().cloned().unwrap_or_default();
            if index.is_empty() || index == "2" {
                Ok(format!("\\sqrt{{{radicand}}}"))
            } else {
                Ok(format!("\\sqrt[{index}]{{{radicand}}}"))
            }
        }
        "abs" => Ok(format!("\\left|{}\\right|", rendered.join(", "))),
        "floor" => Ok(format!("\\lfloor {}\\rfloor", rendered.join(", "))),
        "ceiling" => Ok(format!("\\lceil {}\\rceil", rendered.join(", "))),
        "factorial" => Ok(format!("{}!", rendered.join(""))),
        "sum" | "product" | "int" => convert_apply_bigop(&name, &rendered, bvar, lower, upper, budget),
        "diff" => {
            let var = apply_qualifier(bvar, budget)?;
            let body = rendered.join("");
            if var.is_empty() {
                Ok(format!("\\frac{{d}}{{dx}}{body}"))
            } else {
                Ok(format!("\\frac{{d}}{{d{var}}}{body}"))
            }
        }
        // An operator the mapping does not name still parses and still says what
        // the source said.
        _ => Ok(format!(
            "\\operatorname{{{}}}\\left({}\\right)",
            name,
            rendered.join(", ")
        )),
    }
}

/// Resolve a qualifier child (`bvar`, `lowlimit`/`condition`, `uplimit`, `degree`) of a
/// content-MathML `apply`'s operator to its rendered LaTeX text, or the empty string when the
/// qualifier is absent.
fn apply_qualifier(qualifier: Option<Node>, budget: &mut SecurityBudget) -> Result<String, SecurityError> {
    match qualifier {
        Some(n) => {
            let parts: Vec<String> = n
                .children()
                .filter(|c| c.is_element())
                .map(|c| convert_content_node(c, budget))
                .collect::<Result<_, _>>()?;
            Ok(parts.join(""))
        }
        None => Ok(String::new()),
    }
}

/// Convert a content-MathML `sum`/`product`/`int` `apply` to its big-operator LaTeX form, with
/// the bound variable (`bvar`) and limits (`lowlimit`/`condition`, `uplimit`) applied as
/// sub/superscripts, and (for `int`) a trailing `\,d<var>` differential.
fn convert_apply_bigop(
    name: &str,
    rendered: &[String],
    bvar: Option<Node>,
    lower: Option<Node>,
    upper: Option<Node>,
    budget: &mut SecurityBudget,
) -> Result<String, SecurityError> {
    let command = match name {
        "sum" => "\\sum",
        "product" => "\\prod",
        _ => "\\int",
    };
    let var = apply_qualifier(bvar, budget)?;
    let from = apply_qualifier(lower, budget)?;
    let to = apply_qualifier(upper, budget)?;
    let mut out = String::from(command);
    if !from.is_empty() {
        let start = if var.is_empty() { from } else { format!("{var}={from}") };
        out.push_str(&format!("_{{{start}}}"));
    } else if !var.is_empty() {
        out.push_str(&format!("_{{{var}}}"));
    }
    if !to.is_empty() {
        out.push_str(&format!("^{{{to}}}"));
    }
    out.push(' ');
    out.push_str(&rendered.join(""));
    if name == "int" && !var.is_empty() {
        out.push_str(&format!("\\,d{var}"));
    }
    Ok(out)
}

/// Convert one content-MathML node to LaTeX.
fn convert_content_node(node: Node, budget: &mut SecurityBudget) -> Result<String, SecurityError> {
    budget.step()?;
    match node.tag_name().name().to_ascii_lowercase().as_str() {
        "apply" => convert_apply(node, budget),
        "ci" | "cn" | "csymbol" => {
            let text = collect_text(node, budget)?;
            let mut out = String::new();
            crate::extraction::math_symbols::render_run_text(text.trim(), &mut out);
            Ok(out)
        }
        "matrix" | "vector" => {
            let rows: Vec<String> = node
                .children()
                .filter(|c| c.is_element())
                .map(|row| {
                    let cells: Vec<String> = row
                        .children()
                        .filter(|c| c.is_element())
                        .map(|c| convert_content_node(c, budget))
                        .collect::<Result<_, _>>()?;
                    Ok(if cells.is_empty() {
                        convert_content_node(row, budget)?
                    } else {
                        cells.join(" & ")
                    })
                })
                .collect::<Result<_, SecurityError>>()?;
            Ok(format!("\\begin{{pmatrix}}{}\\end{{pmatrix}}", rows.join(" \\\\ ")))
        }
        "piecewise" => {
            let mut rows: Vec<String> = Vec::new();
            for piece in node.children().filter(|c| c.is_element()) {
                let parts: Vec<String> = piece
                    .children()
                    .filter(|c| c.is_element())
                    .map(|c| convert_content_node(c, budget))
                    .collect::<Result<_, _>>()?;
                rows.push(match piece.tag_name().name().to_ascii_lowercase().as_str() {
                    "otherwise" => format!("{} & \\text{{otherwise}}", parts.join("")),
                    _ => parts.join(" & \\text{if }"),
                });
            }
            Ok(format!("\\begin{{cases}}{}\\end{{cases}}", rows.join(" \\\\ ")))
        }
        "list" | "set" => {
            let items: Vec<String> = node
                .children()
                .filter(|c| c.is_element())
                .map(|c| convert_content_node(c, budget))
                .collect::<Result<_, _>>()?;
            let inner = items.join(", ");
            Ok(if node.tag_name().name().eq_ignore_ascii_case("set") {
                format!("\\{{{inner}\\}}")
            } else {
                format!("\\left({inner}\\right)")
            })
        }
        // A constant such as `<pi/>` or `<exponentiale/>` carries its meaning in
        // its name.
        "" => Ok(String::new()),
        other => match other {
            "pi" => Ok("\\pi".to_string()),
            "exponentiale" => Ok("e".to_string()),
            "imaginaryi" => Ok("i".to_string()),
            "infinity" => Ok("\\infty".to_string()),
            "true" => Ok("\\text{true}".to_string()),
            "false" => Ok("\\text{false}".to_string()),
            "emptyset" => Ok("\\emptyset".to_string()),
            _ => {
                let text = collect_text(node, budget)?;
                Ok(text.trim().to_string())
            }
        },
    }
}

fn collect_node_inner(node: Node, budget: &mut SecurityBudget) -> Result<MmlNode, SecurityError> {
    if is_annotation(node) {
        return Ok(MmlNode::Group { children: Vec::new() });
    }

    let tag_lower = node.tag_name().name().to_ascii_lowercase();
    match tag_lower.as_str() {
        "mi" | "mn" | "ms" | "mo" => Ok(MmlNode::Run(collect_text(node, budget)?)),
        "mtext" => Ok(MmlNode::Text(collect_text(node, budget)?)),
        "mspace" => Ok(MmlNode::Space),
        // Content MathML states meaning rather than layout, so it converts by
        // operator. It appears as a `math` child in content documents and inside
        // `annotation-xml` in mixed ones.
        "apply" | "piecewise" | "matrix" | "vector" | "set" | "list" | "ci" | "cn" | "csymbol" => {
            Ok(MmlNode::Verbatim(convert_content_node(node, budget)?))
        }
        "semantics" => collect_semantics_node(node, budget),
        t if TRANSPARENT_ELEMENTS.contains(&t) => Ok(MmlNode::Group {
            children: collect_children(node, budget)?,
        }),
        "mfrac" | "msup" | "msub" | "msubsup" | "msqrt" | "mroot" | "mfenced" | "munder" | "mover" | "munderover"
        | "mphantom" | "mtable" => collect_scripted_node(&tag_lower, node, budget),
        "mmultiscripts" => collect_multiscripts(node, budget),
        _ => Ok(MmlNode::Group {
            children: collect_children(node, budget)?,
        }),
    }
}

/// Collect a `semantics` element: an embedded TeX annotation wins outright over the
/// presentation tree; otherwise collect the non-annotation (presentation) children,
/// falling back to a content-MathML annotation when the presentation branch renders to
/// nothing.
fn collect_semantics_node(node: Node, budget: &mut SecurityBudget) -> Result<MmlNode, SecurityError> {
    if let Some(tex) = tex_annotation(node, budget)? {
        return Ok(MmlNode::Verbatim(tex));
    }
    let children = node
        .children()
        .filter(|c| c.is_element() && !is_annotation(*c))
        .map(|c| collect_node(c, budget))
        .collect::<Result<Vec<_>, _>>()?;
    // A document may carry only the content branch, in which case the
    // presentation side renders to nothing and the meaning is all there
    // is to work from.
    if render_nodes(&children).trim().is_empty()
        && let Some(latex) = content_annotation(node, budget)?
    {
        return Ok(MmlNode::Verbatim(latex));
    }
    Ok(MmlNode::Group { children })
}

/// Collect a scripted/structural MathML element (fraction, sub/superscript, root, fence,
/// under/over, phantom, or table) into its `MmlNode` variant. `tag` is `node`'s
/// lowercased tag name, already matched by the caller against this function's own arms.
fn collect_scripted_node(tag: &str, node: Node, budget: &mut SecurityBudget) -> Result<MmlNode, SecurityError> {
    match tag {
        "mfrac" => Ok(MmlNode::Frac {
            num: Box::new(collect_nth_child(node, 0, budget)?),
            den: Box::new(collect_nth_child(node, 1, budget)?),
        }),
        "msup" => Ok(MmlNode::Sup {
            base: Box::new(collect_nth_child(node, 0, budget)?),
            sup: Box::new(collect_nth_child(node, 1, budget)?),
        }),
        "msub" => Ok(MmlNode::Sub {
            base: Box::new(collect_nth_child(node, 0, budget)?),
            sub: Box::new(collect_nth_child(node, 1, budget)?),
        }),
        "msubsup" => Ok(MmlNode::SubSup {
            base: Box::new(collect_nth_child(node, 0, budget)?),
            sub: Box::new(collect_nth_child(node, 1, budget)?),
            sup: Box::new(collect_nth_child(node, 2, budget)?),
        }),
        "msqrt" => Ok(MmlNode::Sqrt {
            body: Box::new(MmlNode::Group {
                children: collect_children(node, budget)?,
            }),
        }),
        "mroot" => Ok(MmlNode::Root {
            body: Box::new(collect_nth_child(node, 0, budget)?),
            index: Box::new(collect_nth_child(node, 1, budget)?),
        }),
        "mfenced" => collect_fenced(node, budget),
        "munder" => Ok(MmlNode::Under {
            base: Box::new(collect_nth_child(node, 0, budget)?),
            under: Box::new(collect_nth_child(node, 1, budget)?),
            stacked: base_limits_unmovable(node),
            no_accentunder: accent_disabled(node, "accentunder", 1),
        }),
        "mover" => Ok(MmlNode::Over {
            base: Box::new(collect_nth_child(node, 0, budget)?),
            over: Box::new(collect_nth_child(node, 1, budget)?),
            stacked: base_limits_unmovable(node),
            no_accent: accent_disabled(node, "accent", 1),
        }),
        "munderover" => Ok(MmlNode::UnderOver {
            base: Box::new(collect_nth_child(node, 0, budget)?),
            under: Box::new(collect_nth_child(node, 1, budget)?),
            over: Box::new(collect_nth_child(node, 2, budget)?),
            stacked: base_limits_unmovable(node),
            no_accent: accent_disabled(node, "accent", 2),
            no_accentunder: accent_disabled(node, "accentunder", 1),
        }),
        "mphantom" => Ok(MmlNode::Phantom {
            body: Box::new(MmlNode::Group {
                children: collect_children(node, budget)?,
            }),
        }),
        "mtable" => collect_table(node, budget),
        // Unreachable in practice: the caller only dispatches here for the tags matched
        // above, mirroring its own outer pattern's tag set.
        _ => Ok(MmlNode::Group {
            children: collect_children(node, budget)?,
        }),
    }
}

/// Collect an `mmultiscripts` element: a base, then sub/sup pairs after it and, past `mprescripts`, before
/// it. Each pair becomes plain scripts, a prescript on an empty base (`{}_{6}^{14}C`), which is how LaTeX
/// writes one. A `none` script is left out.
fn collect_multiscripts(node: Node, budget: &mut SecurityBudget) -> Result<MmlNode, SecurityError> {
    let mut elements = node.children().filter(|c| c.is_element());
    let mut base = elements.next().map(|first| collect_node(first, budget)).transpose()?;
    let (mut post, mut pre) = (Vec::new(), Vec::new());
    let mut in_prescripts = false;
    for child in elements {
        budget.step()?;
        let tag = child.tag_name().name();
        if tag.eq_ignore_ascii_case("mprescripts") {
            in_prescripts = true;
            continue;
        }
        // A script that renders to nothing sets nothing either, and kept it would write `R_{i}^{}`.
        let script = if tag.eq_ignore_ascii_case("none") {
            None
        } else {
            Some(collect_node(child, budget)?).filter(|n| !is_blank_script(n))
        };
        if in_prescripts {
            pre.push(script);
        } else {
            post.push(script);
        }
    }

    let empty = || MmlNode::Group { children: Vec::new() };
    let mut children = Vec::new();
    let mut pre = pre.into_iter();
    while let Some(sub) = pre.next() {
        children.push(scripted(empty(), sub, pre.next().flatten()));
    }
    let mut post = post.into_iter();
    while let Some(sub) = post.next() {
        children.push(scripted(base.take().unwrap_or_else(empty), sub, post.next().flatten()));
    }
    children.extend(base);
    Ok(MmlNode::Group { children })
}

/// Report whether an `mmultiscripts` script renders to nothing. A blank `mtext` is kept, since `\text{ }` shows.
fn is_blank_script(node: &MmlNode) -> bool {
    match node {
        MmlNode::Run(text) => text.trim().is_empty(),
        MmlNode::Text(text) => text.is_empty(),
        MmlNode::Group { children } => children.iter().all(is_blank_script),
        _ => false,
    }
}

/// `base` with the sub and sup of one `mmultiscripts` pair attached, skipping an absent one.
fn scripted(base: MmlNode, sub: Option<MmlNode>, sup: Option<MmlNode>) -> MmlNode {
    let base = Box::new(base);
    match (sub, sup) {
        (Some(sub), Some(sup)) => MmlNode::SubSup {
            base,
            sub: Box::new(sub),
            sup: Box::new(sup),
        },
        (Some(sub), None) => MmlNode::Sub {
            base,
            sub: Box::new(sub),
        },
        (None, Some(sup)) => MmlNode::Sup {
            base,
            sup: Box::new(sup),
        },
        (None, None) => *base,
    }
}

/// Collect the direct text content of a leaf element (`mi`/`mn`/`mo`/`ms`/`mtext`).
fn collect_text(node: Node, budget: &mut SecurityBudget) -> Result<String, SecurityError> {
    let mut text = String::new();
    // Only real text nodes — `Node::text()` also returns content for comment
    // nodes, and MathML fixtures commonly annotate entities with a comment
    // (e.g. `<mo>&#x222B;<!-- ∫ --></mo>`) that must not be double-counted. ~keep
    for child in node.children().filter(|c| c.is_text()) {
        if let Some(t) = child.text() {
            budget.check_entity(t)?;
            budget.account_text(t.len())?;
            text.extend(t.chars().filter(|c| !is_private_use(*c)));
        }
    }
    Ok(text)
}

/// Report whether `c` sits in a Unicode private use area.
///
/// A private use codepoint carries no meaning outside the font that defines it.
/// OpenOffice writes its stretchy fences that way, and a renderer shows a
/// missing glyph or rejects the formula, so the character is dropped rather
/// than passed through as LaTeX.
fn is_private_use(c: char) -> bool {
    matches!(c, '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}')
}

/// Collect an `mfenced` element: `open`/`close`/`separators` attributes plus
/// one element per fenced argument.
fn collect_fenced(node: Node, budget: &mut SecurityBudget) -> Result<MmlNode, SecurityError> {
    // A fence may be a private use codepoint, which OpenOffice writes for the
    // bracket shapes of its own font. It has no meaning to a renderer, so it is
    // dropped the way it is in element text.
    let strip = |value: &str| -> String { value.chars().filter(|c| !is_private_use(*c)).collect() };
    let open = node.attribute("open").map(strip).unwrap_or_else(|| "(".to_string());
    let close = node.attribute("close").map(strip).unwrap_or_else(|| ")".to_string());
    let sep = node
        .attribute("separators")
        .map(strip)
        .and_then(|s| s.chars().next())
        .unwrap_or(',')
        .to_string();

    let elements = node
        .children()
        .filter(|c| c.is_element())
        .map(|c| collect_node(c, budget))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MmlNode::Fenced {
        open,
        close,
        sep,
        elements,
    })
}

/// Collect an `mtable` element into rows of cells (`mtr` > `mtd`).
fn collect_table(node: Node, budget: &mut SecurityBudget) -> Result<MmlNode, SecurityError> {
    let mut rows = Vec::new();
    for row in node
        .children()
        .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("mtr"))
    {
        budget.step()?;
        let cells = row
            .children()
            .filter(|c| c.is_element() && c.tag_name().name().eq_ignore_ascii_case("mtd"))
            .map(|c| {
                Ok(MmlNode::Group {
                    children: collect_children(c, budget)?,
                })
            })
            .collect::<Result<Vec<_>, SecurityError>>()?;
        rows.push(cells);
    }
    Ok(MmlNode::Table { rows })
}

mod render;
use render::{render_node, render_nodes};

#[cfg(test)]
mod tests;
