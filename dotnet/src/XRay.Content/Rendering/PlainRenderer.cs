using System.Text;
using XRay.Content.Types;

namespace XRay.Content.Rendering;

/// <summary>Renders an <see cref="InternalDocument"/> to plain text. Ported from `rendering/plain.rs`.</summary>
public static class PlainRenderer
{
    public static string Render(InternalDocument doc)
    {
        var sb = new StringBuilder(doc.Elements.Count * 80);
        ushort? lastHeadingDepth = null;

        foreach (var elem in doc.Elements)
        {
            if (elem.Layer != ContentLayer.Body) continue;
            if (elem.Kind.IsContainerStart || elem.Kind.IsContainerEnd) continue;

            switch (elem.Kind.Tag)
            {
                case ElementKindTag.Title:
                case ElementKindTag.Heading:
                case ElementKindTag.Paragraph:
                    if (elem.Text.Length > 0)
                    {
                        bool isHeading = elem.Kind.Tag == ElementKindTag.Heading;
                        if (isHeading)
                        {
                            if (lastHeadingDepth is ushort last)
                            {
                                if ((last == 0 || last == 1) && last == elem.Depth
                                    && sb.Length > 0 && !EndsWith(sb, "\n\n"))
                                {
                                    sb.Append('\n');
                                }
                            }
                            lastHeadingDepth = elem.Depth;
                        }

                        if (elem.Kind.Tag == ElementKindTag.Paragraph || (isHeading && elem.Depth > 0))
                            sb.Append(new string(' ', 2 * elem.Depth));

                        if (isHeading && elem.Attributes is not null)
                        {
                            sb.Append(elem.Text);
                            var filtered = elem.Attributes
                                .Where(kv => !kv.Key.StartsWith("xmlns") && kv.Value.Length > 0)
                                .OrderBy(kv => kv.Key, StringComparer.Ordinal)
                                .Select(kv => $"{kv.Key}: {kv.Value}")
                                .ToList();
                            if (filtered.Count > 0)
                            {
                                sb.Append(" (");
                                sb.Append(string.Join(", ", filtered));
                                sb.Append(')');
                            }
                        }
                        else
                        {
                            sb.Append(elem.Text);
                        }

                        if (isHeading) sb.Append('\n');
                        else sb.Append("\n\n");
                    }
                    break;

                case ElementKindTag.ListItem:
                    // Plain text renders no marker glyph at all — this format is markers-off
                    // across the board. A literal source label (e.g. "B.", "(a)") is the one piece
                    // of list-item structure it can still carry, so it is kept as visible text.
                    if (elem.ListItemSourceLabel is { Length: > 0 } plainLabel)
                    {
                        sb.Append(plainLabel);
                        sb.Append(' ');
                    }
                    sb.Append(elem.Text);
                    sb.Append('\n');
                    break;

                case ElementKindTag.Code:
                    sb.Append(elem.Text);
                    if (!elem.Text.EndsWith('\n')) sb.Append('\n');
                    sb.Append('\n');
                    break;

                case ElementKindTag.Formula:
                    sb.Append(elem.Text);
                    sb.Append("\n\n");
                    break;

                case ElementKindTag.Table:
                    {
                        int ti = (int)elem.Kind.TableIndex;
                        if (ti >= 0 && ti < doc.Tables.Count)
                        {
                            var table = doc.Tables[ti];
                            // An extractor that already rendered canonical plain text for this
                            // table stores it on the element; prefer it over re-rendering cells.
                            string tableStr = elem.Text.Length > 0
                                ? elem.Text
                                : table.Cells.Count > 0
                                    ? RenderCommon.RenderTablePlain(table.Cells)
                                    : table.Markdown;
                            if (tableStr.Trim().Length > 0)
                            {
                                sb.Append(tableStr);
                                sb.Append('\n');
                            }
                        }
                    }
                    break;

                case ElementKindTag.Image:
                    {
                        int ii = (int)elem.Kind.ImageIndex;
                        if (ii >= 0 && ii < doc.Images.Count)
                        {
                            var img = doc.Images[ii];
                            if (img.Description is { Length: > 0 } desc)
                            {
                                sb.Append("[Image: ");
                                sb.Append(desc);
                                sb.Append("]\n\n");
                            }
                            if (img.OcrResult is { Content.Length: > 0 } ocr)
                            {
                                sb.Append(ocr.Content);
                                sb.Append("\n\n");
                            }
                        }
                        else if (elem.Text.Trim().Length > 0)
                        {
                            // An image the extractor could not resolve (a missing archive member)
                            // still carries its alt text or caption.
                            sb.Append("[Image: ");
                            sb.Append(elem.Text.Trim());
                            sb.Append("]\n\n");
                        }
                    }
                    break;

                case ElementKindTag.FootnoteRef:
                case ElementKindTag.FootnoteDefinition:
                case ElementKindTag.CommentRef:
                case ElementKindTag.CommentDefinition:
                    // Skipped in body pass.
                    break;

                case ElementKindTag.Citation:
                    if (elem.Text.Length > 0)
                    {
                        sb.Append(elem.Text);
                        sb.Append("\n\n");
                    }
                    break;

                case ElementKindTag.PageBreak:
                    sb.Append('\n');
                    break;

                case ElementKindTag.Slide:
                    if (elem.Text.Length > 0)
                    {
                        sb.Append(elem.Text);
                        sb.Append("\n\n");
                    }
                    break;

                case ElementKindTag.DefinitionTerm:
                    sb.Append(elem.Text);
                    sb.Append(": ");
                    break;

                case ElementKindTag.DefinitionDescription:
                    sb.Append(elem.Text);
                    sb.Append("\n\n");
                    break;

                case ElementKindTag.Admonition:
                    {
                        var title = RenderCommon.GetAdmonitionTitle(elem);
                        sb.Append(title ?? RenderCommon.GetAdmonitionKind(elem));
                        sb.Append("\n\n");
                        if (elem.Text.Length > 0)
                        {
                            sb.Append(elem.Text);
                            sb.Append("\n\n");
                        }
                    }
                    break;

                case ElementKindTag.RawBlock:
                    sb.Append(elem.Text);
                    if (!elem.Text.EndsWith('\n')) sb.Append('\n');
                    sb.Append('\n');
                    break;

                case ElementKindTag.MetadataBlock:
                    {
                        var entries = RenderCommon.ParseMetadataEntries(elem.Text);
                        if (entries.Count > 0)
                        {
                            foreach (var (key, value) in entries)
                            {
                                sb.Append(key);
                                sb.Append(": ");
                                sb.Append(value);
                                sb.Append('\n');
                            }
                            sb.Append('\n');
                        }
                        else if (elem.Text.Length > 0)
                        {
                            sb.Append(elem.Text);
                            if (!elem.Text.EndsWith('\n')) sb.Append('\n');
                            sb.Append('\n');
                        }
                    }
                    break;

                case ElementKindTag.OcrText:
                    if (elem.Text.Length > 0)
                    {
                        sb.Append(elem.Text);
                        sb.Append("\n\n");
                    }
                    break;
            }
        }

        bool hasFootnotes = doc.Elements.Any(e =>
            e.Kind.Tag == ElementKindTag.FootnoteDefinition && e.Layer == ContentLayer.Footnote);
        if (hasFootnotes)
        {
            sb.Append('\n');
            foreach (var elem in doc.Elements)
            {
                if (elem.Kind.Tag == ElementKindTag.FootnoteDefinition && elem.Layer == ContentLayer.Footnote)
                {
                    sb.Append(elem.Text);
                    sb.Append("\n\n");
                }
            }
        }

        // A comment body is furniture rather than body flow, exactly as a footnote definition
        // is, so it is surfaced the same way rather than silently dropped now that it no longer
        // shares the footnote kind.
        bool hasComments = doc.Elements.Any(e =>
            e.Kind.Tag == ElementKindTag.CommentDefinition && e.Layer == ContentLayer.Footnote);
        if (hasComments)
        {
            sb.Append('\n');
            foreach (var elem in doc.Elements)
            {
                if (elem.Kind.Tag == ElementKindTag.CommentDefinition && elem.Layer == ContentLayer.Footnote)
                {
                    sb.Append(elem.Text);
                    sb.Append("\n\n");
                }
            }
        }

        // Document-level PDF annotations (only present when the extractor was asked for them).
        if (doc.Annotations is { Count: > 0 } annotations)
        {
            string block = RenderCommon.RenderAnnotationsPlain(annotations);
            if (block.Length > 0)
            {
                if (sb.ToString().AsSpan().TrimEnd().Length > 0) sb.Append("\n\n");
                sb.Append(block);
            }
        }

        // Trim trailing whitespace, no trailing newline.
        string result = sb.ToString();
        return result.Substring(0, RenderCommon.TrimEndLen(result));
    }

    private static bool EndsWith(StringBuilder sb, string suffix)
    {
        if (sb.Length < suffix.Length) return false;
        for (int i = 0; i < suffix.Length; i++)
            if (sb[sb.Length - suffix.Length + i] != suffix[i]) return false;
        return true;
    }
}
