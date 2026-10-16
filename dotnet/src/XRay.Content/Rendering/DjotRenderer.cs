using System.Text;
using XRay.Content.Types;

namespace XRay.Content.Rendering;

/// <summary>
/// Renders an <see cref="InternalDocument"/> to Djot markup.
///
/// DEVIATION FROM RUST: this is a pragmatic direct element-walk writer covering the common
/// block constructs, not a full port of `rendering/djot.rs`. Djot pipe tables reuse the GFM
/// table writer. Documented in PORT_NOTES.
/// </summary>
public static class DjotRenderer
{
    public static string Render(InternalDocument doc)
    {
        var sb = new StringBuilder(doc.Elements.Count * 80);

        void Block(string s)
        {
            if (sb.Length > 0) sb.Append('\n');
            sb.Append(s);
            if (sb.Length == 0 || sb[^1] != '\n') sb.Append('\n');
        }

        foreach (var elem in doc.Elements)
        {
            if (elem.Layer != ContentLayer.Body) continue;

            switch (elem.Kind.Tag)
            {
                case ElementKindTag.Title:
                    if (elem.Text.Length > 0) Block("# " + elem.Text);
                    break;
                case ElementKindTag.Heading:
                    if (elem.Text.Length > 0)
                    {
                        int level = Math.Clamp(elem.Kind.Level, (byte)1, (byte)6);
                        Block(new string('#', level) + " " + elem.Text);
                    }
                    break;
                case ElementKindTag.Paragraph:
                    if (elem.Text.Length > 0) Block(EmitInline(elem));
                    break;
                case ElementKindTag.ListItem:
                    // Djot's auto-incrementing decimal marker cannot express a literal source
                    // label (e.g. "B.", "(a)") any more than CommonMark's can, so an item
                    // carrying one falls back to a bullet with the label written out — the same
                    // trade-off the markdown renderer makes.
                    if (elem.ListItemSourceLabel is { Length: > 0 } djotLabel)
                        sb.Append("- ").Append(djotLabel).Append(' ');
                    else
                        sb.Append(elem.Kind.Ordered ? "1. " : "- ");
                    sb.Append(elem.Text).Append('\n');
                    break;
                case ElementKindTag.Code:
                    {
                        string lang = RenderCommon.GetLanguage(elem) ?? "";
                        Block("```" + lang + "\n" + elem.Text.TrimEnd('\n') + "\n```");
                    }
                    break;
                case ElementKindTag.Formula:
                    if (elem.Text.Length > 0) Block("$$" + elem.Text + "$$");
                    break;
                case ElementKindTag.Table:
                    {
                        int ti = (int)elem.Kind.TableIndex;
                        if (ti >= 0 && ti < doc.Tables.Count)
                        {
                            var table = doc.Tables[ti];
                            string t = table.Cells.Count > 0
                                ? RenderCommon.RenderTableDjot(table.Cells)
                                : table.Markdown;
                            if (t.Trim().Length > 0) Block(t.TrimEnd('\n'));
                        }
                    }
                    break;
                case ElementKindTag.Image:
                    {
                        int ii = (int)elem.Kind.ImageIndex;
                        if (ii >= 0 && ii < doc.Images.Count)
                        {
                            var img = doc.Images[ii];
                            string alt = img.Description ?? elem.Text;
                            string src = img.Data.Length > 0
                                ? $"image_{elem.Kind.ImageIndex}.{img.Format}"
                                : img.SourcePath ?? "";
                            Block($"![{alt}]({src})");
                        }
                    }
                    break;
                case ElementKindTag.Citation:
                case ElementKindTag.Slide:
                case ElementKindTag.OcrText:
                case ElementKindTag.RawBlock:
                    if (elem.Text.Length > 0) Block(elem.Text);
                    break;
                case ElementKindTag.PageBreak:
                    Block("---");
                    break;
            }
        }

        if (doc.Annotations is { Count: > 0 } annotations)
        {
            string block = RenderCommon.RenderAnnotationsDjot(annotations);
            if (block.Length > 0)
            {
                if (sb.ToString().AsSpan().TrimEnd().Length > 0) sb.Append('\n');
                sb.Append(block);
            }
        }

        string output = sb.ToString();
        int trimmedLen = RenderCommon.TrimEndLen(output);
        if (trimmedLen == 0) return "";
        return output.Substring(0, trimmedLen) + "\n";
    }

    private static string EmitInline(InternalElement elem)
    {
        if (elem.Annotations.Count == 0) return elem.Text;
        return RenderCommon.RenderAnnotatedText(elem.Text, elem.Annotations, (span, kind) => kind.Which switch
        {
            AnnotationKind.Tag.Bold => "*" + span + "*",
            AnnotationKind.Tag.Italic => "_" + span + "_",
            AnnotationKind.Tag.Code => "`" + span + "`",
            AnnotationKind.Tag.Strikethrough => "{-" + span + "-}",
            AnnotationKind.Tag.Link => "[" + span + "](" + (kind.Url ?? "") + ")",
            _ => span,
        });
    }
}
