using System.Text;
using XRay.Content.Core;
using XRay.Content.Types;

namespace XRay.Content.Rendering;

/// <summary>
/// Direct <see cref="InternalDocument"/> to HTML5 with <c>kb-*</c> class hooks, ported from Rust
/// <c>rendering/html_styled.rs</c>. Selected in place of the markdown-based renderer when
/// <see cref="ExtractionConfig.HtmlOutput"/> is set.
/// </summary>
/// <remarks>
/// Every class name (behind <see cref="HtmlOutputConfig.ClassPrefix"/>) and every
/// <c>--kb-*</c> custom property is part of a stability contract; they are not free to rename.
/// </remarks>
public sealed class StyledHtmlRenderer
{
    /// <summary>A stylesheet larger than this is refused rather than read.</summary>
    private const long MaxCssFileSize = 1_048_576;

    private readonly HtmlOutputConfig _config;

    /// <summary>Theme, file and inline CSS concatenated once, at construction.</summary>
    private readonly string _resolvedCss;

    public StyledHtmlRenderer(HtmlOutputConfig config)
    {
        foreach (char c in config.ClassPrefix)
            if (!char.IsLetterOrDigit(c) && c != '-' && c != '_')
                throw new ValidationException(
                    "html_output.class_prefix must contain only alphanumerics, hyphens, and "
                    + $"underscores, got: \"{config.ClassPrefix}\"");

        var css = new StringBuilder(ThemeCss(config.Theme));

        if (config.CssFile is { } path)
        {
            var info = new FileInfo(path);
            if (!info.Exists)
                throw new ValidationException($"html_output.css_file \"{path}\": file not found");
            if (info.Length > MaxCssFileSize)
                throw new ValidationException(
                    $"html_output.css_file \"{path}\": file size {info.Length} exceeds maximum "
                    + $"of {MaxCssFileSize} bytes");
            css.Append('\n').Append(File.ReadAllText(path));
        }

        if (config.Css is { } inline)
            css.Append('\n').Append(inline);

        // A closing tag inside the stylesheet would end the `<style>` block early and let the
        // rest of the CSS out as markup.
        _config = config;
        _resolvedCss = css.ToString().Replace("</style>", "").Replace("</STYLE>", "");
    }

    public string Render(InternalDocument doc)
    {
        string p = _config.ClassPrefix;
        var buf = new StringBuilder();

        buf.Append($"<div class=\"{p}doc\">");

        if (_config.EmbedCss && _resolvedCss.Length > 0)
            buf.Append("<style>").Append(_resolvedCss).Append("</style>");

        buf.Append($"<main class=\"{p}content\">");
        RenderElements(doc, p, buf);
        buf.Append("</main></div>");

        return buf.ToString();
    }

    private static string ThemeCss(HtmlTheme theme) => theme switch
    {
        HtmlTheme.Unstyled => "",
        HtmlTheme.Default or HtmlTheme.Light => DefaultCss,
        HtmlTheme.GitHub => GitHubCss,
        HtmlTheme.Dark => DarkCss,
        _ => "",
    };

    /// <summary>
    /// Escape for HTML text and attribute positions alike.
    /// </summary>
    /// <remarks>
    /// The set is <c>v_htmlescape</c>'s, which is OWASP's: it covers the slash as well as the
    /// five usual characters. That matters for byte-for-byte agreement — a URL in an
    /// <c>href</c> comes out with <c>&amp;#x2f;</c> where <c>WebUtility.HtmlEncode</c> would
    /// leave a bare <c>/</c>.
    /// </remarks>
    internal static string Esc(string text)
    {
        var sb = new StringBuilder(text.Length);
        foreach (char c in text)
        {
            switch (c)
            {
                case '<': sb.Append("&lt;"); break;
                case '>': sb.Append("&gt;"); break;
                case '&': sb.Append("&amp;"); break;
                case '"': sb.Append("&quot;"); break;
                case '\'': sb.Append("&#x27;"); break;
                case '/': sb.Append("&#x2f;"); break;
                default: sb.Append(c); break;
            }
        }
        return sb.ToString();
    }

    private static void RenderElements(InternalDocument doc, string p, StringBuilder buf)
    {
        var state = new RenderState();
        var listOrderedStack = new List<bool>();

        int elementIndex = 0;
        foreach (var elem in doc.Elements)
        {
            Cancel.Every(elementIndex++, 0x3FF);
            if (elem.Layer != ContentLayer.Body) continue;

            switch (elem.Kind.Tag)
            {
                case ElementKindTag.Title:
                    buf.Append($"<h1 class=\"{p}doc-title\">{Esc(elem.Text)}</h1>");
                    break;

                case ElementKindTag.Heading:
                {
                    int lvl = Math.Clamp(elem.Kind.Level, (byte)1, (byte)6);
                    buf.Append($"<h{lvl} class=\"{p}h {p}h{lvl}\">{RenderInline(elem, p)}</h{lvl}>");
                    break;
                }

                case ElementKindTag.Paragraph:
                case ElementKindTag.OcrText:
                    buf.Append($"<p class=\"{p}p\">{RenderInline(elem, p)}</p>");
                    break;

                case ElementKindTag.ListStart:
                {
                    bool ordered = elem.Kind.Ordered;
                    listOrderedStack.Add(ordered);
                    state.PushContainer(NestingKind.ListKind(ordered, 0), elem.Depth);
                    buf.Append(ordered
                        ? $"<ol class=\"{p}list {p}ol\">"
                        : $"<ul class=\"{p}list {p}ul\">");
                    break;
                }
                case ElementKindTag.ListEnd:
                {
                    bool ordered = false;
                    if (listOrderedStack.Count > 0)
                    {
                        ordered = listOrderedStack[^1];
                        listOrderedStack.RemoveAt(listOrderedStack.Count - 1);
                    }
                    state.PopContainer(NestingKindTag.List);
                    buf.Append(ordered ? "</ol>" : "</ul>");
                    break;
                }
                case ElementKindTag.ListItem:
                    // <ol>/<ul> only ever render an auto-incrementing decimal or a bullet glyph;
                    // a literal source label that shape cannot express is written out as visible
                    // leading text, the convention every other renderer uses.
                    if (elem.ListItemSourceLabel is { Length: > 0 } htmlLabel)
                        buf.Append($"<li class=\"{p}li\"><span class=\"{p}list-marker\">"
                                   + $"{Esc(htmlLabel)}</span> {RenderInline(elem, p)}</li>");
                    else
                        buf.Append($"<li class=\"{p}li\">{RenderInline(elem, p)}</li>");
                    break;

                case ElementKindTag.QuoteStart:
                    state.PushContainer(NestingKind.BlockQuote, elem.Depth);
                    buf.Append($"<blockquote class=\"{p}blockquote\">");
                    break;
                case ElementKindTag.QuoteEnd:
                    state.PopContainer(NestingKindTag.BlockQuote);
                    buf.Append("</blockquote>");
                    break;

                case ElementKindTag.Code:
                {
                    string lang = Attr(elem, "language") ?? Attr(elem, "lang") ?? "";
                    if (lang.Length == 0)
                        buf.Append($"<pre class=\"{p}pre\"><code class=\"{p}code\">{Esc(elem.Text)}</code></pre>");
                    else
                        buf.Append($"<pre class=\"{p}pre\"><code class=\"{p}code {p}lang-{Esc(lang)}\">"
                                   + $"{Esc(elem.Text)}</code></pre>");
                    break;
                }

                case ElementKindTag.Formula:
                    // Math, not code: LaTeX in `$$...$$` display delimiters, the convention the
                    // markdown and djot renderers already use. KaTeX and MathJax auto-render pick
                    // that up as-is; without either, it degrades to visible LaTeX source rather
                    // than to a monospace block that claims to be code.
                    buf.Append($"<div class=\"{p}formula {p}math\" data-math-style=\"display\">"
                               + $"$${Esc(elem.Text)}$$</div>");
                    break;

                case ElementKindTag.FootnoteDefinition:
                    buf.Append($"<aside class=\"{p}footnote\" id=\"fn-{Esc(elem.Anchor ?? "")}\">"
                               + $"{RenderInline(elem, p)}</aside>");
                    break;
                case ElementKindTag.FootnoteRef:
                    buf.Append($"<sup class=\"{p}footnote-ref\"><a href=\"#fn-{Esc(elem.Anchor ?? "")}\">"
                               + $"{Esc(elem.Text)}</a></sup>");
                    break;
                case ElementKindTag.CommentDefinition:
                    buf.Append($"<aside class=\"{p}comment\" id=\"fn-{Esc(elem.Anchor ?? "")}\">"
                               + $"{RenderInline(elem, p)}</aside>");
                    break;
                case ElementKindTag.CommentRef:
                    buf.Append($"<sup class=\"{p}comment-ref\"><a href=\"#fn-{Esc(elem.Anchor ?? "")}\">"
                               + $"{Esc(elem.Text)}</a></sup>");
                    break;
                case ElementKindTag.Citation:
                    buf.Append($"<cite class=\"{p}citation\">{Esc(elem.Text)}</cite>");
                    break;

                case ElementKindTag.Slide:
                    // A slide is a marker, not a container — there is no SlideEnd — so the
                    // section is opened and closed in one go. An unbalanced opening tag would
                    // leave every deck's HTML malformed.
                    buf.Append($"<section class=\"{p}slide\" data-slide=\"{elem.Kind.Number}\">");
                    if (elem.Text.Length > 0)
                        buf.Append($"<h2 class=\"{p}h {p}h2\">{RenderInline(elem, p)}</h2>");
                    buf.Append("</section>");
                    break;

                case ElementKindTag.DefinitionTerm:
                    buf.Append($"<dt class=\"{p}dt\">{RenderInline(elem, p)}</dt>");
                    break;
                case ElementKindTag.DefinitionDescription:
                    buf.Append($"<dd class=\"{p}dd\">{RenderInline(elem, p)}</dd>");
                    break;

                case ElementKindTag.Admonition:
                {
                    string kind = Attr(elem, "kind") ?? Attr(elem, "type") ?? "note";
                    buf.Append($"<aside class=\"{p}admonition {p}admonition-{Esc(kind)}\">"
                               + $"{RenderInline(elem, p)}</aside>");
                    break;
                }

                case ElementKindTag.RawBlock:
                {
                    // Only a block that says it is HTML goes through verbatim. Most producers of
                    // a raw block do not: ODP pushes speaker notes and master-page text, Org
                    // pushes Org source, the HTML walker pushes script and style bodies. Writing
                    // those unescaped puts author-typed text — a `<` in a speaker note, or
                    // LibreOffice's literal `<number>` placeholder — into the output as markup,
                    // which corrupts the structure and is an injection vector.
                    bool isHtml = Attr(elem, "format") == "html";
                    if (isHtml) buf.Append(elem.Text);
                    else buf.Append($"<pre class=\"{p}pre {p}raw\">{Esc(elem.Text)}</pre>");
                    break;
                }
                case ElementKindTag.MetadataBlock:
                    buf.Append($"<dl class=\"{p}metadata\">{Esc(elem.Text)}</dl>");
                    break;

                case ElementKindTag.GroupStart:
                    state.PushContainer(NestingKind.Group, elem.Depth);
                    buf.Append($"<div class=\"{p}group\">");
                    break;
                case ElementKindTag.GroupEnd:
                    state.PopContainer(NestingKindTag.Group);
                    buf.Append("</div>");
                    break;

                case ElementKindTag.Table:
                {
                    int index = (int)elem.Kind.TableIndex;
                    if (index >= 0 && index < doc.Tables.Count) RenderTable(doc.Tables[index], p, buf);
                    break;
                }

                case ElementKindTag.Image:
                {
                    int index = (int)elem.Kind.ImageIndex;
                    if (index >= 0 && index < doc.Images.Count)
                        RenderImage(doc.Images[index], elem.Text, p, buf);
                    break;
                }

                case ElementKindTag.PageBreak:
                    buf.Append($"<hr class=\"{p}page-break\" data-page=\"{elem.Page ?? 0}\">");
                    break;
            }
        }
    }

    private static string? Attr(InternalElement elem, string key) =>
        elem.Attributes is { } a && a.TryGetValue(key, out var v) ? v : null;

    private static string RenderInline(InternalElement elem, string p)
    {
        if (elem.Annotations.Count == 0) return Esc(elem.Text);

        return RenderCommon.RenderAnnotatedTextWithPlain(
            elem.Text,
            elem.Annotations,
            (span, kind) => kind.Which switch
            {
                AnnotationKind.Tag.Bold => $"<strong>{Esc(span)}</strong>",
                AnnotationKind.Tag.Italic => $"<em>{Esc(span)}</em>",
                AnnotationKind.Tag.Strikethrough => $"<del>{Esc(span)}</del>",
                AnnotationKind.Tag.Link => $"<a class=\"{p}link\" href=\"{Esc(kind.Url ?? "")}\">{Esc(span)}</a>",
                _ => Esc(span),
            },
            Esc);
    }

    private static void RenderTable(Table table, string p, StringBuilder buf)
    {
        buf.Append($"<table class=\"{p}table\">");

        // The first row is the header, whether or not the source called it one — which is what
        // upstream does, and what keeps a headerless table from rendering as an empty thead.
        if (table.Cells.Count > 0)
        {
            buf.Append($"<thead class=\"{p}thead\"><tr class=\"{p}tr\">");
            foreach (var cell in table.Cells[0]) buf.Append($"<th class=\"{p}th\">{Esc(cell)}</th>");
            buf.Append("</tr></thead>");
        }

        if (table.Cells.Count > 1)
        {
            buf.Append($"<tbody class=\"{p}tbody\">");
            for (int r = 1; r < table.Cells.Count; r++)
            {
                buf.Append($"<tr class=\"{p}tr\">");
                foreach (var cell in table.Cells[r]) buf.Append($"<td class=\"{p}td\">{Esc(cell)}</td>");
                buf.Append("</tr>");
            }
            buf.Append("</tbody>");
        }

        buf.Append("</table>");
    }

    /// <summary>
    /// Emit an image as a data URI inside a figure.
    /// </summary>
    /// <remarks>
    /// Upstream also has two OCR branches here — text in place of the image, or text as a
    /// caption beneath it. Neither is reachable in this port: OCR is out of scope, so no
    /// document it produces carries an OCR result to render.
    /// </remarks>
    private static void RenderImage(ExtractedImage image, string alt, string p, StringBuilder buf)
    {
        string b64 = Convert.ToBase64String(image.Data);
        string mime = image.Format switch
        {
            "jpeg" or "jpg" => "image/jpeg",
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            "tiff" => "image/tiff",
            _ => "image/png",
        };
        buf.Append($"<figure class=\"{p}figure\"><img class=\"{p}img\" "
                   + $"src=\"data:{mime};base64,{b64}\" alt=\"{Esc(alt)}\"></figure>");
    }

    private const string DefaultCss = """
:root {
  --kb-font-family: system-ui, sans-serif;
  --kb-mono-font-family: ui-monospace, monospace;
  --kb-text-color: #1a1a1a;
  --kb-bg-color: #ffffff;
  --kb-heading-color: #111111;
  --kb-link-color: #0066cc;
  --kb-link-hover-color: #004499;
  --kb-code-bg: #f5f5f5;
  --kb-code-color: #c7254e;
  --kb-border-color: #e0e0e0;
  --kb-table-border: #cccccc;
  --kb-blockquote-border: #0066cc;
  --kb-max-width: 72ch;
  --kb-line-height: 1.6;
}
.kb-doc { font-family: var(--kb-font-family); color: var(--kb-text-color); background: var(--kb-bg-color); line-height: var(--kb-line-height); }
.kb-content { max-width: var(--kb-max-width); margin: 0 auto; padding: 1rem; }
.kb-h { color: var(--kb-heading-color); margin: 1.5em 0 0.5em; line-height: 1.25; }
.kb-p { margin: 0.75em 0; }
.kb-list { margin: 0.75em 0; padding-left: 2em; }
.kb-li { margin: 0.25em 0; }
.kb-blockquote { border-left: 4px solid var(--kb-blockquote-border); margin: 1em 0; padding: 0.5em 1em; color: #555; }
.kb-pre { background: var(--kb-code-bg); border: 1px solid var(--kb-border-color); border-radius: 4px; overflow-x: auto; padding: 1em; margin: 1em 0; }
.kb-code { font-family: var(--kb-mono-font-family); font-size: 0.875em; }
p .kb-code { background: var(--kb-code-bg); color: var(--kb-code-color); padding: 0.1em 0.3em; border-radius: 3px; }
.kb-table { border-collapse: collapse; width: 100%; margin: 1em 0; }
.kb-th, .kb-td { border: 1px solid var(--kb-table-border); padding: 0.5em 0.75em; text-align: left; }
.kb-th { background: var(--kb-code-bg); font-weight: 600; }
.kb-figure { margin: 1em 0; }
.kb-img { max-width: 100%; height: auto; }
.kb-page-break { border: none; border-top: 1px dashed var(--kb-border-color); margin: 2em 0; }

""";

    private const string GitHubCss = """
:root {
  --kb-font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif;
  --kb-mono-font-family: SFMono-Regular, Consolas, "Liberation Mono", Menlo, monospace;
  --kb-text-color: #24292f;
  --kb-bg-color: #ffffff;
  --kb-heading-color: #24292f;
  --kb-link-color: #0969da;
  --kb-link-hover-color: #0550ae;
  --kb-code-bg: #f6f8fa;
  --kb-code-color: #e01e5a;
  --kb-border-color: #d0d7de;
  --kb-table-border: #d0d7de;
  --kb-blockquote-border: #d0d7de;
  --kb-max-width: 80ch;
  --kb-line-height: 1.5;
}
.kb-doc { font-family: var(--kb-font-family); color: var(--kb-text-color); background: var(--kb-bg-color); line-height: var(--kb-line-height); }
.kb-content { max-width: var(--kb-max-width); margin: 0 auto; padding: 1rem 2rem; }
.kb-h { color: var(--kb-heading-color); margin: 1.5em 0 0.5em; border-bottom: 1px solid var(--kb-border-color); padding-bottom: 0.3em; }
.kb-p { margin: 0.75em 0; }
.kb-list { margin: 0.75em 0; padding-left: 2em; }
.kb-li { margin: 0.25em 0; }
.kb-blockquote { border-left: 4px solid var(--kb-blockquote-border); margin: 1em 0; padding: 0.5em 1em; color: #57606a; }
.kb-pre { background: var(--kb-code-bg); border: 1px solid var(--kb-border-color); border-radius: 6px; overflow-x: auto; padding: 1em; margin: 1em 0; }
.kb-code { font-family: var(--kb-mono-font-family); font-size: 85%; }
p .kb-code { background: var(--kb-code-bg); color: var(--kb-code-color); padding: 0.2em 0.4em; border-radius: 6px; }
.kb-table { border-collapse: collapse; width: 100%; margin: 1em 0; }
.kb-th, .kb-td { border: 1px solid var(--kb-table-border); padding: 0.4em 0.8em; }
.kb-th { background: var(--kb-code-bg); font-weight: 600; }
.kb-figure { margin: 1em 0; }
.kb-img { max-width: 100%; height: auto; }
.kb-page-break { border: none; border-top: 1px dashed var(--kb-border-color); margin: 2em 0; }

""";

    private const string DarkCss = """
:root {
  --kb-font-family: system-ui, sans-serif;
  --kb-mono-font-family: ui-monospace, monospace;
  --kb-text-color: #e6edf3;
  --kb-bg-color: #0d1117;
  --kb-heading-color: #f0f6fc;
  --kb-link-color: #58a6ff;
  --kb-link-hover-color: #79c0ff;
  --kb-code-bg: #161b22;
  --kb-code-color: #ff7b72;
  --kb-border-color: #30363d;
  --kb-table-border: #30363d;
  --kb-blockquote-border: #3d444d;
  --kb-max-width: 72ch;
  --kb-line-height: 1.6;
}
.kb-doc { font-family: var(--kb-font-family); color: var(--kb-text-color); background: var(--kb-bg-color); line-height: var(--kb-line-height); }
.kb-content { max-width: var(--kb-max-width); margin: 0 auto; padding: 1rem; }
.kb-h { color: var(--kb-heading-color); margin: 1.5em 0 0.5em; }
.kb-p { margin: 0.75em 0; }
.kb-list { margin: 0.75em 0; padding-left: 2em; }
.kb-li { margin: 0.25em 0; }
.kb-blockquote { border-left: 4px solid var(--kb-blockquote-border); margin: 1em 0; padding: 0.5em 1em; color: #8d96a0; }
.kb-pre { background: var(--kb-code-bg); border: 1px solid var(--kb-border-color); border-radius: 4px; overflow-x: auto; padding: 1em; margin: 1em 0; }
.kb-code { font-family: var(--kb-mono-font-family); font-size: 0.875em; }
p .kb-code { background: var(--kb-code-bg); color: var(--kb-code-color); padding: 0.1em 0.3em; border-radius: 3px; }
.kb-table { border-collapse: collapse; width: 100%; margin: 1em 0; }
.kb-th, .kb-td { border: 1px solid var(--kb-table-border); padding: 0.5em 0.75em; }
.kb-th { background: var(--kb-code-bg); font-weight: 600; }
.kb-figure { margin: 1em 0; }
.kb-img { max-width: 100%; height: auto; }
.kb-page-break { border: none; border-top: 1px dashed var(--kb-border-color); margin: 2em 0; }

""";
}
