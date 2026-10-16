// Ported from crates/xberg/src/extraction/email.rs — header decoding that `mail_parser`
// performs (RFC 2047 encoded-words, address `.address()` extraction, Content-Type params).
// Native MIME parser.
using System.Text;

namespace XRay.Content.Internal.Email;

/// <summary>Header-level helpers: RFC 2047 encoded-word decoding, address and Content-Type parsing.</summary>
internal static class HeaderDecoder
{
    /// <summary>
    /// Decode an RFC 2047 header value (<c>=?charset?B|Q?text?=</c>). Runs of adjacent
    /// encoded-words separated only by whitespace are concatenated with the whitespace
    /// dropped (per RFC 2047 §6.2). Non-encoded text is passed through unchanged.
    /// </summary>
    internal static string DecodeEncodedWords(string input)
    {
        if (input.IndexOf("=?", StringComparison.Ordinal) < 0)
            return input;

        var sb = new StringBuilder(input.Length);
        int i = 0;
        int n = input.Length;
        bool prevWasEncoded = false;

        while (i < n)
        {
            int start = input.IndexOf("=?", i, StringComparison.Ordinal);
            if (start < 0)
            {
                sb.Append(input, i, n - i);
                break;
            }

            // Emit the literal gap before this token.
            string gap = input.Substring(i, start - i);
            if (!(prevWasEncoded && gap.Trim().Length == 0))
                sb.Append(gap);

            if (TryDecodeOneWord(input, start, out string decoded, out int end))
            {
                sb.Append(decoded);
                i = end;
                prevWasEncoded = true;
            }
            else
            {
                sb.Append("=?");
                i = start + 2;
                prevWasEncoded = false;
            }
        }

        return sb.ToString();
    }

    private static bool TryDecodeOneWord(string s, int start, out string decoded, out int end)
    {
        decoded = "";
        end = start;
        // Layout: =?charset?enc?text?=
        int p1 = s.IndexOf('?', start + 2);
        if (p1 < 0) return false;
        int p2 = s.IndexOf('?', p1 + 1);
        if (p2 < 0 || p2 != p1 + 2) return false; // encoding is a single char
        int p3 = s.IndexOf("?=", p2 + 1, StringComparison.Ordinal);
        if (p3 < 0) return false;

        string charset = s.Substring(start + 2, p1 - (start + 2));
        char enc = char.ToUpperInvariant(s[p1 + 1]);
        string text = s.Substring(p2 + 1, p3 - (p2 + 1));

        byte[] bytes;
        if (enc == 'B')
        {
            try { bytes = ContentTransferDecoder.DecodeBase64(text); }
            catch (Exception) when (Cancel.NotCancelling) { return false; }
        }
        else if (enc == 'Q')
        {
            bytes = DecodeQEncoding(text);
        }
        else
        {
            return false;
        }

        decoded = CharsetDecoder.Decode(charset, bytes);
        end = p3 + 2;
        return true;
    }

    // RFC 2047 "Q" encoding: like quoted-printable but '_' means space.
    private static byte[] DecodeQEncoding(string text)
    {
        var outBytes = new List<byte>(text.Length);
        int i = 0;
        while (i < text.Length)
        {
            char c = text[i];
            if (c == '_')
            {
                outBytes.Add((byte)' ');
                i++;
            }
            else if (c == '=' && i + 2 < text.Length && TryHex(text[i + 1], out int hi) && TryHex(text[i + 2], out int lo))
            {
                outBytes.Add((byte)((hi << 4) | lo));
                i += 3;
            }
            else
            {
                outBytes.Add((byte)(c & 0xFF));
                i++;
            }
        }
        return outBytes.ToArray();
    }

    private static bool TryHex(char c, out int value)
    {
        if (c >= '0' && c <= '9') { value = c - '0'; return true; }
        if (c >= 'A' && c <= 'F') { value = c - 'A' + 10; return true; }
        if (c >= 'a' && c <= 'f') { value = c - 'a' + 10; return true; }
        value = 0;
        return false;
    }

    /// <summary>
    /// Extract email addresses from an address-list header (From/To/Cc/Bcc), mirroring
    /// `mail_parser` `Address::address()` which returns the bare address for each entry.
    /// Splits on top-level commas (respecting quotes and angle brackets), decodes any
    /// encoded-words in display names, and returns the address inside <c>&lt;...&gt;</c> or the
    /// bare token when no angle brackets are present.
    /// </summary>
    internal static List<string> ExtractAddresses(string headerValue)
    {
        var result = new List<string>();
        foreach (string token in SplitTopLevelCommas(headerValue))
        {
            string t = token.Trim();
            if (t.Length == 0) continue;

            int lt = t.IndexOf('<');
            if (lt >= 0)
            {
                int gt = t.IndexOf('>', lt + 1);
                if (gt > lt)
                {
                    string addr = t.Substring(lt + 1, gt - lt - 1).Trim();
                    if (addr.Length > 0) result.Add(addr);
                    continue;
                }
            }

            // No angle brackets: the whole token is the address (strip surrounding quotes).
            string bare = t.Trim('"').Trim();
            if (bare.Length > 0) result.Add(bare);
        }
        return result;
    }

    /// <summary>
    /// The display name of the first mailbox in an address-list header, mirroring
    /// <c>mail_parser</c>'s <c>Address::name()</c>: the text before the angle brackets, with any
    /// surrounding quotes removed. Null when the mailbox is a bare address.
    /// </summary>
    internal static string? ExtractFirstDisplayName(string headerValue)
    {
        foreach (string token in SplitTopLevelCommas(headerValue))
        {
            string t = token.Trim();
            if (t.Length == 0) continue;

            int lt = t.IndexOf('<');
            if (lt <= 0) return null;
            string name = t.Substring(0, lt).Trim().Trim('"').Trim();
            return name.Length == 0 ? null : name;
        }
        return null;
    }

    private static IEnumerable<string> SplitTopLevelCommas(string s)
    {
        var current = new StringBuilder();
        bool inQuotes = false;
        int angle = 0;
        foreach (char c in s)
        {
            switch (c)
            {
                case '"':
                    inQuotes = !inQuotes;
                    current.Append(c);
                    break;
                case '<' when !inQuotes:
                    angle++;
                    current.Append(c);
                    break;
                case '>' when !inQuotes:
                    if (angle > 0) angle--;
                    current.Append(c);
                    break;
                case ',' when !inQuotes && angle == 0:
                    yield return current.ToString();
                    current.Clear();
                    break;
                default:
                    current.Append(c);
                    break;
            }
        }
        if (current.Length > 0) yield return current.ToString();
    }

    /// <summary>Strip a single pair of surrounding angle brackets (Message-ID normalization).</summary>
    internal static string StripAngleBrackets(string value)
    {
        string v = value.Trim();
        if (v.StartsWith('<') && v.EndsWith('>') && v.Length >= 2)
            return v.Substring(1, v.Length - 2).Trim();
        return v;
    }

    /// <summary>
    /// Parse a Content-Type / Content-Disposition value into a lower-cased primary token
    /// and a case-insensitive parameter map. Parameter values may be quoted.
    /// </summary>
    internal static (string Value, Dictionary<string, string> Params) ParseParameterized(string raw)
    {
        var parts = SplitSemicolons(raw);
        string value = parts.Count > 0 ? parts[0].Trim().ToLowerInvariant() : "";
        var dict = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        for (int i = 1; i < parts.Count; i++)
        {
            string p = parts[i].Trim();
            int eq = p.IndexOf('=');
            if (eq < 0) continue;
            string key = p.Substring(0, eq).Trim().ToLowerInvariant();
            string val = p.Substring(eq + 1).Trim();
            if (val.Length >= 2 && val[0] == '"' && val[^1] == '"')
                val = val.Substring(1, val.Length - 2);
            if (key.Length > 0) dict[key] = val;
        }
        return (value, dict);
    }

    private static List<string> SplitSemicolons(string s)
    {
        var result = new List<string>();
        var current = new StringBuilder();
        bool inQuotes = false;
        foreach (char c in s)
        {
            if (c == '"') { inQuotes = !inQuotes; current.Append(c); }
            else if (c == ';' && !inQuotes) { result.Add(current.ToString()); current.Clear(); }
            else current.Append(c);
        }
        if (current.Length > 0) result.Add(current.ToString());
        return result;
    }
}
