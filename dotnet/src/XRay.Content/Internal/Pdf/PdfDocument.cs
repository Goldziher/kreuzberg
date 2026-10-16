// Pure-managed PDF document reader: header, xref (table + stream), trailer, object
// streams, indirect-object resolution, page tree. Ports the parts of pdf_oxide/lopdf
// (crates/xberg/src/pdf/**) needed for text + metadata extraction.
using System.Text;

namespace XRay.Content.Internal.Pdf;

public sealed class PdfDocument
{
    private readonly byte[] _buf;
    public int VersionMajor { get; private set; }
    public int VersionMinor { get; private set; }
    public PdfDict Trailer { get; private set; } = new();

    // xref entries: object number -> location
    private readonly Dictionary<int, XrefEntry> _xref = new();
    private readonly Dictionary<int, PdfObject?> _cache = new();
    private readonly HashSet<int> _resolving = new();
    public PdfDecryptor? Decryptor { get; private set; }

    private struct XrefEntry
    {
        public bool InStream;       // stored in an object stream
        public long Offset;         // file offset (Type 1) or object-stream object number (Type 2)
        public int StreamIndex;     // index within object stream (Type 2)
        public int Gen;
    }

    private PdfDocument(byte[] buf) => _buf = buf;

    public static PdfDocument Open(byte[] bytes) => Open(bytes, null);

    /// <summary>
    /// Open a document, authenticating an encrypted one with the first of <paramref name="passwords"/>
    /// that works (each tried as the user password, then as the owner password) and finally with the
    /// empty password. A document that no password opens still loads: <see cref="IsEncrypted"/> is
    /// true, <see cref="Decryptor"/> is null and <see cref="Encryption"/> says what it declares, so the
    /// caller can tell "needs a password" from "corrupt".
    /// </summary>
    public static PdfDocument Open(byte[] bytes, IReadOnlyList<string>? passwords)
    {
        var doc = new PdfDocument(bytes);
        doc.ReadHeader();
        try { doc.ReadXref(); }
        catch (Exception) when (Cancel.NotCancelling) { /* fall through to brute-force */ }
        if (doc._xref.Count == 0 || doc.Trailer.Get("Root") == null || !doc.RootIsValid())
            doc.BruteForceScan();
        doc.SetupDecryption(passwords);
        return doc;
    }

    /// <summary>
    /// What the <c>/Encrypt</c> dictionary declares (algorithm, key length, permissions) and how
    /// the document authenticated; null when the document is not encrypted or the dictionary is
    /// unreadable.
    /// </summary>
    public PdfEncryptionInfo? Encryption { get; private set; }

    private void ReadHeader()
    {
        // "%PDF-x.y" within first 1024 bytes.
        int limit = Math.Min(1024, _buf.Length);
        for (int i = 0; i + 8 <= limit; i++)
        {
            if (_buf[i] == (byte)'%' && _buf[i + 1] == (byte)'P' && _buf[i + 2] == (byte)'D' && _buf[i + 3] == (byte)'F' && _buf[i + 4] == (byte)'-')
            {
                int j = i + 5;
                int major = 0, minor = 0;
                while (j < _buf.Length && _buf[j] >= (byte)'0' && _buf[j] <= (byte)'9') { major = major * 10 + (_buf[j] - (byte)'0'); j++; }
                if (j < _buf.Length && _buf[j] == (byte)'.') j++;
                while (j < _buf.Length && _buf[j] >= (byte)'0' && _buf[j] <= (byte)'9') { minor = minor * 10 + (_buf[j] - (byte)'0'); j++; }
                VersionMajor = major; VersionMinor = minor;
                return;
            }
        }
    }

    // ---- xref parsing ----

    private void ReadXref()
    {
        long start = FindStartXref();
        if (start < 0) throw new InvalidDataException("no startxref");
        var visited = new HashSet<long>();
        long pos = start;
        while (pos >= 0 && pos < _buf.Length && visited.Add(pos))
        {
            Cancel.ThrowIfRequested();
            long next = ParseXrefSection((int)pos, out long xrefStm);
            // Hybrid: also parse the XRefStm if present, then follow Prev.
            if (xrefStm >= 0 && visited.Add(xrefStm))
                ParseXrefSection((int)xrefStm, out _);
            pos = next;
        }
    }

    private long FindStartXref()
    {
        // Search backwards for "startxref".
        var key = "startxref";
        for (int i = _buf.Length - key.Length; i >= 0; i--)
        {
            Cancel.Every(i, 0xFFFF);
            bool m = true;
            for (int k = 0; k < key.Length; k++) if (_buf[i + k] != (byte)key[k]) { m = false; break; }
            if (m)
            {
                int j = i + key.Length;
                while (j < _buf.Length && (_buf[j] == 32 || _buf[j] == 13 || _buf[j] == 10 || _buf[j] == 9)) j++;
                long val = 0; bool any = false;
                while (j < _buf.Length && _buf[j] >= (byte)'0' && _buf[j] <= (byte)'9') { val = val * 10 + (_buf[j] - (byte)'0'); j++; any = true; }
                if (any) return val;
            }
        }
        return -1;
    }

    // Returns the /Prev offset (or -1), and sets xrefStm to /XRefStm offset (or -1).
    private long ParseXrefSection(int pos, out long xrefStm)
    {
        xrefStm = -1;
        if (pos < 0 || pos >= _buf.Length) return -1;
        var lex = new PdfLexer(_buf, pos, ResolveRaw);
        lex.SkipWhitespace();
        // Either "xref" keyword (table) or an object (xref stream).
        if (Match(lex.Pos, "xref"))
        {
            lex.Pos += 4;
            ParseXrefTable(lex);
            lex.SkipWhitespace();
            if (Match(lex.Pos, "trailer"))
            {
                lex.Pos += 7;
                var tr = lex.ParseObject().AsDict();
                if (tr != null)
                {
                    MergeTrailer(tr);
                    if (tr.Get("XRefStm") is PdfNumber xs) xrefStm = (long)xs.Value;
                    if (tr.Get("Prev") is PdfNumber p) return (long)p.Value;
                }
            }
            return -1;
        }
        else
        {
            // xref stream: "n g obj <<...>> stream ... endstream"
            var obj = lex.ParseIndirectObject();
            if (obj is PdfStream st)
            {
                ParseXrefStream(st);
                MergeTrailer(st.Dict);
                if (st.Dict.Get("Prev") is PdfNumber p) return (long)p.Value;
            }
            return -1;
        }
    }

    private void ParseXrefTable(PdfLexer lex)
    {
        while (true)
        {
            lex.SkipWhitespace();
            if (Match(lex.Pos, "trailer")) break;
            // subsection header: "start count"
            if (lex.Pos >= _buf.Length || !(_buf[lex.Pos] >= (byte)'0' && _buf[lex.Pos] <= (byte)'9')) break;
            string? startTok = lex.ReadToken();
            string? countTok = lex.ReadToken();
            if (!int.TryParse(startTok, out int startObj) || !int.TryParse(countTok, out int count)) break;
            for (int i = 0; i < count; i++)
            {
                Cancel.Every(i, 0x3FF);
                lex.SkipWhitespace();
                // Entry: 10 digits offset, 5 digits gen, 1 char type.
                string? offTok = lex.ReadToken();
                string? genTok = lex.ReadToken();
                string? typeTok = lex.ReadToken();
                if (offTok == null || genTok == null || typeTok == null) break;
                int objNum = startObj + i;
                if (typeTok.StartsWith("n") && long.TryParse(offTok, out long off) && int.TryParse(genTok, out int gen))
                {
                    if (!_xref.ContainsKey(objNum))
                        _xref[objNum] = new XrefEntry { InStream = false, Offset = off, Gen = gen };
                }
            }
        }
    }

    private void ParseXrefStream(PdfStream st)
    {
        var wObj = st.Dict.Get("W").AsArray();
        if (wObj == null || wObj.Items.Count < 3) return;
        int w0 = (int)(wObj.Items[0].AsLong() ?? 0);
        int w1 = (int)(wObj.Items[1].AsLong() ?? 0);
        int w2 = (int)(wObj.Items[2].AsLong() ?? 0);
        int size = (int)(st.Dict.Get("Size").AsLong() ?? 0);

        var index = new List<(int start, int count)>();
        if (st.Dict.Get("Index").AsArray() is PdfArray idx)
        {
            for (int i = 0; i + 1 < idx.Items.Count; i += 2)
                index.Add(((int)(idx.Items[i].AsLong() ?? 0), (int)(idx.Items[i + 1].AsLong() ?? 0)));
        }
        else index.Add((0, size));

        byte[] data = PdfFilters.Decode(st, this);
        int rowLen = w0 + w1 + w2;
        if (rowLen == 0) return;
        int p = 0;
        foreach (var (start, count) in index)
        {
            for (int i = 0; i < count && p + rowLen <= data.Length; i++)
            {
                Cancel.Every(i, 0x3FF);
                long f0 = w0 == 0 ? 1 : ReadBE(data, p, w0);
                long f1 = ReadBE(data, p + w0, w1);
                long f2 = ReadBE(data, p + w0 + w1, w2);
                p += rowLen;
                int objNum = start + i;
                if (_xref.ContainsKey(objNum)) continue;
                if (f0 == 1) _xref[objNum] = new XrefEntry { InStream = false, Offset = f1, Gen = (int)f2 };
                else if (f0 == 2) _xref[objNum] = new XrefEntry { InStream = true, Offset = f1, StreamIndex = (int)f2 };
            }
        }
    }

    private static long ReadBE(byte[] d, int off, int len)
    {
        long v = 0;
        for (int i = 0; i < len; i++) v = (v << 8) | d[off + i];
        return v;
    }

    private void MergeTrailer(PdfDict tr)
    {
        foreach (var kv in tr.Map)
            if (!Trailer.Map.ContainsKey(kv.Key))
                Trailer.Map[kv.Key] = kv.Value;
    }

    private void BruteForceScan()
    {
        // Scan for "N G obj" patterns; build xref (later objects win, matching PDF update semantics).
        for (int i = 0; i < _buf.Length; i++)
        {
            Cancel.Every(i, 0xFFFF);
            if (_buf[i] == (byte)'o' && Match(i, "obj") && (i == 0 || IsDelimOrWhite(_buf[i - 1]) || _buf[i - 1] >= (byte)'0' && _buf[i - 1] <= (byte)'9'))
            {
                // Walk back over: whitespace, gen digits, whitespace, obj-num digits.
                int j = i - 1;
                while (j >= 0 && IsWhite(_buf[j])) j--;
                int genEnd = j;
                while (j >= 0 && _buf[j] >= (byte)'0' && _buf[j] <= (byte)'9') j--;
                int genStart = j + 1;
                if (genStart > genEnd) continue;
                while (j >= 0 && IsWhite(_buf[j])) j--;
                int numEnd = j;
                while (j >= 0 && _buf[j] >= (byte)'0' && _buf[j] <= (byte)'9') j--;
                int numStart = j + 1;
                if (numStart > numEnd) continue;
                if (!int.TryParse(Encoding.ASCII.GetString(_buf, numStart, numEnd - numStart + 1), out int objNum)) continue;
                int gen = int.TryParse(Encoding.ASCII.GetString(_buf, genStart, genEnd - genStart + 1), out int g) ? g : 0;
                _xref[objNum] = new XrefEntry { InStream = false, Offset = numStart, Gen = gen };
            }
        }
        _cache.Clear();

        // Merge every "trailer" dictionary found (later ones win via MergeTrailer order),
        // scanning from end to start so the newest update takes precedence but earlier
        // sections still supply keys (e.g. /Info) missing from the last trailer.
        foreach (long tpos in FindAll("trailer"))
        {
            var lex = new PdfLexer(_buf, (int)tpos + 7, ResolveRaw);
            if (lex.ParseObject().AsDict() is PdfDict tr) MergeTrailer(tr);
        }
        if (!RootIsValid())
        {
            // Scan objects for a Catalog and use it (overwriting any bogus Root).
            foreach (var kv in _xref)
            {
                Cancel.ThrowIfRequested();
                var d = Resolve(new PdfRef(kv.Key, kv.Value.Gen)).AsDict();
                if (d != null && d.Get("Type").AsName() == "Catalog")
                {
                    Trailer.Map["Root"] = new PdfRef(kv.Key, kv.Value.Gen);
                    break;
                }
            }
        }
        // Also register objects inside object streams found during scan.
        RegisterObjectStreams();
    }

    private void RegisterObjectStreams()
    {
        foreach (var kv in new List<KeyValuePair<int, XrefEntry>>(_xref))
        {
            Cancel.ThrowIfRequested();
            if (kv.Value.InStream) continue;
            var o = ResolveRaw(kv.Key, kv.Value.Gen);
            if (o is PdfStream st && st.Dict.Get("Type").AsName() == "ObjStm")
            {
                int n = (int)(st.Dict.Get("N").AsLong() ?? 0);
                var (nums, _) = ReadObjStmHeader(st);
                for (int i = 0; i < nums.Count; i++)
                    if (!_xref.ContainsKey(nums[i]))
                        _xref[nums[i]] = new XrefEntry { InStream = true, Offset = kv.Key, StreamIndex = i };
            }
        }
    }

    // ---- object resolution ----

    /// <summary>Resolve an object, following one level of indirect reference. Returns Null on failure.</summary>
    public PdfObject Resolve(PdfObject? o)
    {
        if (o is PdfRef r) return LoadObject(r.Number, r.Generation) ?? PdfObject.Null;
        return o ?? PdfObject.Null;
    }

    public PdfObject? LoadObject(int num, int gen)
    {
        if (_cache.TryGetValue(num, out var cached)) return cached;
        if (!_resolving.Add(num)) return PdfObject.Null; // cycle guard
        try
        {
            PdfObject? result = LoadObjectUncached(num, gen);
            // Decrypt strings/streams if needed (skip xref/ObjStm handled elsewhere).
            _cache[num] = result;
            return result;
        }
        finally { _resolving.Remove(num); }
    }

    private PdfObject? LoadObjectUncached(int num, int gen)
    {
        if (!_xref.TryGetValue(num, out var e)) return null;
        if (!e.InStream)
        {
            var o = ResolveRaw(num, gen);
            if (Decryptor != null && o != null) o = Decryptor.DecryptObject(o, num, e.Gen);
            return o;
        }
        else
        {
            // In an object stream. Object streams themselves are not encrypted per-object
            // (they're decrypted as a whole stream), so strings inside are already plain.
            int streamObjNum = (int)e.Offset;
            var objs = GetObjectStreamObjects(streamObjNum);
            if (objs != null && e.StreamIndex < objs.Count) return objs[e.StreamIndex];
            return null;
        }
    }

    // Raw load (no decryption): parse "n g obj ... endobj" at the xref offset.
    private PdfObject? ResolveRaw(int num, int gen)
    {
        if (!_xref.TryGetValue(num, out var e)) return null;
        if (e.InStream)
        {
            var objs = GetObjectStreamObjects((int)e.Offset);
            if (objs != null && e.StreamIndex < objs.Count) return objs[e.StreamIndex];
            return null;
        }
        if (e.Offset < 0 || e.Offset >= _buf.Length) return null;
        var lex = new PdfLexer(_buf, (int)e.Offset, ResolveRaw);
        return lex.ParseIndirectObject();
    }

    private readonly Dictionary<int, List<PdfObject>?> _objStmCache = new();

    private List<PdfObject>? GetObjectStreamObjects(int streamObjNum)
    {
        if (_objStmCache.TryGetValue(streamObjNum, out var cached)) return cached;
        _objStmCache[streamObjNum] = null; // guard
        if (!_xref.TryGetValue(streamObjNum, out var e) || e.InStream) return null;
        var lex = new PdfLexer(_buf, (int)e.Offset, ResolveRaw);
        var obj = lex.ParseIndirectObject();
        if (obj is not PdfStream st) return null;
        // Object streams are decrypted as a whole (their raw data), then parsed.
        if (Decryptor != null) st = Decryptor.DecryptStreamObject(st, streamObjNum, e.Gen);
        var (nums, offsets) = ReadObjStmHeader(st);
        byte[] data = PdfFilters.Decode(st, this);
        int first = (int)(st.Dict.Get("First").AsLong() ?? 0);
        var list = new List<PdfObject>(nums.Count);
        for (int i = 0; i < nums.Count; i++)
        {
            Cancel.Every(i, 0x3FF);
            int off = first + offsets[i];
            if (off < 0 || off > data.Length) { list.Add(PdfObject.Null); continue; }
            var l2 = new PdfLexer(data, off, ResolveRaw);
            list.Add(l2.ParseObject());
        }
        _objStmCache[streamObjNum] = list;
        return list;
    }

    private (List<int> nums, List<int> offsets) ReadObjStmHeader(PdfStream st)
    {
        var nums = new List<int>(); var offsets = new List<int>();
        int n = (int)(st.Dict.Get("N").AsLong() ?? 0);
        // Header data must be decoded; but header is in the decoded stream before First.
        byte[] data;
        try { data = PdfFilters.Decode(st, this); } catch (Exception) when (Cancel.NotCancelling) { return (nums, offsets); }
        var lex = new PdfLexer(data, 0, ResolveRaw);
        for (int i = 0; i < n; i++)
        {
            Cancel.Every(i, 0x3FF);
            string? a = lex.ReadToken(); string? b = lex.ReadToken();
            if (a == null || b == null) break;
            if (int.TryParse(a, out int on) && int.TryParse(b, out int oo)) { nums.Add(on); offsets.Add(oo); }
        }
        return (nums, offsets);
    }

    // ---- decryption setup ----
    private void SetupDecryption(IReadOnlyList<string>? passwords)
    {
        var enc = Trailer.Get("Encrypt");
        if (enc == null) return;
        // Resolve the encrypt dict WITHOUT decryption (it's never encrypted).
        PdfDict? encDict = enc is PdfRef er ? ResolveRaw(er.Number, er.Generation).AsDict() : enc.AsDict();
        if (encDict == null) return;
        byte[]? id = null;
        if (Trailer.Get("ID").AsArray() is PdfArray idArr && idArr.Items.Count > 0 && idArr.Items[0] is PdfString ids)
            id = ids.Bytes;
        try
        {
            Encryption = PdfDecryptor.Describe(encDict, this);
            if (!Encryption.Supported) return;
            Decryptor = PdfDecryptor.TryCreate(encDict, id, this, passwords, Encryption);
            Encryption.RequiresPassword = Decryptor is null || Encryption.RequiresPassword;
        }
        catch (Exception) when (Cancel.NotCancelling) { Decryptor = null; }
        // Anything parsed while the document was still locked was parsed from ciphertext.
        _cache.Clear();
        _objStmCache.Clear();
    }

    public bool IsEncrypted => Trailer.Get("Encrypt") != null;

    // ---- catalog / pages ----

    public PdfDict? Catalog => Resolve(Trailer.Get("Root")).AsDict();

    public PdfDict? InfoDict
    {
        get
        {
            var info = Trailer.Get("Info");
            if (info == null) return null;
            // Info strings are encrypted; LoadObject applies decryption.
            if (info is PdfRef r) return LoadObject(r.Number, r.Generation).AsDict();
            return info.AsDict();
        }
    }

    private List<PdfDict>? _pages;

    public List<PdfDict> Pages
    {
        get
        {
            if (_pages != null) return _pages;
            _pages = new List<PdfDict>();
            _pageRefs = new List<PdfRef?>();
            var cat = Catalog;
            var pagesRoot = Resolve(cat?.Get("Pages")).AsDict();
            var visited = new HashSet<PdfDict>();
            if (pagesRoot != null) CollectPages(pagesRoot, new PdfDict(), visited, 0, null);
            if (_pages.Count == 0) FallbackCollectPages();
            return _pages;
        }
    }

    private List<PdfRef?>? _pageRefs;

    /// <summary>
    /// One-based page number for each page's indirect reference, for destinations that
    /// name a page object (outline items, link annotations).
    /// </summary>
    public Dictionary<PdfRef, int> PageNumbersByRef
    {
        get
        {
            _ = Pages;
            var map = new Dictionary<PdfRef, int>();
            if (_pageRefs is null) return map;
            for (int i = 0; i < _pageRefs.Count; i++)
                if (_pageRefs[i] is { } r) map[r] = i + 1;
            return map;
        }
    }

    private static readonly string[] InheritableKeys = { "Resources", "MediaBox", "CropBox", "Rotate" };

    private void CollectPages(PdfDict node, PdfDict inherited, HashSet<PdfDict> visited, int depth, PdfRef? nodeRef)
    {
        if (depth > 100 || _pages!.Count > 100000) return;
        if (!visited.Add(node)) return;
        string? type = node.Get("Type").AsName();
        // Build effective inherited attributes. When two Pages nodes on the same root path both
        // name one, the *outer* one wins: the reference walks the tree once per page into a map
        // it only ever fills in, so whichever ancestor is reached first is the one a leaf below
        // sees. ISO 32000-1 §7.7.3.4 gives the nearest ancestor instead, but matching the
        // reference is what keeps a page's size and resources the same on both sides.
        var eff = new PdfDict();
        foreach (var kv in inherited.Map) eff.Map[kv.Key] = kv.Value;
        foreach (var k in InheritableKeys) if (node.Has(k) && !eff.Has(k)) eff.Map[k] = node.Map[k];

        if (type == "Page" || (type == null && node.Has("Contents") && !node.Has("Kids")))
        {
            var page = new PdfDict();
            foreach (var kv in node.Map) page.Map[kv.Key] = kv.Value;
            foreach (var k in InheritableKeys) if (!page.Has(k) && eff.Has(k)) page.Map[k] = eff.Map[k];
            _pages!.Add(page);
            _pageRefs?.Add(nodeRef);
            return;
        }
        var kids = Resolve(node.Get("Kids")).AsArray();
        if (kids == null) return;
        foreach (var kid in kids.Items)
        {
            Cancel.ThrowIfRequested();
            var kd = Resolve(kid).AsDict();
            if (kd != null) CollectPages(kd, eff, visited, depth + 1, kid as PdfRef);
        }
    }

    private void FallbackCollectPages()
    {
        // Scan all objects for /Type /Page.
        foreach (var num in new List<int>(_xref.Keys))
        {
            Cancel.ThrowIfRequested();
            var o = LoadObject(num, _xref[num].Gen);
            var d = o.AsDict();
            if (d != null && d.Get("Type").AsName() == "Page")
            {
                _pages!.Add(d);
                _pageRefs?.Add(new PdfRef(num, _xref[num].Gen));
            }
        }
    }

    public int PageCount => Pages.Count;

    /// <summary>MediaBox of a page as (llx, lly, urx, ury). Defaults to US Letter.</summary>
    public (double llx, double lly, double urx, double ury) GetPageMediaBox(int pageIndex)
    {
        if (pageIndex < 0 || pageIndex >= Pages.Count) return (0, 0, 612, 792);
        var mb = Resolve(Pages[pageIndex].Get("MediaBox")).AsArray();
        if (mb != null && mb.Items.Count >= 4)
        {
            double a = Resolve(mb.Items[0]).AsNumber() ?? 0;
            double b = Resolve(mb.Items[1]).AsNumber() ?? 0;
            double c = Resolve(mb.Items[2]).AsNumber() ?? 612;
            double d = Resolve(mb.Items[3]).AsNumber() ?? 792;
            return (a, b, c, d);
        }
        return (0, 0, 612, 792);
    }

    /// <summary>Concatenated, decoded content stream bytes for a page.</summary>
    public byte[] GetPageContent(int pageIndex)
    {
        if (pageIndex < 0 || pageIndex >= Pages.Count) return Array.Empty<byte>();
        var contents = Pages[pageIndex].Get("Contents");
        var resolved = Resolve(contents);
        using var ms = new MemoryStream();
        if (resolved is PdfStream st)
        {
            var d = PdfFilters.Decode(st, this);
            ms.Write(d, 0, d.Length);
        }
        else if (resolved is PdfArray arr)
        {
            foreach (var it in arr.Items)
            {
                if (Resolve(it) is PdfStream s)
                {
                    var d = PdfFilters.Decode(s, this);
                    ms.Write(d, 0, d.Length);
                    ms.WriteByte((byte)'\n');
                }
            }
        }
        return ms.ToArray();
    }

    public byte[] DecodeStream(PdfStream st) => PdfFilters.Decode(st, this);

    // ---- helpers ----
    private bool Match(int at, string s)
    {
        if (at < 0 || at + s.Length > _buf.Length) return false;
        for (int i = 0; i < s.Length; i++) if (_buf[at + i] != (byte)s[i]) return false;
        return true;
    }

    private long FindLast(string s)
    {
        for (int i = _buf.Length - s.Length; i >= 0; i--)
            if (Match(i, s)) return i;
        return -1;
    }

    // All occurrences, from end to start.
    private IEnumerable<long> FindAll(string s)
    {
        for (int i = _buf.Length - s.Length; i >= 0; i--)
        {
            Cancel.Every(i, 0xFFFF);
            if (Match(i, s)) yield return i;
        }
    }

    private bool RootIsValid()
    {
        var root = Trailer.Get("Root");
        if (root == null) return false;
        var d = (root is PdfRef r ? ResolveRaw(r.Number, r.Generation) : root).AsDict();
        if (d == null) return false;
        // A catalog has /Type /Catalog or at least a /Pages entry.
        return d.Get("Type").AsName() == "Catalog" || d.Has("Pages");
    }

    private static bool IsWhite(byte b) => b == 0 || b == 9 || b == 10 || b == 12 || b == 13 || b == 32;
    private static bool IsDelimOrWhite(byte b) => IsWhite(b) || b == (byte)'>' || b == (byte)']';
}
