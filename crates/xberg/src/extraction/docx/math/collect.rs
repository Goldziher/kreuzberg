use super::elements::*;
use super::*;

/// Recursively collect child nodes until the matching close tag.
pub(super) fn collect_children(
    reader: &mut Reader<&[u8]>,
    end_tag: &str,
    budget: &mut SecurityBudget,
) -> Result<Vec<MathNode>, SecurityError> {
    let mut nodes = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                let tag = e.name().as_ref().to_string();
                if let Some(node) = dispatch_omml_tag(&tag, reader, budget)? {
                    nodes.push(node);
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == end_tag {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(nodes)
}

/// Collect the node for one OMML start tag encountered by [`collect_children`], or
/// `None` for a tag this converter does not recognise (already skipped to its own end
/// tag).
fn dispatch_omml_tag(
    tag: &str,
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<Option<MathNode>, SecurityError> {
    let node = match tag {
        "m:r" => collect_run(reader, budget)?,
        "m:sSup" => collect_ssup(reader, budget)?,
        "m:sSub" => collect_ssub(reader, budget)?,
        "m:sSubSup" => collect_ssubsup(reader, budget)?,
        "m:f" => collect_frac(reader, budget)?,
        "m:rad" => collect_rad(reader, budget)?,
        "m:nary" => collect_nary(reader, budget)?,
        "m:d" => collect_delim(reader, budget)?,
        "m:func" => collect_func(reader, budget)?,
        "m:acc" => collect_acc(reader, budget)?,
        "m:eqArr" => collect_eqarr(reader, budget)?,
        "m:limLow" => collect_limlow(reader, budget)?,
        "m:limUpp" => collect_limupp(reader, budget)?,
        "m:bar" => collect_bar(reader, budget)?,
        "m:groupChr" => collect_group_chr(reader, budget)?,
        "m:borderBox" => collect_borderbox(reader, budget)?,
        "m:m" => collect_matrix(reader, budget)?,
        "m:box" | "m:phant" => {
            let children = collect_element_body(reader, tag, budget)?;
            MathNode::Group { children }
        }
        "m:sPre" => collect_spre(reader, budget)?,
        "m:oMath" => {
            let inner = collect_children(reader, "m:oMath", budget)?;
            MathNode::Group { children: inner }
        }
        _ => {
            // `skip_to_end` reads through its own matching end tag directly, so
            // `collect_children`'s `Event::End` arm never sees it. Refund the enter
            // above or every unrecognized OMML tag leaks one depth level. ~keep
            skip_to_end(reader, tag);
            budget.leave();
            return Ok(None);
        }
    };
    Ok(Some(node))
}

/// Collect text from an m:r element (reads until </m:r>).
fn collect_run(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut text = String::new();
    let mut buf = Vec::new();
    let mut in_text = false;

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:t" => in_text = true,
                    "m:rPr" => {
                        // Consumes its own `</m:rPr>`; refund the enter above.
                        skip_to_end(reader, "m:rPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) if in_text => {
                let t = e.xml10_content();
                budget.check_entity(&t)?;
                budget.account_text(t.len())?;
                text.push_str(&t);
            }
            Ok(Event::GeneralRef(ref e)) if in_text => {
                let t = crate::utils::xml_utils::resolve_general_ref(e);
                budget.account_text(t.len())?;
                text.push_str(&t);
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                match e.name().as_ref() {
                    "m:t" => in_text = false,
                    "m:r" => break,
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Run(text))
}

/// Collect an m:sSup (superscript) element.
fn collect_ssup(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut base = Vec::new();
    let mut sup = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:e" => base = collect_children(reader, "m:e", budget)?,
                    "m:sup" => sup = collect_children(reader, "m:sup", budget)?,
                    "m:sSupPr" => {
                        // Consumes its own `</m:sSupPr>`; refund the enter above.
                        skip_to_end(reader, "m:sSupPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:sSup" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::SSup { base, sup })
}

/// Collect an m:sSub (subscript) element.
fn collect_ssub(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut base = Vec::new();
    let mut sub = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:e" => base = collect_children(reader, "m:e", budget)?,
                    "m:sub" => sub = collect_children(reader, "m:sub", budget)?,
                    "m:sSubPr" => {
                        // Consumes its own `</m:sSubPr>`; refund the enter above.
                        skip_to_end(reader, "m:sSubPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:sSub" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::SSub { base, sub })
}

/// Collect an m:sSubSup element.
fn collect_ssubsup(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut base = Vec::new();
    let mut sub = Vec::new();
    let mut sup = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:e" => base = collect_children(reader, "m:e", budget)?,
                    "m:sub" => sub = collect_children(reader, "m:sub", budget)?,
                    "m:sup" => sup = collect_children(reader, "m:sup", budget)?,
                    "m:sSubSupPr" => {
                        // Consumes its own `</m:sSubSupPr>`; refund the enter above.
                        skip_to_end(reader, "m:sSubSupPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:sSubSup" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::SSubSup { base, sub, sup })
}
