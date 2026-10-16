//! PDF bookmark/outline extraction using lopdf.
//!
//! Extracts the document outline (bookmarks) from the PDF catalog and returns
//! them as a list of `Uri` values: external URLs as `ExtractedUri::hyperlink()`,
//! page destinations as `ExtractedUri::anchor()`.

use std::collections::{HashMap, HashSet};

use crate::types::uri::ExtractedUri;
use lopdf::{Dictionary, Document, Object, ObjectId};

const MAX_OUTLINE_DEPTH: usize = 50;
const MAX_OUTLINE_ITEMS: usize = 500;
const MAX_NAME_TREE_DEPTH: usize = 50;
const MAX_NAME_TREE_NODES: usize = 500;
const MAX_NAMED_DESTINATIONS: usize = 2_000;
const MAX_DESTINATION_HOPS: usize = 50;

type NamedDestinations = HashMap<Vec<u8>, Object>;
type PageNumbers = HashMap<ObjectId, u32>;

#[derive(Debug, Clone, Copy)]
struct ResolvedPageDestination {
    page_number: Option<u32>,
}

/// A resolved PDF outline item retained for structural recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PdfOutlineEntry {
    /// The human-readable outline title.
    pub(crate) title: String,
    /// Zero-based depth relative to the document's root outline items.
    pub(crate) depth: usize,
    /// One-based destination page, when the destination resolves within this PDF.
    pub(crate) page_number: Option<u32>,
    uri: Option<ExtractedUri>,
}

#[cfg(test)]
impl PdfOutlineEntry {
    pub(crate) fn test_entry(title: &str, depth: usize, page_number: u32) -> Self {
        Self {
            title: title.to_string(),
            depth,
            page_number: Some(page_number),
            uri: None,
        }
    }
}

/// Extract rich outline entries from a PDF document loaded via lopdf.
///
/// Traversal is bounded independently of successfully decoded entries, and
/// indirect object IDs are visited at most once so malformed outline cycles
/// cannot recurse indefinitely.
pub(crate) fn extract_outline_entries(document: &Document) -> Vec<PdfOutlineEntry> {
    let Some(first_id) = first_outline_item(document) else {
        return Vec::new();
    };

    let named_destinations = collect_named_destinations(document);
    let page_numbers = document.get_pages().into_iter().map(|(page, id)| (id, page)).collect();
    let mut walker = OutlineWalker::new(document, named_destinations, page_numbers);
    walker.walk(first_id, 0);
    walker.entries
}

/// Extract bookmarks (outlines) through the existing URI-facing API.
///
/// Resolved named destinations retain their anchor URL and now also carry page
/// metadata when the destination points into this document.
#[cfg(test)]
pub(crate) fn extract_bookmarks(document: &Document) -> Vec<ExtractedUri> {
    extract_bookmarks_from_entries(&extract_outline_entries(document))
}

pub(crate) fn extract_bookmarks_from_entries(entries: &[PdfOutlineEntry]) -> Vec<ExtractedUri> {
    entries.iter().filter_map(|entry| entry.uri.clone()).collect()
}

fn first_outline_item(document: &Document) -> Option<ObjectId> {
    let catalog = document.catalog().ok()?;
    let outlines = dereferenced_dictionary(document, catalog.get(b"Outlines").ok()?)?;
    outlines.get(b"First").ok()?.as_reference().ok()
}

struct OutlineWalker<'a> {
    document: &'a Document,
    named_destinations: NamedDestinations,
    page_numbers: PageNumbers,
    visited: HashSet<ObjectId>,
    visited_count: usize,
    entries: Vec<PdfOutlineEntry>,
}

impl<'a> OutlineWalker<'a> {
    fn new(document: &'a Document, named_destinations: NamedDestinations, page_numbers: PageNumbers) -> Self {
        Self {
            document,
            named_destinations,
            page_numbers,
            visited: HashSet::new(),
            visited_count: 0,
            entries: Vec::new(),
        }
    }

    fn walk(&mut self, item_id: ObjectId, depth: usize) {
        if depth > MAX_OUTLINE_DEPTH || self.visited_count >= MAX_OUTLINE_ITEMS || !self.visited.insert(item_id) {
            return;
        }
        self.visited_count += 1;

        let Some(dict) = self
            .document
            .get_object(item_id)
            .ok()
            .and_then(|object| object.as_dict().ok())
        else {
            return;
        };
        let entry = self.extract_entry(dict, depth);
        let child = dict.get(b"First").ok().and_then(|object| object.as_reference().ok());
        let sibling = dict.get(b"Next").ok().and_then(|object| object.as_reference().ok());

        if let Some(entry) = entry {
            self.entries.push(entry);
        }
        if let Some(child) = child {
            self.walk(child, depth + 1);
        }
        if let Some(sibling) = sibling {
            self.walk(sibling, depth);
        }
    }

    fn extract_entry(&self, dict: &Dictionary, depth: usize) -> Option<PdfOutlineEntry> {
        let title = dict
            .get(b"Title")
            .ok()
            .and_then(|object| decode_pdf_text(self.document, object));
        let uri = self
            .extract_destination(dict, title.clone())
            .or_else(|| self.extract_action(dict, title.clone()));
        if title.is_none() && uri.is_none() {
            return None;
        }
        let page_number = uri.as_ref().and_then(|value| value.page);

        Some(PdfOutlineEntry {
            title: title.unwrap_or_default(),
            depth,
            page_number,
            uri,
        })
    }

    fn extract_destination(&self, dict: &Dictionary, label: Option<String>) -> Option<ExtractedUri> {
        self.internal_destination_uri(dict.get(b"Dest").ok()?, label)
    }

    fn extract_action(&self, dict: &Dictionary, label: Option<String>) -> Option<ExtractedUri> {
        let action = dereferenced_dictionary(self.document, dict.get(b"A").ok()?)?;
        match action.get(b"S").ok()?.as_name().ok()? {
            b"URI" => action
                .get(b"URI")
                .ok()
                .and_then(|object| decode_uri(self.document, object))
                .map(|url| ExtractedUri::hyperlink(url, label)),
            b"GoTo" => self.internal_destination_uri(action.get(b"D").ok()?, label),
            _ => None,
        }
    }

    fn internal_destination_uri(&self, destination: &Object, label: Option<String>) -> Option<ExtractedUri> {
        let resolved = self.document.dereference(destination).ok()?.1;
        if let Some(name) = destination_name(resolved) {
            let page = self.resolve_named_page(name);
            return Some(page_anchor(format!("#{}", String::from_utf8_lossy(name)), label, page));
        }

        let destination = self.resolve_page_destination(resolved, &mut HashSet::new(), 0)?;
        Some(page_anchor(
            format!("#page={}", destination.page_number.unwrap_or(1)),
            label,
            destination.page_number,
        ))
    }

    fn resolve_named_page(&self, name: &[u8]) -> Option<u32> {
        let destination = self.named_destinations.get(name)?;
        let mut visited_names = HashSet::new();
        visited_names.insert(name.to_vec());
        self.resolve_page_destination(destination, &mut visited_names, 0)?
            .page_number
    }

    fn resolve_page_destination(
        &self,
        destination: &Object,
        visited_names: &mut HashSet<Vec<u8>>,
        hops: usize,
    ) -> Option<ResolvedPageDestination> {
        if hops > MAX_DESTINATION_HOPS {
            return None;
        }
        let resolved = self.document.dereference(destination).ok()?.1;
        match resolved {
            Object::Array(items) => Some(ResolvedPageDestination {
                page_number: items
                    .first()
                    .and_then(|page| page.as_reference().ok())
                    .and_then(|id| self.page_numbers.get(&id).copied()),
            }),
            Object::Dictionary(dict) => self.resolve_page_destination(dict.get(b"D").ok()?, visited_names, hops + 1),
            Object::String(name, _) | Object::Name(name) if visited_names.insert(name.clone()) => self
                .named_destinations
                .get(name)
                .and_then(|target| self.resolve_page_destination(target, visited_names, hops + 1)),
            _ => None,
        }
    }
}

fn decode_pdf_text(document: &Document, object: &Object) -> Option<String> {
    let object = document.dereference(object).ok()?.1;
    lopdf::decode_text_string(object).ok().or_else(|| {
        object
            .as_str()
            .ok()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    })
}

fn decode_uri(document: &Document, object: &Object) -> Option<String> {
    let object = document.dereference(object).ok()?.1;
    object
        .as_str()
        .ok()
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
}

fn destination_name(object: &Object) -> Option<&[u8]> {
    match object {
        Object::String(name, _) | Object::Name(name) => Some(name),
        _ => None,
    }
}

fn page_anchor(url: String, label: Option<String>, page: Option<u32>) -> ExtractedUri {
    let mut uri = ExtractedUri::anchor(url, label);
    uri.page = page;
    uri
}

fn dereferenced_dictionary<'a>(document: &'a Document, object: &'a Object) -> Option<&'a Dictionary> {
    document.dereference(object).ok()?.1.as_dict().ok()
}

fn collect_named_destinations(document: &Document) -> NamedDestinations {
    let mut destinations = HashMap::new();
    let Some(catalog) = document.catalog().ok() else {
        return destinations;
    };

    if let Ok(object) = catalog.get(b"Dests") {
        collect_legacy_destinations(document, object, &mut destinations);
    }
    if let Some(dests) = catalog
        .get(b"Names")
        .ok()
        .and_then(|object| dereferenced_dictionary(document, object))
        .and_then(|names| names.get(b"Dests").ok())
    {
        collect_name_tree(document, dests, &mut destinations, &mut NameTreeBudget::default(), 0);
    }
    destinations
}

fn collect_legacy_destinations(document: &Document, object: &Object, destinations: &mut NamedDestinations) {
    let Some(dict) = dereferenced_dictionary(document, object) else {
        return;
    };
    for (name, destination) in dict.iter().take(MAX_NAMED_DESTINATIONS) {
        destinations.insert(name.clone(), destination.clone());
    }
}

#[derive(Default)]
struct NameTreeBudget {
    visited: HashSet<ObjectId>,
    attempted_nodes: usize,
    attempted_pairs: usize,
}

fn collect_name_tree(
    document: &Document,
    object: &Object,
    destinations: &mut NamedDestinations,
    budget: &mut NameTreeBudget,
    depth: usize,
) {
    if depth > MAX_NAME_TREE_DEPTH || budget.attempted_nodes >= MAX_NAME_TREE_NODES {
        return;
    }
    budget.attempted_nodes += 1;
    if let Ok(id) = object.as_reference()
        && !budget.visited.insert(id)
    {
        return;
    }
    let Some(dict) = dereferenced_dictionary(document, object) else {
        return;
    };

    collect_name_pairs(document, dict, destinations, budget);
    let Some(kids) = dict
        .get(b"Kids")
        .ok()
        .and_then(|value| document.dereference(value).ok())
        .and_then(|(_, value)| value.as_array().ok())
    else {
        return;
    };
    for kid in kids {
        if budget.attempted_nodes >= MAX_NAME_TREE_NODES {
            break;
        }
        collect_name_tree(document, kid, destinations, budget, depth + 1);
    }
}

fn collect_name_pairs(
    document: &Document,
    dict: &Dictionary,
    destinations: &mut NamedDestinations,
    budget: &mut NameTreeBudget,
) {
    let Some(names) = dict
        .get(b"Names")
        .ok()
        .and_then(|value| document.dereference(value).ok())
        .and_then(|(_, value)| value.as_array().ok())
    else {
        return;
    };

    for pair in names.as_chunks::<2>().0 {
        if budget.attempted_pairs >= MAX_NAMED_DESTINATIONS || destinations.len() >= MAX_NAMED_DESTINATIONS {
            return;
        }
        budget.attempted_pairs += 1;
        if let Ok(name) = pair[0].as_str() {
            destinations.insert(name.to_vec(), pair[1].clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::uri::UriKind;
    use lopdf::{StringFormat, dictionary};

    fn document_with_pages(page_count: usize) -> (Document, Vec<ObjectId>) {
        let mut document = Document::with_version("1.7");
        let pages_id = document.new_object_id();
        let page_ids = (0..page_count)
            .map(|_| {
                document.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                })
            })
            .collect::<Vec<_>>();
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => page_ids.iter().copied().map(Object::Reference).collect::<Vec<_>>(),
                "Count" => page_count as i64,
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        document.trailer.set("Root", catalog_id);
        (document, page_ids)
    }

    fn attach_outlines(document: &mut Document, first: ObjectId) {
        let outlines_id = document.add_object(dictionary! {
            "Type" => "Outlines",
            "First" => first,
        });
        document.catalog_mut().unwrap().set("Outlines", outlines_id);
    }

    fn page_destination(page_id: ObjectId) -> Object {
        Object::Array(vec![Object::Reference(page_id), Object::Name(b"Fit".to_vec())])
    }

    #[test]
    fn extracts_direct_and_named_destinations() {
        let (mut document, pages) = document_with_pages(2);
        let direct_id = document.new_object_id();
        let named_destination_id = document.add_object(Object::string_literal("chapter-two"));
        let named_id = document.add_object(dictionary! {
            "Title" => Object::string_literal("Named"),
            "Dest" => named_destination_id,
        });
        document.objects.insert(
            direct_id,
            Object::Dictionary(dictionary! {
                "Title" => Object::string_literal("Direct"),
                "Dest" => page_destination(pages[0]),
                "Next" => named_id,
            }),
        );
        let dests_id = document.add_object(dictionary! {
            "Names" => vec![Object::string_literal("chapter-two"), page_destination(pages[1])],
        });
        let names_id = document.add_object(dictionary! { "Dests" => dests_id });
        document.catalog_mut().unwrap().set("Names", names_id);
        attach_outlines(&mut document, direct_id);

        let entries = extract_outline_entries(&document);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].page_number, Some(1));
        assert_eq!(entries[1].page_number, Some(2));
        let uris = extract_bookmarks(&document);
        assert_eq!(uris[0].url, "#page=1");
        assert_eq!(uris[1].url, "#chapter-two");
        assert_eq!(uris[1].page, Some(2));
    }

    #[test]
    fn legacy_destinations_allow_names_and_kids_keys() {
        let (mut document, pages) = document_with_pages(2);
        let legacy_id = document.add_object(dictionary! {
            "Kids" => page_destination(pages[0]),
            "Names" => page_destination(pages[1]),
        });
        document.catalog_mut().unwrap().set("Dests", legacy_id);

        let destinations = collect_named_destinations(&document);
        assert!(destinations.contains_key(b"Kids".as_slice()));
        assert!(destinations.contains_key(b"Names".as_slice()));
    }

    #[test]
    fn repeated_name_pairs_consume_the_attempt_budget() {
        let (mut document, pages) = document_with_pages(1);
        let mut names = Vec::with_capacity((MAX_NAMED_DESTINATIONS + 1) * 2);
        for _ in 0..MAX_NAMED_DESTINATIONS {
            names.push(Object::string_literal("duplicate"));
            names.push(page_destination(pages[0]));
        }
        names.push(Object::string_literal("after-budget"));
        names.push(page_destination(pages[0]));
        let dests_id = document.add_object(dictionary! { "Names" => names });
        let names_id = document.add_object(dictionary! { "Dests" => dests_id });
        document.catalog_mut().unwrap().set("Names", names_id);

        let destinations = collect_named_destinations(&document);
        assert!(destinations.contains_key(b"duplicate".as_slice()));
        assert!(!destinations.contains_key(b"after-budget".as_slice()));
    }

    #[test]
    fn malformed_kids_consume_the_node_budget() {
        let (mut document, pages) = document_with_pages(1);
        let late_leaf = document.add_object(dictionary! {
            "Names" => vec![Object::string_literal("after-budget"), page_destination(pages[0])],
        });
        let mut kids = vec![Object::Null; MAX_NAME_TREE_NODES];
        kids.push(Object::Reference(late_leaf));
        let dests_id = document.add_object(dictionary! { "Kids" => kids });
        let names_id = document.add_object(dictionary! { "Dests" => dests_id });
        document.catalog_mut().unwrap().set("Names", names_id);

        let destinations = collect_named_destinations(&document);
        assert!(!destinations.contains_key(b"after-budget".as_slice()));
    }

    #[test]
    fn retains_relative_nested_depth() {
        let (mut document, pages) = document_with_pages(1);
        let child_id = document.add_object(dictionary! {
            "Title" => Object::string_literal("Child"),
            "Dest" => page_destination(pages[0]),
        });
        let root_id = document.add_object(dictionary! {
            "Title" => Object::string_literal("Root"),
            "Dest" => page_destination(pages[0]),
            "First" => child_id,
        });
        attach_outlines(&mut document, root_id);

        let entries = extract_outline_entries(&document);
        assert_eq!(entries.iter().map(|entry| entry.depth).collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn stops_at_malformed_sibling_cycle() {
        let (mut document, pages) = document_with_pages(1);
        let first_id = document.new_object_id();
        let second_id = document.new_object_id();
        document.objects.insert(
            first_id,
            Object::Dictionary(dictionary! {
                "Title" => Object::string_literal("First"),
                "Dest" => page_destination(pages[0]),
                "Next" => second_id,
            }),
        );
        document.objects.insert(
            second_id,
            Object::Dictionary(dictionary! {
                "Title" => Object::string_literal("Second"),
                "Dest" => page_destination(pages[0]),
                "Next" => first_id,
            }),
        );
        attach_outlines(&mut document, first_id);

        let entries = extract_outline_entries(&document);
        assert_eq!(
            entries.iter().map(|entry| entry.title.as_str()).collect::<Vec<_>>(),
            vec!["First", "Second"]
        );
    }

    #[test]
    fn stops_at_malformed_child_cycle() {
        let (mut document, pages) = document_with_pages(1);
        let root_id = document.new_object_id();
        document.objects.insert(
            root_id,
            Object::Dictionary(dictionary! {
                "Title" => Object::string_literal("Root"),
                "Dest" => page_destination(pages[0]),
                "First" => root_id,
            }),
        );
        attach_outlines(&mut document, root_id);

        let entries = extract_outline_entries(&document);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].depth, 0);
    }

    #[test]
    fn preserves_compatibility_uri_output_and_decodes_pdf_text() {
        let (mut document, pages) = document_with_pages(2);
        let title_id = document.add_object(Object::String(b"Rate \x8b".to_vec(), StringFormat::Literal));
        let url_id = document.add_object(Object::string_literal("https://example.com"));
        let uri_id = document.add_object(dictionary! {
            "Title" => title_id,
            "A" => dictionary! {
                "S" => "URI",
                "URI" => url_id,
            },
        });
        let page_id = document.add_object(dictionary! {
            "Title" => Object::string_literal("Page two"),
            "Dest" => page_destination(pages[1]),
            "Next" => uri_id,
        });
        attach_outlines(&mut document, page_id);

        let uris = extract_bookmarks(&document);
        assert_eq!(uris.len(), 2);
        assert_eq!(uris[0].url, "#page=2");
        assert_eq!(uris[0].label.as_deref(), Some("Page two"));
        assert_eq!(uris[0].page, Some(2));
        assert_eq!(uris[0].kind, UriKind::Anchor);
        assert_eq!(uris[1].url, "https://example.com");
        assert_eq!(uris[1].label.as_deref(), Some("Rate ‰"));
        assert_eq!(uris[1].page, None);
        assert_eq!(uris[1].kind, UriKind::Hyperlink);
    }

    #[test]
    fn missing_outlines_returns_no_entries() {
        let (document, _) = document_with_pages(1);
        assert!(extract_outline_entries(&document).is_empty());
        assert!(extract_bookmarks(&document).is_empty());
    }
}
