using System.Text;
using System.Text.Json.Serialization;
using XRay.Content.Types;

namespace XRay.Content.Internal.Html;

/// <summary>
/// Extracts <see cref="HtmlMetadata"/> from raw HTML: head meta/title/link/base, the html lang
/// attribute, and document collections (headers, links, images, JSON-LD structured data).
/// Mirrors the metadata produced by html-to-markdown's conversion result. DOM depth for headers
/// is the count of open ancestor elements.
/// </summary>
public static class HtmlMeta
{
    private static readonly HashSet<string> Void = new(StringComparer.OrdinalIgnoreCase)
    {
        "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
        "param", "source", "track", "wbr",
    };

    public static HtmlMetadata Extract(string html)
    {
        // The collector runs inside the conversion walk, so it sees the same stripped source the
        // converter parses — including the canonical attribute spellings a document that needs
        // the html5ever repair reaches the walk with.
        html = HtmlToMarkdown.StripHiddenElements(HtmlToMarkdown.StripScriptAndStyleTags(html));
        bool canonicalAttrs = HtmlToMarkdown.NeedsCanonicalAttributes(html);
        string? Raw(string attrs, string name)
        {
            string? v = HtmlWalker.ExtractAttr(attrs, name);
            return v is not null && canonicalAttrs ? HtmlToMarkdown.CanonicalizeAttrValue(v) : v;
        }
        var m = new HtmlMetadata();
        int pos = 0, n = html.Length;
        int domDepth = 0;

        // Text-capture targets
        int captureHeading = 0;           // heading level currently open (0 = none)
        var headingText = new StringBuilder();
        int headingDepthAtOpen = 0;
        string? headingId = null;
        // A heading's recorded text is the markdown its children convert to, so a permalink
        // anchor inside one is `[label](href)` rather than the label alone.
        var headingLinks = new List<(int LabelStart, string Href, string? Title, List<string[]> Attrs, List<string> Rel)>();
        // html-to-markdown walks a table's subtree once per pass its handler makes over it, and
        // the collector records what it sees on every pass. A markdown table is walked three
        // times — a column-width pre-pass, the render, and the grid the structure collector
        // wants — while a table the handler renders as a layout list has no width pre-pass and
        // so is walked twice. A nested table therefore multiplies: it is re-entered once per
        // pass of its parent, except during the width pre-pass, which stops at a nested table
        // and measures its text instead. Collections are buffered per table and replayed into
        // the enclosing table (or the document) when that table closes.
        var tables = new List<TableFrame>();
        // Text inside a table cell is written with `*`, `_` and `|` escaped, and a link's label
        // is the markdown its children produced — so the label a cell's link records carries
        // those escapes too.
        int cellDepth = 0;
        List<object> HeaderSink() => tables.Count > 0 ? tables[^1].OwnSegment(tables[^1].Headers) : m.Headers;
        List<object> LinkSink() => tables.Count > 0 ? tables[^1].OwnSegment(tables[^1].Links) : m.Links;
        List<object> ImageSink() => tables.Count > 0 ? tables[^1].OwnSegment(tables[^1].Images) : m.Images;

        void CloseTable()
        {
            var frame = tables[^1];
            tables.RemoveAt(tables.Count - 1);
            frame.CloseRow();
            int passes = frame.Passes;
            var headers = frame.Replay(frame.Headers, passes);
            var links = frame.Replay(frame.Links, passes);
            var images = frame.Replay(frame.Images, passes);
            if (tables.Count > 0)
            {
                var parent = tables[^1];
                parent.Absorb(frame);
                parent.Headers.Add((TableFrame.Origin.Child, headers));
                parent.Links.Add((TableFrame.Origin.Child, links));
                parent.Images.Add((TableFrame.Origin.Child, images));
            }
            else
            {
                m.Headers.AddRange(headers);
                m.Links.AddRange(links);
                m.Images.AddRange(images);
            }
        }

        bool inAnchor = false;
        // Where each open inline marker was written, so its whitespace can be moved outside the
        // delimiters when it closes.
        var openMarkers = new List<(StringBuilder Buffer, string Marker, int At)>();
        var anchorText = new StringBuilder();
        // Open `<abbr>` expansions, appended when each one closes.
        var abbrTitles = new List<string>();
        // The autolink test is made against the anchor's plain text, not the markdown its
        // children render to, so the two are accumulated separately.
        var anchorRawText = new StringBuilder();
        string anchorHref = "";
        string? anchorTitle = null;
        // Where the anchor's content starts, so an anchor that renders to nothing can still be
        // told apart from one with no children at all.
        int anchorInnerStart = 0;
        List<string[]> anchorAttrs = new();
        List<string> anchorRel = new();
        bool inTitle = false;
        var titleText = new StringBuilder();
        // Preprocessing removes navigation and form subtrees before anything is collected, so a
        // sidebar's `<h2>Contents</h2>` is not one of the document's headings.
        int skipDepth = -1;
        // Head metadata is read from the children of `<head>`. A `<title>` written before the
        // head — as some Federal Register pages are — is not one of them, and a document whose
        // head holds nothing has no metadata at all.
        bool inHead = false;
        // Head metadata is gathered into a sorted map keyed the way the collector keys it, then
        // interpreted once at the end. The sort matters: several fields take the first value they
        // are offered, and "first" means first by key, not first in the document.
        var headMetadata = new SortedDictionary<string, string>(StringComparer.Ordinal);
        // A `<pre>` block is emitted as its raw text, so nothing inside it is visited as an
        // element: a link written inside preformatted text is not one of the document's links.
        int preDepth = 0;

        while (pos < n)
        {
            Cancel.ThrowIfRequested();
            if (html[pos] == '<')
            {
                if (string.CompareOrdinal(html, pos, "<!--", 0, 4) == 0)
                {
                    pos = HtmlWalker.CommentEnd(html, pos);
                    continue;
                }
                int tagStart = pos;
                int gt = html.IndexOf('>', pos);
                if (gt < 0) break;
                string raw = html[(pos + 1)..gt];
                pos = gt + 1;
                if (raw.StartsWith('!') || raw.StartsWith('?')) continue;

                bool closing = raw.StartsWith('/');
                string content = closing ? raw[1..] : raw;
                bool selfClose = content.TrimEnd().EndsWith('/');
                content = content.TrimEnd('/').Trim();
                var (nameRaw, attrsStr) = HtmlWalker.SplitTagName(content);
                string tag = nameRaw.ToLowerInvariant();

                // `scan_table` reads the whole subtree of the table it is deciding about, and it
                // reads the parsed tree — preprocessing drops nothing from it — so the scan runs
                // for every tag inside a table, skipped regions included.
                if (tables.Count > 0)
                {
                    var frame = tables[^1];
                    if (closing)
                    {
                        if (tag is "td" or "th" or "cell" && cellDepth > 0) cellDepth--;
                        if (tag is "tr" or "row") frame.CloseRow();
                        else if (tag == "caption" && frame.CaptionDepth > 0) frame.CaptionDepth--;
                    }
                    else
                    {
                        switch (tag)
                        {
                            case "a": frame.LinkCount++; break;
                            case "caption":
                                frame.HasCaption = true;
                                if (!selfClose) frame.CaptionDepth++;
                                break;
                            case "th": frame.HasHeader = true; break;
                            case "img" or "graphic":
                                if (HtmlWalker.ExtractAttr(attrsStr, "src") is not null
                                    || HtmlWalker.ExtractAttr(attrsStr, "alt") is not null) frame.HasText = true;
                                break;
                            case "cell":
                                if (HtmlWalker.ExtractAttr(attrsStr, "role") == "head") frame.HasHeader = true;
                                break;
                        }
                        if (tag is "td" or "th" or "cell" && !selfClose) cellDepth++;
                        if (tag is "tr" or "row") frame.OpenRow();
                        else if (tag is "td" or "th" or "cell")
                        {
                            string? colspan = HtmlWalker.ExtractAttr(attrsStr, "colspan");
                            string? rowspan = HtmlWalker.ExtractAttr(attrsStr, "rowspan");
                            if (colspan is not null || rowspan is not null) frame.HasSpan = true;
                            frame.AddCell(colspan is not null && int.TryParse(colspan, out int cs) && cs > 0 ? cs : 1);
                        }
                    }
                }

                if (closing)
                {
                    if (skipDepth >= 0)
                    {
                        if (!Void.Contains(tag) && domDepth > 0)
                        {
                            domDepth--;
                            if (domDepth <= skipDepth) skipDepth = -1;
                        }
                        continue;
                    }
                    if (tag == "pre" && preDepth > 0) preDepth--;
                    if (tag == "table" && tables.Count > 0) CloseTable();
                    if (tag is "h1" or "h2" or "h3" or "h4" or "h5" or "h6" && captureHeading != 0)
                    {
                        string text = HtmlWalker.NormalizeWhitespace(headingText.ToString());
                        if (text.Length > 0)
                            HeaderSink().Add(new Header
                            {
                                Level = captureHeading, Text = text, Id = headingId,
                                // A heading inside a table carries its real DOM depth. It read 0
                                // while the re-walks that recorded it started from a fresh tree;
                                // since 3.11.0 the render is the only pass that records, and it
                                // walks the cell at the table's own depth.
                                Depth = headingDepthAtOpen,
                                HtmlOffset = 0,
                            });
                        captureHeading = 0;
                        headingId = null;
                    }
                    else if (tag == "a" && inAnchor)
                    {
                        // An autolink returns from the link handler before it reaches the
                        // metadata collector, so `<https://example.com>` is a link in the output
                        // and no entry in `links` (`handlers/link.rs`).
                        string rawText = HtmlWalker.NormalizeWhitespace(anchorRawText.ToString()).Trim();
                        bool isAutolink = anchorHref.Length > 0 && HtmlToMarkdown.HasUriScheme(anchorHref)
                            && (rawText == anchorHref
                                || (anchorHref.StartsWith("mailto:", StringComparison.Ordinal)
                                    && rawText == anchorHref[7..]));
                        if (!isAutolink)
                        {
                            // An anchor whose children render to nothing — an icon button built
                            // from empty spans, say — falls back to its own href for a label.
                            string linkLabel = HtmlToMarkdown.NormalizeLinkLabel(anchorText.ToString());
                            if (linkLabel.Length == 0 && anchorHref.Length > 0 && tagStart > anchorInnerStart)
                                linkLabel = anchorHref;
                            LinkSink().Add(new Link
                            {
                                Href = anchorHref,
                                Text = CiteBacklinkLabel(linkLabel, anchorHref),
                                Title = anchorTitle,
                                LinkType = ClassifyLink(anchorHref),
                                Rel = anchorRel,
                                Attributes = anchorAttrs,
                            });
                        }
                        inAnchor = false;
                    }
                    else if (InlineMarker(tag) is { } closeMarker && (captureHeading != 0 || inAnchor))
                    {
                        var target = captureHeading != 0 ? headingText : anchorText;
                        CloseInlineMarker(openMarkers, target, closeMarker);
                    }
                    else if (tag == "abbr" && abbrTitles.Count > 0 && (captureHeading != 0 || inAnchor))
                    {
                        string expansion = abbrTitles[^1];
                        abbrTitles.RemoveAt(abbrTitles.Count - 1);
                        if (expansion.Length > 0)
                            (captureHeading != 0 ? headingText : anchorText)
                                .Append(" (").Append(expansion).Append(')');
                    }
                    else if (tag == "a" && captureHeading != 0 && headingLinks.Count > 0)
                    {
                        var (labelStart, linkHref, linkTitle, linkAttrs, linkRel) = headingLinks[^1];
                        headingLinks.RemoveAt(headingLinks.Count - 1);
                        string label = HtmlWalker.NormalizeWhitespace(
                            headingText.ToString(labelStart, headingText.Length - labelStart));
                        headingText.Length = labelStart;
                        headingText.Append(label).Append("](").Append(linkHref);
                        if (linkTitle is { Length: > 0 }) headingText.Append(" \"").Append(linkTitle).Append('"');
                        headingText.Append(')');
                        // A permalink anchor inside a heading is still a link: upstream collects
                        // it from its ordinary `<a>` handler, which the heading path does not
                        // bypass, so recording it only in the heading's markdown loses it.
                        LinkSink().Add(new Link
                        {
                            Href = linkHref,
                            Text = CiteBacklinkLabel(label, linkHref),
                            Title = linkTitle,
                            LinkType = ClassifyLink(linkHref),
                            Rel = linkRel,
                            Attributes = linkAttrs,
                        });
                    }
                    else if (tag == "head") inHead = false;
                    else if (tag == "title" && inTitle)
                    {
                        {
                            // Trimmed but not collapsed: a title written with two spaces between
                            // its halves keeps them.
                            string t = titleText.ToString().Trim();
                            // A repaired document reaches the walk with its references already
                            // resolved, so `&deg;` in a title is the degree sign by the time the
                            // collector reads it.
                            if (canonicalAttrs) t = HtmlToMarkdown.CanonicalizeText(t).Trim();
                            if (t.Length > 0) headMetadata["title"] = t;
                        }
                        inTitle = false;
                    }
                    if (!Void.Contains(tag) && domDepth > 0) domDepth--;
                    continue;
                }

                if (skipDepth >= 0)
                {
                    if (!Void.Contains(tag) && !selfClose) domDepth++;
                    continue;
                }

                if (!selfClose && !Void.Contains(tag)
                    && HtmlToMarkdown.ShouldDropForPreprocessing(
                        tag,
                        ExtractAttrDecoded(attrsStr, "role"),
                        ExtractAttrDecoded(attrsStr, "aria-label"),
                        ExtractAttrDecoded(attrsStr, "class"),
                        ExtractAttrDecoded(attrsStr, "id")))
                {
                    skipDepth = domDepth;
                    domDepth++;
                    continue;
                }

                if (tag == "pre" && !selfClose) preDepth++;
                if (preDepth > 0 && tag is "a" or "img" or "h1" or "h2" or "h3" or "h4" or "h5" or "h6")
                {
                    if (!Void.Contains(tag) && !selfClose) domDepth++;
                    continue;
                }

                // Emphasis inside a heading or a link is part of the markdown those record.
                if (InlineMarker(tag) is { } openMarker && (captureHeading != 0 || inAnchor) && !selfClose)
                {
                    var target = captureHeading != 0 ? headingText : anchorText;
                    openMarkers.Add((target, openMarker, target.Length));
                    target.Append(openMarker);
                    domDepth++;
                    continue;
                }

                // A `<br>` is a hard break in the markdown its heading or link records, and the
                // label collapses that break back to a single space.
                if (tag == "br" && (captureHeading != 0 || inAnchor))
                {
                    var target = captureHeading != 0 ? headingText : anchorText;
                    if (captureHeading != 0)
                    {
                        while (target.Length > 0 && target[^1] is ' ' or '\t' or '\n') target.Length--;
                        target.Append("  ");
                    }
                    else if (target.Length == 0 || target[^1] == '\n') target.Append('\n');
                    else target.Append("  \n");
                    continue;
                }

                // An abbreviation renders as `text (expansion)`, so a link whose label is one —
                // the `v`/`t`/`e` of a navigation box — records the expansion as part of its text.
                if (tag == "abbr" && (captureHeading != 0 || inAnchor) && !selfClose)
                {
                    abbrTitles.Add(Raw(attrsStr, "title")?.Trim() ?? "");
                    domDepth++;
                    continue;
                }

                // Opening / self-closing tag.
                switch (tag)
                {
                    // `lang` and `dir` are read from whichever of these carries them, first
                    // occurrence winning: a page that sets direction on `<body>` rather than
                    // `<html>` is still declaring the document's direction.
                    case "head" when !selfClose:
                        inHead = true;
                        goto case "html";
                    case "html":
                    case "body":
                    {
                        string? lang = ExtractAttrDecoded(attrsStr, "lang");
                        if (lang is not null && m.Language is null) m.Language = lang;
                        string? dir = ExtractAttrDecoded(attrsStr, "dir");
                        if (dir is not null && m.TextDirection is null) m.TextDirection = ParseTextDirection(dir);
                        break;
                    }
                    case "meta" when inHead:
                    {
                        // `Raw` so a repaired document's `content` reads as html5ever's
                        // serializer wrote it, the same as every other attribute here.
                        string? metaContent = Raw(attrsStr, "content");
                        if (metaContent is null) break;
                        if (HtmlWalker.ExtractAttr(attrsStr, "name") is { } metaName)
                            headMetadata["meta-" + metaName] = metaContent;
                        if (HtmlWalker.ExtractAttr(attrsStr, "property") is { } metaProperty)
                            headMetadata["meta-" + metaProperty] = metaContent;
                        break;
                    }
                    case "base" when inHead:
                    {
                        if (HtmlWalker.ExtractAttr(attrsStr, "href") is { } href)
                            headMetadata["base"] = href;
                        break;
                    }
                    case "link" when inHead:
                    {
                        // Substring, not token: the collector asks whether the rel list contains
                        // "canonical" at all.
                        string? rel = HtmlWalker.ExtractAttr(attrsStr, "rel");
                        string? href = HtmlWalker.ExtractAttr(attrsStr, "href");
                        if (rel is not null && href is not null && rel.Contains("canonical", StringComparison.Ordinal))
                            headMetadata["canonical"] = href;
                        break;
                    }
                    case "title":
                        if (inHead) { inTitle = true; titleText.Clear(); }
                        break;
                    case "table":
                        if (!selfClose)
                        {
                            // A layout table's rows are walked inline, which is what makes an
                            // image inside one degrade to its alt text. The verdict comes from
                            // the handler's own predicate over this table's markup, so the two
                            // sides cannot drift apart.
                            int tableEnd = FindElementEnd(html, tagStart, "table");
                            string tableHtml = html[tagStart..(tableEnd < 0 ? n : tableEnd)];
                            tables.Add(new TableFrame
                            {
                                BorderZero = HtmlWalker.ExtractAttr(attrsStr, "border") == "0",
                                Layout = HtmlToMarkdown.TableMarkupRendersAsLayoutList(tableHtml),
                            });
                        }
                        break;
                    case "h1": case "h2": case "h3": case "h4": case "h5": case "h6":
                        captureHeading = tag[1] - '0';
                        headingText.Clear();
                        headingLinks.Clear();
                        headingDepthAtOpen = domDepth;
                        headingId = ExtractAttrDecoded(attrsStr, "id");
                        break;
                    // An anchor with no href is a link target, not a link: its children are
                    // written straight into the heading.
                    case "a" when captureHeading != 0 && HtmlWalker.ExtractAttr(attrsStr, "href") is { } headingHref:
                    {
                        headingText.Append('[');
                        var headingAttrs = ParseAttrs(attrsStr, exclude: "href", canonicalAttrs, out var headingRel);
                        headingLinks.Add((headingText.Length, headingHref,
                            Raw(attrsStr, "title"), headingAttrs, headingRel));
                        break;
                    }
                    // An anchor with no href at all never reaches the link handler's collector:
                    // it is a link target, and only its children are written.
                    case "a" when HtmlWalker.ExtractAttr(attrsStr, "href") is null:
                        break;
                    case "a":
                    {
                        inAnchor = true;
                        anchorText.Clear();
                        anchorRawText.Clear();
                        anchorHref = ExtractAttrDecoded(attrsStr, "href") ?? "";
                        // The title is recorded as written: the link handler hands the collector
                        // the attribute's own bytes, and only the href is entity-decoded.
                        anchorTitle = Raw(attrsStr, "title");
                        anchorInnerStart = pos;
                        anchorAttrs = ParseAttrs(attrsStr, exclude: "href", canonicalAttrs, out anchorRel);
                        break;
                    }
                    case "img":
                    {
                        // The source is recorded as written. A query string spelled
                        // `?a=1&amp;b=2` stays that way — the collector reads the attribute, it
                        // does not resolve the URL.
                        string? src = Raw(attrsStr, "src");
                        string? alt = Raw(attrsStr, "alt");
                        var attrs = ParseAttrs(attrsStr, exclude: "src", canonicalAttrs, out _);
                        // Dimensions are recorded only when the element states both, which is
                        // what makes them a size rather than half of one.
                        uint[]? dimensions =
                            uint.TryParse(ExtractAttrDecoded(attrsStr, "width"), out uint width)
                            && uint.TryParse(ExtractAttrDecoded(attrsStr, "height"), out uint height)
                                ? [width, height]
                                : null;
                        var image = new Img
                        {
                            Src = src ?? "",
                            // An empty `alt` is the absence of alt text, not alt text that is
                            // empty, and is recorded as absent.
                            Alt = alt is { Length: > 0 } ? alt : null,
                            Title = Raw(attrsStr, "title"),
                            Dimensions = dimensions,
                            ImageType = ClassifyImage(src ?? ""),
                            Attributes = attrs,
                        };
                        ImageSink().Add(image);
                        // A link wrapping an image carries the image markdown as its label text —
                        // except inside a heading, where the image degrades to its alt text and
                        // the label carries that instead. A heading holding an image directly
                        // carries the alt text the same way.
                        // Inline context — a heading, or a cell of a table the handler renders
                        // as a list — degrades the image to its alt text.
                        bool inlineImage = captureHeading != 0 || tables.Exists(t => t.Layout);
                        // The title rides along in the markdown, exactly as the image handler
                        // writes it: `![alt](src "title")`.
                        string? imgTitle = Raw(attrsStr, "title");
                        string rendered = inlineImage
                            ? alt ?? ""
                            : "![" + (alt ?? "") + "](" + (src ?? "")
                              + (imgTitle is { Length: > 0 } ? " \"" + imgTitle + "\"" : "") + ")";
                        if (inAnchor) anchorText.Append(rendered);
                        else if (captureHeading != 0) headingText.Append(rendered);
                        break;
                    }
                    // An `<svg>` converts to an image whose source is the serialized subtree, so
                    // a link or heading wrapping one carries that markdown in its label. Its
                    // children never reach the converter's walk, so the subtree is skipped here
                    // too rather than scanned as page markup.
                    case "svg" when !selfClose:
                    {
                        int svgEnd = FindElementEnd(html, tagStart, "svg");
                        string markup = html[tagStart..(svgEnd < 0 ? n : svgEnd)];
                        if (captureHeading != 0 || inAnchor)
                        {
                            string rendered = HtmlToMarkdown.RenderSvgImage(markup);
                            if (captureHeading != 0) headingText.Append(rendered);
                            else anchorText.Append(rendered);
                        }
                        pos = svgEnd < 0 ? n : svgEnd;
                        continue;
                    }
                    case "script":
                    {
                        string? type = ExtractAttrDecoded(attrsStr, "type");
                        var (close, after) = FindRawTextEnd(html, pos, "script");
                        string body = close < 0 ? html[pos..] : html[pos..close];
                        pos = close < 0 ? n : after;
                        if (type is not null && type.Contains("ld+json", StringComparison.OrdinalIgnoreCase))
                        {
                            string rawJson = body.Trim();
                            if (rawJson.Length > 0)
                                m.StructuredData.Add(new StructuredDatum
                                {
                                    DataType = "json-ld",
                                    RawJson = rawJson,
                                    SchemaType = ExtractSchemaType(rawJson),
                                });
                        }
                        continue; // pos already advanced past </script>
                    }
                    case "style":
                    {
                        var (close, after) = FindRawTextEnd(html, pos, "style");
                        pos = close < 0 ? n : after;
                        continue;
                    }
                    // `<template>` content is an inert, unrendered document fragment per the HTML
                    // spec, and `<noscript>` content only renders with scripting disabled, which a
                    // Markdown conversion never is. The converter skips both subtrees outright, so
                    // nothing inside them is part of the document — including the images and links
                    // this pass collects. Skipping them here is what keeps a tracking pixel in
                    // `<noscript>` out of the image list.
                    case "template":
                    case "noscript":
                    {
                        int end = FindElementEnd(html, tagStart, tag);
                        pos = end < 0 ? n : end;
                        continue;
                    }
                }

                if (!Void.Contains(tag) && !selfClose) domDepth++;
            }
            else
            {
                int lt = html.IndexOf('<', pos);
                if (lt < 0) lt = n;
                string text = html[pos..lt];
                if (tables.Count > 0 && !tables[^1].HasText
                    && HtmlWalker.DecodeEntities(text).Trim().Length > 0) tables[^1].HasText = true;
                if (captureHeading != 0) headingText.Append(HtmlWalker.DecodeEntities(text));
                else if (inAnchor)
                {
                    string decoded = HtmlWalker.DecodeEntities(text);
                    anchorText.Append(cellDepth > 0 ? HtmlToMarkdown.EscapeCellText(decoded) : decoded);
                    anchorRawText.Append(decoded);
                }
                // Read as written. Only a document that reaches the walk through the html5ever
                // repair arrives with its references resolved, and there the serializer's own
                // spelling is what the collector sees — `&deg;` becomes `°`, `&nbsp;` stays.
                else if (inTitle) titleText.Append(canonicalAttrs ? HtmlToMarkdown.CanonicalizeText(text) : text);
                pos = lt;
            }
        }
        // An unclosed table still had its subtree walked once per pass.
        while (tables.Count > 0) CloseTable();
        ApplyHeadMetadata(m, headMetadata);
        return m;
    }

    /// <summary>
    /// Locate the end of a raw-text element's body: the offset where <c>&lt;/name</c> starts, and
    /// the offset just past the tag's <c>&gt;</c>.
    /// </summary>
    /// <remarks>
    /// The close tag is not always spelled <c>&lt;/style&gt;</c> — whitespace is allowed before
    /// the bracket, and pages in the corpus write <c>&lt;/style\n&gt;</c>. Matching the literal
    /// swallowed the rest of the document, taking every heading and link after it with it.
    /// </remarks>
    private static (int Start, int After) FindRawTextEnd(string html, int from, string name)
    {
        string needle = "</" + name;
        int search = from;
        while (true)
        {
            int close = html.IndexOf(needle, search, StringComparison.OrdinalIgnoreCase);
            if (close < 0) return (-1, html.Length);
            int k = close + needle.Length;
            while (k < html.Length && char.IsWhiteSpace(html[k])) k++;
            if (k < html.Length && html[k] == '>') return (close, k + 1);
            search = close + 1;
        }
    }

    /// <summary>
    /// Close an emphasis span, leaving its whitespace outside the delimiters.
    /// </summary>
    /// <remarks>
    /// <c>&lt;b&gt;label &lt;/b&gt;</c> is <c>**label** </c>, not <c>**label **</c> — a delimiter
    /// with a space against its inner edge is not emphasis at all in CommonMark, so the
    /// converter chomps the span and re-emits the whitespace outside. The recorded markdown has
    /// to match what the converter wrote.
    /// </remarks>
    private static void CloseInlineMarker(
        List<(StringBuilder Buffer, string Marker, int At)> open, StringBuilder target, string marker)
    {
        int idx = open.FindLastIndex(o => ReferenceEquals(o.Buffer, target) && o.Marker == marker);
        if (idx < 0) { target.Append(marker); return; }
        var (_, openMarker, at) = open[idx];
        open.RemoveAt(idx);

        int contentStart = at + openMarker.Length;
        if (contentStart > target.Length) { target.Append(marker); return; }
        string content = target.ToString(contentStart, target.Length - contentStart);
        string trimmed = content.Trim();
        if (trimmed.Length == 0)
        {
            // Nothing but whitespace between the delimiters: neither delimiter survives.
            target.Length = at;
            target.Append(content);
            return;
        }
        string lead = content[..(content.Length - content.TrimStart().Length)];
        string trail = content[content.TrimEnd().Length..];
        target.Length = at;
        target.Append(lead).Append(openMarker).Append(trimmed).Append(marker).Append(trail);
    }

    /// <summary>The markdown delimiter an inline element is written with, or null.</summary>
    private static string? InlineMarker(string tag) => tag switch
    {
        "em" or "i" or "var" or "dfn" or "cite" => "*",
        "strong" or "b" => "**",
        "code" => "`",
        "del" or "s" or "strike" => "~~",
        "mark" or "ins" => "==",
        _ => null,
    };

    /// <summary>The `dir` attribute's value, or null when it is not one the spec defines.</summary>
    private static TextDirection? ParseTextDirection(string value) => value.Trim().ToLowerInvariant() switch
    {
        "ltr" => TextDirection.LeftToRight,
        "rtl" => TextDirection.RightToLeft,
        "auto" => TextDirection.Auto,
        _ => null,
    };

    /// <summary>
    /// Interpret the collected head entries. Ports <c>extract_document_metadata</c>: a key is
    /// stripped of its <c>meta-</c> prefix, has any colon rewritten to a hyphen, and is matched
    /// case-insensitively; anything unrecognised is kept as a meta tag under the key as spelled.
    /// </summary>
    private static void ApplyHeadMetadata(HtmlMetadata m, SortedDictionary<string, string> head)
    {
        foreach (var (rawKey, value) in head)
        {
            string key = rawKey.StartsWith("meta-", StringComparison.Ordinal) ? rawKey[5..] : rawKey;
            string? replacedKey = key.Contains(':', StringComparison.Ordinal) ? key.Replace(':', '-') : null;
            if (replacedKey is not null) key = replacedKey;
            string lower = key.ToLowerInvariant();

            switch (lower)
            {
                case "title": m.Title = value; continue;
                case "description": m.Description = value; continue;
                case "author" or "creator" or "publisher": m.Author ??= value; continue;
                case "canonical": m.CanonicalUrl = value; continue;
                case "base" or "base-href": m.BaseHref = value; continue;
                case "keywords" or "news_keywords" or "citation_keywords" or "subject" or "topic"
                    or "category" or "classification":
                    if (m.Keywords.Count == 0) m.Keywords.AddRange(SplitKeywords(value));
                    continue;
            }

            if (lower.StartsWith("og-", StringComparison.Ordinal))
            {
                m.OpenGraph[lower[3..].Replace('-', '_')] = value;
                continue;
            }
            if (lower.StartsWith("twitter-", StringComparison.Ordinal))
            {
                m.TwitterCard[lower["twitter-".Length..].Replace('-', '_')] = value;
                continue;
            }
            if (DublinCorePrefix(lower) is { } dc)
            {
                string field = lower[dc.Length..];
                switch (field)
                {
                    case "title" or "alternative": m.Title ??= value; continue;
                    case "description" or "abstract": m.Description ??= value; continue;
                    case "creator" or "contributor" or "publisher": m.Author ??= value; continue;
                    case "subject" or "keywords":
                        if (m.Keywords.Count == 0) m.Keywords.AddRange(SplitKeywords(value));
                        continue;
                    default:
                        m.MetaTags[dc.TrimEnd('.', '-').Replace('.', '_') + "_" + field.Replace('-', '_')] = value;
                        continue;
                }
            }

            m.MetaTags[replacedKey ?? key] = value;
        }
    }

    /// <summary>The Dublin Core prefix a key carries, including its separator, or null.</summary>
    private static string? DublinCorePrefix(string lower)
    {
        foreach (string prefix in (ReadOnlySpan<string>)["dcterms.", "dcterms-", "dc.", "dc-"])
            if (lower.StartsWith(prefix, StringComparison.Ordinal)) return prefix;
        return null;
    }

    private static IEnumerable<string> SplitKeywords(string value) =>
        value.Split(',').Select(k => k.Trim()).Where(k => k.Length > 0);

    /// <summary>
    /// One open <c>&lt;table&gt;</c>: what the collector recorded inside it, and the structural
    /// facts `scan_table` reads to decide how the handler will render it. The scan covers the
    /// whole subtree, nested tables included, which is why a frame absorbs its children's counts.
    /// </summary>
    private sealed class TableFrame
    {
        /// <summary>Where a run of records came from, which decides the passes it is replayed for.</summary>
        public enum Origin { Cell, Caption, Child }

        /// <summary>Set when the handler renders this table as a list of its rows.</summary>
        public bool Layout;

        public readonly List<(Origin From, List<object> Items)> Headers = new(), Links = new(), Images = new();

        /// <summary>Set while the scan is inside this table's <c>&lt;caption&gt;</c>.</summary>
        public int CaptionDepth;

        private readonly List<int> _rowCounts = new();
        private int _openRowCells = -1;
        public bool HasSpan, HasHeader, HasCaption, HasText, BorderZero;
        public int NestedTables, LinkCount;

        /// <summary>The list new records go into: the trailing run of this table's own items.</summary>
        public List<object> OwnSegment(List<(Origin From, List<object> Items)> segments)
        {
            Origin origin = CaptionDepth > 0 ? Origin.Caption : Origin.Cell;
            if (segments.Count == 0 || segments[^1].From != origin) segments.Add((origin, new List<object>()));
            return segments[^1].Items;
        }

        public void OpenRow()
        {
            CloseRow();
            _openRowCells = 0;
        }

        public void CloseRow()
        {
            if (_openRowCells < 0) return;
            _rowCounts.Add(_openRowCells);
            _openRowCells = -1;
        }

        public void AddCell(int colspan)
        {
            if (_openRowCells >= 0) _openRowCells += colspan;
        }

        /// <summary>Roll a closed nested table's scan into this one.</summary>
        public void Absorb(TableFrame child)
        {
            child.CloseRow();
            _rowCounts.AddRange(child._rowCounts);
            NestedTables += child.NestedTables + 1;
            LinkCount += child.LinkCount;
            HasSpan |= child.HasSpan;
            HasHeader |= child.HasHeader;
            HasCaption |= child.HasCaption;
            HasText |= child.HasText;
        }

        /// <summary>
        /// How many times a walk that records reaches this table's cells.
        /// </summary>
        /// <remarks>
        /// The handler still walks a table up to three times — a column-width pre-pass, the
        /// render, and the grid the structure collector wants — but since 3.11.0 only one of
        /// those walks carries the collectors. The pre-pass detaches them (its measurement is an
        /// internal detail and must not be visible in the result) and so does the grid walk
        /// (which runs after the render has already recorded the same cells), leaving the render
        /// as the single recording pass. Upstream keeps the pre-pass's handles instead when it
        /// can reuse the pre-pass's markdown verbatim, but that requires no structure collector,
        /// and this port's options always install one — so it is the render either way, and one
        /// walk either way.
        /// </remarks>
        public int Passes => 1;

        /// <summary>
        /// This table's records, once per pass, in table order. The width pre-pass — which only
        /// a three-pass table has — never enters a nested table, so nothing a nested table
        /// contributed is replayed for it.
        /// </summary>
        public List<object> Replay(List<(Origin From, List<object> Items)> segments, int passes)
        {
            // The render is the only pass that records, and it walks the caption as well as the
            // rows, so every segment is replayed exactly `passes` times in table order.
            var result = new List<object>();
            for (int pass = 0; pass < passes; pass++)
                foreach (var (_, items) in segments)
                    result.AddRange(items);
            return result;
        }
    }

    /// <summary>
    /// The label a link records. A caret-only label on a fragment link is the citation
    /// backlink Wikipedia writes; the link handler rewrites it to an arrow before the
    /// metadata collector sees it, so the recorded text carries the arrow too.
    /// </summary>
    private static string CiteBacklinkLabel(string label, string href)
        => label == "^" && href.StartsWith('#') ? "\u2191" : label;

    // Mirrors html-to-markdown's LinkMetadata::classify_link (metadata/types.rs). Note the
    // scheme checks are case-sensitive there, and "//"/"/" both classify as internal.
    private static string ClassifyLink(string href)
    {
        if (href.StartsWith('#')) return "anchor";
        if (href.StartsWith("mailto:", StringComparison.Ordinal)) return "email";
        if (href.StartsWith("tel:", StringComparison.Ordinal)) return "phone";
        if (href.StartsWith("http://", StringComparison.Ordinal) ||
            href.StartsWith("https://", StringComparison.Ordinal)) return "external";
        if (href.StartsWith('/') || href.StartsWith("../", StringComparison.Ordinal) ||
            href.StartsWith("./", StringComparison.Ordinal)) return "internal";
        return "other";
    }

    /// <summary>
    /// An image source is external only when it names its own scheme. A protocol-relative
    /// <c>//host/path</c> inherits the page's, which makes it relative.
    /// </summary>
    private static string ClassifyImage(string src)
    {
        if (src.StartsWith("data:", StringComparison.Ordinal)) return "data-uri";
        if (src.StartsWith("http://", StringComparison.Ordinal)
            || src.StartsWith("https://", StringComparison.Ordinal)) return "external";
        if (src.StartsWith('<') && src.Contains("svg", StringComparison.Ordinal)) return "inline-svg";
        return "relative";
    }

    private static string? ExtractSchemaType(string json)
    {
        int idx = json.IndexOf("\"@type\"", StringComparison.Ordinal);
        if (idx < 0) return null;
        int colon = json.IndexOf(':', idx);
        if (colon < 0) return null;
        int q1 = json.IndexOf('"', colon);
        if (q1 < 0) return null;
        int q2 = json.IndexOf('"', q1 + 1);
        if (q2 < 0) return null;
        return json[(q1 + 1)..q2];
    }

    private static string? ExtractAttrDecoded(string attrs, string name)
    {
        var v = HtmlWalker.ExtractAttr(attrs, name);
        return v is null ? null : HtmlWalker.DecodeEntities(v);
    }

    /// <summary>
    /// Index just past the close tag matching the element that starts at <paramref name="start"/>,
    /// counting nested opens of the same name; -1 when it is never closed.
    /// </summary>
    private static int FindElementEnd(string html, int start, string name)
    {
        int depth = 0;
        int i = start;
        while (i < html.Length)
        {
            int lt = html.IndexOf('<', i);
            if (lt < 0) return -1;
            int gt = html.IndexOf('>', lt);
            if (gt < 0) return -1;
            string inner = html[(lt + 1)..gt];
            bool closing = inner.StartsWith('/');
            var (tagName, _) = HtmlWalker.SplitTagName(closing ? inner[1..] : inner.TrimEnd('/').Trim());
            if (tagName.Equals(name, StringComparison.OrdinalIgnoreCase))
            {
                if (closing)
                {
                    if (--depth == 0) return gt + 1;
                }
                else if (!inner.TrimEnd().EndsWith('/'))
                {
                    depth++;
                }
            }
            i = gt + 1;
        }
        return -1;
    }

    private static List<string[]> ParseAttrs(string attrs, string exclude, bool canonical, out List<string> rel)
    {
        rel = new List<string>();
        var result = new List<string[]>();
        int i = 0, n = attrs.Length;
        while (i < n)
        {
            while (i < n && char.IsWhiteSpace(attrs[i])) i++;
            if (i >= n) break;
            int ks = i;
            while (i < n && attrs[i] != '=' && !char.IsWhiteSpace(attrs[i]) && attrs[i] != '>') i++;
            string key = HtmlWalker.TrimAttributeNameStart(attrs[ks..i]);
            if (key.Length == 0) { i++; continue; }
            // Recorded lower-case: the parser upstream collects from stores names folded, so
            // `<IMG ALIGN=…>` records `align`. That is only the record — the converter's own
            // attribute lookups stay case-sensitive, which is why `<A HREF=…>` still has no href.
            key = key.ToLowerInvariant();
            while (i < n && char.IsWhiteSpace(attrs[i])) i++;
            string value = "";
            if (i < n && attrs[i] == '=')
            {
                i++;
                while (i < n && char.IsWhiteSpace(attrs[i])) i++;
                if (i < n && (attrs[i] == '"' || attrs[i] == '\''))
                {
                    char q = attrs[i++];
                    int vs = i;
                    while (i < n && attrs[i] != q) i++;
                    value = attrs[vs..i];
                    if (i < n) i++;
                }
                else
                {
                    int vs = i;
                    while (i < n && !char.IsWhiteSpace(attrs[i]) && attrs[i] != '>') i++;
                    value = attrs[vs..i];
                }
            }
            // Attribute values are recorded as written. The collector reads the attribute; it
            // does not resolve it, so `alt="\lambda&gt;0"` keeps its reference.
            if (canonical) value = HtmlToMarkdown.CanonicalizeAttrValue(value);
            if (key == "rel")
                rel.AddRange(value.Split(' ', StringSplitOptions.RemoveEmptyEntries));
            if (!key.Equals(exclude, StringComparison.OrdinalIgnoreCase))
                result.Add(new[] { key, value });
        }
        // html-to-markdown collects attributes into a BTreeMap → alphabetical by key.
        result.Sort((a, b) => string.CompareOrdinal(a[0], b[0]));
        return result;
    }

    // Shapes for HtmlMetadata's List<object> collections. Serialized with the global
    // snake_case policy → level/text/depth/html_offset, link_type, image_type, etc.
    private sealed class Header
    {
        public int Level { get; set; }
        public string Text { get; set; } = "";

        /// <summary>The heading's `id` attribute, which is what an in-page link targets.</summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public string? Id { get; set; }

        public int Depth { get; set; }
        public int HtmlOffset { get; set; }
    }

    private sealed class Link
    {
        public string Href { get; set; } = "";
        public string Text { get; set; } = "";
        [System.Text.Json.Serialization.JsonIgnore(Condition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull)]
        public string? Title { get; set; }
        public string LinkType { get; set; } = "other";
        public List<string> Rel { get; set; } = new();
        public List<string[]> Attributes { get; set; } = new();
    }

    private sealed class Img
    {
        public string Src { get; set; } = "";

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public string? Alt { get; set; }

        [JsonIgnore(Condition = JsonIgnoreCondition.WhenWritingNull)]
        public string? Title { get; set; }

        /// <summary>
        /// Always serialised, as null when the element states no size — the collector's own
        /// field carries no skip, so a sizeless image still names the key.
        /// </summary>
        [JsonIgnore(Condition = JsonIgnoreCondition.Never)]
        public uint[]? Dimensions { get; set; }

        public string ImageType { get; set; } = "external";
        public List<string[]> Attributes { get; set; } = new();
    }

    private sealed class StructuredDatum
    {
        public string DataType { get; set; } = "json-ld";
        public string RawJson { get; set; } = "";
        public string? SchemaType { get; set; }
    }
}
