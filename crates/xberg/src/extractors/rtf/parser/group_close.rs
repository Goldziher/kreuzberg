//! Group-close helpers for the text-extraction pass: each ends a buffered destination
//! (`\listtext`, `\footnote`, `\shptxt`, `\annotation`) when its closing `}` is reached.

/// Close a `\listtext`/`\pntext` group if this `}` ends it, marking the
/// current list item as ordered when its buffered text looks like a numbered
/// or lettered marker (e.g. `1.` or `a)`). ~keep
pub(super) fn close_listtext_group(
    group_depth: i32,
    in_listtext: &mut bool,
    listtext_depth: i32,
    listtext_buf: &mut String,
    cur_ordered: &mut bool,
) {
    if !*in_listtext || group_depth >= listtext_depth {
        return;
    }
    *in_listtext = false;
    let lt = listtext_buf.trim();
    let is_ordered = lt
        .strip_suffix('.')
        .or_else(|| lt.strip_suffix(')'))
        .is_some_and(|prefix| {
            let p = prefix.trim();
            if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() {
                return true;
            }
            if p.chars().all(|c| c.is_ascii_alphabetic()) && !p.is_empty() {
                return true;
            }
            false
        });
    if is_ordered {
        *cur_ordered = true;
    }
    listtext_buf.clear();
}

/// Close a `\footnote` destination if this `}` ends it, recording the
/// buffered note text.
pub(super) fn close_footnote_group(
    group_depth: i32,
    in_footnote: &mut bool,
    footnote_depth: i32,
    footnote_buf: &mut String,
    footnotes: &mut Vec<String>,
) {
    if !*in_footnote || group_depth >= footnote_depth {
        return;
    }
    *in_footnote = false;
    let note = footnote_buf.trim().to_string();
    if !note.is_empty() {
        footnotes.push(note);
    }
    footnote_buf.clear();
}

/// Close a `\shptxt` (drawing-object/text-box text) destination if this `}`
/// ends it, recording the buffered text-box text.
pub(super) fn close_shptxt_group(
    group_depth: i32,
    in_shptxt: &mut bool,
    shptxt_depth: i32,
    shptxt_buf: &mut String,
    text_boxes: &mut Vec<String>,
) {
    if !*in_shptxt || group_depth >= shptxt_depth {
        return;
    }
    *in_shptxt = false;
    let text_box = shptxt_buf.trim().to_string();
    if !text_box.is_empty() {
        text_boxes.push(text_box);
    }
    shptxt_buf.clear();
}

/// Close an `\annotation` (Word comment) destination if this `}` ends it,
/// labeling the comment with its `\atnid` (if any) or a running counter.
pub(super) fn close_annotation_group(
    group_depth: i32,
    in_annotation: &mut bool,
    annotation_depth: i32,
    annotation_buf: &mut String,
    comments: &mut Vec<String>,
    pending_atnid: &mut Option<i32>,
) {
    if !*in_annotation || group_depth >= annotation_depth {
        return;
    }
    *in_annotation = false;
    let comment = annotation_buf.trim().to_string();
    if !comment.is_empty() {
        let label = pending_atnid
            .take()
            .map(|id| id.to_string())
            .unwrap_or_else(|| (comments.len() + 1).to_string());
        comments.push(format!("[Comment {label}]: {comment}"));
    } else {
        *pending_atnid = None;
    }
    annotation_buf.clear();
}
