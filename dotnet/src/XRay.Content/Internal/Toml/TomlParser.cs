using System.Globalization;
using System.Text;
using System.Text.Json.Nodes;

namespace XRay.Content.Internal.Toml;

/// <summary>
/// Minimal TOML parser producing a <see cref="JsonNode"/> tree, sufficient for the structured
/// extractor's metadata walk: comments, tables <c>[a.b]</c>, array-of-tables <c>[[a]]</c>,
/// bare/quoted/dotted keys, basic/literal/multiline strings, integers, floats, booleans,
/// datetimes (kept as strings), arrays, and inline tables.
/// </summary>
internal static class TomlParser
{
    /// <summary>The key a TOML datetime is serialized under when it becomes JSON.</summary>
    private const string DatetimeKey = "$__toml_private_datetime";

    public static JsonObject Parse(string text)
    {
        var root = new JsonObject();
        JsonObject current = root;

        var lines = text.Replace("\r\n", "\n").Replace('\r', '\n').Split('\n');
        int i = 0;
        while (i < lines.Length)
        {
            Cancel.ThrowIfRequested();
            string line = StripComment(lines[i]).Trim();
            i++;
            if (line.Length == 0) continue;

            if (line.StartsWith("[[", StringComparison.Ordinal))
            {
                string name = line.Substring(2, line.IndexOf("]]", StringComparison.Ordinal) - 2).Trim();
                current = NavigateTable(root, ParseKeyPath(name), arrayOfTables: true);
            }
            else if (line.StartsWith("[", StringComparison.Ordinal))
            {
                string name = line.Substring(1, line.IndexOf(']') - 1).Trim();
                current = NavigateTable(root, ParseKeyPath(name), arrayOfTables: false);
            }
            else
            {
                int eq = FindAssignment(line);
                if (eq < 0) continue;
                string keyPart = line.Substring(0, eq).Trim();
                string rhs = line.Substring(eq + 1).Trim();

                // Assemble continuation lines for unterminated arrays / inline tables / multiline strings.
                while (NeedsContinuation(rhs) && i < lines.Length)
                {
                    rhs += "\n" + StripComment(lines[i]);
                    i++;
                }

                var value = ParseValue(rhs.Trim());
                SetKeyPath(current, ParseKeyPath(keyPart), value);
            }
        }

        return SortKeys(root);
    }

    /// <summary>
    /// Rebuild every table with its keys in sorted order.
    /// </summary>
    /// <remarks>
    /// A TOML table is a sorted map, not an ordered one: two documents that differ only in the
    /// order their keys were written parse to the same value, and the serialized form is sorted.
    /// Preserving file order instead makes the extracted text depend on how the file was typed.
    /// </remarks>
    private static JsonObject SortKeys(JsonObject table)
    {
        var sorted = new JsonObject();
        foreach (var key in table.Select(kv => kv.Key).OrderBy(k => k, StringComparer.Ordinal))
        {
            var value = table[key];
            table[key] = null;   // detach before re-parenting; a node has one parent
            sorted[key] = SortNode(value);
        }
        return sorted;
    }

    private static JsonNode? SortNode(JsonNode? node) => node switch
    {
        JsonObject obj => SortKeys(obj),
        JsonArray arr => SortArray(arr),
        _ => node,
    };

    private static JsonArray SortArray(JsonArray array)
    {
        var rebuilt = new JsonArray();
        for (int idx = 0; idx < array.Count; idx++)
        {
            var item = array[idx];
            array[idx] = null;
            rebuilt.Add(SortNode(item));
        }
        return rebuilt;
    }

    // ── table navigation ──────────────────────────────────────────────────

    private static JsonObject NavigateTable(JsonObject root, List<string> segments, bool arrayOfTables)
    {
        JsonObject node = root;
        for (int i = 0; i < segments.Count - 1; i++)
        {
            string seg = segments[i];
            var child = node[seg];
            if (child is JsonArray arr && arr.Count > 0 && arr[^1] is JsonObject ao) node = ao;
            else if (child is JsonObject o) node = o;
            else { var n = new JsonObject(); node[seg] = n; node = n; }
        }

        string last = segments[^1];
        if (arrayOfTables)
        {
            if (node[last] is not JsonArray arr) { arr = new JsonArray(); node[last] = arr; }
            var entry = new JsonObject();
            arr.Add(entry);
            return entry;
        }
        if (node[last] is JsonObject existing) return existing;
        var created = new JsonObject();
        node[last] = created;
        return created;
    }

    private static void SetKeyPath(JsonObject table, List<string> segments, JsonNode? value)
    {
        JsonObject node = table;
        for (int i = 0; i < segments.Count - 1; i++)
        {
            string seg = segments[i];
            if (node[seg] is JsonObject o) node = o;
            else { var n = new JsonObject(); node[seg] = n; node = n; }
        }
        node[segments[^1]] = value;
    }

    private static int FindAssignment(string line)
    {
        bool inS = false, inD = false;
        for (int i = 0; i < line.Length; i++)
        {
            char c = line[i];
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (c == '=' && !inS && !inD) return i;
        }
        return -1;
    }

    private static List<string> ParseKeyPath(string s)
    {
        var segs = new List<string>();
        var sb = new StringBuilder();
        bool inS = false, inD = false;
        for (int i = 0; i < s.Length; i++)
        {
            char c = s[i];
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (c == '.' && !inS && !inD) { segs.Add(TrimKey(sb.ToString())); sb.Clear(); }
            else sb.Append(c);
        }
        segs.Add(TrimKey(sb.ToString()));
        return segs;
    }

    private static string TrimKey(string s)
    {
        s = s.Trim();
        if (s.Length >= 2 && ((s[0] == '"' && s[^1] == '"') || (s[0] == '\'' && s[^1] == '\'')))
            return s.Substring(1, s.Length - 2);
        return s;
    }

    // ── value parsing ───────────────────────────────────────────────────────

    private static bool NeedsContinuation(string rhs)
    {
        bool inS = false, inD = false;
        int depth = 0;
        for (int i = 0; i < rhs.Length; i++)
        {
            char c = rhs[i];
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (!inS && !inD)
            {
                if (c is '[' or '{') depth++;
                else if (c is ']' or '}') depth--;
            }
        }
        return depth > 0;
    }

    private static JsonNode? ParseValue(string s)
    {
        int i = 0;
        var v = ParseValueAt(s, ref i);
        return v;
    }

    private static JsonNode? ParseValueAt(string s, ref int i)
    {
        SkipWs(s, ref i);
        if (i >= s.Length) return null;
        char c = s[i];

        if (c == '"' || c == '\'') return ParseString(s, ref i);
        if (c == '[') return ParseArray(s, ref i);
        if (c == '{') return ParseInlineTable(s, ref i);

        // Bare token: number / bool / datetime.
        int start = i;
        while (i < s.Length && s[i] != ',' && s[i] != ']' && s[i] != '}' && s[i] != '\n') i++;
        string tok = s.Substring(start, i - start).Trim();
        return ParseBareToken(tok);
    }

    private static JsonNode ParseString(string s, ref int i)
    {
        char q = s[i];
        bool triple = i + 2 < s.Length && s[i + 1] == q && s[i + 2] == q;
        if (triple)
        {
            i += 3;
            int start = i;
            while (i + 2 < s.Length && !(s[i] == q && s[i + 1] == q && s[i + 2] == q)) i++;
            string body = s.Substring(start, i - start);
            i = Math.Min(s.Length, i + 3);
            // Trim a leading newline immediately after the opening delimiter (TOML rule).
            if (body.StartsWith("\n", StringComparison.Ordinal)) body = body.Substring(1);
            return JsonValue.Create(q == '"' ? Unescape(body) : body)!;
        }

        i++; // opening quote
        int s0 = i;
        if (q == '\'')
        {
            while (i < s.Length && s[i] != '\'') i++;
            string body = s.Substring(s0, i - s0);
            if (i < s.Length) i++;
            return JsonValue.Create(body)!;
        }

        var sb = new StringBuilder();
        while (i < s.Length && s[i] != '"')
        {
            if (s[i] == '\\' && i + 1 < s.Length) { sb.Append(UnescapeChar(s, ref i)); }
            else sb.Append(s[i++]);
        }
        if (i < s.Length) i++;
        return JsonValue.Create(sb.ToString())!;
    }

    private static JsonArray ParseArray(string s, ref int i)
    {
        var arr = new JsonArray();
        i++; // '['
        while (true)
        {
            SkipWsAndSeparators(s, ref i);
            if (i >= s.Length || s[i] == ']') { if (i < s.Length) i++; break; }
            arr.Add(ParseValueAt(s, ref i));
            SkipWsAndSeparators(s, ref i);
            if (i < s.Length && s[i] == ',') i++;
        }
        return arr;
    }

    private static JsonObject ParseInlineTable(string s, ref int i)
    {
        var obj = new JsonObject();
        i++; // '{'
        while (true)
        {
            SkipWs(s, ref i);
            if (i >= s.Length || s[i] == '}') { if (i < s.Length) i++; break; }

            int keyStart = i;
            bool inS = false, inD = false;
            while (i < s.Length)
            {
                char c = s[i];
                if (c == '\'' && !inD) inS = !inS;
                else if (c == '"' && !inS) inD = !inD;
                else if (c == '=' && !inS && !inD) break;
                i++;
            }
            string keyPart = s.Substring(keyStart, i - keyStart).Trim();
            if (i < s.Length) i++; // '='
            var value = ParseValueAt(s, ref i);
            SetKeyPath(obj, ParseKeyPath(keyPart), value);

            SkipWs(s, ref i);
            if (i < s.Length && s[i] == ',') i++;
        }
        return obj;
    }

    private static JsonNode? ParseBareToken(string tok)
    {
        if (tok.Length == 0) return null;
        if (tok == "true") return JsonValue.Create(true);
        if (tok == "false") return JsonValue.Create(false);

        // A datetime is its own TOML type, not a string. Serialized to JSON it becomes a
        // one-entry table under the reserved key the reference serializer uses, which is how a
        // consumer can tell a timestamp from a string that looks like one.
        if (tok.Length >= 8 && char.IsDigit(tok[0]) && (LooksDate(tok) || LooksTime(tok)))
            return new JsonObject { [DatetimeKey] = JsonValue.Create(tok) };

        string cleaned = tok.Replace("_", "");
        if (TryParseInteger(cleaned, out long l)) return JsonValue.Create(l);
        // A float keeps the lexeme it was written with, so 80.0 stays a float rather than
        // becoming the integer 80 — TOML distinguishes the two types, and so does the output.
        // JSON has no leading `+`; dropping it changes nothing about the value.
        string jsonForm = cleaned.StartsWith('+') ? cleaned[1..] : cleaned;
        if (jsonForm.IndexOfAny(new[] { '.', 'e', 'E' }) >= 0
            && double.TryParse(jsonForm, NumberStyles.Float, CultureInfo.InvariantCulture, out double magnitude)
            && double.IsFinite(magnitude))
        {
            try
            {
                if (JsonNode.Parse(jsonForm) is JsonValue parsed) return parsed;
            }
            catch (System.Text.Json.JsonException) { }
        }
        if (double.TryParse(cleaned, NumberStyles.Float, CultureInfo.InvariantCulture, out double d))
            return JsonValue.Create(d);
        if (cleaned is "inf" or "+inf") return JsonValue.Create(double.PositiveInfinity);
        if (cleaned == "-inf") return JsonValue.Create(double.NegativeInfinity);
        if (cleaned is "nan" or "+nan" or "-nan") return JsonValue.Create(double.NaN);

        return JsonValue.Create(tok);
    }

    private static bool TryParseInteger(string s, out long value)
    {
        value = 0;
        if (s.Length == 0) return false;
        try
        {
            if (s.StartsWith("0x", StringComparison.Ordinal)) { value = Convert.ToInt64(s.Substring(2), 16); return true; }
            if (s.StartsWith("0o", StringComparison.Ordinal)) { value = Convert.ToInt64(s.Substring(2), 8); return true; }
            if (s.StartsWith("0b", StringComparison.Ordinal)) { value = Convert.ToInt64(s.Substring(2), 2); return true; }
        }
        catch (Exception) when (Cancel.NotCancelling) { return false; }
        return long.TryParse(s, NumberStyles.AllowLeadingSign, CultureInfo.InvariantCulture, out value);
    }

    private static bool LooksDate(string t) =>
        t.Length >= 10 && t[4] == '-' && t[7] == '-' && char.IsDigit(t[0]);

    private static bool LooksTime(string t) =>
        t.Length >= 8 && t[2] == ':' && t[5] == ':' && char.IsDigit(t[0]);

    private static void SkipWs(string s, ref int i)
    {
        while (i < s.Length && (s[i] == ' ' || s[i] == '\t')) i++;
    }

    private static void SkipWsAndSeparators(string s, ref int i)
    {
        while (i < s.Length && (s[i] == ' ' || s[i] == '\t' || s[i] == '\n' || s[i] == '\r')) i++;
    }

    private static string UnescapeChar(string s, ref int i)
    {
        i++; // backslash
        char n = s[i++];
        switch (n)
        {
            case 'n': return "\n";
            case 't': return "\t";
            case 'r': return "\r";
            case '"': return "\"";
            case '\\': return "\\";
            case 'b': return "\b";
            case 'f': return "\f";
            case 'u':
                string hex4 = s.Substring(i, Math.Min(4, s.Length - i)); i += 4;
                return char.ConvertFromUtf32(Convert.ToInt32(hex4, 16));
            case 'U':
                string hex8 = s.Substring(i, Math.Min(8, s.Length - i)); i += 8;
                return char.ConvertFromUtf32(Convert.ToInt32(hex8, 16));
            default: return n.ToString();
        }
    }

    private static string Unescape(string s)
    {
        var sb = new StringBuilder(s.Length);
        int i = 0;
        while (i < s.Length)
        {
            if (s[i] == '\\' && i + 1 < s.Length) sb.Append(UnescapeChar(s, ref i));
            else sb.Append(s[i++]);
        }
        return sb.ToString();
    }

    private static string StripComment(string line)
    {
        bool inS = false, inD = false;
        for (int i = 0; i < line.Length; i++)
        {
            char c = line[i];
            if (c == '\'' && !inD) inS = !inS;
            else if (c == '"' && !inS) inD = !inD;
            else if (c == '#' && !inS && !inD) return line.Substring(0, i);
        }
        return line;
    }
}
