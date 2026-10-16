// Managed 7z container reader — ports the subset of the `sevenz-rust2` crate used by Rust's
// SevenZExtractor (extraction/archive/sevenz.rs): file listing + full-content extraction.
// Supports Copy, LZMA1 and LZMA2 folders (single-coder chains, which is what 7z produces by
// default); filtered/encrypted folders raise a parse error, mirroring the Rust error path.

using System.Text;
using XRay.Content.Internal.Archive;

namespace XRay.Content.Internal.SevenZip;

internal sealed class SevenZipException : Exception
{
    public SevenZipException(string message) : base(message) { }
}

internal static class SevenZipReader
{
    private static ReadOnlySpan<byte> Signature => new byte[] { 0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C };

    // Header property ids (7zFormat.txt).
    private const int KEnd = 0x00;
    private const int KHeader = 0x01;
    private const int KMainStreamsInfo = 0x04;
    private const int KFilesInfo = 0x05;
    private const int KPackInfo = 0x06;
    private const int KUnpackInfo = 0x07;
    private const int KSubStreamsInfo = 0x08;
    private const int KSize = 0x09;
    private const int KCrc = 0x0A;
    private const int KFolder = 0x0B;
    private const int KCodersUnpackSize = 0x0C;
    private const int KNumUnpackStream = 0x0D;
    private const int KEmptyStream = 0x0E;
    private const int KEmptyFile = 0x0F;
    private const int KName = 0x11;
    private const int KEncodedHeader = 0x17;

    /// <summary>Read a 7z archive into the shared archive model (listing + text + raw bytes).</summary>
    public static ArchiveReadResult Read(byte[] bytes)
    {
        var (files, contents) = Parse(bytes);

        var result = new ArchiveReadResult();
        result.Info.Format = "7Z";
        ulong total = 0;
        foreach (var f in files)
        {
            if (!f.IsDir) total += f.Size;
            result.Info.FileList.Add(new ArchiveFileEntry(f.Name, f.Size, f.IsDir));
        }
        result.Info.FileCount = files.Count;
        result.Info.TotalSize = total;

        var strictUtf8 = new UTF8Encoding(false, throwOnInvalidBytes: true);
        for (int i = 0; i < files.Count; i++)
        {
            if (files[i].IsDir || contents[i] is null) continue;
            result.FileBytes.Add(new(files[i].Name, contents[i]!));
            if (ArchiveConstants.IsTextFile(files[i].Name))
            {
                try { result.TextContents.Add(new(files[i].Name, strictUtf8.GetString(contents[i]!))); }
                catch (DecoderFallbackException) { /* non-UTF-8 text file: excluded, mirrors Rust */ }
            }
        }
        return result;
    }

    private sealed record FileEntry(string Name, ulong Size, bool IsDir);

    private static (List<FileEntry> Files, List<byte[]?> Contents) Parse(byte[] bytes)
    {
        if (bytes.Length < 32 || !bytes.AsSpan(0, 6).SequenceEqual(Signature))
            throw new SevenZipException("not a 7z archive");

        long nextHeaderOffset = checked((long)ReadU64(bytes, 12));
        long nextHeaderSize = checked((long)ReadU64(bytes, 20));
        if (nextHeaderSize == 0) return (new List<FileEntry>(), new List<byte[]?>());
        if (32 + nextHeaderOffset + nextHeaderSize > bytes.Length)
            throw new SevenZipException("7z next-header out of range");

        var header = new byte[nextHeaderSize];
        Array.Copy(bytes, 32 + nextHeaderOffset, header, 0, nextHeaderSize);

        var r = new Reader(header);
        int id = (int)r.ReadNumber();
        if (id == KEncodedHeader)
        {
            // The header itself is a compressed folder; decode it and re-parse.
            var si = ParseStreamsInfo(r);
            var decoded = DecodeFolders(bytes, si);
            if (decoded.Count == 0) throw new SevenZipException("empty encoded header");
            r = new Reader(decoded[0]);
            id = (int)r.ReadNumber();
        }
        if (id != KHeader) throw new SevenZipException($"unexpected 7z header id 0x{id:X2}");

        StreamsInfo? streams = null;
        List<FileEntry>? files = null;
        List<bool>? emptyStreamFlags = null;

        while (true)
        {
            int prop = (int)r.ReadNumber();
            if (prop == KEnd) break;
            if (prop == KMainStreamsInfo)
            {
                streams = ParseStreamsInfo(r);
            }
            else if (prop == KFilesInfo)
            {
                (files, emptyStreamFlags) = ParseFilesInfo(r);
            }
            else
            {
                r.SkipBlock();
            }
        }

        files ??= new List<FileEntry>();
        var contents = new List<byte[]?>(new byte[files.Count][]!);
        for (int i = 0; i < files.Count; i++) contents[i] = null;

        if (streams is not null && files.Count > 0)
        {
            // Decode all folders and slice into substreams.
            var substreams = new List<byte[]>();
            var folderData = DecodeFolders(bytes, streams);
            for (int f = 0; f < folderData.Count; f++)
            {
                Cancel.ThrowIfRequested();
                int n = streams.NumUnpackStreams[f];
                if (n == 1)
                {
                    substreams.Add(folderData[f]);
                    continue;
                }
                long off = 0;
                for (int s = 0; s < n; s++)
                {
                    long size = streams.SubStreamSizes[f][s];
                    var piece = new byte[size];
                    Array.Copy(folderData[f], off, piece, 0, size);
                    off += size;
                    substreams.Add(piece);
                }
            }

            // Files with a stream consume substreams in order.
            int next = 0;
            for (int i = 0; i < files.Count; i++)
            {
                Cancel.Every(i, 0x0F);
                bool hasStream = emptyStreamFlags is null || !emptyStreamFlags[i];
                if (!hasStream || files[i].IsDir) continue;
                if (next < substreams.Count)
                {
                    contents[i] = substreams[next];
                    files[i] = files[i] with { Size = (ulong)substreams[next].Length };
                    next++;
                }
            }
        }

        return (files, contents);
    }

    // ── streams info ─────────────────────────────────────────────────────────────
    private sealed class Coder
    {
        public byte[] CodecId = Array.Empty<byte>();
        public byte[] Props = Array.Empty<byte>();
    }

    private sealed class Folder
    {
        public List<Coder> Coders = new();
        public List<long> UnpackSizes = new();
        public int PackStreamIndex; // index of the first packed stream feeding this folder
        public long UnpackSize => UnpackSizes.Count > 0 ? UnpackSizes[^1] : 0;
    }

    private sealed class StreamsInfo
    {
        public long PackPos;
        public List<long> PackSizes = new();
        public List<Folder> Folders = new();
        public List<int> NumUnpackStreams = new();
        public List<List<long>> SubStreamSizes = new();
    }

    private static StreamsInfo ParseStreamsInfo(Reader r)
    {
        var si = new StreamsInfo();

        while (true)
        {
            int prop = (int)r.ReadNumber();
            if (prop == KEnd) break;

            if (prop == KPackInfo)
            {
                si.PackPos = (long)r.ReadNumber();
                int numPack = (int)r.ReadNumber();
                while (true)
                {
                    int p = (int)r.ReadNumber();
                    if (p == KEnd) break;
                    if (p == KSize)
                        for (int i = 0; i < numPack; i++) si.PackSizes.Add((long)r.ReadNumber());
                    else if (p == KCrc) SkipCrcBlock(r, numPack); // not size-prefixed
                    else r.SkipBlock();
                }
            }
            else if (prop == KUnpackInfo)
            {
                while (true)
                {
                    int p = (int)r.ReadNumber();
                    if (p == KEnd) break;
                    if (p == KFolder)
                    {
                        int numFolders = (int)r.ReadNumber();
                        byte external = r.ReadByte();
                        if (external != 0) throw new SevenZipException("external folder data unsupported");
                        int packIndex = 0;
                        for (int i = 0; i < numFolders; i++)
                        {
                            var folder = ParseFolder(r, out int numPackedStreams);
                            folder.PackStreamIndex = packIndex;
                            packIndex += numPackedStreams;
                            si.Folders.Add(folder);
                        }
                    }
                    else if (p == KCodersUnpackSize)
                    {
                        foreach (var folder in si.Folders)
                        {
                            int outStreams = folder.Coders.Count; // 1 out-stream per supported coder
                            for (int i = 0; i < outStreams; i++)
                                folder.UnpackSizes.Add((long)r.ReadNumber());
                        }
                    }
                    else if (p == KCrc) SkipCrcBlock(r, si.Folders.Count); // not size-prefixed
                    else r.SkipBlock();
                }
            }
            else if (prop == KSubStreamsInfo)
            {
                bool haveCounts = false;
                while (true)
                {
                    int p = (int)r.ReadNumber();
                    if (p == KEnd) break;
                    if (p == KNumUnpackStream)
                    {
                        haveCounts = true;
                        foreach (var _ in si.Folders)
                            si.NumUnpackStreams.Add((int)r.ReadNumber());
                    }
                    else if (p == KSize)
                    {
                        EnsureDefaultCounts(si, haveCounts);
                        for (int f = 0; f < si.Folders.Count; f++)
                        {
                            int n = si.NumUnpackStreams[f];
                            var sizes = new List<long>();
                            long sum = 0;
                            for (int s = 0; s < n - 1; s++)
                            {
                                long v = (long)r.ReadNumber();
                                sizes.Add(v);
                                sum += v;
                            }
                            if (n > 0) sizes.Add(si.Folders[f].UnpackSize - sum);
                            si.SubStreamSizes.Add(sizes);
                        }
                    }
                    else if (p == KCrc)
                    {
                        SkipCrcBlock(r, TotalSubStreams(si, haveCounts));
                    }
                    else r.SkipBlock();
                }
                EnsureDefaultCounts(si, haveCounts);
            }
            else
            {
                r.SkipBlock();
            }
        }

        // No SubStreamsInfo → 1 substream per folder.
        if (si.NumUnpackStreams.Count == 0)
            foreach (var _ in si.Folders) si.NumUnpackStreams.Add(1);
        while (si.SubStreamSizes.Count < si.Folders.Count)
        {
            int f = si.SubStreamSizes.Count;
            si.SubStreamSizes.Add(new List<long> { si.Folders[f].UnpackSize });
        }
        return si;
    }

    private static void EnsureDefaultCounts(StreamsInfo si, bool haveCounts)
    {
        if (!haveCounts && si.NumUnpackStreams.Count == 0)
            foreach (var _ in si.Folders) si.NumUnpackStreams.Add(1);
    }

    private static int TotalSubStreams(StreamsInfo si, bool haveCounts)
    {
        if (!haveCounts) return si.Folders.Count;
        int total = 0;
        foreach (var n in si.NumUnpackStreams) total += n;
        return total;
    }

    private static void SkipCrcBlock(Reader r, int numStreams)
    {
        byte allDefined = r.ReadByte();
        int defined = numStreams;
        if (allDefined == 0)
        {
            var bits = r.ReadBitVector(numStreams);
            defined = bits.Count(b => b);
        }
        r.Skip(4 * defined);
    }

    private static Folder ParseFolder(Reader r, out int numPackedStreams)
    {
        var folder = new Folder();
        int numCoders = (int)r.ReadNumber();
        int totalIn = 0, totalOut = 0;

        for (int c = 0; c < numCoders; c++)
        {
            byte flags = r.ReadByte();
            int idSize = flags & 0x0F;
            var coder = new Coder { CodecId = r.ReadBytes(idSize) };
            int inStreams = 1, outStreams = 1;
            if ((flags & 0x10) != 0)
            {
                inStreams = (int)r.ReadNumber();
                outStreams = (int)r.ReadNumber();
            }
            if ((flags & 0x20) != 0)
            {
                int propsSize = (int)r.ReadNumber();
                coder.Props = r.ReadBytes(propsSize);
            }
            totalIn += inStreams;
            totalOut += outStreams;
            folder.Coders.Add(coder);
        }

        int numBindPairs = totalOut - 1;
        for (int i = 0; i < numBindPairs; i++) { r.ReadNumber(); r.ReadNumber(); }
        numPackedStreams = totalIn - numBindPairs;
        if (numPackedStreams > 1)
            for (int i = 0; i < numPackedStreams; i++) r.ReadNumber();

        return folder;
    }

    // ── files info ───────────────────────────────────────────────────────────────
    private static (List<FileEntry>, List<bool>?) ParseFilesInfo(Reader r)
    {
        int numFiles = (int)r.ReadNumber();
        List<bool>? emptyStream = null;
        List<bool>? emptyFile = null;
        var names = new List<string>();

        while (true)
        {
            int type = (int)r.ReadNumber();
            if (type == KEnd) break;
            long size = (long)r.ReadNumber();
            int end = r.Position + (int)size;

            if (type == KEmptyStream)
            {
                emptyStream = r.ReadBitVector(numFiles);
            }
            else if (type == KEmptyFile)
            {
                int numEmptyStreams = emptyStream?.Count(b => b) ?? 0;
                emptyFile = r.ReadBitVector(numEmptyStreams);
            }
            else if (type == KName)
            {
                byte external = r.ReadByte();
                if (external != 0) throw new SevenZipException("external file names unsupported");
                var sb = new StringBuilder();
                while (names.Count < numFiles && r.Position + 1 < end)
                {
                    char ch = (char)(r.ReadByte() | (r.ReadByte() << 8));
                    if (ch == '\0')
                    {
                        names.Add(sb.ToString());
                        sb.Clear();
                    }
                    else sb.Append(ch);
                }
            }

            r.Position = end; // skip any unparsed remainder of the block
        }

        var files = new List<FileEntry>(numFiles);
        int emptyIdx = 0;
        for (int i = 0; i < numFiles; i++)
        {
            string name = i < names.Count ? names[i].Replace('\\', '/') : $"file_{i}";
            bool isEmptyStream = emptyStream is not null && emptyStream[i];
            bool isDir = false;
            if (isEmptyStream)
            {
                // Empty-stream entries are directories unless flagged as empty files.
                bool isEmptyFile = emptyFile is not null && emptyIdx < emptyFile.Count && emptyFile[emptyIdx];
                emptyIdx++;
                isDir = !isEmptyFile;
            }
            files.Add(new FileEntry(name, 0, isDir));
        }
        return (files, emptyStream);
    }

    // ── folder decoding ──────────────────────────────────────────────────────────
    private static List<byte[]> DecodeFolders(byte[] archive, StreamsInfo si)
    {
        // Absolute offset of each packed stream.
        var offsets = new long[si.PackSizes.Count];
        long off = 32 + si.PackPos;
        for (int i = 0; i < si.PackSizes.Count; i++)
        {
            offsets[i] = off;
            off += si.PackSizes[i];
        }

        var result = new List<byte[]>(si.Folders.Count);
        foreach (var folder in si.Folders)
        {
            Cancel.ThrowIfRequested();
            if (folder.Coders.Count != 1)
                throw new SevenZipException("multi-coder 7z folders (filters/encryption) are unsupported");

            int ps = folder.PackStreamIndex;
            if (ps >= offsets.Length) throw new SevenZipException("7z pack stream index out of range");
            long start = offsets[ps];
            long size = si.PackSizes[ps];
            if (start + size > archive.Length) throw new SevenZipException("7z packed stream out of range");
            var packed = new ReadOnlyMemory<byte>(archive, (int)start, (int)size);

            var coder = folder.Coders[0];
            long unpackSize = folder.UnpackSize;
            byte[] data;
            if (coder.CodecId.Length == 1 && coder.CodecId[0] == 0x00)
            {
                data = packed.ToArray(); // Copy
            }
            else if (coder.CodecId.Length == 3 && coder.CodecId[0] == 0x03 && coder.CodecId[1] == 0x01 && coder.CodecId[2] == 0x01)
            {
                if (coder.Props.Length < 1) throw new SevenZipException("missing LZMA properties");
                data = Lzma.DecodeLzma1(coder.Props[0], packed, unpackSize); // props[1..5] = dict size, unused
            }
            else if (coder.CodecId.Length == 1 && coder.CodecId[0] == 0x21)
            {
                data = Lzma.DecodeLzma2(packed, unpackSize);
            }
            else
            {
                throw new SevenZipException($"unsupported 7z codec {Convert.ToHexString(coder.CodecId)}");
            }
            result.Add(data);
        }
        return result;
    }

    private static ulong ReadU64(byte[] b, int i) =>
        b[i] | ((ulong)b[i + 1] << 8) | ((ulong)b[i + 2] << 16) | ((ulong)b[i + 3] << 24)
        | ((ulong)b[i + 4] << 32) | ((ulong)b[i + 5] << 40) | ((ulong)b[i + 6] << 48) | ((ulong)b[i + 7] << 56);

    // ── header byte reader ───────────────────────────────────────────────────────
    private sealed class Reader
    {
        private readonly byte[] _data;
        public int Position;

        public Reader(byte[] data) => _data = data;

        public byte ReadByte()
        {
            if (Position >= _data.Length) throw new SevenZipException("7z header truncated");
            return _data[Position++];
        }

        public byte[] ReadBytes(int n)
        {
            if (Position + n > _data.Length) throw new SevenZipException("7z header truncated");
            var r = new byte[n];
            Array.Copy(_data, Position, r, 0, n);
            Position += n;
            return r;
        }

        public void Skip(int n)
        {
            if (Position + n > _data.Length) throw new SevenZipException("7z header truncated");
            Position += n;
        }

        /// <summary>7z variable-length number: leading 1-bits of the first byte give the extra byte count.</summary>
        public ulong ReadNumber()
        {
            byte first = ReadByte();
            ulong value = 0;
            int mask = 0x80;
            for (int i = 0; i < 8; i++)
            {
                if ((first & mask) == 0)
                {
                    value |= (ulong)(first & (mask - 1)) << (8 * i);
                    return value;
                }
                value |= (ulong)ReadByte() << (8 * i);
                mask >>= 1;
            }
            return value;
        }

        /// <summary>MSB-first bit vector of <paramref name="count"/> bits.</summary>
        public List<bool> ReadBitVector(int count)
        {
            var bits = new List<bool>(count);
            byte b = 0;
            int avail = 0;
            for (int i = 0; i < count; i++)
            {
                if (avail == 0) { b = ReadByte(); avail = 8; }
                bits.Add((b & 0x80) != 0);
                b <<= 1;
                avail--;
            }
            return bits;
        }

        /// <summary>Skip a size-prefixed property block (used for blocks we don't parse).</summary>
        public void SkipBlock()
        {
            long size = (long)ReadNumber();
            Skip((int)size);
        }
    }
}
