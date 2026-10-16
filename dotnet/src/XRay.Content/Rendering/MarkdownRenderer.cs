using System.Text;
using System.Text.RegularExpressions;
using XRay.Content.Internal.Commonmark;
using XRay.Content.Types;

namespace XRay.Content.Rendering;

/// <summary>
/// Renders an <see cref="InternalDocument"/> to GFM Markdown.
///
/// Faithful port of <c>crates/xberg/src/rendering/markdown.rs</c>: builds a comrak AST via
/// <see cref="ComrakBridge"/>, serializes it with <see cref="CommonMarkFormatter"/>
/// (<c>format_commonmark</c>, <c>render.width = 0</c>), then applies the same string
/// post-processing as Rust so output matches byte-for-byte.
/// </summary>
public static class MarkdownRenderer
{
    private static readonly char[] UnescapeTargets = { '_', '[', ']', '(', ')', '*', '=' };

    private static readonly Regex ArxivWatermarkRegex = new(
        @"(?:\s+\S+(?:\s+\S+){0,8})?\s*arXiv:\d{4}\.\d{4,5}(?:v\d+)?(?:\s*\[[\w.-]+\])?\s*(?:\d{1,2}\s+\w+\s+\d{4})?",
        RegexOptions.Compiled);

    public static string Render(InternalDocument doc)
    {
        var root = ComrakBridge.Build(doc);

        // Guard: empty AST (comrak panics on this; we return empty) — except that a document whose
        // only content is an annotation (a highlight on a scanned page) still renders that.
        if (root.FirstChild is null)
        {
            string only = doc.Annotations is { Count: > 0 } a ? RenderCommon.RenderAnnotationsMarkdown(a) : "";
            int onlyLen = RenderCommon.TrimEndLen(only);
            return onlyLen == 0 ? "" : only.Substring(0, onlyLen) + "\n";
        }

        string output = CommonMarkFormatter.Format(root);

        // Strip comrak-generated HTML comments (e.g. `<!-- end list -->`).
        if (output.Contains("<!--"))
        {
            var kept = new List<string>();
            foreach (var line in RenderCommon.SplitLines(output))
            {
                string trimmed = line.Trim();
                if (!(trimmed.StartsWith("<!--", StringComparison.Ordinal) && trimmed.EndsWith("-->", StringComparison.Ordinal)))
                    kept.Add(line);
            }
            output = string.Join("\n", kept);
        }

        // Decode leftover HTML entities: `&#10;` -> space, `&#2;` -> removed.
        output = ReplaceHtmlEntities(output);

        // Un-escape underscores, brackets, parens, stars, equals in one pass.
        output = UnescapeBackslashSequences(output, UnescapeTargets);

        // Un-escape `\*` and `\#` at the START of lines only.
        if (output.Contains("\\*") || output.Contains("\\#"))
        {
            var mapped = new List<string>();
            foreach (var line in RenderCommon.SplitLines(output))
            {
                string trimmed = line.TrimStart();
                if (trimmed.StartsWith("\\* ", StringComparison.Ordinal)
                    || trimmed.StartsWith("\\#.", StringComparison.Ordinal)
                    || trimmed.StartsWith("\\#\\.", StringComparison.Ordinal))
                    mapped.Add(ReplaceFirst(ReplaceFirst(line, "\\*", "*"), "\\#", "#"));
                else
                    mapped.Add(line);
            }
            output = string.Join("\n", mapped);
        }

        // Collapse runs of 3+ newlines into exactly 2.
        output = CollapseExcessNewlines(output);

        // Strip arXiv watermark noise.
        output = StripArxivWatermarkNoise(output);

        // Document-level PDF annotations, appended as their own section.
        if (doc.Annotations is { Count: > 0 } annotations)
        {
            string block = RenderCommon.RenderAnnotationsMarkdown(annotations);
            if (block.Length > 0)
            {
                if (RenderCommon.TrimEndLen(output) > 0) output += "\n\n";
                output += block;
            }
        }

        // Trim trailing whitespace but keep a single trailing newline.
        int trimmedLen = RenderCommon.TrimEndLen(output);
        if (trimmedLen == 0) return "";
        return output.Substring(0, trimmedLen) + "\n";
    }

    private static string StripArxivWatermarkNoise(string text)
    {
        int searchLimit = Math.Min(text.Length, 6000);
        if (searchLimit > 0 && searchLimit < text.Length && char.IsLowSurrogate(text[searchLimit])) searchLimit--;
        string searchArea = text.Substring(0, searchLimit);

        var m = ArxivWatermarkRegex.Match(searchArea);
        if (!m.Success || m.Length == 0) return text;

        string after = searchArea.Substring(m.Index + m.Length);
        char? beforeChar = m.Index > 0 ? searchArea[m.Index - 1] : null;
        bool atBoundary = beforeChar == '.' || after.StartsWith("\n", StringComparison.Ordinal);
        if (!atBoundary) return text;

        return text.Remove(m.Index, m.Length);
    }

    private static string ReplaceFirst(string input, string oldValue, string newValue)
    {
        int idx = input.IndexOf(oldValue, StringComparison.Ordinal);
        if (idx < 0) return input;
        return input.Substring(0, idx) + newValue + input.Substring(idx + oldValue.Length);
    }

    // ------------------------------------------------------------------
    // String post-processing helpers (ported verbatim + unit-tested).
    // ------------------------------------------------------------------

    /// <summary>Drops the backslash before any target char (single pass).</summary>
    public static string UnescapeBackslashSequences(string input, char[] targets)
    {
        int firstHit = -1;
        for (int i = 0; i + 1 < input.Length; i++)
        {
            if (input[i] == '\\' && Array.IndexOf(targets, input[i + 1]) >= 0)
            {
                firstHit = i;
                break;
            }
        }
        if (firstHit < 0) return input;

        var sb = new StringBuilder(input.Length);
        sb.Append(input, 0, firstHit);
        int idx = firstHit;
        while (idx < input.Length)
        {
            if (idx + 1 < input.Length && input[idx] == '\\' && Array.IndexOf(targets, input[idx + 1]) >= 0)
            {
                sb.Append(input[idx + 1]);
                idx += 2;
                continue;
            }
            sb.Append(input[idx]);
            idx++;
        }
        return sb.ToString();
    }

    /// <summary>`&amp;#10;` → space, `&amp;#2;` → removed (single pass).</summary>
    public static string ReplaceHtmlEntities(string input)
    {
        if (!input.Contains("&#10;") && !input.Contains("&#2;")) return input;

        var sb = new StringBuilder(input.Length);
        string rest = input;
        while (rest.Length > 0)
        {
            int pos = rest.IndexOf("&#", StringComparison.Ordinal);
            if (pos < 0)
            {
                sb.Append(rest);
                break;
            }
            sb.Append(rest, 0, pos);
            string after = rest.Substring(pos);
            if (after.StartsWith("&#10;", StringComparison.Ordinal))
            {
                sb.Append(' ');
                rest = after.Substring(5);
            }
            else if (after.StartsWith("&#2;", StringComparison.Ordinal))
            {
                rest = after.Substring(4);
            }
            else
            {
                sb.Append("&#");
                rest = after.Substring(2);
            }
        }
        return sb.ToString();
    }

    /// <summary>Collapses runs of 3+ newlines down to exactly two (single pass).</summary>
    public static string CollapseExcessNewlines(string input)
    {
        if (!input.Contains("\n\n\n")) return input;

        var sb = new StringBuilder(input.Length);
        int newlineRun = 0;
        foreach (var c in input)
        {
            if (c == '\n')
            {
                newlineRun++;
                if (newlineRun <= 2) sb.Append('\n');
            }
            else
            {
                newlineRun = 0;
                sb.Append(c);
            }
        }
        return sb.ToString();
    }
}
