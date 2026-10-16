using System.Text.RegularExpressions;
using System.Xml.Linq;

namespace XRay.Content.Internal.Citation;

internal sealed class CitationAuthor
{
    public string Name = "";
    public string? GivenName;
}

internal sealed class Citation
{
    public string Title = "";
    public List<CitationAuthor> Authors = new();
    public int Year;
    public string? Doi;
    public string? Journal;
    public string? Volume;
    public string? Issue;
    public string? Pages;
    public string? Pmid;
    public string? AbstractText;
    public List<string> Keywords = new();
}

/// <summary>
/// Citation-format parsers (RIS, PubMed/MEDLINE, EndNote XML), a lightweight stand-in for the
/// Rust `biblib` crate. Only the fields consumed by the extractor are populated.
/// </summary>
internal static class CitationParser
{
    // ── RIS ────────────────────────────────────────────────────────────────

    public static List<Citation> ParseRis(string src)
    {
        var citations = new List<Citation>();
        Citation? cur = null;
        string startEp = "";
        foreach (var rawLine in src.Split('\n'))
        {
            string line = rawLine.TrimEnd('\r');
            var m = Regex.Match(line, @"^([A-Z0-9]{2})\s{0,2}-\s?(.*)$");
            if (!m.Success)
            {
                if (cur is not null && line.Trim().Length != 0 && cur.AbstractText is not null)
                    cur.AbstractText += " " + line.Trim();
                continue;
            }
            string tag = m.Groups[1].Value;
            string value = m.Groups[2].Value.Trim();
            switch (tag)
            {
                case "TY": cur = new Citation(); startEp = ""; citations.Add(cur); break;
                case "ER": cur = null; break;
                default:
                    if (cur is null) break;
                    ApplyRis(cur, tag, value, ref startEp);
                    break;
            }
        }
        // remove empty trailing entries (TY with no ER handled: kept)
        return citations;
    }

    private static void ApplyRis(Citation c, string tag, string value, ref string startPage)
    {
        switch (tag)
        {
            case "AU": case "A1": c.Authors.Add(ParseAuthor(value)); break;
            case "TI": case "T1": c.Title = value; break;
            case "KW": if (value.Length != 0) c.Keywords.Add(value); break;
            case "PY": case "Y1": case "DA": { int y = ExtractYear(value); if (y > 0 && c.Year == 0) c.Year = y; break; }
            case "DO": case "DI": c.Doi = value; break;
            case "JO": case "JF": case "T2": c.Journal ??= value; break;
            case "VL": c.Volume = value; break;
            case "IS": case "CP": c.Issue = value; break;
            case "SP": startPage = value; c.Pages = value; break;
            case "EP": c.Pages = startPage.Length != 0 ? $"{startPage}-{value}" : value; break;
            case "AB": case "N2": c.AbstractText = value; break;
        }
    }

    // ── PubMed / MEDLINE ─────────────────────────────────────────────────────

    public static List<Citation> ParsePubMed(string src)
    {
        var citations = new List<Citation>();
        Citation? cur = null;
        string lastTag = "";
        foreach (var rawLine in src.Split('\n'))
        {
            string line = rawLine.TrimEnd('\r');
            var m = Regex.Match(line, @"^([A-Z]{2,4})\s*-\s?(.*)$");
            if (!m.Success)
            {
                // MEDLINE folds any long value onto continuation lines indented under their tag,
                // not just the title; an abstract is the field that most often needs it, and
                // dropping the continuations truncated it at the first line break.
                if (line.StartsWith("      ", StringComparison.Ordinal) && cur is not null)
                {
                    string cont = line.Trim();
                    if (cont.Length != 0) AppendContinuation(cur, lastTag, cont);
                }
                continue;
            }
            string tag = m.Groups[1].Value;
            string value = m.Groups[2].Value.Trim();
            lastTag = tag;
            if (tag == "PMID")
            {
                cur = new Citation { Pmid = value };
                citations.Add(cur);
                continue;
            }
            if (cur is null) { cur = new Citation(); citations.Add(cur); }
            switch (tag)
            {
                case "TI": cur.Title = cur.Title.Length == 0 ? value : cur.Title + " " + value; break;
                case "FAU": case "AU": cur.Authors.Add(ParsePubMedAuthor(value)); break;
                case "DP": { int y = ExtractYear(value); if (y > 0) cur.Year = y; break; }
                case "LID": case "AID": if (value.Contains("10.")) cur.Doi ??= ExtractDoi(value); break;
                case "AB": cur.AbstractText = value; break;
                case "TA": case "JT": cur.Journal ??= value; break;
                case "MH": case "OT": if (value.Length != 0) cur.Keywords.Add(value); break;
            }
        }
        return citations;
    }

    /// <summary>Continue the field the previous tag opened, joining with a single space.</summary>
    private static void AppendContinuation(Citation citation, string lastTag, string text)
    {
        switch (lastTag)
        {
            case "TI": citation.Title = Join(citation.Title, text); break;
            case "AB": citation.AbstractText = Join(citation.AbstractText, text); break;
            case "TA":
            case "JT": citation.Journal = Join(citation.Journal, text); break;
        }

        static string Join(string? existing, string addition) =>
            string.IsNullOrEmpty(existing) ? addition : existing + " " + addition;
    }

    // ── EndNote XML ──────────────────────────────────────────────────────────

    public static List<Citation> ParseEndNoteXml(string src)
    {
        var citations = new List<Citation>();
        XDocument doc;
        try { doc = Cancel.ParseXml(src); }
        catch (Exception) when (Cancel.NotCancelling) { return citations; }
        foreach (var record in doc.Descendants("record"))
        {
            var c = new Citation();
            var title = record.Descendants("titles").Elements("title").FirstOrDefault();
            if (title is not null) c.Title = title.Value.Trim();
            foreach (var a in record.Descendants("authors").Elements("author"))
            {
                string v = a.Value.Trim();
                if (v.Length != 0) c.Authors.Add(ParseAuthor(v));
            }
            citations.Add(c);
        }
        return citations;
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    private static CitationAuthor ParseAuthor(string value)
    {
        int comma = value.IndexOf(',');
        if (comma >= 0)
        {
            string last = value.Substring(0, comma).Trim();
            string given = value.Substring(comma + 1).Trim();
            return new CitationAuthor { Name = last, GivenName = given.Length != 0 ? given : null };
        }
        return new CitationAuthor { Name = value.Trim() };
    }

    private static CitationAuthor ParsePubMedAuthor(string value)
    {
        int comma = value.IndexOf(',');
        if (comma >= 0) return ParseAuthor(value);
        int sp = value.LastIndexOf(' ');
        if (sp > 0)
        {
            string last = value.Substring(0, sp).Trim();
            string given = value.Substring(sp + 1).Trim();
            return new CitationAuthor { Name = last, GivenName = given.Length != 0 ? given : null };
        }
        return new CitationAuthor { Name = value.Trim() };
    }

    private static int ExtractYear(string value)
    {
        var m = Regex.Match(value, @"(\d{4})");
        return m.Success && int.TryParse(m.Groups[1].Value, out var y) ? y : 0;
    }

    private static string? ExtractDoi(string value)
    {
        var m = Regex.Match(value, @"10\.\S+");
        return m.Success ? m.Value.TrimEnd(']', ' ') : null;
    }
}
