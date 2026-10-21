//! XML parsing and document structure traversal for JATS documents.

use crate::Result;
use crate::extractors::security::SecurityBudget;
use quick_xml::events::Event;

use crate::utils::xml_utils::EntityReader;

/// Extract the LaTeX for a `disp-formula` / `inline-formula` subtree.
///
/// JATS writes verbatim TeX in `tex-math` and the equation number in `label`.
pub(super) fn extract_formula_latex(reader: &mut EntityReader<'_>, budget: &mut SecurityBudget) -> Result<String> {
    crate::extraction::formula_xml::extract_formula_latex(
        reader,
        budget,
        &crate::extraction::formula_xml::FormulaElements {
            tex: "tex-math",
            label: Some("label"),
        },
    )
}

/// JATS elements that are blocks of their own inside the element whose text is read; a
/// space sets their text apart. Any other element (`italic`, `sub`, `sup`, `xref`, ...) is
/// inline, and its text joins the text around it as written.
const BLOCK_ELEMENTS: &[&str] = &[
    "p",
    "title",
    "label",
    "caption",
    "list-item",
    "tr",
    "td",
    "th",
    "sec",
    "term",
    "def",
    "break",
];

/// Extract text content from a JATS element and its children.
///
/// Inline markup keeps its text inside the word or sentence it belongs to (`CO<sub>2</sub>`
/// reads `CO2`, not `CO 2`), and runs of whitespace from the XML layout collapse to one space.
pub(super) fn extract_text_content(reader: &mut EntityReader<'_>, budget: &mut SecurityBudget) -> Result<String> {
    let mut text = String::new();
    let mut depth = 0;

    loop {
        budget.step()?;
        match reader.read_event() {
            Ok(Event::Start(_)) => {
                budget.enter()?;
                depth += 1;
            }
            Ok(Event::End(e)) => {
                budget.leave();
                if depth == 0 {
                    break;
                }
                depth -= 1;
                if BLOCK_ELEMENTS.contains(&e.local_name().as_ref()) {
                    text.push(' ');
                }
            }
            Ok(Event::Empty(e)) if BLOCK_ELEMENTS.contains(&e.local_name().as_ref()) => {
                text.push(' ');
            }
            Ok(Event::Text(t)) => {
                let decoded = t.as_ref().to_string();
                if !decoded.is_empty() {
                    budget.check_entity(&decoded)?;
                    budget.account_text(decoded.len())?;
                    text.push_str(&decoded);
                }
            }
            Ok(Event::CData(t)) => {
                let decoded = t.as_ref().to_string();
                if !decoded.trim().is_empty() {
                    budget.check_entity(&decoded)?;
                    budget.account_text(decoded.len())?;
                    text.push_str(&decoded);
                    text.push(' ');
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::error::XbergError::parsing(format!("XML parsing error: {}", e)));
            }
            _ => {}
        }
    }

    Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// Extract a formatted citation string from a `<ref>` element.
///
/// Parses structured `<element-citation>` children (person-group, article-title,
/// source, year, volume, fpage, lpage) into a conventional citation string like:
/// `Brown T, Davis K. Cognitive effects of caffeine. J Neurosci. 2002;15:234-241.`
///
/// Falls back to plain text extraction for `<mixed-citation>` or unrecognized structures.
pub(super) fn extract_citation_text(reader: &mut EntityReader<'_>, budget: &mut SecurityBudget) -> Result<String> {
    let mut depth: u32 = 0;
    let mut in_element_citation = false;
    let mut in_mixed_citation = false;
    let mut in_person_group = false;
    let mut in_name = false;

    let mut authors: Vec<String> = Vec::new();
    let mut current_surname = String::new();
    let mut current_given = String::new();
    let mut article_title = String::new();
    let mut source = String::new();
    let mut year = String::new();
    let mut volume = String::new();
    let mut fpage = String::new();
    let mut lpage = String::new();
    let mut doi = String::new();
    let mut publisher_name = String::new();
    let mut publisher_loc = String::new();

    let mut current_tag = String::new();

    let mut mixed_text = String::new();

    loop {
        budget.step()?;
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                depth += 1;
                let tag = e.name().as_ref().to_string();

                match tag.as_str() {
                    "element-citation" => {
                        in_element_citation = true;
                    }
                    "mixed-citation" => {
                        in_mixed_citation = true;
                    }
                    "person-group" if in_element_citation => {
                        in_person_group = true;
                    }
                    "name" if in_person_group => {
                        in_name = true;
                        current_surname.clear();
                        current_given.clear();
                    }
                    "surname" | "given-names" | "article-title" | "source" | "year" | "volume" | "fpage" | "lpage"
                    | "publisher-name" | "publisher-loc"
                        if in_element_citation =>
                    {
                        current_tag = tag;
                    }
                    "pub-id" | "article-id" if in_element_citation => {
                        let mut id_type = String::new();
                        for attr in e.attributes().flatten() {
                            let key = std::borrow::Cow::Borrowed(attr.key.as_ref());
                            let val = std::borrow::Cow::Borrowed(attr.value.as_ref());
                            budget.check_attr(&key, &val)?;
                            if key == "pub-id-type" {
                                id_type = val.to_string();
                            }
                        }
                        if id_type == "doi" {
                            current_tag = "pub-id-doi".to_string();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                budget.leave();
                if depth == 0 {
                    break;
                }
                let tag = e.name().as_ref().to_string();

                match tag.as_str() {
                    "name" if in_name => {
                        in_name = false;
                        let mut author = String::new();
                        if !current_surname.is_empty() {
                            author.push_str(current_surname.trim());
                        }
                        if !current_given.is_empty() {
                            if !author.is_empty() {
                                author.push(' ');
                            }
                            author.push_str(current_given.trim());
                        }
                        if !author.is_empty() {
                            authors.push(author);
                        }
                    }
                    "person-group" => {
                        in_person_group = false;
                    }
                    "element-citation" => {
                        in_element_citation = false;
                    }
                    "mixed-citation" => {
                        in_mixed_citation = false;
                    }
                    _ => {}
                }

                current_tag.clear();
                depth -= 1;
            }
            Ok(Event::Text(t)) => {
                let decoded = t.as_ref().to_string();
                let trimmed = decoded.trim();

                if !trimmed.is_empty() {
                    budget.check_entity(trimmed)?;
                    budget.account_text(trimmed.len())?;
                    if in_mixed_citation {
                        if !mixed_text.is_empty() {
                            mixed_text.push(' ');
                        }
                        mixed_text.push_str(trimmed);
                    } else if in_element_citation {
                        match current_tag.as_str() {
                            "surname" => current_surname.push_str(trimmed),
                            "given-names" => current_given.push_str(trimmed),
                            "article-title" => {
                                if !article_title.is_empty() {
                                    article_title.push(' ');
                                }
                                article_title.push_str(trimmed);
                            }
                            "source" => source.push_str(trimmed),
                            "year" => year.push_str(trimmed),
                            "volume" => volume.push_str(trimmed),
                            "fpage" => fpage.push_str(trimmed),
                            "lpage" => lpage.push_str(trimmed),
                            "pub-id-doi" => doi.push_str(trimmed),
                            "publisher-name" => publisher_name.push_str(trimmed),
                            "publisher-loc" => publisher_loc.push_str(trimmed),
                            _ => {}
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::error::XbergError::parsing(format!("XML parsing error: {}", e)));
            }
            _ => {}
        }
    }

    if !mixed_text.is_empty() {
        return Ok(mixed_text);
    }

    let mut citation = String::new();

    if !authors.is_empty() {
        citation.push_str(&authors.join(", "));
        citation.push_str(". ");
    }

    if !article_title.is_empty() {
        citation.push_str(&article_title);
        citation.push_str(". ");
    }

    if !source.is_empty() {
        citation.push_str(&source);
        citation.push('.');
    }

    if !year.is_empty() {
        citation.push(' ');
        citation.push_str(&year);
    }
    if !volume.is_empty() {
        citation.push(';');
        citation.push_str(&volume);
    }
    if !fpage.is_empty() {
        citation.push(':');
        citation.push_str(&fpage);
        if !lpage.is_empty() {
            citation.push('-');
            citation.push_str(&lpage);
        }
    }
    if !citation.is_empty() && !citation.ends_with('.') {
        citation.push('.');
    }

    if !publisher_name.is_empty() || !publisher_loc.is_empty() {
        if !citation.is_empty() {
            citation.push(' ');
        }
        if !publisher_loc.is_empty() {
            citation.push_str(&publisher_loc);
        }
        if !publisher_loc.is_empty() && !publisher_name.is_empty() {
            citation.push_str(": ");
        }
        if !publisher_name.is_empty() {
            citation.push_str(&publisher_name);
        }
        citation.push('.');
    }

    if !doi.is_empty() {
        if !citation.is_empty() {
            citation.push(' ');
        }
        citation.push_str("DOI: ");
        citation.push_str(&doi);
        citation.push('.');
    }

    Ok(citation.trim().to_string())
}

/// Extract structured content from a JATS `<fig>` element.
///
/// Parses `<label>`, `<caption>` (with nested `<title>`/`<p>`), and
/// `<graphic xlink:href="...">` so callers can associate the figure's caption
/// text with its graphic reference instead of dropping or flattening them.
///
/// Returns `(label, caption_text, graphic_href)`.
pub(super) fn extract_fig_content(
    reader: &mut EntityReader<'_>,
    budget: &mut SecurityBudget,
) -> Result<(Option<String>, Option<String>, Option<String>)> {
    let mut depth: u32 = 0;
    let mut in_caption = false;
    let mut current_tag = String::new();

    let mut label = String::new();
    let mut caption = String::new();
    let mut href: Option<String> = None;

    loop {
        budget.step()?;
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                budget.enter()?;
                depth += 1;
                let name = e.name();
                let tag = crate::utils::xml_tag_name(name.as_ref()).to_string();

                match tag.as_str() {
                    "caption" => in_caption = true,
                    "label" | "title" | "p" => current_tag = tag.clone(),
                    "graphic" => {
                        for attr in e.attributes().flatten() {
                            let key = std::borrow::Cow::Borrowed(attr.key.as_ref());
                            let val = std::borrow::Cow::Borrowed(attr.value.as_ref());
                            budget.check_attr(&key, &val)?;
                            if key == "xlink:href" || key.ends_with(":href") || key == "href" {
                                href = Some(val.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            // `<graphic xlink:href="..."/>` is almost always self-closing, which quick-xml
            // reports as a standalone `Empty` event (no matching `Start`/`End` pair), so it
            // must be handled separately from `Event::Start` or the href is silently dropped.
            Ok(Event::Empty(e)) => {
                budget.enter()?;
                budget.leave();
                let name = e.name();
                let tag = crate::utils::xml_tag_name(name.as_ref());
                if tag.as_ref() == "graphic" {
                    for attr in e.attributes().flatten() {
                        let key = std::borrow::Cow::Borrowed(attr.key.as_ref());
                        let val = std::borrow::Cow::Borrowed(attr.value.as_ref());
                        budget.check_attr(&key, &val)?;
                        if key == "xlink:href" || key.ends_with(":href") || key == "href" {
                            href = Some(val.to_string());
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                budget.leave();
                if depth == 0 {
                    break;
                }
                let name = e.name();
                let tag = crate::utils::xml_tag_name(name.as_ref());
                if tag.as_ref() == "caption" {
                    in_caption = false;
                }
                current_tag.clear();
                depth -= 1;
            }
            Ok(Event::Text(t)) => {
                let decoded = t.as_ref().to_string();
                let trimmed = decoded.trim();

                if !trimmed.is_empty() {
                    budget.check_entity(trimmed)?;
                    budget.account_text(trimmed.len())?;
                    match current_tag.as_str() {
                        "label" => {
                            if !label.is_empty() {
                                label.push(' ');
                            }
                            label.push_str(trimmed);
                        }
                        "title" | "p" if in_caption => {
                            if !caption.is_empty() {
                                caption.push(' ');
                            }
                            caption.push_str(trimmed);
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(crate::error::XbergError::parsing(format!("XML parsing error: {}", e)));
            }
            _ => {}
        }
    }

    Ok((
        if label.is_empty() { None } else { Some(label) },
        if caption.is_empty() { None } else { Some(caption) },
        href,
    ))
}
