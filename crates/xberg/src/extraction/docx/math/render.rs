use super::*;

/// Render a slice of MathNodes to LaTeX.
pub(super) fn render_nodes(nodes: &[MathNode]) -> String {
    let mut out = String::new();
    for node in nodes {
        render_node(node, &mut out);
    }
    out
}

/// Render a single MathNode to LaTeX, appending to `out`.
fn render_node(node: &MathNode, out: &mut String) {
    match node {
        MathNode::Run(text) => render_run_text(text, out),
        MathNode::SSup { base, sup } => render_ssup(base, sup, out),
        MathNode::SSub { base, sub } => render_ssub(base, sub, out),
        MathNode::SSubSup { base, sub, sup } => render_ssubsup(base, sub, sup, out),
        MathNode::Frac { num, den, frac_type } => render_frac(num, den, frac_type, out),
        MathNode::Rad { deg, body, deg_hide } => render_rad(deg, body, *deg_hide, out),
        MathNode::Nary {
            chr,
            sub,
            sup,
            body,
            sub_hide,
            sup_hide,
        } => render_nary(chr, sub, sup, body, (*sub_hide, *sup_hide), out),
        MathNode::Delim {
            begin_chr,
            end_chr,
            sep_chr,
            elements,
        } => render_delim(begin_chr, end_chr, sep_chr, elements, out),
        MathNode::Func { name, body } => render_func(name, body, out),
        MathNode::Acc { chr, body } => render_acc(chr, body, out),
        MathNode::EqArr { rows } => render_eqarr(rows, out),
        MathNode::LimLow { body, lim } => render_limlow(body, lim, out),
        MathNode::LimUpp { body, lim } => render_limupp(body, lim, out),
        MathNode::Bar { body, top } => render_bar(body, *top, out),
        MathNode::GroupChr { chr, top, body } => render_group_chr(chr, *top, body, out),
        MathNode::BorderBox { body } => render_borderbox(body, out),
        MathNode::Matrix { rows } => render_matrix(rows, out),
        MathNode::Group { children } => out.push_str(&render_nodes(children)),
        MathNode::SPre { base, sub, sup } => render_spre(base, sub, sup, out),
    }
}

fn render_ssup(base: &[MathNode], sup: &[MathNode], out: &mut String) {
    render_group(base, out);
    out.push_str("^{");
    out.push_str(&render_nodes(sup));
    out.push('}');
}

fn render_ssub(base: &[MathNode], sub: &[MathNode], out: &mut String) {
    render_group(base, out);
    out.push_str("_{");
    out.push_str(&render_nodes(sub));
    out.push('}');
}

fn render_ssubsup(base: &[MathNode], sub: &[MathNode], sup: &[MathNode], out: &mut String) {
    render_group(base, out);
    out.push_str("_{");
    out.push_str(&render_nodes(sub));
    out.push_str("}^{");
    out.push_str(&render_nodes(sup));
    out.push('}');
}

fn render_frac(num: &[MathNode], den: &[MathNode], frac_type: &FracType, out: &mut String) {
    match frac_type {
        FracType::Bar => {
            out.push_str("\\frac{");
            out.push_str(&render_nodes(num));
            out.push_str("}{");
            out.push_str(&render_nodes(den));
            out.push('}');
        }
        FracType::NoBar => {
            out.push_str("\\binom{");
            out.push_str(&render_nodes(num));
            out.push_str("}{");
            out.push_str(&render_nodes(den));
            out.push('}');
        }
        FracType::Linear | FracType::Skewed => {
            let num_s = render_nodes(num);
            let den_s = render_nodes(den);
            if num_s.len() > 1 {
                out.push('{');
                out.push_str(&num_s);
                out.push('}');
            } else {
                out.push_str(&num_s);
            }
            out.push('/');
            if den_s.len() > 1 {
                out.push('{');
                out.push_str(&den_s);
                out.push('}');
            } else {
                out.push_str(&den_s);
            }
        }
    }
}

fn render_rad(deg: &[MathNode], body: &[MathNode], deg_hide: bool, out: &mut String) {
    out.push_str("\\sqrt");
    if !deg_hide && !deg.is_empty() {
        let deg_s = render_nodes(deg);
        if !deg_s.is_empty() {
            out.push('[');
            out.push_str(&deg_s);
            out.push(']');
        }
    }
    out.push('{');
    out.push_str(&render_nodes(body));
    out.push('}');
}

/// Render an `m:nary` node. `hide` is `(sub_hide, sup_hide)`, bundled purely to keep this
/// function's parameter list manageable.
fn render_nary(chr: &str, sub: &[MathNode], sup: &[MathNode], body: &[MathNode], hide: (bool, bool), out: &mut String) {
    let (sub_hide, sup_hide) = hide;
    out.push_str(&nary_chr_to_latex(chr));
    if !sub_hide && !sub.is_empty() {
        out.push_str("_{");
        out.push_str(&render_nodes(sub));
        out.push('}');
    }
    if !sup_hide && !sup.is_empty() {
        out.push_str("^{");
        out.push_str(&render_nodes(sup));
        out.push('}');
    }
    if !body.is_empty() {
        out.push('{');
        out.push_str(&render_nodes(body));
        out.push('}');
    }
}

fn render_delim(begin_chr: &str, end_chr: &str, sep_chr: &str, elements: &[Vec<MathNode>], out: &mut String) {
    out.push_str("\\left");
    out.push_str(&delim_chr_to_latex(begin_chr));
    for (i, elem) in elements.iter().enumerate() {
        if i > 0 {
            out.push_str(&delim_sep_to_latex(sep_chr));
        }
        out.push_str(&render_nodes(elem));
    }
    out.push_str("\\right");
    out.push_str(&delim_chr_to_latex(end_chr));
}

fn render_func(name: &[MathNode], body: &[MathNode], out: &mut String) {
    let func_name = render_nodes(name);
    let latex_func = match func_name.trim() {
        "sin" => "\\sin",
        "cos" => "\\cos",
        "tan" => "\\tan",
        "cot" => "\\cot",
        "sec" => "\\sec",
        "csc" => "\\csc",
        "log" => "\\log",
        "ln" => "\\ln",
        "exp" => "\\exp",
        "lim" => "\\lim",
        "max" => "\\max",
        "min" => "\\min",
        "sup" => "\\sup",
        "inf" => "\\inf",
        "det" => "\\det",
        "gcd" => "\\gcd",
        "deg" => "\\deg",
        "dim" => "\\dim",
        "hom" => "\\hom",
        "ker" => "\\ker",
        "arg" => "\\arg",
        "sinh" => "\\sinh",
        "cosh" => "\\cosh",
        "tanh" => "\\tanh",
        _ => "",
    };
    if !latex_func.is_empty() {
        out.push_str(latex_func);
    } else {
        out.push_str("\\mathrm{");
        out.push_str(&func_name);
        out.push('}');
    }
    out.push('{');
    out.push_str(&render_nodes(body));
    out.push('}');
}

fn render_acc(chr: &str, body: &[MathNode], out: &mut String) {
    out.push_str(&accent_chr_to_latex(chr));
    out.push('{');
    out.push_str(&render_nodes(body));
    out.push('}');
}

fn render_eqarr(rows: &[Vec<MathNode>], out: &mut String) {
    out.push_str("\\begin{aligned}");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push_str(" \\\\ ");
        }
        out.push_str(&render_nodes(row));
    }
    out.push_str("\\end{aligned}");
}

fn render_limlow(body: &[MathNode], lim: &[MathNode], out: &mut String) {
    out.push_str("\\underset{");
    out.push_str(&render_nodes(lim));
    out.push_str("}{");
    out.push_str(&render_nodes(body));
    out.push('}');
}

fn render_limupp(body: &[MathNode], lim: &[MathNode], out: &mut String) {
    out.push_str("\\overset{");
    out.push_str(&render_nodes(lim));
    out.push_str("}{");
    out.push_str(&render_nodes(body));
    out.push('}');
}

fn render_bar(body: &[MathNode], top: bool, out: &mut String) {
    if top {
        out.push_str("\\overline{");
    } else {
        out.push_str("\\underline{");
    }
    out.push_str(&render_nodes(body));
    out.push('}');
}

fn render_group_chr(chr: &str, top: bool, body: &[MathNode], out: &mut String) {
    match chr {
        "\u{23DE}" | "\u{23DF}" => out.push_str(if top { "\\overbrace{" } else { "\\underbrace{" }),
        _ => {
            out.push_str(if top { "\\overset{" } else { "\\underset{" });
            render_run_text(chr, out);
            out.push_str("}{");
        }
    }
    out.push_str(&render_nodes(body));
    out.push('}');
}

fn render_borderbox(body: &[MathNode], out: &mut String) {
    out.push_str("\\boxed{");
    out.push_str(&render_nodes(body));
    out.push('}');
}

fn render_matrix(rows: &[Vec<Vec<MathNode>>], out: &mut String) {
    out.push_str("\\begin{matrix}");
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            out.push_str(" \\\\ ");
        }
        for (j, cell) in row.iter().enumerate() {
            if j > 0 {
                out.push_str(" & ");
            }
            out.push_str(&render_nodes(cell));
        }
    }
    out.push_str("\\end{matrix}");
}

fn render_spre(base: &[MathNode], sub: &[MathNode], sup: &[MathNode], out: &mut String) {
    out.push_str("{}_{");
    out.push_str(&render_nodes(sub));
    out.push_str("}^{");
    out.push_str(&render_nodes(sup));
    out.push('}');
    render_group(base, out);
}

/// Render base nodes, wrapping in braces if needed for subscript/superscript.
fn render_group(nodes: &[MathNode], out: &mut String) {
    let rendered = render_nodes(nodes);
    let needs_braces = rendered.chars().count() > 1 && !rendered.starts_with('\\') && !rendered.starts_with('{');
    if needs_braces {
        out.push('{');
        out.push_str(&rendered);
        out.push('}');
    } else {
        out.push_str(&rendered);
    }
}

/// Map n-ary character to LaTeX command.
fn nary_chr_to_latex(chr: &str) -> String {
    if let Some(ch) = chr.chars().next() {
        match ch {
            '\u{2211}' => return "\\sum".to_string(),
            '\u{220F}' => return "\\prod".to_string(),
            '\u{2210}' => return "\\coprod".to_string(),
            '\u{222B}' => return "\\int".to_string(),
            '\u{222C}' => return "\\iint".to_string(),
            '\u{222D}' => return "\\iiint".to_string(),
            '\u{222E}' => return "\\oint".to_string(),
            '\u{22C0}' => return "\\bigwedge".to_string(),
            '\u{22C1}' => return "\\bigvee".to_string(),
            '\u{22C2}' => return "\\bigcap".to_string(),
            '\u{22C3}' => return "\\bigcup".to_string(),
            _ => {}
        }
    }
    chr.to_string()
}

/// Map delimiter character to LaTeX.
fn delim_chr_to_latex(chr: &str) -> String {
    match chr {
        "(" | ")" | "[" | "]" => chr.to_string(),
        "{" => "\\{".to_string(),
        "}" => "\\}".to_string(),
        "|" => "|".to_string(),
        "\u{2016}" => "\\|".to_string(),
        "\u{2329}" | "\u{27E8}" => "\\langle".to_string(),
        "\u{232A}" | "\u{27E9}" => "\\rangle".to_string(),
        "\u{230A}" => "\\lfloor".to_string(),
        "\u{230B}" => "\\rfloor".to_string(),
        "\u{2308}" => "\\lceil".to_string(),
        "\u{2309}" => "\\rceil".to_string(),
        "" => ".".to_string(),
        _ => chr.to_string(),
    }
}

/// Map delimiter separator character to LaTeX.
fn delim_sep_to_latex(sep: &str) -> String {
    match sep {
        "|" => " \\mid ".to_string(),
        _ => sep.to_string(),
    }
}

/// Map accent character to LaTeX command.
fn accent_chr_to_latex(chr: &str) -> String {
    if let Some(ch) = chr.chars().next() {
        match ch {
            '\u{0302}' | '^' => return "\\hat".to_string(),
            '\u{0303}' | '~' => return "\\tilde".to_string(),
            '\u{0304}' | '\u{0305}' => return "\\bar".to_string(),
            '\u{20D7}' | '\u{2192}' => return "\\vec".to_string(),
            '\u{0307}' => return "\\dot".to_string(),
            '\u{0308}' => return "\\ddot".to_string(),
            '\u{030C}' => return "\\check".to_string(),
            '\u{0306}' => return "\\breve".to_string(),
            '\u{0301}' => return "\\acute".to_string(),
            '\u{0300}' => return "\\grave".to_string(),
            _ => {}
        }
    }
    "\\hat".to_string()
}
