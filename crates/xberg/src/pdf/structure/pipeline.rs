//! Main PDF-to-Markdown pipeline orchestrator (native backend).

// TODO(xberg-io/xberg#1567): 4 cyclomatic-complexity and 25 size/complexity findings
// in this file, currently excluded via the quality-debt baseline in alef.toml. Splitting
// these needs compiler-in-the-loop verification, not a mechanical pass. Delete this
// note and the file's baseline entry together once it goes green. Help wanted.

use std::borrow::Cow;

use crate::pdf::bookmarks::PdfOutlineEntry;
use crate::pdf::error::Result;
use crate::pdf::hierarchy::{BoundingBox, SegmentData, TextBlock, assign_heading_levels_smart, cluster_font_sizes};
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use super::assembly::assemble_internal_document;
use super::classify::{
    classify_paragraphs, demote_heading_runs, demote_structure_annotation_headings, demote_unnumbered_subsections,
    is_body_size_bold_heading_candidate, is_body_size_bold_signal, is_numbered_section_heading, mark_arxiv_noise,
    mark_cross_page_repeating_short_text, mark_cross_page_repeating_text, refine_heading_hierarchy,
};
use super::constants::{FULL_LINE_FRACTION, MIN_BLOCKS_FOR_FONT_HEADING, MIN_HEADING_FONT_GAP, MIN_HEADING_FONT_RATIO};
use super::lines::{is_cjk_char, segments_need_space};
use super::paragraphs::{merge_continuation_paragraphs, split_embedded_list_items};
use super::text_repair::{
    MIN_LIGATURE_WITNESS_WORD_LEN, WordWitnesses, apply_to_all_segments, clean_duplicate_punctuation,
    collapse_spaced_hyphens, expand_ligatures_with_space_absorption, normalize_text_encoding, normalize_unicode_text,
    repair_contextual_ligatures, repair_ligature_spaces,
};
use super::types::{LayoutHint, PdfParagraph};

const SPARSE_REPEATED_TIER_MIN_PAGES: usize = 2;
const SPARSE_FONT_TIER_CLUSTER_COUNT: usize = 2;
const MIN_BODY_SIZE_BOLD_SIGNALS: usize = 3;
const MAX_OTHER_HEADING_RATIO: usize = 2;
const BODY_SIZE_BOLD_ALIGNMENT_TOLERANCE_EM: f32 = 1.5;
const MIN_OPENED_BODY_WORDS: usize = 4;
const SAME_ROW_MIN_VERTICAL_OVERLAP_RATIO: f32 = 0.5;
/// Font-size "same tier" tolerance (absolute, in the unit `font_size` happens to carry —
/// points for native PDFs, a render-DPI-dependent pixel measurement for OCR segments).
///
/// This IS scale-dependent, and a naive ratio-of-centroid conversion was tried and
/// reverted: forcing `cluster_font_sizes(_, 2)` on a sparse (<5 block) document routinely
/// produces two centroids that are themselves not tight (e.g. a genuine 22pt/21pt/12pt
/// three-tier native document forced into k=2 merges 22 and 21 into one ~21.7 centroid).
/// The tight 0.5pt absolute tolerance deliberately rejects that merge as "not narrow
/// enough" via `has_only_two_narrow_font_tiers`, which is what keeps a non-repeated 21pt
/// "Display prose" line (see `test_build_heading_map_sparse_multi_page_does_not_promote_
/// non_repeated_intermediate_tier`) out of the repeated-heading cluster. A ratio tolerance
/// wide enough to be useful for OCR pixel noise (e.g. 5% of a ~21px cluster) is also wide
/// enough to swallow that native 0.667pt merge slop, which flips `find_heading_level` from
/// `None` to `Some(2)` for the 21pt line and regresses that guard. No tolerance value is
/// simultaneously tight enough to guard native merge slop and loose enough for OCR pixel
/// noise, because both slop magnitudes scale with the *same* input (the forced-k=2
/// centroid), so this stays absolute — see the report for the full trade-off. ~keep
const SPARSE_FONT_TIER_TOLERANCE: f32 = 0.5;
// A tier repeated at the top of multiple pages represents peer sections, not a
// unique document title; reserve H1 for a title and emit these sections as H2. ~keep
const SPARSE_REPEATED_TIER_HEADING_LEVEL: u8 = 2;

type HeadingMap = Vec<(f32, Option<u8>)>;

/// Lowercased `(left, right)` word pairs the document itself writes as a single
/// hyphenated token elsewhere in the text, gathered once per document (#1543).
/// Threaded alongside [`HeadingMap`] as a document-scoped shared reference. ~keep
pub(super) type HyphenWitnesses = ahash::AHashSet<(String, String)>;

/// Document-scoped text-repair evidence, collected once per document by
/// [`collect_hyphen_witnesses`] and [`collect_word_witnesses`] and threaded through
/// paragraph assembly as a single shared reference, alongside [`HeadingMap`]. ~keep
#[derive(Default)]
struct TextRepairWitnesses {
    hyphens: HyphenWitnesses,
    words: WordWitnesses,
}

fn sparse_multi_page_heading_map(
    all_page_segments: &[Vec<SegmentData>],
    heuristic_pages: &[usize],
    all_blocks: &[TextBlock],
    has_struct_tree_blocks: bool,
) -> Result<Option<HeadingMap>> {
    if has_struct_tree_blocks || heuristic_pages.len() < SPARSE_REPEATED_TIER_MIN_PAGES {
        return Ok(None);
    }

    let clusters = cluster_font_sizes(all_blocks, SPARSE_FONT_TIER_CLUSTER_COUNT)?;
    if clusters.len() != SPARSE_FONT_TIER_CLUSTER_COUNT {
        return Ok(None);
    }

    let has_only_two_narrow_font_tiers = all_blocks.iter().all(|block| {
        block.font_size.is_finite()
            && clusters
                .iter()
                .any(|cluster| (block.font_size - cluster.centroid).abs() <= SPARSE_FONT_TIER_TOLERANCE)
    });
    if !has_only_two_narrow_font_tiers {
        return Ok(None);
    }

    let heading_font_size = clusters[0].centroid;
    let body_font_size = clusters[1].centroid;
    let has_distinct_body_tier = heading_font_size - body_font_size > SPARSE_FONT_TIER_TOLERANCE;
    let clears_font_gate = heading_font_size >= body_font_size * MIN_HEADING_FONT_RATIO
        && heading_font_size >= body_font_size + MIN_HEADING_FONT_GAP;
    if !has_distinct_body_tier || !clears_font_gate {
        return Ok(None);
    }

    let repeated_pages: ahash::AHashSet<usize> = heuristic_pages
        .iter()
        .copied()
        .filter(|&page_index| {
            all_page_segments[page_index]
                .iter()
                .find(|segment| !segment.text.trim().is_empty())
                .is_some_and(|segment| {
                    segment.font_size.is_finite()
                        && (segment.font_size - heading_font_size).abs() <= SPARSE_FONT_TIER_TOLERANCE
                })
        })
        .collect();
    if repeated_pages.len() < SPARSE_REPEATED_TIER_MIN_PAGES {
        return Ok(None);
    }

    Ok(Some(
        clusters
            .iter()
            .map(|cluster| {
                let level = ((cluster.centroid - heading_font_size).abs() <= SPARSE_FONT_TIER_TOLERANCE)
                    .then_some(SPARSE_REPEATED_TIER_HEADING_LEVEL);
                (cluster.centroid, level)
            })
            .collect(),
    ))
}

/// Stage 2: Cluster font sizes globally and assign heading levels.
///
/// Returns (heading_map, set of struct-tree page indices needing font-size classification).
#[allow(clippy::type_complexity)]
fn build_heading_map(
    all_page_segments: &[Vec<SegmentData>],
    struct_tree_results: &[Option<Vec<PdfParagraph>>],
    heuristic_pages: &[usize],
    k_clusters: usize,
) -> Result<(Vec<(f32, Option<u8>)>, ahash::AHashSet<usize>)> {
    let struct_tree_needs_classify: ahash::AHashSet<usize> = struct_tree_results
        .iter()
        .enumerate()
        .filter_map(|(i, result)| {
            result.as_ref().and_then(|paragraphs| {
                let has_headings = paragraphs.iter().any(|p| p.heading_level.is_some());
                let has_untagged_bold = paragraphs
                    .iter()
                    .any(|p| p.heading_level.is_none() && p.is_bold && !p.is_list_item);
                if (!has_headings && has_font_size_variation(paragraphs)) || has_untagged_bold {
                    Some(i)
                } else {
                    None
                }
            })
        })
        .collect();

    let mut all_blocks: Vec<TextBlock> = Vec::new();
    let empty_bbox = BoundingBox {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };
    // The text is carried so `assign_heading_levels_smart` can pick the body
    // cluster by character mass (char-weighted body size). Leaving it empty makes
    // every cluster tie at length 0, so `max_by_key` falls back to the smallest
    // font as "body" and over-promotes every larger run to a heading. ~keep
    for &i in heuristic_pages {
        for seg in &all_page_segments[i] {
            if seg.text.trim().is_empty() {
                continue;
            }
            all_blocks.push(TextBlock {
                text: seg.text.clone(),
                bbox: empty_bbox,
                font_size: seg.font_size,
            });
        }
    }
    for &i in &struct_tree_needs_classify {
        if let Some(paragraphs) = &struct_tree_results[i] {
            for para in paragraphs {
                let text = if !para.text.is_empty() {
                    para.text.clone()
                } else {
                    para.lines
                        .iter()
                        .flat_map(|l| l.segments.iter())
                        .map(|s| s.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                all_blocks.push(TextBlock {
                    text,
                    bbox: empty_bbox,
                    font_size: para.dominant_font_size,
                });
            }
        }
    }

    let paragraph_count = all_blocks.len();
    let heading_map = if all_blocks.is_empty() {
        Vec::new()
    } else if paragraph_count < MIN_BLOCKS_FOR_FONT_HEADING {
        if let Some(map) = sparse_multi_page_heading_map(
            all_page_segments,
            heuristic_pages,
            &all_blocks,
            !struct_tree_needs_classify.is_empty(),
        )? {
            tracing::debug!(
                paragraph_count,
                "heading map: promoting a repeated sparse font tier across pages"
            );
            map
        } else {
            // Sparsity gate: too few text blocks to establish a reliable body-font
            // baseline. Return a body-only map (every cluster centroid mapped to
            // `None`) and skip both k-means heading promotion and the fallback
            // title promotion, so a lone larger line on a cover/title/one-line
            // document is not over-promoted to a heading. ~keep
            tracing::debug!(
                paragraph_count,
                min_blocks = MIN_BLOCKS_FOR_FONT_HEADING,
                "heading map: document too sparse for font-size heading inference; suppressing promotion"
            );
            let clusters = cluster_font_sizes(&all_blocks, 1)?;
            clusters.iter().map(|c| (c.centroid, None)).collect()
        }
    } else {
        let effective_k = if paragraph_count < 20 {
            k_clusters.min(2usize.max(paragraph_count / 4))
        } else {
            k_clusters
        };

        let clusters = cluster_font_sizes(&all_blocks, effective_k)?;
        let mut map = assign_heading_levels_smart(&clusters, MIN_HEADING_FONT_RATIO);

        let has_any_heading = map.iter().any(|(_, level)| level.is_some());
        if !has_any_heading && !heuristic_pages.is_empty() {
            let first_page = heuristic_pages[0];
            let first_seg_font = all_page_segments[first_page]
                .iter()
                .find(|s| !s.text.trim().is_empty())
                .map(|s| s.font_size);

            if let Some(first_font) = first_seg_font {
                let mut sizes: Vec<f32> = all_blocks.iter().map(|b| b.font_size).collect();
                sizes.sort_by(|a, b| a.total_cmp(b));
                let median = if sizes.is_empty() { 0.0 } else { sizes[sizes.len() / 2] };

                if median > 0.0
                    && first_font >= median * 1.2
                    // Absolute-unit match tolerance; kept as-is for the same reason
                    // `SPARSE_FONT_TIER_TOLERANCE` was kept absolute — see its doc comment.
                    && let Some(entry) = map.iter_mut().find(|(fs, _)| (*fs - first_font).abs() < 0.5)
                {
                    entry.1 = Some(1);
                }
            }
        }

        map
    };

    Ok((heading_map, struct_tree_needs_classify))
}

fn bbox_coordinates_are_finite(bbox: (f32, f32, f32, f32)) -> bool {
    let (left, bottom, right, top) = bbox;
    left.is_finite() && bottom.is_finite() && right.is_finite() && top.is_finite()
}

fn shares_same_text_row(candidate: &PdfParagraph, following: &PdfParagraph, is_same_page: bool) -> bool {
    if !is_same_page {
        return false;
    }
    let (Some(candidate_bbox), Some(following_bbox)) = (candidate.block_bbox, following.block_bbox) else {
        return false;
    };
    if !bbox_coordinates_are_finite(candidate_bbox) || !bbox_coordinates_are_finite(following_bbox) {
        return false;
    }

    let candidate_bottom = candidate_bbox.1.min(candidate_bbox.3);
    let candidate_top = candidate_bbox.1.max(candidate_bbox.3);
    let following_bottom = following_bbox.1.min(following_bbox.3);
    let following_top = following_bbox.1.max(following_bbox.3);
    let minimum_height = (candidate_top - candidate_bottom).min(following_top - following_bottom);
    let overlap = candidate_top.min(following_top) - candidate_bottom.max(following_bottom);
    minimum_height > 0.0 && overlap.max(0.0) / minimum_height >= SAME_ROW_MIN_VERTICAL_OVERLAP_RATIO
}

fn opens_aligned_body_block(
    candidate: &PdfParagraph,
    following: &PdfParagraph,
    body_font_size: f32,
    is_same_page: bool,
) -> bool {
    if following.word_count < MIN_OPENED_BODY_WORDS
        || following.heading_level.is_some()
        || is_body_size_bold_signal(following, body_font_size)
    {
        return false;
    }

    if candidate
        .block_bbox
        .is_some_and(|bbox| !bbox_coordinates_are_finite(bbox))
        || following
            .block_bbox
            .is_some_and(|bbox| !bbox_coordinates_are_finite(bbox))
    {
        return false;
    }
    let (Some(candidate_bbox), Some(following_bbox)) = (candidate.block_bbox, following.block_bbox) else {
        return true;
    };

    if shares_same_text_row(candidate, following, is_same_page) {
        return false;
    }

    let candidate_left = candidate_bbox.0.min(candidate_bbox.2);
    let following_left = following_bbox.0.min(following_bbox.2);
    candidate_left <= following_left + body_font_size * BODY_SIZE_BOLD_ALIGNMENT_TOLERANCE_EM
}

fn is_explicit_numbered_body_size_section(text: &str) -> bool {
    let trimmed = text.trim();
    if !is_numbered_section_heading(trimmed) {
        return false;
    }

    let bytes = trimmed.as_bytes();
    let roman_prefix_length = bytes
        .iter()
        .position(|byte| !b"IVXLCDM".contains(byte))
        .unwrap_or(bytes.len());
    if roman_prefix_length == 0 || bytes.get(roman_prefix_length) != Some(&b' ') {
        return true;
    }

    let remainder = trimmed[roman_prefix_length..].trim_start();
    remainder.chars().any(char::is_alphabetic)
        && remainder
            .chars()
            .filter(|character| character.is_alphabetic())
            .all(char::is_uppercase)
}

fn body_size_heading_eligibility(all_pages: &[Vec<PdfParagraph>], body_font_size: f32) -> Vec<Vec<bool>> {
    let mut next_content: Option<(usize, &PdfParagraph)> = None;
    let mut eligibility = Vec::with_capacity(all_pages.len());
    for (page_index, page) in all_pages.iter().enumerate().rev() {
        let mut page_eligibility = Vec::with_capacity(page.len());
        for paragraph in page.iter().rev() {
            let is_candidate = is_body_size_bold_heading_candidate(paragraph, body_font_size);
            let is_explicit_section = is_explicit_numbered_body_size_section(paragraph_text_raw(paragraph).trim());
            let has_non_finite_geometry = paragraph
                .block_bbox
                .is_some_and(|bbox| !bbox_coordinates_are_finite(bbox))
                || next_content.is_some_and(|(_, following)| {
                    following
                        .block_bbox
                        .is_some_and(|bbox| !bbox_coordinates_are_finite(bbox))
                });
            page_eligibility.push(
                is_candidate
                    && !has_non_finite_geometry
                    && if is_explicit_section {
                        !next_content.is_some_and(|(following_page_index, following)| {
                            shares_same_text_row(paragraph, following, page_index == following_page_index)
                        })
                    } else {
                        next_content.is_some_and(|(following_page_index, following)| {
                            opens_aligned_body_block(
                                paragraph,
                                following,
                                body_font_size,
                                page_index == following_page_index,
                            )
                        })
                    },
            );
            if paragraph.word_count > 0 && !paragraph.is_page_furniture {
                next_content = Some((page_index, paragraph));
            }
        }
        page_eligibility.reverse();
        eligibility.push(page_eligibility);
    }
    eligibility.reverse();
    eligibility
}

fn promote_repeated_body_size_bold_headings(all_pages: &mut [Vec<PdfParagraph>], body_font_size: Option<f32>) {
    let Some(body_font_size) = body_font_size else {
        return;
    };
    let eligible = body_size_heading_eligibility(all_pages, body_font_size);
    let signal_count = eligible.iter().flatten().filter(|&&is_eligible| is_eligible).count();
    let other_heading_count = all_pages
        .iter()
        .flatten()
        .filter(|paragraph| paragraph.heading_level.is_some())
        .count();
    if signal_count < MIN_BODY_SIZE_BOLD_SIGNALS
        || signal_count <= other_heading_count.saturating_mul(MAX_OTHER_HEADING_RATIO)
    {
        return;
    }

    for (page, page_eligibility) in all_pages.iter_mut().zip(eligible) {
        for (paragraph, is_eligible) in page.iter_mut().zip(page_eligibility) {
            if is_eligible {
                paragraph.heading_level = Some(3);
            }
        }
    }
}

/// Build a heading map from structure-tree-assigned roles on segments.
///
/// Instead of clustering font sizes heuristically, this examines the
/// `assigned_role` field on each segment (populated from the PDF structure tree).
/// Each unique font size is mapped to the heading level most commonly assigned
/// to segments at that size. Font sizes with no assigned role are treated as body text.
fn build_heading_map_from_assigned_roles(all_page_segments: &[Vec<SegmentData>]) -> Vec<(f32, Option<u8>)> {
    use std::collections::HashMap;

    let mut size_roles: HashMap<u32, Vec<Option<u8>>> = HashMap::new();
    for page_segs in all_page_segments {
        for seg in page_segs {
            if seg.text.trim().is_empty() {
                continue;
            }
            let key = (seg.font_size * 10.0).round() as u32;
            size_roles.entry(key).or_default().push(seg.assigned_role);
        }
    }

    let mut heading_map: Vec<(f32, Option<u8>)> = size_roles
        .into_iter()
        .map(|(quantized_size, roles)| {
            let font_size = quantized_size as f32 / 10.0;
            let total = roles.len();
            let mut level_counts: HashMap<u8, usize> = HashMap::new();
            let mut none_count = 0usize;
            for role in &roles {
                match role {
                    Some(level) => *level_counts.entry(*level).or_default() += 1,
                    None => none_count += 1,
                }
            }
            let dominant_level = level_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .and_then(|(level, count)| if count * 2 >= total { Some(level) } else { None });

            if none_count > total / 2 && dominant_level.is_none() {
                (font_size, None)
            } else {
                (font_size, dominant_level)
            }
        })
        .collect();

    heading_map.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    heading_map
}

/// Font-size tolerance (points) for merging consecutive raw segments into one
/// logical block in [`count_logical_blocks`]. Matches the font-change
/// threshold `blocks_to_paragraphs` uses to decide paragraph breaks, so a
/// single logical line that a font extractor split into several same-size
/// runs (ligature repair, kerning artifacts, mid-word splits) is not
/// double-counted as multiple blocks.
const LOGICAL_BLOCK_FONT_TOLERANCE: f32 = 1.5;

/// Count logical text blocks by merging consecutive same-role, same-size
/// segments, rather than counting raw segments.
///
/// Raw segment extraction can split one visual line into several runs (a
/// mid-word split from a font-encoding quirk, a bold/italic switch inside a
/// single sentence) that all carry the same `assigned_role`. Counting those
/// raw segments as separate "blocks" over-counts a document's real size and
/// can push a genuinely tiny document (see `hello_structure.pdf`,
/// `issue-987-test.pdf`) above the sparsity floor it should fall under.
/// Consecutive segments on the same page with the same `assigned_role` and a
/// font size within [`LOGICAL_BLOCK_FONT_TOLERANCE`] points collapse into a
/// single block, matching the granularity `total_paragraphs` eventually
/// reports after paragraph assembly.
fn count_logical_blocks(all_page_segments: &[Vec<SegmentData>]) -> usize {
    let mut total = 0usize;
    for page_segs in all_page_segments {
        let mut prev: Option<&SegmentData> = None;
        for seg in page_segs {
            if seg.text.trim().is_empty() {
                continue;
            }
            let continues_prev = prev.is_some_and(|p| {
                p.assigned_role == seg.assigned_role
                    && (p.font_size - seg.font_size).abs() <= LOGICAL_BLOCK_FONT_TOLERANCE
            });
            if !continues_prev {
                total += 1;
            }
            prev = Some(seg);
        }
    }
    total
}

/// Suppress structure-tree heading roles on documents too sparse to trust them.
///
/// A tagged PDF's structure tree is normally a reliable ground truth for
/// heading levels, but on a document with only a handful of text blocks (a
/// one-line note, a cover slide, a two-paragraph test fixture) the same
/// per-block noise that makes font-size clustering unreliable on small
/// samples (see [`MIN_BLOCKS_FOR_FONT_HEADING`] on the heuristic path) also
/// undermines the structure tree: a single mis-tagged or inconsistently
/// authored run is enough to make an entire tiny document look like it is
/// "mostly headings" even when nothing in it is a genuine section heading.
/// Below the same block floor, heading roles are suppressed regardless of
/// whether a body tier is present, matching the heuristic path's rule that
/// a reliable body-font baseline (or, here, a reliable heading/body
/// contrast) needs more than a couple of paragraphs to establish.
///
/// When the condition holds, every segment's `assigned_role` is cleared so
/// paragraph classification (which reads `assigned_role` directly,
/// bypassing the heading map) also treats the document as plain text, and
/// `heading_map` is rewritten to a single body-only entry.
///
/// Returns `true` when suppression fired.
fn suppress_all_heading_roles_when_sparse_and_untrusted(
    heading_map: &mut Vec<(f32, Option<u8>)>,
    all_page_segments: &mut [Vec<SegmentData>],
) -> bool {
    let total_blocks = count_logical_blocks(all_page_segments);
    let has_any_heading = heading_map.iter().any(|(_, level)| level.is_some());

    if total_blocks == 0 || total_blocks >= MIN_BLOCKS_FOR_FONT_HEADING || !has_any_heading {
        return false;
    }

    tracing::debug!(
        total_blocks,
        min_blocks = MIN_BLOCKS_FOR_FONT_HEADING,
        "structure tree: document too sparse to trust tagged heading roles; suppressing all heading roles"
    );

    for page_segs in all_page_segments.iter_mut() {
        for seg in page_segs.iter_mut() {
            seg.assigned_role = None;
        }
    }
    heading_map.clear();
    true
}

/// Promote an untagged document-title font tier above structure-tree headings.
///
/// Word processors tag the document title with a non-heading structure type
/// (e.g. LibreOffice's "Title" style resolves to a non-`H*` element), so
/// `build_heading_map_from_assigned_roles` classifies it as body text even
/// though it is visually the top-level heading. When such a tier exists —
/// strictly larger than every tagged heading font, bold, few segments, and
/// present on the first page — assign it level 1 and demote all tagged
/// heading levels by one (Title = h1, tagged H1 = h2, ...), matching the
/// pandoc/HTML convention that the document title outranks section headings.
///
/// Returns `true` when a title tier was promoted. The caller must then also
/// demote the per-segment `assigned_role` values (see
/// `demote_assigned_roles`), because paragraph classification honours
/// `assigned_role` directly, bypassing the heading map.
fn promote_untagged_document_title(
    heading_map: &mut [(f32, Option<u8>)],
    all_page_segments: &[Vec<SegmentData>],
) -> bool {
    /// A title is a handful of segments at most; more means a body/pull-quote tier.
    const MAX_TITLE_SEGMENTS: usize = 3;

    let Some(max_heading_font) = heading_map
        .iter()
        .filter(|(_, level)| level.is_some())
        .map(|(font, _)| *font)
        .fold(None, |acc: Option<f32>, f| Some(acc.map_or(f, |a| a.max(f))))
    else {
        return false;
    };

    let candidate = heading_map
        .iter()
        .position(|(font, level)| level.is_none() && *font > max_heading_font);
    let Some(candidate_idx) = candidate else {
        return false;
    };
    let candidate_font = heading_map[candidate_idx].0;

    let mut tier_segments = 0usize;
    let mut all_bold = true;
    let mut on_first_page = false;
    for (page_idx, page_segs) in all_page_segments.iter().enumerate() {
        for seg in page_segs {
            if seg.text.trim().is_empty() || (seg.font_size - candidate_font).abs() >= 0.05 {
                continue;
            }
            tier_segments += 1;
            all_bold &= seg.is_bold;
            on_first_page |= page_idx == 0;
        }
    }
    if tier_segments == 0 || tier_segments > MAX_TITLE_SEGMENTS || !all_bold || !on_first_page {
        return false;
    }

    tracing::debug!(
        title_font = candidate_font,
        max_heading_font,
        tier_segments,
        "structure tree: promoting untagged document-title tier to h1, demoting tagged levels"
    );
    for (font, level) in heading_map.iter_mut() {
        if let Some(l) = level {
            *level = Some((*l + 1).min(6));
        } else if (*font - candidate_font).abs() < 0.05 {
            *level = Some(1);
        }
    }
    true
}

/// Demote every structure-tree-assigned heading role by one level (capped at 6).
///
/// Companion to `promote_untagged_document_title`: paragraph classification
/// (`bridge.rs`) uses `assigned_role` directly as "the author's stated intent",
/// so the map-level demotion must be mirrored on the segments themselves.
fn demote_assigned_roles(all_page_segments: &mut [Vec<SegmentData>]) {
    for page_segs in all_page_segments.iter_mut() {
        for seg in page_segs.iter_mut() {
            if let Some(role) = seg.assigned_role {
                seg.assigned_role = Some((role + 1).min(6));
            }
        }
    }
}

/// Per-page input bundle for Stage 3 parallel processing.
///
/// Each page's data is pre-extracted before `into_par_iter` so all threads
/// receive owned, non-overlapping slices of the document's data.
struct PageInput {
    /// Index of this page in the document (0-based).
    page_index: usize,
    /// Paragraphs from the PDF structure tree, if extraction succeeded.
    struct_paragraphs: Option<Vec<PdfParagraph>>,
    /// Segments from heuristic extraction (non-empty only when `struct_paragraphs` is `None`).
    heuristic_segments: Vec<SegmentData>,
    /// Layout hints for this page, if layout detection was run.
    page_hints: Option<Vec<LayoutHint>>,
    /// Footprint and cell text of tables successfully extracted for this page.
    table_bboxes: Vec<TableCoverage>,
    /// Whether native semantic classification should be preserved while layout
    /// hints continue to control reading order and record region provenance.
    preserve_native_semantics: bool,
    /// Whether layout geometry should reorder native paragraphs on this page.
    /// Semantic refinement runs in native order before this spatial order is
    /// applied during final assembly.
    use_layout_reading_order: bool,
    /// Per-hint validation results from CC analysis (parallel to page_hints).
    /// Empty when layout-detection is not active.
    #[cfg(feature = "layout-detection")]
    hint_validations: Vec<super::regions::layout_validation::RegionValidation>,
    /// Actual PDF page width in points, used by layout reading-order refinement.
    #[cfg(feature = "layout-detection")]
    page_width_pts: Option<f32>,
    /// Whether this page's structure-tree paragraphs need font-size classification.
    needs_classify: bool,
    /// Y-coordinates of paragraph gaps detected from segment boundaries.
    paragraph_gap_ys: Vec<f32>,
    /// When true, paragraphs classified as `PageHeader` by the layout model are
    /// preserved rather than marked as furniture. Mirrors `ContentFilterConfig::include_headers`.
    include_headers: bool,
    /// When true, paragraphs classified as `PageFooter` by the layout model are
    /// preserved rather than marked as furniture. Mirrors `ContentFilterConfig::include_footers`.
    include_footers: bool,
    /// When true, paragraphs classified as `Footnote` by the layout model are
    /// preserved rather than marked as furniture. Mirrors `ContentFilterConfig::include_footnotes`.
    include_footnotes: bool,
}

/// Process a single page's data through Stage 3: classification, text repair,
/// layout overrides, dehyphenation, and list splitting.
///
/// This function is intentionally free of any shared mutable state so it can be
/// called from multiple threads via `rayon::par_iter`.
fn process_single_page(
    input: PageInput,
    heading_map: &[(f32, Option<u8>)],
    doc_body_font_size: Option<f32>,
    witnesses: &TextRepairWitnesses,
) -> Vec<PdfParagraph> {
    let PageInput {
        page_index: i,
        struct_paragraphs,
        heuristic_segments,
        page_hints,
        table_bboxes,
        preserve_native_semantics,
        use_layout_reading_order,
        #[cfg(feature = "layout-detection")]
        hint_validations,
        #[cfg(feature = "layout-detection")]
        page_width_pts,
        needs_classify,
        paragraph_gap_ys,
        include_headers,
        include_footers,
        include_footnotes,
    } = input;
    #[cfg(not(feature = "layout-detection"))]
    let _ = preserve_native_semantics;
    #[cfg(not(feature = "layout-detection"))]
    let _ = use_layout_reading_order;
    if let Some(mut paragraphs) = struct_paragraphs {
        apply_text_repair_to_structure_tree_paragraphs(&mut paragraphs, true, witnesses);
        if needs_classify {
            tracing::debug!(
                page = i,
                "PDF structure pipeline: classifying struct tree page via font-size clustering"
            );
            classify_paragraphs(&mut paragraphs, heading_map);
        }
        merge_continuation_paragraphs(&mut paragraphs);
        synchronize_paragraph_text_metadata(&mut paragraphs);
        merge_spatial_footnote_markers(&mut paragraphs);
        if let Some(ref hints) = page_hints {
            let classification_hints = regular_layout_hints(hints);
            super::layout_classify::apply_layout_overrides(
                &mut paragraphs,
                &classification_hints,
                0.5,
                0.2,
                doc_body_font_size,
            );
            un_mark_layout_furniture_per_config(&mut paragraphs, include_headers, include_footers, include_footnotes);
            tracing::debug!(
                page = i,
                headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
                lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
                furniture = paragraphs.iter().filter(|p| p.is_page_furniture).count(),
                "layout overrides applied"
            );
            retain_page_furniture_safely(&mut paragraphs);
        }
        demote_structure_annotation_headings(&mut paragraphs);
        paragraphs
    } else {
        let page_segments = heuristic_segments;
        tracing::debug!(
            page = i,
            segments = page_segments.len(),
            has_layout_hints = page_hints.is_some(),
            "process_single_page: heuristic path"
        );
        let page_segments = filter_segments_by_table_bboxes(page_segments, &table_bboxes);
        #[cfg(feature = "layout-detection")]
        let mut paragraphs = if let Some(ref hints) = page_hints {
            let wrapper_ownership = wrapper_ownership_by_hint(hints, &hint_validations);
            if use_layout_reading_order
                && crate::extractors::pdf::reading_order::has_eligible_layout_hints(hints, &wrapper_ownership)
            {
                process_layout_segment_groups(
                    page_segments,
                    hints,
                    &wrapper_ownership,
                    LayoutParagraphContext {
                        heading_map,
                        paragraph_gap_ys: &paragraph_gap_ys,
                        doc_body_font_size,
                        include_headers,
                        include_footers,
                        include_footnotes,
                        page_width_pts,
                        apply_layout_overrides: !preserve_native_semantics,
                        witnesses,
                    },
                )
            } else {
                let mut paragraphs = segments_to_paragraphs(page_segments, heading_map, &paragraph_gap_ys, witnesses);
                let classification_hints = regular_layout_hints(hints);
                super::layout_classify::annotate_layout_classes(&mut paragraphs, &classification_hints, 0.5, 0.2);
                paragraphs
            }
        } else {
            segments_to_paragraphs(page_segments, heading_map, &paragraph_gap_ys, witnesses)
        };
        #[cfg(not(feature = "layout-detection"))]
        let mut paragraphs = segments_to_paragraphs(page_segments, heading_map, &paragraph_gap_ys, witnesses);
        tracing::debug!(
            page = i,
            paragraphs = paragraphs.len(),
            "heuristic paragraphs classified"
        );
        #[cfg(not(feature = "layout-detection"))]
        if let Some(ref hints) = page_hints {
            let classification_hints = regular_layout_hints(hints);
            super::layout_classify::apply_layout_overrides(
                &mut paragraphs,
                &classification_hints,
                0.5,
                0.2,
                doc_body_font_size,
            );
            un_mark_layout_furniture_per_config(&mut paragraphs, include_headers, include_footers, include_footnotes);
        }
        if page_hints.is_some() {
            tracing::debug!(
                page = i,
                headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
                lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
                furniture = paragraphs.iter().filter(|p| p.is_page_furniture).count(),
                "layout overrides applied"
            );
        }
        demote_structure_annotation_headings(&mut paragraphs);
        merge_spatial_footnote_markers(&mut paragraphs);
        retain_page_furniture_safely(&mut paragraphs);
        paragraphs
    }
}

const TABLE_DOMINANT_MIN_BODY_ROWS: usize = 40;
const TABLE_DOMINANT_MIN_VISIBLE_CHAR_SHARE: f64 = 0.85;
const TABLE_SPILL_MIN_PARAGRAPH_OVERLAP: f64 = 0.2;

/// Remove non-semantic spill inside table regions on overwhelmingly tabular pages.
///
/// Layout table crops can leave out-of-bounds table continuations as ordinary
/// paragraphs. The page-level dominance guards are only activation gates: a
/// paragraph is removed only when enough of its own geometry overlaps an emitted
/// table rectangle and its content looks like a row continuation or marker,
/// preserving surrounding headings and short explanatory prose.
fn suppress_table_dominant_paragraph_spill(pages: &mut [Vec<PdfParagraph>], emitted_tables: &[crate::types::Table]) {
    for (page_index, paragraphs) in pages.iter_mut().enumerate() {
        let page_number = page_index.saturating_add(1) as u32;
        let page_tables = emitted_tables
            .iter()
            .filter(|table| table.page_number == page_number)
            .collect::<Vec<_>>();
        let body_rows = page_tables
            .iter()
            .map(|table| table.cells.len().saturating_sub(1))
            .sum::<usize>();
        if body_rows < TABLE_DOMINANT_MIN_BODY_ROWS {
            continue;
        }

        let table_chars = page_tables
            .iter()
            .flat_map(|table| table.cells.iter().flatten())
            .map(|cell| visible_char_count(cell))
            .sum::<usize>();
        let paragraph_chars = paragraphs
            .iter()
            .map(paragraph_text_raw)
            .map(|text| visible_char_count(&text))
            .sum::<usize>();
        let total_chars = table_chars.saturating_add(paragraph_chars);
        let table_share = if total_chars == 0 {
            0.0
        } else {
            table_chars as f64 / total_chars as f64
        };
        if table_share < TABLE_DOMINANT_MIN_VISIBLE_CHAR_SHARE {
            continue;
        }

        let table_bboxes = page_tables
            .iter()
            .filter_map(|table| table.bounding_box.as_ref())
            .collect::<Vec<_>>();
        if table_bboxes.is_empty() {
            continue;
        }
        let before = paragraphs.len();
        paragraphs.retain(|paragraph| !is_table_crop_spill(paragraph, &table_bboxes));
        tracing::debug!(
            page = page_number,
            body_rows,
            table_share,
            removed = before.saturating_sub(paragraphs.len()),
            "table-dominant paragraph spill cleanup"
        );
    }
}

fn visible_char_count(text: &str) -> usize {
    text.chars().filter(|character| !character.is_whitespace()).count()
}

fn is_table_crop_spill(paragraph: &PdfParagraph, table_bboxes: &[&crate::types::BoundingBox]) -> bool {
    if is_preserved_table_page_annotation(paragraph) {
        return false;
    }
    let Some(paragraph_bbox) = paragraph_geometry_bbox(paragraph) else {
        return false;
    };
    if !table_bboxes.iter().any(|table_bbox| {
        table_paragraph_overlap_fraction(paragraph_bbox, table_bbox) >= TABLE_SPILL_MIN_PARAGRAPH_OVERLAP
    }) {
        return false;
    }

    let text = paragraph_text_raw(paragraph);
    let alphabetic_chars = text.chars().filter(|character| character.is_alphabetic()).count();
    let numeric_chars = text.chars().filter(|character| character.is_numeric()).count();
    paragraph.word_count >= 13 || alphabetic_chars < 5 || numeric_chars >= alphabetic_chars
}

fn table_paragraph_overlap_fraction(
    paragraph_bbox: (f32, f32, f32, f32),
    table_bbox: &crate::types::BoundingBox,
) -> f64 {
    let (raw_left, raw_bottom, raw_right, raw_top) = paragraph_bbox;
    let paragraph_left = raw_left.min(raw_right) as f64;
    let paragraph_right = raw_left.max(raw_right) as f64;
    let paragraph_bottom = raw_bottom.min(raw_top) as f64;
    let paragraph_top = raw_bottom.max(raw_top) as f64;
    let paragraph_area = (paragraph_right - paragraph_left) * (paragraph_top - paragraph_bottom);
    if paragraph_area <= 0.0 {
        return 0.0;
    }

    let intersection_width = (paragraph_right.min(table_bbox.x0.max(table_bbox.x1))
        - paragraph_left.max(table_bbox.x0.min(table_bbox.x1)))
    .max(0.0);
    let intersection_height = (paragraph_top.min(table_bbox.y0.max(table_bbox.y1))
        - paragraph_bottom.max(table_bbox.y0.min(table_bbox.y1)))
    .max(0.0);
    intersection_width * intersection_height / paragraph_area
}

fn is_preserved_table_page_annotation(paragraph: &PdfParagraph) -> bool {
    if paragraph.heading_level.is_some()
        || paragraph.caption_for.is_some()
        || paragraph.is_list_item
        || paragraph.is_code_block
        || paragraph.is_formula
        || matches!(
            paragraph.layout_class,
            Some(
                super::types::LayoutHintClass::Title
                    | super::types::LayoutHintClass::SectionHeader
                    | super::types::LayoutHintClass::Caption
                    | super::types::LayoutHintClass::Footnote
                    | super::types::LayoutHintClass::PageHeader
                    | super::types::LayoutHintClass::PageFooter
                    | super::types::LayoutHintClass::ListItem
                    | super::types::LayoutHintClass::Code
                    | super::types::LayoutHintClass::Formula
                    | super::types::LayoutHintClass::DocumentIndex
                    | super::types::LayoutHintClass::Form
                    | super::types::LayoutHintClass::KeyValueRegion
            )
        )
    {
        return true;
    }

    let text = paragraph_text_raw(paragraph);
    let label = text
        .trim_start()
        .split_once(char::is_whitespace)
        .map_or(text.trim(), |(first, _)| first)
        .trim_end_matches([':', '.'])
        .to_ascii_lowercase();
    matches!(label.as_str(), "note" | "notes" | "source" | "sources" | "table")
}

fn is_wrapper_layout_hint(hint: &LayoutHint) -> bool {
    hint.class_name.is_wrapper()
}

fn regular_layout_hints(hints: &[LayoutHint]) -> Vec<LayoutHint> {
    hints
        .iter()
        .filter(|hint| !is_wrapper_layout_hint(hint))
        .cloned()
        .collect()
}

fn segments_to_paragraphs(
    segments: Vec<SegmentData>,
    heading_map: &[(f32, Option<u8>)],
    paragraph_gap_ys: &[f32],
    witnesses: &TextRepairWitnesses,
) -> Vec<PdfParagraph> {
    let segments = order_segments_in_reading_frames(segments);
    let mut paragraphs = blocks_to_paragraphs(segments, heading_map, paragraph_gap_ys);
    apply_text_repair_to_structure_tree_paragraphs(&mut paragraphs, true, witnesses);
    reattach_detached_list_markers(&mut paragraphs, DetachedMarkerFrame::Native);
    merge_continuation_paragraphs(&mut paragraphs);
    synchronize_paragraph_text_metadata(&mut paragraphs);
    paragraphs
}

/// Master switch for [`reattach_detached_list_markers`].
///
/// Flip this single constant to `false` to build a control binary that differs
/// from the shipped one only in this behaviour; nothing else guards the pass.
const REATTACH_DETACHED_LIST_MARKERS: bool = true;

/// Suppress heading promotion for a fragment whose text starts lowercase or with
/// a sentence-continuation word (#712).
///
/// This is the fabrication signature of the OCR mid-line paragraph break: when
/// `font_change` (`(line.font_size - prev.font_size).abs() > 1.5`) splits a
/// physical line on intra-line ascender/descender noise, the stray tail
/// fragment almost always starts mid-sentence -- lowercase, or with "is", "of",
/// "and", and the like -- because a real sentence or heading boundary does not
/// land there. That stray fragment's own (noise-inflated) font size is then
/// read as `first.font_size` for the *next* paragraph and can clear the
/// heading-distance gate in [`super::classify::find_heading_level`], fabricating
/// a heading out of a sentence fragment (e.g. `### storage.`,
/// `### groundwork for future developments. Over time,`). Reuses
/// [`super::classify::starts_with_lowercase_or_continuation`], the same guard
/// the rescue pass already trusts for the identical judgment, so this adds no
/// new heuristic surface. Flip to `false` to restore pre-#712 behaviour.
const SUPPRESS_LOWERCASE_START_HEADINGS: bool = true;

/// How closely a detached marker's baseline must agree with the baseline of the
/// body line it is claimed to belong to, as a multiple of the larger of the two
/// font sizes. Scale-free by construction, so it behaves identically on
/// point-scale native input and on OCR font sizes of a different magnitude.
const DETACHED_MARKER_BASELINE_TOLERANCE_FONT_FACTOR: f32 = 0.6;

/// Largest hanging indent, measured from the marker's right edge to the body
/// line's left edge, as a multiple of the body font size. Real hanging indents
/// run a quarter to half an inch; this admits those while refusing to pair a
/// marker in one column with a body in another.
const DETACHED_MARKER_MAX_INDENT_FONT_FACTOR: f32 = 6.0;

/// Largest overlap tolerated in the other direction, as a multiple of the body
/// font size, so a marker whose measured width slightly overruns the body's
/// left edge still pairs.
const DETACHED_MARKER_MAX_OVERLAP_FONT_FACTOR: f32 = 0.5;

/// How many paragraphs ahead of a detached marker its body may sit. A marker
/// *column* emits every marker before any body ("(a)", "(b)", "(c)", then three
/// bodies), so the body is not necessarily the next paragraph.
///
/// Also reused by the OCR layout route's `adapters::reattach_ocr_layout_list_markers`.
pub(super) const DETACHED_MARKER_MAX_LOOKAHEAD: usize = 8;

/// Minimum word count of the body paragraph. Excludes single-token neighbours,
/// which is what a marker-shaped table column looks like.
///
/// `pub(super)` so `adapters::accepts_marker_run_body` (#729) can reuse it for
/// the marker-run/body-run pairing phase of `adapters::reattach_ocr_layout_list_markers`.
pub(super) const DETACHED_MARKER_MIN_BODY_WORDS: usize = 2;

/// Whether the two detached-list-marker reattachment passes (this module's
/// [`detached_list_marker`] and `adapters::ocr_detached_list_marker`) reject a
/// lone `*` and a bracketed integer `[N]` as marker paragraphs, on top of the
/// general [`is_bare_list_marker`] test.
///
/// Both shapes are ambiguous specifically in the *detached* (cross-paragraph)
/// case, where the marker paragraph can be reattached to a body many
/// paragraphs away: a standalone `*` line is also a bare multiplication sign
/// in isolated mathematical prose, and `[N]` is the standard printed
/// paragraph-number notation in reference works (e.g. Jung's Collected
/// Works), not a list marker. Reattaching either turns unrelated prose into a
/// fabricated list item. Flip to `false` to restore the pre-tightening
/// behaviour where both are accepted. Deliberately does NOT touch
/// `is_bare_list_marker` itself, which stays available to the *same-line*
/// split-marker cases in `blocks_to_paragraphs` and `finalize_paragraph`,
/// where the marker and body are already adjacent segments on one physical
/// line and this cross-paragraph ambiguity does not arise. ~keep
const EXCLUDE_AMBIGUOUS_DETACHED_MARKERS: bool = true;

/// Whether `text` is a marker shape that is ambiguous enough to reject in the
/// *detached* (cross-paragraph) reattachment passes even though
/// [`is_bare_list_marker`] accepts it. See [`EXCLUDE_AMBIGUOUS_DETACHED_MARKERS`].
fn is_ambiguous_detached_marker(text: &str) -> bool {
    let t = text.trim();
    if t == "*" {
        return true;
    }
    if let Some(inner) = t.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
        return !inner.is_empty() && inner.chars().all(|character| character.is_ascii_digit());
    }
    false
}

/// Narrower sibling of [`is_bare_list_marker`] used only by the two detached
/// (cross-paragraph) reattachment passes -- this module's
/// [`detached_list_marker`] and `adapters::ocr_detached_list_marker`. See
/// [`EXCLUDE_AMBIGUOUS_DETACHED_MARKERS`] for why the two predicates
/// deliberately differ.
pub(super) fn is_bare_detached_list_marker(text: &str) -> bool {
    is_bare_list_marker(text) && !(EXCLUDE_AMBIGUOUS_DETACHED_MARKERS && is_ambiguous_detached_marker(text))
}

/// Which reading frame [`accepts_detached_list_marker`] should compare geometry in.
///
/// `Native` is the pre-#760 behaviour: it reuses each segment's own
/// `rotation_degrees` via [`SegmentData::upright_baseline`] /
/// [`SegmentData::upright_advance_extent`], unchanged. `OcrOnPage` is the OCR
/// route's correction (#760): OCR segments always carry `rotation_degrees ==
/// 0.0` -- `rotation_degrees` on native text encodes that TEXT RUN's own
/// orientation on the page, and OCR's raster boxes have no analogous per-run
/// signal, so `adapters::make_ocr_pdf_line` hardcodes `0.0` -- but on a page
/// with a PDF `/Rotate`, the OCR raster stays MediaBox-oriented by design, so a
/// rotated page's segment `x`/`y`/`width`/`height` sit in the RASTER frame while
/// this predicate needs the UPRIGHT reading frame. `OcrOnPage(degrees)`
/// recovers that frame locally, from the page's own `/Rotate` value, without
/// writing anything back onto [`SegmentData`].
///
/// Writing the correction onto `SegmentData::rotation_degrees` globally instead
/// was tried and rejected: it silently activates roughly 82 other
/// `is_unrotated()` / `has_same_rotation()` / `upright_*()` call sites across 10
/// files, all written for native text's TEXT-LOCAL-ADVANCE convention (`width`
/// is the run's advance along its own baseline), which OCR's axis-aligned boxes
/// do not satisfy -- it regressed `ocr_test_rotated_90` (word count 15 -> 13,
/// glued "conversion toJSON") and scrambled `ocr_test_rotated_270`'s reading
/// order entirely. Keeping the correction local to this one predicate, computed
/// fresh from the raw fields on every call, avoids all of that blast radius.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DetachedMarkerFrame {
    /// Native PDF text: defer to the segment's own `rotation_degrees`.
    Native,
    /// OCR route on a page whose PDF `/Rotate` is `degrees` (0/90/180/270).
    ///
    /// Only ever constructed from the OCR adapters, so on a feature set without
    /// OCR this variant is genuinely dead -- the `Native` arm still carries the
    /// whole native structure-tree path. `-D warnings` on the narrow `pdf` leg
    /// turns that into a hard error, so silence it exactly there rather than
    /// splitting the enum. ~keep
    #[cfg_attr(not(any(feature = "ocr", feature = "ocr-pipeline")), allow(dead_code))]
    OcrOnPage(u32),
}

impl DetachedMarkerFrame {
    /// The baseline coordinate to compare, in this frame.
    ///
    /// The OCR arms are measured against fixture `ordinance_2197` (`/Rotate
    /// 270`), tesseract backend, except `90`, which mirrors `270` by symmetry
    /// and has no fixture measurement backing it -- see this type's doc
    /// comment. `180` is confirmed a no-op: it falls through to the same
    /// `baseline_y` read as the unrotated default.
    fn baseline(self, segment: &SegmentData) -> f32 {
        match self {
            Self::Native => segment.upright_baseline(),
            // Measured: the FAR raster-x edge (`x + width`), not the near edge
            // (`x`) -- the far edge discriminates correct marker/body pairs
            // from wrong ones (delta 0-1 vs 215 against an 18-wide tolerance);
            // the near edge cannot (75-76 vs 139).
            Self::OcrOnPage(270) => segment.x + segment.width,
            // UNVERIFIED: derived by mirroring the 270 case (the near edge
            // instead of the far edge, matching the opposite rotation
            // handedness), not measured against any fixture.
            Self::OcrOnPage(90) => segment.x,
            Self::OcrOnPage(_) => segment.baseline_y,
        }
    }

    /// `(start, end)` along the reading direction, in this frame.
    fn advance_extent(self, segment: &SegmentData) -> (f32, f32) {
        match self {
            Self::Native => segment.upright_advance_extent(),
            // Measured: the advance axis runs along -y on a 270-rotated page,
            // and the reading-order START is the FAR raster-y edge
            // (`y + height`), not the near edge -- omitting the raster-y
            // extent mirrors the span ([-y, -y+height] instead of
            // [-(y+height), -y]) and inverts the indent test.
            Self::OcrOnPage(270) => (-(segment.y + segment.height), -segment.y),
            // UNVERIFIED: derived by mirroring the 270 case (the advance axis
            // runs along +y instead of -y, so the near/far roles swap and no
            // negation is needed), not measured against any fixture.
            Self::OcrOnPage(90) => (segment.y, segment.y + segment.height),
            Self::OcrOnPage(_) => (segment.x, segment.x + segment.width),
        }
    }
}

/// Reattach a list marker that was emitted as a paragraph of its own to the
/// body line it belongs to.
///
/// A hanging-indent list puts its markers in a narrow left column and its item
/// text in a wide right column. Both OCR block segmentation and some native
/// producers treat those columns as separate blocks, so the markers arrive as
/// isolated single-segment paragraphs — sometimes the whole marker column
/// ahead of the whole text column — and every item loses the only evidence that
/// it is an item. `finalize_paragraph`'s `starts_with_split_list_marker` already
/// handles the case where the marker and the body ended up in the *same*
/// paragraph; this handles the case where they did not.
///
/// Pairing is by *baseline*, not by adjacency, which is what makes a marker
/// column recoverable: "(a)", "(b)", "(c)" each match the body line they share a
/// baseline with regardless of how many paragraphs sit between them.
///
/// Deliberately narrow, because this pass is shared with native extraction:
/// - The marker paragraph must be exactly one line holding exactly one segment
///   whose whole text is a bare marker ([`is_bare_detached_list_marker`]) —
///   prose can never produce that, so flowing text has nothing here to match.
///   Narrower than the general [`is_bare_list_marker`]: see
///   [`EXCLUDE_AMBIGUOUS_DETACHED_MARKERS`].
/// - The body must not already be a heading or a list item, and its first line
///   must not already start with a marker.
/// - The body must be strictly to the right of the marker, within one hanging
///   indent, on the same baseline, in the same rotation frame.
/// - The body must carry at least [`DETACHED_MARKER_MIN_BODY_WORDS`] words.
///
/// Everything is expressed relative to font size, so the OCR route (whose
/// geometry may be in a different unit space) and the native route (points) get
/// the same behaviour.
pub(super) fn reattach_detached_list_markers(paragraphs: &mut Vec<PdfParagraph>, frame: DetachedMarkerFrame) {
    if !REATTACH_DETACHED_LIST_MARKERS || paragraphs.len() < 2 {
        return;
    }

    let mut consumed = vec![false; paragraphs.len()];
    let mut pairs: Vec<(usize, usize)> = Vec::new();

    for marker_index in 0..paragraphs.len() {
        if consumed[marker_index] {
            continue;
        }
        let Some(marker) = detached_list_marker(&paragraphs[marker_index]) else {
            continue;
        };
        let limit = (marker_index + 1 + DETACHED_MARKER_MAX_LOOKAHEAD).min(paragraphs.len());
        let body_index = (marker_index + 1..limit).find(|&candidate| {
            !consumed[candidate] && accepts_detached_list_marker(&paragraphs[candidate], &marker, frame)
        });
        let Some(body_index) = body_index else {
            continue;
        };
        consumed[marker_index] = true;
        consumed[body_index] = true;
        pairs.push((marker_index, body_index));
    }

    if pairs.is_empty() {
        return;
    }

    for (marker_index, body_index) in &pairs {
        let Some(marker_segment) = paragraphs[*marker_index]
            .lines
            .first()
            .and_then(|line| line.segments.first())
            .cloned()
        else {
            continue;
        };
        let marker_bbox = paragraphs[*marker_index].block_bbox;
        let body = &mut paragraphs[*body_index];
        if let Some(line) = body.lines.first_mut() {
            line.segments.insert(0, marker_segment);
        }
        body.is_list_item = true;
        body.block_bbox = match (body.block_bbox, marker_bbox) {
            (Some(body_bbox), Some(marker_bbox)) => Some((
                body_bbox.0.min(marker_bbox.0),
                body_bbox.1.min(marker_bbox.1),
                body_bbox.2.max(marker_bbox.2),
                body_bbox.3.max(marker_bbox.3),
            )),
            (bbox @ Some(_), None) | (None, bbox @ Some(_)) => bbox,
            (None, None) => None,
        };
        // Text is rebuilt from `lines` downstream (see
        // `synchronize_paragraph_text_metadata`); a stale cached string here
        // would silently win over the segment we just spliced in.
        body.text.clear();
        body.word_count = PdfParagraph::compute_word_count("", &body.lines);
    }

    let mut index = 0usize;
    paragraphs.retain(|_| {
        let keep = !pairs.iter().any(|(marker_index, _)| *marker_index == index);
        index += 1;
        keep
    });
}

/// The lone segment of a paragraph that is nothing but a list marker.
fn detached_list_marker(paragraph: &PdfParagraph) -> Option<SegmentData> {
    if paragraph.heading_level.is_some() || paragraph.is_list_item || paragraph.is_code_block || paragraph.is_formula {
        return None;
    }
    let [line] = paragraph.lines.as_slice() else {
        return None;
    };
    let [segment] = line.segments.as_slice() else {
        return None;
    };
    if !is_bare_detached_list_marker(&segment.text) {
        return None;
    }
    let geometry_is_usable = segment.x.is_finite()
        && segment.width.is_finite()
        && segment.width >= 0.0
        && segment.font_size.is_finite()
        && segment.font_size > 0.0
        && segment.upright_baseline().is_finite();
    geometry_is_usable.then(|| segment.clone())
}

/// Whether `paragraph` is the body line the detached `marker` belongs to.
///
/// Also reused by the OCR layout route's own reattachment pass
/// (`adapters::reattach_ocr_layout_list_markers`) -- this body-side test has no
/// dependency on how the marker paragraph itself was classified, only on the
/// candidate body's own shape, so it applies identically to both routes once
/// given the right [`DetachedMarkerFrame`] (#760): the OCR route passes
/// `OcrOnPage`, native passes `Native`. See that function's doc comment for why
/// the marker-side test (`detached_list_marker`, below) is NOT similarly
/// shared.
pub(super) fn accepts_detached_list_marker(
    paragraph: &PdfParagraph,
    marker: &SegmentData,
    frame: DetachedMarkerFrame,
) -> bool {
    if paragraph.heading_level.is_some()
        || paragraph.is_list_item
        || paragraph.is_code_block
        || paragraph.is_formula
        || paragraph.is_page_furniture
    {
        return false;
    }
    let Some(first_line) = paragraph.lines.first() else {
        return false;
    };
    if first_line.segments.is_empty() {
        return false;
    }
    let first_line_text = first_line
        .segments
        .iter()
        .map(|segment| segment.text.trim())
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if looks_like_list_item(&first_line_text) || is_bare_list_marker(&first_line_text) {
        return false;
    }

    let body_words = paragraph
        .lines
        .iter()
        .flat_map(|line| line.segments.iter())
        .flat_map(|segment| segment.text.split_whitespace())
        .count();
    if body_words < DETACHED_MARKER_MIN_BODY_WORDS {
        return false;
    }

    let Some(anchor) = first_line.segments.first() else {
        return false;
    };
    if !anchor.has_same_rotation(marker) {
        return false;
    }
    let font_size = anchor.font_size.max(marker.font_size);
    if !font_size.is_finite() || font_size <= 0.0 {
        return false;
    }

    let baseline_delta = (frame.baseline(anchor) - frame.baseline(marker)).abs();
    if !baseline_delta.is_finite() || baseline_delta > font_size * DETACHED_MARKER_BASELINE_TOLERANCE_FONT_FACTOR {
        return false;
    }

    let body_left = first_line
        .segments
        .iter()
        .map(|segment| frame.advance_extent(segment).0)
        .fold(f32::INFINITY, f32::min);
    let (marker_start, marker_end) = frame.advance_extent(marker);
    if !body_left.is_finite() || !marker_start.is_finite() || !marker_end.is_finite() {
        return false;
    }
    let indent = body_left - marker_end;
    body_left > marker_start
        && indent >= -(font_size * DETACHED_MARKER_MAX_OVERLAP_FONT_FACTOR)
        && indent <= font_size * DETACHED_MARKER_MAX_INDENT_FONT_FACTOR
}

/// Repair reading order inside maximal rotated runs without touching upright
/// stream order or moving content across a rotation boundary.
fn order_segments_in_reading_frames(segments: Vec<SegmentData>) -> Vec<SegmentData> {
    if segments.iter().all(SegmentData::is_unrotated) {
        return segments;
    }

    let mut groups: Vec<Vec<SegmentData>> = Vec::new();
    for segment in segments {
        match groups.last_mut() {
            Some(group) if group[0].has_same_rotation(&segment) => group.push(segment),
            _ => groups.push(vec![segment]),
        }
    }

    groups
        .into_iter()
        .flat_map(|group| {
            if group[0].is_unrotated() {
                group
            } else {
                order_rotated_segment_group(group)
            }
        })
        .collect()
}

fn order_rotated_segment_group(mut segments: Vec<SegmentData>) -> Vec<SegmentData> {
    segments.sort_by(|first, second| {
        second
            .upright_baseline()
            .total_cmp(&first.upright_baseline())
            .then_with(|| {
                first
                    .upright_advance_extent()
                    .0
                    .total_cmp(&second.upright_advance_extent().0)
            })
    });

    let mut visual_lines: Vec<Vec<SegmentData>> = Vec::new();
    for segment in segments {
        let belongs_to_last_line = visual_lines.last().is_some_and(|line| {
            let anchor = &line[0];
            let tolerance = anchor.height.max(segment.height).max(anchor.font_size * 0.5) * 0.5;
            (anchor.upright_baseline() - segment.upright_baseline()).abs() <= tolerance
        });
        if belongs_to_last_line {
            if let Some(line) = visual_lines.last_mut() {
                line.push(segment);
            }
        } else {
            visual_lines.push(vec![segment]);
        }
    }

    for line in &mut visual_lines {
        line.sort_by(|first, second| {
            first
                .upright_advance_extent()
                .0
                .total_cmp(&second.upright_advance_extent().0)
        });
    }
    visual_lines.into_iter().flatten().collect()
}

#[cfg(feature = "layout-detection")]
fn wrapper_ownership_by_hint(
    hints: &[LayoutHint],
    validations: &[super::regions::layout_validation::RegionValidation],
) -> Vec<bool> {
    hints
        .iter()
        .enumerate()
        .map(|(index, hint)| {
            !is_wrapper_layout_hint(hint)
                || !matches!(
                    validations.get(index),
                    Some(super::regions::layout_validation::RegionValidation::Empty)
                )
        })
        .collect()
}

#[cfg(feature = "layout-detection")]
struct LayoutParagraphContext<'a> {
    heading_map: &'a [(f32, Option<u8>)],
    paragraph_gap_ys: &'a [f32],
    doc_body_font_size: Option<f32>,
    include_headers: bool,
    include_footers: bool,
    include_footnotes: bool,
    page_width_pts: Option<f32>,
    apply_layout_overrides: bool,
    witnesses: &'a TextRepairWitnesses,
}

#[cfg(feature = "layout-detection")]
struct NativeLayoutProjection {
    groups: Vec<crate::extractors::pdf::reading_order::LayoutSegmentGroup>,
    group_bounds: Vec<Option<(f32, f32, f32, f32)>>,
    classification_hints: Vec<LayoutHint>,
}

#[cfg(feature = "layout-detection")]
fn process_layout_segment_groups(
    segments: Vec<SegmentData>,
    hints: &[LayoutHint],
    wrapper_ownership: &[bool],
    context: LayoutParagraphContext<'_>,
) -> Vec<PdfParagraph> {
    let no_reorder = super::layout_debug::layout_debug_flags().no_reorder;
    let groups = crate::extractors::pdf::reading_order::plan_segment_groups_by_layout(
        &segments,
        hints,
        wrapper_ownership,
        no_reorder,
        context.page_width_pts,
    );
    if matches!(groups.as_slice(), [group] if group.hint_indices.is_empty() && group.region_path.is_none()) {
        return segments_to_paragraphs(
            segments,
            context.heading_map,
            context.paragraph_gap_ys,
            context.witnesses,
        );
    }
    if !context.apply_layout_overrides {
        let group_bounds = layout_group_bounds(&groups, &segments);
        let mut paragraphs = segments_to_paragraphs(
            segments,
            context.heading_map,
            context.paragraph_gap_ys,
            context.witnesses,
        );
        assign_native_paragraph_layout(&mut paragraphs, &groups, &group_bounds);
        let classification_hints = regular_layout_hints(hints);
        super::layout_classify::annotate_layout_classes(&mut paragraphs, &classification_hints, 0.5, 0.2);
        return paragraphs;
    }
    let mut slots = segments.into_iter().map(Some).collect::<Vec<_>>();
    let mut paragraphs = Vec::new();

    for group in groups {
        let region_path = group.region_path;
        let group_segments = group
            .segment_indices
            .into_iter()
            .filter_map(|index| slots.get_mut(index).and_then(Option::take))
            .collect::<Vec<_>>();
        if group_segments.is_empty() {
            continue;
        }
        let gap_ys = compute_paragraph_gap_ys(&group_segments);
        let mut group_paragraphs =
            segments_to_paragraphs(group_segments, context.heading_map, &gap_ys, context.witnesses);
        let group_hints = group
            .hint_indices
            .into_iter()
            .filter_map(|index| hints.get(index).cloned())
            .collect::<Vec<_>>();
        if context.apply_layout_overrides {
            super::layout_classify::apply_layout_overrides(
                &mut group_paragraphs,
                &group_hints,
                0.5,
                0.2,
                context.doc_body_font_size,
            );
            un_mark_layout_furniture_per_config(
                &mut group_paragraphs,
                context.include_headers,
                context.include_footers,
                context.include_footnotes,
            );
        } else {
            super::layout_classify::annotate_layout_classes(&mut group_paragraphs, &group_hints, 0.5, 0.2);
        }
        for paragraph in &mut group_paragraphs {
            paragraph.layout_region_path = region_path;
        }
        paragraphs.extend(group_paragraphs);
    }

    let leftovers = slots.into_iter().flatten().collect::<Vec<_>>();
    if !leftovers.is_empty() {
        tracing::warn!(
            segments = leftovers.len(),
            "layout region plan omitted segments; appending an unsorted fallback group"
        );
        let gap_ys = compute_paragraph_gap_ys(&leftovers);
        paragraphs.extend(segments_to_paragraphs(
            leftovers,
            context.heading_map,
            &gap_ys,
            context.witnesses,
        ));
    }
    paragraphs
}

#[cfg(feature = "layout-detection")]
fn assign_native_paragraph_layout(
    paragraphs: &mut Vec<PdfParagraph>,
    groups: &[crate::extractors::pdf::reading_order::LayoutSegmentGroup],
    group_bounds: &[Option<(f32, f32, f32, f32)>],
) {
    for paragraph in paragraphs {
        let group_rank = paragraph_geometry_bbox(paragraph).and_then(|paragraph_bbox| {
            groups
                .iter()
                .enumerate()
                .filter_map(|(group_index, _)| {
                    let overlap = group_bounds
                        .get(group_index)
                        .and_then(|bounds| *bounds)
                        .map_or(0.0, |bounds| rectangle_overlap_area(paragraph_bbox, bounds));
                    (overlap > 0.0).then_some((group_index, overlap))
                })
                .max_by(|left, right| left.1.total_cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
                .map(|(group_index, _)| group_index)
        });
        let Some(group_index) = group_rank else {
            continue;
        };
        let Some(mut path) = groups[group_index].region_path else {
            continue;
        };
        let original_root_id = path.root.id;
        let root_order = groups
            .iter()
            .position(|group| {
                group
                    .region_path
                    .is_some_and(|candidate| candidate.root.id == original_root_id)
            })
            .unwrap_or(group_index);
        path.root.id = root_order;
        if let Some(child) = &mut path.child {
            child.id = group_index;
        }
        paragraph.layout_region_path = Some(path);
    }
}

fn reorder_pages_by_layout_region(pages: &mut [Vec<PdfParagraph>]) {
    for page in pages {
        page.sort_by_key(|paragraph| {
            paragraph
                .layout_region_path
                .map(|path| path.child.map_or(path.root.id, |child| child.id))
                .unwrap_or(usize::MAX)
        });
    }
}

fn paragraph_geometry_bbox(paragraph: &PdfParagraph) -> Option<(f32, f32, f32, f32)> {
    if let Some(block_bbox) = paragraph.block_bbox {
        return Some(block_bbox);
    }
    let mut segments = paragraph.lines.iter().flat_map(|line| line.segments.iter());
    let first = segments.next()?;
    let mut bounds = (
        first.x,
        first.y.min(first.baseline_y),
        first.x + first.width,
        (first.y + first.height).max(first.baseline_y + first.height),
    );
    for segment in segments {
        bounds.0 = bounds.0.min(segment.x);
        bounds.1 = bounds.1.min(segment.y.min(segment.baseline_y));
        bounds.2 = bounds.2.max(segment.x + segment.width);
        bounds.3 = bounds
            .3
            .max((segment.y + segment.height).max(segment.baseline_y + segment.height));
    }
    Some(bounds)
}

#[cfg(feature = "layout-detection")]
fn layout_group_bounds(
    groups: &[crate::extractors::pdf::reading_order::LayoutSegmentGroup],
    segments: &[SegmentData],
) -> Vec<Option<(f32, f32, f32, f32)>> {
    groups
        .iter()
        .map(|group| {
            let mut group_segments = group.segment_indices.iter().filter_map(|index| segments.get(*index));
            let first = group_segments.next()?;
            let mut bounds = (
                first.x,
                first.y.min(first.baseline_y),
                first.x + first.width,
                (first.y + first.height).max(first.baseline_y + first.height),
            );
            for segment in group_segments {
                bounds.0 = bounds.0.min(segment.x);
                bounds.1 = bounds.1.min(segment.y.min(segment.baseline_y));
                bounds.2 = bounds.2.max(segment.x + segment.width);
                bounds.3 = bounds
                    .3
                    .max((segment.y + segment.height).max(segment.baseline_y + segment.height));
            }
            Some(bounds)
        })
        .collect()
}

#[cfg(feature = "layout-detection")]
fn rectangle_overlap_area(left: (f32, f32, f32, f32), right: (f32, f32, f32, f32)) -> f32 {
    let width = left.2.min(right.2) - left.0.max(right.0);
    let height = left.3.min(right.3) - left.1.max(right.1);
    width.max(0.0) * height.max(0.0)
}

/// Multiple of the median line height a whitespace band must exceed to count
/// as a paragraph break. Normal line pitch leaves well under one line height of
/// whitespace; a blank line leaves more than one.
const PARAGRAPH_GAP_HEIGHT_FACTOR: f32 = 1.5;

/// Multiple of the page's own body leading a baseline-to-baseline advance must
/// reach to count as a paragraph break.
///
/// [`PARAGRAPH_GAP_HEIGHT_FACTOR`] measures the *whitespace band* between two
/// lines against the glyph height, which makes it blind to the most common
/// paragraph separator there is. With glyph height `h` and leading `L`, single
/// spacing leaves a band of `L - h` and a blank line leaves `2L - h`; for the
/// usual `L` of 1.1–1.3 `h` that blank line is only 1.2–1.6 `h`, so a 1.5 `h`
/// band threshold demands more vertical space than a blank line actually
/// provides. Comparing the advance to the leading instead is scale-free: a
/// blank line doubles the advance, so anything at or past 1.5× the body leading
/// is a break while ordinary wrapped lines (1.0×) are not, whatever the leading
/// happens to be. The two rules are OR-ed, so no break the band rule already
/// finds is lost.
const PARAGRAPH_BREAK_LEADING_MULTIPLE: f32 = 1.5;
const INLINE_STYLE_BASELINE_TOLERANCE: f32 = 0.5;
/// How many already-accumulated segments back to look for a sub/superscript's base.
///
/// ~keep GH#1617: two is enough on the reproducer (`dB` then `L`); four covers a row with a couple
/// more cells to the right of the base without letting the search wander off the current row.
/// GH#1628 reuses this window for the word-level table path, which searches the same segment list.
pub(crate) const SCRIPT_RUN_BASE_LOOKBACK: usize = 4;
const INLINE_STYLE_MAX_FORWARD_GAP_FONT_FACTOR: f32 = 1.0;
const INLINE_FONT_SIZE_MAX_FORWARD_GAP_FONT_FACTOR: f32 = 1.5;
const INLINE_STYLE_MAX_OVERLAP_FONT_FACTOR: f32 = 0.15;

/// Multiple of font size within which two consecutive lines' right edges must
/// agree for the second to read as the wrapped tail of a heading, rather than
/// unrelated content that merely follows it.
///
/// A heading that wraps mid-sentence fills its first physical line out to the
/// text column's right margin before continuing below, so the wrapped line and
/// its continuation land their right edges close together; a heading followed
/// by unrelated content (a callout, a new paragraph) has no reason to share
/// that edge and typically differs by far more. Two font-size widths is
/// generous enough to absorb ordinary word-wrap slack -- the space left unused
/// because the next word did not fit -- without also treating a long heading
/// followed by a much shorter, unrelated line as a wrap. See #1467.
const HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR: f32 = 2.0;
/// How closely a wrapped heading's continuation must resume at the same left edge
/// as the line it continues, in font-sizes. Measured on GH#1615's reproducer the
/// two align exactly (both x 83.64) while the body line that must NOT merge sits
/// 35.4pt away at the margin, so the separation is wide and the tolerance only has
/// to absorb sub-pixel drift. ~keep
const HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR: f32 = 0.5;
/// Points within which two lines count as set at the same size for the
/// heading-continuation tests, which is the same threshold `blocks_to_paragraphs`
/// uses for its own `font_change` break term.
const HEADING_CONTINUATION_FONT_SIZE_TOLERANCE_PT: f32 = 1.5;
/// Fraction of the font size charged for the word space in the "did the next word
/// fit?" sum of [`heading_line_ran_out_of_room`].
///
/// A word space is 0.25-0.33 em in the text faces this rule sees (Helvetica's is
/// 0.278 em). The low end of that range is deliberate: a wider space makes the sum
/// overflow the column edge more readily and so accepts MORE continuations, and a
/// rule that welds a heading to the body is the regression this whole family of
/// tests exists to prevent. See GH#1609. ~keep
const HEADING_WRAP_INTERWORD_SPACE_FONT_FACTOR: f32 = 0.25;
/// How far a further line's baseline advance may differ from the heading's own
/// pitch, as a fraction of that pitch, and still read as the same heading run in
/// [`next_visual_line_right_edge`].
///
/// A heading run is set solid at one pitch, and the seam to the body beneath is
/// wider (21pt against a 10.45pt pitch on GH#1758's reproducer). The band is
/// two-sided on purpose: an upper bound alone would let the body's own tighter
/// pitch pass whenever the heading-to-body seam was used as the reference. ~keep
const HEADING_CONTINUATION_RUN_PITCH_TOLERANCE_FACTOR: f32 = 0.25;

/// Detect paragraph-break y-positions from horizontal whitespace bands.
///
/// Segments are clustered into visual lines after sorting by y — stream order
/// is not positional (multi-column PDFs interleave columns, which a pairwise
/// scan misreads as phantom gaps). A break is recorded where the band between
/// two consecutive lines is taller than [`PARAGRAPH_GAP_HEIGHT_FACTOR`] × the
/// median line height, or where their baseline advance reaches
/// [`PARAGRAPH_BREAK_LEADING_MULTIPLE`] × the page's own body leading — the
/// signal a blank line actually produces. Bands between two monospace lines are
/// skipped, because code listings legitimately contain blank lines inside one
/// logical block.
///
/// Without this, the heuristic path only breaks paragraphs on font/bold/list
/// changes, fusing visually separated blocks (standalone headings, display
/// formulas) into surrounding prose.
fn compute_paragraph_gap_ys(segments: &[SegmentData]) -> Vec<f32> {
    if segments.len() < 2 {
        return Vec::new();
    }

    if segments.iter().all(SegmentData::is_unrotated) {
        return compute_paragraph_gap_ys_in_shared_frame(segments);
    }

    let mut gaps = Vec::new();
    let mut group_start = 0;
    for index in 1..=segments.len() {
        let ends_group = index == segments.len() || !segments[index - 1].has_same_rotation(&segments[index]);
        if ends_group {
            gaps.extend(compute_paragraph_gap_ys_in_shared_frame(&segments[group_start..index]));
            group_start = index;
        }
    }
    gaps
}

/// One visual line of a page, as clustered by [`compute_paragraph_gap_ys_in_shared_frame`].
struct LineBand {
    top: f32,
    bottom: f32,
    height: f32,
    monospace: bool,
    anchor_y: f32,
}

fn compute_paragraph_gap_ys_in_shared_frame(segments: &[SegmentData]) -> Vec<f32> {
    if segments.len() < 2 {
        return Vec::new();
    }

    let mut order: Vec<usize> = (0..segments.len()).collect();
    order.sort_by(|&a, &b| {
        paragraph_gap_axis(&segments[b])
            .partial_cmp(&paragraph_gap_axis(&segments[a]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut lines: Vec<LineBand> = Vec::new();
    for &i in &order {
        let seg = &segments[i];
        let (bottom, top) = if seg.is_unrotated() {
            (seg.y, seg.y + seg.height)
        } else {
            seg.upright_cross_extent()
        };
        let baseline = paragraph_gap_axis(seg);
        let tolerance = (seg.height * 0.5).max(1.0);
        match lines.last_mut() {
            Some(line) if (baseline - line.anchor_y).abs() <= tolerance => {
                line.top = line.top.max(top);
                line.bottom = line.bottom.min(bottom);
                line.height = line.height.max(seg.height);
                line.monospace &= seg.is_monospace;
            }
            _ => lines.push(LineBand {
                top,
                bottom,
                height: seg.height,
                monospace: seg.is_monospace,
                anchor_y: baseline,
            }),
        }
    }
    if lines.len() < 2 {
        return Vec::new();
    }

    let mut heights: Vec<f32> = lines.iter().map(|l| l.height).collect();
    heights.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_height = heights[heights.len() / 2];
    let gap_threshold = median_height * PARAGRAPH_GAP_HEIGHT_FACTOR;
    let advance_threshold = body_leading(&lines, median_height) * PARAGRAPH_BREAK_LEADING_MULTIPLE;

    let mut gap_ys = Vec::new();
    for pair in lines.windows(2) {
        let gap = pair[0].bottom - pair[1].top;
        let advance = pair[0].anchor_y - pair[1].anchor_y;
        if (gap > gap_threshold || advance > advance_threshold) && !(pair[0].monospace && pair[1].monospace) {
            gap_ys.push((pair[0].bottom + pair[1].top) / 2.0);
        }
    }
    gap_ys
}

/// Estimate the body leading of a page from its own line pitch: the tightest
/// baseline-to-baseline advance between consecutive lines, floored at the
/// median line height.
///
/// The tightest advance is used rather than the median or the mode because a
/// short block-structured page — a memo of five one-line blocks, say — has more
/// break-sized advances than body-sized ones, so any central statistic reports
/// the break spacing as normal and no break can ever be detected. The floor is
/// what makes the minimum safe: an advance below one line height is not a
/// wrapped line at all (stacked accents, a subscript resolved onto its own
/// band) and must not be allowed to shrink the estimate and split every
/// ordinary line on the page.
fn body_leading(lines: &[LineBand], median_height: f32) -> f32 {
    let tightest = lines
        .windows(2)
        .map(|pair| pair[0].anchor_y - pair[1].anchor_y)
        .filter(|advance| advance.is_finite() && *advance > 0.0)
        .fold(f32::INFINITY, f32::min);
    if tightest.is_finite() {
        tightest.max(median_height)
    } else {
        median_height
    }
}

fn paragraph_gap_axis(segment: &SegmentData) -> f32 {
    if segment.is_unrotated() {
        segment.y
    } else {
        segment.upright_baseline()
    }
}

/// Convert a flat list of text segments into grouped paragraphs.
///
/// Groups consecutive segments by font changes, bold changes, list markers, and
/// paragraph gap positions. Each group is then classified via `finalize_paragraph`.
/// The text of the whole visual line each segment belongs to, indexed alongside
/// `lines`.
///
/// The numbered-heading break terms test a predicate against a line's opening
/// token, but this loop walks SEGMENTS, and a heading set with a hanging number
/// arrives as two of them on one baseline -- `"3.1.7"` and
/// `"Innovatie/ontwikkelingen"`. Neither segment alone starts with a section
/// number the way the assembled line does, so the terms never fired and the
/// heading was left to the ordinary paragraph-gap rule, which needs a gap wider
/// than ordinary line pitch. `merge_continuation_paragraphs::starts_numbered_section`
/// already re-joins a paragraph's first line for exactly this reason; this is the
/// same re-join on the grouper side, so the two passes agree. See #1609. ~keep
fn visual_line_texts(lines: &[SegmentData]) -> Vec<String> {
    let mut texts = vec![String::new(); lines.len()];
    let mut start = 0usize;
    while start < lines.len() {
        let mut end = start + 1;
        // Same-visual-line test as `starts_new_line` below: consecutive, so the two
        // cannot disagree about where a line ends. ~keep
        while end < lines.len()
            && lines[end].has_same_rotation(&lines[end - 1])
            && (lines[end].upright_baseline() - lines[end - 1].upright_baseline()).abs()
                <= INLINE_STYLE_BASELINE_TOLERANCE
        {
            end += 1;
        }
        let joined = lines[start..end]
            .iter()
            .map(|segment| segment.text.trim())
            .filter(|text| !text.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        for slot in &mut texts[start..end] {
            slot.clone_from(&joined);
        }
        start = end;
    }
    texts
}

// GH#1806: values mirror the column-gutter bounds already measured for the equivalent
// span-level question in `pdf::native::text` (`MIN_DENSE_COLUMN_GUTTER_FRACTION`/`_PTS`,
// `MAX_DENSE_COLUMN_GUTTER_FRACTION`, `FULL_WIDTH_FURNITURE_FRACTION`), restated here
// against this consumer's own content-width estimate rather than re-derived from
// scratch. ~keep
const PARAGRAPH_COLUMN_GUTTER_MIN_FRACTION: f32 = 0.02;
const PARAGRAPH_COLUMN_GUTTER_MIN_PTS: f32 = 10.0;
const PARAGRAPH_COLUMN_GUTTER_MAX_FRACTION: f32 = 0.25;
const PARAGRAPH_FULL_WIDTH_FURNITURE_FRACTION: f32 = 0.55;

/// The page's single column corridor, from `lines`' own geometry: an x-interval (in each
/// segment's own upright reading frame) that no segment's ink occupies. `None` when the
/// page presents no such corridor (a single column) or several (a grid/form/table) --
/// several qualifying corridors is the same "not a two-column page" signal
/// `document::columns::prose_two_column_gutter` already applies at the span level.
///
/// GH#1806: computed here, from `SegmentData`, rather than threaded down from
/// `native::hierarchy`'s own span-level gutter detection. This grouper is the only
/// consumer of the result, and threading it through would add a per-page channel through
/// `extract_all_segments`, `SegmentStructureConfig` and `PageInput` for one bool per
/// paragraph boundary. This is a MERGE-SUPPRESSION consumer only -- it can suppress a
/// break vote's absence, never force one -- so a corridor found here that later turns out
/// not to be a real column gutter can only leave two segments un-merged, never wrongly
/// merge or reorder anything. ~keep
fn page_column_corridor(lines: &[SegmentData]) -> Option<(f32, f32)> {
    let mut extents: Vec<(f32, f32)> = lines
        .iter()
        .filter(|line| !line.text.trim().is_empty())
        .map(SegmentData::upright_advance_extent)
        .filter(|&(left, right)| left.is_finite() && right.is_finite() && right > left)
        .collect();
    if extents.len() < 2 {
        return None;
    }
    let content_min = extents.iter().map(|&(left, _)| left).fold(f32::INFINITY, f32::min);
    let content_max = extents
        .iter()
        .map(|&(_, right)| right)
        .fold(f32::NEG_INFINITY, f32::max);
    let content_width = content_max - content_min;
    if !(content_width.is_finite() && content_width > 0.0) {
        return None;
    }

    // A full-width line (a title, a running header, a footer) straddles any real column
    // corridor by construction and must not be read as occupying "one side" of it. ~keep
    let furniture_width = content_width * PARAGRAPH_FULL_WIDTH_FURNITURE_FRACTION;
    extents.retain(|&(left, right)| right - left < furniture_width);
    if extents.len() < 2 {
        return None;
    }
    let min_gutter = (content_width * PARAGRAPH_COLUMN_GUTTER_MIN_FRACTION).max(PARAGRAPH_COLUMN_GUTTER_MIN_PTS);
    let max_gutter = content_width * PARAGRAPH_COLUMN_GUTTER_MAX_FRACTION;
    extents.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut corridors: Vec<(f32, f32)> = Vec::new();
    let mut running_right = extents[0].1;
    for &(left, right) in &extents[1..] {
        if left - running_right >= min_gutter {
            corridors.push((running_right, left));
        }
        running_right = running_right.max(right);
    }
    let mut qualifying = corridors
        .into_iter()
        .filter(|&(left, right)| right - left <= max_gutter);
    let first = qualifying.next()?;
    qualifying.next().is_none().then_some(first)
}

fn blocks_to_paragraphs(
    lines: Vec<SegmentData>,
    heading_map: &[(f32, Option<u8>)],
    paragraph_gap_ys: &[f32],
) -> Vec<PdfParagraph> {
    if lines.is_empty() {
        return Vec::new();
    }

    let gap_info = super::classify::precompute_gap_info(heading_map);
    let visual_line_texts = visual_line_texts(&lines);
    let column_corridor = page_column_corridor(&lines);

    let mut paragraphs: Vec<PdfParagraph> = Vec::new();
    let mut current_lines: Vec<&SegmentData> = Vec::new();
    let mut current_is_single_visual_line = true;
    let mut prev_idx = 0usize;

    for (line_idx, line) in lines.iter().enumerate() {
        let should_break = if current_lines.is_empty() {
            false
        } else {
            let prev = current_lines.last().unwrap();
            // The look-back exists because a subscript is not always adjacent to its base in segment
            // order. Its baseline is below the row's, and the upstream row-band sort keys on top-y,
            // so a cell further right on the row can be emitted between the two: on GH#1617's
            // reproducer `WA` (x 211.08) arrives after `dB` (x 253.30) and is compared against it
            // rather than against `L` (x 206.78), which it actually abuts. Reuniting them in the
            // ORDER the page prints would need per-glyph positions; suppressing the break is what
            // keeps the subscript from becoming an element of its own, which is the defect. ~keep
            let font_change = (line.font_size - prev.font_size).abs() > 1.5
                && !is_inline_style_transition(
                    current_is_single_visual_line,
                    prev,
                    line,
                    INLINE_FONT_SIZE_MAX_FORWARD_GAP_FONT_FACTOR,
                )
                && !current_lines
                    .iter()
                    .rev()
                    .take(SCRIPT_RUN_BASE_LOOKBACK)
                    .any(|candidate| is_script_run_of(candidate, line));
            let role_change = line.assigned_role != prev.assigned_role;
            let bold_change = line.is_bold != prev.is_bold
                && !is_inline_style_transition(
                    current_is_single_visual_line,
                    prev,
                    line,
                    INLINE_STYLE_MAX_FORWARD_GAP_FONT_FACTOR,
                );
            let rotation_change = !line.has_same_rotation(prev);
            let starts_new_line = rotation_change
                || (line.upright_baseline() - prev.upright_baseline()).abs() > INLINE_STYLE_BASELINE_TOLERANCE;
            let has_same_line_follower = lines.get(line_idx + 1).is_some_and(|next| {
                next.has_same_rotation(line)
                    && (next.upright_baseline() - line.upright_baseline()).abs() <= INLINE_STYLE_BASELINE_TOLERANCE
            });
            let is_list = starts_new_line
                && (looks_like_list_item(&line.text) || (has_same_line_follower && is_bare_list_marker(&line.text)));
            // A numbered section heading always begins a new element. Without this
            // term a run of same-size, same-weight, evenly-spaced headings
            // ("1.3 Gasinstallatie", "1.4 Elektrische installatie", ...) yields no
            // break signal at all: `looks_like_list_item` deliberately returns
            // `false` for numbered section headings, so recognising the line as a
            // heading removes the only boundary this grouper would otherwise see,
            // and the whole run collapses into one paragraph. `is_numbered_section_heading`
            // (not the looser `starts_with_section_number`) is used deliberately so
            // prose beginning with a bare year — "2024 was een druk jaar" — does not
            // break its paragraph. See #1386. ~keep
            let starts_section =
                starts_new_line && super::classify::is_numbered_section_heading(&visual_line_texts[line_idx]);
            // A numbered section heading also always ENDS the element it opens: without
            // this term nothing else distinguishes a heading from the body text that
            // follows it when both share font size, weight, role and line spacing --
            // exactly the shape of a bold-only page in #1467, where the heading and the
            // callout beneath it are otherwise identical on every signal this grouper
            // checks. `starts_section` (above) only fires while classifying the
            // heading's OWN line and cannot see forward to close it once it opens; this
            // looks backward at `prev` instead. Restricting to `current_lines.len() ==
            // 1` scopes the break to the line directly after the heading, so a
            // paragraph that is already several lines long is untouched, and
            // `heading_wraps_onto` exempts a heading that is itself still wrapping onto
            // its next physical line rather than handing off to unrelated content. See
            // #1467. ~keep
            // GH#1634: `current_is_single_visual_line` also switches the closing
            // term off once a genuine heading wrap has been absorbed, so a
            // two-line heading was never closed and pulled the whole body in
            // after it. A numbered heading that spans exactly one wrap is still
            // a heading and must still close. ~keep
            // GH#1740: closing used to require `visual_line_count(&current_lines) ==
            // 2` -- a numbered heading was allowed to wrap once and never twice,
            // because a THIRD line made the element no longer "a single visual
            // line plus one wrap" and this term stopped applying, so
            // `follows_section` closed the heading one line early regardless of
            // whether the third line actually continued it. The count is dropped:
            // this stays true for as long as `current_lines` is nothing but the
            // heading's own first line and lines already accepted as its
            // continuation, however many there have been, so the SAME per-line
            // continuation test (`heading_continuation_accepted`, below) decides
            // every subsequent line too, and `follows_section` closes at the
            // first one that fails it. ~keep
            let heading_wrap_chain_open = !current_is_single_visual_line
                && current_lines
                    .first()
                    .is_some_and(|first| super::classify::is_numbered_section_heading(first.text.trim()));
            // For a wrapped heading `prev` is the continuation line, whose own
            // text carries no number -- so the numbered-heading test has to look
            // at the paragraph's first segment instead, which
            // `heading_wrap_chain_open` already does. ~keep
            // GH#1637: `heading_wraps_onto` is a RIGHT-edge test and must stay
            // scoped to the `current_is_single_visual_line` branch (#1467's
            // no-indent heading, where a right edge is the only signal available).
            // A hanging-indent heading that has absorbed a wrap already has a
            // LEFT-edge answer from `heading_continuation_is_hanging_indent` below,
            // and that answer is authoritative: a wrap's last line and a run-in
            // sub-heading beneath it are both short by definition, so their right
            // edges land within tolerance of each other by coincidence on real
            // documents, not because the run-in continues the heading. Applying
            // `heading_wraps_onto` beyond the first wrap let that coincidence
            // override the correct left-edge answer and kept the heading open
            // across the run-in and the body beneath it -- true at every wrap
            // depth, not just the second line, so `heading_wraps_onto` stays
            // scoped to `current_is_single_visual_line` after GH#1740 too. ~keep
            // GH#1650: `heading_wraps_onto` and `heading_continuation_is_hanging_indent`
            // are both blind to a no-indent heading whose title is set in the HEADING
            // font: there is no indent to measure, and the wrap's short last line never
            // matches the heading's own right edge. `heading_continuation_at_margin` is
            // the third exemption -- it measures the line BEFORE the one being tested
            // against the body column beneath the pair, not the continuation's own
            // width, so a short last line cannot satisfy it by accident (see its own
            // doc comment and GH#1650's page 4 control). GH#1740: that is also what
            // lets it generalise to a heading's third line, fourth, and so on --
            // `prev` is always the immediately preceding line, wrap or not, so the
            // same "does the line before `line` fill the column" test applies
            // unchanged at every step, and a heading's genuinely last line (always
            // short, by definition) fails it and stops the chain. ~keep
            let heading_continuation_accepted = current_lines.first().is_some_and(|heading_start| {
                (current_is_single_visual_line
                    && super::classify::is_numbered_section_heading(&visual_line_texts[prev_idx])
                    && (heading_wraps_onto(prev, line)
                        || heading_continuation_at_margin(heading_start, prev, line, &lines, line_idx)))
                    || (heading_wrap_chain_open
                        && heading_continuation_at_margin(heading_start, prev, line, &lines, line_idx))
                    || heading_continuation_is_hanging_indent(heading_start, prev, line)
            });
            let follows_section = starts_new_line
                && ((current_is_single_visual_line
                    && super::classify::is_numbered_section_heading(&visual_line_texts[prev_idx])
                    && !heading_continuation_accepted)
                    || heading_wrap_chain_open)
                && !heading_continuation_accepted;
            // GH#1650: a heading set larger than the body outruns the body's own
            // leading by construction -- `compute_paragraph_gap_ys_in_shared_frame`'s
            // `body_leading` is the page's tightest pitch, and a 12pt heading at
            // 16.8pt leading is always going to exceed 1.5x an 8pt body's 10.56pt
            // pitch. `crossed_gap` cannot see that this is the heading's OWN pitch
            // rather than a blank-line paragraph break, so a continuation the other
            // two terms already accepted must not be re-cut here. See #1467 and
            // #1615 for why a continuation `follows_section` rejects must still be
            // able to cross a gap -- this exemption is scoped to exactly the same
            // boundary `follows_section` accepts, not to headings in general. ~keep
            let crossed_gap = !heading_continuation_accepted
                && paragraph_gap_ys.iter().any(|&gap_y| {
                    let previous_baseline = prev.upright_baseline();
                    let current_baseline = line.upright_baseline();
                    let (upper, lower) = if previous_baseline > current_baseline {
                        (previous_baseline, current_baseline)
                    } else {
                        (current_baseline, previous_baseline)
                    };
                    gap_y < upper && gap_y > lower
                });
            // GH#1806: two lines on opposite sides of the page's column corridor, sharing
            // no horizontal extent at all, are not one paragraph -- whatever reading order
            // the page is in. The existing terms vote on rotation, font, role, weight, list
            // marker, section number and VERTICAL gap; none of them has a horizontal term,
            // so a left column's last line and the right column's first body line, whose
            // boxes happened to overlap by a couple of points in y, were merged into one
            // full-width element. A full-width line straddles the corridor by construction
            // (see `page_column_corridor`), so it can never satisfy "opposite sides" and is
            // unaffected; a heading's own two sides sharing a baseline are two entries in
            // `lines`, not one, so `starts_new_line` (required here) already excludes that
            // case, which `find_heading_runs` handles on its own path (GH#1809). ~keep
            let crosses_column_corridor = starts_new_line
                && column_corridor.is_some_and(|(corridor_left, corridor_right)| {
                    let (prev_left, prev_right) = prev.upright_advance_extent();
                    let (line_left, line_right) = line.upright_advance_extent();
                    (prev_right <= corridor_left && line_left >= corridor_right)
                        || (line_right <= corridor_left && prev_left >= corridor_right)
                });
            rotation_change
                || font_change
                || role_change
                || bold_change
                || is_list
                || starts_section
                || follows_section
                || crossed_gap
                || crosses_column_corridor
        };

        if should_break && !current_lines.is_empty() {
            if let Some(para) = finalize_paragraph(&current_lines, heading_map, &gap_info) {
                paragraphs.push(para);
            }
            current_lines.clear();
            current_is_single_visual_line = true;
        }
        if let Some(first) = current_lines.first() {
            current_is_single_visual_line &= line.has_same_rotation(first)
                && (line.upright_baseline() - first.upright_baseline()).abs() <= INLINE_STYLE_BASELINE_TOLERANCE;
        }
        current_lines.push(line);
        prev_idx = line_idx;
    }

    if !current_lines.is_empty()
        && let Some(para) = finalize_paragraph(&current_lines, heading_map, &gap_info)
    {
        paragraphs.push(para);
    }

    tracing::debug!(
        input_lines = lines.len(),
        output_paragraphs = paragraphs.len(),
        headings = paragraphs.iter().filter(|p| p.heading_level.is_some()).count(),
        lists = paragraphs.iter().filter(|p| p.is_list_item).count(),
        "blocks_to_paragraphs complete"
    );

    paragraphs
}

/// Whether a style transition is an inline run on the same visual line.
///
/// PDF glyph runs can overlap slightly because of font metrics. Larger
/// overlaps, reverse ordering, and wide gaps remain structural boundaries.
fn is_inline_style_transition(
    current_is_single_visual_line: bool,
    previous: &SegmentData,
    next: &SegmentData,
    max_forward_gap_font_factor: f32,
) -> bool {
    if previous.is_monospace || next.is_monospace || previous.assigned_role != next.assigned_role {
        return false;
    }
    if !previous.has_same_rotation(next) {
        return false;
    }
    if !previous.font_size.is_finite()
        || !next.font_size.is_finite()
        || previous.font_size <= 0.0
        || next.font_size <= 0.0
        || !previous.upright_baseline().is_finite()
        || !next.upright_baseline().is_finite()
        || !previous.x.is_finite()
        || !next.x.is_finite()
        || !previous.width.is_finite()
        || !next.width.is_finite()
        || previous.width < 0.0
        || next.width < 0.0
    {
        return false;
    }
    let font_size = previous.font_size.max(next.font_size);
    let baseline_delta = (next.upright_baseline() - previous.upright_baseline()).abs();
    let (previous_start, previous_end) = previous.upright_advance_extent();
    let (next_start, _) = next.upright_advance_extent();
    let advance_gap = next_start - previous_end;

    // A sub/superscript is judged against the run it abuts, NOT against the paragraph's first
    // segment, so it is deliberately decided ahead of `current_is_single_visual_line`. That flag
    // answers "has this paragraph wrapped yet", and by the time a subscript appears on a product
    // card's fourth row the answer is yes -- which is why the reproducer stayed torn while every
    // unit test of the pair in isolation passed. The predicate below is tight enough to stand on
    // its own: same rotation, same role, non-monospace, a materially smaller font, a baseline
    // offset of a fraction of it, and a start inside or abutting the previous run. See GH#1617.
    //
    // Containment rather than `INLINE_STYLE_MAX_OVERLAP_FONT_FACTOR` is what reaches the four pairs
    // whose base glyph is not a span of its own: the row arrives as one `TJ` whose kerning spreads
    // its glyphs across the column, so the script starts *within* the previous run's extent, not a
    // few tenths of a point behind its end. ~keep
    if is_script_run_of(previous, next) {
        return true;
    }

    if !current_is_single_visual_line || baseline_delta > INLINE_STYLE_BASELINE_TOLERANCE {
        return false;
    }
    next_start >= previous_start
        && advance_gap >= -(font_size * INLINE_STYLE_MAX_OVERLAP_FONT_FACTOR)
        && advance_gap <= font_size * max_forward_gap_font_factor
}

/// Whether two runs on nearly the same baseline differ the way a sub/superscript differs from its
/// base: measurably smaller, and raised or lowered by a small fraction of the base's font size.
///
/// Deliberately requires a *non-zero* offset. A run at the identical baseline is already handled by
/// `INLINE_STYLE_BASELINE_TOLERANCE`, so this predicate only ever relaxes a comparison the existing
/// gate rejects outright -- it cannot change the outcome of any pair that passes today. ~keep
fn is_script_run_offset(previous: &SegmentData, next: &SegmentData, baseline_delta: f32, font_size: f32) -> bool {
    let smaller_font_size = previous.font_size.min(next.font_size);
    crate::script_run::is_script_run_baseline_offset(baseline_delta, font_size, smaller_font_size)
}

/// Whether `next` reads as a sub/superscript attached to `previous`: same rotation and role,
/// neither monospace, a materially smaller font raised or lowered by a fraction of it, and a start
/// inside or abutting `previous`'s advance extent.
pub(crate) fn is_script_run_of(previous: &SegmentData, next: &SegmentData) -> bool {
    if previous.is_monospace || next.is_monospace || previous.assigned_role != next.assigned_role {
        return false;
    }
    if !previous.has_same_rotation(next) {
        return false;
    }
    if !previous.font_size.is_finite()
        || !next.font_size.is_finite()
        || previous.font_size <= 0.0
        || next.font_size <= 0.0
        || !previous.upright_baseline().is_finite()
        || !next.upright_baseline().is_finite()
        || !previous.x.is_finite()
        || !next.x.is_finite()
        || !previous.width.is_finite()
        || !next.width.is_finite()
        || previous.width < 0.0
        || next.width < 0.0
    {
        return false;
    }
    let font_size = previous.font_size.max(next.font_size);
    let baseline_delta = (next.upright_baseline() - previous.upright_baseline()).abs();
    let (previous_start, previous_end) = previous.upright_advance_extent();
    let (next_start, _) = next.upright_advance_extent();
    is_script_run_offset(previous, next, baseline_delta, font_size)
        && crate::script_run::is_script_run_forward_gap(previous_start, previous_end, next_start, font_size)
}

/// Whether `line` reads as the wrapped continuation of the numbered-heading
/// line `prev`, rather than a new, unrelated line that happens to follow it.
///
/// Both lines' right edges (`upright_advance_extent().1` -- the same geometry
/// [`is_inline_style_transition`] uses for its own right-edge test) are
/// compared: a heading that wraps mid-sentence fills its column before
/// continuing below, so its right edge and the next line's right edge land
/// within [`HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR`] font-sizes of each
/// other. A short heading followed by unrelated content has no reason to share
/// that edge, so the two lines' right edges typically differ by far more.
/// Reused from [`super::paragraphs::merge_continuation_paragraphs`] so the
/// grouper's split and the merge pass's guard agree on the same wrap
/// exemption. See #1467.
pub(super) fn heading_wraps_onto(prev: &SegmentData, line: &SegmentData) -> bool {
    if !prev.has_same_rotation(line) {
        return false;
    }
    if !prev.font_size.is_finite() || !line.font_size.is_finite() {
        return false;
    }
    let (_, prev_end) = prev.upright_advance_extent();
    let (_, line_end) = line.upright_advance_extent();
    if !prev_end.is_finite() || !line_end.is_finite() {
        return false;
    }
    let tolerance = HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR * prev.font_size.max(line.font_size).max(1.0);
    (prev_end - line_end).abs() <= tolerance
}

/// Whether the numbered-heading line `prev` reaches far enough right to have run
/// out of room, which is the "fills its column" half of the wrap rule that
/// [`heading_wraps_onto`]'s doc comment states but its code never measured.
///
/// `next_right_edge` is the widest right edge among the lines that would be merged
/// onto it. A heading that stops well short of that width did not wrap, it ended.
/// Measured: GH#1605's wrapped heading stops 58.7pt short of its own continuation
/// but only ~19pt short of the widest line beneath it, while GH#1609's COMPLETE
/// heading stops hundreds of points short of the body prose it was being welded
/// into. A lowercase opening alone cannot tell those apart -- both continue in
/// lowercase -- which is why it must not be the whole test. See #1609. ~keep
/// Whether `line` is the continuation of a numbered heading set with a HANGING
/// INDENT: the number at the left margin, the title starting to its right, and a
/// title too long for one line resuming at the title's own left edge.
///
/// Two things must hold, and the second alone is not enough. `heading_start` is the
/// first segment of the heading's visual line and `prev` its last, so
/// `prev` starting to the right of `heading_start` is what establishes that this
/// heading HAS a hanging indent at all. Only then does `line` sharing `prev`'s left
/// edge mean "the title continues" rather than "the next line happens to be at the
/// same margin".
///
/// Measured on GH#1615's reproducer, where the wrap and the body that must NOT merge
/// are identical on every other signal this grouper checks -- same font, same weight,
/// same line pitch:
///
/// ```text
/// 5.7.3                                     x 48.24            the number, at the margin
/// Roof terminal combined duct vertical and  x 83.64  y 774.96  the title, indented 35.4pt
/// twin pipe duct vertical                   x 83.64  y 762.24  the wrap -- aligns with the title
/// Appliance category: C33                   x 48.24  y 745.08  the body -- returns to the margin
/// ```
///
/// This is why the right-edge test in [`heading_wraps_onto`] cannot stand alone: a
/// wrap's LAST line is short by definition -- being short is what makes it the last
/// line -- so its right edge never matches the line it continues, and every two-line
/// heading looked like a heading handing off to unrelated content.
///
/// The hanging-indent requirement is what keeps #1467 working: there the heading is a
/// single segment at the margin and the callout beneath it is at the same margin, so
/// `heading_start` and `prev` coincide, no indent is established, and the pair still
/// splits. `starts_section` is evaluated independently of all this, so a following
/// line that is itself a numbered heading breaks regardless. ~keep
pub(super) fn heading_continuation_is_hanging_indent(
    heading_start: &SegmentData,
    prev: &SegmentData,
    line: &SegmentData,
) -> bool {
    if !prev.has_same_rotation(line) || !prev.has_same_rotation(heading_start) {
        return false;
    }
    if !prev.font_size.is_finite() || !line.font_size.is_finite() {
        return false;
    }
    let (heading_left, _) = heading_start.upright_advance_extent();
    let (prev_left, _) = prev.upright_advance_extent();
    let (line_left, _) = line.upright_advance_extent();
    if !heading_left.is_finite() || !prev_left.is_finite() || !line_left.is_finite() {
        return false;
    }
    let tolerance =
        HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR * prev.font_size.max(line.font_size).max(1.0);
    if prev_left - heading_left <= tolerance || (prev_left - line_left).abs() > tolerance {
        return false;
    }
    // GH#1634: left edges alone cannot tell a wrapped heading from a body
    // indented to the title's edge -- number in the margin, title and body
    // alike at one edge, which is how contracts, tenders and many installation
    // manuals are set. A line only wraps when the line before it ran out of
    // room, so require that too: a heading that stops well short of the
    // following line's width did not wrap, it ended. ~keep
    let (_, line_end) = line.upright_advance_extent();
    line_end.is_finite() && heading_fills_column(prev, line_end)
}

pub(super) fn heading_fills_column(prev: &SegmentData, next_right_edge: f32) -> bool {
    if !prev.font_size.is_finite() || !next_right_edge.is_finite() {
        return false;
    }
    let (_, prev_end) = prev.upright_advance_extent();
    if !prev_end.is_finite() {
        return false;
    }
    let tolerance = HEADING_WRAP_RIGHT_EDGE_TOLERANCE_FONT_FACTOR * prev.font_size.max(1.0);
    prev_end >= next_right_edge - tolerance
}

/// Whether the heading line `prev` had run out of room by the time `continuation`
/// began, which is what makes `continuation` a wrap rather than the next thing on
/// the page.
///
/// [`heading_fills_column`]'s fixed two-font-size slack is a stand-in for the
/// question this asks directly: a line wraps when the next word no longer fits, so
/// `prev_end + space + width(first word of the continuation) > column edge`. On a
/// ragged right edge the slack left behind is simply however wide the next word
/// was, which on GH#1758's carrier reaches 50.8pt -- three times the tolerance at
/// 8pt -- so every one of those genuine wraps was refused and its last line handed
/// to the body.
///
/// ~keep The first word's width is ESTIMATED, proportionally, from the
/// continuation's own mean character width (`width / chars`). True per-word advance
/// widths are not reachable here: `SegmentData` carries whole-segment geometry only
/// (`pdf/hierarchy/types.rs`), and a span's chars and glyph widths are dropped
/// where it is built (`pdf/native/hierarchy.rs`). The estimate is defensible
/// because the mean is taken over the SAME line as the word it measures -- same
/// font, same size. Checked against Helvetica's own advance widths on GH#1758's
/// reproducer it lands within -14%/+24% (`patches` 28.02pt true against 25.78pt
/// estimated), and [`heading_fills_column`] stays as a FLOOR, so a face or a line
/// shape the estimate serves badly is never worse off than before.
fn heading_line_ran_out_of_room(prev: &SegmentData, continuation: &SegmentData, column_right_edge: f32) -> bool {
    if heading_fills_column(prev, column_right_edge) {
        return true;
    }
    if !column_right_edge.is_finite() || !prev.font_size.is_finite() {
        return false;
    }
    let (_, prev_end) = prev.upright_advance_extent();
    let Some(leading_word_width) = estimated_leading_word_width(continuation) else {
        return false;
    };
    if !prev_end.is_finite() {
        return false;
    }
    let word_space = HEADING_WRAP_INTERWORD_SPACE_FONT_FACTOR * prev.font_size.max(1.0);
    prev_end + word_space + leading_word_width > column_right_edge
}

/// The width the segment's first whitespace-delimited word would have taken,
/// estimated from the segment's own mean character width. `None` when the segment
/// carries no usable geometry or no text.
fn estimated_leading_word_width(segment: &SegmentData) -> Option<f32> {
    if !segment.width.is_finite() || segment.width <= 0.0 {
        return None;
    }
    let character_count = segment.text.chars().count();
    if character_count == 0 {
        return None;
    }
    let leading_word = segment.text.split_whitespace().next()?;
    let mean_character_width = segment.width / character_count as f32;
    Some(mean_character_width * leading_word.chars().count() as f32)
}

/// The widest right edge of the visual line beneath `after_idx`'s heading run, and
/// that line's leading segment, or `None` when nothing follows.
///
/// This is the "lines beneath the pair" the merge pass's `next_right_edge`
/// already uses (`paragraphs.rs`): the body text a wrapped heading hands off to,
/// not the continuation's own short last line. A wrap's last line is short by
/// definition, so measuring the column against it (as
/// `heading_continuation_is_hanging_indent` does for the hanging-indent shape)
/// is trivially satisfied and cannot tell a genuine wrap from a heading that
/// simply ended. See GH#1650. ~keep
///
/// GH#1758 extends that reasoning from one continuation to a RUN of them. Skipping
/// only `after_idx`'s own baseline left a three-line heading's middle line measured
/// against its own last line (142.1 rather than the column's 281.7), which the
/// `line_end <= next_right_edge` guard then rejected, cutting the heading after
/// line 1. Any further line that would continue the same heading -- same face, same
/// size, same left edge, same pitch ([`continues_heading_run`]) -- is skipped too.
/// When the run reaches the end of the page the last skipped line is used after
/// all, so a heading that ends the page is measured exactly as it was before. ~keep
///
/// ~keep Skipping the run can walk off the end of a block and land on a stub -- a
/// panel label, a page number. Measured on
/// `test_documents/pdf/an_introduction_to_statistical_learning_...`, past a long
/// figure caption the line beneath ends at x 69.9 against a 375.2 measure. A stub
/// is not a column, which is why the caller floors what it gets here at the
/// heading's own right edge before measuring against it.
fn next_visual_line_right_edge(
    lines: &[SegmentData],
    after_idx: usize,
    heading_pitch: f32,
) -> Option<(f32, &SegmentData)> {
    let anchor = &lines[after_idx];
    let (anchor_left, _) = anchor.upright_advance_extent();
    let mut start = after_idx + 1;
    while start < lines.len()
        && lines[start].has_same_rotation(anchor)
        && (lines[start].upright_baseline() - anchor.upright_baseline()).abs() <= INLINE_STYLE_BASELINE_TOLERANCE
    {
        start += 1;
    }
    let mut previous_baseline = anchor.upright_baseline();
    let mut last_skipped = None;
    loop {
        let Some((end, left_edge, right_edge, leader)) = visual_line_group(lines, start) else {
            return last_skipped;
        };
        let measured = right_edge.is_finite().then_some((right_edge, leader));
        if !continues_heading_run(anchor, anchor_left, leader, left_edge, previous_baseline, heading_pitch) {
            return measured;
        }
        last_skipped = measured.or(last_skipped);
        previous_baseline = leader.upright_baseline();
        start = end;
    }
}

/// The visual line beginning at `start`: `(index after it, left edge, right edge,
/// leading segment)`. `None` once `start` is past the end.
fn visual_line_group(lines: &[SegmentData], start: usize) -> Option<(usize, f32, f32, &SegmentData)> {
    let leader = lines.get(start)?;
    let mut right_edge = f32::NEG_INFINITY;
    let mut left_edge = f32::INFINITY;
    let mut end = start;
    while end < lines.len()
        && lines[end].has_same_rotation(leader)
        && (lines[end].upright_baseline() - leader.upright_baseline()).abs() <= INLINE_STYLE_BASELINE_TOLERANCE
    {
        let (left, edge) = lines[end].upright_advance_extent();
        if edge.is_finite() {
            right_edge = right_edge.max(edge);
        }
        if left.is_finite() {
            left_edge = left_edge.min(left);
        }
        end += 1;
    }
    Some((end, left_edge, right_edge, leader))
}

/// Whether the visual line led by `candidate` reads as a further line of the same
/// heading run as `anchor`, rather than the column beneath it.
///
/// All four signals are required together, because none of them separates a
/// heading run from body text on its own: a body paragraph set in the heading's
/// own face at the heading's own margin would otherwise be skipped wholesale, and
/// the column would be measured somewhere far below the heading. See GH#1758. ~keep
fn continues_heading_run(
    anchor: &SegmentData,
    anchor_left: f32,
    candidate: &SegmentData,
    candidate_left: f32,
    previous_baseline: f32,
    heading_pitch: f32,
) -> bool {
    if !anchor.has_same_rotation(candidate) || candidate.is_bold != anchor.is_bold {
        return false;
    }
    if candidate.is_italic != anchor.is_italic {
        return false;
    }
    if !anchor.font_size.is_finite() || !candidate.font_size.is_finite() {
        return false;
    }
    if (candidate.font_size - anchor.font_size).abs() > HEADING_CONTINUATION_FONT_SIZE_TOLERANCE_PT {
        return false;
    }
    if !anchor_left.is_finite() || !candidate_left.is_finite() {
        return false;
    }
    let left_tolerance =
        HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR * anchor.font_size.max(candidate.font_size).max(1.0);
    if (candidate_left - anchor_left).abs() > left_tolerance {
        return false;
    }
    if !heading_pitch.is_finite() || heading_pitch <= 0.0 {
        return false;
    }
    let advance = (previous_baseline - candidate.upright_baseline()).abs();
    (advance - heading_pitch).abs() <= HEADING_CONTINUATION_RUN_PITCH_TOLERANCE_FACTOR * heading_pitch
}

/// Whether `line` continues the numbered heading `prev`/`heading_start` at the
/// page MARGIN: no hanging indent (the shape `heading_continuation_is_hanging_indent`
/// covers), but the heading's own line fills the text column measured against
/// the body beneath the pair -- not `line`'s own width, which a short last line
/// would satisfy trivially -- the continuation resumes at the heading's own left
/// edge, opens lowercase, and keeps the heading's font size and weight.
///
/// This is the third `follows_section` exemption, alongside `heading_wraps_onto`
/// (a RIGHT-edge test that a short wrapped last line fails by construction) and
/// `heading_continuation_is_hanging_indent` (which requires an indent this shape
/// does not have). See GH#1650, where a numbered heading set in the HEADING font
/// -- not the body font GH#1605 already covers -- wraps at the margin and is cut
/// after its first line. GH#1609's control (a COMPLETE heading followed by wider
/// lowercase body prose) and GH#1650's own page-4 control (a complete heading
/// followed by an unrelated bold line) both stay split: neither heading's own
/// line reaches the column edge. ~keep
///
/// GH#1758 loosens the two tests the reporter's nine real titles fail. A lowercase
/// opener is now measured with [`heading_line_ran_out_of_room`], which asks whether
/// the next WORD fitted rather than trusting a fixed slack. A capital opener, which
/// was refused outright, is admitted only when
/// [`heading_style_carries_capital_opener`] holds -- and then only against the
/// strict [`heading_fills_column`], never the word-fit relaxation, so the two
/// loosenings cannot compound on one boundary. ~keep
fn heading_continuation_at_margin(
    heading_start: &SegmentData,
    prev: &SegmentData,
    line: &SegmentData,
    lines: &[SegmentData],
    line_idx: usize,
) -> bool {
    if !prev.has_same_rotation(line) || !prev.has_same_rotation(heading_start) {
        return false;
    }
    if !prev.font_size.is_finite() || !line.font_size.is_finite() {
        return false;
    }
    if (line.font_size - prev.font_size).abs() > HEADING_CONTINUATION_FONT_SIZE_TOLERANCE_PT
        || line.is_bold != prev.is_bold
    {
        return false;
    }
    let (heading_left, _) = heading_start.upright_advance_extent();
    let (line_left, line_end) = line.upright_advance_extent();
    if !heading_left.is_finite() || !line_left.is_finite() || !line_end.is_finite() {
        return false;
    }
    let tolerance =
        HEADING_HANGING_INDENT_LEFT_EDGE_TOLERANCE_FONT_FACTOR * prev.font_size.max(line.font_size).max(1.0);
    if (line_left - heading_left).abs() > tolerance {
        return false;
    }
    let heading_pitch = (prev.upright_baseline() - line.upright_baseline()).abs();
    let Some((line_beneath_edge, beneath)) = next_visual_line_right_edge(lines, line_idx, heading_pitch) else {
        return false;
    };
    // GH#1758: the line beneath is only EVIDENCE of where the column ends, and a line
    // narrower than the heading's own is no evidence at all -- the heading already
    // proved the measure reaches at least that far. Floored here rather than inside
    // `heading_fills_column`, which the merge pass shares and which must keep
    // measuring GH#1605's boundary exactly as it does today. Without the floor a stub
    // beneath a long run (x 69.9 against a 375.2 measure, measured on
    // `test_documents/pdf/an_introduction_to_statistical_learning_...`) rejects the
    // run's own middle line through the overflow guard below. ~keep
    let (_, prev_end) = prev.upright_advance_extent();
    let column_right_edge = if prev_end.is_finite() {
        line_beneath_edge.max(prev_end)
    } else {
        line_beneath_edge
    };
    if line_end > column_right_edge + tolerance {
        return false;
    }
    if line.text.trim_start().chars().next().is_some_and(char::is_lowercase) {
        return heading_line_ran_out_of_room(prev, line, column_right_edge);
    }
    heading_style_carries_capital_opener(prev, line, beneath) && heading_fills_column(prev, column_right_edge)
}

/// Whether a continuation opening with a CAPITAL still reads as the heading's own
/// wrap rather than the next element on the page.
///
/// Refusing a capital outright is right in the great majority of cases -- a
/// sub-heading, a table row or a code line beneath a numbered heading all open
/// capitalised, and all sit at the heading's own margin. What separates GH#1758's
/// genuine wraps from those is that the continuation keeps the heading's own FACE
/// (italic, on the reporter's journal) while the column beneath it does not, so the
/// three lines are one typographic run and the body is not. The same italic change
/// at a line start GH#1740's suggestion 2 proposed as a break term, read the other
/// way round. Weight and size are already required to match by the caller; `prev`
/// closing a sentence is refused as well, because a heading line that ends in `.?!`
/// had no reason to continue. ~keep
fn heading_style_carries_capital_opener(prev: &SegmentData, line: &SegmentData, beneath: &SegmentData) -> bool {
    line.is_italic == prev.is_italic
        && beneath.is_italic != prev.is_italic
        && !matches!(
            prev.text.trim_end().chars().last(),
            Some('.' | '?' | '!' | '\u{3002}' | '\u{FF1F}' | '\u{FF01}')
        )
}

/// Reconstruct PdfLine objects from a flat list of SegmentData, grouping by baseline_y.
///
/// This preserves inline formatting information (is_bold, is_italic, is_monospace)
/// at the segment level so that the assembly layer can emit properly annotated markdown
/// with bold/italic emphasis.
fn reconstruct_pdf_lines(segments: &[&SegmentData]) -> Vec<super::types::PdfLine> {
    const MAX_LINE_TOLERANCE_PT: f32 = 3.0;
    const LINE_TOLERANCE_SCALE_FACTOR: f32 = 0.25;

    fn finish_line(mut segments: Vec<SegmentData>, baseline_y: f32) -> super::types::PdfLine {
        let contains_rtl = segments.iter().any(|segment| {
            segment
                .text
                .chars()
                .any(|character| xberg_native_pdf::text::is_rtl_text(character as u32))
        });
        if !contains_rtl {
            segments.sort_by(|a, b| a.upright_advance_extent().0.total_cmp(&b.upright_advance_extent().0));
        }

        let dominant_font_size = segments.iter().map(|s| s.font_size).fold(0.0, |a, b| {
            if a > 0.0 && b > a / 2.0 && b < a * 2.0 {
                (a + b) / 2.0
            } else {
                a.max(b)
            }
        });
        let is_bold = segments.iter().filter(|s| s.is_bold).count() > segments.len() / 2;
        let is_monospace = segments.iter().all(|s| s.is_monospace);
        super::types::PdfLine {
            segments,
            baseline_y,
            dominant_font_size,
            is_bold,
            is_monospace,
        }
    }

    if segments.is_empty() {
        return Vec::new();
    }

    let mut lines: Vec<super::types::PdfLine> = Vec::new();
    let mut current_baseline = segments[0].upright_baseline();
    let mut current_rotation = segments[0].rotation_degrees;
    let mut current_scale = segments[0].font_size.max(segments[0].height).abs();
    let mut current_segments: Vec<SegmentData> = Vec::new();

    for seg in segments {
        let same_rotation = (seg.rotation_degrees - current_rotation).abs() <= f32::EPSILON;
        let segment_baseline = seg.upright_baseline();
        let segment_scale = seg.font_size.max(seg.height).abs();
        let baseline_tolerance =
            (current_scale.max(segment_scale) * LINE_TOLERANCE_SCALE_FACTOR).min(MAX_LINE_TOLERANCE_PT);
        if !same_rotation || (segment_baseline - current_baseline).abs() > baseline_tolerance {
            if !current_segments.is_empty() {
                lines.push(finish_line(std::mem::take(&mut current_segments), current_baseline));
            }
            current_baseline = segment_baseline;
            current_rotation = seg.rotation_degrees;
            current_scale = segment_scale;
        } else {
            current_scale = current_scale.max(segment_scale);
        }
        current_segments.push((*seg).clone());
    }

    if !current_segments.is_empty() {
        lines.push(finish_line(current_segments, current_baseline));
    }

    lines
}

/// Build a PdfParagraph from a group of consecutive lines with compatible font properties.
fn paragraph_text(lines: &[&SegmentData]) -> String {
    if lines.iter().all(|segment| segment.is_unrotated()) {
        return lines
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
    }

    let mut text = String::new();
    let mut previous: Option<&SegmentData> = None;
    for segment in lines {
        if let Some(previous) = previous {
            if !previous.has_same_rotation(segment) {
                text.push_str("\n\n");
            } else {
                let same_line = (previous.upright_baseline() - segment.upright_baseline()).abs()
                    < previous.height.max(segment.height).max(segment.font_size * 0.5) * 0.5;
                if same_line {
                    let previous_word = previous.text.split_whitespace().next_back().unwrap_or("");
                    let next_word = segment.text.split_whitespace().next().unwrap_or("");
                    if !text.ends_with(char::is_whitespace)
                        && !segment.text.starts_with(char::is_whitespace)
                        && segments_need_space(previous, previous_word, segment, next_word)
                    {
                        text.push(' ');
                    }
                } else {
                    text.push('\n');
                }
            }
        }
        text.push_str(&segment.text);
        previous = Some(segment);
    }
    text
}

fn finalize_paragraph(
    lines: &[&SegmentData],
    heading_map: &[(f32, Option<u8>)],
    gap_info: &super::classify::GapInfo,
) -> Option<PdfParagraph> {
    if lines.is_empty() {
        return None;
    }

    let text = paragraph_text(lines);

    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let first = lines[0];
    let word_count = trimmed.split_whitespace().count();
    let is_bold = lines.iter().filter(|l| l.is_bold).count() > lines.len() / 2;
    let has_mixed_inline_styles = lines
        .iter()
        .skip(1)
        .any(|line| line.is_bold != first.is_bold || line.is_italic != first.is_italic);

    let reconstructed_lines = reconstruct_pdf_lines(lines);
    let starts_with_split_list_marker = lines.get(1).is_some_and(|body| {
        is_bare_list_marker(&first.text)
            && body.has_same_rotation(first)
            && (body.upright_baseline() - first.upright_baseline()).abs() <= INLINE_STYLE_BASELINE_TOLERANCE
            && !body.text.trim().is_empty()
    });
    let is_list_candidate = looks_like_list_item(trimmed) || starts_with_split_list_marker;

    let structure_tree_role = {
        let role_counts: std::collections::HashMap<u8, usize> =
            lines
                .iter()
                .filter_map(|l| l.assigned_role)
                .fold(std::collections::HashMap::new(), |mut acc, level| {
                    *acc.entry(level).or_default() += 1;
                    acc
                });
        role_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(level, _)| level)
    };
    if let Some(level) = structure_tree_role {
        let para_text = trimmed.to_string();
        let word_count = PdfParagraph::compute_word_count(&para_text, &reconstructed_lines);
        return Some(PdfParagraph {
            text: if has_mixed_inline_styles {
                String::new()
            } else {
                para_text
            },
            lines: reconstructed_lines,
            dominant_font_size: first.font_size,
            heading_level: Some(level),
            is_bold,
            is_list_item: is_list_candidate,
            is_code_block: first.is_monospace && lines.len() > 1,
            is_formula: false,
            is_page_furniture: false,
            layout_class: None,
            layout_region_path: None,
            caption_for: None,
            block_bbox: None,
            word_count,
        });
    }

    // Shape-only page-number test (GH#1411). It is used here purely to *suppress*
    // heading promotion, which is non-destructive. The decision to mark such a
    // paragraph as deletable furniture is made document-wide in
    // `mark_validated_page_numbers`, which additionally requires margin position
    // and cross-page sequence agreement.
    let page_number_like =
        word_count <= MAX_PAGE_NUMBER_WORD_COUNT && super::page_number::classify_page_number_text(trimmed).is_some();

    let mut heading_level = super::classify::find_heading_level(first.font_size, heading_map, gap_info);
    if heading_level.is_some()
        && (word_count > 20
            || super::layout_classify::is_separator_text(trimmed)
            || page_number_like
            // A heading is not a sentence. This gate had no shape test, so a block whose font
            // clustered above body became a heading on word count alone -- and 20 words is a whole
            // sentence. On a scanned page that promoted ordinary prose and split the paragraph in
            // two, the promoted line becoming a heading and its continuation staying body text.
            // The bold branch below already refuses a block that ends in a period; this is the same
            // judgement, plus the case where the line runs on past an interior full stop. GH#1599.
            // ~keep
            || super::classify::reads_as_body_content(trimmed, word_count)
            || (SUPPRESS_LOWERCASE_START_HEADINGS && super::classify::starts_with_lowercase_or_continuation(trimmed)))
    {
        heading_level = None;
    }

    let body_font_size = heading_map
        .iter()
        .find(|(_, level)| level.is_none())
        .map(|(centroid, _)| *centroid)
        .unwrap_or(0.0);

    // A bold, short, single-line paragraph is only a heading candidate when
    // its font is also meaningfully larger than the document's body font
    // (the same ratio/gap the font-size clustering path already requires via
    // `assign_heading_levels_smart`). Without this check, any bold one-word
    // line — including body-sized emphasis, or a stray oversized glyph from
    // a font-metric artifact — gets promoted regardless of scale, which is
    // exactly the pattern that over-promoted "Big"/"Text" in a 3-paragraph
    // document with no real headings. ~keep
    let clears_bold_font_gate = body_font_size > 0.0
        && first.font_size >= body_font_size * super::constants::MIN_HEADING_FONT_RATIO
        && first.font_size >= body_font_size + super::constants::MIN_HEADING_FONT_GAP;

    if heading_level.is_none()
        && is_bold
        && clears_bold_font_gate
        && (1..=8).contains(&word_count)
        && lines.len() == 1
        && !trimmed.ends_with('.')
        && !trimmed.ends_with(':')
        && !trimmed.ends_with(',')
        && !trimmed.ends_with(';')
        && !trimmed.contains('@')
        && !trimmed.contains('(')
        && !trimmed.contains(',')
        && trimmed
            .chars()
            .next()
            .is_some_and(|c| c.is_uppercase() || c.is_ascii_digit())
        && !super::layout_classify::is_separator_text(trimmed)
        && !super::regions::looks_like_figure_label(trimmed)
    {
        heading_level = Some(2);
    }

    if heading_level.is_none() {
        let min_heading_threshold = body_font_size * super::constants::MIN_HEADING_FONT_RATIO;
        // `first.font_size >= min_heading_threshold` already implies
        // `first.font_size > body_font_size + 0.5` for every realistic body font size:
        // `body * MIN_HEADING_FONT_RATIO > body + 0.5` reduces to `body > 0.5 / (RATIO - 1) ≈ 3.33`
        // (in whatever unit `font_size` is — points or OCR render pixels), and no real body-text
        // cluster is that small. An explicit `+ 0.5` absolute-unit check was previously required
        // here too; it was redundant on point-scale input and, being absolute, would have been
        // both too-permissive at pixel scale and too-strict on a hypothetically tiny render, so
        // it has been removed rather than converted. ~keep
        if body_font_size > 0.0
            && first.font_size >= min_heading_threshold
            && word_count <= super::constants::MAX_BOLD_HEADING_WORD_COUNT
            && lines.len() <= 2
            && !trimmed.ends_with(':')
            && !trimmed.contains('@')
            && (super::classify::is_section_pattern(trimmed) || is_structural_heading_word(trimmed))
            && !super::layout_classify::is_separator_text(trimmed)
            && !super::regions::looks_like_figure_label(trimmed)
            && !is_list_candidate
            && !page_number_like
        {
            heading_level = Some(2);
        }
    }

    let is_list_item = heading_level.is_none() && is_list_candidate;
    let is_code_block =
        heading_level.is_none() && !is_list_item && lines.iter().all(|l| l.is_monospace) && lines.len() >= 2;

    tracing::debug!(
        font_size = first.font_size,
        is_bold,
        word_count,
        heading_level = ?heading_level,
        is_list_item,
        is_code_block,
        page_number_like,
        text_preview = %&trimmed.chars().take(60).collect::<String>(),
        "classified paragraph"
    );

    let para_text = trimmed.to_string();
    let word_count = PdfParagraph::compute_word_count(&para_text, &reconstructed_lines);

    Some(PdfParagraph {
        text: if has_mixed_inline_styles {
            String::new()
        } else {
            para_text
        },
        lines: reconstructed_lines,
        dominant_font_size: first.font_size,
        heading_level,
        is_bold,
        is_list_item,
        is_code_block,
        is_formula: false,
        // Page-number furniture is decided document-wide, not here — see
        // `mark_validated_page_numbers` (GH#1411).
        is_page_furniture: false,
        layout_class: None,
        layout_region_path: None,
        caption_for: None,
        block_bbox: Some({
            let left = lines.iter().map(|l| l.x).fold(f32::MAX, f32::min);
            let bottom = lines.iter().map(|l| l.baseline_y).fold(f32::MAX, f32::min);
            let right = lines.iter().map(|l| l.x + l.width).fold(f32::MIN, f32::max);
            let top = lines.iter().map(|l| l.baseline_y + l.height).fold(f32::MIN, f32::max);
            (left, bottom, right, top)
        }),
        word_count,
    })
}

/// Check if text is ENTIRELY a list marker with no item text after it.
///
/// Word processors often emit list numbering as its own text run, so the
/// marker ("1.", "a)", "(2)", "•") and the item body arrive as separate
/// spans on the same line. `looks_like_list_item` rejects those markers
/// because it requires trailing text; this predicate accepts them.
pub(super) fn is_bare_list_marker(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || t.chars().count() > 5 {
        return false;
    }
    let mut marker_chars = t.chars();
    if marker_chars.next().is_some_and(super::list_marker::is_bullet_glyph) && marker_chars.next().is_none() {
        return true;
    }
    if matches!(t, "–" | "—" | "-" | "*") {
        return true;
    }
    super::list_marker::parse_ordered_list_marker(t).is_some_and(|marker| !marker.has_content)
}

/// Check if text starts with a common list marker.
///
/// Also consulted by the OCR+layout paragraph route (`extractors::pdf::ocr`), which has
/// no list classification of its own: reusing this predicate keeps the two routes from
/// drifting into two different notions of what a list marker is.
pub(crate) fn looks_like_list_item(text: &str) -> bool {
    let t = text.trim_start();

    if t.starts_with(super::list_marker::BULLET_GLYPHS) {
        return true;
    }

    if let Some(rest) = t.strip_prefix('–').or_else(|| t.strip_prefix('—')) {
        if !rest.starts_with(' ') && !rest.starts_with('\t') {
            return false;
        }
        let body = rest.trim_start_matches([' ', '\t']);
        return !body.is_empty() && !body.starts_with('\r') && !body.starts_with('\n');
    }

    if let Some(rest) = t.strip_prefix("- ") {
        return rest.chars().next().is_some_and(|c| c.is_alphabetic());
    }

    if super::classify::is_numbered_section_heading(t) {
        return false;
    }
    let Some(marker) = super::list_marker::parse_ordered_list_marker(t) else {
        return false;
    };
    let Some(first_content_char) = t.get(marker.content_start..).and_then(|content| content.chars().next()) else {
        return false;
    };
    marker.has_content
        && marker.has_separator
        && !is_probable_author_byline(t)
        && first_content_char.is_alphabetic()
        && !is_inline_parenthesized_quantity(t, &marker, first_content_char)
}

/// Reject a line-leading `(N)` when it reads as a mid-sentence quantity
/// clarification -- "Two\n(2) additional on-street parking spaces" wraps onto
/// a physical line that *starts* with `(2)`, which is shaped identically to a
/// genuine numbered marker like `(2) Second item`.
///
/// The distinguishing signal is capitalization plus how the marker was
/// separated from its content:
///
/// - A **newline** between the marker and its content (`"(2)\nsecond item"`)
///   means the marker arrived as its own text run, glued to the next run by
///   line reconstruction rather than by the source author -- that shape is
///   trusted regardless of case, exactly as it always has been.
/// - A plain **space** on the same physical line, followed by a **lowercase**
///   word (`"(2) additional …"`, `"(7) on-street …"`), is the shape of a
///   number spelled out in prose ("Two (2) additional…") that happens to
///   start a wrapped line. A genuine enumerated item is a new sentence and so
///   starts with a capital letter (`"(2) Second point."`); this heuristic
///   costs nothing there.
///
/// Scoped to `(`-parenthesized **numeric** markers only: `(a)`/`(b)`/`(c)` are
/// this same ordinance's genuine sub-item markers (never quantity
/// clarifications, since nobody writes "two (b) items"), and non-parenthesized
/// families (`"1. "`, `"[1] "`) have no equivalent English idiom that produces
/// this false positive, so they are left untouched.
fn is_inline_parenthesized_quantity(
    t: &str,
    marker: &super::list_marker::OrderedListMarker,
    first_content_char: char,
) -> bool {
    if !t.starts_with('(') || marker.numeric_value.is_none() {
        return false;
    }
    let separator_region = t.get(..marker.content_start).unwrap_or("");
    if separator_region.contains(['\n', '\r']) {
        return false;
    }
    !first_content_char.is_uppercase()
}

/// Whether a single-capital marker is more likely the first author initial.
///
/// The comma plus a second compact initial or journal-style slash supplies
/// the contextual evidence; a standalone `A. First item` remains a list.
pub(super) fn is_probable_author_byline(text: &str) -> bool {
    let mut chars = text.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_uppercase()) || chars.next() != Some('.') {
        return false;
    }
    let remainder = chars.as_str().trim_start();
    let Some((surname, remainder)) = remainder.split_once(char::is_whitespace) else {
        return false;
    };
    surname.ends_with(',') && starts_with_author_initial_or_slash(remainder.trim_start())
}

fn starts_with_author_initial_or_slash(text: &str) -> bool {
    if text.starts_with('/') {
        return true;
    }
    let mut chars = text.chars().peekable();
    let mut initials = 0;
    while chars.peek().is_some_and(|c| c.is_ascii_uppercase()) {
        chars.next();
        if chars.next() != Some('.') {
            return false;
        }
        initials += 1;
    }
    initials > 0 && chars.peek().is_some_and(|c| c.is_whitespace())
}

/// Check if text is a well-known structural heading word.
///
/// These single-word headings appear frequently in academic papers and reports
/// and are reliable heading indicators when combined with a larger-than-body font.
fn is_structural_heading_word(text: &str) -> bool {
    let t = text.trim();
    matches!(
        t,
        "Abstract"
            | "References"
            | "Appendix"
            | "Acknowledgments"
            | "Acknowledgements"
            | "Conclusion"
            | "Conclusions"
            | "Bibliography"
            | "Contents"
            | "Index"
            | "Glossary"
            | "Summary"
            | "Discussion"
            | "Methods"
            | "Results"
            | "Methodology"
    )
}

/// Build a structured `InternalDocument` from pre-extracted per-page segments.
///
/// This is the native-backend entry point. It accepts segments already extracted
/// via `native::hierarchy::extract_all_segments` and runs the same font-clustering,
/// heading-classification, paragraph-assembly, and post-processing stages without
/// requiring a PDF document.
///
/// Image positions can be supplied to insert image placeholders into the document.
/// Layout hints (from RT-DETR layout detection) are optional; when present they
/// drive furniture marking, heading overrides, and table region detection.
///
/// Returns the assembled `InternalDocument`.
pub(crate) struct SegmentStructureConfig<'a> {
    pub k_clusters: usize,
    pub tables: &'a [crate::types::Table],
    pub outline_entries: &'a [PdfOutlineEntry],
    pub strip_repeating_text: bool,
    pub include_headers: bool,
    pub include_footers: bool,
    pub include_footnotes: bool,
    pub include_watermarks: bool,
    pub used_structure_tree: bool,
    pub image_positions: &'a [(u32, u32)],
    pub images: Option<&'a [crate::types::ExtractedImage]>,
    pub inject_placeholders: bool,
    pub layout_hints: Option<&'a [Vec<LayoutHint>]>,
    pub allow_single_column: bool,
    pub cancel_token: Option<&'a crate::cancellation::CancellationToken>,
    #[cfg(feature = "layout-detection")]
    pub layout_images: Option<&'a [image::RgbImage]>,
    #[cfg(feature = "layout-detection")]
    pub layout_results: Option<&'a [super::types::PageLayoutResult]>,
    #[cfg(feature = "layout-detection")]
    pub table_model: crate::core::config::layout::TableModel,
    #[cfg(feature = "layout-detection")]
    pub table_overlap_preference: crate::core::config::layout::TableOverlapPreference,
    #[cfg(feature = "layout-detection")]
    pub acceleration: Option<&'a crate::core::config::acceleration::AccelerationConfig>,
    #[cfg(feature = "layout-detection")]
    pub session_thread_budget: usize,
}

#[cfg(feature = "layout-detection")]
fn slanet_variant_for_table_model(table_model: crate::core::config::layout::TableModel) -> Option<&'static str> {
    use crate::core::config::layout::TableModel;

    match table_model {
        TableModel::SlanetWired | TableModel::SlanetAuto => Some("slanet_wired"),
        TableModel::SlanetWireless => Some("slanet_wireless"),
        TableModel::SlanetPlus => Some("slanet_plus"),
        TableModel::Tatr | TableModel::Disabled => None,
    }
}

pub(crate) fn extract_document_structure_from_segments(
    mut all_page_segments: Vec<Vec<SegmentData>>,
    config: SegmentStructureConfig<'_>,
) -> Result<crate::types::internal::InternalDocument> {
    let SegmentStructureConfig {
        k_clusters,
        tables,
        outline_entries,
        strip_repeating_text,
        include_headers,
        include_footers,
        include_footnotes,
        include_watermarks,
        used_structure_tree,
        image_positions,
        images,
        inject_placeholders,
        layout_hints,
        allow_single_column,
        cancel_token,
        #[cfg(feature = "layout-detection")]
        layout_images,
        #[cfg(feature = "layout-detection")]
        layout_results,
        #[cfg(feature = "layout-detection")]
        table_model,
        #[cfg(feature = "layout-detection")]
        table_overlap_preference,
        #[cfg(feature = "layout-detection")]
        acceleration,
        #[cfg(feature = "layout-detection")]
        session_thread_budget,
    } = config;
    let page_count = all_page_segments.len();
    tracing::debug!(
        page_count,
        used_structure_tree,
        "native structure pipeline: starting from pre-extracted segments"
    );

    let struct_tree_results: Vec<Option<Vec<PdfParagraph>>> = vec![None; page_count];
    let heuristic_pages: Vec<usize> = (0..page_count).collect();

    let (heading_map, doc_body_font_size) = if used_structure_tree {
        let mut heading_map = build_heading_map_from_assigned_roles(&all_page_segments);
        if !suppress_all_heading_roles_when_sparse_and_untrusted(&mut heading_map, &mut all_page_segments)
            && promote_untagged_document_title(&mut heading_map, &all_page_segments)
        {
            demote_assigned_roles(&mut all_page_segments);
        }
        let doc_body_font_size: Option<f32> = heading_map
            .iter()
            .find(|(_, level)| level.is_none())
            .map(|(size, _)| *size);
        tracing::debug!(
            heading_map_len = heading_map.len(),
            "native structure pipeline: heading map from structure tree"
        );
        (heading_map, doc_body_font_size)
    } else {
        let (heading_map, _struct_tree_needs_classify) =
            build_heading_map(&all_page_segments, &struct_tree_results, &heuristic_pages, k_clusters)?;
        let doc_body_font_size: Option<f32> = heading_map
            .iter()
            .find(|(_, level)| level.is_none())
            .map(|(size, _)| *size);
        (heading_map, doc_body_font_size)
    };

    let page_heights: Vec<f32> = all_page_segments
        .iter()
        .map(|segs| segs.iter().map(|s| s.y + s.height).fold(0.0_f32, f32::max).max(792.0))
        .collect();

    let mut layout_tables: Vec<crate::types::Table> = Vec::new();
    if let Some(hints_pages) = layout_hints {
        struct TablePageData {
            page_idx: usize,
            words: Vec<crate::pdf::table_reconstruct::HocrWord>,
            page_height: f32,
        }
        let mut table_pages: Vec<TablePageData> = Vec::new();
        // Geometric fallback (#1316): pages the ML detector left without any
        // Table region, but whose text geometry forms a column-aligned grid.
        // Reconstructed through the same guarded path as ML hints, below.
        let mut geometric_table_pages: Vec<(
            usize,
            Vec<crate::pdf::table_reconstruct::HocrWord>,
            f32,
            Vec<LayoutHint>,
        )> = Vec::new();

        #[allow(clippy::needless_range_loop)]
        for page_idx in 0..page_count {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                tracing::debug!(page_idx, "native structure pipeline: cancelled during table page prep");
                break;
            }
            let Some(hints) = hints_pages.get(page_idx) else {
                continue;
            };
            let ml_table_hints: Vec<&LayoutHint> = hints
                .iter()
                .filter(|h| h.class_name == super::types::LayoutHintClass::Table)
                .collect();
            let has_table_hint = !ml_table_hints.is_empty();
            #[cfg(feature = "layout-detection")]
            let page_height = layout_results
                .and_then(|results| results.get(page_idx))
                .map(|pr| pr.page_height_pts)
                .unwrap_or(page_heights[page_idx]);
            #[cfg(not(feature = "layout-detection"))]
            let page_height = page_heights[page_idx];
            let words = crate::pdf::table_reconstruct::segments_to_words(&all_page_segments[page_idx], page_height);
            if words.is_empty() {
                tracing::trace!(
                    page = page_idx,
                    "native layout table extraction: no words from segments, skipping"
                );
                continue;
            }
            // Run the geometric fallback per-region rather than per-page (#1321):
            // an ML `Table` hint elsewhere on the page must not suppress recovery
            // of a spatially separate borderless region, so both paths run
            // whenever a page has words, with the fallback excluding words
            // already claimed by an ML table hint.
            let synthetic = super::regions::detect_geometric_table_hints(&words, page_height, &ml_table_hints);
            if !synthetic.is_empty() {
                tracing::debug!(
                    page = page_idx,
                    regions = synthetic.len(),
                    has_table_hint,
                    "geometric table fallback: synthesized Table region(s) outside existing ML hints"
                );
                geometric_table_pages.push((page_idx, words.clone(), page_height, synthetic));
            }
            if has_table_hint {
                tracing::trace!(
                    page = page_idx,
                    word_count = words.len(),
                    page_height,
                    "native layout table extraction: page prepared"
                );
                table_pages.push(TablePageData {
                    page_idx,
                    words,
                    page_height,
                });
            }
        }

        #[cfg(feature = "layout-detection")]
        {
            use crate::core::config::layout::TableModel;

            let use_model_inference = table_model != TableModel::Disabled;

            let slanet_variant = slanet_variant_for_table_model(table_model);
            let is_auto = table_model == TableModel::SlanetAuto;

            let model_name = match table_model {
                TableModel::Tatr => "TATR",
                TableModel::SlanetWired | TableModel::SlanetWireless | TableModel::SlanetPlus => "SLANeXT",
                TableModel::SlanetAuto => "SLANeXT (auto)",
                TableModel::Disabled => "disabled",
            };

            let has_table_model = if use_model_inference {
                let available = match table_model {
                    TableModel::Tatr => crate::layout::is_tatr_available(acceleration, session_thread_budget),
                    TableModel::SlanetWired
                    | TableModel::SlanetWireless
                    | TableModel::SlanetPlus
                    | TableModel::SlanetAuto => slanet_variant.is_some_and(|variant| {
                        crate::layout::is_slanet_available(variant, acceleration, session_thread_budget)
                    }),
                    TableModel::Disabled => false,
                };

                if !available && !table_pages.is_empty() {
                    return Err(crate::pdf::error::PdfError::TextExtractionFailed(format!(
                        "Layout detection found table regions but {model_name} model is not available. \
                         Ensure the ONNX model is downloaded. Tables cannot be extracted without it."
                    )));
                }
                available
            } else {
                false
            };

            if has_table_model {
                if let (Some(images @ [_, ..]), Some(results @ [_, ..])) = (layout_images, layout_results) {
                    #[cfg(not(target_arch = "wasm32"))]
                    let recognized_tables: Vec<Vec<crate::types::Table>> = table_pages
                        .iter()
                        .map(|tp| {
                            if let Some(variant) = slanet_variant {
                                let Some(mut slanet) =
                                    crate::layout::take_or_create_slanet(variant, acceleration, session_thread_budget)
                                else {
                                    tracing::warn!("SLANeXT model unavailable in worker thread");
                                    return Vec::new();
                                };

                                if let (Some(page_image), Some(page_result)) =
                                    (images.get(tp.page_idx), results.get(tp.page_idx))
                                {
                                    let hints = &hints_pages[tp.page_idx];
                                    let mut classifier_pair = if is_auto {
                                        match (
                                            crate::layout::take_or_create_table_classifier(
                                                acceleration,
                                                session_thread_budget,
                                            ),
                                            crate::layout::take_or_create_slanet(
                                                "slanet_wireless",
                                                acceleration,
                                                session_thread_budget,
                                            ),
                                        ) {
                                            (Some(classifier), Some(alternate)) => Some((classifier, alternate)),
                                            _ => None,
                                        }
                                    } else {
                                        None
                                    };
                                    let classifier_arg = classifier_pair
                                        .as_mut()
                                        .map(|(classifier, alternate)| (&mut ***classifier, &mut ***alternate));
                                    let slanet_tables = super::regions::recognize_tables_slanet(
                                        page_image,
                                        hints,
                                        &tp.words,
                                        page_result,
                                        tp.page_height,
                                        tp.page_idx,
                                        &mut slanet,
                                        classifier_arg,
                                    );
                                    if !slanet_tables.is_empty() {
                                        return slanet_tables;
                                    }
                                }

                                let hints = &hints_pages[tp.page_idx];
                                super::regions::extract_tables_from_layout_hints(
                                    &tp.words,
                                    hints,
                                    tp.page_idx,
                                    tp.page_height,
                                    0.5,
                                    allow_single_column,
                                    false,
                                )
                            } else {
                                let Some(mut tatr) =
                                    crate::layout::take_or_create_tatr(acceleration, session_thread_budget)
                                else {
                                    tracing::warn!("TATR model unavailable in worker thread");
                                    return Vec::new();
                                };

                                if let (Some(page_image), Some(page_result)) =
                                    (images.get(tp.page_idx), results.get(tp.page_idx))
                                {
                                    let hints = &hints_pages[tp.page_idx];
                                    let tatr_tables = super::regions::recognize_tables_for_native_page(
                                        page_image,
                                        hints,
                                        &tp.words,
                                        page_result,
                                        tp.page_height,
                                        super::regions::NativeTatrRecognitionOptions {
                                            page_index: tp.page_idx,
                                            allow_single_column,
                                        },
                                        &mut tatr,
                                    );
                                    if !tatr_tables.is_empty() {
                                        return tatr_tables;
                                    }
                                }

                                let hints = &hints_pages[tp.page_idx];
                                super::regions::extract_tables_from_layout_hints(
                                    &tp.words,
                                    hints,
                                    tp.page_idx,
                                    tp.page_height,
                                    0.5,
                                    allow_single_column,
                                    false,
                                )
                            }
                        })
                        .collect();
                    #[cfg(target_arch = "wasm32")]
                    let recognized_tables: Vec<Vec<crate::types::Table>> = table_pages
                        .iter()
                        .map(|tp| {
                            if let (Some(page_image), Some(page_result)) =
                                (images.get(tp.page_idx), results.get(tp.page_idx))
                            {
                                let hints = &hints_pages[tp.page_idx];
                                let Some(mut tatr) =
                                    crate::layout::take_or_create_tatr(acceleration, session_thread_budget)
                                else {
                                    return Vec::new();
                                };
                                let tatr_tables = super::regions::recognize_tables_for_native_page(
                                    page_image,
                                    hints,
                                    &tp.words,
                                    page_result,
                                    tp.page_height,
                                    super::regions::NativeTatrRecognitionOptions {
                                        page_index: tp.page_idx,
                                        allow_single_column,
                                    },
                                    &mut tatr,
                                );
                                if !tatr_tables.is_empty() {
                                    return tatr_tables;
                                }
                                super::regions::extract_tables_from_layout_hints(
                                    &tp.words,
                                    hints,
                                    tp.page_idx,
                                    tp.page_height,
                                    0.5,
                                    allow_single_column,
                                    false,
                                )
                            } else {
                                Vec::new()
                            }
                        })
                        .collect();
                    layout_tables.extend(recognized_tables.into_iter().flatten());
                } else {
                    for tp in &table_pages {
                        if cancel_token.is_some_and(|t| t.is_cancelled()) {
                            tracing::debug!("native structure pipeline: cancelled during heuristic table extraction");
                            break;
                        }
                        let hints = &hints_pages[tp.page_idx];
                        layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                            &tp.words,
                            hints,
                            tp.page_idx,
                            tp.page_height,
                            0.5,
                            allow_single_column,
                            false,
                        ));
                    }
                }
            } else {
                for tp in &table_pages {
                    if cancel_token.is_some_and(|t| t.is_cancelled()) {
                        tracing::debug!("native structure pipeline: cancelled during heuristic table extraction");
                        break;
                    }
                    let hints = &hints_pages[tp.page_idx];
                    layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                        &tp.words,
                        hints,
                        tp.page_idx,
                        tp.page_height,
                        0.5,
                        allow_single_column,
                        false,
                    ));
                }
            }
        }

        #[cfg(not(feature = "layout-detection"))]
        for tp in &table_pages {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                tracing::debug!("native structure pipeline: cancelled during heuristic table extraction");
                break;
            }
            let hints = &hints_pages[tp.page_idx];
            layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                &tp.words,
                hints,
                tp.page_idx,
                tp.page_height,
                0.5,
                allow_single_column,
                false,
            ));
        }

        // Geometric table fallback (#1316): reconstruct the synthesized regions
        // through the SAME guarded path (post_process_table, is_well_formed_table,
        // numeric-exemption prose gate, code-listing/single-cell-row guards). This
        // never runs the ML table models — it only recovers tables the detector
        // missed on otherwise Table-region-free pages.
        for (page_idx, words, page_height, synthetic_hints) in &geometric_table_pages {
            if cancel_token.is_some_and(|t| t.is_cancelled()) {
                tracing::debug!("native structure pipeline: cancelled during geometric table fallback");
                break;
            }
            let before = layout_tables.len();
            layout_tables.extend(super::regions::extract_tables_from_layout_hints(
                words,
                synthetic_hints,
                *page_idx,
                *page_height,
                0.5,
                allow_single_column,
                // Geometrically pre-vetted (row/column/gutter guards): skip the
                // downstream columnar-prose heuristic that mistakes a regular
                // key-value grid for wrapped prose (#1319).
                true,
            ));
            let recovered = layout_tables.len() - before;
            if recovered > 0 {
                tracing::debug!(
                    page = page_idx,
                    recovered,
                    "geometric table fallback: recovered table(s) the ML detector missed"
                );
            }
        }
    }

    tracing::debug!(
        layout_tables_found = layout_tables.len(),
        "native layout table extraction complete"
    );

    #[cfg(feature = "layout-detection")]
    let overlap_preference = table_overlap_preference;
    #[cfg(not(feature = "layout-detection"))]
    let overlap_preference = crate::core::config::layout::TableOverlapPreference::Content;
    let stitched_native_tables = stitch_fragmented_tables(tables.to_vec(), &all_page_segments);
    let emitted_tables = prepare_emitted_tables(&stitched_native_tables, layout_tables, overlap_preference);

    let extracted_table_bboxes_by_page = table_bboxes_by_page(&emitted_tables);
    tracing::debug!(
        native_tables = tables.len(),
        emitted_tables = emitted_tables.len(),
        pages_with_bboxes = extracted_table_bboxes_by_page.len(),
        "native table bbox suppression map built"
    );

    #[cfg(feature = "layout-detection")]
    let validations_by_page: ahash::AHashMap<usize, Vec<super::regions::layout_validation::RegionValidation>> = {
        let mut map = ahash::AHashMap::new();
        if let (Some(images), Some(results), Some(hints_pages)) = (layout_images, layout_results, layout_hints) {
            for page_idx in 0..page_count {
                if let (Some(img), Some(res), Some(hints)) =
                    (images.get(page_idx), results.get(page_idx), hints_pages.get(page_idx))
                {
                    let validations = super::regions::layout_validation::validate_page_regions(img, hints, res);
                    if validations.contains(&super::regions::layout_validation::RegionValidation::Empty) {
                        tracing::debug!(
                            page = page_idx,
                            empty_count = validations
                                .iter()
                                .filter(|v| **v == super::regions::layout_validation::RegionValidation::Empty)
                                .count(),
                            "native layout validation: found empty regions"
                        );
                    }
                    map.insert(page_idx, validations);
                }
            }
        }
        map
    };
    #[cfg(feature = "layout-detection")]
    let effective_layout_hints = layout_hints;
    #[cfg(feature = "layout-detection")]
    let native_layout_projections: Vec<Option<NativeLayoutProjection>> = (0..page_count)
        .map(|page_index| {
            let hints = effective_layout_hints.and_then(|pages| pages.get(page_index))?;
            let validations = validations_by_page
                .get(&page_index)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let wrapper_ownership = wrapper_ownership_by_hint(hints, validations);
            if !crate::extractors::pdf::reading_order::has_eligible_layout_hints(hints, &wrapper_ownership) {
                return None;
            }

            let table_bboxes = extracted_table_bboxes_by_page
                .get(&page_index)
                .map(Vec::as_slice)
                .unwrap_or_default();
            let projected_segments =
                filter_segments_by_table_bboxes(all_page_segments[page_index].clone(), table_bboxes);
            let no_reorder = super::layout_debug::layout_debug_flags().no_reorder;
            let page_width_pts = layout_results
                .and_then(|results| results.get(page_index))
                .map(|result| result.page_width_pts);
            let groups = crate::extractors::pdf::reading_order::plan_segment_groups_by_layout(
                &projected_segments,
                hints,
                &wrapper_ownership,
                no_reorder,
                page_width_pts,
            );
            let group_bounds = layout_group_bounds(&groups, &projected_segments);
            Some(NativeLayoutProjection {
                groups,
                group_bounds,
                classification_hints: regular_layout_hints(hints),
            })
        })
        .collect();
    let witnesses = TextRepairWitnesses {
        hyphens: collect_hyphen_witnesses(&all_page_segments),
        words: collect_word_witnesses(&all_page_segments),
    };
    let page_inputs: Vec<PageInput> = (0..page_count)
        .map(|i| {
            let heuristic_segments = std::mem::take(&mut all_page_segments[i]);
            let paragraph_gap_ys = compute_paragraph_gap_ys(&heuristic_segments);
            PageInput {
                page_index: i,
                struct_paragraphs: None,
                heuristic_segments,
                // Native paragraphs are fully refined without layout semantic
                // annotations. Geometry is projected after semantic refinement.
                page_hints: None,
                table_bboxes: extracted_table_bboxes_by_page.get(&i).cloned().unwrap_or_default(),
                preserve_native_semantics: true,
                use_layout_reading_order: false,
                #[cfg(feature = "layout-detection")]
                hint_validations: validations_by_page.get(&i).cloned().unwrap_or_default(),
                #[cfg(feature = "layout-detection")]
                page_width_pts: layout_results
                    .and_then(|results| results.get(i))
                    .map(|result| result.page_width_pts),
                needs_classify: false,
                paragraph_gap_ys,
                include_headers,
                include_footers,
                include_footnotes,
            }
        })
        .collect();

    if cancel_token.is_some_and(|t| t.is_cancelled()) {
        return Err(crate::pdf::error::PdfError::TextExtractionFailed(
            "extraction cancelled".to_string(),
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    let mut all_page_paragraphs: Vec<Vec<PdfParagraph>> = page_inputs
        .into_par_iter()
        .map(|input| process_single_page(input, &heading_map, doc_body_font_size, &witnesses))
        .collect();
    #[cfg(target_arch = "wasm32")]
    let mut all_page_paragraphs: Vec<Vec<PdfParagraph>> = page_inputs
        .into_iter()
        .map(|input| process_single_page(input, &heading_map, doc_body_font_size, &witnesses))
        .collect();

    refine_heading_hierarchy(&mut all_page_paragraphs);
    demote_unnumbered_subsections(&mut all_page_paragraphs);
    demote_heading_runs(&mut all_page_paragraphs);
    split_colon_semicolon_run_in_lists(&mut all_page_paragraphs);

    if strip_repeating_text {
        mark_cross_page_repeating_text(&mut all_page_paragraphs, &page_heights);
        mark_cross_page_repeating_short_text(&mut all_page_paragraphs);
    }
    if !include_watermarks {
        mark_arxiv_noise(&mut all_page_paragraphs);
    }
    recover_headings_from_outline(&mut all_page_paragraphs, outline_entries);
    // Runs after heading recovery (so recovered headings are excluded) and
    // immediately before the deletion pass it feeds. It needs every page in
    // hand, which is why it cannot live in `process_single_page`.
    mark_validated_page_numbers(
        &mut all_page_paragraphs,
        &page_heights,
        include_headers,
        include_footers,
    );
    for page in &mut all_page_paragraphs {
        retain_page_furniture_safely(page);
    }
    if strip_repeating_text {
        deduplicate_paragraphs(&mut all_page_paragraphs, &extracted_table_bboxes_by_page);
    }
    compact_final_heading_hierarchy(&mut all_page_paragraphs);
    promote_repeated_body_size_bold_headings(&mut all_page_paragraphs, doc_body_font_size);
    #[cfg(feature = "layout-detection")]
    for (page, projection) in all_page_paragraphs.iter_mut().zip(native_layout_projections) {
        let Some(projection) = projection else {
            continue;
        };
        assign_native_paragraph_layout(page, &projection.groups, &projection.group_bounds);
        super::layout_classify::annotate_layout_classes(page, &projection.classification_hints, 0.5, 0.2);
    }
    // Spill cleanup runs after deferred annotation so layout-only captions,
    // footnotes, and furniture retain their semantic provenance.
    suppress_table_dominant_paragraph_spill(&mut all_page_paragraphs, &emitted_tables);
    // Native semantic roles are finalized in source order before the
    // independent layout geometry projection controls final reading order.
    reorder_pages_by_layout_region(&mut all_page_paragraphs);

    let total_paragraphs: usize = all_page_paragraphs.iter().map(|p| p.len()).sum();
    tracing::debug!(
        total_paragraphs,
        heading_map_len = heading_map.len(),
        "native structure pipeline: paragraph extraction complete, assembling document"
    );

    let effective_image_positions = if inject_placeholders { image_positions } else { &[] };
    let mut doc = assemble_internal_document(
        all_page_paragraphs,
        &emitted_tables,
        images,
        effective_image_positions,
        &witnesses.hyphens,
    );

    for elem in &mut doc.elements {
        if elem.text.is_empty() {
            continue;
        }
        let t1 = repair_contextual_ligatures(&elem.text);
        let t2 = expand_ligatures_with_space_absorption(&t1);
        let t3 = normalize_unicode_text(&t2);
        if let Cow::Owned(normalized) = t3 {
            elem.text = normalized;
        } else if let Cow::Owned(normalized) = t2 {
            elem.text = normalized;
        } else if let Cow::Owned(normalized) = t1 {
            elem.text = normalized;
        }
    }

    tracing::debug!(
        elements = doc.elements.len(),
        "native structure pipeline: assembly complete"
    );

    Ok(doc)
}

/// Maximum vertical gap (PDF points) between one fragment's bottom edge and the
/// next fragment's top edge for the two to be considered the same physical
/// table split by `native::table`'s row-gap clustering.
const TABLE_STITCH_Y_GAP_TOLERANCE_PTS: f64 = 4.0;
/// Maximum difference in a chain's shared left/right edge for two fragments to
/// be considered the same table (rather than two unrelated tables that happen
/// to sit close together vertically).
const TABLE_STITCH_X_TOLERANCE_PTS: f64 = 6.0;
/// Bound on fragments merged into one stitched chain. Real continuation splits
/// rarely exceed a handful of fragments; this caps the (already page-scoped,
/// already `native::table::MAX_REGIONS_PER_PAGE`-bounded) chain walk.
const TABLE_STITCH_MAX_CHAIN_FRAGMENTS: usize = 12;
/// Bound on additional data rows the trailing-continuation recovery pass will
/// attempt to pull from raw page segments below a stitched chain's last known
/// fragment. Keeps the scan from reading arbitrarily far down the page.
const TABLE_STITCH_TRAILING_RECOVERY_MAX_ROWS: usize = 6;
/// Row-gap multiplier used to split recovered trailing words into per-entity
/// bands. Mirrors `native::table::cluster_words_into_vertical_regions`'s
/// `row_gap_split`; reimplemented here because that clustering helper is
/// private to the `native::table` module, which this pass cannot depend on.
const TABLE_STITCH_TRAILING_ROW_GAP_MULTIPLIER: f32 = 1.8;

/// Stitch table fragments that `native::table`'s row-gap region clustering split
/// out of one physical table back into a single table.
///
/// `native::table::cluster_words_into_vertical_regions` splits a page's words
/// into regions at any row-gap exceeding `median_height * 1.8`. A table whose
/// header wraps onto several lines, or whose rows are visually separated by
/// generous line spacing, can land in several such regions — each one then
/// independently goes through header/data-row post-processing, which corrupts
/// a real multi-line header (see `post_process_table_inner`'s header cap) and
/// mis-promotes a lone data row to a fake header. This pass reassembles those
/// fragments after the fact: each fragment's own rows are themselves raw
/// word-wrapped sub-lines of a single logical row (there is no reliable way to
/// tell, post hoc, which fragment "really" had a header split correctly), so
/// stitching column-merges every fragment's rows into exactly one row — the
/// topmost fragment in a chain becomes the header, the rest become data rows —
/// and then attempts to recover any trailing data rows that fell below the
/// last known fragment without ever becoming a table fragment at all (e.g.
/// because the row-gap clustering merged them into an unrelated, rejected
/// region).
///
/// Bounded to avoid quadratic blowup: fragments are grouped by page first (an
/// `O(n)` pass), and each page's fragment list is walked once after an
/// `O(m log m)` sort, with the inner chain-adjacency check bounded by
/// `TABLE_STITCH_MAX_CHAIN_FRAGMENTS`. `native::table::MAX_REGIONS_PER_PAGE`
/// already caps how many fragments a single page can contribute.
fn stitch_fragmented_tables(
    tables: Vec<crate::types::Table>,
    all_page_segments: &[Vec<SegmentData>],
) -> Vec<crate::types::Table> {
    let mut by_page: ahash::AHashMap<u32, Vec<crate::types::Table>> = ahash::AHashMap::new();
    let mut unbboxed = Vec::new();
    for table in tables {
        if table.bounding_box.is_some() {
            by_page.entry(table.page_number).or_default().push(table);
        } else {
            unbboxed.push(table);
        }
    }

    let mut result = unbboxed;
    let mut page_numbers: Vec<u32> = by_page.keys().copied().collect();
    page_numbers.sort_unstable();
    for page_number in page_numbers {
        if let Some(page_tables) = by_page.remove(&page_number) {
            result.extend(stitch_page_tables(page_tables, all_page_segments));
        }
    }
    result
}

/// Assign a stable, deterministic `table_id` (and, when missing, `columns`) to
/// every table in `tables`, in the given order.
///
/// Ids are sequential (`"table-1"`, `"table-2"`, ...) rather than derived from
/// randomness or wall-clock time, so the same input document always produces
/// the same ids. Must run over the final, post-dedup set of tables a document
/// will actually emit (see [`prepare_emitted_tables`]) — running it any
/// earlier, e.g. over native tables alone, would leave layout-detected tables
/// that survive dedup without an id.
///
/// Fragments of one physical table that [`stitch_page_tables`] merged into a
/// single [`crate::types::Table`] naturally share one id, since by this point
/// they are already one entry; distinct tables receive distinct ids because
/// they remain distinct entries. Cross-page continuations of one physical
/// table are not linked: [`fragments_are_stitchable`] only merges fragments on
/// the same page, so a table split across a page boundary is intentionally
/// emitted as separate `tables[]` entries with separate ids today. Sharing an
/// id across page-boundary fragments is a known possible future extension,
/// not attempted here.
fn assign_deterministic_table_ids(tables: &mut [crate::types::Table]) {
    for (index, table) in tables.iter_mut().enumerate() {
        table.table_id = Some(format!("table-{}", index + 1));
        if table.columns.is_none() {
            table.columns = table.cells.first().cloned();
        }
    }
}

/// Stitch one page's table fragments. See [`stitch_fragmented_tables`].
fn stitch_page_tables(
    mut fragments: Vec<crate::types::Table>,
    all_page_segments: &[Vec<SegmentData>],
) -> Vec<crate::types::Table> {
    fragments.sort_by(|a, b| {
        let a_top = a.bounding_box.map_or(f64::MIN, |bbox| bbox.y1);
        let b_top = b.bounding_box.map_or(f64::MIN, |bbox| bbox.y1);
        b_top.total_cmp(&a_top)
    });

    let mut output = Vec::with_capacity(fragments.len());
    let mut index = 0;
    while index < fragments.len() {
        let mut chain_end = index + 1;
        while chain_end < fragments.len()
            && chain_end - index < TABLE_STITCH_MAX_CHAIN_FRAGMENTS
            && fragments_are_stitchable(&fragments[chain_end - 1], &fragments[chain_end])
        {
            chain_end += 1;
        }

        if chain_end - index >= 2 {
            let chain = fragments[index..chain_end].to_vec();
            output.push(merge_table_chain(chain, all_page_segments));
        } else {
            output.push(fragments[index].clone());
        }
        index = chain_end;
    }
    output
}

/// Whether `next` is the vertically-adjacent continuation of `prev` within one
/// stitch chain: same page, same column count, near-zero row gap, and matching
/// left/right edges.
fn fragments_are_stitchable(prev: &crate::types::Table, next: &crate::types::Table) -> bool {
    if prev.page_number != next.page_number {
        return false;
    }
    let (Some(a), Some(b)) = (prev.bounding_box, next.bounding_box) else {
        return false;
    };

    let prev_cols = prev.cells.first().map_or(0, Vec::len);
    let next_cols = next.cells.first().map_or(0, Vec::len);
    if prev_cols == 0 || prev_cols != next_cols {
        return false;
    }

    (a.y0 - b.y1).abs() <= TABLE_STITCH_Y_GAP_TOLERANCE_PTS
        && (a.x0 - b.x0).abs() <= TABLE_STITCH_X_TOLERANCE_PTS
        && (a.x1 - b.x1).abs() <= TABLE_STITCH_X_TOLERANCE_PTS
}

/// Merge a chain of >= 2 stitchable fragments into one table.
///
/// The topmost fragment's rows collapse into the header; every other
/// fragment's rows collapse into one data row apiece. See
/// [`stitch_fragmented_tables`] for why a whole-fragment column merge is used
/// instead of trying to re-derive a header/data split.
fn merge_table_chain(chain: Vec<crate::types::Table>, all_page_segments: &[Vec<SegmentData>]) -> crate::types::Table {
    let column_count = chain
        .iter()
        .filter_map(|table| table.cells.first())
        .map(Vec::len)
        .max()
        .unwrap_or(0);

    let page_number = chain[0].page_number;
    let mut bbox = chain
        .iter()
        .find_map(|table| table.bounding_box)
        .unwrap_or(crate::types::BoundingBox {
            x0: 0.0,
            y0: 0.0,
            x1: 0.0,
            y1: 0.0,
        });
    for table in &chain {
        if let Some(b) = table.bounding_box {
            bbox.x0 = bbox.x0.min(b.x0);
            bbox.x1 = bbox.x1.max(b.x1);
            bbox.y0 = bbox.y0.min(b.y0);
            bbox.y1 = bbox.y1.max(b.y1);
        }
    }

    let mut rows: Vec<Vec<String>> = chain
        .iter()
        .map(|table| crate::pdf::table_reconstruct::merge_rows_columnwise(&table.cells, column_count))
        .collect();

    if let Some(page_segments) = all_page_segments.get((page_number.saturating_sub(1)) as usize) {
        recover_trailing_continuation_rows(&mut rows, &mut bbox, column_count, page_segments);
    }

    let markdown = crate::pdf::table_reconstruct::table_to_markdown(&rows);
    let columns = rows.first().cloned();
    crate::types::Table {
        cells: rows,
        markdown,
        page_number,
        bounding_box: Some(bbox),
        columns,
        ..Default::default()
    }
}

/// Recover trailing data rows that never became their own table fragment.
///
/// `native::table`'s region clustering sometimes merges the last entities of a
/// fragmented table into a region with unrelated following content (or drops
/// them entirely when the merged region fails `post_process_table`
/// validation), so those rows leak into the document as plain paragraph text
/// instead of table data. This scans the raw page segments strictly below the
/// stitched chain's known bottom edge, within its column span, and — bounded
/// by [`TABLE_STITCH_TRAILING_RECOVERY_MAX_ROWS`] iterations — pulls one
/// row-gap-bounded entity band at a time. A band is only accepted if
/// reconstructing it independently yields the same column count as the
/// stitched table; any mismatch (e.g. the band actually contains an unrelated
/// heading below the table) stops recovery immediately rather than skipping
/// past it, since skipping risks pulling in arbitrary downstream content.
fn recover_trailing_continuation_rows(
    rows: &mut Vec<Vec<String>>,
    bbox: &mut crate::types::BoundingBox,
    column_count: usize,
    page_segments: &[SegmentData],
) {
    if column_count == 0 || page_segments.is_empty() {
        return;
    }

    let page_height = page_segments
        .iter()
        .map(|s| s.y + s.height)
        .fold(0.0_f32, f32::max)
        .max(792.0);
    let x_lo = (bbox.x0 - TABLE_STITCH_X_TOLERANCE_PTS) as f32;
    let x_hi = (bbox.x1 + TABLE_STITCH_X_TOLERANCE_PTS) as f32;
    let mut search_floor = bbox.y0 as f32;

    for _ in 0..TABLE_STITCH_TRAILING_RECOVERY_MAX_ROWS {
        let band_words: Vec<crate::pdf::table_reconstruct::HocrWord> = page_segments
            .iter()
            .filter(|seg| {
                !seg.text.trim().is_empty()
                    && seg.y + seg.height <= search_floor + TABLE_STITCH_Y_GAP_TOLERANCE_PTS as f32
                    && seg.x + seg.width >= x_lo
                    && seg.x <= x_hi
            })
            .flat_map(|seg| crate::pdf::table_reconstruct::split_segment_to_words(seg, page_height))
            .collect();
        if band_words.is_empty() {
            break;
        }

        let Some((entity_words, entity_bottom_image_y)) = take_next_entity_band(&band_words) else {
            break;
        };

        let col_gap = super::regions::tables::compute_adaptive_column_gap(&entity_words, (x_hi - x_lo).max(1.0));
        let grid = crate::pdf::table_reconstruct::reconstruct_table(&entity_words, col_gap, 0.5);
        if grid.is_empty() || grid[0].len() != column_count {
            break;
        }

        let merged_row = crate::pdf::table_reconstruct::merge_rows_columnwise(&grid, column_count);
        if merged_row.iter().all(|cell| cell.trim().is_empty()) {
            break;
        }

        let entity_bottom_pdf_y = page_height - entity_bottom_image_y as f32;
        rows.push(merged_row);
        bbox.y0 = bbox.y0.min(entity_bottom_pdf_y as f64);
        search_floor = entity_bottom_pdf_y;
    }
}

/// Take the topmost row-gap-bounded contiguous band of words from `words`
/// (which may span more than one logical entity), stopping at the first gap
/// larger than `median_height * TABLE_STITCH_TRAILING_ROW_GAP_MULTIPLIER`.
///
/// Returns the band's words and the image-coordinate bottom edge (`top +
/// height`, max across the band) of the last line included.
fn take_next_entity_band(
    words: &[crate::pdf::table_reconstruct::HocrWord],
) -> Option<(Vec<crate::pdf::table_reconstruct::HocrWord>, u32)> {
    if words.is_empty() {
        return None;
    }

    let mut heights: Vec<u32> = words.iter().map(|w| w.height).collect();
    heights.sort_unstable();
    let median_height = heights[heights.len() / 2].max(1);
    let row_gap_split = (median_height as f32 * TABLE_STITCH_TRAILING_ROW_GAP_MULTIPLIER) as u32;
    let row_tolerance = (median_height / 2).max(3);

    let mut sorted: Vec<&crate::pdf::table_reconstruct::HocrWord> = words.iter().collect();
    sorted.sort_by_key(|w| w.top);

    let mut band: Vec<crate::pdf::table_reconstruct::HocrWord> = Vec::new();
    let mut band_bottom = 0u32;
    let mut last_row_yc: Option<u32> = None;
    let mut idx = 0;
    while idx < sorted.len() {
        let row_yc = sorted[idx].top + sorted[idx].height / 2;
        let mut end = idx + 1;
        while end < sorted.len() {
            let yc = sorted[end].top + sorted[end].height / 2;
            if yc.abs_diff(row_yc) <= row_tolerance {
                end += 1;
            } else {
                break;
            }
        }

        if let Some(prev_yc) = last_row_yc
            && row_yc > prev_yc
            && row_yc - prev_yc > row_gap_split
            && !band.is_empty()
        {
            break;
        }

        for word in &sorted[idx..end] {
            band_bottom = band_bottom.max(word.top + word.height);
            band.push((*word).clone());
        }
        last_row_yc = Some(row_yc);
        idx = end;
    }

    if band.is_empty() {
        None
    } else {
        Some((band, band_bottom))
    }
}

/// Select the exact tables that final assembly will emit.
///
/// Suppression must consume this same set so a duplicate or empty table cannot
/// remove source text without contributing a corresponding table element.
fn prepare_emitted_tables(
    native_tables: &[crate::types::Table],
    layout_tables: Vec<crate::types::Table>,
    overlap_preference: crate::core::config::layout::TableOverlapPreference,
) -> Vec<crate::types::Table> {
    let mut emitted_tables: Vec<crate::types::Table> = native_tables.iter().cloned().chain(layout_tables).collect();
    emitted_tables.retain(|table| !table.markdown.trim().is_empty());
    let native_count = native_tables
        .iter()
        .filter(|table| !table.markdown.trim().is_empty())
        .count();
    deduplicate_overlapping_tables(&mut emitted_tables, native_count, overlap_preference);
    normalize_sparse_currency_affix_columns(&mut emitted_tables);
    normalize_wrapped_financial_rows(&mut emitted_tables);
    deduplicate_identical_tables(&mut emitted_tables);
    assign_deterministic_table_ids(&mut emitted_tables);
    emitted_tables
}

const MAX_CURRENCY_AFFIX_CELLS: usize = 3;
const MAX_CURRENCY_AFFIX_OCCUPANCY: f64 = 0.1;
const MIN_FINANCIAL_TARGET_CELLS: usize = 5;
const MIN_FINANCIAL_TARGET_RATIO: f64 = 0.9;
const MIN_WRAPPED_FINANCIAL_VALUE_ROWS: usize = 8;
const FINANCIAL_COLUMN_HEADERS: &[&str] = &[
    "shares",
    "par",
    "principal",
    "quantity",
    "value",
    "market value",
    "cost",
];
const ISO_CURRENCY_CODES: &[&str] = &[
    "AUD", "BRL", "CAD", "CHF", "CNY", "DKK", "EUR", "GBP", "HKD", "INR", "JPY", "KRW", "MXN", "NOK", "NZD", "SEK",
    "SGD", "USD", "ZAR",
];

fn normalize_sparse_currency_affix_columns(tables: &mut [crate::types::Table]) {
    for table in tables {
        normalize_sparse_currency_affix_columns_in_table(table);
    }
}

fn normalize_wrapped_financial_rows(tables: &mut [crate::types::Table]) {
    for table in tables {
        if !is_wrapped_financial_table(&table.cells) {
            continue;
        }
        fold_wrapped_financial_rows(&mut table.cells);
        table.markdown = crate::extractors::frontmatter_utils::cells_to_markdown(&table.cells);
        table.columns = table.cells.first().cloned();
    }
}

fn is_wrapped_financial_table(rows: &[Vec<String>]) -> bool {
    let Some(header) = rows.first() else {
        return false;
    };
    if header.len() < 3
        || rows.len() <= 1
        || !header.iter().skip(1).all(|cell| is_financial_column_header(cell))
        || rows.iter().any(|row| row.len() != header.len())
    {
        return false;
    }

    let body = &rows[1..];
    let value_rows = body.iter().filter(|row| is_value_bearing_financial_row(row)).count();
    let descriptor_rows = body
        .iter()
        .filter(|row| is_descriptor_only_financial_row(row) && !is_financial_section_label(row))
        .count();
    value_rows >= MIN_WRAPPED_FINANCIAL_VALUE_ROWS
        && descriptor_rows > value_rows
        && body
            .iter()
            .all(|row| is_descriptor_only_financial_row(row) || is_value_bearing_financial_row(row))
}

fn is_descriptor_only_financial_row(row: &[String]) -> bool {
    row.first().is_some_and(|cell| !cell.trim().is_empty()) && row.iter().skip(1).all(|cell| cell.trim().is_empty())
}

fn is_value_bearing_financial_row(row: &[String]) -> bool {
    row.first().is_some_and(|cell| !cell.trim().is_empty()) && row.iter().skip(1).all(|cell| is_financial_value(cell))
}

fn is_financial_section_label(row: &[String]) -> bool {
    if !is_descriptor_only_financial_row(row) {
        return false;
    }
    let text = row[0].trim().to_ascii_lowercase();
    text.contains("(continued)") || has_allocation_percentage_suffix(&text)
}

fn has_allocation_percentage_suffix(text: &str) -> bool {
    const MAX_FOOTNOTE_MARKER_CHARS: usize = 4;

    let Some((dash_index, dash)) = text
        .char_indices()
        .rev()
        .find(|(_, character)| matches!(character, '–' | '—'))
    else {
        return false;
    };
    if text[..dash_index].trim().is_empty() {
        return false;
    }
    let suffix = text[dash_index + dash.len_utf8()..].trim();
    let Some((allocation, remainder)) = suffix.split_once('%') else {
        return false;
    };
    let Ok(allocation) = allocation.trim().parse::<f64>() else {
        return false;
    };
    if !allocation.is_finite() || !(0.0..=100.0).contains(&allocation) {
        return false;
    }

    let remainder = remainder.trim();
    if remainder.is_empty() {
        return true;
    }
    let Some(marker) = remainder.strip_prefix('(').and_then(|value| value.strip_suffix(')')) else {
        return false;
    };
    !marker.is_empty()
        && marker.chars().count() <= MAX_FOOTNOTE_MARKER_CHARS
        && marker.chars().all(char::is_alphanumeric)
}

fn is_financial_value(cell: &str) -> bool {
    let trimmed = cell.trim();
    if is_financial_number(trimmed) {
        return true;
    }
    trimmed
        .split_once(' ')
        .is_some_and(|(marker, value)| is_currency_marker(marker) && is_financial_number(value.trim()))
}

fn fold_wrapped_financial_rows(rows: &mut Vec<Vec<String>>) {
    let mut folded = Vec::with_capacity(rows.len());
    folded.extend(rows.first().cloned());
    let mut pending = Vec::new();
    for mut row in rows.iter().skip(1).cloned() {
        if is_financial_section_label(&row) {
            folded.append(&mut pending);
            folded.push(row);
            continue;
        }
        if is_descriptor_only_financial_row(&row) {
            pending.push(row);
            continue;
        }
        if !pending.is_empty() {
            let prefix = pending
                .drain(..)
                .filter_map(|pending_row| pending_row.into_iter().next())
                .collect::<Vec<_>>()
                .join(" ");
            row[0] = format!("{prefix} {}", row[0]);
        }
        folded.push(row);
    }
    folded.extend(pending);
    *rows = folded;
}

fn normalize_sparse_currency_affix_columns_in_table(table: &mut crate::types::Table) {
    let Some(header) = table.cells.first() else {
        return;
    };
    if table.cells.len() <= 1 || header.len() < 2 {
        return;
    }

    let mut source_columns = (0..header.len() - 1)
        .filter(|&source| {
            header[source].trim().is_empty()
                && header
                    .get(source + 1)
                    .is_some_and(|target| is_financial_column_header(target))
                && is_sparse_currency_affix_column(&table.cells[1..], source, source + 1)
        })
        .collect::<Vec<_>>();
    source_columns.sort_unstable_by(|left, right| right.cmp(left));
    let normalized = !source_columns.is_empty();

    for source in source_columns {
        merge_currency_affix_column(&mut table.cells, source);
    }
    if normalized {
        table.markdown = crate::extractors::frontmatter_utils::cells_to_markdown(&table.cells);
        table.columns = table.cells.first().cloned();
    }
}

fn is_financial_column_header(header: &str) -> bool {
    let normalized = header.trim().to_ascii_lowercase();
    FINANCIAL_COLUMN_HEADERS
        .iter()
        .any(|candidate| normalized == *candidate || normalized.starts_with(&format!("{candidate} ")))
}

fn is_sparse_currency_affix_column(rows: &[Vec<String>], source: usize, target: usize) -> bool {
    let source_cells = rows
        .iter()
        .filter_map(|row| row.get(source))
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>();
    let occupied_limit =
        MAX_CURRENCY_AFFIX_CELLS.min(((rows.len() as f64) * MAX_CURRENCY_AFFIX_OCCUPANCY).ceil() as usize);
    if source_cells.is_empty()
        || source_cells.len() > occupied_limit
        || !source_cells.iter().all(|cell| is_currency_marker(cell))
        || rows.iter().any(|row| {
            row.get(source).is_some_and(|cell| !cell.trim().is_empty())
                && row.get(target).is_none_or(|cell| cell.trim().is_empty())
        })
    {
        return false;
    }

    let target_cells = rows
        .iter()
        .filter_map(|row| row.get(target))
        .map(|cell| cell.trim())
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>();
    target_cells.len() >= MIN_FINANCIAL_TARGET_CELLS
        && target_cells.iter().filter(|cell| is_financial_number(cell)).count() as f64 / target_cells.len() as f64
            >= MIN_FINANCIAL_TARGET_RATIO
}

fn is_currency_marker(cell: &str) -> bool {
    const CURRENCY_SYMBOLS: &[&str] = &["$", "€", "£", "¥", "₹", "₩", "₽", "₺", "₪", "₫", "₦", "₱", "฿"];
    CURRENCY_SYMBOLS.contains(&cell) || ISO_CURRENCY_CODES.contains(&cell.to_ascii_uppercase().as_str())
}

fn is_financial_number(cell: &str) -> bool {
    let mut has_digit = false;
    cell.chars().all(|character| {
        if character.is_ascii_digit() {
            has_digit = true;
            true
        } else {
            character.is_ascii_whitespace() || matches!(character, ',' | '.' | '-' | '+' | '(' | ')' | '%')
        }
    }) && has_digit
}

fn merge_currency_affix_column(rows: &mut [Vec<String>], source: usize) {
    for row in rows {
        if row.len() <= source {
            continue;
        }
        if let Some(marker) = row.get(source).map(|cell| cell.trim()).filter(|cell| !cell.is_empty())
            && let Some(value) = row
                .get(source + 1)
                .map(|cell| cell.trim())
                .filter(|cell| !cell.is_empty())
        {
            row[source + 1] = format!("{marker} {value}");
        }
        row.remove(source);
    }
}

/// Collapse byte-identical table duplicates on the same page.
///
/// [`deduplicate_overlapping_tables`] only merges a pair when both tables carry
/// a `bounding_box`; a native/layout pair that detects the same physical table
/// but disagrees on bbox presence (e.g. native reconstruction leaves
/// `bounding_box: None` for some heuristic grids) can otherwise escape that
/// pass entirely. This pass is origin- and bbox-agnostic: any two tables on the
/// same page with byte-identical markdown are the same table by definition, so
/// the second (and any further) occurrence is dropped regardless of bbox state.
///
/// Runs in `O(n)` over the page's table count using a hash set keyed on
/// `(page_number, markdown)`.
fn deduplicate_identical_tables(tables: &mut Vec<crate::types::Table>) {
    if tables.len() < 2 {
        return;
    }

    let mut seen: ahash::AHashSet<(u32, &str)> = ahash::AHashSet::with_capacity(tables.len());
    let mut keep = vec![true; tables.len()];
    for (index, table) in tables.iter().enumerate() {
        if !seen.insert((table.page_number, table.markdown.as_str())) {
            keep[index] = false;
        }
    }

    let mut index = 0;
    tables.retain(|_| {
        let keep_this = keep[index];
        index += 1;
        keep_this
    });
}

/// A table's footprint on a page together with the text its grid actually carries.
///
/// The two are recorded side by side because suppression needs both: geometry alone
/// cannot tell whether the grid REPRESENTS a run it happens to cover. See
/// [`filter_segments_by_table_bboxes`]. ~keep
#[derive(Clone)]
struct TableCoverage {
    bbox: crate::types::BoundingBox,
    /// Every cell's alphanumeric glyphs, lowercased and concatenated in row-major order.
    /// Built once per table so the per-segment test is a substring search.
    cell_text: String,
    /// One entry per table row: that row's cells' glyphs, concatenated.
    rows: Vec<String>,
    /// Every whitespace-separated word of every cell, normalized.
    words: ahash::AHashSet<String>,
}

/// Shortest normalized run that may match across cell boundaries within one row. A shorter
/// run (an item number, a price) is only ever carried by a whole cell word: its digits also
/// occur, by accident, across the seam of two neighbouring numeric cells (#2171). ~keep
const MIN_CROSS_CELL_RUN_GLYPHS: usize = 8;

impl TableCoverage {
    fn from_rows<'a>(bbox: crate::types::BoundingBox, rows: impl IntoIterator<Item = Vec<&'a str>>) -> Self {
        let mut row_texts = Vec::new();
        let mut words = ahash::AHashSet::new();
        for row in rows {
            row_texts.push(
                row.iter()
                    .map(|cell| normalize_for_table_coverage(cell))
                    .collect::<String>(),
            );
            for cell in row {
                words.extend(
                    cell.split_whitespace()
                        .map(normalize_for_table_coverage)
                        .filter(|word| !word.is_empty()),
                );
            }
        }
        Self {
            bbox,
            cell_text: row_texts.concat(),
            rows: row_texts,
            words,
        }
    }

    /// Whether the grid represents `text`: every word is a whole cell word, or a long run
    /// sits inside one row's cells. A run is not matched against the whole table's
    /// concatenated glyphs, which lets digits straddling unrelated cells stand in for it.
    fn carries(&self, text: &str) -> bool {
        let normalized = normalize_for_table_coverage(text);
        if normalized.is_empty() {
            return true;
        }
        text.split_whitespace()
            .map(normalize_for_table_coverage)
            .filter(|word| !word.is_empty())
            .all(|word| self.words.contains(&word))
            || (normalized.len() >= MIN_CROSS_CELL_RUN_GLYPHS && self.rows.iter().any(|row| row.contains(&normalized)))
    }
}

/// Reduce text to lowercase alphanumerics, dropping whitespace and punctuation entirely.
///
/// Cell assembly does not preserve a printed run's boundaries: one visual line commonly spans
/// several cells, and a wrapped cell inserts separators a printed run does not have. GH#1616's
/// first fix compared whitespace-collapsed text, so any run the grid split across cells failed to
/// match and was emitted a second time as prose -- on `issue-912` that cost 81 of 263 words,
/// dropping precision from 0.984 to 0.692 while recall stayed flat, which is the signature of
/// duplication rather than loss. Concatenating glyphs makes the test indifferent to where the grid
/// chose to put its boundaries, which is the only thing it was ever wrong about. ~keep
fn normalize_for_table_coverage(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn table_bboxes_by_page(tables: &[crate::types::Table]) -> ahash::AHashMap<usize, Vec<TableCoverage>> {
    let mut coverage_by_page: ahash::AHashMap<usize, Vec<TableCoverage>> = ahash::AHashMap::new();
    for table in tables {
        if let Some(bbox) = table.bounding_box {
            // `cells` is authoritative when populated, but a table can reach here
            // carrying only rendered `markdown` (layout-sourced tables, and the
            // overlap-preference merge, both produce that shape). Falling back to the
            // markdown keeps suppression working for those instead of silently
            // disabling it, which would emit their contents twice. ~keep
            let coverage = if table.cells.iter().any(|row| !row.is_empty()) {
                TableCoverage::from_rows(
                    bbox,
                    table.cells.iter().map(|row| row.iter().map(String::as_str).collect()),
                )
            } else {
                TableCoverage::from_rows(
                    bbox,
                    table
                        .markdown
                        .lines()
                        .map(|line| line.split('|').collect::<Vec<&str>>()),
                )
            };
            coverage_by_page
                .entry(table.page_number.saturating_sub(1) as usize)
                .or_default()
                .push(coverage);
        }
    }
    coverage_by_page
}

/// Filter out segments a table both COVERS and CARRIES.
///
/// Suppression exists so text a table already renders is not emitted a second time as
/// prose. Geometry alone was the whole test, and that is unsound: a reconstructed grid
/// need not span every printed column inside its own bounding box, and the runs in the
/// columns it left out were dropped from the prose flow without ever reaching a cell.
/// They were deleted from the document -- not in a cell, not in an element, nowhere.
/// Measured on GH#1616: a four-column fault-finding grid was reconstructed with two
/// columns over a bbox spanning all four, and 26 words vanished across two pages.
///
/// The text test restores the invariant that a bounding box cannot delete content the
/// grid does not represent: a covered run is dropped only when some cell actually
/// carries it. Matching is on [`normalize_for_table_coverage`]'s glyph concatenation, so
/// it is indifferent to where cell assembly put its boundaries -- a printed run split
/// across two cells, or a visual line spanning several, still matches. Requiring the
/// grid's own whitespace instead suppressed almost nothing and re-emitted whole tables
/// as prose (GH#1616 again, from the other side).
///
/// Segments with zero area or empty text are always kept. ~keep
fn filter_segments_by_table_bboxes(segments: Vec<SegmentData>, tables: &[TableCoverage]) -> Vec<SegmentData> {
    if tables.is_empty() {
        return segments;
    }
    segments
        .into_iter()
        .filter(|seg| {
            let seg_area = seg.width * seg.height;
            let seg_text = seg.text.trim();
            if seg_area <= 0.0 || seg_text.is_empty() {
                return true;
            }
            !tables.iter().any(|table| {
                let bb = &table.bbox;
                let inter_left = seg.x.max(bb.x0 as f32);
                let inter_right = (seg.x + seg.width).min(bb.x1 as f32);
                let inter_bottom = seg.y.max(bb.y0 as f32);
                let inter_top = (seg.y + seg.height).min(bb.y1 as f32);
                if inter_left >= inter_right || inter_bottom >= inter_top {
                    return false;
                }
                let inter_area = (inter_right - inter_left) * (inter_top - inter_bottom);
                inter_area / seg_area >= 0.5 && table.carries(seg_text)
            })
        })
        .collect()
}

/// Apply all 5 text repair passes in a single traversal over a segment's text.
///
/// Returns `Cow::Borrowed` if nothing changed, `Cow::Owned` otherwise.
fn fused_text_repairs<'a>(text: &'a str, word_witnesses: &WordWitnesses) -> Cow<'a, str> {
    let t1 = normalize_text_encoding(text);
    let t2 = repair_ligature_spaces(&t1, word_witnesses);
    let t3 = expand_ligatures_with_space_absorption(&t2);
    let t3b = collapse_spaced_hyphens(&t3);
    let t4 = normalize_unicode_text(&t3b);
    let t5 = clean_duplicate_punctuation(&t4);
    match (&t1, &t2, &t3, &t3b, &t4, &t5) {
        (
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
            Cow::Borrowed(_),
        ) => Cow::Borrowed(text),
        _ => Cow::Owned(t5.into_owned()),
    }
}

/// Deduplicate tables that overlap on the same page.
///
/// When both native detection and layout-based table extraction produce tables
/// for the same region, they can overlap. Tables at index `< native_count` are native;
/// the rest are layout (TATR/SLANeXT) tables. Complete side-by-side layout replacements
/// are selected atomically before ordinary pairwise arbitration. Outside those replacements,
/// `preference` decides mixed native/layout overlaps, while content weight decides same-origin
/// overlaps and [`TableOverlapPreference::Content`].
fn deduplicate_overlapping_tables(
    tables: &mut Vec<crate::types::Table>,
    native_count: usize,
    preference: crate::core::config::layout::TableOverlapPreference,
) {
    use crate::core::config::layout::TableOverlapPreference;

    if tables.len() < 2 {
        return;
    }

    let mut to_remove = ahash::AHashSet::new();
    let mut protected_layout_children = ahash::AHashSet::new();

    if preference != TableOverlapPreference::Native {
        // A complete split cohort is one structural alternative to its native source
        // cohort. Select it atomically for every non-Native preference: pairwise
        // content weighting could otherwise mix incompatible rows from both grids.
        for (parents, children) in side_by_side_layout_replacements(tables, native_count) {
            protected_layout_children.extend(children);
            to_remove.extend(parents);
        }
    }

    for i in 0..tables.len() {
        if to_remove.contains(&i) {
            continue;
        }
        for j in (i + 1)..tables.len() {
            if to_remove.contains(&j) {
                continue;
            }
            if tables[i].page_number != tables[j].page_number {
                continue;
            }
            if let (Some(a), Some(b)) = (&tables[i].bounding_box, &tables[j].bounding_box) {
                let inter_x = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0);
                let inter_y = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
                let intersection = inter_x * inter_y;
                let area_a = (a.x1 - a.x0) * (a.y1 - a.y0);
                let area_b = (b.x1 - b.x0) * (b.y1 - b.y0);
                let min_area = area_a.min(area_b);

                if min_area > 0.0 && intersection / min_area > 0.5 {
                    let i_is_native = i < native_count;
                    let j_is_native = j < native_count;
                    let mixed_origin = i_is_native != j_is_native;
                    let i_is_protected = protected_layout_children.contains(&i);
                    let j_is_protected = protected_layout_children.contains(&j);
                    let remove = match (i_is_protected, j_is_protected) {
                        (true, false) => Some(j),
                        (false, true) => Some(i),
                        (true, true) => {
                            let duplicate = intersection / area_a >= LAYOUT_CHILD_DUPLICATE_OVERLAP
                                && intersection / area_b >= LAYOUT_CHILD_DUPLICATE_OVERLAP;
                            duplicate.then(|| lower_content_table(tables, i, j))
                        }
                        (false, false) => Some(match preference {
                            TableOverlapPreference::Native if mixed_origin => {
                                if i_is_native {
                                    j
                                } else {
                                    i
                                }
                            }
                            TableOverlapPreference::Layout if mixed_origin => {
                                if i_is_native {
                                    i
                                } else {
                                    j
                                }
                            }
                            _ => lower_content_table(tables, i, j),
                        }),
                    };
                    let Some(remove) = remove else {
                        continue;
                    };
                    to_remove.insert(remove);
                    if remove == i {
                        break;
                    }
                }
            }
        }
    }

    let surviving_protected: Vec<_> = protected_layout_children
        .iter()
        .copied()
        .filter(|index| !to_remove.contains(index))
        .collect();
    let affected_rows: Vec<_> = surviving_protected
        .iter()
        .filter_map(|&index| tables[index].bounding_box.map(|bbox| (tables[index].page_number, bbox)))
        .collect();

    let mut idx = 0;
    tables.retain(|_| {
        let keep = !to_remove.contains(&idx);
        idx += 1;
        keep
    });
    canonicalize_affected_table_rows(tables, affected_rows);
}

/// Required containment for a layout child and, in one-to-one cohort matching,
/// reciprocal coverage of its corresponding native parent.
const SIDE_BY_SIDE_CHILD_PARENT_OVERLAP: f64 = 0.8;
/// Crop jitter tolerance for reciprocal one-to-one cohort correspondence.
const SIDE_BY_SIDE_CORRESPONDENCE_EPSILON: f64 = 0.005;
/// Both children must describe the same row band, rather than stacked tables.
const SIDE_BY_SIDE_VERTICAL_OVERLAP: f64 = 0.6;
/// The children together must account for most of the parent's horizontal span.
const SIDE_BY_SIDE_PARENT_WIDTH_COVERAGE: f64 = 0.75;
/// Every child must span most of the parent's height, rejecting shallow row fragments.
const SIDE_BY_SIDE_PARENT_HEIGHT_COVERAGE: f64 = 0.75;
/// Disjoint children must jointly explain most of the parent's total area.
const SIDE_BY_SIDE_PARENT_AREA_COVERAGE: f64 = 0.65;
/// Candidate detections that mutually cover nearly all of one another represent
/// the same layout table rather than distinct parts of a split table.
const LAYOUT_CHILD_DUPLICATE_OVERLAP: f64 = 0.9;

fn side_by_side_layout_replacements(
    tables: &[crate::types::Table],
    native_count: usize,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let native_count = native_count.min(tables.len());
    let rows = native_candidate_rows(tables, native_count);
    let mut used_native = ahash::AHashSet::new();
    let mut used_layout = ahash::AHashSet::new();
    let mut replacements = Vec::new();
    for parent in rows.iter().flatten().copied() {
        let parent_bbox = tables[parent].bounding_box.as_ref().expect("candidate has bbox");
        let mut children = layout_children_for_parent(tables, native_count, tables[parent].page_number, parent_bbox);
        children.retain(|child| !used_layout.contains(child));
        if !used_native.contains(&parent) && is_side_by_side_replacement(tables, parent_bbox, &children) {
            used_native.insert(parent);
            used_layout.extend(children.iter().copied());
            replacements.push((vec![parent], children));
        }
    }
    replacements.extend(side_by_side_native_cohort_replacements(
        tables,
        native_count,
        &rows,
        &mut used_native,
        &mut used_layout,
    ));
    replacements
}

fn side_by_side_native_cohort_replacements(
    tables: &[crate::types::Table],
    native_count: usize,
    rows: &[Vec<usize>],
    used_native: &mut ahash::AHashSet<usize>,
    used_layout: &mut ahash::AHashSet<usize>,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut replacements = Vec::new();
    for pair in rows.iter().flat_map(|row| row.windows(2)) {
        let parents = vec![pair[0], pair[1]];
        if parents.iter().any(|parent| used_native.contains(parent)) {
            continue;
        }
        let Some(parent_bbox) = table_union_bbox(&tables[parents[0]], &tables[parents[1]]) else {
            continue;
        };
        if !is_side_by_side_replacement(tables, &parent_bbox, &parents) {
            continue;
        }
        let mut children =
            layout_children_for_parent(tables, native_count, tables[parents[0]].page_number, &parent_bbox);
        children.retain(|child| !used_layout.contains(child));
        if children.len() != 2
            || !is_side_by_side_replacement(tables, &parent_bbox, &children)
            || !replacement_children_correspond(tables, &parents, &children)
        {
            continue;
        }
        used_native.extend(parents.iter().copied());
        used_layout.extend(children.iter().copied());
        replacements.push((parents, children));
    }
    replacements
}

fn native_candidate_rows(tables: &[crate::types::Table], native_count: usize) -> Vec<Vec<usize>> {
    let mut candidates: Vec<_> = (0..native_count)
        .filter(|&index| tables[index].bounding_box.is_some())
        .collect();
    candidates.sort_by(|&left, &right| {
        tables[left]
            .page_number
            .cmp(&tables[right].page_number)
            .then_with(|| table_top(tables, right).total_cmp(&table_top(tables, left)))
            .then_with(|| table_left(tables, left).total_cmp(&table_left(tables, right)))
    });
    let mut rows: Vec<Vec<usize>> = Vec::new();
    for candidate in candidates {
        let joins_last_row = rows.last().is_some_and(|row| {
            tables[row[0]].page_number == tables[candidate].page_number
                && vertical_overlap_fraction(
                    tables[row[0]].bounding_box.as_ref().expect("candidate has bbox"),
                    tables[candidate].bounding_box.as_ref().expect("candidate has bbox"),
                ) >= SIDE_BY_SIDE_VERTICAL_OVERLAP
        });
        if joins_last_row {
            rows.last_mut().expect("row exists").push(candidate);
        } else {
            rows.push(vec![candidate]);
        }
    }
    for row in &mut rows {
        row.sort_by(|&left, &right| table_left(tables, left).total_cmp(&table_left(tables, right)));
    }
    rows
}

fn replacement_children_correspond(tables: &[crate::types::Table], parents: &[usize], children: &[usize]) -> bool {
    parents
        .iter()
        .zip(children)
        .enumerate()
        .all(|(position, (&parent, &child))| {
            let parent_bbox = tables[parent].bounding_box.as_ref().expect("candidate has bbox");
            let child_bbox = tables[child].bounding_box.as_ref().expect("candidate has bbox");
            let paired_intersection = bbox_intersection_area(parent_bbox, child_bbox);
            let sibling_intersection = children
                .get(1 - position)
                .and_then(|&sibling| tables[sibling].bounding_box.as_ref())
                .map_or(0.0, |sibling_bbox| bbox_intersection_area(parent_bbox, sibling_bbox));
            bbox_center_x(child_bbox) >= parent_bbox.x0
                && bbox_center_x(child_bbox) <= parent_bbox.x1
                && paired_intersection > sibling_intersection
                && bbox_overlap_fraction(child_bbox, parent_bbox) + SIDE_BY_SIDE_CORRESPONDENCE_EPSILON
                    >= SIDE_BY_SIDE_CHILD_PARENT_OVERLAP
                && bbox_overlap_fraction(parent_bbox, child_bbox) + SIDE_BY_SIDE_CORRESPONDENCE_EPSILON
                    >= SIDE_BY_SIDE_CHILD_PARENT_OVERLAP
        })
}

fn bbox_center_x(bbox: &crate::types::BoundingBox) -> f64 {
    (bbox.x0 + bbox.x1) / 2.0
}

fn table_top(tables: &[crate::types::Table], index: usize) -> f64 {
    tables[index]
        .bounding_box
        .as_ref()
        .map_or(f64::NEG_INFINITY, |bbox| bbox.y1)
}

fn layout_children_for_parent(
    tables: &[crate::types::Table],
    native_count: usize,
    page_number: u32,
    parent_bbox: &crate::types::BoundingBox,
) -> Vec<usize> {
    let children = (native_count..tables.len())
        .filter(|&child| {
            tables[child].page_number == page_number
                && tables[child]
                    .bounding_box
                    .as_ref()
                    .is_some_and(|bbox| bbox_overlap_fraction(bbox, parent_bbox) >= SIDE_BY_SIDE_CHILD_PARENT_OVERLAP)
        })
        .collect();
    deduplicate_layout_candidates(tables, children)
}

fn table_union_bbox(left: &crate::types::Table, right: &crate::types::Table) -> Option<crate::types::BoundingBox> {
    let left = left.bounding_box.as_ref()?;
    let right = right.bounding_box.as_ref()?;
    Some(crate::types::BoundingBox {
        x0: left.x0.min(right.x0),
        y0: left.y0.min(right.y0),
        x1: left.x1.max(right.x1),
        y1: left.y1.max(right.y1),
    })
}

fn deduplicate_layout_candidates(tables: &[crate::types::Table], mut candidates: Vec<usize>) -> Vec<usize> {
    candidates.sort_by(|&left, &right| table_left(tables, left).total_cmp(&table_left(tables, right)));
    let mut unique: Vec<usize> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let duplicate = unique.iter().position(|&existing| {
            let candidate_bbox = tables[candidate].bounding_box.as_ref().expect("candidate has bbox");
            let existing_bbox = tables[existing].bounding_box.as_ref().expect("candidate has bbox");
            bbox_overlap_fraction(candidate_bbox, existing_bbox) >= LAYOUT_CHILD_DUPLICATE_OVERLAP
                && bbox_overlap_fraction(existing_bbox, candidate_bbox) >= LAYOUT_CHILD_DUPLICATE_OVERLAP
        });
        if let Some(position) = duplicate {
            let existing = unique[position];
            if table_content_weight(&tables[candidate]) > table_content_weight(&tables[existing]) {
                unique[position] = candidate;
            }
        } else {
            unique.push(candidate);
        }
    }
    unique.sort_by(|&left, &right| table_left(tables, left).total_cmp(&table_left(tables, right)));
    unique
}

fn lower_content_table(tables: &[crate::types::Table], left: usize, right: usize) -> usize {
    if table_content_weight(&tables[left]) >= table_content_weight(&tables[right]) {
        right
    } else {
        left
    }
}

fn table_content_weight(table: &crate::types::Table) -> usize {
    table.cells.len() + table.markdown.len()
}

fn is_side_by_side_replacement(
    tables: &[crate::types::Table],
    parent: &crate::types::BoundingBox,
    children: &[usize],
) -> bool {
    let parent_width = parent.x1 - parent.x0;
    let parent_height = parent.y1 - parent.y0;
    if children.len() < 2 || parent_width <= 0.0 || parent_height <= 0.0 {
        return false;
    }
    let horizontally_disjoint = children.windows(2).all(|pair| {
        let left = tables[pair[0]].bounding_box.as_ref().expect("candidate has bbox");
        let right = tables[pair[1]].bounding_box.as_ref().expect("candidate has bbox");
        left.x1 <= right.x0 && vertical_overlap_fraction(left, right) >= SIDE_BY_SIDE_VERTICAL_OVERLAP
    });
    if !horizontally_disjoint {
        return false;
    }
    let covers_parent_height = children.iter().all(|&index| {
        let bbox = tables[index].bounding_box.as_ref().expect("candidate has bbox");
        let covered_height = (bbox.y1.min(parent.y1) - bbox.y0.max(parent.y0)).max(0.0);
        covered_height / parent_height >= SIDE_BY_SIDE_PARENT_HEIGHT_COVERAGE
    });
    if !covers_parent_height {
        return false;
    }
    let covered_width: f64 = children
        .iter()
        .map(|&index| {
            let bbox = tables[index].bounding_box.as_ref().expect("candidate has bbox");
            (bbox.x1.min(parent.x1) - bbox.x0.max(parent.x0)).max(0.0)
        })
        .sum();
    let covered_area: f64 = children
        .iter()
        .map(|&index| bbox_intersection_area(tables[index].bounding_box.as_ref().expect("candidate has bbox"), parent))
        .sum();
    covered_width / parent_width >= SIDE_BY_SIDE_PARENT_WIDTH_COVERAGE
        && covered_area / (parent_width * parent_height) >= SIDE_BY_SIDE_PARENT_AREA_COVERAGE
}

fn bbox_overlap_fraction(child: &crate::types::BoundingBox, parent: &crate::types::BoundingBox) -> f64 {
    let child_area = (child.x1 - child.x0).max(0.0) * (child.y1 - child.y0).max(0.0);
    if child_area == 0.0 {
        return 0.0;
    }
    bbox_intersection_area(child, parent) / child_area
}

fn bbox_intersection_area(a: &crate::types::BoundingBox, b: &crate::types::BoundingBox) -> f64 {
    let intersection_width = (a.x1.min(b.x1) - a.x0.max(b.x0)).max(0.0);
    let intersection_height = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
    intersection_width * intersection_height
}

fn vertical_overlap_fraction(a: &crate::types::BoundingBox, b: &crate::types::BoundingBox) -> f64 {
    let overlap = (a.y1.min(b.y1) - a.y0.max(b.y0)).max(0.0);
    let min_height = (a.y1 - a.y0).min(b.y1 - b.y0);
    if min_height <= 0.0 { 0.0 } else { overlap / min_height }
}

fn table_left(tables: &[crate::types::Table], index: usize) -> f64 {
    tables[index]
        .bounding_box
        .as_ref()
        .map_or(f64::INFINITY, |bbox| bbox.x0)
}

fn canonicalize_affected_table_rows(
    tables: &mut Vec<crate::types::Table>,
    affected_rows: Vec<(u32, crate::types::BoundingBox)>,
) {
    let cohorts = affected_row_cohorts(affected_rows);
    if cohorts.is_empty() {
        return;
    }
    let assignments: Vec<_> = tables.iter().map(|table| table_row_cohort(table, &cohorts)).collect();
    let mut cohort_tables: Vec<Vec<_>> = (0..cohorts.len()).map(|_| Vec::new()).collect();
    let mut source: Vec<Option<_>> = std::mem::take(tables).into_iter().map(Some).collect();
    for (index, cohort) in assignments.iter().enumerate() {
        if let Some(cohort) = cohort {
            cohort_tables[*cohort].push(source[index].take().expect("assigned table is present"));
        }
    }
    for cohort in &mut cohort_tables {
        cohort.sort_by(canonical_table_order);
        cohort.reverse();
    }
    for (index, cohort) in assignments.iter().enumerate() {
        if let Some(cohort) = cohort {
            source[index] = Some(
                cohort_tables[*cohort]
                    .pop()
                    .expect("cohort table count matches assigned slots"),
            );
        }
    }
    tables.extend(source.into_iter().flatten());
}

fn affected_row_cohorts(mut rows: Vec<(u32, crate::types::BoundingBox)>) -> Vec<(u32, Vec<crate::types::BoundingBox>)> {
    let mut cohorts = Vec::new();
    while let Some((page, seed)) = rows.pop() {
        let mut cohort = vec![seed];
        let mut changed = true;
        while changed {
            changed = false;
            rows.retain(|(candidate_page, candidate)| {
                let connected = *candidate_page == page
                    && cohort
                        .iter()
                        .any(|member| vertical_overlap_fraction(member, candidate) >= SIDE_BY_SIDE_VERTICAL_OVERLAP);
                if connected {
                    cohort.push(*candidate);
                    changed = true;
                }
                !connected
            });
        }
        cohorts.push((page, cohort));
    }
    cohorts.sort_by(|(left_page, left_rows), (right_page, right_rows)| {
        left_page.cmp(right_page).then_with(|| {
            let left_y = left_rows.iter().map(|row| row.y0).fold(f64::INFINITY, f64::min);
            let right_y = right_rows.iter().map(|row| row.y0).fold(f64::INFINITY, f64::min);
            left_y.total_cmp(&right_y)
        })
    });
    cohorts
}

fn table_row_cohort(table: &crate::types::Table, cohorts: &[(u32, Vec<crate::types::BoundingBox>)]) -> Option<usize> {
    let bbox = table.bounding_box.as_ref()?;
    cohorts
        .iter()
        .enumerate()
        .filter(|(_, (page, _))| *page == table.page_number)
        .map(|(index, (_, rows))| {
            let overlap = rows
                .iter()
                .map(|row| vertical_overlap_fraction(bbox, row))
                .fold(0.0_f64, f64::max);
            (index, overlap)
        })
        .filter(|(_, overlap)| *overlap >= SIDE_BY_SIDE_VERTICAL_OVERLAP)
        .max_by(|left, right| left.1.total_cmp(&right.1).then_with(|| right.0.cmp(&left.0)))
        .map(|(index, _)| index)
}

fn canonical_table_order(left: &crate::types::Table, right: &crate::types::Table) -> std::cmp::Ordering {
    let left_bbox = left.bounding_box.as_ref();
    let right_bbox = right.bounding_box.as_ref();
    left.page_number
        .cmp(&right.page_number)
        .then_with(|| {
            left_bbox
                .map_or(f64::INFINITY, |bbox| bbox.y0)
                .total_cmp(&right_bbox.map_or(f64::INFINITY, |bbox| bbox.y0))
        })
        .then_with(|| {
            left_bbox
                .map_or(f64::INFINITY, |bbox| bbox.x0)
                .total_cmp(&right_bbox.map_or(f64::INFINITY, |bbox| bbox.x0))
        })
        .then_with(|| left.markdown.cmp(&right.markdown))
}

/// Clear `is_page_furniture` on paragraphs whose `layout_class` was set to
/// `PageHeader`, `PageFooter`, or `Footnote` by the layout model, when the
/// caller has opted in to keeping those regions via `include_headers` /
/// `include_footers` / `include_footnotes`.
///
/// This must run **before** `retain_page_furniture_safely`, which physically
/// removes furniture paragraphs via `.retain()`. Un-marking here ensures that
/// user-opted-in header/footer/footnote paragraphs survive that pass.
pub(crate) fn un_mark_layout_furniture_per_config(
    paragraphs: &mut [PdfParagraph],
    include_headers: bool,
    include_footers: bool,
    include_footnotes: bool,
) {
    if !include_headers && !include_footers && !include_footnotes {
        return;
    }
    for para in paragraphs.iter_mut() {
        if !para.is_page_furniture {
            continue;
        }
        match para.layout_class {
            Some(super::types::LayoutHintClass::PageHeader) if include_headers => {
                para.is_page_furniture = false;
            }
            Some(super::types::LayoutHintClass::PageFooter) if include_footers => {
                para.is_page_furniture = false;
            }
            Some(super::types::LayoutHintClass::Footnote) if include_footnotes => {
                para.is_page_furniture = false;
            }
            _ => {}
        }
    }
}

const FOOTNOTE_MARKER_MAX_CHARS: usize = 3;
const FOOTNOTE_MARKER_MAX_FONT_RATIO: f32 = 0.8;
const FOOTNOTE_MARKER_MAX_GAP_EM: f32 = 0.5;
const FOOTNOTE_MARKER_MIN_VERTICAL_OVERLAP_RATIO: f32 = 0.8;
const FOOTNOTE_MARKER_MIN_RISE_EM: f32 = 0.1;
const FOOTNOTE_RUN_MIN_LENGTH: usize = 3;
const FOOTNOTE_RUN_MAX_FONT_DELTA: f32 = 0.5;
const FOOTNOTE_RUN_MAX_LEFT_DELTA_EM: f32 = 0.5;
const FOOTNOTE_RUN_MAX_VERTICAL_GAP_EM: f32 = 0.5;
const FOOTNOTE_RUN_MAX_GAP_SPREAD_EM: f32 = 0.25;

/// Rejoin a small raised footnote marker that was split from its body.
///
/// Standalone numeric markers are initially classified as page numbers. Only
/// strong same-line geometry can override that classification, so genuine page
/// numbers and numbered list items remain untouched.
fn merge_spatial_footnote_markers(paragraphs: &mut Vec<PdfParagraph>) {
    let mut merged_pairs = vec![false; paragraphs.len()];
    let mut index = 0;
    while index + 1 < paragraphs.len() {
        if !is_spatial_footnote_pair(&paragraphs[index], &paragraphs[index + 1]) {
            index += 1;
            continue;
        }

        let marker = paragraphs.remove(index);
        merged_pairs.remove(index);
        let body = &mut paragraphs[index];
        let mut lines = marker.lines;
        lines.append(&mut body.lines);
        body.lines = lines;
        body.text.clear();
        body.block_bbox = marker.block_bbox.zip(body.block_bbox).map(|(marker_bbox, body_bbox)| {
            (
                marker_bbox.0.min(body_bbox.0),
                marker_bbox.1.min(body_bbox.1),
                marker_bbox.2.max(body_bbox.2),
                marker_bbox.3.max(body_bbox.3),
            )
        });
        body.word_count = PdfParagraph::compute_word_count("", &body.lines);
        body.is_page_furniture = false;
        merged_pairs[index] = true;
        index += 1;
    }
    merge_consecutive_spatial_footnotes(paragraphs, &mut merged_pairs);
}

fn spatial_footnote_number(paragraph: &PdfParagraph) -> Option<u32> {
    if paragraph.lines.len() < 2 || paragraph.heading_level.is_some() || paragraph.is_list_item {
        return None;
    }
    let marker = paragraph.lines.first()?.segments.first()?;
    if marker.font_size > paragraph.dominant_font_size * FOOTNOTE_MARKER_MAX_FONT_RATIO {
        return None;
    }
    marker.text.trim().parse().ok()
}

fn spatial_footnote_gap(upper: &PdfParagraph, lower: &PdfParagraph) -> Option<f32> {
    let (_, upper_bottom, _, _) = upper.block_bbox?;
    let (_, _, _, lower_top) = lower.block_bbox?;
    Some(upper_bottom - lower_top)
}

fn spatial_footnotes_are_adjacent(upper: &PdfParagraph, lower: &PdfParagraph) -> bool {
    let Some(upper_number) = spatial_footnote_number(upper) else {
        return false;
    };
    let Some(lower_number) = spatial_footnote_number(lower) else {
        return false;
    };
    let Some((upper_left, _, _, _)) = upper.block_bbox else {
        return false;
    };
    let Some((lower_left, _, _, _)) = lower.block_bbox else {
        return false;
    };
    let Some(gap) = spatial_footnote_gap(upper, lower) else {
        return false;
    };
    upper_number.checked_add(1) == Some(lower_number)
        && upper.layout_region_path == lower.layout_region_path
        && (upper.dominant_font_size - lower.dominant_font_size).abs() < FOOTNOTE_RUN_MAX_FONT_DELTA
        && (upper_left - lower_left).abs() <= upper.dominant_font_size * FOOTNOTE_RUN_MAX_LEFT_DELTA_EM
        && gap >= 0.0
        && gap <= upper.dominant_font_size * FOOTNOTE_RUN_MAX_VERTICAL_GAP_EM
}

fn spatial_footnote_run_is_regular(run: &[PdfParagraph]) -> bool {
    if run.len() < FOOTNOTE_RUN_MIN_LENGTH
        || !run
            .windows(2)
            .all(|pair| spatial_footnotes_are_adjacent(&pair[0], &pair[1]))
    {
        return false;
    }
    let gaps = run
        .windows(2)
        .filter_map(|pair| spatial_footnote_gap(&pair[0], &pair[1]))
        .collect::<Vec<_>>();
    let minimum = gaps.iter().copied().fold(f32::INFINITY, f32::min);
    let maximum = gaps.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    maximum - minimum <= run[0].dominant_font_size * FOOTNOTE_RUN_MAX_GAP_SPREAD_EM
}

fn merge_spatial_footnote_run(run: Vec<PdfParagraph>) -> PdfParagraph {
    let mut iter = run.into_iter();
    let mut merged = iter.next().expect("footnote run is non-empty");
    for mut paragraph in iter {
        merged.lines.append(&mut paragraph.lines);
        merged.block_bbox = merged.block_bbox.zip(paragraph.block_bbox).map(|(left, right)| {
            (
                left.0.min(right.0),
                left.1.min(right.1),
                left.2.max(right.2),
                left.3.max(right.3),
            )
        });
    }
    merged.text.clear();
    merged.word_count = PdfParagraph::compute_word_count("", &merged.lines);
    merged
}

fn merge_consecutive_spatial_footnotes(paragraphs: &mut Vec<PdfParagraph>, merged_pairs: &mut Vec<bool>) {
    let mut start = 0;
    while start + FOOTNOTE_RUN_MIN_LENGTH <= paragraphs.len() {
        let minimum_end = start + FOOTNOTE_RUN_MIN_LENGTH;
        if !merged_pairs[start..minimum_end].iter().all(|merged| *merged)
            || !spatial_footnote_run_is_regular(&paragraphs[start..minimum_end])
        {
            start += 1;
            continue;
        }

        let mut end = minimum_end;
        while end < paragraphs.len() && merged_pairs[end] && spatial_footnote_run_is_regular(&paragraphs[start..=end]) {
            end += 1;
        }

        let merged = merge_spatial_footnote_run(paragraphs.drain(start..end).collect());
        paragraphs.insert(start, merged);
        merged_pairs.drain(start..end);
        merged_pairs.insert(start, false);
        start += 1;
    }
}

fn is_spatial_footnote_pair(marker: &PdfParagraph, body: &PdfParagraph) -> bool {
    if !marker.is_page_furniture
        || body.is_page_furniture
        || body.heading_level.is_some()
        || body.is_list_item
        || body.is_code_block
        || body.is_formula
        || body.caption_for.is_some()
        || marker.layout_region_path != body.layout_region_path
        || !is_compact_footnote_marker(&paragraph_text_raw(marker))
    {
        return false;
    }

    let Some((marker_left, marker_bottom, marker_right, marker_top)) = marker.block_bbox else {
        return false;
    };
    let Some((body_left, body_bottom, _, body_top)) = body.block_bbox else {
        return false;
    };
    let marker_height = marker_top - marker_bottom;
    let body_height = body_top - body_bottom;
    let overlap = marker_top.min(body_top) - marker_bottom.max(body_bottom);
    let minimum_height = marker_height.min(body_height);
    let horizontal_gap = body_left - marker_right;

    marker_left < body_left
        && horizontal_gap >= 0.0
        && horizontal_gap <= body.dominant_font_size * FOOTNOTE_MARKER_MAX_GAP_EM
        && marker.dominant_font_size <= body.dominant_font_size * FOOTNOTE_MARKER_MAX_FONT_RATIO
        && marker_bottom - body_bottom >= body.dominant_font_size * FOOTNOTE_MARKER_MIN_RISE_EM
        && minimum_height > 0.0
        && overlap >= minimum_height * FOOTNOTE_MARKER_MIN_VERTICAL_OVERLAP_RATIO
}

fn is_compact_footnote_marker(text: &str) -> bool {
    let marker = text.trim();
    let char_count = marker.chars().count();
    char_count > 0
        && char_count <= FOOTNOTE_MARKER_MAX_CHARS
        && (marker.chars().all(|character| character.is_ascii_digit())
            || marker
                .chars()
                .all(|character| matches!(character, '*' | '†' | '‡' | '§')))
}

/// Maximum word count for a paragraph to be considered a page-number candidate.
///
/// The longest conventional form ("Chapter 2 — Page 14 of 30") is eight tokens;
/// ten leaves headroom without admitting prose.
const MAX_PAGE_NUMBER_WORD_COUNT: usize = 10;

/// Page width assumed when a document yields no usable paragraph geometry.
///
/// US Letter portrait, matching the 792pt height fallback used for `page_heights`.
const FALLBACK_PAGE_WIDTH_PTS: f32 = 612.0;

/// Page height assumed when `page_heights` carries no entry for a page index.
const FALLBACK_PAGE_HEIGHT_PTS: f32 = 792.0;

/// Mark confirmed running page numbers as furniture (GH#1411).
///
/// This replaces a context-free string test that matched any short numeric or
/// Roman-looking token anywhere on the page, including table cells, list
/// markers, footnote references and stray capitals. Because furniture under 80
/// alphanumeric characters is physically deleted by `retain_page_furniture_safely`,
/// that test caused silent content loss.
///
/// Three independent signals must agree before anything is marked:
///
/// 1. **Shape** — `classify_page_number_text` recognizes the token. Shape alone
///    is never sufficient; it only makes a paragraph a candidate.
/// 2. **Position** — the paragraph's vertical centre falls in the top or bottom
///    margin band. Body-band candidates are observed (they are counter-evidence
///    for the sequence) but are never deletable.
/// 3. **Cross-page sequence** — `PageNumberSequence` has seen every page before
///    any deletion decision is taken, so a single isolated match can never be
///    removed. Deletion requires `confidence_at` to reach `DELETION_THRESHOLD`.
///
/// Where confidence falls short the text is kept. Retaining an occasional page
/// number is far cheaper than silently dropping a table cell.
///
/// # Relationship to the layout model
///
/// Layout hints **inform, never override**, and this heuristic **stands down**
/// wherever the layout model has an opinion:
///
/// - `layout_class == None` — the layout model did not run, or produced no
///   class for this paragraph. Geometry plus cross-page sequence decide here.
/// - `layout_class == Some(PageHeader | PageFooter)` — the layout path already
///   owns this paragraph. It is marked furniture by `apply_layout_overrides` and
///   then selectively un-marked by `un_mark_layout_furniture_per_config`
///   according to `include_headers` / `include_footers`. Re-marking it here
///   would silently override that user configuration.
/// - `layout_class == Some(_other_)` — a positive body-content classification
///   from the model, which counts as evidence against deletion.
///
/// The single consequence of this rule is `layout_class_permits_page_number_deletion`.
fn mark_validated_page_numbers(
    all_pages: &mut [Vec<PdfParagraph>],
    page_heights: &[f32],
    include_headers: bool,
    include_footers: bool,
) {
    use super::page_number::{MarginBand, PageNumberSequence, margin_band};

    let page_width = document_content_width(all_pages);
    let mut sequence = PageNumberSequence::new();
    // (page index, paragraph index, y ratio, x ratio) for every observed candidate.
    let mut observations: Vec<(usize, usize, f32, f32)> = Vec::new();

    // Pass 1: observe every candidate on every page. No deletion decision is
    // taken here — the sequence is not usable until it has seen all pages.
    for (page_index, page) in all_pages.iter().enumerate() {
        let page_height = page_heights
            .get(page_index)
            .copied()
            .unwrap_or(FALLBACK_PAGE_HEIGHT_PTS);
        for (paragraph_index, paragraph) in page.iter().enumerate() {
            let Some((y_ratio, x_ratio, candidate)) = page_number_observation(paragraph, page_height, page_width)
            else {
                continue;
            };
            sequence.observe(page_index, margin_band(y_ratio), x_ratio, &candidate);
            observations.push((page_index, paragraph_index, y_ratio, x_ratio));
        }
    }

    // Pass 2: confirm. Every page has now been observed.
    let mut confirmed = 0_usize;
    for (page_index, paragraph_index, y_ratio, x_ratio) in observations {
        let band = margin_band(y_ratio);
        // ~keep Geometric page numbers have a known region, unlike unpositioned tagged PageNumber artifacts.
        let preserve_region = match band {
            MarginBand::Top => include_headers,
            MarginBand::Bottom => include_footers,
            MarginBand::Body => true,
        };
        if preserve_region {
            continue;
        }
        if sequence.confidence_at(page_index, band, x_ratio) < PageNumberSequence::DELETION_THRESHOLD {
            continue;
        }
        if let Some(paragraph) = all_pages
            .get_mut(page_index)
            .and_then(|page| page.get_mut(paragraph_index))
        {
            paragraph.is_page_furniture = true;
            confirmed += 1;
        }
    }

    tracing::debug!(
        pages = all_pages.len(),
        confirmed,
        "page-number furniture confirmed by position and cross-page sequence"
    );
}

/// Whether the layout model's opinion permits the page-number heuristic to act.
///
/// See the "Relationship to the layout model" section on
/// `mark_validated_page_numbers`: any layout class at all — header, footer, or
/// body content — takes precedence, so this heuristic only acts where the model
/// is silent.
fn layout_class_permits_page_number_deletion(paragraph: &PdfParagraph) -> bool {
    paragraph.layout_class.is_none()
}

/// Build a page-number observation for one paragraph.
///
/// Returns `(y_ratio, x_ratio, candidate)`, where `y_ratio` is 0.0 at the top of
/// the page and 1.0 at the bottom. `None` when the paragraph is not a candidate
/// or carries no usable geometry — either way it is never deletable.
fn page_number_observation(
    paragraph: &PdfParagraph,
    page_height: f32,
    page_width: f32,
) -> Option<(f32, f32, super::page_number::PageNumberCandidate)> {
    if paragraph.heading_level.is_some()
        || paragraph.is_list_item
        || paragraph.is_code_block
        || paragraph.is_page_furniture
        || paragraph.word_count > MAX_PAGE_NUMBER_WORD_COUNT
    {
        return None;
    }
    if !layout_class_permits_page_number_deletion(paragraph) {
        return None;
    }
    let text = paragraph_text_raw(paragraph);
    let candidate = super::page_number::classify_page_number_text(text.trim())?;
    let (y_ratio, x_ratio) = paragraph_position_ratios(paragraph, page_height, page_width)?;
    Some((y_ratio, x_ratio, candidate))
}

/// Normalized position of a paragraph's centre within its page.
///
/// Returns `(y_ratio, x_ratio)` with `y_ratio` 0.0 at the top of the page and
/// 1.0 at the bottom — PDF space measures y upward from the page bottom, so the
/// vertical axis is inverted here to match the band API.
fn paragraph_position_ratios(paragraph: &PdfParagraph, page_height: f32, page_width: f32) -> Option<(f32, f32)> {
    if !page_height.is_finite() || page_height <= 0.0 || !page_width.is_finite() || page_width <= 0.0 {
        return None;
    }
    let (left, bottom, right, top) = finite_paragraph_bbox(paragraph)?;
    let centre_y = (bottom + top) * 0.5;
    let centre_x = (left + right) * 0.5;
    let y_ratio = (1.0 - centre_y / page_height).clamp(0.0, 1.0);
    let x_ratio = (centre_x / page_width).clamp(0.0, 1.0);
    Some((y_ratio, x_ratio))
}

/// `paragraph_geometry_bbox` restricted to fully finite boxes.
///
/// A paragraph assembled from degenerate font metrics can carry NaN or infinite
/// bounds; normalizing those would produce a meaningless position ratio, so such
/// paragraphs are treated as having no geometry and are therefore never deletable.
fn finite_paragraph_bbox(paragraph: &PdfParagraph) -> Option<(f32, f32, f32, f32)> {
    let bbox = paragraph_geometry_bbox(paragraph)?;
    let (left, bottom, right, top) = bbox;
    (left.is_finite() && bottom.is_finite() && right.is_finite() && top.is_finite()).then_some(bbox)
}

/// Widest right edge across the whole document, used to normalize horizontal
/// position.
///
/// A document-wide value rather than a per-page one: `PageNumberSequence`
/// compares horizontal positions *across* pages, so the normalizer must be the
/// same on every page or a stable footer slot would read as drifting.
fn document_content_width(all_pages: &[Vec<PdfParagraph>]) -> f32 {
    let widest = all_pages
        .iter()
        .flatten()
        .filter_map(finite_paragraph_bbox)
        .map(|(_, _, right, _)| right)
        .filter(|right| *right > 0.0)
        .fold(0.0_f32, f32::max);
    if widest > 0.0 { widest } else { FALLBACK_PAGE_WIDTH_PTS }
}

/// Apply the structure pipeline's cross-page repeating-text policy to pages that
/// were already classified by another source, such as OCR layout detection.
///
/// No table data is available here, so same-page table-backed duplicates are
/// preserved. Consecutive rendering artifacts still require coincident valid
/// geometry; unpositioned OCR text is never collapsed by that pass. ~keep
#[cfg(any(feature = "ocr", feature = "ocr-pipeline"))]
pub(crate) fn strip_repeating_text_from_pages(pages: &mut [Vec<PdfParagraph>], page_heights: &[f32]) {
    mark_cross_page_repeating_text(pages, page_heights);
    mark_cross_page_repeating_short_text(pages);
    for page in pages.iter_mut() {
        retain_page_furniture_safely(page);
    }
    deduplicate_paragraphs(pages, &ahash::AHashMap::new());
}

/// Filter page furniture paragraphs with a safety valve.
///
/// Removes paragraphs marked as page furniture (headers/footers) by layout
/// detection. If removing ALL furniture-marked paragraphs would leave zero
/// content, the furniture markings are cleared instead — better to include
/// headers/footers than to produce empty output. This handles layout models
/// misclassifying body text as page furniture on non-standard document types
/// (e.g., legal transcripts, cover pages).
fn retain_page_furniture_safely(paragraphs: &mut Vec<PdfParagraph>) {
    let total = paragraphs.len();
    let furniture_count = paragraphs.iter().filter(|p| p.is_page_furniture).count();

    if furniture_count == 0 {
        return;
    }

    if furniture_count >= total {
        for para in paragraphs.iter_mut() {
            para.is_page_furniture = false;
        }
        return;
    }

    let total_alphanum: usize = paragraphs.iter().map(paragraph_alphanum_len).sum();

    if total_alphanum > 0 {
        let furniture_alphanum: usize = paragraphs
            .iter()
            .filter(|p| p.is_page_furniture)
            .map(paragraph_alphanum_len)
            .sum();

        if furniture_alphanum * 100 > total_alphanum * 30 {
            for para in paragraphs.iter_mut() {
                para.is_page_furniture = false;
            }
            return;
        }
    }

    const MIN_SUBSTANTIVE_CHARS: usize = 80;

    paragraphs.retain(|p| {
        if !p.is_page_furniture {
            return true;
        }
        paragraph_alphanum_len(p) > MIN_SUBSTANTIVE_CHARS
    });
}

/// Count alphanumeric characters in a paragraph's text content.
fn paragraph_alphanum_len(para: &PdfParagraph) -> usize {
    para.lines
        .iter()
        .flat_map(|line| line.segments.iter())
        .map(|seg| seg.text.bytes().filter(|b| b.is_ascii_alphanumeric()).count())
        .sum()
}

/// Dehyphenate paragraphs by rejoining words split across line boundaries.
///
/// When `has_positions` is true (heuristic extraction path), both explicit
/// trailing hyphens and implicit breaks (no hyphen, full line) are handled.
/// When false (structure tree path with x=0, width=0), only explicit trailing
/// hyphens are rejoined to avoid false positives.
fn dehyphenate_paragraphs(paragraphs: &mut [PdfParagraph], has_positions: bool, hyphen_witnesses: &HyphenWitnesses) {
    for para in paragraphs.iter_mut() {
        if para.is_code_block || para.lines.len() < 2 {
            continue;
        }
        if has_positions {
            dehyphenate_paragraph_lines(para, hyphen_witnesses);
        } else {
            dehyphenate_hyphen_only(para, hyphen_witnesses);
        }
    }
}

/// High-confidence lexical compounds whose source hyphen must survive a line break.
///
/// A trailing ASCII hyphen is otherwise indistinguishable from a discretionary PDF
/// line-wrap hyphen. Exact pair matching is intentionally narrower than prefix or
/// suffix rules: it protects common compounds without suppressing repairs such as
/// `soft-` + `ware`.
const PRESERVED_LEXICAL_COMPOUNDS: &[(&str, &str)] = &[
    ("cost", "effective"),
    ("evidence", "based"),
    ("high", "level"),
    ("long", "term"),
    ("low", "level"),
    ("real", "time"),
    ("short", "term"),
    ("state", "of-the-art"),
    ("user", "defined"),
    ("well", "known"),
];

/// Minimum letters required on each side of a mid-run hyphen before
/// [`collect_hyphen_witnesses`] records it, to avoid single-letter noise
/// (initials, bullet dashes) minting spurious witness pairs.
const MIN_HYPHEN_WITNESS_WORD_LEN: usize = 2;

/// Collect `(left, right)` word pairs the document itself writes as a single
/// hyphenated token, so a genuine authored hyphen at a line break can be told
/// apart from a hyphen that merely happens to fall at a line-wrap boundary (#1543).
///
/// Only a hyphen that is NOT the last character of its segment's text can witness a
/// real compound: a line-wrap hyphen is, by construction, the final character before
/// the break, so restricting the scan to strictly mid-run hyphens avoids witnessing
/// the very artifact this collector exists to judge. Must run before any page's
/// segments are moved out of `all_page_segments` (see call site in
/// `extract_document_structure_from_segments`), since a witness on one page can be
/// the sole evidence for a break on another. ~keep
fn collect_hyphen_witnesses(all_page_segments: &[Vec<SegmentData>]) -> HyphenWitnesses {
    let mut witnesses = HyphenWitnesses::default();
    for segment in all_page_segments.iter().flatten() {
        let characters: Vec<char> = segment.text.chars().collect();
        if characters.len() < 3 {
            continue;
        }
        for position in 1..characters.len() - 1 {
            if characters[position] != '-' {
                continue;
            }
            let left: String = characters[..position]
                .iter()
                .rev()
                .take_while(|character| character.is_alphabetic())
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            let right: String = characters[position + 1..]
                .iter()
                .take_while(|character| character.is_alphabetic())
                .collect();
            let left_len = left.chars().count();
            let right_len = right.chars().count();
            if left_len < MIN_HYPHEN_WITNESS_WORD_LEN || right_len < MIN_HYPHEN_WITNESS_WORD_LEN {
                continue;
            }
            witnesses.insert((left.to_ascii_lowercase(), right.to_ascii_lowercase()));
        }
    }
    witnesses
}

/// Collect standalone alphabetic words the document itself writes elsewhere, so
/// [`repair_ligature_spaces`] can tell a genuine word boundary apart from a
/// decomposed-ligature gap that looks identical at the string layer (#1591).
///
/// A token that is itself one half of a ligature-space candidate pattern (ends in
/// `f` right before whitespace, or starts with `i`/`l`/`f` right after it) is not
/// independent evidence for that occurrence: the very space under judgment put it
/// there, so counting it would make every candidate witness itself and disable the
/// repair (see the `f irst` false-negative this guards against). The same word
/// witnessed elsewhere in the document, in a position that is not itself a
/// candidate, is unaffected and still counts. Must run before any page's segments
/// are moved out of `all_page_segments` (see call site in
/// `extract_document_structure_from_segments`), mirroring
/// [`collect_hyphen_witnesses`]. ~keep
fn collect_word_witnesses(all_page_segments: &[Vec<SegmentData>]) -> WordWitnesses {
    let mut witnesses = WordWitnesses::default();
    for segment in all_page_segments.iter().flatten() {
        let cores: Vec<&str> = segment
            .text
            .split_whitespace()
            .map(|token| token.trim_matches(|c: char| !c.is_alphabetic()))
            .collect();
        for index in 0..cores.len() {
            let core = cores[index];
            if core.chars().count() < MIN_LIGATURE_WITNESS_WORD_LEN {
                continue;
            }
            let is_left_of_candidate = core.ends_with('f')
                && cores
                    .get(index + 1)
                    .and_then(|next| next.chars().next())
                    .is_some_and(|c| matches!(c, 'i' | 'l' | 'f'));
            let is_right_of_candidate = index > 0
                && cores[index - 1].ends_with('f')
                && core.chars().next().is_some_and(|c| matches!(c, 'i' | 'l' | 'f'));
            if is_left_of_candidate || is_right_of_candidate {
                continue;
            }
            witnesses.insert(core.to_ascii_lowercase());
        }
    }
    witnesses
}

pub(super) fn should_preserve_lexical_hyphen(
    trailing_word: &str,
    leading_word: &str,
    hyphen_witnesses: &HyphenWitnesses,
) -> bool {
    let trim_non_lexical = |ch: char| !ch.is_alphanumeric() && ch != '-';
    let left = trailing_word.trim_matches(trim_non_lexical);
    let right = leading_word.trim_matches(trim_non_lexical);

    let matches_static_compound = PRESERVED_LEXICAL_COMPOUNDS
        .iter()
        .any(|&(expected_left, expected_right)| {
            left.eq_ignore_ascii_case(expected_left) && right.eq_ignore_ascii_case(expected_right)
        });
    matches_static_compound || hyphen_witnesses.contains(&(left.to_ascii_lowercase(), right.to_ascii_lowercase()))
}

/// Whether adjacent extraction runs actually cross a visual line boundary.
///
/// `PdfLine` boundaries can also be introduced by inline style/run splitting. A
/// suspended hyphen such as `vracht- en` may therefore appear at the end of one
/// logical line and the start of the next while both runs still share a baseline.
/// Dehyphenation is only licensed when the runs use the same reading frame and
/// their upright baselines differ by more than the inline-style tolerance.
fn spans_visual_line_break(trailing: &SegmentData, leading: &SegmentData) -> bool {
    if !trailing.has_same_rotation(leading) {
        return false;
    }

    let trailing_baseline = trailing.upright_baseline();
    let leading_baseline = leading.upright_baseline();
    trailing_baseline.is_finite()
        && leading_baseline.is_finite()
        && (trailing_baseline - leading_baseline).abs() > INLINE_STYLE_BASELINE_TOLERANCE
}

/// Core dehyphenation with position-based full-line detection.
///
/// For each line boundary, checks whether the line extends close to the right
/// margin. If so, attempts to rejoin the trailing word of one line with the
/// leading word of the next.
fn dehyphenate_paragraph_lines(para: &mut PdfParagraph, hyphen_witnesses: &HyphenWitnesses) {
    let max_right_edge = para
        .lines
        .iter()
        .flat_map(|l| l.segments.iter())
        .map(|s| s.x + s.width)
        .fold(0.0_f32, f32::max);

    if max_right_edge <= 0.0 {
        dehyphenate_hyphen_only(para, hyphen_witnesses);
        return;
    }

    let threshold = max_right_edge * FULL_LINE_FRACTION;

    let n = para.lines.len();
    for i in 0..(n - 1) {
        let trailing_right = para.lines[i].segments.last().map(|s| s.x + s.width).unwrap_or(0.0);
        if trailing_right < threshold {
            continue;
        }

        let crosses_visual_line = match (para.lines[i].segments.last(), para.lines[i + 1].segments.first()) {
            (Some(trailing), Some(leading)) => spans_visual_line_break(trailing, leading),
            _ => false,
        };
        if !crosses_visual_line {
            continue;
        }

        let trailing_text = match para.lines[i].segments.last() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };
        let leading_text = match para.lines[i + 1].segments.first() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };

        let has_trailing_hyphen = trailing_text.ends_with('-');
        if !has_trailing_hyphen {
            continue;
        }

        let leading_word = leading_text.split_whitespace().next().unwrap_or("");
        if leading_word.chars().next().is_some_and(|c| c.is_uppercase()) {
            continue;
        }

        let trailing_word = trailing_text
            .trim_end_matches('-')
            .split_whitespace()
            .last()
            .unwrap_or("");
        if trailing_word.chars().last().is_some_and(is_cjk_char) {
            continue;
        }

        let preserved_hyphen = if should_preserve_lexical_hyphen(trailing_word, leading_word, hyphen_witnesses) {
            "-"
        } else {
            ""
        };
        let joined_word = format!("{trailing_word}{preserved_hyphen}{leading_word}");

        if let Some(seg) = para.lines[i].segments.last_mut() {
            let text_without_word: String = seg
                .text
                .chars()
                .rev()
                .skip(trailing_word.len() + 1)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            seg.text = format!("{text_without_word}{joined_word}");
        }

        if let Some(seg) = para.lines[i + 1].segments.first_mut() {
            let after_leading_word = seg.text.trim_start_matches(leading_word).trim_start();
            seg.text = after_leading_word.to_string();
        }
    }
}

/// Hyphen-only dehyphenation (no position data required).
///
/// Only joins lines when the trailing segment ends with an explicit hyphen.
/// Used for structure tree pages where x/width may be zero.
fn dehyphenate_hyphen_only(para: &mut PdfParagraph, hyphen_witnesses: &HyphenWitnesses) {
    let n = para.lines.len();
    for i in 0..(n - 1) {
        let crosses_visual_line = match (para.lines[i].segments.last(), para.lines[i + 1].segments.first()) {
            (Some(trailing), Some(leading)) => spans_visual_line_break(trailing, leading),
            _ => false,
        };
        if !crosses_visual_line {
            continue;
        }

        let trailing_text = match para.lines[i].segments.last() {
            Some(s) if s.text.ends_with('-') => s.text.clone(),
            _ => continue,
        };
        let leading_text = match para.lines[i + 1].segments.first() {
            Some(s) if !s.text.is_empty() => s.text.clone(),
            _ => continue,
        };

        let leading_word = leading_text.split_whitespace().next().unwrap_or("");
        if leading_word.chars().next().is_some_and(|c| c.is_uppercase()) {
            continue;
        }

        let trailing_word = trailing_text
            .trim_end_matches('-')
            .split_whitespace()
            .last()
            .unwrap_or("");
        if trailing_word.chars().last().is_some_and(is_cjk_char) {
            continue;
        }

        let preserved_hyphen = if should_preserve_lexical_hyphen(trailing_word, leading_word, hyphen_witnesses) {
            "-"
        } else {
            ""
        };
        let joined_word = format!("{trailing_word}{preserved_hyphen}{leading_word}");

        if let Some(seg) = para.lines[i].segments.last_mut() {
            let text_without_word: String = seg
                .text
                .chars()
                .rev()
                .skip(trailing_word.len() + 1)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            seg.text = format!("{text_without_word}{joined_word}");
        }

        if let Some(seg) = para.lines[i + 1].segments.first_mut() {
            let after_leading_word = seg.text.trim_start_matches(leading_word).trim_start();
            seg.text = after_leading_word.to_string();
        }
    }
}

/// Detect whether a set of paragraphs contains any font-size variation.
///
/// Variation is defined as any paragraph whose font size differs from the first
/// non-zero size by more than 0.5pt. Used to decide whether structure-tree pages
/// need font-size clustering for heading assignment.
fn has_font_size_variation(paragraphs: &[PdfParagraph]) -> bool {
    let mut first_size: Option<f32> = None;
    for para in paragraphs {
        let size = para.dominant_font_size;
        if size <= 0.0 {
            continue;
        }
        match first_size {
            None => first_size = Some(size),
            Some(fs) if (size - fs).abs() > 0.5 => return true,
            _ => {}
        }
    }
    false
}

/// Deduplicate paragraphs with identical text within each page.
///
/// Two-pass approach:
/// 1. Consecutive duplicates: remove identical body paragraphs at known,
///    coincident positions (catches repeated rendering artifacts).
/// 2. Non-consecutive duplicates: remove a body-text paragraph whose text is
///    also carried by a table detected on the same page (catches table
///    content rendered as both table and body text). A paragraph that no
///    detected table's cells carry is never touched by this pass, even if it
///    repeats another paragraph verbatim -- GH#1623 found a body sentence
///    deleted for matching an earlier title's words, with no table involved
///    at all. The comparison also preserves case, so a title-cased heading
///    cannot match a body fragment that differs only in case. ~keep
///
/// Only deduplicates body text — headings, list items, code blocks,
/// formulas, and captions are preserved even if duplicated.
fn deduplicate_paragraphs(
    all_pages: &mut [Vec<PdfParagraph>],
    table_coverage_by_page: &ahash::AHashMap<usize, Vec<TableCoverage>>,
) {
    for (page_index, page) in all_pages.iter_mut().enumerate() {
        if page.len() < 2 {
            continue;
        }

        let mut i = 0;
        while i + 1 < page.len() {
            let a_text = paragraph_text_whitespace_collapsed(&page[i]);
            let b_text = paragraph_text_whitespace_collapsed(&page[i + 1]);
            if is_dedup_candidate(&page[i])
                && is_dedup_candidate(&page[i + 1])
                && page[i].layout_region_path == page[i + 1].layout_region_path
                && a_text.len() >= 5
                && a_text == b_text
                && rendering_artifact_bbox(&page[i])
                    .is_some_and(|bbox| Some(bbox) == rendering_artifact_bbox(&page[i + 1]))
            {
                page.remove(i + 1);
            } else {
                i += 1;
            }
        }

        let Some(page_tables) = table_coverage_by_page
            .get(&page_index)
            .filter(|tables| !tables.is_empty())
        else {
            continue;
        };

        let mut seen = ahash::AHashSet::new();
        let mut to_remove = Vec::new();
        for (idx, para) in page.iter().enumerate() {
            if !is_dedup_candidate(para) {
                continue;
            }
            let text = paragraph_text_whitespace_collapsed(para);
            if text.len() < 15 {
                continue;
            }
            let glyphs = normalize_for_table_coverage(&text);
            if glyphs.is_empty() || !page_tables.iter().any(|table| table.cell_text.contains(&glyphs)) {
                continue;
            }
            if !seen.insert((para.layout_region_path, text)) {
                to_remove.push(idx);
            }
        }

        for &idx in to_remove.iter().rev() {
            page.remove(idx);
        }
    }
}

const DEFAULT_OUTLINE_HEADING_OFFSET: i64 = 2;
const MIN_OUTLINE_CALIBRATION_ANCHORS: usize = 2;
const MIN_MARKDOWN_HEADING_LEVEL: i64 = 1;
const MAX_MARKDOWN_HEADING_LEVEL: i64 = 6;

#[derive(Debug, Clone, Copy)]
struct OutlineParagraphMatch {
    page_index: usize,
    paragraph_index: usize,
    depth: usize,
}

fn recover_headings_from_outline(all_pages: &mut [Vec<PdfParagraph>], outline_entries: &[PdfOutlineEntry]) {
    let matches = collect_unique_outline_matches(all_pages, outline_entries);
    let offset = calibrated_outline_heading_offset(all_pages, &matches);

    for matched in matches {
        let paragraph = &mut all_pages[matched.page_index][matched.paragraph_index];
        if paragraph.heading_level.is_some() || !outline_layout_allows_heading(paragraph) {
            continue;
        }
        let depth = i64::try_from(matched.depth).unwrap_or(i64::MAX);
        let level = depth
            .saturating_add(offset)
            .clamp(MIN_MARKDOWN_HEADING_LEVEL, MAX_MARKDOWN_HEADING_LEVEL);
        paragraph.heading_level = Some(level as u8);
        paragraph.is_list_item = false;
        paragraph.is_page_furniture = false;
    }
}

fn collect_unique_outline_matches(
    all_pages: &[Vec<PdfParagraph>],
    outline_entries: &[PdfOutlineEntry],
) -> Vec<OutlineParagraphMatch> {
    let mut outline_counts = ahash::AHashMap::<(usize, String), usize>::new();
    for entry in outline_entries {
        if let Some(key) = outline_match_key(entry, all_pages.len()) {
            *outline_counts.entry(key).or_default() += 1;
        }
    }
    let paragraph_matches = all_pages
        .iter()
        .map(|page| {
            let mut matches = ahash::AHashMap::<String, (usize, usize)>::new();
            for (index, paragraph) in page.iter().enumerate() {
                let title = normalize_outline_title(&paragraph_text_raw(paragraph));
                let entry = matches.entry(title).or_insert((0, index));
                entry.0 += 1;
            }
            matches
        })
        .collect::<Vec<_>>();

    outline_entries
        .iter()
        .filter_map(|entry| {
            let (page_index, title) = outline_match_key(entry, all_pages.len())?;
            if outline_counts.get(&(page_index, title.clone())) != Some(&1) {
                return None;
            }
            let &(paragraph_count, paragraph_index) = paragraph_matches[page_index].get(&title)?;
            (paragraph_count == 1).then_some(OutlineParagraphMatch {
                page_index,
                paragraph_index,
                depth: entry.depth,
            })
        })
        .collect()
}

fn outline_match_key(entry: &PdfOutlineEntry, page_count: usize) -> Option<(usize, String)> {
    let page_number = entry.page_number?;
    let page_index = usize::try_from(page_number.checked_sub(1)?).ok()?;
    let title = normalize_outline_title(&entry.title);
    (page_index < page_count && !title.is_empty()).then_some((page_index, title))
}

fn calibrated_outline_heading_offset(all_pages: &[Vec<PdfParagraph>], matches: &[OutlineParagraphMatch]) -> i64 {
    let mut counts = ahash::AHashMap::<i64, usize>::new();
    for matched in matches {
        let paragraph = &all_pages[matched.page_index][matched.paragraph_index];
        if !outline_layout_allows_heading(paragraph) {
            continue;
        }
        if let Some(level) = paragraph.heading_level {
            let depth = i64::try_from(matched.depth).unwrap_or(i64::MAX);
            *counts.entry(i64::from(level).saturating_sub(depth)).or_default() += 1;
        }
    }

    let max_count = counts.values().copied().max().unwrap_or_default();
    let mut winners = counts.into_iter().filter(|(_, count)| *count == max_count);
    let winner = winners.next();
    match (winner, winners.next(), max_count) {
        (Some((offset, _)), None, count) if count >= MIN_OUTLINE_CALIBRATION_ANCHORS => offset,
        _ => DEFAULT_OUTLINE_HEADING_OFFSET,
    }
}

fn outline_layout_allows_heading(paragraph: &PdfParagraph) -> bool {
    if paragraph.is_code_block || paragraph.is_formula || paragraph.caption_for.is_some() {
        return false;
    }
    matches!(
        paragraph.layout_class,
        None | Some(super::types::LayoutHintClass::Title)
            | Some(super::types::LayoutHintClass::SectionHeader)
            | Some(super::types::LayoutHintClass::Text)
            | Some(super::types::LayoutHintClass::Other)
    )
}

fn normalize_outline_title(text: &str) -> String {
    let text = strip_section_label(text.trim());
    let mut normalized = String::new();
    let mut pending_space = false;
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            if pending_space && !normalized.is_empty() {
                normalized.push(' ');
            }
            normalized.push(character);
            pending_space = false;
        } else if !normalized.is_empty() {
            pending_space = true;
        }
    }
    normalized
}

fn strip_section_label(text: &str) -> &str {
    let Some((first, rest)) = text.split_once(char::is_whitespace) else {
        return text;
    };
    let punctuated = first
        .chars()
        .next()
        .is_some_and(|character| matches!(character, '(' | '['))
        || first
            .chars()
            .last()
            .is_some_and(|character| matches!(character, '.' | ')' | ']' | ':'));
    let core = first.trim_matches(|character| matches!(character, '(' | '[' | '.' | ')' | ']' | ':'));
    let decimal_parts = core.split('.').collect::<Vec<_>>();
    let decimal = !decimal_parts.is_empty()
        && decimal_parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    let decimal_label = decimal && (punctuated || decimal_parts.len() > 1 || core.len() <= 3);
    let roman_label = punctuated
        && !core.is_empty()
        && core
            .chars()
            .all(|character| matches!(character.to_ascii_uppercase(), 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M'));
    let letter_label = punctuated && core.len() == 1 && core.chars().all(|character| character.is_ascii_alphabetic());

    if decimal_label || roman_label || letter_label {
        rest.trim_start()
    } else {
        text
    }
}

fn paragraph_text_raw(para: &PdfParagraph) -> String {
    if para.text.is_empty() {
        para.lines
            .iter()
            .flat_map(|line| line.segments.iter())
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    } else {
        para.text.clone()
    }
}

/// Preserve case when comparing rendering artifacts or table duplicates (GH#1623). ~keep
fn paragraph_text_whitespace_collapsed(para: &PdfParagraph) -> String {
    paragraph_text_raw(para)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn rendering_artifact_bbox(paragraph: &PdfParagraph) -> Option<(f32, f32, f32, f32)> {
    // The geometry union's f32 min/max can hide NaN, so validate each used segment first. ~keep
    if paragraph.block_bbox.is_none()
        && paragraph.lines.iter().flat_map(|line| &line.segments).any(|segment| {
            !segment.x.is_finite()
                || !segment.y.is_finite()
                || !segment.baseline_y.is_finite()
                || !segment.width.is_finite()
                || !segment.height.is_finite()
                || segment.width <= 0.0
                || segment.height <= 0.0
        })
    {
        return None;
    }
    paragraph_geometry_bbox(paragraph)
        .filter(|&bbox| bbox_coordinates_are_finite(bbox) && bbox.0 < bbox.2 && bbox.1 < bbox.3)
}

fn is_dedup_candidate(p: &PdfParagraph) -> bool {
    p.heading_level.is_none()
        && !p.is_list_item
        && !p.is_code_block
        && !p.is_formula
        && !p.is_page_furniture
        && p.caption_for.is_none()
}

/// Minimum word count for the lead sentence before a run-in list's anchor
/// colon; guards against matching short labels (`"Note:"`, abbreviations)
/// that happen to be followed by a semicolon elsewhere in the text.
const RUN_IN_LIST_MIN_LEAD_WORDS: usize = 4;
/// Minimum semicolon-delimited clauses required to call a colon-introduced
/// run a "list" — a single clause is just a qualified sentence, not an
/// enumeration.
const RUN_IN_LIST_MIN_ITEMS: usize = 2;
/// Minimum word count per clause; guards against matching stray short
/// fragments (e.g. an abbreviation followed by `;`) as list items.
const RUN_IN_LIST_MIN_ITEM_WORDS: usize = 3;

/// Split a colon-introduced, semicolon-delimited "run-in" list — a prose
/// convention common in legal/contract text, e.g. "...is authorised to
/// exclude subscription rights: to exclude fractional amounts...; where the
/// new shares...;" — out of a single assembled paragraph into a lead
/// paragraph plus one list-item paragraph per clause.
///
/// These enumerations are frequently rendered with no distinguishing
/// indentation or line break from the surrounding prose (the source document
/// never used a real list, just semicolon-separated clauses within one
/// paragraph flow), so geometry-based list detection
/// (`classify::detect_indentation_based_lists`) never sees them — that pass
/// only promotes paragraphs already indented relative to the page's modal
/// left margin. This pass instead recognizes the enumeration from paragraph
/// text alone, after normal paragraph assembly, and works for both the
/// heuristic and structure-tree paragraph paths via [`paragraph_text_raw`].
///
/// xberg-io/xberg#1301.
fn split_colon_semicolon_run_in_lists(all_page_paragraphs: &mut [Vec<PdfParagraph>]) {
    for page_paragraphs in all_page_paragraphs.iter_mut() {
        let mut index = 0;
        while index < page_paragraphs.len() {
            match try_split_run_in_list(&page_paragraphs[index]) {
                Some(replacement) => {
                    let inserted = replacement.len();
                    page_paragraphs.splice(index..=index, replacement);
                    index += inserted;
                }
                None => index += 1,
            }
        }
    }
}

/// Attempt to split one paragraph into a lead paragraph plus run-in list
/// items. Returns `None` when the paragraph does not match the pattern, in
/// which case it is left untouched.
fn try_split_run_in_list(para: &PdfParagraph) -> Option<Vec<PdfParagraph>> {
    if para.heading_level.is_some()
        || para.is_list_item
        || para.is_code_block
        || para.is_formula
        || para.is_page_furniture
    {
        return None;
    }

    let normalized: String = paragraph_text_raw(para)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let colon_byte = normalized.rfind(':')?;
    let lead = normalized[..=colon_byte].trim();
    if lead.split_whitespace().count() < RUN_IN_LIST_MIN_LEAD_WORDS {
        return None;
    }

    let tail = normalized[colon_byte + 1..].trim_start();
    if tail.is_empty() {
        return None;
    }

    let items: Vec<&str> = tail
        .split_inclusive(';')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect();
    if items.len() < RUN_IN_LIST_MIN_ITEMS || !items.iter().all(|item| is_probable_run_in_list_item(item)) {
        return None;
    }

    let mut split = Vec::with_capacity(items.len() + 1);
    split.push(run_in_list_fragment(para, lead.to_string(), false));
    for item in items {
        split.push(run_in_list_fragment(para, item.to_string(), true));
    }
    Some(split)
}

/// Whether one semicolon-delimited clause reads as a genuine list item:
/// substantial (several words), a lowercase continuation of the lead
/// sentence (real clauses read as "to exclude...", "where...", not a new
/// capitalized sentence), and clause-terminated.
fn is_probable_run_in_list_item(item: &str) -> bool {
    item.split_whitespace().count() >= RUN_IN_LIST_MIN_ITEM_WORDS
        && item.chars().next().is_some_and(char::is_lowercase)
        && matches!(item.chars().last(), Some(';' | '.'))
}

/// Build one split-off fragment, inheriting the source paragraph's
/// non-textual attributes (font size, boldness, page association, etc.).
fn run_in_list_fragment(source: &PdfParagraph, text: String, is_list_item: bool) -> PdfParagraph {
    let word_count = text.split_whitespace().count();
    PdfParagraph {
        text,
        lines: Vec::new(),
        heading_level: None,
        is_list_item,
        is_code_block: false,
        is_formula: false,
        layout_class: if is_list_item {
            Some(super::types::LayoutHintClass::ListItem)
        } else {
            source.layout_class
        },
        word_count,
        ..source.clone()
    }
}

fn apply_text_repair_to_structure_tree_paragraphs(
    paragraphs: &mut Vec<PdfParagraph>,
    has_positions: bool,
    witnesses: &TextRepairWitnesses,
) {
    apply_to_all_segments(paragraphs, |text| fused_text_repairs(text, &witnesses.words));
    dehyphenate_paragraphs(paragraphs, has_positions, &witnesses.hyphens);
    split_embedded_list_items(paragraphs);
    synchronize_paragraph_text_metadata(paragraphs);
}

/// Invalidate cached paragraph text after mutating segments and refresh derived metadata.
///
/// Assembly derives both the emitted text and inline annotation byte ranges from segments
/// when `text` is empty. Keeping that cache empty prevents repaired segment text from
/// diverging from the stale pre-repair string used by the heuristic path.
fn synchronize_paragraph_text_metadata(paragraphs: &mut [PdfParagraph]) {
    for paragraph in paragraphs {
        paragraph.text.clear();
        paragraph.word_count = PdfParagraph::compute_word_count("", &paragraph.lines);
    }
}

fn compact_final_heading_hierarchy(all_pages: &mut [Vec<PdfParagraph>]) {
    let headings = all_pages
        .iter()
        .flat_map(|page| page.iter())
        .filter_map(|paragraph| paragraph.heading_level);
    let (h1_count, has_h2, has_deeper) = headings.fold((0usize, false, false), |state, level| {
        (
            state.0 + usize::from(level == 1),
            state.1 || level == 2,
            state.2 || level >= 3,
        )
    });
    if h1_count != 1 || has_h2 || !has_deeper {
        return;
    }

    for paragraph in all_pages.iter_mut().flat_map(|page| page.iter_mut()) {
        if let Some(level @ 3..) = paragraph.heading_level {
            paragraph.heading_level = Some(level - 1);
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod list_marker_tests;
