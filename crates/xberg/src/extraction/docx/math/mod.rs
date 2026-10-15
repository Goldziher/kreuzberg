//! OMML (Office Math Markup Language) to LaTeX converter.
//!
//! Converts OMML math elements found in DOCX files to LaTeX notation.
//! When the streaming parser encounters `m:oMathPara` or `m:oMath`, it
//! delegates here. We collect the subtree into a `MathNode` tree, then
//! recursively render it to LaTeX.

use crate::extraction::math_symbols::render_run_text;
use crate::extractors::security::{SecurityBudget, SecurityError};
use quick_xml::Reader;
use quick_xml::events::Event;

#[derive(Debug, Clone)]
enum FracType {
    Bar,
    NoBar,
    Linear,
    Skewed,
}

#[cfg_attr(alef, alef(skip))]
#[derive(Debug, Clone)]
enum MathNode {
    /// Plain text from m:r/m:t
    Run(String),
    /// Superscript: base^{sup}
    SSup { base: Vec<MathNode>, sup: Vec<MathNode> },
    /// Subscript: base_{sub}
    SSub { base: Vec<MathNode>, sub: Vec<MathNode> },
    /// Sub-superscript: base_{sub}^{sup}
    SSubSup {
        base: Vec<MathNode>,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
    },
    /// Fraction: \frac{num}{den}
    Frac {
        num: Vec<MathNode>,
        den: Vec<MathNode>,
        frac_type: FracType,
    },
    /// Radical: \sqrt{body} or \sqrt[deg]{body}
    Rad {
        deg: Vec<MathNode>,
        body: Vec<MathNode>,
        deg_hide: bool,
    },
    /// N-ary operator: \sum_{sub}^{sup}{body}
    Nary {
        chr: String,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
        body: Vec<MathNode>,
        sub_hide: bool,
        sup_hide: bool,
    },
    /// Delimiter: \left( ... \right)
    Delim {
        begin_chr: String,
        end_chr: String,
        sep_chr: String,
        elements: Vec<Vec<MathNode>>,
    },
    /// Function: \funcname{body}
    Func { name: Vec<MathNode>, body: Vec<MathNode> },
    /// Accent: \hat{body}
    Acc { chr: String, body: Vec<MathNode> },
    /// Equation array: \begin{aligned}...\end{aligned}
    EqArr { rows: Vec<Vec<MathNode>> },
    /// Lower limit: \underset{lim}{body}
    LimLow { body: Vec<MathNode>, lim: Vec<MathNode> },
    /// Upper limit: \overset{lim}{body}
    LimUpp { body: Vec<MathNode>, lim: Vec<MathNode> },
    /// Bar (overline/underline)
    Bar { body: Vec<MathNode>, top: bool },
    /// Group character: \underbrace{body}/\overbrace{body} for a brace, else \underset{chr}{body}/\overset{chr}{body}
    GroupChr {
        chr: String,
        top: bool,
        body: Vec<MathNode>,
    },
    /// Border box: \boxed{body}
    BorderBox { body: Vec<MathNode> },
    /// Matrix: \begin{matrix}...\end{matrix}
    Matrix { rows: Vec<Vec<Vec<MathNode>>> },
    /// Grouping container (m:box, m:phant, etc.) — passes through children
    Group { children: Vec<MathNode> },
    /// Pre-sub-superscript: {}_{sub}^{sup}{base}
    SPre {
        base: Vec<MathNode>,
        sub: Vec<MathNode>,
        sup: Vec<MathNode>,
    },
}

/// Collect an `m:oMathPara` subtree and convert to LaTeX (display math).
/// The reader should be positioned right after the `<m:oMathPara>` start tag.
pub(crate) fn collect_and_convert_omath_para(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<String, SecurityError> {
    let children = collect_children(reader, "m:oMathPara", budget)?;
    let mut parts = Vec::new();
    for child in &children {
        if let MathNode::Group { children: inner } = child {
            let rendered = render_nodes(inner);
            if !rendered.is_empty() {
                parts.push(rendered);
            }
        }
    }
    if parts.is_empty() {
        Ok(render_nodes(&children))
    } else {
        Ok(parts.join(" \\\\ "))
    }
}

/// Collect an `m:oMath` subtree and convert to LaTeX (inline math).
/// The reader should be positioned right after the `<m:oMath>` start tag.
pub(crate) fn collect_and_convert_omath(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<String, SecurityError> {
    let children = collect_children(reader, "m:oMath", budget)?;
    Ok(render_nodes(&children))
}

mod collect;
mod elements;
mod render;

use collect::collect_children;
use render::render_nodes;

#[cfg(test)]
mod tests;
