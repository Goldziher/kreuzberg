//! LaTeX command processing.
//!
//! This module handles inline LaTeX commands like formatting (\textbf, \emph, etc.),
//! math mode ($...$), and other inline elements.

use super::utilities::read_braced_from_chars;

/// Processes a line of LaTeX, handling commands and inline math.
///
/// Recursively processes nested commands and preserves math mode content.
pub(crate) fn process_line(line: &str) -> String {
    let line = super::utilities::resolve_text_symbols(line);
    let mut result = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            let mut cmd = String::new();
            while let Some(&c) = chars.peek() {
                if c.is_alphabetic() {
                    cmd.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            process_command(&cmd, &mut chars, &mut result);
        } else if ch == '$' {
            result.push(ch);
            while let Some(&c) = chars.peek() {
                result.push(chars.next().unwrap());
                if c == '$' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Processes a single LaTeX command.
///
/// Handles formatting commands (\textbf, \emph, etc.) and extracts their content.
fn process_command(cmd: &str, chars: &mut std::iter::Peekable<std::str::Chars>, result: &mut String) {
    match cmd {
        "textbf" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
        "textit" | "emph" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
        "texttt" => {
            if let Some(content) = read_braced_from_chars(chars) {
                result.push_str(&content);
            }
        }
        "underline" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
        "font" => {
            while let Some(&c) = chars.peek() {
                if c == '\\' {
                    break;
                }
                chars.next();
            }
        }
        "usepackage" | "documentclass" | "pagestyle" | "setlength" | "newcommand" | "renewcommand" | "def" | "let"
        | "input" | "include" | "bibliography" | "bibliographystyle" | "graphicspath" | "geometry" | "hypersetup"
        | "rule" | "hspace" | "vspace" | "addtolength" | "setcounter" | "addtocounter" | "value"
        | "VerbatimFootnotes" | "numberwithin" => {
            while chars.peek() == Some(&'{') || chars.peek() == Some(&'[') {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == ']' {
                            break;
                        }
                    }
                } else {
                    read_braced_from_chars(chars);
                }
            }
        }
        "cite" | "citep" | "citet" | "citealp" | "citeauthor" | "citeyear" => {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == ']' {
                        break;
                    }
                }
            }
            if let Some(key) = read_braced_from_chars(chars) {
                result.push('[');
                result.push_str(&key);
                result.push(']');
            }
        }
        "ref" | "eqref" | "pageref" | "autoref" | "cref" | "Cref" | "nameref" => {
            if let Some(label) = read_braced_from_chars(chars) {
                result.push('[');
                result.push_str(&label);
                result.push(']');
            }
        }
        "label" => {
            read_braced_from_chars(chars);
        }
        "url" => {
            if let Some(url) = read_braced_from_chars(chars) {
                result.push_str(&url);
            }
        }
        "href" => {
            let url = read_braced_from_chars(chars);
            let text = read_braced_from_chars(chars);
            match (text, url) {
                (Some(text), Some(url)) => {
                    let processed = process_line(&text);
                    result.push_str(&processed);
                    result.push_str(" (");
                    result.push_str(&url);
                    result.push(')');
                }
                (Some(text), None) => {
                    let processed = process_line(&text);
                    result.push_str(&processed);
                }
                (None, Some(url)) => {
                    result.push_str(&url);
                }
                _ => {}
            }
        }
        "footnote" | "footnotetext" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(" (");
                result.push_str(&processed);
                result.push(')');
            }
        }
        "textsuperscript" => {
            if let Some(content) = read_braced_from_chars(chars) {
                result.push_str(&content);
            }
        }
        "textsubscript" => {
            if let Some(content) = read_braced_from_chars(chars) {
                result.push_str(&content);
            }
        }
        "mbox" | "hbox" | "vbox" | "text" | "mathrm" | "mathbf" | "mathit" | "mathsf" | "mathtt" | "boldsymbol"
        | "textrm" | "textsf" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
        "sout" => {
            if let Some(content) = read_braced_from_chars(chars) {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
        "ensuremath" => {
            if let Some(content) = read_braced_from_chars(chars) {
                result.push_str(&content);
            }
        }
        "textgreater" => {
            if chars.peek() == Some(&'{') {
                read_braced_from_chars(chars);
            }
            result.push('>');
        }
        "textless" => {
            if chars.peek() == Some(&'{') {
                read_braced_from_chars(chars);
            }
            result.push('<');
        }
        "textbackslash" => {
            if chars.peek() == Some(&'{') {
                read_braced_from_chars(chars);
            }
            result.push('\\');
        }
        "ldots" | "dots" => {
            if chars.peek() == Some(&'{') {
                read_braced_from_chars(chars);
            }
            result.push('\u{2026}');
        }
        "textendash" => {
            if chars.peek() == Some(&'{') {
                read_braced_from_chars(chars);
            }
            result.push('\u{2013}');
        }
        "textemdash" => {
            if chars.peek() == Some(&'{') {
                read_braced_from_chars(chars);
            }
            result.push('\u{2014}');
        }
        "par" | "noindent" | "newline" | "linebreak" | "pagebreak" | "newpage" | "clearpage" | "cleardoublepage"
        | "bigskip" | "medskip" | "smallskip" | "vfill" | "hfill" | "centering" | "raggedright" | "raggedleft"
        | "maketitle" | "tableofcontents" | "listoffigures" | "listoftables" | "appendix" | "indent" | "relax"
        | "protect" | "nobreak" | "allowbreak" | "sloppy" | "fussy" | "normalsize" | "small" | "footnotesize"
        | "large" | "Large" | "LARGE" | "huge" | "Huge" | "tiny" | "scriptsize" | "doublespacing" | "singlespacing"
        | "onehalfspacing" => {}
        _ => {
            if chars.peek() == Some(&'{')
                && let Some(content) = read_braced_from_chars(chars)
            {
                let processed = process_line(&content);
                result.push_str(&processed);
            }
        }
    }
}
