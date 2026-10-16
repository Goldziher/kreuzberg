// Ported from crates/xberg/src/extraction/email.rs — `parse_eml_content`,
// `extract_raw_headers`, `extract_raw_date_header`, `build_metadata`, `parse_content_type`,
// `maybe_transcode_utf16`, and the `mail_parser` body/attachment classification.
// Native MIME parser (System.Net.Mail cannot parse raw MIME faithfully).
using System.Text;
using System.Text.RegularExpressions;

namespace XRay.Content.Internal.Email;

/// <summary>Parses raw .eml bytes into an <see cref="EmailExtractionResult"/>.</summary>
internal static class MimeParser
{
    private static readonly Encoding Utf8NoBom = new UTF8Encoding(false);

    /// <summary>Port of Rust `parse_eml_content`.</summary>
    internal static EmailExtractionResult ParseEmlContent(byte[] rawData)
    {
        byte[] data = MaybeTranscodeUtf16(rawData) ?? rawData;

        string latin1 = Encoding.Latin1.GetString(data);
        MimePart root = ParsePart(latin1, unterminated: false);

        string? subject = DecodeHeaderOpt(root.GetHeader("Subject"));

        string? fromHeader = root.GetHeader("From");
        var fromAddrs = AddressList(fromHeader);
        string? fromEmail = fromAddrs.Count > 0 ? fromAddrs[0] : null;
        string? fromName = fromHeader is null
            ? null
            : HeaderDecoder.ExtractFirstDisplayName(HeaderDecoder.DecodeEncodedWords(fromHeader));

        var toEmails = AddressList(root.GetHeader("To"));
        var ccEmails = AddressList(root.GetHeader("Cc"));
        var bccEmails = AddressList(root.GetHeader("Bcc"));

        // Date: raw header text preferred (preserves original format).
        string? date = ExtractRawDateHeader(data);

        string? messageId = root.GetHeader("Message-ID") is { } mid
            ? EmptyToNull(HeaderDecoder.StripAngleBrackets(mid))
            : null;

        var replyTo = AddressList(root.GetHeader("Reply-To"));
        var inReplyTo = TextList(root.GetHeader("In-Reply-To"));
        var references = TextList(root.GetHeader("References"));

        var rawHeaders = ExtractRawHeaders(data);

        // Classify parts.
        var textParts = new List<MimePart>();
        var htmlParts = new List<MimePart>();
        var attachmentParts = new List<MimePart>();
        Collect(root, null, textParts, htmlParts, attachmentParts);

        bool hasGenuineText = textParts.Count > 0;
        bool hasGenuineHtml = htmlParts.Count > 0;
        bool shouldTreatAsHtml = string.Equals(root.ContentSubtype, "html", StringComparison.OrdinalIgnoreCase);

        string? plainText = null;
        if (hasGenuineText || shouldTreatAsHtml)
        {
            var texts = new List<string>();
            foreach (var p in textParts) texts.Add(p.DecodedText());
            if (texts.Count == 0 && shouldTreatAsHtml)
                foreach (var p in htmlParts) texts.Add(p.DecodedText());
            plainText = texts.Count == 0 ? null : string.Join("\n\n", texts);
        }

        string? htmlContent = null;
        if (hasGenuineHtml || shouldTreatAsHtml)
        {
            var htmls = new List<string>();
            foreach (var p in htmlParts) htmls.Add(p.DecodedText());
            htmlContent = htmls.Count == 0 ? null : string.Join("\n\n", htmls);
        }

        // Single-part HTML fallback.
        if (shouldTreatAsHtml && htmlContent is null && plainText is not null)
            htmlContent = plainText;

        string content;
        if (htmlContent is not null) content = CleanHtmlContent(htmlContent);
        else if (plainText is not null) content = plainText;
        else content = "";

        var attachments = new List<EmailAttachment>();
        foreach (var part in attachmentParts)
        {
            string? filename = part.AttachmentName;
            string mimeType = ParseContentType($"{part.ContentTypeMain}/{NonEmptyOr(part.ContentSubtype, "octet-stream")}");
            byte[] bytes = part.DecodedBytes();
            attachments.Add(new EmailAttachment
            {
                Name = filename,
                Filename = filename,
                MimeType = mimeType,
                Size = bytes.Length,
                IsImage = mimeType.StartsWith("image/", StringComparison.Ordinal),
                Data = bytes,
            });
        }

        var metadata = BuildMetadata(subject, fromEmail, fromName, toEmails, ccEmails, bccEmails, date, messageId, attachments);

        if (replyTo.Count > 0) metadata["reply_to"] = string.Join(", ", replyTo);
        if (inReplyTo.Count > 0) metadata["in_reply_to"] = string.Join(", ", inReplyTo);
        if (references.Count > 0) metadata["references"] = string.Join(", ", references);

        foreach (var (k, v) in rawHeaders) metadata[k] = v;

        if (attachments.Count > 0)
        {
            var details = attachments.Select(att =>
            {
                string name = att.Filename ?? att.Name ?? "unnamed";
                string mime = att.MimeType ?? "application/octet-stream";
                int size = att.Size ?? 0;
                return $"{name}|{mime}|{size}";
            });
            metadata["attachment_details"] = string.Join("; ", details);
        }

        return new EmailExtractionResult
        {
            Subject = subject,
            FromEmail = fromEmail,
            ToEmails = toEmails,
            CcEmails = ccEmails,
            BccEmails = bccEmails,
            Date = date,
            MessageId = messageId,
            PlainText = plainText,
            HtmlContent = htmlContent,
            Content = content,
            Attachments = attachments,
            Metadata = metadata,
        };
    }

    // -----------------------------------------------------------------------
    // Part tree parsing
    // -----------------------------------------------------------------------

    private static void Collect(MimePart p, MimePart? parent, List<MimePart> text, List<MimePart> html, List<MimePart> attach)
    {
        if (p.IsMultipart)
        {
            foreach (var child in p.Children) Collect(child, p, text, html, attach);
            return;
        }

        // A part the boundary never closed is re-read raw and demoted: its media type becomes
        // "text, other" unless it was text/plain, and it stops counting as inline content. Only
        // a text/plain still holding its slot in a multipart/alternative stays in the body; every
        // other shape falls through to the attachment list.
        if (p.IsEncodingProblem)
        {
            bool keepsBodySlot = p.ContentType == "text/plain"
                && parent is not null
                && string.Equals(parent.ContentType, "multipart/alternative", StringComparison.Ordinal);
            if (keepsBodySlot) text.Add(p);
            else attach.Add(p);
            return;
        }

        string disp = (p.Disposition ?? "").Trim().ToLowerInvariant();
        if (disp == "attachment") { attach.Add(p); return; }
        if (p.ContentType == "text/plain") { text.Add(p); return; }
        if (p.ContentType == "text/html") { html.Add(p); return; }
        attach.Add(p);
    }

    private static MimePart ParsePart(string latin1, bool unterminated)
    {
        var part = new MimePart();

        int sepIdx = latin1.IndexOf("\r\n\r\n", StringComparison.Ordinal);
        int sepLen = 4;
        if (sepIdx < 0)
        {
            sepIdx = latin1.IndexOf("\n\n", StringComparison.Ordinal);
            sepLen = 2;
        }

        string headerStr;
        string bodyStr;
        if (sepIdx < 0) { headerStr = latin1; bodyStr = ""; }
        else { headerStr = latin1.Substring(0, sepIdx); bodyStr = latin1.Substring(sepIdx + sepLen); }

        ParseHeaders(headerStr, part);

        // Content-Type.
        if (part.GetHeader("Content-Type") is { } ctRaw)
        {
            var (value, ps) = HeaderDecoder.ParseParameterized(ctRaw);
            if (value.Length > 0) part.ContentType = value;
            if (ps.TryGetValue("boundary", out var b)) part.Boundary = b;
            if (ps.TryGetValue("charset", out var cs)) part.Charset = cs;
            if (ps.TryGetValue("name", out var nm)) part.ContentTypeName = HeaderDecoder.DecodeEncodedWords(nm);
        }

        part.TransferEncoding = part.GetHeader("Content-Transfer-Encoding")?.Trim();

        if (part.GetHeader("Content-Disposition") is { } cdRaw)
        {
            var (disp, ps) = HeaderDecoder.ParseParameterized(cdRaw);
            part.Disposition = disp;
            if (ps.TryGetValue("filename", out var fn)) part.DispositionFilename = HeaderDecoder.DecodeEncodedWords(fn);
        }

        if (part.IsMultipart)
        {
            foreach (var (segment, terminated) in SplitBoundary(bodyStr, part.Boundary!))
                part.Children.Add(ParsePart(segment, unterminated: !terminated));
        }
        else
        {
            part.BodyLatin1 = bodyStr;
            part.IsEncodingProblem = unterminated;
        }

        return part;
    }

    private static void ParseHeaders(string headerStr, MimePart part)
    {
        string? curName = null;
        var curVal = new StringBuilder();

        void Flush()
        {
            if (curName is not null)
                part.Headers.Add((curName, curVal.ToString()));
            curName = null;
            curVal.Clear();
        }

        foreach (string rawLine in headerStr.Split('\n'))
        {
            string line = rawLine.EndsWith('\r') ? rawLine[..^1] : rawLine;
            if (line.Length == 0) continue;

            if (line[0] == ' ' || line[0] == '\t')
            {
                // Folded continuation.
                if (curName is not null)
                {
                    curVal.Append(' ');
                    curVal.Append(line.Trim());
                }
                continue;
            }

            int colon = line.IndexOf(':');
            if (colon < 0)
                continue; // not a header line
            Flush();
            curName = line.Substring(0, colon).Trim();
            curVal.Append(line.Substring(colon + 1).TrimStart());
        }
        Flush();
    }

    /// <summary>
    /// Split a multipart body on its boundary, dropping preamble and epilogue. Each segment says
    /// whether a boundary actually closed it; the last one does not when the message is cut off,
    /// and it then keeps every byte to the end, trailing newline included.
    /// </summary>
    private static List<(string Segment, bool Terminated)> SplitBoundary(string body, string boundary)
    {
        string marker = "--" + boundary;
        var markerPositions = new List<int>();
        int searchFrom = 0;
        while (true)
        {
            int idx = body.IndexOf(marker, searchFrom, StringComparison.Ordinal);
            if (idx < 0) break;
            if (idx == 0 || body[idx - 1] == '\n')
                markerPositions.Add(idx);
            searchFrom = idx + marker.Length;
        }

        var segments = new List<(string Segment, bool Terminated)>();
        for (int k = 0; k < markerPositions.Count; k++)
        {
            Cancel.ThrowIfRequested();
            int start = markerPositions[k];
            int afterMarker = start + marker.Length;
            // Close delimiter "--boundary--".
            if (afterMarker + 1 < body.Length && body[afterMarker] == '-' && body[afterMarker + 1] == '-')
                break;

            int nl = body.IndexOf('\n', afterMarker);
            if (nl < 0) break; // no content follows the marker line
            int contentStart = nl + 1;

            bool terminated = k + 1 < markerPositions.Count;
            int contentEnd = terminated ? markerPositions[k + 1] : body.Length;
            int end = contentEnd;
            if (terminated)
            {
                // Strip one trailing newline (and optional CR) that belongs to the next delimiter.
                if (end > contentStart && body[end - 1] == '\n')
                {
                    end--;
                    if (end > contentStart && body[end - 1] == '\r') end--;
                }
            }
            if (end < contentStart) end = contentStart;
            segments.Add((body.Substring(contentStart, end - contentStart), terminated));
        }
        return segments;
    }

    // -----------------------------------------------------------------------
    // Address / list helpers
    // -----------------------------------------------------------------------

    private static List<string> AddressList(string? headerValue)
    {
        if (headerValue is null) return new();
        return HeaderDecoder.ExtractAddresses(HeaderDecoder.DecodeEncodedWords(headerValue));
    }

    private static List<string> TextList(string? headerValue)
    {
        if (headerValue is null) return new();
        // In-Reply-To / References are whitespace-separated message-ids.
        var list = new List<string>();
        foreach (var tok in headerValue.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries))
        {
            string id = HeaderDecoder.StripAngleBrackets(tok);
            if (id.Length > 0) list.Add(id);
        }
        return list;
    }

    private static string? DecodeHeaderOpt(string? value) =>
        value is null ? null : HeaderDecoder.DecodeEncodedWords(value);

    // -----------------------------------------------------------------------
    // Raw header scans (byte-level, independent of MIME tree) — Rust ports
    // -----------------------------------------------------------------------

    private static readonly (string Prefix, string Key)[] RawHeaderTargets =
    {
        ("content-type:", "content_type"),
        ("mime-version:", "mime_version"),
        ("x-mailer:", "x_mailer"),
        ("user-agent:", "user_agent"),
        ("list-id:", "list_id"),
        ("list-unsubscribe:", "list_unsubscribe"),
    };

    internal static Dictionary<string, string> ExtractRawHeaders(byte[] data)
    {
        var headers = new Dictionary<string, string>(StringComparer.Ordinal);
        if (!TryUtf8(data, out string text)) return headers;

        int headerEnd = FindHeaderEnd(text, 16384);
        string section = text.Substring(0, headerEnd);

        string? curKey = null;
        var curVal = new StringBuilder();

        foreach (string line in Lines(section))
        {
            if (line.StartsWith(' ') || line.StartsWith('\t'))
            {
                if (curKey is not null)
                {
                    curVal.Append(' ');
                    curVal.Append(line.Trim());
                }
                continue;
            }

            if (curKey is not null)
            {
                if (curVal.Length > 0) headers[curKey] = curVal.ToString();
                curKey = null;
                curVal.Clear();
            }

            string lower = line.ToLowerInvariant();
            foreach (var (prefix, key) in RawHeaderTargets)
            {
                if (lower.StartsWith(prefix, StringComparison.Ordinal))
                {
                    curKey = key;
                    curVal.Clear();
                    curVal.Append(line.Substring(prefix.Length).Trim());
                    break;
                }
            }
        }

        if (curKey is not null && curVal.Length > 0)
            headers[curKey] = curVal.ToString();

        return headers;
    }

    internal static string? ExtractRawDateHeader(byte[] data)
    {
        if (!TryUtf8(data, out string text)) return null;

        int headerEnd = FindHeaderEnd(text, 8192);
        string headers = text.Substring(0, headerEnd);

        string? dateValue = null;
        foreach (string line in Lines(headers))
        {
            string? val = StripPrefix(line, "Date:") ?? StripPrefix(line, "date:");
            if (val is not null)
            {
                dateValue = val.Trim();
            }
            else if (dateValue is not null && (line.StartsWith(' ') || line.StartsWith('\t')))
            {
                dateValue = dateValue + " " + line.Trim();
            }
            else if (dateValue is not null)
            {
                break;
            }
        }

        return EmptyToNull(dateValue);
    }

    private static int FindHeaderEnd(string text, int cap)
    {
        int idx = text.IndexOf("\r\n\r\n", StringComparison.Ordinal);
        if (idx >= 0) return idx;
        idx = text.IndexOf("\n\n", StringComparison.Ordinal);
        if (idx >= 0) return idx;
        return Math.Min(text.Length, cap);
    }

    // Mirrors Rust `str::lines()`: split on '\n', strip a trailing '\r'.
    private static IEnumerable<string> Lines(string s)
    {
        foreach (string raw in s.Split('\n'))
            yield return raw.EndsWith('\r') ? raw[..^1] : raw;
    }

    private static string? StripPrefix(string s, string prefix) =>
        s.StartsWith(prefix, StringComparison.Ordinal) ? s.Substring(prefix.Length) : null;

    private static bool TryUtf8(byte[] data, out string text)
    {
        try
        {
            text = new UTF8Encoding(false, throwOnInvalidBytes: true).GetString(data);
            return true;
        }
        catch (DecoderFallbackException)
        {
            text = "";
            return false;
        }
    }

    // -----------------------------------------------------------------------
    // build_metadata / parse_content_type / clean_html_content ports
    // -----------------------------------------------------------------------

    private static Dictionary<string, string> BuildMetadata(
        string? subject, string? fromEmail, string? fromName, List<string> to, List<string> cc, List<string> bcc,
        string? date, string? messageId, List<EmailAttachment> attachments)
    {
        var metadata = new Dictionary<string, string>(StringComparer.Ordinal);
        if (subject is not null) metadata["subject"] = subject;
        if (fromEmail is not null) metadata["email_from"] = fromEmail;
        if (fromName is not null) metadata["from_name"] = fromName;
        if (to.Count > 0) metadata["email_to"] = string.Join(", ", to);
        if (cc.Count > 0) metadata["email_cc"] = string.Join(", ", cc);
        if (bcc.Count > 0) metadata["email_bcc"] = string.Join(", ", bcc);
        if (date is not null) metadata["date"] = date;
        if (messageId is not null) metadata["message_id"] = messageId;

        if (attachments.Count > 0)
        {
            var names = attachments
                .Select(a => a.Name ?? a.Filename)
                .Where(n => n is not null)
                .Select(n => n!)
                .ToList();
            if (names.Count > 0) metadata["attachments"] = string.Join(", ", names);
        }
        return metadata;
    }

    internal static string ParseContentType(string contentType)
    {
        string trimmed = contentType.Trim();
        if (trimmed.Length == 0) return "application/octet-stream";
        string first = trimmed.Split(';')[0].Trim();
        if (first.Length == 0) return "application/octet-stream";
        return first.ToLowerInvariant();
    }

    private static readonly Regex ScriptRe = new("<script[^>]*>.*?</script>", RegexOptions.IgnoreCase | RegexOptions.Singleline | RegexOptions.Compiled);
    private static readonly Regex StyleRe = new("<style[^>]*>.*?</style>", RegexOptions.IgnoreCase | RegexOptions.Singleline | RegexOptions.Compiled);
    private static readonly Regex TagRe = new("<[^>]+>", RegexOptions.Compiled);
    private static readonly Regex WsRe = new(@"\s+", RegexOptions.Compiled);

    /// <summary>
    /// Regex-based HTML stripping (the `html` feature's html-to-markdown path is not ported;
    /// this matches the Rust fallback). Only feeds the internal `content` field, which
    /// build_internal_document ignores whenever html_content is present.
    /// </summary>
    internal static string CleanHtmlContent(string html)
    {
        if (html.Length == 0) return "";
        string cleaned = ScriptRe.Replace(html, "");
        cleaned = StyleRe.Replace(cleaned, "");
        cleaned = TagRe.Replace(cleaned, "");
        cleaned = WsRe.Replace(cleaned, " ");
        return cleaned.Trim();
    }

    // -----------------------------------------------------------------------
    // UTF-16 transcode (BOM only; the no-BOM chardet heuristic is deferred)
    // -----------------------------------------------------------------------

    /// <summary>
    /// Port of Rust `maybe_transcode_utf16`. `mail_parser` (and this parser) expect
    /// ASCII/UTF-8 input, so UTF-16 EML files are transcoded to UTF-8 first.
    ///
    /// Detection: (1) an explicit BOM (FF FE = LE, FE FF = BE), or (2) no BOM but an
    /// alternating-null-byte pattern in the first 8 bytes — the common shape of a UTF-16
    /// file that starts with ASCII headers. Rust confirms case (2) with chardetng (accepting
    /// only a UTF-8/windows-1252 guess); lacking chardetng, we instead require the payload to
    /// decode as valid UTF-16 (throwing decoder), which rejects legacy single-byte content.
    /// </summary>
    private static byte[]? MaybeTranscodeUtf16(byte[] data)
    {
        if (data.Length < 4) return null;

        bool isLe;
        int skip;

        if (data[0] == 0xFF && data[1] == 0xFE) { isLe = true; skip = 2; }
        else if (data[0] == 0xFE && data[1] == 0xFF) { isLe = false; skip = 2; }
        else if (data.Length >= 16)
        {
            // No BOM: look for alternating null bytes across the first 8 bytes.
            bool isLeHeuristic = data[1] == 0x00 && data[3] == 0x00 && data[5] == 0x00 && data[7] == 0x00;
            bool isBeHeuristic = data[0] == 0x00 && data[2] == 0x00 && data[4] == 0x00 && data[6] == 0x00;
            if (isLeHeuristic || isBeHeuristic) { isLe = isLeHeuristic; skip = 0; }
            else return null;
        }
        else return null;

        int len = (data.Length - skip) & ~1;
        var decoder = isLe
            ? new UnicodeEncoding(bigEndian: false, byteOrderMark: false, throwOnInvalidBytes: true)
            : new UnicodeEncoding(bigEndian: true, byteOrderMark: false, throwOnInvalidBytes: true);
        try
        {
            string s = decoder.GetString(data, skip, len);
            return Utf8NoBom.GetBytes(s);
        }
        catch (DecoderFallbackException)
        {
            return null;
        }
    }

    private static string NonEmptyOr(string s, string fallback) => s.Length > 0 ? s : fallback;
    private static string? EmptyToNull(string? s) => string.IsNullOrEmpty(s) ? null : s;
}
