// Ported from crates/xberg/src/extractors/iwork/mod.rs — the IWA container shared by
// Pages, Numbers and Keynote: ZIP member → Apple's Snappy framing → protobuf payload.
//
// The Rust module threads a SecurityBudget and an IwaExpansionBudget through every call;
// this port has no budget layer. The one limit kept is the 64 MiB decompression cap, which
// upstream applies as a fixed constant rather than a configured limit.

using System.IO.Compression;
using System.Text;
using XRay.Content.Types;

namespace XRay.Content.Internal.IWork;

/// <summary>A malformed IWA member. Upstream's `XbergError::parsing` for the same conditions.</summary>
internal sealed class IwaFormatException(string message) : Exception(message);

internal static class IwaContainer
{
    /// <summary>Maximum size for an individual IWA file, to guard against decompression bombs.</summary>
    private const int MaxDecompressedSize = 64 * 1024 * 1024;

    /// <summary><c>ProcessingWarning.Source</c> used for every degradation the iWork extractors report.</summary>
    public const string WarningSource = "iwork";

    /// <summary>
    /// Record that an IWA archive member could not be parsed and its content was dropped
    /// from the output.
    /// </summary>
    public static void PushMemberParseWarning(List<ProcessingWarning> warnings, string member, Exception cause) =>
        PushWarning(
            warnings,
            $"Failed to parse iWork archive member '{member}'; its content was not extracted (cause: {cause.Message})");

    /// <summary>Append a warning unless an identical one is already present.</summary>
    public static void PushWarning(List<ProcessingWarning> warnings, string message)
    {
        if (warnings.Any(w => w.Source == WarningSource && w.Message == message)) return;
        warnings.Add(new ProcessingWarning { Source = WarningSource, Message = message });
    }

    /// <summary>Every archive entry whose path ends with <c>.iwa</c>, in archive order.</summary>
    public static List<string> CollectIwaPaths(ZipArchive archive) =>
        archive.Entries
            .Select(e => e.FullName)
            .Where(name => name.EndsWith(".iwa", StringComparison.Ordinal))
            .ToList();

    /// <summary>Read and Snappy-decompress a single <c>.iwa</c> member.</summary>
    public static byte[] ReadIwaFile(ZipArchive archive, string path)
    {
        var entry = archive.GetEntry(path) ?? throw new IwaFormatException($"IWA file not found in archive: {path}");
        if (entry.Length > MaxDecompressedSize)
            throw new IwaFormatException($"IWA file {path} is larger than the {MaxDecompressedSize} byte limit");

        byte[] raw;
        using (var stream = entry.Open())
        using (var buffer = new MemoryStream())
        {
            stream.CopyTo(buffer);
            raw = buffer.ToArray();
        }
        return DecodeIwaStream(raw);
    }

    /// <summary>
    /// Decode an Apple IWA byte stream into the raw protobuf payload.
    ///
    /// Framing: each block is 1 type byte + 3 little-endian length bytes + N payload bytes.
    /// Type 0x00 is a raw Snappy block; type 0x01 is stored as-is.
    /// </summary>
    public static byte[] DecodeIwaStream(ReadOnlySpan<byte> data)
    {
        var output = new MemoryStream();
        int position = 0;

        while (data.Length - position >= 4)
        {
            Cancel.ThrowIfRequested();
            byte chunkType = data[position];
            int chunkLength = data[position + 1] | (data[position + 2] << 8) | (data[position + 3] << 16);
            int payloadOffset = position + 4;
            long end = (long)payloadOffset + chunkLength;
            if (end > data.Length)
                throw new IwaFormatException(
                    $"IWA chunk out of bounds: offset={payloadOffset}, chunk_len={chunkLength}, data_len={data.Length}");

            var payload = data.Slice(payloadOffset, chunkLength);
            switch (chunkType)
            {
                case 0x00:
                    try
                    {
                        // Length first: the declared size is checked against the budget before any
                        // bytes are expanded, so a bomb is refused rather than decompressed.
                        AccountExpansion(output.Length, Snappy.DecompressedLength(payload));
                        output.Write(Snappy.Decompress(payload));
                    }
                    catch (Snappy.FormatException error)
                    {
                        throw new IwaFormatException($"Snappy decompression failed: {error.Message}");
                    }
                    break;
                case 0x01:
                    AccountExpansion(output.Length, payload.Length);
                    output.Write(payload);
                    break;
                default:
                    throw new IwaFormatException($"Unknown IWA chunk type: 0x{chunkType:x2}");
            }
            position = (int)end;
        }

        if (position != data.Length)
            throw new IwaFormatException($"IWA stream has {data.Length - position} trailing framing bytes");

        return output.ToArray();
    }

    private static void AccountExpansion(long current, long added)
    {
        if (current + added > MaxDecompressedSize)
            throw new IwaFormatException($"IWA stream expands past the {MaxDecompressedSize} byte limit");
    }

    /// <summary>
    /// Extract all UTF-8 text strings from a raw protobuf byte slice, using a schema-free
    /// wire scanner: every length-delimited field that is valid UTF-8 is a text candidate,
    /// and is then rescanned for nested fields.
    /// </summary>
    public static List<string> ExtractTextFromProto(ReadOnlySpan<byte> data)
    {
        var texts = new List<string>();
        ExtractProtoFields(data, texts, 0);
        return texts;
    }

    /// <summary>
    /// Upstream's nesting limit (`SecurityLimits::max_nesting_depth`). Upstream aborts the
    /// extraction on reaching it; with no budget layer to report through, the scan simply
    /// stops descending.
    /// </summary>
    private const int MaxNestingDepth = 1024;

    private static void ExtractProtoFields(ReadOnlySpan<byte> data, List<string> texts, int depth)
    {
        if (depth >= MaxNestingDepth) return;

        int position = 0;
        while (position < data.Length)
        {
            Cancel.ThrowIfRequested();
            if (!TryReadVarint(data, position, out ulong tag, out int tagLength)) break;
            position += tagLength;
            if (!ExtractProtoField(data, ref position, tag & 0x7, texts, depth)) break;
        }
    }

    private static bool ExtractProtoField(
        ReadOnlySpan<byte> data, ref int position, ulong wireType, List<string> texts, int depth)
    {
        switch (wireType)
        {
            case 0:
                if (!TryReadVarint(data, position, out _, out int length)) return false;
                position += length;
                return true;
            case 1:
                position += 8;
                return true;
            case 2:
                return ExtractLengthDelimitedField(data, ref position, texts, depth);
            case 5:
                position += 4;
                return true;
            default:
                return false;
        }
    }

    private static bool ExtractLengthDelimitedField(
        ReadOnlySpan<byte> data, ref int position, List<string> texts, int depth)
    {
        if (!TryReadVarint(data, position, out ulong length, out int prefixLength)) return false;
        position += prefixLength;
        if (length > int.MaxValue)
            throw new IwaFormatException("protobuf length does not fit this platform");
        long end = (long)position + (long)length;
        if (end > data.Length) return false;

        var payload = data.Slice(position, (int)length);
        position = (int)end;
        AppendProtoText(payload, texts);
        ExtractProtoFields(payload, texts, depth + 1);
        return true;
    }

    private static void AppendProtoText(ReadOnlySpan<byte> payload, List<string> texts)
    {
        if (!TryDecodeUtf8(payload, out string text)) return;
        string trimmed = text.Trim();
        // A field is accepted as text once it has any alphanumeric character; no byte-length
        // floor, which would silently drop single-letter headings, numeric answers and unit
        // labels like "OK", "5", "Q1".
        if (IsAlphanumeric(trimmed)) texts.Add(trimmed);
    }

    /// <summary>Whether any character is alphanumeric, matching Rust's <c>char::is_alphanumeric</c>.</summary>
    public static bool IsAlphanumeric(string text)
    {
        foreach (var rune in text.EnumerateRunes())
        {
            if (System.Text.Rune.IsLetterOrDigit(rune)) return true;
            var category = System.Text.Rune.GetUnicodeCategory(rune);
            if (category is System.Globalization.UnicodeCategory.LetterNumber
                or System.Globalization.UnicodeCategory.OtherNumber) return true;
        }
        return false;
    }

    public static bool TryDecodeUtf8(ReadOnlySpan<byte> bytes, out string text)
    {
        try
        {
            text = StrictUtf8.GetString(bytes);
            return true;
        }
        catch (DecoderFallbackException)
        {
            text = "";
            return false;
        }
    }

    private static readonly UTF8Encoding StrictUtf8 = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    /// <summary>Read a protobuf varint; false when the bytes run out or the value exceeds 64 bits.</summary>
    public static bool TryReadVarint(ReadOnlySpan<byte> data, int position, out ulong value, out int consumed)
    {
        value = 0;
        consumed = 0;
        int shift = 0;
        int i = position;
        while (true)
        {
            if (i >= data.Length) return false;
            ulong b = data[i];
            i++;
            value |= (b & 0x7F) << shift;
            if ((b & 0x80) == 0)
            {
                consumed = i - position;
                return true;
            }
            shift += 7;
            if (shift >= 64) return false;
        }
    }

    /// <summary>
    /// Collapse only adjacent duplicates. The wire format emits the same run twice when a
    /// payload is read as text and then rescanned for nested fields; a non-adjacent repeat
    /// is real content — a heading reused on two slides, a footer on every page.
    /// </summary>
    public static List<string> DedupText(IEnumerable<string> texts)
    {
        var result = new List<string>();
        foreach (var text in texts)
        {
            if (result.Count > 0 && result[^1] == text) continue;
            result.Add(text);
        }
        return result;
    }

    /// <summary>
    /// Metadata from <c>Metadata/Properties.plist</c> and <c>Metadata/DocumentIdentifier</c>.
    /// An unreadable or absent member simply leaves the field unset.
    /// </summary>
    public static Metadata ExtractMetadataFromZip(ZipArchive archive)
    {
        var metadata = new Metadata();

        if (ReadEntryText(archive, "Metadata/Properties.plist") is { } plist)
            ParsePlistMetadata(plist, metadata);

        if (ReadEntryText(archive, "Metadata/DocumentIdentifier") is { } identifier)
        {
            string trimmed = identifier.Trim();
            if (trimmed.Length > 0 && metadata.Title is null) metadata.Title = trimmed;
        }

        return metadata;
    }

    private static string? ReadEntryText(ZipArchive archive, string name)
    {
        var entry = archive.GetEntry(name);
        if (entry is null) return null;
        try
        {
            using var stream = entry.Open();
            using var buffer = new MemoryStream();
            stream.CopyTo(buffer);
            return TryDecodeUtf8(buffer.ToArray(), out string text) ? text : null;
        }
        catch (InvalidDataException)
        {
            return null;
        }
    }

    /// <summary>
    /// iWork plist metadata uses <c>&lt;key&gt;…&lt;/key&gt;&lt;string&gt;…&lt;/string&gt;</c>
    /// pairs on consecutive lines.
    /// </summary>
    private static void ParsePlistMetadata(string plist, Metadata metadata)
    {
        var lines = plist.Split('\n').Select(l => l.TrimEnd('\r').Trim()).ToList();
        int i = 0;
        while (i < lines.Count)
        {
            if (ExtractPlistTag(lines[i], "key") is { } key)
            {
                int j = i + 1;
                while (j < lines.Count && lines[j].Length == 0) j++;
                if (j < lines.Count && ExtractPlistTag(lines[j], "string") is { } value)
                {
                    ApplyPlistValue(metadata, key, value);
                    i = j + 1;
                    continue;
                }
            }
            i++;
        }
    }

    private static void ApplyPlistValue(Metadata metadata, string key, string value)
    {
        switch (key)
        {
            case "title" or "Title" when metadata.Title is null:
                metadata.Title = value;
                break;
            case "author" or "Author" or "creator" or "Creator":
                var authors = metadata.Authors ??= new List<string>();
                if (!authors.Contains(value)) authors.Add(value);
                break;
            case "keywords" or "Keywords":
                var keywords = metadata.Keywords ??= new List<string>();
                foreach (var word in value.Split(','))
                {
                    string trimmed = word.Trim();
                    if (trimmed.Length > 0 && !keywords.Contains(trimmed)) keywords.Add(trimmed);
                }
                break;
            case "language" or "Language" when metadata.Language is null:
                metadata.Language = value;
                break;
        }
    }

    /// <summary>The text of a simple XML tag, e.g. <c>&lt;string&gt;value&lt;/string&gt;</c>.</summary>
    private static string? ExtractPlistTag(string line, string tag)
    {
        string open = $"<{tag}>";
        string close = $"</{tag}>";
        int start = line.IndexOf(open, StringComparison.Ordinal);
        if (start < 0) return null;
        int end = line.IndexOf(close, StringComparison.Ordinal);
        if (end < start + open.Length) return null;
        return line.Substring(start + open.Length, end - (start + open.Length));
    }

    /// <summary>UTF-8 byte length, which is what Rust's <c>str::len</c> measures.</summary>
    public static int Utf8Length(string text) => Encoding.UTF8.GetByteCount(text);
}
