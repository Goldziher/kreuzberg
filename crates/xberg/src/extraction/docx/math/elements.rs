use super::*;

/// Collect an m:f (fraction) element.
pub(super) fn collect_frac(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut num = Vec::new();
    let mut den = Vec::new();
    let mut frac_type = FracType::Bar;
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:fPr" => {
                        // `collect_frac_pr` reads through its own `</m:fPr>` without
                        // calling `budget.leave()`; refund the enter above.
                        frac_type = collect_frac_pr(reader, budget)?;
                        budget.leave();
                    }
                    "m:num" => num = collect_children(reader, "m:num", budget)?,
                    "m:den" => den = collect_children(reader, "m:den", budget)?,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:f" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Frac { num, den, frac_type })
}

/// Read fraction properties to determine type.
fn collect_frac_pr(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<FracType, SecurityError> {
    let mut frac_type = FracType::Bar;
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                if e.name().as_ref() == "m:type"
                    && let Some(val) = get_m_val(e)
                {
                    frac_type = match val.as_str() {
                        "noBar" => FracType::NoBar,
                        "lin" => FracType::Linear,
                        "skw" => FracType::Skewed,
                        _ => FracType::Bar,
                    };
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == "m:fPr" => {
                break;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(frac_type)
}

/// Collect an m:rad (radical/sqrt) element.
pub(super) fn collect_rad(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut deg = Vec::new();
    let mut body = Vec::new();
    // An absent m:radPr or m:degHide is off, so the degree shows. ~keep
    let mut deg_hide = false;
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:radPr" => {
                        // `collect_rad_pr` reads through its own `</m:radPr>` without
                        // calling `budget.leave()`; refund the enter above.
                        deg_hide = collect_rad_pr(reader, budget)?;
                        budget.leave();
                    }
                    "m:deg" => deg = collect_children(reader, "m:deg", budget)?,
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:rad" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Rad { deg, body, deg_hide })
}

/// Read radical properties (degHide).
fn collect_rad_pr(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<bool, SecurityError> {
    let mut deg_hide = false;
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) if e.name().as_ref() == "m:degHide" => {
                deg_hide = is_toggle_on(e);
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == "m:radPr" => {
                break;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(deg_hide)
}

/// Collect an m:nary (n-ary operator) element.
pub(super) fn collect_nary(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut chr = "\u{222B}".to_string();
    let mut sub = Vec::new();
    let mut sup = Vec::new();
    let mut body = Vec::new();
    let mut sub_hide = false;
    let mut sup_hide = false;
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:naryPr" => {
                        // `collect_nary_pr` reads through its own `</m:naryPr>` without
                        // calling `budget.leave()`; refund the enter above.
                        collect_nary_pr(reader, &mut chr, &mut sub_hide, &mut sup_hide, budget)?;
                        budget.leave();
                    }
                    "m:sub" => sub = collect_children(reader, "m:sub", budget)?,
                    "m:sup" => sup = collect_children(reader, "m:sup", budget)?,
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:nary" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Nary {
        chr,
        sub,
        sup,
        body,
        sub_hide,
        sup_hide,
    })
}

/// Read n-ary properties.
pub(super) fn collect_nary_pr(
    reader: &mut Reader<&[u8]>,
    chr: &mut String,
    sub_hide: &mut bool,
    sup_hide: &mut bool,
    budget: &mut SecurityBudget,
) -> Result<(), SecurityError> {
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => match e.name().as_ref() {
                "m:chr" => {
                    if let Some(val) = get_m_val(e) {
                        *chr = val;
                    }
                }
                "m:subHide" => {
                    *sub_hide = get_m_val(e).as_deref() != Some("0");
                }
                "m:supHide" => {
                    *sup_hide = get_m_val(e).as_deref() != Some("0");
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == "m:naryPr" => {
                break;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Collect an m:d (delimiter) element.
pub(super) fn collect_delim(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<MathNode, SecurityError> {
    let mut begin_chr = "(".to_string();
    let mut end_chr = ")".to_string();
    let mut sep_chr = "|".to_string();
    let mut elements: Vec<Vec<MathNode>> = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:dPr" => {
                        // `collect_delim_pr` reads through its own `</m:dPr>` without
                        // calling `budget.leave()`; refund the enter above.
                        collect_delim_pr(reader, &mut begin_chr, &mut end_chr, &mut sep_chr, budget)?;
                        budget.leave();
                    }
                    "m:e" => {
                        elements.push(collect_children(reader, "m:e", budget)?);
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:d" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Delim {
        begin_chr,
        end_chr,
        sep_chr,
        elements,
    })
}

/// Read delimiter properties.
pub(super) fn collect_delim_pr(
    reader: &mut Reader<&[u8]>,
    begin_chr: &mut String,
    end_chr: &mut String,
    sep_chr: &mut String,
    budget: &mut SecurityBudget,
) -> Result<(), SecurityError> {
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => match e.name().as_ref() {
                "m:begChr" => {
                    if let Some(val) = get_m_val(e) {
                        *begin_chr = val;
                    }
                }
                "m:endChr" => {
                    if let Some(val) = get_m_val(e) {
                        *end_chr = val;
                    }
                }
                "m:sepChr" => {
                    if let Some(val) = get_m_val(e) {
                        *sep_chr = val;
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) if e.name().as_ref() == "m:dPr" => {
                break;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Collect an m:func element.
pub(super) fn collect_func(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut name = Vec::new();
    let mut body = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:fName" => name = collect_children(reader, "m:fName", budget)?,
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    "m:funcPr" => {
                        // Consumes its own `</m:funcPr>`; refund the enter above.
                        skip_to_end(reader, "m:funcPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:func" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Func { name, body })
}

/// Collect an m:acc (accent) element.
pub(super) fn collect_acc(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut chr = "\u{0302}".to_string();
    let mut body = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:accPr" => {
                        // `collect_acc_pr` reads through its own `</m:accPr>` without
                        // calling `budget.leave()`; refund the enter above.
                        collect_acc_pr(reader, &mut chr, budget)?;
                        budget.leave();
                    }
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:acc" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Acc { chr, body })
}

/// Read accent properties.
pub(super) fn collect_acc_pr(
    reader: &mut Reader<&[u8]>,
    chr: &mut String,
    budget: &mut SecurityBudget,
) -> Result<(), SecurityError> {
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                if e.name().as_ref() == "m:chr"
                    && let Some(val) = get_m_val(e)
                {
                    *chr = val;
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == "m:accPr" => {
                break;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Collect an m:eqArr element.
pub(super) fn collect_eqarr(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<MathNode, SecurityError> {
    let mut rows = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:e" => rows.push(collect_children(reader, "m:e", budget)?),
                    "m:eqArrPr" => {
                        // Consumes its own `</m:eqArrPr>`; refund the enter above.
                        skip_to_end(reader, "m:eqArrPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:eqArr" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::EqArr { rows })
}

/// Collect an m:limLow element.
pub(super) fn collect_limlow(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<MathNode, SecurityError> {
    let mut body = Vec::new();
    let mut lim = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    "m:lim" => lim = collect_children(reader, "m:lim", budget)?,
                    "m:limLowPr" => {
                        // Consumes its own `</m:limLowPr>`; refund the enter above.
                        skip_to_end(reader, "m:limLowPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:limLow" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::LimLow { body, lim })
}

/// Collect an m:limUpp element.
pub(super) fn collect_limupp(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<MathNode, SecurityError> {
    let mut body = Vec::new();
    let mut lim = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    "m:lim" => lim = collect_children(reader, "m:lim", budget)?,
                    "m:limUppPr" => {
                        // Consumes its own `</m:limUppPr>`; refund the enter above.
                        skip_to_end(reader, "m:limUppPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:limUpp" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::LimUpp { body, lim })
}

/// Collect an m:bar element.
pub(super) fn collect_bar(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
    let mut body = Vec::new();
    let mut top = true;
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:barPr" => {
                        // `collect_bar_pr` reads through its own `</m:barPr>` without
                        // calling `budget.leave()`; refund the enter above.
                        top = collect_bar_pr(reader, budget)?;
                        budget.leave();
                    }
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:bar" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Bar { body, top })
}

/// Read bar properties (pos).
fn collect_bar_pr(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<bool, SecurityError> {
    let mut top = true;
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => {
                if e.name().as_ref() == "m:pos"
                    && let Some(val) = get_m_val(e)
                {
                    top = val != "bot";
                }
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == "m:barPr" => {
                break;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(top)
}

/// Collect an m:groupChr element.
pub(super) fn collect_group_chr(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<MathNode, SecurityError> {
    let mut chr = "\u{23DF}".to_string();
    let mut top = false;
    let mut body = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:groupChrPr" => {
                        collect_group_chr_pr(reader, &mut chr, &mut top, budget)?;
                    }
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:groupChr" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    budget.check_entity(&chr)?;
    budget.account_text(chr.len())?;
    Ok(MathNode::GroupChr { chr, top, body })
}

/// Read group character properties (chr, pos).
fn collect_group_chr_pr(
    reader: &mut Reader<&[u8]>,
    chr: &mut String,
    top: &mut bool,
    budget: &mut SecurityBudget,
) -> Result<(), SecurityError> {
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        let event = reader.read_event_into(&mut buf);
        if matches!(event, Ok(Event::Start(_))) {
            budget.enter()?;
        }
        match event {
            Ok(Event::Start(ref e) | Event::Empty(ref e)) => match e.name().as_ref() {
                "m:chr" => {
                    if let Some(val) = get_m_val(e) {
                        budget.check_attr("m:val", &val)?;
                        *chr = val;
                    }
                }
                "m:pos" => {
                    if let Some(val) = get_m_val(e) {
                        budget.check_attr("m:val", &val)?;
                        *top = val == "top";
                    }
                }
                _ => {}
            },
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:groupChrPr" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(())
}

/// Collect an m:borderBox element.
pub(super) fn collect_borderbox(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<MathNode, SecurityError> {
    let mut body = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:e" => body = collect_children(reader, "m:e", budget)?,
                    "m:borderBoxPr" => {
                        // Consumes its own `</m:borderBoxPr>`; refund the enter above.
                        skip_to_end(reader, "m:borderBoxPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:borderBox" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::BorderBox { body })
}

/// Collect an m:m (matrix) element.
pub(super) fn collect_matrix(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<MathNode, SecurityError> {
    let mut rows: Vec<Vec<Vec<MathNode>>> = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                match e.name().as_ref() {
                    "m:mr" => {
                        rows.push(collect_matrix_row(reader, budget)?);
                    }
                    "m:mPr" => {
                        // Consumes its own `</m:mPr>`; refund the enter above.
                        skip_to_end(reader, "m:mPr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:m" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::Matrix { rows })
}

/// Collect a matrix row (m:mr) — returns cells.
fn collect_matrix_row(
    reader: &mut Reader<&[u8]>,
    budget: &mut SecurityBudget,
) -> Result<Vec<Vec<MathNode>>, SecurityError> {
    let mut cells: Vec<Vec<MathNode>> = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == "m:e" => {
                budget.enter()?;
                cells.push(collect_children(reader, "m:e", budget)?);
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:mr" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(cells)
}

/// Collect an m:sPre (pre-sub-superscript) element.
pub(super) fn collect_spre(reader: &mut Reader<&[u8]>, budget: &mut SecurityBudget) -> Result<MathNode, SecurityError> {
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
                    "m:sPrePr" => {
                        // Consumes its own `</m:sPrePr>`; refund the enter above.
                        skip_to_end(reader, "m:sPrePr");
                        budget.leave();
                    }
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                budget.leave();
                if e.name().as_ref() == "m:sPre" {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }

    Ok(MathNode::SPre { base, sub, sup })
}

/// Collect body of a generic element (skip its *Pr, gather m:e children).
pub(super) fn collect_element_body(
    reader: &mut Reader<&[u8]>,
    end_tag: &str,
    budget: &mut SecurityBudget,
) -> Result<Vec<MathNode>, SecurityError> {
    let mut children = Vec::new();
    let mut buf = Vec::new();

    loop {
        budget.step()?;
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                budget.enter()?;
                let tag = e.name().as_ref().to_string();
                if tag.ends_with("Pr") {
                    // Consumes its own matching end tag; refund the enter above.
                    skip_to_end(reader, &tag);
                    budget.leave();
                } else if tag == "m:e" {
                    children.extend(collect_children(reader, "m:e", budget)?);
                } else {
                    // Same as the `*Pr` branch: consumes its own matching end tag.
                    skip_to_end(reader, &tag);
                    budget.leave();
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

    Ok(children)
}

/// Get the `m:val` attribute value from a start/empty element.
fn get_m_val(e: &quick_xml::events::BytesStart) -> Option<String> {
    for attr in e.attributes().flatten() {
        let key = attr.key.as_ref();
        if key == "m:val" || key == "val" {
            let raw = attr.value.as_ref();
            return Some(quick_xml::escape::unescape(raw).map_or_else(|_| raw.to_string(), |s| s.into_owned()));
        }
    }
    None
}

/// Check an ST_OnOff `m:val`: absent, `1`, `true` and `on` are on.
fn is_toggle_on(e: &quick_xml::events::BytesStart) -> bool {
    !matches!(get_m_val(e).as_deref(), Some("0" | "false" | "off"))
}

/// Skip forward until the matching end tag is consumed.
pub(super) fn skip_to_end(reader: &mut Reader<&[u8]>, tag: &str) {
    let mut depth = 1u32;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) if e.name().as_ref() == tag => {
                depth += 1;
            }
            Ok(Event::End(ref e)) if e.name().as_ref() == tag => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}
