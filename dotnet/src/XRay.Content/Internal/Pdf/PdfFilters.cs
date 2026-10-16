// PDF stream filters (ISO 32000-1 §7.4): Flate, LZW, ASCIIHex, ASCII85, RunLength,
// plus PNG/TIFF predictors. Pure managed.
using System.IO.Compression;

namespace XRay.Content.Internal.Pdf;

public static class PdfFilters
{
    /// <summary>Decode a stream's raw data by applying its /Filter chain.</summary>
    public static byte[] Decode(PdfStream stream, PdfDocument doc) => Decode(stream, doc, long.MaxValue);

    /// <summary>
    /// Decode a stream, stopping with a <see cref="Core.SecurityException"/> as soon as a filter
    /// would produce more than <paramref name="maxBytes"/>: the guard against a few kilobytes that
    /// inflate to gigabytes.
    /// </summary>
    public static byte[] Decode(PdfStream stream, PdfDocument doc, long maxBytes)
    {
        byte[] data = stream.RawData;
        var filters = GetNameList(stream.Dict.Get("Filter") ?? stream.Dict.Get("F"), doc);
        if (filters.Count == 0)
        {
            if (data.LongLength > maxBytes) throw Core.SecurityException.ContentTooLarge(data.LongLength, maxBytes);
            return data;
        }
        var parms = GetDictList(stream.Dict.Get("DecodeParms") ?? stream.Dict.Get("DP"), doc, filters.Count);

        for (int i = 0; i < filters.Count; i++)
        {
            string f = filters[i];
            PdfDict? parm = i < parms.Count ? parms[i] : null;
            data = f switch
            {
                "FlateDecode" or "Fl" => ApplyPredictor(Inflate(data, maxBytes), parm, doc),
                "LZWDecode" or "LZW" => ApplyPredictor(LzwDecode(data, EarlyChange(parm, doc)), parm, doc),
                "ASCIIHexDecode" or "AHx" => AsciiHexDecode(data),
                "ASCII85Decode" or "A85" => Ascii85Decode(data),
                "RunLengthDecode" or "RL" => RunLengthDecode(data),
                // Image filters (DCT/CCITT/JBIG2/JPX) are not decoded for text; return as-is.
                _ => data,
            };
            if (data.LongLength > maxBytes)
                throw Core.SecurityException.ContentTooLarge(data.LongLength, maxBytes);
        }
        return data;
    }

    private static List<string> GetNameList(PdfObject? o, PdfDocument doc)
    {
        o = doc.Resolve(o);
        var list = new List<string>();
        if (o is PdfName n) list.Add(n.Value);
        else if (o is PdfArray a)
            foreach (var it in a.Items)
                if (doc.Resolve(it) is PdfName nn) list.Add(nn.Value);
        return list;
    }

    private static List<PdfDict?> GetDictList(PdfObject? o, PdfDocument doc, int count)
    {
        o = doc.Resolve(o);
        var list = new List<PdfDict?>();
        if (o is PdfArray a)
            foreach (var it in a.Items) list.Add(doc.Resolve(it).AsDict());
        else if (o is PdfDict) list.Add(o.AsDict());
        else if (o is PdfNull || o is null) { }
        return list;
    }

    private static bool EarlyChange(PdfDict? parm, PdfDocument doc)
    {
        if (parm == null) return true;
        var ec = doc.Resolve(parm.Get("EarlyChange")).AsLong();
        return ec != 0;
    }

    public static byte[] Inflate(byte[] data) => Inflate(data, long.MaxValue);

    public static byte[] Inflate(byte[] data, long maxBytes)
    {
        // zlib stream: skip 2-byte header if present.
        int offset = 0;
        if (data.Length >= 2)
        {
            int cmf = data[0], flg = data[1];
            if ((cmf & 0x0F) == 8 && ((cmf << 8 | flg) % 31) == 0) offset = 2;
        }
        try
        {
            using var input = new MemoryStream(data, offset, data.Length - offset);
            using var ds = new DeflateStream(input, CompressionMode.Decompress);
            using var outMs = new MemoryStream();
            CopyBounded(ds, outMs, maxBytes);
            return outMs.ToArray();
        }
        catch (Exception e) when (Cancel.NotCancelling && e is not Core.SecurityException)
        {
            // Retry raw (no header skip) as a fallback.
            try
            {
                using var input = new MemoryStream(data);
                using var ds = new DeflateStream(input, CompressionMode.Decompress);
                using var outMs = new MemoryStream();
                CopyBounded(ds, outMs, maxBytes);
                return outMs.ToArray();
            }
            catch (Exception e2) when (Cancel.NotCancelling && e2 is not Core.SecurityException) { return Array.Empty<byte>(); }
        }
    }

    /// <summary>
    /// <see cref="Stream.CopyTo(Stream)"/> that gives up past <paramref name="maxBytes"/> and
    /// notices a cancellation between chunks, so inflating a huge stream is neither unbounded nor
    /// uninterruptible.
    /// </summary>
    private static void CopyBounded(Stream source, MemoryStream destination, long maxBytes)
    {
        var buffer = new byte[81920];
        int read;
        while ((read = source.Read(buffer, 0, buffer.Length)) > 0)
        {
            Cancel.ThrowIfRequested();
            destination.Write(buffer, 0, read);
            if (destination.Length > maxBytes)
                throw Core.SecurityException.ContentTooLarge(destination.Length, maxBytes);
        }
    }

    private static byte[] ApplyPredictor(byte[] data, PdfDict? parm, PdfDocument doc)
    {
        if (parm == null) return data;
        int predictor = (int)(doc.Resolve(parm.Get("Predictor")).AsLong() ?? 1);
        if (predictor <= 1) return data;
        int colors = (int)(doc.Resolve(parm.Get("Colors")).AsLong() ?? 1);
        int bpc = (int)(doc.Resolve(parm.Get("BitsPerComponent")).AsLong() ?? 8);
        int columns = (int)(doc.Resolve(parm.Get("Columns")).AsLong() ?? 1);
        int bytesPerPixel = Math.Max(1, (colors * bpc + 7) / 8);
        int rowLen = (colors * bpc * columns + 7) / 8;
        if (rowLen <= 0) return data;

        if (predictor == 2)
        {
            // TIFF predictor 2 (only handle 8-bit).
            if (bpc == 8)
            {
                var outp = (byte[])data.Clone();
                for (int r = 0; r + rowLen <= outp.Length; r += rowLen)
                    for (int i = bytesPerPixel; i < rowLen; i++)
                        outp[r + i] = (byte)(outp[r + i] + outp[r + i - bytesPerPixel]);
                return outp;
            }
            return data;
        }

        // PNG predictors (>=10): each row prefixed by a filter-type byte.
        var result = new List<byte>(data.Length);
        byte[] prev = new byte[rowLen];
        int pos = 0;
        while (pos + 1 + rowLen <= data.Length + rowLen && pos < data.Length)
        {
            int ft = data[pos++];
            int avail = Math.Min(rowLen, data.Length - pos);
            if (avail <= 0) break;
            byte[] cur = new byte[rowLen];
            Array.Copy(data, pos, cur, 0, avail);
            pos += avail;
            for (int i = 0; i < rowLen; i++)
            {
                int a = i >= bytesPerPixel ? cur[i - bytesPerPixel] : 0;
                int b = prev[i];
                int c = i >= bytesPerPixel ? prev[i - bytesPerPixel] : 0;
                int x = cur[i];
                int val = ft switch
                {
                    0 => x,
                    1 => x + a,
                    2 => x + b,
                    3 => x + (a + b) / 2,
                    4 => x + Paeth(a, b, c),
                    _ => x,
                };
                cur[i] = (byte)(val & 0xFF);
            }
            result.AddRange(cur);
            prev = cur;
        }
        return result.ToArray();
    }

    private static int Paeth(int a, int b, int c)
    {
        int p = a + b - c;
        int pa = Math.Abs(p - a), pb = Math.Abs(p - b), pc = Math.Abs(p - c);
        if (pa <= pb && pa <= pc) return a;
        return pb <= pc ? b : c;
    }

    public static byte[] LzwDecode(byte[] data, bool earlyChange)
    {
        var outp = new List<byte>(data.Length * 3);
        var table = new List<byte[]>(4096);
        void Reset()
        {
            table.Clear();
            for (int i = 0; i < 256; i++) table.Add(new[] { (byte)i });
            table.Add(Array.Empty<byte>()); // 256 clear
            table.Add(Array.Empty<byte>()); // 257 eod
        }
        Reset();
        int codeWidth = 9;
        int bitBuffer = 0, bitCount = 0;
        int pos = 0;
        byte[]? prev = null;
        int ec = earlyChange ? 1 : 0;

        while (pos < data.Length || bitCount >= codeWidth)
        {
            while (bitCount < codeWidth && pos < data.Length)
            {
                bitBuffer = (bitBuffer << 8) | data[pos++];
                bitCount += 8;
            }
            if (bitCount < codeWidth) break;
            int code = (bitBuffer >> (bitCount - codeWidth)) & ((1 << codeWidth) - 1);
            bitCount -= codeWidth;

            if (code == 256) { Reset(); codeWidth = 9; prev = null; continue; }
            if (code == 257) break;

            byte[] entry;
            if (code < table.Count) entry = table[code];
            else if (prev != null) { entry = new byte[prev.Length + 1]; Array.Copy(prev, entry, prev.Length); entry[prev.Length] = prev[0]; }
            else break;

            outp.AddRange(entry);
            if (prev != null)
            {
                var newEntry = new byte[prev.Length + 1];
                Array.Copy(prev, newEntry, prev.Length);
                newEntry[prev.Length] = entry[0];
                table.Add(newEntry);
            }
            prev = entry;

            if (table.Count + ec >= (1 << codeWidth) && codeWidth < 12) codeWidth++;
        }
        return outp.ToArray();
    }

    public static byte[] AsciiHexDecode(byte[] data)
    {
        var outp = new List<byte>();
        int hi = -1;
        foreach (byte b in data)
        {
            if (b == (byte)'>') break;
            int v;
            if (b >= (byte)'0' && b <= (byte)'9') v = b - (byte)'0';
            else if (b >= (byte)'a' && b <= (byte)'f') v = b - (byte)'a' + 10;
            else if (b >= (byte)'A' && b <= (byte)'F') v = b - (byte)'A' + 10;
            else continue;
            if (hi < 0) hi = v;
            else { outp.Add((byte)((hi << 4) | v)); hi = -1; }
        }
        if (hi >= 0) outp.Add((byte)(hi << 4));
        return outp.ToArray();
    }

    public static byte[] Ascii85Decode(byte[] data)
    {
        var outp = new List<byte>();
        var tuple = new int[5];
        int count = 0;
        int i = 0;
        // Skip optional <~ prefix.
        if (data.Length >= 2 && data[0] == (byte)'<' && data[1] == (byte)'~') i = 2;
        for (; i < data.Length; i++)
        {
            byte b = data[i];
            if (b == (byte)'~') break;
            if (b == (byte)'z' && count == 0) { outp.Add(0); outp.Add(0); outp.Add(0); outp.Add(0); continue; }
            if (b < (byte)'!' || b > (byte)'u') continue; // whitespace/other
            tuple[count++] = b - (byte)'!';
            if (count == 5)
            {
                long val = 0;
                for (int k = 0; k < 5; k++) val = val * 85 + tuple[k];
                outp.Add((byte)(val >> 24));
                outp.Add((byte)(val >> 16));
                outp.Add((byte)(val >> 8));
                outp.Add((byte)val);
                count = 0;
            }
        }
        if (count > 0)
        {
            for (int k = count; k < 5; k++) tuple[k] = 84;
            long val = 0;
            for (int k = 0; k < 5; k++) val = val * 85 + tuple[k];
            for (int k = 0; k < count - 1; k++) outp.Add((byte)(val >> (24 - k * 8)));
        }
        return outp.ToArray();
    }

    public static byte[] RunLengthDecode(byte[] data)
    {
        var outp = new List<byte>();
        int i = 0;
        while (i < data.Length)
        {
            int len = data[i++];
            if (len == 128) break;
            if (len < 128)
            {
                for (int k = 0; k <= len && i < data.Length; k++) outp.Add(data[i++]);
            }
            else
            {
                if (i >= data.Length) break;
                byte val = data[i++];
                for (int k = 0; k < 257 - len; k++) outp.Add(val);
            }
        }
        return outp.ToArray();
    }
}
