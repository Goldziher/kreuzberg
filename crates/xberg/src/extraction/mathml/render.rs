//! LaTeX rendering for a collected [`super::MmlNode`] tree: the second half of MathML
//! conversion, split out from `mathml.rs` once the collection phase (XML subtree ->
//! `MmlNode`) and this rendering phase (`MmlNode` -> LaTeX string) together made the parent
//! file too long. Only [`render_nodes`] and [`render_node`] are called from outside this
//! module; everything else here is a private helper of the rendering phase.

use super::MmlNode;
use crate::extraction::math_symbols::{big_operator_to_latex, operator_name_to_latex, render_run_text};

/// Render a slice of `MmlNode`s to LaTeX, concatenated with no separators.
pub(super) fn render_nodes(nodes: &[MmlNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        render_node(node, &mut out);
    }
    out
}

/// Render a single `MmlNode` to LaTeX, appending to `out`.
pub(super) fn render_node(node: &MmlNode, out: &mut String) {
    match node {
        MmlNode::Verbatim(latex) => out.push_str(latex),
        MmlNode::Run(text) => render_run_text(text, out),
        MmlNode::Text(text) => render_text_content(text, out),
        MmlNode::Space => out.push(' '),
        MmlNode::Frac { num, den } => {
            out.push_str("\\frac{");
            render_node(num, out);
            out.push_str("}{");
            render_node(den, out);
            out.push('}');
        }
        MmlNode::Sup { base, sup } => {
            render_arg(base, out);
            out.push_str("^{");
            render_node(sup, out);
            out.push('}');
        }
        MmlNode::Sub { base, sub } => {
            render_arg(base, out);
            out.push_str("_{");
            render_node(sub, out);
            out.push('}');
        }
        MmlNode::SubSup { base, sub, sup } => {
            render_arg(base, out);
            out.push_str("_{");
            render_node(sub, out);
            out.push_str("}^{");
            render_node(sup, out);
            out.push('}');
        }
        MmlNode::Sqrt { body } => {
            out.push_str("\\sqrt{");
            render_node(body, out);
            out.push('}');
        }
        MmlNode::Root { body, index } => {
            out.push_str("\\sqrt[");
            render_node(index, out);
            out.push_str("]{");
            render_node(body, out);
            out.push('}');
        }
        MmlNode::Fenced {
            open,
            close,
            sep,
            elements,
        } => render_fenced(open, close, sep, elements, out),
        MmlNode::Under {
            base,
            under,
            stacked,
            no_accentunder,
        } => render_under(base, under, *stacked, *no_accentunder, out),
        MmlNode::Over {
            base,
            over,
            stacked,
            no_accent,
        } => render_over(base, over, *stacked, *no_accent, out),
        MmlNode::UnderOver {
            base,
            under,
            over,
            stacked,
            no_accent,
            no_accentunder,
        } => render_under_over(base, (under, over), *stacked, *no_accent, *no_accentunder, out),
        MmlNode::Phantom { body } => {
            out.push_str("\\phantom{");
            render_node(body, out);
            out.push('}');
        }
        MmlNode::Table { rows } => render_table(rows, out),
        MmlNode::Group { children } => out.push_str(&render_nodes(children)),
    }
}

/// Render an underscript: a recognized under-script command (e.g. `\underline`) applied directly unless
/// `no_accentunder`, a limit under a big operator or `lim`, or `\underset{under}{base}` otherwise.
fn render_under(base: &MmlNode, under: &MmlNode, stacked: bool, no_accentunder: bool, out: &mut String) {
    if let Some(cmd) = under_script_command(under).filter(|_| !no_accentunder) {
        out.push_str(cmd);
        out.push('{');
        render_node(base, out);
        out.push('}');
        return;
    }
    if let Some(op) = limits_base(base) {
        push_limits(op, stacked, Some(under), None, out);
        return;
    }
    out.push_str("\\underset{");
    push_script(under, out);
    out.push_str("}{");
    render_node(base, out);
    out.push('}');
}

/// Render an overscript: a recognized over-script command (e.g. `\vec`, `\hat`) applied directly unless
/// `no_accent`, a limit over a big operator or `lim`, or `\overset{over}{base}` otherwise.
fn render_over(base: &MmlNode, over: &MmlNode, stacked: bool, no_accent: bool, out: &mut String) {
    // Rendered once and reused: the accent choice reads it, and rendering it again there made nested
    // accents cost 2^depth.
    let base_latex = rendered(base);
    if let Some(cmd) = over_script_command(over, !is_single_glyph(&base_latex)).filter(|_| !no_accent) {
        out.push_str(cmd);
        out.push('{');
        out.push_str(&base_latex);
        out.push('}');
        return;
    }
    if let Some(op) = limits_base(base) {
        push_limits(op, stacked, None, Some(over), out);
        return;
    }
    out.push_str("\\overset{");
    push_script(over, out);
    out.push_str("}{");
    out.push_str(&base_latex);
    out.push('}');
}

/// Render a combined under+overscript as limits on a big operator or `lim`, or as
/// `\overset{over}{\underset{under}{base}}` with an accent script applied as its command instead. An accent
/// in either script rules out limits, since it is decoration.
fn render_under_over(
    base: &MmlNode,
    (under, over): (&MmlNode, &MmlNode),
    stacked: bool,
    no_accent: bool,
    no_accentunder: bool,
    out: &mut String,
) {
    let base_latex = rendered(base);
    let under_cmd = under_script_command(under).filter(|_| !no_accentunder);
    // Over an `\underset` stack the accent spans more than one glyph, so it takes the wide form.
    let wide = under_cmd.is_none() || !is_single_glyph(&base_latex);
    let over_cmd = over_script_command(over, wide).filter(|_| !no_accent);
    if under_cmd.is_none()
        && over_cmd.is_none()
        && let Some(op) = limits_base(base)
    {
        push_limits(op, stacked, Some(under), Some(over), out);
        return;
    }
    match over_cmd {
        Some(cmd) => out.push_str(cmd),
        None => {
            out.push_str("\\overset{");
            push_script(over, out);
            out.push('}');
        }
    }
    out.push('{');
    match under_cmd {
        Some(cmd) => out.push_str(cmd),
        None => {
            out.push_str("\\underset{");
            push_script(under, out);
            out.push('}');
        }
    }
    out.push('{');
    out.push_str(&base_latex);
    out.push_str("}}");
}

/// Write `op_{under}^{over}`, leaving out an absent script. A `stacked` operator (`movablelimits="false"`)
/// keeps its limits stacked inline too, which TeX needs `\limits` for.
fn push_limits(op: &str, stacked: bool, under: Option<&MmlNode>, over: Option<&MmlNode>, out: &mut String) {
    out.push_str(op);
    if stacked && !op.ends_with("\\limits") {
        out.push_str("\\limits");
    }
    if let Some(under) = under {
        out.push_str("_{");
        push_script(under, out);
        out.push('}');
    }
    if let Some(over) = over {
        out.push_str("^{");
        push_script(over, out);
        out.push('}');
    }
}

/// Render an under or over script that is not applied as an accent, writing an accent-like character as its
/// visible mark.
fn push_script(script: &MmlNode, out: &mut String) {
    match script_mark(script).and_then(|mark| mark.standalone) {
        Some(standalone) => out.push_str(standalone),
        None => render_node(script, out),
    }
}

/// Render an `mfenced` node's open/close delimiters and separator-joined elements.
///
/// Authors use `mfenced` as plain grouping with operators as direct children; inserting the
/// spec-default comma separators there turns `(1 - x)` into `(1,-,x)`. Suppress separators when
/// any child is itself an infix operator.
fn render_fenced(open: &str, close: &str, sep: &str, elements: &[MmlNode], out: &mut String) {
    let sep = if elements.iter().any(is_operator_child) {
        ""
    } else {
        sep
    };
    let (left, right) = (fence_chr_to_latex(open), fence_chr_to_latex(close));
    match (left, right) {
        (Some(left), Some(right)) => {
            out.push_str("\\left");
            out.push_str(left);
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep);
                }
                render_node(elem, out);
            }
            out.push_str("\\right");
            out.push_str(right);
        }
        // A fence char LaTeX cannot use after `\left`: emit the fences
        // as plain glyphs instead of producing an unparseable string.
        _ => {
            render_run_text(open, out);
            for (i, elem) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(sep);
                }
                render_node(elem, out);
            }
            render_run_text(close, out);
        }
    }
}

/// Render an `mtable` node as a LaTeX `matrix` environment, rows separated by `\\` and cells by
/// `&`.
fn render_table(rows: &[Vec<MmlNode>], out: &mut String) {
    out.push_str("\\begin{matrix}");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push_str(" \\\\ ");
        }
        for (j, cell) in row.iter().enumerate() {
            if j > 0 {
                out.push_str(" & ");
            }
            render_node(cell, out);
        }
    }
    out.push_str("\\end{matrix}");
}

/// True when `s` is exactly one balanced brace group (`{...}`): the opening
/// brace's closer is the final character. `{a}^{b}` starts with `{` and ends
/// with `}` but is two atoms — treating it as pre-braced produces double
/// scripts when another script attaches.
fn is_single_brace_group(s: &str) -> bool {
    if !s.starts_with('{') || !s.ends_with('}') {
        return false;
    }
    let mut depth = 0usize;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return i == s.len() - 1;
                }
            }
            _ => {}
        }
    }
    false
}

/// Render `mtext` content. Plain text goes inside `\text{...}` with text-mode
/// escaping; characters that map to math commands (Greek letters, operators)
/// are emitted *outside* the `\text` group, because commands like `\Delta` are
/// math-mode-only and fail inside `\text{}`.
fn render_text_content(text: &str, out: &mut String) {
    let mut in_text = false;
    for ch in text.chars() {
        if let Some(latex) = crate::extraction::math_symbols::unicode_to_latex(ch) {
            if in_text {
                out.push('}');
                in_text = false;
            }
            out.push_str(latex);
            continue;
        }
        if !in_text {
            out.push_str("\\text{");
            in_text = true;
        }
        match ch {
            '{' => out.push_str("\\{"),
            '}' => out.push_str("\\}"),
            '&' => out.push_str("\\&"),
            '%' => out.push_str("\\%"),
            '#' => out.push_str("\\#"),
            '$' => out.push_str("\\$"),
            '_' => out.push_str("\\_"),
            '\\' => out.push_str("\\textbackslash "),
            '^' => out.push_str("\\textasciicircum "),
            '~' => out.push_str("\\textasciitilde "),
            _ => out.push(ch),
        }
    }
    if in_text {
        out.push('}');
    }
}

/// The LaTeX of a base whose under and over scripts are its limits: a big operator (a sum, an integral) or a
/// function name that takes limits (`lim`, `max`, `det`).
///
/// MathML's operator dictionary gives integrals no `movablelimits`, so their limits stack in every style,
/// while TeX sets `\int_{a}^{b}` to the side; the integral family therefore carries `\limits`. An `mi`
/// `lim` also stacks in every style, but is left as `\lim_{n}`, which matches it in display style.
fn limits_base(base: &MmlNode) -> Option<&'static str> {
    let text = script_leaf_text(base)?;
    if let Some((latex, true)) = operator_name_to_latex(text) {
        return Some(latex);
    }
    let mut chars = text.chars();
    let first = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    match first {
        '\u{222B}' => Some("\\int\\limits"),
        '\u{222C}' => Some("\\iint\\limits"),
        '\u{222D}' => Some("\\iiint\\limits"),
        '\u{222E}' => Some("\\oint\\limits"),
        _ => big_operator_to_latex(first).map(str::trim_end),
    }
}

/// The trimmed text of a node that is a single `mo`/`mi` leaf, possibly inside grouping, or `None` when the
/// node is anything larger. Used for both script characters (accents) and operator bases.
fn script_leaf_text(node: &MmlNode) -> Option<&str> {
    match node {
        MmlNode::Run(text) => Some(text.trim()),
        MmlNode::Group { children } if children.len() == 1 => script_leaf_text(&children[0]),
        _ => None,
    }
}

/// `node` rendered to its own string.
fn rendered(node: &MmlNode) -> String {
    let mut latex = String::new();
    render_node(node, &mut latex);
    latex
}

/// True when a rendered base is a single glyph (possibly one LaTeX command),
/// used to pick `\bar`/`\vec` over `\overline`/`\overrightarrow`.
fn is_single_glyph(base_latex: &str) -> bool {
    let t = base_latex.trim();
    t.chars().count() == 1 || (t.starts_with('\\') && t[1..].chars().all(|c| c.is_ascii_alphabetic()))
}

/// An accent-like script character: the accent it applies over and under a base, and the mark written for it
/// when it is a plain script, where a combining mark has nothing to attach to and vanishes. `standalone` is
/// `None` where the run renderer already writes the character visibly.
struct ScriptMark {
    over: Option<&'static str>,
    under: Option<&'static str>,
    standalone: Option<&'static str>,
}

/// The [`ScriptMark`] of a script that is a single accent-like character. MathML sources write accents as
/// literal spacing or combining characters (`<mover><mi>x</mi><mo>^</mo></mover>`).
fn script_mark(script: &MmlNode) -> Option<ScriptMark> {
    let (over, under, standalone) = match script_leaf_text(script)? {
        "^" => (Some("\\hat"), None, Some("\\hat{}")),
        "\u{02C6}" | "\u{0302}" => (Some("\\hat"), None, Some("\\hat{}")),
        "~" => (Some("\\tilde"), None, Some("\\sim ")),
        "\u{02DC}" | "\u{0303}" => (Some("\\tilde"), None, Some("\\tilde{}")),
        "\u{223C}" => (Some("\\tilde"), None, Some("\\sim ")),
        "\u{02D9}" | "\u{0307}" => (Some("\\dot"), None, Some("\\dot{}")),
        "\u{00A8}" | "\u{0308}" => (Some("\\ddot"), None, Some("\\ddot{}")),
        "\u{00AF}" | "\u{203E}" => (Some("\\bar"), Some("\\underline"), Some("\\bar{}")),
        "\u{0304}" | "\u{0305}" => (Some("\\bar"), None, Some("\\bar{}")),
        "\u{2192}" => (Some("\\vec"), None, None),
        "\u{20D7}" => (Some("\\vec"), None, Some("\\vec{}")),
        "\u{02C7}" | "\u{030C}" => (Some("\\check"), None, Some("\\check{}")),
        "\u{02D8}" | "\u{0306}" => (Some("\\breve"), None, Some("\\breve{}")),
        "\u{00B4}" | "\u{0301}" => (Some("\\acute"), None, Some("\\acute{}")),
        "`" | "\u{0300}" => (Some("\\grave"), None, Some("\\grave{}")),
        "\u{02DA}" | "\u{030A}" => (Some("\\mathring"), None, Some("\\mathring{}")),
        // A brace over nothing has no width, so it spans a quad to show.
        "\u{23DE}" => (Some("\\overbrace"), None, Some("\\overbrace{\\qquad}")),
        "_" => (None, Some("\\underline"), None),
        "\u{0332}" | "\u{02CD}" => (None, Some("\\underline"), Some("\\_")),
        "\u{23DF}" => (None, Some("\\underbrace"), Some("\\underbrace{\\qquad}")),
        _ => return None,
    };
    Some(ScriptMark {
        over,
        under,
        standalone,
    })
}

/// The accent an `mover` script applies, in its wide form (`\overline`, `\overrightarrow`) over more than one
/// glyph.
fn over_script_command(over: &MmlNode, wide: bool) -> Option<&'static str> {
    script_mark(over)?.over.map(|cmd| match (cmd, wide) {
        ("\\bar", true) => "\\overline",
        ("\\vec", true) => "\\overrightarrow",
        (cmd, _) => cmd,
    })
}

/// The accent an `munder` script applies, like [`over_script_command`].
fn under_script_command(under: &MmlNode) -> Option<&'static str> {
    script_mark(under)?.under
}

/// Render an argument (sup/sub base), wrapping in braces unless it is a single
/// atom (one character, one LaTeX command, or one brace group).
///
/// A compound base that already carries a script (`\lambda _{1}^{'}`) MUST be
/// wrapped, or attaching the outer script produces a double superscript. An
/// empty base (script-only markup like tensor `{}_{,\nu}`) renders as `{}` so
/// the script cannot fuse onto the preceding atom.
fn render_arg(node: &MmlNode, out: &mut String) {
    let mut rendered = String::new();
    render_node(node, &mut rendered);
    let trimmed = rendered.trim();
    if trimmed.is_empty() {
        out.push_str("{}");
        return;
    }
    let single_char = trimmed.chars().count() == 1;
    let single_command =
        trimmed.starts_with('\\') && trimmed.len() > 1 && trimmed[1..].chars().all(|c| c.is_ascii_alphabetic());
    if single_char || single_command || is_single_brace_group(trimmed) {
        out.push_str(&rendered);
    } else {
        out.push('{');
        out.push_str(trimmed);
        out.push('}');
    }
}

/// True when a fenced child renders to a bare infix operator, meaning the
/// `mfenced` is grouping an expression, not listing arguments.
fn is_operator_child(node: &MmlNode) -> bool {
    let MmlNode::Run(text) = node else { return false };
    matches!(
        text.trim(),
        "+" | "-" | "\u{2212}" | "=" | "\u{00B1}" | "\u{00D7}" | "\u{22C5}" | "/" | "<" | ">" | "\u{2264}" | "\u{2265}"
    )
}

/// Map an `mfenced` open/close character to a LaTeX delimiter valid after
/// `\left`/`\right`, or `None` for characters LaTeX cannot use there.
/// Word-form commands carry a trailing space so following content never glues
/// onto the control word (`\langle A`, not `\langleA`).
fn fence_chr_to_latex(chr: &str) -> Option<&'static str> {
    match chr {
        "(" => Some("("),
        ")" => Some(")"),
        "[" => Some("["),
        "]" => Some("]"),
        "{" => Some("\\{"),
        "}" => Some("\\}"),
        "|" | "\u{2223}" => Some("|"),
        "\u{2016}" | "\u{2225}" => Some("\\|"),
        "\u{2329}" | "\u{27E8}" => Some("\\langle "),
        "\u{232A}" | "\u{27E9}" => Some("\\rangle "),
        "\u{230A}" => Some("\\lfloor "),
        "\u{230B}" => Some("\\rfloor "),
        "\u{2308}" => Some("\\lceil "),
        "\u{2309}" => Some("\\rceil "),
        "/" => Some("/"),
        "\\" => Some("\\backslash "),
        "" => Some("."),
        _ => None,
    }
}
