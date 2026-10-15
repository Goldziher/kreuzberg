//! Shared Unicode-to-LaTeX symbol mapping for math converters.
//!
//! Both the OMML (`docx::math`) and MathML (`mathml`) converters render run/text
//! nodes through the same table of Unicode math symbols (Greek letters, operators,
//! arrows, set notation, etc.) so a given symbol always maps to the same LaTeX
//! command regardless of which source format it came from.

use std::borrow::Cow;

/// Render run text, mapping Unicode math symbols to LaTeX commands and
/// escaping TeX structural characters.
///
/// Source text can carry literal `{`, `}`, `&`, `\` and friends (e.g. a
/// stretchy `<mo>{</mo>` cases brace, or a set-difference backslash in
/// `<mi>`). Passed through raw they change the LaTeX *structure* — an
/// unpaired brace makes the whole formula unparseable. Escaped they render
/// as the glyphs the source displayed.
pub(crate) fn render_run_text(text: &str, out: &mut String) {
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        // A combining accent applies to the character before it. Sources write
        // `U̅` (U + combining overline) inside identifiers; KaTeX rejects the
        // raw mark, so fold the pair into the accent macro.
        if let Some(&next) = chars.peek()
            && let Some(cmd) = combining_accent_to_latex(next)
        {
            chars.next();
            out.push_str(cmd);
            out.push('{');
            push_mapped_char(ch, out);
            out.push('}');
            continue;
        }
        if let Some(cmd) = combining_accent_to_latex(ch) {
            // A mark with no base in this run applies to whatever the output
            // currently ends with (the source split base and mark across
            // elements, e.g. `<mi>Σ</mi><mo>̅</mo>`).
            wrap_trailing_atom(cmd, out);
            continue;
        }
        push_mapped_char(ch, out);
    }
}

/// Map one char through the symbol table / structural escapes onto `out`.
fn push_mapped_char(ch: char, out: &mut String) {
    if let Some(latex) = unicode_to_latex(ch) {
        out.push_str(latex);
    } else if let Some(escaped) = escape_tex_structural(ch) {
        out.push_str(escaped);
    } else {
        out.push(ch);
    }
}

/// Map a combining accent character to its LaTeX accent macro.
fn combining_accent_to_latex(ch: char) -> Option<&'static str> {
    match ch {
        '\u{0302}' => Some("\\hat"),
        '\u{0303}' => Some("\\tilde"),
        '\u{0304}' | '\u{0305}' => Some("\\bar"),
        '\u{0306}' => Some("\\breve"),
        '\u{0307}' => Some("\\dot"),
        '\u{0308}' => Some("\\ddot"),
        '\u{030A}' => Some("\\mathring"),
        '\u{030C}' => Some("\\check"),
        '\u{0332}' => Some("\\underline"),
        '\u{20D7}' => Some("\\vec"),
        _ => None,
    }
}

/// Wrap the trailing atom of `out` (one trailing LaTeX command, else one
/// trailing char) in `cmd{...}`. No-op on empty output.
fn wrap_trailing_atom(cmd: &str, out: &mut String) {
    let trimmed_len = out.trim_end().len();
    let trailing_ws = out.split_off(trimmed_len);
    let atom_start = match out.rfind('\\') {
        Some(i) if out[i + 1..].chars().all(|c| c.is_ascii_alphabetic()) && i + 1 < out.len() => i,
        _ => match out.chars().last() {
            Some(last) => out.len() - last.len_utf8(),
            None => {
                out.push_str(&trailing_ws);
                return;
            }
        },
    };
    // Never wrap a structural character: losing the accent is better than
    // producing an unbalanced group.
    if matches!(&out[atom_start..], "{" | "}" | "\\" | "^" | "_" | "&") {
        out.push_str(&trailing_ws);
        return;
    }
    let atom = out.split_off(atom_start);
    out.push_str(cmd);
    out.push('{');
    out.push_str(&atom);
    out.push('}');
    out.push_str(&trailing_ws);
}

/// Escape a TeX structural character appearing as literal content.
pub(crate) fn escape_tex_structural(ch: char) -> Option<&'static str> {
    match ch {
        '{' => Some("\\{"),
        '}' => Some("\\}"),
        '&' => Some("\\&"),
        '%' => Some("\\%"),
        '#' => Some("\\#"),
        '$' => Some("\\$"),
        '_' => Some("\\_"),
        '\\' => Some("\\backslash "),
        _ => None,
    }
}

/// Map a Unicode character to its LaTeX command (if any).
///
/// Delegates to one table per symbol family (Greek letters, operators, relations, arrows,
/// calculus/logic/set notation, letter-like/delimiter forms, big operators) so no single match
/// grows unwieldy; the families are disjoint over `char`, so lookup order does not affect the
/// result. ~keep
pub(crate) fn unicode_to_latex(ch: char) -> Option<&'static str> {
    greek_letter_to_latex(ch)
        .or_else(|| operator_and_relation_to_latex(ch))
        .or_else(|| membership_and_arrow_to_latex(ch))
        .or_else(|| calculus_logic_and_set_to_latex(ch))
        .or_else(|| letter_like_and_delimiter_to_latex(ch))
        .or_else(|| big_operator_to_latex(ch))
}

/// Greek letters (lowercase, uppercase, and the upsilon-with-hook variant `ϒ`).
fn greek_letter_to_latex(ch: char) -> Option<&'static str> {
    match ch {
        '\u{03B1}' => Some("\\alpha "),
        '\u{03B2}' => Some("\\beta "),
        '\u{03B3}' => Some("\\gamma "),
        '\u{03B4}' => Some("\\delta "),
        '\u{03B5}' => Some("\\epsilon "),
        '\u{03B6}' => Some("\\zeta "),
        '\u{03B7}' => Some("\\eta "),
        '\u{03B8}' => Some("\\theta "),
        '\u{03B9}' => Some("\\iota "),
        '\u{03BA}' => Some("\\kappa "),
        '\u{03BB}' => Some("\\lambda "),
        '\u{03BC}' => Some("\\mu "),
        '\u{03BD}' => Some("\\nu "),
        '\u{03BE}' => Some("\\xi "),
        '\u{03BF}' => Some("o"),
        '\u{03C0}' => Some("\\pi "),
        '\u{03C1}' => Some("\\rho "),
        '\u{03C2}' => Some("\\varsigma "),
        '\u{03C3}' => Some("\\sigma "),
        '\u{03C4}' => Some("\\tau "),
        '\u{03C5}' => Some("\\upsilon "),
        '\u{03C6}' => Some("\\phi "),
        '\u{03C7}' => Some("\\chi "),
        '\u{03C8}' => Some("\\psi "),
        '\u{03C9}' => Some("\\omega "),
        '\u{0393}' => Some("\\Gamma "),
        '\u{0394}' => Some("\\Delta "),
        '\u{0398}' => Some("\\Theta "),
        '\u{039B}' => Some("\\Lambda "),
        '\u{039E}' => Some("\\Xi "),
        '\u{03A0}' => Some("\\Pi "),
        '\u{03A3}' => Some("\\Sigma "),
        '\u{03A5}' => Some("\\Upsilon "),
        '\u{03A6}' => Some("\\Phi "),
        '\u{03A8}' => Some("\\Psi "),
        '\u{03A9}' => Some("\\Omega "),
        '\u{03D2}' => Some("\\Upsilon "),
        _ => None,
    }
}

/// Arithmetic/comparison operators and set-inclusion relations.
fn operator_and_relation_to_latex(ch: char) -> Option<&'static str> {
    match ch {
        '\u{00B1}' => Some("\\pm "),
        '\u{2213}' => Some("\\mp "),
        '\u{00D7}' => Some("\\times "),
        '\u{00F7}' => Some("\\div "),
        '\u{22C5}' => Some("\\cdot "),
        '\u{2217}' => Some("\\ast "),
        '\u{2218}' => Some("\\circ "),
        '\u{2219}' => Some("\\bullet "),
        '\u{2264}' => Some("\\leq "),
        '\u{2265}' => Some("\\geq "),
        '\u{2260}' => Some("\\neq "),
        '\u{2248}' => Some("\\approx "),
        '\u{2261}' => Some("\\equiv "),
        '\u{227A}' => Some("\\prec "),
        '\u{227B}' => Some("\\succ "),
        '\u{2286}' => Some("\\subseteq "),
        '\u{2287}' => Some("\\supseteq "),
        '\u{2282}' => Some("\\subset "),
        '\u{2283}' => Some("\\supset "),
        _ => None,
    }
}

/// Set-membership symbols and directional arrows.
fn membership_and_arrow_to_latex(ch: char) -> Option<&'static str> {
    match ch {
        '\u{2208}' => Some("\\in "),
        '\u{2209}' => Some("\\notin "),
        '\u{220B}' => Some("\\ni "),
        '\u{2190}' => Some("\\leftarrow "),
        '\u{2192}' => Some("\\rightarrow "),
        '\u{2191}' => Some("\\uparrow "),
        '\u{2193}' => Some("\\downarrow "),
        '\u{2194}' => Some("\\leftrightarrow "),
        '\u{21D0}' => Some("\\Leftarrow "),
        '\u{21D2}' => Some("\\Rightarrow "),
        '\u{21D4}' => Some("\\Leftrightarrow "),
        '\u{21A6}' => Some("\\mapsto "),
        _ => None,
    }
}

/// Calculus operators, quantifiers/logic connectives, set operations, and ellipses.
fn calculus_logic_and_set_to_latex(ch: char) -> Option<&'static str> {
    match ch {
        '\u{221E}' => Some("\\infty "),
        '\u{2202}' => Some("\\partial "),
        '\u{2207}' => Some("\\nabla "),
        '\u{2200}' => Some("\\forall "),
        '\u{2203}' => Some("\\exists "),
        '\u{2205}' => Some("\\emptyset "),
        '\u{2227}' => Some("\\wedge "),
        '\u{2228}' => Some("\\vee "),
        '\u{00AC}' => Some("\\neg "),
        '\u{2229}' => Some("\\cap "),
        '\u{222A}' => Some("\\cup "),
        '\u{2026}' => Some("\\ldots "),
        '\u{22EF}' => Some("\\cdots "),
        '\u{22EE}' => Some("\\vdots "),
        '\u{22F1}' => Some("\\ddots "),
        _ => None,
    }
}

/// Prime marks, letter-like symbols (ℏ, ℓ, ℜ, ℑ, ℘, ℵ), and paired delimiters.
fn letter_like_and_delimiter_to_latex(ch: char) -> Option<&'static str> {
    match ch {
        '\u{2032}' => Some("'"),
        '\u{2033}' => Some("''"),
        '\u{210F}' => Some("\\hbar "),
        '\u{2113}' => Some("\\ell "),
        '\u{211C}' => Some("\\Re "),
        '\u{2111}' => Some("\\Im "),
        '\u{2118}' => Some("\\wp "),
        '\u{2135}' => Some("\\aleph "),
        '\u{2016}' | '\u{2225}' => Some("\\Vert "),
        '\u{2223}' => Some("\\mid "),
        '\u{2329}' | '\u{27E8}' => Some("\\langle "),
        '\u{232A}' | '\u{27E9}' => Some("\\rangle "),
        '\u{204E}' => Some("\\ast "),
        _ => None,
    }
}

/// The LaTeX command of a named operator (`sin`, `lim`, `det`), and whether it takes its under and over
/// scripts as limits (`\lim_{n}`). Whitespace is ignored, since producers write `\liminf` as `lim inf`.
pub(crate) fn operator_name_to_latex(name: &str) -> Option<(&'static str, bool)> {
    let name: Cow<'_, str> = if name.chars().any(char::is_whitespace) {
        Cow::Owned(name.chars().filter(|c| !c.is_whitespace()).collect())
    } else {
        Cow::Borrowed(name)
    };
    let latex = match name.as_ref() {
        "lim" => ("\\lim", true),
        "liminf" => ("\\liminf", true),
        "limsup" => ("\\limsup", true),
        "max" => ("\\max", true),
        "min" => ("\\min", true),
        "sup" => ("\\sup", true),
        "inf" => ("\\inf", true),
        "det" => ("\\det", true),
        "gcd" => ("\\gcd", true),
        // Only presentation MathML reaches this case-sensitive arm; content names arrive lowercased.
        "Pr" => ("\\Pr", true),
        "sin" => ("\\sin", false),
        "cos" => ("\\cos", false),
        "tan" => ("\\tan", false),
        "cot" => ("\\cot", false),
        "sec" => ("\\sec", false),
        "csc" => ("\\csc", false),
        "arcsin" => ("\\arcsin", false),
        "arccos" => ("\\arccos", false),
        "arctan" => ("\\arctan", false),
        "sinh" => ("\\sinh", false),
        "cosh" => ("\\cosh", false),
        "tanh" => ("\\tanh", false),
        "coth" => ("\\coth", false),
        "exp" => ("\\exp", false),
        "ln" => ("\\ln", false),
        "lg" => ("\\lg", false),
        "log" => ("\\log", false),
        "deg" => ("\\deg", false),
        "dim" => ("\\dim", false),
        "hom" => ("\\hom", false),
        "ker" => ("\\ker", false),
        "arg" => ("\\arg", false),
        _ => return None,
    };
    Some(latex)
}

/// N-ary big operators (sums, products, integrals, and their "big" logic/set variants).
pub(crate) fn big_operator_to_latex(ch: char) -> Option<&'static str> {
    match ch {
        '\u{2211}' => Some("\\sum "),
        '\u{220F}' => Some("\\prod "),
        '\u{222B}' => Some("\\int "),
        '\u{222C}' => Some("\\iint "),
        '\u{222D}' => Some("\\iiint "),
        '\u{222E}' => Some("\\oint "),
        '\u{2210}' => Some("\\coprod "),
        '\u{22C0}' => Some("\\bigwedge "),
        '\u{22C1}' => Some("\\bigvee "),
        '\u{22C2}' => Some("\\bigcap "),
        '\u{22C3}' => Some("\\bigcup "),
        _ => None,
    }
}
