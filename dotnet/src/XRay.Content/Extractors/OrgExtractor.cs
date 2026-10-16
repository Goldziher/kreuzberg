using System.Text;
using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Internal.Markup;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// Org Mode extractor. Ported from Rust `extractors/orgmode.rs`. The InternalDocument is built
/// by a self-contained line parser; metadata is derived from preamble `#+` directives; tables
/// are collected by scanning pipe-table runs (mirroring the org-tree walk).
/// </summary>
public sealed class OrgExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "text/x-org", "text/org", "application/x-org" };
    public int Priority => 50;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        // Upstream's one-line growth guard: the input's own size is charged against
        // `max_content_size` before parsing, so a document too large to render is refused
        // rather than rendered and then found to be too large.
        SecurityBudget.FromConfig(config).AccountText(content.Length);
        string text = Encoding.UTF8.GetString(content);
        var metadata = ExtractMetadata(text);

        // Tables are parsed in place while the document is built, which produces table elements
        // positioned where the table sits. A second raw pass pushed each of them again, so every
        // org table was reported twice and neither copy was the one the renderers referenced.
        var doc = BuildInternalDocument(text);
        doc.MimeType = mimeType;
        doc.Metadata = metadata;
        return doc;
    }

    // ── metadata ──────────────────────────────────────────────────────────

    private static Metadata ExtractMetadata(string orgText)
    {
        var meta = new Metadata();
        var additional = new Dictionary<string, string>();
        string? title = null;
        List<string>? authors = null;
        List<string>? keywords = null;

        var lines = MarkupHelpers.Lines(orgText);
        foreach (var line in lines.Take(100))
        {
            string trimmed = line.Trim();
            if (trimmed.StartsWith("#+TITLE:")) title = trimmed.Substring("#+TITLE:".Length).Trim();
            else if (trimmed.StartsWith("#+AUTHOR:")) authors = new List<string> { trimmed.Substring("#+AUTHOR:".Length).Trim() };
            else if (trimmed.StartsWith("#+DATE:")) { string v = trimmed.Substring("#+DATE:".Length).Trim(); meta.CreatedAt = v; additional["date"] = v; }
            else if (trimmed.StartsWith("#+KEYWORDS:")) keywords = trimmed.Substring("#+KEYWORDS:".Length).Split(',').Select(s => s.Trim()).ToList();
            else if (trimmed.StartsWith("#+"))
            {
                string rest = trimmed.Substring(2);
                int c = rest.IndexOf(':');
                if (c >= 0)
                {
                    string key = rest.Substring(0, c).Trim().ToLowerInvariant();
                    string value = rest.Substring(c + 1).Trim();
                    if (key.Length != 0 && value.Length != 0) additional[$"directive_{key}"] = value;
                }
            }
        }

        meta.Title = title;
        meta.Authors = authors;
        meta.Keywords = keywords;
        foreach (var (k, v) in additional) meta.Additional[k] = JsonSerializer.SerializeToElement(v, Json.Options);
        return meta;
    }

    // ── tables (mirrors extract_tables_from_tree over the whole doc) ─────────

    private static List<Table> ExtractTables(string orgText)
    {
        var tables = new List<Table>();
        var lines = MarkupHelpers.Lines(orgText);
        bool inTable = false;
        var current = new List<List<string>>();
        foreach (var line in lines)
        {
            string t = line.Trim();
            if (t.StartsWith('|') && t.EndsWith('|'))
            {
                inTable = true;
                var cells = t.Split('|').Select(c => c.Trim()).Where(c => c.Length != 0).ToList();
                if (cells.Count > 0) current.Add(cells);
            }
            else if (inTable)
            {
                if (current.Count > 0)
                {
                    tables.Add(new Table { Cells = current.Select(r => new List<string>(r)).ToList(), Markdown = "", PageNumber = 1 });
                    current = new List<List<string>>();
                }
                inTable = false;
            }
        }
        if (current.Count > 0)
            tables.Add(new Table { Cells = current, Markdown = "", PageNumber = 1 });
        return tables;
    }

    // ── build ───────────────────────────────────────────────────────────────

    private static InternalDocument BuildInternalDocument(string orgText)
    {
        var b = new InternalDocumentBuilder("orgmode");
        var lines = MarkupHelpers.Lines(orgText);
        int i = 0;

        var metaEntries = new List<(string, string)>();
        while (i < lines.Count)
        {
            Cancel.ThrowIfRequested();
            string trimmed = lines[i].Trim();
            if (trimmed.StartsWith("#+"))
            {
                string rest = trimmed.Substring(2);
                string ru = rest.ToUpperInvariant();
                if (ru.StartsWith("BEGIN") || ru.StartsWith("END")) break;
                int c = rest.IndexOf(':');
                if (c >= 0)
                {
                    string key = rest.Substring(0, c).Trim().ToUpperInvariant();
                    string value = rest.Substring(c + 1).Trim();
                    if (value.Length != 0) metaEntries.Add((key, value));
                }
                i++;
                continue;
            }
            if (trimmed.Length != 0) break;
            i++;
        }
        if (metaEntries.Count > 0) b.PushMetadataBlock(metaEntries, null);

        while (i < lines.Count)
        {
            Cancel.ThrowIfRequested();
            string trimmed = lines[i].Trim();

            if (trimmed.StartsWith("#+") && !trimmed.StartsWith("#+BEGIN") && !trimmed.StartsWith("#+begin")
                && !trimmed.StartsWith("#+END") && !trimmed.StartsWith("#+end")) { i++; continue; }

            if (trimmed == ":PROPERTIES:")
            {
                var props = new List<(string, string)>();
                i++;
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string pt = lines[i].Trim();
                    if (pt == ":END:") { i++; break; }
                    if (pt.StartsWith(':') && pt.Length > 1)
                    {
                        int c2 = pt.Substring(1).IndexOf(':');
                        if (c2 >= 0)
                        {
                            string key = pt.Substring(1, c2);
                            string value = pt.Substring(2 + c2).Trim();
                            if (key.Length != 0) props.Add((key, value));
                        }
                    }
                    i++;
                }
                if (props.Count > 0) b.PushMetadataBlock(props, null);
                continue;
            }

            if (trimmed.StartsWith('*'))
            {
                int level = 0;
                foreach (char ch in trimmed) { if (ch == '*') level++; else break; }
                if (level > 0 && trimmed.Length > level && trimmed[level] == ' ')
                {
                    string rawHeading = trimmed.Substring(level + 1).Trim();
                    if (rawHeading.Length != 0)
                    {
                        string headingText = rawHeading;
                        foreach (var kw in new[] { "TODO", "DONE", "NEXT", "WAITING", "CANCELLED", "CANCELED" })
                        {
                            if (headingText.StartsWith(kw))
                            {
                                string after = headingText.Substring(kw.Length);
                                if (after.Length == 0 || after.StartsWith(' ')) { headingText = after.TrimStart(); break; }
                            }
                        }
                        int tagStart = headingText.LastIndexOf(" :", StringComparison.Ordinal);
                        if (tagStart >= 0)
                        {
                            string potentialTags = headingText.Substring(tagStart + 1);
                            if (potentialTags.EndsWith(':') && potentialTags.Length > 2)
                                headingText = headingText.Substring(0, tagStart).TrimEnd();
                        }
                        b.PushHeading((byte)level, headingText, null, null);
                    }
                    i++;
                    continue;
                }
            }

            if (trimmed.StartsWith("#+BEGIN_SRC") || trimmed.StartsWith("#+begin_src"))
            {
                var parts = trimmed.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
                string? language = parts.Length > 1 ? parts[1] : null;
                i++;
                var code = new StringBuilder();
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string t = lines[i].Trim();
                    if (t.StartsWith("#+END_SRC") || t.StartsWith("#+end_src")) { i++; break; }
                    if (code.Length != 0) code.Append('\n');
                    code.Append(lines[i]);
                    i++;
                }
                b.PushCode(code.ToString().TrimEnd(), language, null, null);
                continue;
            }

            if (trimmed.StartsWith("#+BEGIN_QUOTE") || trimmed.StartsWith("#+begin_quote"))
            {
                b.PushQuoteStart();
                i++;
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string t = lines[i].Trim();
                    if (t.StartsWith("#+END_QUOTE") || t.StartsWith("#+end_quote")) { i++; break; }
                    if (t.Length != 0)
                    {
                        // A quote keeps its lines separate, so only math that fits on one line
                        // leaves the text here.
                        var (lineText, quoteMath) = SplitDisplayMath(t);
                        PushDisplayMath(b, quoteMath);
                        if (lineText.Trim().Length != 0) b.PushParagraph(lineText, new(), null, null);
                    }
                    i++;
                }
                b.PushQuoteEnd();
                continue;
            }

            if (trimmed.StartsWith("#+BEGIN_EXAMPLE") || trimmed.StartsWith("#+begin_example"))
            {
                i++;
                var block = new StringBuilder();
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string t = lines[i].Trim();
                    if (t.StartsWith("#+END_EXAMPLE") || t.StartsWith("#+end_example")) { i++; break; }
                    if (block.Length != 0) block.Append('\n');
                    block.Append(lines[i]);
                    i++;
                }
                b.PushCode(block.ToString().TrimEnd(), null, null, null);
                continue;
            }

            if (trimmed.StartsWith("#+BEGIN_") || trimmed.StartsWith("#+begin_"))
            {
                string first = trimmed.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).FirstOrDefault() ?? "";
                string blockType = first.StartsWith("#+BEGIN_") ? first.Substring("#+BEGIN_".Length)
                    : first.StartsWith("#+begin_") ? first.Substring("#+begin_".Length) : "UNKNOWN";
                if (blockType.Length == 0) blockType = "UNKNOWN";
                string endUpper = $"#+END_{blockType}";
                string endLower = endUpper.ToLowerInvariant();
                i++;
                var block = new StringBuilder();
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string t = lines[i].Trim();
                    if (t.StartsWith(endUpper) || t.StartsWith(endLower)) { i++; break; }
                    if (block.Length != 0) block.Append('\n');
                    block.Append(lines[i]);
                    i++;
                }
                b.PushRawBlock("orgmode", block.ToString().TrimEnd(), null);
                continue;
            }

            if (trimmed.StartsWith('|') && trimmed.EndsWith('|'))
            {
                var tableCells = new List<List<string>>();
                // An org table only has a header if the source drew a rule under its first row.
                // A rule before any row is decoration, not a header separator.
                bool hasHeaderSeparator = false;
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string t = lines[i].Trim();
                    if (!t.StartsWith('|') || !t.EndsWith('|')) break;
                    if (IsOrgTableHorizontalLine(t))
                    {
                        hasHeaderSeparator |= tableCells.Count > 0;
                        i++;
                        continue;
                    }
                    var cells = t.Split('|').Select(c => c.Trim()).Where(c => c.Length != 0).ToList();
                    if (cells.Count > 0) tableCells.Add(cells);
                    i++;
                }
                if (tableCells.Count > 0) PushOrgTable(b, tableCells, hasHeaderSeparator);
                continue;
            }

            if (trimmed.Length != 0 && IsOrgListItem(trimmed))
            {
                bool isOrdered = IsOrgOrderedItem(trimmed);
                b.PushList(isOrdered);
                while (i < lines.Count)
                {
                    Cancel.ThrowIfRequested();
                    string t = lines[i].Trim();
                    if (t.Length == 0) break;
                    if (IsOrgListItem(t))
                    {
                        string itemText = StripListPrefix(t);
                        var parts = new List<string> { itemText };
                        i++;
                        while (i < lines.Count)
                        {
                            Cancel.ThrowIfRequested();
                            string rawNext = lines[i];
                            string nextT = rawNext.Trim();
                            if (nextT.Length == 0 || IsOrgListItem(nextT) || IsStructuralStart(nextT)) break;
                            if (rawNext.StartsWith(" ") || rawNext.StartsWith("\t")) { parts.Add(nextT); i++; }
                            else break;
                        }
                        var (joinedItem, itemMath) = SplitDisplayMath(string.Join(" ", parts));
                        PushDisplayMath(b, itemMath);
                        if (joinedItem.Trim().Length != 0)
                            b.PushListItem(joinedItem, isOrdered, new(), null, null);
                    }
                    else break;
                }
                b.EndList();
                continue;
            }

            if (trimmed.StartsWith("[fn:"))
            {
                int close = trimmed.IndexOf(']');
                if (close >= 0)
                {
                    string name = trimmed.Substring(4, close - 4);
                    if (name.Length != 0)
                    {
                        string defText = trimmed.Substring(close + 1).Trim();
                        if (defText.Length != 0) b.PushFootnoteDefinition(defText, name, null);
                    }
                }
                i++;
                continue;
            }

            if (trimmed.Length != 0)
            {
                var link = ParseOrgLink(trimmed, 0);
                if (link is not null && link.Value.consumedTo == Encoding.UTF8.GetByteCount(trimmed) && IsImageUrl(link.Value.url))
                {
                    string alt = link.Value.display == link.Value.url ? "" : link.Value.display;
                    var kind = ElementKind.Image(uint.MaxValue);
                    var elem = new InternalElement
                    {
                        Id = InternalElementId.Generate(kind.Discriminant(), alt, null, 0),
                        Kind = kind,
                        Text = alt,
                        Depth = 0,
                        Layer = ContentLayer.Body,
                        Annotations = new(),
                    };
                    b.PushElement(elem);
                    string? label = link.Value.display == link.Value.url ? null : link.Value.display;
                    b.PushUri(MarkupHelpers.Image(link.Value.url, label));
                    i++;
                    continue;
                }

                var paraRaw = new List<string> { trimmed };
                int next = i + 1;
                while (next < lines.Count)
                {
                    string nextTrimmed = lines[next].Trim();
                    if (nextTrimmed.Length == 0 || IsStructuralStart(nextTrimmed)) break;
                    var nl = ParseOrgLink(nextTrimmed, 0);
                    if (nl is not null && nl.Value.consumedTo == Encoding.UTF8.GetByteCount(nextTrimmed) && IsImageUrl(nl.Value.url)) break;
                    paraRaw.Add(nextTrimmed);
                    next++;
                }

                string joinedRaw = string.Join(" ", paraRaw);
                // Math leaves the text before the markup parser runs: Org markup characters
                // (`_`, `/`, `=`) also occur inside LaTeX.
                var (joinedWithoutMath, paraMath) = SplitDisplayMath(joinedRaw);
                PushDisplayMath(b, paraMath);
                if (joinedWithoutMath.Trim().Length == 0) { i = next; continue; }
                joinedRaw = joinedWithoutMath;

                var footnoteRefs = FindFootnoteReferences(joinedRaw);
                var (stripped, annotations) = ParseInlineMarkup(joinedRaw);
                byte[] strippedBytes = Encoding.UTF8.GetBytes(stripped);

                foreach (var ann in annotations)
                {
                    if (ann.Kind.Which != AnnotationKind.Tag.Link) continue;
                    string url = ann.Kind.Url ?? "";
                    if (url.Length == 0) continue;
                    string? label = null;
                    if (ann.End <= (uint)strippedBytes.Length && ann.Start <= ann.End)
                        label = Encoding.UTF8.GetString(strippedBytes, (int)ann.Start, (int)(ann.End - ann.Start));
                    bool isImage = url.EndsWith(".png") || url.EndsWith(".jpg") || url.EndsWith(".jpeg")
                        || url.EndsWith(".gif") || url.EndsWith(".svg")
                        || (url.StartsWith("file:") && label is not null && (label.EndsWith(".png") || label.EndsWith(".jpg") || label.EndsWith(".jpeg")));
                    if (isImage) b.PushUri(MarkupHelpers.Image(url, label));
                    else b.PushUri(MarkupHelpers.Hyperlink(url, label));
                }

                uint idx = b.PushParagraph(stripped, annotations, null, null);
                foreach (var fref in footnoteRefs) b.PushFootnoteRef($"[fn:{fref}]", fref, null);
                ExtractInternalLinks(joinedRaw, idx, b);

                i = next;
                continue;
            }

            i++;
        }
        return b.Build();
    }

    private static void ExtractInternalLinks(string line, uint sourceIdx, InternalDocumentBuilder b)
    {
        int searchFrom = 0;
        while (true)
        {
            int pos = line.IndexOf("[[", searchFrom, StringComparison.Ordinal);
            if (pos < 0) break;
            string after = line.Substring(pos + 2);
            int? close;
            int descStart = after.IndexOf("][", StringComparison.Ordinal);
            if (descStart >= 0)
            {
                int c = after.Substring(descStart + 2).IndexOf("]]", StringComparison.Ordinal);
                close = c >= 0 ? descStart + 2 + c + 2 : (int?)null;
            }
            else
            {
                int c = after.IndexOf("]]", StringComparison.Ordinal);
                close = c >= 0 ? c + 2 : (int?)null;
            }
            if (close is int consumed)
            {
                string linkContent = after.Substring(0, consumed - 2);
                int sep = linkContent.IndexOf("][", StringComparison.Ordinal);
                string urlPart = sep >= 0 ? linkContent.Substring(0, sep) : linkContent;
                if (urlPart.StartsWith('#'))
                    b.PushRelationship(sourceIdx, RelationshipTarget.FromKey(urlPart.Substring(1)), RelationshipKind.InternalLink);
                else if (urlPart.StartsWith('*'))
                    b.PushRelationship(sourceIdx, RelationshipTarget.FromKey(urlPart.Substring(1)), RelationshipKind.InternalLink);
                searchFrom = pos + 2 + consumed;
            }
            else break;
        }
    }

    private static bool IsStructuralStart(string trimmed)
    {
        if (trimmed.StartsWith('*'))
        {
            int level = 0;
            foreach (char ch in trimmed) { if (ch == '*') level++; else break; }
            if (level > 0 && trimmed.Length > level && trimmed[level] == ' ') return true;
        }
        if (trimmed.StartsWith("#+BEGIN") || trimmed.StartsWith("#+begin") || trimmed.StartsWith("#+END") || trimmed.StartsWith("#+end")) return true;
        if (trimmed.StartsWith("#+")) return true;
        if (trimmed == ":PROPERTIES:" || trimmed == ":END:") return true;
        if (trimmed.StartsWith('|') && trimmed.EndsWith('|')) return true;
        if (IsOrgListItem(trimmed)) return true;
        if (trimmed.StartsWith("[fn:")) return true;
        return false;
    }

    /// <summary>A rule line: pipes, dashes and column separators, and nothing else.</summary>
    private static bool IsOrgTableHorizontalLine(string line)
    {
        string t = line.Trim();
        if (!t.StartsWith('|') || !t.EndsWith('|') || t.Length < 2) return false;
        string inner = t[1..^1];
        foreach (string segment in inner.Split('+'))
        {
            if (segment.Length == 0) return false;
            foreach (char c in segment) if (c != '-') return false;
        }
        return true;
    }

    /// <summary>
    /// Push an org table, recording whether the source declared a header row.
    /// </summary>
    /// <remarks>
    /// A table whose first row is data has no header, and rendering it as one would relabel that
    /// record as the column names. The renderer reads that from whether Columns is set.
    /// </remarks>
    private static void PushOrgTable(InternalDocumentBuilder b, List<List<string>> cells, bool hasHeader)
    {
        // The table's own markdown gets an empty header row when the source declared none, since
        // a pipe table always has one and the alternative is promoting a data row into it.
        var markdownCells = cells.Select(r => new List<string>(r)).ToList();
        if (!hasHeader)
        {
            int columnCount = cells.Count == 0 ? 0 : cells.Max(r => r.Count);
            markdownCells.Insert(0, Enumerable.Repeat("", columnCount).ToList());
        }

        b.PushTable(new Table
        {
            Cells = cells.Select(r => new List<string>(r)).ToList(),
            Markdown = InternalDocumentBuilder.CellsToMarkdown(markdownCells),
            PageNumber = 0,
            BoundingBox = null,
            Columns = hasHeader ? new List<string>(cells[0]) : null,
        }, null, null);
    }

    private static bool IsOrgListItem(string line)
    {
        string t = line.TrimStart();
        if (t.StartsWith("- ") || t.StartsWith("+ ")) return true;
        int sp = t.IndexOf(' ');
        if (sp > 0 && sp < 5)
        {
            string prefix = t.Substring(0, sp);
            if ((prefix.EndsWith('.') || prefix.EndsWith(')')) && prefix.Substring(0, prefix.Length - 1).All(char.IsDigit))
                return true;
        }
        return false;
    }

    private static bool IsOrgOrderedItem(string line)
    {
        string t = line.TrimStart();
        int sp = t.IndexOf(' ');
        if (sp > 0 && sp < 5)
        {
            string prefix = t.Substring(0, sp);
            return (prefix.EndsWith('.') || prefix.EndsWith(')')) && prefix.Substring(0, prefix.Length - 1).All(char.IsDigit);
        }
        return false;
    }

    private static string StripListPrefix(string line)
    {
        string t = line.TrimStart();
        if (t.StartsWith("- ")) return t.Substring(2);
        if (t.StartsWith("+ ")) return t.Substring(2);
        int sp = t.IndexOf(' ');
        if (sp >= 0) return t.Substring(sp + 1);
        return t;
    }

    private static bool IsImageUrl(string url)
    {
        string path = (url.StartsWith("file:") ? url.Substring(5) : url).Split('?', '#')[0];
        string lower = path.ToLowerInvariant();
        return lower.EndsWith(".png") || lower.EndsWith(".jpg") || lower.EndsWith(".jpeg") || lower.EndsWith(".gif")
            || lower.EndsWith(".svg") || lower.EndsWith(".webp") || lower.EndsWith(".bmp") || lower.EndsWith(".tiff")
            || lower.EndsWith(".tif") || lower.EndsWith(".avif");
    }

    private static List<string> FindFootnoteReferences(string line)
    {
        var refs = new List<string>();
        int searchFrom = 0;
        while (true)
        {
            int pos = line.IndexOf("[fn:", searchFrom, StringComparison.Ordinal);
            if (pos < 0) break;
            int close = line.Substring(pos).IndexOf(']');
            if (close >= 0)
            {
                string label = line.Substring(pos + 4, (pos + close) - (pos + 4));
                if (label.Length != 0) refs.Add(label);
                searchFrom = pos + close + 1;
            }
            else break;
        }
        return refs;
    }

    // ── inline markup ─────────────────────────────────────────────────────

    private static (string url, string display, int consumedTo)? ParseOrgLink(string text, int start)
    {
        byte[] b = Encoding.UTF8.GetBytes(text);
        if (!SliceStartsWith(b, start, "[[")) return null;
        int afterOpen = start + 2;
        int descStart = FindSub(b, afterOpen, "][");
        if (descStart >= 0)
        {
            string url = Enc(b, afterOpen, descStart);
            int descBegin = descStart + 2;
            int close = FindSub(b, descBegin, "]]");
            if (close >= 0)
            {
                string description = Enc(b, descBegin, close);
                return (url, description, close + 2);
            }
        }
        else
        {
            int close = FindSub(b, afterOpen, "]]");
            if (close >= 0)
            {
                string url = Enc(b, afterOpen, close);
                return (url, url, close + 2);
            }
        }
        return null;
    }

    /// <summary>
    /// Pull display math out of a block of Org text.
    /// </summary>
    /// <returns>
    /// The text without its math, and the LaTeX of every fragment removed, in the order the
    /// fragments appeared.
    /// </returns>
    /// <remarks>
    /// Org writes display math as <c>\[…\]</c>, <c>$$…$$</c>, or a LaTeX math environment, and
    /// each becomes a formula element. Inline math (<c>\(…\)</c>, <c>$…$</c>) stays in the
    /// text, as it does for markdown: it belongs to the sentence around it.
    /// </remarks>
    private static (string Text, List<string> Formulas) SplitDisplayMath(string raw)
    {
        var formulas = new List<string>();
        var text = new StringBuilder();
        int pos = 0;
        while (true)
        {
            var hit = NextDisplayMath(raw, pos);
            if (hit is null) break;
            var (start, latex, resume) = hit.Value;
            text.Append(raw, pos, start - pos);
            formulas.Add(latex);
            pos = resume;
        }
        if (formulas.Count == 0) return (raw, formulas);
        text.Append(raw, pos, raw.Length - pos);

        return (string.Join(" ", text.ToString().Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries)), formulas);
    }

    /// <summary>
    /// Locate the first display-math fragment at or after <paramref name="from"/>: its start
    /// offset, its LaTeX, and the offset just past its closing delimiter. An unclosed fragment
    /// is not math — the text keeps it.
    /// </summary>
    private static (int Start, string Latex, int Resume)? NextDisplayMath(string text, int from)
    {
        int search = from;
        while (search < text.Length)
        {
            int start = text.IndexOfAny(DisplayMathStarts, search);
            if (start < 0) return null;

            if (StartsAt(text, start, "\\[") )
            {
                int end = text.IndexOf("\\]", start + 2, StringComparison.Ordinal);
                if (end >= 0)
                    return (start, text[(start + 2)..end].Trim(), end + 2);
            }
            else if (StartsAt(text, start, "$$"))
            {
                int end = text.IndexOf("$$", start + 2, StringComparison.Ordinal);
                if (end >= 0)
                    return (start, text[(start + 2)..end].Trim(), end + 2);
            }
            else if (StartsAt(text, start, "\\begin{"))
            {
                int nameEnd = start + 7 < text.Length ? text.IndexOf('}', start + 7) : -1;
                if (nameEnd >= 0)
                {
                    string name = text[(start + 7)..nameEnd];
                    if (IsMathEnvironment(name))
                    {
                        string closing = $"\\end{{{name}}}";
                        int end = text.IndexOf(closing, nameEnd + 1, StringComparison.Ordinal);
                        if (end >= 0)
                        {
                            string inner = text[(nameEnd + 1)..end];
                            return (start, $"\\begin{{{name}}}{inner}\\end{{{name}}}", end + closing.Length);
                        }
                    }
                }
            }

            search = start + 1;
        }
        return null;
    }

    private static readonly char[] DisplayMathStarts = { '\\', '$' };

    private static bool StartsAt(string text, int index, string value) =>
        index + value.Length <= text.Length
        && string.CompareOrdinal(text, index, value, 0, value.Length) == 0;

    /// <summary>The LaTeX environments whose body is display math.</summary>
    private static bool IsMathEnvironment(string name) => name switch
    {
        "equation" or "equation*" or "align" or "align*" or "gather" or "gather*"
            or "multline" or "multline*" or "eqnarray" or "eqnarray*" or "math"
            or "displaymath" or "flalign" or "flalign*" or "cases" => true,
        _ => false,
    };

    /// <summary>Emit one formula element per LaTeX fragment, in order.</summary>
    private static void PushDisplayMath(InternalDocumentBuilder b, List<string> formulas)
    {
        foreach (var latex in formulas)
            if (latex.Length != 0) b.PushFormula(latex, null, null);
    }

    private static (string, List<TextAnnotation>) ParseInlineMarkup(string raw)
    {
        byte[] b = Encoding.UTF8.GetBytes(raw);
        int len = b.Length;
        var outBuf = new Utf8Buf();
        var anns = new List<TextAnnotation>();
        int i = 0;

        while (i < len)
        {
            if (i + 1 < len && b[i] == (byte)'[' && b[i + 1] == (byte)'[')
            {
                var link = ParseOrgLinkBytes(b, i);
                if (link is not null)
                {
                    uint start = outBuf.Len;
                    outBuf.Append(link.Value.display);
                    uint end = outBuf.Len;
                    if (start < end) anns.Add(MarkupHelpers.Annotation(start, end, MarkupHelpers.Link(link.Value.url, null)));
                    i = link.Value.consumedTo;
                    continue;
                }
            }

            if (b[i] < 0x80 && IsOrgMarkupChar(b[i]))
            {
                byte marker = b[i];
                bool precededOk = i == 0 || IsAsciiWs(b[i - 1]) || b[i - 1] == (byte)'(' || b[i - 1] == (byte)'"';
                if (precededOk && i + 1 < len && !IsAsciiWs(b[i + 1]))
                {
                    int close = FindOrgMarkupClose(b, i + 1, marker);
                    if (close >= 0)
                    {
                        string inner = Enc(b, i + 1, close);
                        uint start = outBuf.Len;
                        outBuf.Append(inner);
                        uint end = outBuf.Len;
                        AnnotationKind kind = marker switch
                        {
                            (byte)'*' => MarkupHelpers.Bold,
                            (byte)'/' => MarkupHelpers.Italic,
                            (byte)'_' => MarkupHelpers.Underline,
                            (byte)'=' or (byte)'~' => MarkupHelpers.Code,
                            (byte)'+' => MarkupHelpers.Strikethrough,
                            _ => MarkupHelpers.Bold,
                        };
                        if (start < end) anns.Add(MarkupHelpers.Annotation(start, end, kind));
                        i = close + 1;
                        continue;
                    }
                }
            }

            int clen = Utf8CharLen(b, i);
            outBuf.Append(Enc(b, i, i + clen));
            i += clen;
        }
        return (outBuf.ToString(), anns);
    }

    private static (string url, string display, int consumedTo)? ParseOrgLinkBytes(byte[] b, int start)
    {
        if (!SliceStartsWith(b, start, "[[")) return null;
        int afterOpen = start + 2;
        int descStart = FindSub(b, afterOpen, "][");
        if (descStart >= 0)
        {
            string url = Enc(b, afterOpen, descStart);
            int descBegin = descStart + 2;
            int close = FindSub(b, descBegin, "]]");
            if (close >= 0) return (url, Enc(b, descBegin, close), close + 2);
        }
        else
        {
            int close = FindSub(b, afterOpen, "]]");
            if (close >= 0) { string url = Enc(b, afterOpen, close); return (url, url, close + 2); }
        }
        return null;
    }

    private static bool IsOrgMarkupChar(byte b) => b is (byte)'*' or (byte)'/' or (byte)'_' or (byte)'=' or (byte)'~' or (byte)'+';

    private static int FindOrgMarkupClose(byte[] bytes, int from, byte marker)
    {
        int j = from;
        while (j < bytes.Length)
        {
            if (bytes[j] == marker && j > from && !IsAsciiWs(bytes[j - 1]))
            {
                if (j + 1 >= bytes.Length || IsAsciiWs(bytes[j + 1]) || bytes[j + 1] is (byte)'.' or (byte)',' or (byte)';'
                    or (byte)':' or (byte)')' or (byte)']' or (byte)'"')
                    return j;
            }
            j++;
        }
        return -1;
    }

    // ── byte helpers ────────────────────────────────────────────────────────

    private static bool IsAsciiWs(byte b) => b == (byte)' ' || b == (byte)'\t' || b == (byte)'\n' || b == (byte)'\r' || b == 0x0C || b == 0x0B;
    private static string Enc(byte[] b, int s, int e) => Encoding.UTF8.GetString(b, s, e - s);
    private static bool SliceStartsWith(byte[] b, int at, string s)
    {
        byte[] m = Encoding.ASCII.GetBytes(s);
        if (at + m.Length > b.Length) return false;
        for (int k = 0; k < m.Length; k++) if (b[at + k] != m[k]) return false;
        return true;
    }
    private static int FindSub(byte[] b, int from, string s)
    {
        byte[] m = Encoding.ASCII.GetBytes(s);
        for (int j = from; j + m.Length <= b.Length; j++)
        {
            bool ok = true;
            for (int k = 0; k < m.Length; k++) if (b[j + k] != m[k]) { ok = false; break; }
            if (ok) return j;
        }
        return -1;
    }
    private static int Utf8CharLen(byte[] b, int i)
    {
        byte c = b[i];
        if (c < 0x80) return 1;
        if ((c & 0xE0) == 0xC0) return 2;
        if ((c & 0xF0) == 0xE0) return 3;
        if ((c & 0xF8) == 0xF0) return 4;
        return 1;
    }
}
