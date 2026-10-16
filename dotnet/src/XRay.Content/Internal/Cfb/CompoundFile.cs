using System.Text;

namespace XRay.Content.Internal.Cfb;

/// <summary>
/// Minimal reader for OLE2 / Compound File Binary (CFB) containers — the shared foundation for
/// the legacy Office binary formats (.doc, .ppt, .xls, .msg, .hwp).
///
/// Ports the subset of the Rust <c>cfb</c> crate that the extractors use: open a container from
/// bytes, resolve a named stream (regular- or mini-FAT backed), walk the directory tree, and test
/// stream/storage existence. Directory entries form a per-storage red-black tree (traversed here
/// as an unordered set of children — order is irrelevant for name lookup and matches how the Rust
/// callers use <c>walk()</c> / <c>open_stream()</c>).
///
/// The reader is intentionally lenient: sector references past the physical end of the buffer are
/// treated as zero-filled, which lets slightly-truncated MSG files (whose FAT declares more sectors
/// than the file contains) still parse — mirroring the Rust MSG path's <c>pad_cfb_to_fat_size</c>.
/// </summary>
internal sealed class CompoundFile
{
    private const uint MAXREGSECT = 0xFFFFFFFA;
    private const uint DIFSECT = 0xFFFFFFFC;
    private const uint FATSECT = 0xFFFFFFFD;
    private const uint ENDOFCHAIN = 0xFFFFFFFE;
    private const uint FREESECT = 0xFFFFFFFF;
    private const uint NOSTREAM = 0xFFFFFFFF;

    private readonly byte[] _data;
    private readonly int _sectorSize;
    private readonly int _miniSectorSize;
    private readonly uint _miniCutoff;
    private readonly uint[] _fat;
    private readonly uint[] _miniFat;
    private readonly DirEntry[] _dir;
    private readonly byte[] _miniStream;

    private sealed class DirEntry
    {
        public string Name = "";
        public byte Type;      // 0 empty, 1 storage, 2 stream, 5 root
        public uint Left = NOSTREAM;
        public uint Right = NOSTREAM;
        public uint Child = NOSTREAM;
        public uint StartSector;
        public ulong Size;
        public Guid Clsid;
        public bool IsStorage => Type == 1 || Type == 5;
        public bool IsStream => Type == 2;
    }

    private CompoundFile(byte[] data)
    {
        _data = data;
        if (data.Length < 512)
            throw new InvalidDataException("CFB too short");
        // Signature D0 CF 11 E0 A1 B1 1A E1
        if (!(data[0] == 0xD0 && data[1] == 0xCF && data[2] == 0x11 && data[3] == 0xE0 &&
              data[4] == 0xA1 && data[5] == 0xB1 && data[6] == 0x1A && data[7] == 0xE1))
            throw new InvalidDataException("Not a CFB compound file");

        int sectorShift = U16(data, 30);
        int miniShift = U16(data, 32);
        if (sectorShift < 7 || sectorShift > 20) throw new InvalidDataException("Bad sector shift");
        _sectorSize = 1 << sectorShift;
        _miniSectorSize = 1 << miniShift;

        uint numFatSectors = U32(data, 44);
        uint firstDirSector = U32(data, 48);
        _miniCutoff = U32(data, 56);
        uint firstMiniFat = U32(data, 60);
        uint numMiniFat = U32(data, 64);
        uint firstDifat = U32(data, 68);
        uint numDifat = U32(data, 72);

        _fat = BuildFat(firstDifat, numDifat, numFatSectors);
        _miniFat = ReadChainAsUints(firstMiniFat, numMiniFat);
        _dir = ReadDirectory(firstDirSector);

        // Root entry (type 5) holds the mini stream in regular sectors.
        DirEntry? root = _dir.FirstOrDefault(e => e.Type == 5) ?? (_dir.Length > 0 ? _dir[0] : null);
        _miniStream = root is null ? Array.Empty<byte>() : ReadRegularChain(root.StartSector, root.Size);
    }

    public static CompoundFile Open(ReadOnlySpan<byte> data) => new(data.ToArray());

    // ── public API mirroring the Rust cfb crate usage ──────────────────────────

    /// <summary>Read a named stream, returning null when absent. Path components are separated by
    /// '/'; a leading '/' is ignored. Names are matched exactly (including control-char prefixes
    /// like the 0x05 on SummaryInformation).</summary>
    public byte[]? TryReadStream(string path)
    {
        var entry = Resolve(path);
        if (entry is null || !entry.IsStream) return null;
        return ReadStreamData(entry);
    }

    public bool Exists(string path) => Resolve(path) is not null;

    /// <summary>
    /// The root storage's class id, which is what tells one legacy Office binary format from
    /// another — the container magic is shared by .doc, .xls, .ppt, .msg and .hwp alike.
    /// Returns <see cref="Guid.Empty"/> when the container declares none.
    /// </summary>
    public Guid RootClsid
    {
        get
        {
            var root = _dir.FirstOrDefault(e => e.Type == 5) ?? (_dir.Length > 0 ? _dir[0] : null);
            return root?.Clsid ?? Guid.Empty;
        }
    }

    /// <summary>Enumerate every entry (storages + streams) with a full '/'-prefixed path,
    /// depth-first, matching how the Rust callers use <c>comp.walk()</c>.</summary>
    public IEnumerable<CfbEntry> Walk()
    {
        int rootIdx = Array.FindIndex(_dir, e => e.Type == 5);
        if (rootIdx < 0) yield break;
        var stack = new Stack<(int idx, string parentPath)>();
        // Push top-level children of root.
        foreach (int c in Children(rootIdx).AsEnumerable().Reverse())
            stack.Push((c, ""));
        while (stack.Count > 0)
        {
            var (idx, parent) = stack.Pop();
            var e = _dir[idx];
            string path = parent + "/" + e.Name;
            yield return new CfbEntry(path, e.Name, e.IsStorage, e.IsStream);
            if (e.IsStorage)
                foreach (int c in Children(idx).AsEnumerable().Reverse())
                    stack.Push((c, path));
        }
    }

    // ── resolution ──────────────────────────────────────────────────────────────

    private DirEntry? Resolve(string path)
    {
        int rootIdx = Array.FindIndex(_dir, e => e.Type == 5);
        if (rootIdx < 0) return null;
        var parts = path.Split('/', StringSplitOptions.RemoveEmptyEntries);
        int current = rootIdx;
        foreach (var part in parts)
        {
            int next = -1;
            foreach (int c in Children(current))
            {
                if (string.Equals(_dir[c].Name, part, StringComparison.Ordinal)) { next = c; break; }
            }
            if (next < 0) return null;
            current = next;
        }
        return current == rootIdx ? null : _dir[current];
    }

    private List<int> Children(int storageIdx)
    {
        var result = new List<int>();
        uint child = _dir[storageIdx].Child;
        if (child == NOSTREAM || child >= _dir.Length) return result;
        var stack = new Stack<uint>();
        stack.Push(child);
        var seen = new HashSet<uint>();
        while (stack.Count > 0)
        {
            uint id = stack.Pop();
            if (id == NOSTREAM || id >= _dir.Length || !seen.Add(id)) continue;
            result.Add((int)id);
            var e = _dir[id];
            if (e.Left != NOSTREAM) stack.Push(e.Left);
            if (e.Right != NOSTREAM) stack.Push(e.Right);
        }
        // The directory is a red-black tree ordered by (name length, uppercased name); the cfb
        // crate's walk() yields siblings in that canonical order. Reproduce it so recipient /
        // attachment enumeration matches byte-for-byte.
        result.Sort((a, b) => CompareCfbNames(_dir[a].Name, _dir[b].Name));
        return result;
    }

    private static int CompareCfbNames(string a, string b)
    {
        if (a.Length != b.Length) return a.Length.CompareTo(b.Length);
        for (int i = 0; i < a.Length; i++)
        {
            char ca = char.ToUpperInvariant(a[i]);
            char cb = char.ToUpperInvariant(b[i]);
            if (ca != cb) return ca.CompareTo(cb);
        }
        return 0;
    }

    // ── stream reading ────────────────────────────────────────────────────────

    private byte[] ReadStreamData(DirEntry e)
    {
        if (e.Size < _miniCutoff)
            return ReadMiniChain(e.StartSector, e.Size);
        return ReadRegularChain(e.StartSector, e.Size);
    }

    private byte[] ReadRegularChain(uint start, ulong size)
    {
        var outBuf = new byte[size];
        int written = 0;
        uint sector = start;
        var seen = new HashSet<uint>();
        while (sector <= MAXREGSECT && written < (int)size)
        {
            Cancel.ThrowIfRequested();
            if (!seen.Add(sector)) break;
            long off = (long)(sector + 1) * _sectorSize;
            int toCopy = Math.Min(_sectorSize, (int)size - written);
            CopyFromData(off, outBuf, written, toCopy);
            written += toCopy;
            sector = sector < _fat.Length ? _fat[sector] : ENDOFCHAIN;
        }
        return outBuf;
    }

    private byte[] ReadMiniChain(uint start, ulong size)
    {
        var outBuf = new byte[size];
        int written = 0;
        uint mini = start;
        var seen = new HashSet<uint>();
        while (mini <= MAXREGSECT && written < (int)size)
        {
            Cancel.ThrowIfRequested();
            if (!seen.Add(mini)) break;
            long off = (long)mini * _miniSectorSize;
            int toCopy = Math.Min(_miniSectorSize, (int)size - written);
            if (off >= 0 && off < _miniStream.Length)
            {
                int avail = Math.Min(toCopy, _miniStream.Length - (int)off);
                Array.Copy(_miniStream, (int)off, outBuf, written, avail);
            }
            written += toCopy;
            mini = mini < _miniFat.Length ? _miniFat[mini] : ENDOFCHAIN;
        }
        return outBuf;
    }

    private void CopyFromData(long srcOff, byte[] dst, int dstOff, int count)
    {
        // Lenient: zero-fill any part beyond the physical buffer.
        if (srcOff >= _data.Length) return;
        int avail = (int)Math.Min(count, _data.Length - srcOff);
        if (avail > 0) Array.Copy(_data, (int)srcOff, dst, dstOff, avail);
    }

    // ── FAT / DIFAT / directory parsing ─────────────────────────────────────────

    private uint[] BuildFat(uint firstDifat, uint numDifat, uint numFatSectors)
    {
        var fatSectorIds = new List<uint>();
        // First 109 DIFAT entries live in the header at offset 76.
        for (int i = 0; i < 109; i++)
        {
            uint id = U32(_data, 76 + i * 4);
            if (id == FREESECT || id > MAXREGSECT) continue;
            fatSectorIds.Add(id);
        }
        // Additional DIFAT sectors.
        uint difatSector = firstDifat;
        int entriesPerSector = _sectorSize / 4;
        var seenDifat = new HashSet<uint>();
        int guard = 0;
        while (difatSector <= MAXREGSECT && guard++ < 1 << 20 && seenDifat.Add(difatSector))
        {
            Cancel.ThrowIfRequested();
            byte[] sec = RawSector(difatSector);
            for (int i = 0; i < entriesPerSector - 1; i++)
            {
                uint id = U32(sec, i * 4);
                if (id == FREESECT || id > MAXREGSECT) continue;
                fatSectorIds.Add(id);
            }
            difatSector = U32(sec, (entriesPerSector - 1) * 4);
        }

        var fat = new List<uint>(fatSectorIds.Count * entriesPerSector);
        foreach (uint fs in fatSectorIds)
        {
            byte[] sec = RawSector(fs);
            for (int i = 0; i < entriesPerSector; i++)
                fat.Add(U32(sec, i * 4));
        }
        return fat.ToArray();
    }

    private uint[] ReadChainAsUints(uint start, uint expectedSectors)
    {
        var result = new List<uint>();
        int entriesPerSector = _sectorSize / 4;
        uint sector = start;
        var seen = new HashSet<uint>();
        int guard = 0;
        while (sector <= MAXREGSECT && guard++ < 1 << 20 && seen.Add(sector))
        {
            Cancel.ThrowIfRequested();
            byte[] sec = RawSector(sector);
            for (int i = 0; i < entriesPerSector; i++)
                result.Add(U32(sec, i * 4));
            sector = sector < _fat.Length ? _fat[sector] : ENDOFCHAIN;
        }
        return result.ToArray();
    }

    private DirEntry[] ReadDirectory(uint firstDirSector)
    {
        var bytes = new List<byte>();
        uint sector = firstDirSector;
        var seen = new HashSet<uint>();
        int guard = 0;
        while (sector <= MAXREGSECT && guard++ < 1 << 20 && seen.Add(sector))
        {
            Cancel.ThrowIfRequested();
            bytes.AddRange(RawSector(sector));
            sector = sector < _fat.Length ? _fat[sector] : ENDOFCHAIN;
        }
        var raw = bytes.ToArray();
        int count = raw.Length / 128;
        var entries = new DirEntry[count];
        for (int i = 0; i < count; i++)
            entries[i] = ParseDirEntry(raw, i * 128);
        return entries;
    }

    private static DirEntry ParseDirEntry(byte[] b, int off)
    {
        var e = new DirEntry();
        int nameLen = U16(b, off + 64); // bytes incl. terminating null
        if (nameLen > 64) nameLen = 64;
        int charBytes = Math.Max(0, nameLen - 2);
        e.Name = Encoding.Unicode.GetString(b, off, charBytes);
        e.Type = b[off + 66];
        e.Left = U32(b, off + 68);
        e.Right = U32(b, off + 72);
        e.Child = U32(b, off + 76);
        e.StartSector = U32(b, off + 116);
        e.Size = U64(b, off + 120);
        e.Clsid = off + 96 <= b.Length ? new Guid(b.AsSpan(off + 80, 16)) : Guid.Empty;
        return e;
    }

    private byte[] RawSector(uint id)
    {
        var buf = new byte[_sectorSize];
        long off = (long)(id + 1) * _sectorSize;
        CopyFromData(off, buf, 0, _sectorSize);
        return buf;
    }

    private static int U16(byte[] b, int o) => o + 1 < b.Length ? b[o] | (b[o + 1] << 8) : 0;
    private static uint U32(byte[] b, int o) =>
        o + 3 < b.Length ? (uint)(b[o] | (b[o + 1] << 8) | (b[o + 2] << 16) | (b[o + 3] << 24)) : 0u;
    private static ulong U64(byte[] b, int o) => U32(b, o) | ((ulong)U32(b, o + 4) << 32);
}

/// <summary>A single directory entry surfaced by <see cref="CompoundFile.Walk"/>.</summary>
internal readonly record struct CfbEntry(string Path, string Name, bool IsStorage, bool IsStream);
