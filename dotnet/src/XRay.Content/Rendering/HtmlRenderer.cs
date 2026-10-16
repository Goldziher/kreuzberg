using XRay.Content.Internal.Commonmark;
using XRay.Content.Types;

namespace XRay.Content.Rendering;

/// <summary>
/// Renders an <see cref="InternalDocument"/> to HTML5.
///
/// Faithful port of <c>crates/xberg/src/rendering/html.rs</c>: builds a comrak AST via
/// <see cref="ComrakBridge"/> and serializes it with <see cref="HtmlFormatter"/>
/// (<c>format_html</c> with <c>unsafe = true</c>, <c>github_pre_lang = true</c>,
/// <c>full_info_string = true</c>) so output matches Rust byte-for-byte.
/// </summary>
public static class HtmlRenderer
{
    public static string Render(InternalDocument doc)
    {
        var root = ComrakBridge.Build(doc);
        string html = HtmlFormatter.Format(root);
        if (doc.Annotations is { Count: > 0 } annotations)
            html += RenderCommon.RenderAnnotationsHtml(annotations);
        return html;
    }
}
