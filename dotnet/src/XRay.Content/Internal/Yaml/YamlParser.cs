using System.Globalization;
using System.Text;
using System.Text.Json.Nodes;

namespace XRay.Content.Internal.Yaml;

/// <summary>
/// Minimal YAML parser producing a <see cref="JsonNode"/> tree, sufficient for the structured
/// extractor's metadata walk (block mappings, block sequences, scalars with type inference,
/// simple flow collections). Not a full YAML implementation.
/// </summary>
internal static class YamlParser
{
    private readonly record struct Line(int Indent, string Content);

    public static JsonNode? Parse(string text)
    {
        var lines = new List<Line>();
        int lineCounter = 0;
        foreach (var raw in JoinQuotedContinuations(text.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n')))
        {
            Cancel.Every(lineCounter++, 0x3FF);
            string stripped = StripComment(raw);
            string trimmed = stripped.TrimEnd();
            if (trimmed.Trim().Length == 0) continue;
            string bare = trimmed.Trim();
            if (bare == "---" || bare == "...") continue;
            int indent = 0;
            while (indent < trimmed.Length && trimmed[indent] == ' ') indent++;
            lines.Add(new Line(indent, trimmed.Substring(indent)));
        }

        int pos = 0;
        return ParseBlock(lines, ref pos, -1);
    }

    private static JsonNode? ParseBlock(List<Line> lines, ref int pos, int parentIndent)
    {
        if (pos >= lines.Count) return null;
        int curIndent = lines[pos].Indent;
        if (curIndent <= parentIndent) return null;

        return IsSeqItem(lines[pos].Content)
            ? ParseSeq(lines, ref pos, curIndent)
            : ParseMap(lines, ref pos, curIndent);
    }

    private static JsonObject ParseMap(List<Line> lines, ref int pos, int curIndent)
    {
        var obj = new JsonObject();
        while (pos < lines.Count && lines[pos].Indent == curIndent && !IsSeqItem(lines[pos].Content))
        {
            string content = lines[pos].Content;
            var (key, value) = SplitKeyValue(content);
            pos++;

            if (value.Length == 0)
            {
                if (pos < lines.Count && IsSeqItem(lines[pos].Content) && lines[pos].Indent >= curIndent)
                    obj[key] = ParseSeq(lines, ref pos, lines[pos].Indent);
                else
                    obj[key] = ParseBlock(lines, ref pos, curIndent);
            }
            else
            {
                obj[key] = ParseScalarOrFlow(value);
            }
        }
        return obj;
    }

    private static JsonArray ParseSeq(List<Line> lines, ref int pos, int seqIndent)
    {
        var arr = new JsonArray();
        while (pos < lines.Count && lines[pos].Indent == seqIndent && IsSeqItem(lines[pos].Content))
        {
            string content = lines[pos].Content;
            string rest = content.Length == 1 ? "" : content.Substring(1).Trim();

            if (rest.Length == 0)
            {
                pos++;
                arr.Add(ParseBlock(lines, ref pos, seqIndent));
            }
            else if (rest == "-" || rest.StartsWith("- ", StringComparison.Ordinal))
            {
                // Nested sequence ("- - item"): the inner sequence starts at column seqIndent+2.
                lines[pos] = new Line(seqIndent + 2, rest);
                arr.Add(ParseSeq(lines, ref pos, seqIndent + 2));
            }
            else if (HasKeyColon(rest))
            {
                // Sequence item that is a mapping ("- key: value"). The item's content begins at
                // column seqIndent+2; rewrite this line as a map line there and parse the map.
                lines[pos] = new Line(seqIndent + 2, rest);
                arr.Add(ParseMap(lines, ref pos, seqIndent + 2));
            }
            else
            {
                pos++;
                arr.Add(ParseScalarOrFlow(rest));
            }
        }
        return arr;
    }

    private static bool IsSeqItem(string content) =>
        content == "-" || content.StartsWith("- ", StringComparison.Ordinal);

    // True when the string begins with a "key:" (colon followed by space or end), respecting quotes.
    private static bool HasKeyColon(string content)
    {
        bool inS = false, inD = false;
        for (int i = 0; i < content.Length; i++)
        {
            char c = content[i];
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (c == ':' && !inS && !inD && (i + 1 >= content.Length || content[i + 1] == ' '))
                return i > 0;
        }
        return false;
    }

    private static (string Key, string Value) SplitKeyValue(string content)
    {
        // Find the first colon that ends the key (followed by space or end), respecting quotes.
        bool inS = false, inD = false;
        for (int i = 0; i < content.Length; i++)
        {
            char c = content[i];
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (c == ':' && !inS && !inD && (i + 1 >= content.Length || content[i + 1] == ' '))
            {
                string key = Unquote(content.Substring(0, i).Trim());
                string val = content.Substring(i + 1).Trim();
                return (key, val);
            }
        }
        return (Unquote(content.Trim()), "");
    }

    private static JsonNode? ParseScalarOrFlow(string s)
    {
        s = s.Trim();
        if (s == "[]") return new JsonArray();
        if (s == "{}") return new JsonObject();

        if (s.Length >= 2 && s[0] == '[' && s[^1] == ']')
        {
            var arr = new JsonArray();
            foreach (var part in SplitFlow(s.Substring(1, s.Length - 2)))
                arr.Add(ParseScalarOrFlow(part));
            return arr;
        }
        if (s.Length >= 2 && s[0] == '{' && s[^1] == '}')
        {
            var obj = new JsonObject();
            foreach (var part in SplitFlow(s.Substring(1, s.Length - 2)))
            {
                var (k, v) = SplitKeyValue(part.Trim());
                obj[k] = ParseScalarOrFlow(v);
            }
            return obj;
        }
        return ParseScalar(s);
    }

    private static JsonNode? ParseScalar(string s)
    {
        s = s.Trim();
        if (s.Length >= 2 && s[0] == '"' && s[^1] == '"')
            return JsonValue.Create(Unescape(s.Substring(1, s.Length - 2)));
        if (s.Length >= 2 && s[0] == '\'' && s[^1] == '\'')
            return JsonValue.Create(s.Substring(1, s.Length - 2).Replace("''", "'"));

        if (s.Length == 0 || s == "~" || s.Equals("null", StringComparison.OrdinalIgnoreCase)) return null;
        if (s.Equals("true", StringComparison.OrdinalIgnoreCase)) return JsonValue.Create(true);
        if (s.Equals("false", StringComparison.OrdinalIgnoreCase)) return JsonValue.Create(false);

        // A number keeps the lexeme it was written with. Two things depend on that: a count above
        // long.MaxValue — a 64-bit hash, say — stays an integer instead of degrading to a double
        // and printing as 1.4550011543526097e19; and "397.0" stays a float rather than becoming
        // the integer 397. Both are what the value says it is, and the renderer decides integer
        // from float by looking at the text.
        if (LooksNumeric(s))
        {
            // LooksNumeric only says the characters are number-ish: a bare "-", "1.2.3" and
            // "1e999" all reach here and are not JSON numbers, so a parse failure is expected
            // and simply falls through to the looser handling below.
            try
            {
                // JSON number syntax admits magnitudes a double cannot hold. "1e999" is a valid
                // lexeme and no representable number, and letting it through reaches the text as
                // "Infinity.0". Keeping the scalar as written at least says what the document
                // said. An integer lexeme is exempt: it is exact however long it is, and that is
                // the whole point of preserving it.
                bool isFloatForm = s.IndexOfAny(new[] { '.', 'e', 'E' }) >= 0;
                if (!isFloatForm
                    || (double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out double magnitude)
                        && double.IsFinite(magnitude)))
                {
                    if (JsonNode.Parse(s) is JsonValue parsed) return parsed;
                }
            }
            catch (System.Text.Json.JsonException) { }
            catch (OverflowException) { }
        }

        // JSON number syntax is narrower than YAML's — it has no leading `+`, no bare `.5`. Those
        // are still numbers; they just cannot carry their lexeme through.
        if (long.TryParse(s, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out long l))
            return JsonValue.Create(l);
        // A magnitude past the range of a double parses as infinity, which is not a number any
        // output format can print — it would reach the text as "Infinity.0". The scalar is kept
        // as the text it was written as, which at least says what the document said.
        if (LooksNumeric(s) &&
            double.TryParse(s, NumberStyles.Float, CultureInfo.InvariantCulture, out double d)
            && double.IsFinite(d))
            return JsonValue.Create(d);

        return JsonValue.Create(s);
    }

    private static bool LooksNumeric(string s)
    {
        foreach (char c in s)
            if (!(char.IsDigit(c) || c is '.' or '-' or '+' or 'e' or 'E')) return false;
        return s.Length > 0 && (char.IsDigit(s[0]) || s[0] is '-' or '+' or '.');
    }

    private static IEnumerable<string> SplitFlow(string s)
    {
        var parts = new List<string>();
        bool inS = false, inD = false;
        int depth = 0, start = 0;
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (!inS && !inD)
            {
                if (c is '[' or '{') depth++;
                else if (c is ']' or '}') depth--;
                else if (c == ',' && depth == 0)
                {
                    parts.Add(s.Substring(start, i - start));
                    start = i + 1;
                }
            }
        }
        string last = s.Substring(start);
        if (last.Trim().Length > 0 || parts.Count > 0) parts.Add(last);
        return parts.Where(p => p.Trim().Length > 0);
    }

    private static string Unquote(string s)
    {
        if (s.Length >= 2 && ((s[0] == '"' && s[^1] == '"') || (s[0] == '\'' && s[^1] == '\'')))
            return s.Substring(1, s.Length - 2);
        return s;
    }

    /// <summary>
    /// Fold a double-quoted scalar that runs past the end of its line back onto one line
    /// (YAML 1.2 §7.3.1), so the line-oriented parse below sees a single complete scalar.
    /// </summary>
    /// <remarks>
    /// A break inside such a scalar becomes one space, and a break the writer escaped with a
    /// trailing backslash becomes nothing at all — which is how a long sentence gets wrapped
    /// without gaining or losing a space. Without this the scalar never closes and everything
    /// after it parses as though it were still inside the string.
    /// </remarks>
    private static List<string> JoinQuotedContinuations(string[] raw)
    {
        var joined = new List<string>(raw.Length);
        for (int i = 0; i < raw.Length; i++)
        {
            string line = raw[i];
            while (EndsInsideDoubleQuote(line) && i + 1 < raw.Length)
            {
                string next = raw[++i].TrimStart();
                if (line.EndsWith('\\')) line = line[..^1] + next;
                else line = line + " " + next;
            }
            joined.Add(line);
        }
        return joined;
    }

    /// <summary>Whether a double-quoted scalar opened on this line is still open at its end.</summary>
    private static bool EndsInsideDoubleQuote(string line)
    {
        bool inS = false, inD = false;
        for (int i = 0; i < line.Length; i++)
        {
            char c = line[i];
            if (inD && c == '\\') { i++; continue; }
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (c == '#' && !inS && !inD && (i == 0 || char.IsWhiteSpace(line[i - 1]))) return false;
        }
        return inD;
    }

    private static string StripComment(string line)
    {
        bool inS = false, inD = false;
        for (int i = 0; i < line.Length; i++)
        {
            char c = line[i];
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (c == '#' && !inS && !inD && (i == 0 || char.IsWhiteSpace(line[i - 1])))
                return line.Substring(0, i);
        }
        return line;
    }

    /// <summary>
    /// Resolve the escape sequences a YAML double-quoted scalar may carry (YAML 1.2 §5.7).
    /// </summary>
    /// <remarks>
    /// The numeric forms matter as much as the named ones: a document that writes a curly
    /// apostrophe as <c>’</c> is common, and leaving it unresolved puts the literal
    /// characters <c>u2019</c> into the text.
    /// </remarks>
    private static string Unescape(string s)
    {
        var sb = new StringBuilder(s.Length);
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (c != '\\' || i + 1 >= s.Length) { sb.Append(c); continue; }

            char n = s[++i];
            switch (n)
            {
                case '0': sb.Append('\0'); break;
                case 'a': sb.Append('\a'); break;
                case 'b': sb.Append('\b'); break;
                case 't': case '\t': sb.Append('\t'); break;
                case 'n': sb.Append('\n'); break;
                case 'v': sb.Append('\v'); break;
                case 'f': sb.Append('\f'); break;
                case 'r': sb.Append('\r'); break;
                case 'e': sb.Append('\u001B'); break;
                case 'N': sb.Append('\u0085'); break;   // next line
                case '_': sb.Append('\u00A0'); break;   // non-breaking space
                case 'L': sb.Append('\u2028'); break;   // line separator
                case 'P': sb.Append('\u2029'); break;   // paragraph separator
                case 'x': AppendCodepoint(sb, s, ref i, 2); break;
                case 'u': AppendCodepoint(sb, s, ref i, 4); break;
                case 'U': AppendCodepoint(sb, s, ref i, 8); break;
                default: sb.Append(n); break;           // \" \\ \/ \<space> and anything unknown
            }
        }
        return sb.ToString();
    }

    /// <summary>
    /// Consume <paramref name="digits"/> hex digits after an <c>\x</c>/<c>\u</c>/<c>\U</c> escape
    /// and append the code point; a malformed run is left as written.
    /// </summary>
    private static void AppendCodepoint(StringBuilder sb, string s, ref int i, int digits)
    {
        if (i + digits >= s.Length
            || !uint.TryParse(s.AsSpan(i + 1, digits), System.Globalization.NumberStyles.HexNumber,
                              System.Globalization.CultureInfo.InvariantCulture, out uint value)
            || value > 0x10FFFF
            || (value >= 0xD800 && value <= 0xDFFF))
        {
            sb.Append(s[i]);
            return;
        }
        sb.Append(char.ConvertFromUtf32((int)value));
        i += digits;
    }
}
