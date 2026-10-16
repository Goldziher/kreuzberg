using System.IO.Compression;
using System.Text;
using XRay.Content.Core;
using XRay.Content.Internal.Cfb;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// Hangul Word Processor 5.0 (.hwp) extractor. Ports <c>extractors/hwp.rs</c> +
/// <c>extraction/hwp/*</c>: opens the OLE/CFB container, reads the FileHeader flags, decompresses
/// the BodyText/SectionN streams (raw deflate, zlib fallback), walks the HWP records for paragraph
/// text/shape/char-shape, and emits headings (outline level &gt; 0) / paragraphs plus BinData images
/// with char-shape annotations.
/// </summary>
public sealed class HwpExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/x-hwp" };

    private static readonly byte[] Signature = Encoding.ASCII.GetBytes("HWP Document File");

    private const int HwpTagBegin = 0x010;
    // The offsets from HWPTAG_BEGIN are decimal, per the HWP 5.0 record specification. They had
    // been written as though the spec's "+ 50" were hexadecimal, which put every body-text tag
    // 14 records past where it belongs, so no paragraph record ever matched.
    private const int TagParaHeader = HwpTagBegin + 50;  // 0x42
    private const int TagParaText = HwpTagBegin + 51;    // 0x43
    private const int TagParaShape = HwpTagBegin + 66;   // 0x52
    private const int TagCharShape = HwpTagBegin + 67;   // 0x53
    private const int TagCharShapeInfo = HwpTagBegin + 30; // 0x2E

    private struct CharShape { public bool Bold, Italic, Underline; }

    private sealed class Para
    {
        public string? Text;
        public byte OutlineLevel;
        public List<(uint Pos, ushort ShapeIdx)> Runs = new();
    }

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        var comp = CompoundFile.Open(content);

        byte[] header = comp.TryReadStream("/FileHeader")
            ?? throw new InvalidDataException("Stream 'FileHeader' not found");
        if (header.Length < 256) throw new InvalidDataException("FileHeader must be at least 256 bytes");
        for (int i = 0; i < Signature.Length; i++)
            if (header[i] != Signature[i]) throw new InvalidDataException("Invalid HWP signature");
        uint flags = OleUtil.U32(header, 36);
        if ((flags & 0x02) != 0) throw new InvalidDataException("Password-encrypted HWP documents are not supported");
        bool compressed = (flags & 0x01) != 0;

        var charShapes = new List<CharShape>();
        if (comp.TryReadStream("/DocInfo") is { } docInfo)
            ParseDocInfo(docInfo, charShapes);

        var paragraphs = new List<Para>();
        // Compound-file paths are absolute, so they have to be made relative before being matched
        // against a relative prefix. Comparing them as-is matched nothing, and every HWP fell
        // through to the "no BodyText sections" error — silently, without a warning.
        var streams = comp.Walk().Where(e => e.IsStream).Select(e => e.Path.TrimStart('/')).ToList();
        streams.Sort(StringComparer.Ordinal);
        foreach (var path in streams)
        {
            if (path.StartsWith("BodyText/Section", StringComparison.Ordinal))
            {
                byte[]? sec = comp.TryReadStream(path);
                if (sec is null) continue;
                byte[] data = compressed ? Decompress(sec) : sec;
                ParseBodyText(data, paragraphs);
            }
        }

        if (paragraphs.Count == 0)
            throw new InvalidDataException("no BodyText sections found in HWP document");

        var b = new InternalDocumentBuilder("hwp");
        foreach (var para in paragraphs)
        {
            if (para.Text is { Length: > 0 } t)
            {
                var annotations = ApplyCharShapes(t, para.Runs, charShapes);
                if (para.OutlineLevel > 0)
                {
                    uint idx = b.PushHeading(para.OutlineLevel, t, null, null);
                    if (annotations.Count > 0) b.SetAnnotations(idx, annotations);
                }
                else
                {
                    b.PushParagraph(t, annotations, null, null);
                }
            }
        }

        int imgIndex = 0;
        foreach (var e in comp.Walk().Where(e => e.IsStream))
        {
            // Same leading-'/' behavior as Rust: `path.starts_with("BinData/")` never matches.
            if (!e.Path.StartsWith("BinData/", StringComparison.Ordinal)) continue;
            byte[]? imgData = comp.TryReadStream(e.Path);
            if (imgData is null) continue;
            var image = new ExtractedImage
            {
                Data = imgData,
                Format = DetectImageMime(imgData),
                ImageIndex = (uint)imgIndex++,
                SourcePath = e.Path.TrimStart('/'),
            };
            b.PushImage(null, image, null, null);
        }

        var doc = b.Build();
        if (doc.Elements.Count == 0)
            throw new InvalidDataException("no BodyText sections found in HWP document");
        doc.MimeType = mimeType;
        return doc;
    }

    private static void ParseDocInfo(byte[] data, List<CharShape> shapes)
    {
        int pos = 0;
        while (pos + 4 <= data.Length)
        {
            if (!ReadRecord(data, ref pos, out int tagId, out int dataStart, out int dataLen)) break;
            if (tagId == TagCharShapeInfo && dataLen >= 4)
            {
                uint fontAttr = OleUtil.U32(data, dataStart);
                shapes.Add(new CharShape
                {
                    Bold = (fontAttr & 0x01) != 0,
                    Italic = (fontAttr & 0x02) != 0,
                    Underline = (fontAttr & 0x04) != 0,
                });
            }
        }
    }

    private static void ParseBodyText(byte[] data, List<Para> paragraphs)
    {
        Para? current = null;
        int pos = 0;
        while (pos + 4 <= data.Length)
        {
            if (!ReadRecord(data, ref pos, out int tagId, out int dataStart, out int dataLen)) break;
            switch (tagId)
            {
                case TagParaHeader:
                    if (current is not null) paragraphs.Add(current);
                    current = new Para();
                    break;
                case TagParaText:
                    if (current is not null) current.Text = DecodeParaText(data, dataStart, dataLen);
                    break;
                case TagParaShape:
                    if (current is not null && dataLen > 18) current.OutlineLevel = data[dataStart + 18];
                    break;
                case TagCharShape:
                    if (current is not null)
                    {
                        int p = dataStart;
                        int end = dataStart + dataLen;
                        while (end - p >= 6)
                        {
                            uint posVal = OleUtil.U32(data, p);
                            ushort shapeIdx = (ushort)OleUtil.U16(data, p + 4);
                            current.Runs.Add((posVal, shapeIdx));
                            p += 6;
                        }
                    }
                    break;
            }
        }
        if (current is not null) paragraphs.Add(current);
    }

    /// <summary>Read one HWP record header + payload bounds; advances <paramref name="pos"/>.</summary>
    private static bool ReadRecord(byte[] data, ref int pos, out int tagId, out int dataStart, out int dataLen)
    {
        tagId = 0; dataStart = 0; dataLen = 0;
        if (pos + 4 > data.Length) return false;
        uint headerWord = OleUtil.U32(data, pos);
        pos += 4;
        tagId = (int)(headerWord & 0x3FF);
        uint size = headerWord >> 20;
        if (size == 0xFFF)
        {
            if (pos + 4 > data.Length) return false;
            size = OleUtil.U32(data, pos);
            pos += 4;
        }
        dataLen = (int)size;
        if (dataLen > data.Length - pos) return false;
        dataStart = pos;
        pos += dataLen;
        return true;
    }

    /// <summary>Decode a ParaText record (UTF-16LE with HWP control chars).</summary>
    private static string DecodeParaText(byte[] data, int start, int len)
    {
        int count = len / 2;
        var chars = new ushort[count];
        for (int i = 0; i < count; i++)
            chars[i] = (ushort)(data[start + i * 2] | (data[start + i * 2 + 1] << 8));

        var sb = new StringBuilder(count);
        int idx = 0;
        while (idx < count)
        {
            ushort ch = chars[idx];
            switch (ch)
            {
                case 0x0000: break;
                case >= 0x0001 and <= 0x0008: idx += 7; break;
                case 0x0009: sb.Append('\t'); idx += 7; break;
                case 0x000A: sb.Append('\n'); break;
                case 0x000D: break;
                case (>= 0x000B and <= 0x000C) or (>= 0x000E and <= 0x001F): idx += 7; break;
                case >= 0xF020 and <= 0xF07F: break;
                default: sb.Append((char)ch); break;
            }
            idx++;
        }
        return sb.ToString();
    }

    private static List<TextAnnotation> ApplyCharShapes(string text, List<(uint Pos, ushort ShapeIdx)> runs, List<CharShape> shapes)
    {
        var annotations = new List<TextAnnotation>();
        if (runs.Count == 0 || shapes.Count == 0) return annotations;

        // Byte offset of each char (UTF-8), matching Rust's char_indices over a UTF-8 string.
        var charByteOffsets = new List<int>();
        int running = 0;
        foreach (var rune in text.EnumerateRunes())
        {
            charByteOffsets.Add(running);
            running += rune.Utf8SequenceLength;
        }
        int totalChars = charByteOffsets.Count;
        int totalBytes = running;

        var sorted = runs.OrderBy(r => r.Pos).ToList();
        for (int i = 0; i < sorted.Count; i++)
        {
            uint startPos = sorted[i].Pos;
            ushort shapeIdx = sorted[i].ShapeIdx;
            uint endPos = i + 1 < sorted.Count ? sorted[i + 1].Pos : (uint)totalChars;
            if (shapeIdx >= shapes.Count) continue;
            var shape = shapes[shapeIdx];

            int startByte = startPos < charByteOffsets.Count ? charByteOffsets[(int)startPos] : totalBytes;
            int endByte = endPos < charByteOffsets.Count ? charByteOffsets[(int)endPos] : totalBytes;
            if (startByte >= endByte) continue;

            if (shape.Bold) annotations.Add(new TextAnnotation { Start = (uint)startByte, End = (uint)endByte, Kind = AnnotationKind.Bold });
            if (shape.Italic) annotations.Add(new TextAnnotation { Start = (uint)startByte, End = (uint)endByte, Kind = AnnotationKind.Italic });
            if (shape.Underline) annotations.Add(new TextAnnotation { Start = (uint)startByte, End = (uint)endByte, Kind = new AnnotationKind { Which = AnnotationKind.Tag.Underline } });
        }
        return annotations;
    }

    private static byte[] Decompress(byte[] data)
    {
        if (data.Length == 0) return Array.Empty<byte>();
        // Raw deflate (HWP standard).
        try { return Inflate(data, 0); } catch (Exception) when (Cancel.NotCancelling) { }
        // zlib (skip 2-byte header).
        if (data.Length > 2) { try { return Inflate(data, 2); } catch (Exception) when (Cancel.NotCancelling) { } }
        return data;
    }

    private static byte[] Inflate(byte[] data, int skip)
    {
        using var input = new MemoryStream(data, skip, data.Length - skip);
        using var deflate = new DeflateStream(input, CompressionMode.Decompress);
        using var output = new MemoryStream();
        deflate.CopyTo(output);
        return output.ToArray();
    }

    private static string DetectImageMime(byte[] data)
    {
        if (data.Length >= 4 && data[0] == 0x89 && data[1] == 0x50 && data[2] == 0x4E && data[3] == 0x47) return "image/png";
        if (data.Length >= 3 && data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF) return "image/jpeg";
        if (data.Length >= 3 && data[0] == 0x47 && data[1] == 0x49 && data[2] == 0x46) return "image/gif";
        if (data.Length >= 2 && data[0] == 0x42 && data[1] == 0x4D) return "image/bmp";
        return "application/octet-stream";
    }
}
