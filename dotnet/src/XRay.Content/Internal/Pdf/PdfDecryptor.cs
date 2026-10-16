// PDF standard security handler (ISO 32000-1 §7.6, ISO 32000-2 §7.6): password
// authentication and decryption.
//
// Covers V1/V2 (RC4-40 and RC4-128, R2/R3), V4 (crypt filters: V2 = RC4-128, AESV2, Identity, R4)
// and V5 (AESV3, R5 and R6). A candidate password is tried both as the user password and as the
// owner password; the empty password is always tried last, which is what lets a document that
// only restricts copy/print open without one.
using System.Security.Cryptography;
using System.Text;

namespace XRay.Content.Internal.Pdf;

/// <summary>Which password opened the document.</summary>
public enum PdfAuthentication
{
    /// <summary>No password authenticated (or the document is not encrypted).</summary>
    None,
    /// <summary>The user password, or the empty one.</summary>
    User,
    /// <summary>The owner password.</summary>
    Owner,
}

/// <summary>The permission flags of the <c>/P</c> entry, as a viewer would apply them.</summary>
public sealed class PdfPermissionFlags
{
    /// <summary>The raw 32-bit <c>/P</c> value.</summary>
    public int Raw { get; init; }
    public bool Print { get; init; }
    public bool Modify { get; init; }
    /// <summary>Copy or otherwise extract text and graphics.</summary>
    public bool Copy { get; init; }
    public bool Annotate { get; init; }
    public bool FillForms { get; init; }
    public bool ExtractForAccessibility { get; init; }
    public bool Assemble { get; init; }
    public bool PrintHighQuality { get; init; }

    internal static PdfPermissionFlags FromRaw(int p) => new()
    {
        Raw = p,
        Print = (p & (1 << 2)) != 0,
        Modify = (p & (1 << 3)) != 0,
        Copy = (p & (1 << 4)) != 0,
        Annotate = (p & (1 << 5)) != 0,
        FillForms = (p & (1 << 8)) != 0,
        ExtractForAccessibility = (p & (1 << 9)) != 0,
        Assemble = (p & (1 << 10)) != 0,
        PrintHighQuality = (p & (1 << 11)) != 0,
    };
}

/// <summary>What a document's <c>/Encrypt</c> dictionary declares, whether or not it could be opened.</summary>
public sealed class PdfEncryptionInfo
{
    /// <summary>The security handler (<c>/Filter</c>); <c>Standard</c> is the only one decrypted here.</summary>
    public string Filter { get; init; } = "Standard";
    /// <summary>False for a handler other than Standard (public-key, vendor), which is not decrypted.</summary>
    public bool Supported { get; init; } = true;
    public int V { get; init; }
    public int R { get; init; }
    /// <summary><c>RC4</c>, <c>AESV2</c>, <c>AESV3</c>, or <c>Identity</c>.</summary>
    public string Algorithm { get; init; } = "RC4";
    public int KeyBits { get; init; }
    public bool EncryptMetadata { get; init; } = true;
    public PdfPermissionFlags Permissions { get; init; } = PdfPermissionFlags.FromRaw(-1);
    /// <summary>Whether a password beyond the empty one is needed to read the document.</summary>
    public bool RequiresPassword { get; internal set; }
    public PdfAuthentication Authenticated { get; internal set; }
}

public sealed class PdfDecryptor
{
    private readonly byte[] _key;
    private readonly CryptMethod _stm;
    private readonly CryptMethod _str;
    private readonly CryptMethod _eff;
    private readonly bool _v5;                       // R5/R6: file key is used directly (no per-object key)
    private readonly bool _encryptMetadata;
    private readonly Dictionary<string, CryptMethod> _filters;

    internal enum CryptMethod { None, Rc4, AesV2, AesV3 }

    private static readonly byte[] Pad = {
        0x28,0xBF,0x4E,0x5E,0x4E,0x75,0x8A,0x41,0x64,0x00,0x4E,0x56,0xFF,0xFA,0x01,0x08,
        0x2E,0x2E,0x00,0xB6,0xD0,0x68,0x3E,0x80,0x2F,0x0C,0xA9,0xFE,0x64,0x53,0x69,0x7A
    };

    /// <summary>How the document authenticated, once <see cref="TryCreate"/> succeeded.</summary>
    public PdfAuthentication Authenticated { get; private set; }

    private PdfDecryptor(byte[] key, CryptMethod stm, CryptMethod str, CryptMethod eff, bool v5, bool encMeta,
        Dictionary<string, CryptMethod> filters)
    {
        _key = key; _stm = stm; _str = str; _eff = eff; _v5 = v5; _encryptMetadata = encMeta; _filters = filters;
    }

    /// <summary>Read the encryption dictionary's declarations without authenticating.</summary>
    public static PdfEncryptionInfo Describe(PdfDict enc, PdfDocument doc)
    {
        string filter = doc.Resolve(enc.Get("Filter")).AsName() ?? "Standard";
        int v = (int)(doc.Resolve(enc.Get("V")).AsLong() ?? 0);
        int r = (int)(doc.Resolve(enc.Get("R")).AsLong() ?? 0);
        int p = unchecked((int)(doc.Resolve(enc.Get("P")).AsLong() ?? -1));
        bool encMeta = doc.Resolve(enc.Get("EncryptMetadata")).AsBool() ?? true;
        var (stm, _, _, _, bits) = ReadCryptFilters(enc, doc, v);
        string algorithm = stm switch
        {
            CryptMethod.AesV2 => "AESV2",
            CryptMethod.AesV3 => "AESV3",
            CryptMethod.None => v >= 4 ? "Identity" : "RC4",
            _ => "RC4",
        };
        return new PdfEncryptionInfo
        {
            Filter = filter,
            Supported = filter == "Standard" && r >= 2 && r <= 6,
            V = v,
            R = r,
            Algorithm = algorithm,
            KeyBits = bits,
            EncryptMetadata = encMeta,
            Permissions = PdfPermissionFlags.FromRaw(p),
        };
    }

    /// <summary>Authenticate with the empty password only (the original, password-less form).</summary>
    public static PdfDecryptor? TryCreate(PdfDict enc, byte[]? id, PdfDocument doc) =>
        TryCreate(enc, id, doc, null);

    /// <summary>
    /// Authenticate with the first password that works, trying each as the user password and then
    /// the owner password. Null when none does (or the handler is not the Standard one).
    /// </summary>
    public static PdfDecryptor? TryCreate(
        PdfDict enc, byte[]? id, PdfDocument doc, IReadOnlyList<string>? passwords, PdfEncryptionInfo? info = null)
    {
        string? filter = doc.Resolve(enc.Get("Filter")).AsName();
        if (filter != null && filter != "Standard") return null; // only Standard handler

        int v = (int)(doc.Resolve(enc.Get("V")).AsLong() ?? 0);
        int r = (int)(doc.Resolve(enc.Get("R")).AsLong() ?? 0);
        if (r < 2 || r > 6) return null;
        long p = doc.Resolve(enc.Get("P")).AsLong() ?? 0;
        byte[] o = enc.Get("O").AsStringBytes() ?? Array.Empty<byte>();
        byte[] u = enc.Get("U").AsStringBytes() ?? Array.Empty<byte>();
        bool encMeta = doc.Resolve(enc.Get("EncryptMetadata")).AsBool() ?? true;

        var (stm, str, eff, filters, bits) = ReadCryptFilters(enc, doc, v);
        byte[] idBytes = id ?? Array.Empty<byte>();

        // The empty password goes last: a supplied one that is wrong must not be shadowed by it,
        // and the empty one is what opens a document that only restricts permissions.
        var candidates = new List<byte[]>();
        foreach (var pw in passwords ?? Array.Empty<string>())
            candidates.Add(EncodePassword(pw, r));
        candidates.Add(Array.Empty<byte>());

        foreach (var pw in candidates)
        {
            byte[]? key;
            PdfAuthentication who;
            if (r >= 5)
            {
                byte[] oe = enc.Get("OE").AsStringBytes() ?? Array.Empty<byte>();
                byte[] ue = enc.Get("UE").AsStringBytes() ?? Array.Empty<byte>();
                (key, who) = AuthenticateR56(pw, o, u, oe, ue, r);
            }
            else
            {
                int keyLen = v == 1 || r == 2 ? 5 : Math.Max(5, Math.Min(16, bits / 8));
                if (v >= 4) keyLen = 16;
                (key, who) = AuthenticateR234(pw, o, u, (int)p, idBytes, r, keyLen, encMeta);
            }
            if (key is null) continue;

            var dec = new PdfDecryptor(key, stm, str, eff, r >= 5, encMeta, filters) { Authenticated = who };
            if (info is not null)
            {
                info.Authenticated = who;
                info.RequiresPassword = pw.Length != 0;
            }
            return dec;
        }
        return null;
    }

    private static byte[] EncodePassword(string password, int r)
    {
        if (password.Length == 0) return Array.Empty<byte>();
        if (r >= 5)
        {
            // ISO 32000-2: UTF-8, truncated to 127 bytes (SASLprep is not applied; the
            // characters a password realistically uses are unchanged by it).
            var utf8 = Encoding.UTF8.GetBytes(password);
            return utf8.Length > 127 ? utf8[..127] : utf8;
        }
        // Earlier revisions: PDFDocEncoding, which for a typed password is Latin-1.
        bool latin1 = true;
        foreach (char c in password) if (c > 0xFF) { latin1 = false; break; }
        return latin1 ? Encoding.Latin1.GetBytes(password) : Encoding.UTF8.GetBytes(password);
    }

    // ---- crypt filters ----

    private static (CryptMethod Stm, CryptMethod Str, CryptMethod Eff, Dictionary<string, CryptMethod> Filters, int Bits)
        ReadCryptFilters(PdfDict enc, PdfDocument doc, int v)
    {
        var filters = new Dictionary<string, CryptMethod>(StringComparer.Ordinal) { ["Identity"] = CryptMethod.None };
        if (v < 4)
        {
            int len = (int)(doc.Resolve(enc.Get("Length")).AsLong() ?? 40);
            if (v == 1) len = 40;
            if (len < 40 || len > 128 || len % 8 != 0) len = 40;
            return (CryptMethod.Rc4, CryptMethod.Rc4, CryptMethod.Rc4, filters, len);
        }

        var cf = doc.Resolve(enc.Get("CF")).AsDict();
        if (cf is not null)
        {
            foreach (var kv in cf.Map)
            {
                var d = doc.Resolve(kv.Value).AsDict();
                string? cfm = doc.Resolve(d?.Get("CFM")).AsName();
                filters[kv.Key] = cfm switch
                {
                    "V2" => CryptMethod.Rc4,
                    "AESV2" => CryptMethod.AesV2,
                    "AESV3" => CryptMethod.AesV3,
                    _ => CryptMethod.None,
                };
            }
        }

        CryptMethod Named(string key, string fallback)
        {
            string name = doc.Resolve(enc.Get(key)).AsName() ?? fallback;
            return filters.TryGetValue(name, out var m) ? m : CryptMethod.None;
        }

        var stm = Named("StmF", "Identity");
        var str = Named("StrF", "Identity");
        var eff = enc.Has("EFF") ? Named("EFF", "Identity") : stm;
        int bits = stm == CryptMethod.AesV3 || str == CryptMethod.AesV3 ? 256 : 128;
        return (stm, str, eff, filters, bits);
    }

    // ---- R2..R4 (ISO 32000-1 algorithms 2, 4, 5, 6, 7) ----

    private static (byte[]? Key, PdfAuthentication Who) AuthenticateR234(
        byte[] password, byte[] o, byte[] u, int p, byte[] id, int r, int keyLen, bool encMeta)
    {
        // User password.
        byte[] key = ComputeKeyR234(password, o, p, id, r, keyLen, encMeta);
        if (CheckUserR234(key, u, id, r)) return (key, PdfAuthentication.User);

        // Owner password: recover the user password from /O (algorithm 7), then check that.
        byte[] userPw = RecoverUserPasswordFromOwner(password, o, r, keyLen);
        byte[] key2 = ComputeKeyR234(userPw, o, p, id, r, keyLen, encMeta);
        if (CheckUserR234(key2, u, id, r)) return (key2, PdfAuthentication.Owner);
        return (null, PdfAuthentication.None);
    }

    private static bool CheckUserR234(byte[] key, byte[] u, byte[] id, int r)
    {
        if (u.Length < 16) return false;
        if (r == 2)
        {
            byte[] expect = Rc4(key, Pad);
            return u.Length >= 32 ? expect.AsSpan().SequenceEqual(u.AsSpan(0, 32)) : false;
        }
        using var md5 = MD5.Create();
        var input = new byte[Pad.Length + id.Length];
        Array.Copy(Pad, input, Pad.Length);
        Array.Copy(id, 0, input, Pad.Length, id.Length);
        byte[] h = md5.ComputeHash(input);
        byte[] x = Rc4(key, h);
        for (int i = 1; i <= 19; i++)
        {
            var k = new byte[key.Length];
            for (int j = 0; j < key.Length; j++) k[j] = (byte)(key[j] ^ i);
            x = Rc4(k, x);
        }
        return x.AsSpan(0, 16).SequenceEqual(u.AsSpan(0, 16));
    }

    private static byte[] RecoverUserPasswordFromOwner(byte[] ownerPassword, byte[] o, int r, int keyLen)
    {
        using var md5 = MD5.Create();
        byte[] padded = PadPassword(ownerPassword);
        byte[] hash = md5.ComputeHash(padded);
        if (r >= 3) for (int i = 0; i < 50; i++) hash = md5.ComputeHash(hash);
        byte[] ownerKey = hash[..keyLen];
        byte[] oPad = o.Length >= 32 ? o[..32] : Pad2(o, 32);
        if (r == 2) return Rc4(ownerKey, oPad);
        byte[] x = oPad;
        for (int i = 19; i >= 0; i--)
        {
            var k = new byte[ownerKey.Length];
            for (int j = 0; j < k.Length; j++) k[j] = (byte)(ownerKey[j] ^ i);
            x = Rc4(k, x);
        }
        return x;
    }

    private static byte[] PadPassword(byte[] password)
    {
        byte[] padded = new byte[32];
        int n = Math.Min(password.Length, 32);
        Array.Copy(password, padded, n);
        Array.Copy(Pad, 0, padded, n, 32 - n);
        return padded;
    }

    private static byte[] ComputeKeyR234(byte[] password, byte[] o, int p, byte[] id, int r, int keyLen, bool encMeta)
    {
        using var md5 = MD5.Create();
        var input = new List<byte>();
        input.AddRange(PadPassword(password));
        input.AddRange(o.Length >= 32 ? o[..32] : Pad2(o, 32));
        input.Add((byte)(p & 0xFF));
        input.Add((byte)((p >> 8) & 0xFF));
        input.Add((byte)((p >> 16) & 0xFF));
        input.Add((byte)((p >> 24) & 0xFF));
        input.AddRange(id);
        if (r >= 4 && !encMeta) { input.Add(0xFF); input.Add(0xFF); input.Add(0xFF); input.Add(0xFF); }
        byte[] hash = md5.ComputeHash(input.ToArray());
        if (r >= 3)
            for (int i = 0; i < 50; i++) hash = md5.ComputeHash(hash[..keyLen]);
        return hash[..keyLen];
    }

    private static byte[] Pad2(byte[] src, int len)
    {
        byte[] r = new byte[len];
        Array.Copy(src, r, Math.Min(src.Length, len));
        return r;
    }

    // ---- R5 / R6 (ISO 32000-2 algorithm 2.A) ----

    private static (byte[]? Key, PdfAuthentication Who) AuthenticateR56(
        byte[] password, byte[] o, byte[] u, byte[] oe, byte[] ue, int r)
    {
        if (u.Length < 48) return (null, PdfAuthentication.None);
        byte[] u48 = u[..48];

        // Owner first: the owner password's hash covers the whole of /U, so a document whose
        // owner and user passwords coincide is still recognised as the owner.
        if (o.Length >= 48 && oe.Length >= 32)
        {
            byte[] check = Hash2B(Concat(password, o[32..40], u48), password, u48, r);
            if (check.AsSpan().SequenceEqual(o.AsSpan(0, 32)))
            {
                byte[] ik = Hash2B(Concat(password, o[40..48], u48), password, u48, r);
                var key = DecryptKeyBlob(ik, oe);
                if (key is not null) return (key, PdfAuthentication.Owner);
            }
        }

        byte[] ucheck = Hash2B(Concat(password, u[32..40]), password, Array.Empty<byte>(), r);
        if (ucheck.AsSpan().SequenceEqual(u.AsSpan(0, 32)) && ue.Length >= 32)
        {
            byte[] ik = Hash2B(Concat(password, u[40..48]), password, Array.Empty<byte>(), r);
            var key = DecryptKeyBlob(ik, ue);
            if (key is not null) return (key, PdfAuthentication.User);
        }
        return (null, PdfAuthentication.None);
    }

    private static byte[]? DecryptKeyBlob(byte[] intermediateKey, byte[] blob)
    {
        try
        {
            using var aes = Aes.Create();
            aes.Key = intermediateKey; aes.Mode = CipherMode.CBC; aes.Padding = PaddingMode.None;
            aes.IV = new byte[16];
            using var dec = aes.CreateDecryptor();
            return dec.TransformFinalBlock(blob, 0, 32);
        }
        catch (CryptographicException) { return null; }
    }

    private static byte[] Hash2B(byte[] input, byte[] password, byte[] udata, int r)
    {
        byte[] k = SHA256.HashData(input);
        if (r < 6) return k; // R5 uses plain SHA-256
        // R6 hardened hash (algorithm 2.B).
        int round = 0;
        byte[] k1 = Array.Empty<byte>();
        while (true)
        {
            Cancel.Every(round, 0x0F);
            int blockLen = password.Length + k.Length + udata.Length;
            if (k1.Length != blockLen * 64) k1 = new byte[blockLen * 64];
            for (int i = 0; i < 64; i++)
            {
                int off = i * blockLen;
                Buffer.BlockCopy(password, 0, k1, off, password.Length);
                Buffer.BlockCopy(k, 0, k1, off + password.Length, k.Length);
                Buffer.BlockCopy(udata, 0, k1, off + password.Length + k.Length, udata.Length);
            }
            byte[] e;
            using (var aes = Aes.Create())
            {
                aes.Mode = CipherMode.CBC; aes.Padding = PaddingMode.None;
                aes.Key = k[..16]; aes.IV = k[16..32];
                using var enc = aes.CreateEncryptor();
                e = enc.TransformFinalBlock(k1, 0, k1.Length);
            }
            int mod = 0; for (int i = 0; i < 16; i++) mod += e[i];
            mod %= 3;
            k = mod == 0 ? SHA256.HashData(e) : mod == 1 ? SHA384.HashData(e) : SHA512.HashData(e);
            round++;
            if (round >= 64 && (e[^1] & 0xFF) <= round - 32) break;
        }
        return k[..32];
    }

    private static byte[] Concat(params byte[][] arrs)
    {
        int n = 0; foreach (var a in arrs) n += a.Length;
        byte[] r = new byte[n]; int p = 0;
        foreach (var a in arrs) { Array.Copy(a, 0, r, p, a.Length); p += a.Length; }
        return r;
    }

    // ---- object decryption ----

    private byte[] ObjectKey(int num, int gen, CryptMethod method)
    {
        if (_v5) return _key;
        var input = new List<byte>(_key);
        input.Add((byte)(num & 0xFF));
        input.Add((byte)((num >> 8) & 0xFF));
        input.Add((byte)((num >> 16) & 0xFF));
        input.Add((byte)(gen & 0xFF));
        input.Add((byte)((gen >> 8) & 0xFF));
        if (method == CryptMethod.AesV2) { input.Add(0x73); input.Add(0x41); input.Add(0x6C); input.Add(0x54); } // "sAlT"
        byte[] h = MD5.HashData(input.ToArray());
        int len = Math.Min(_key.Length + 5, 16);
        return h[..len];
    }

    private byte[] DecryptBytes(byte[] data, int num, int gen, CryptMethod method)
    {
        if (data.Length == 0 || method == CryptMethod.None) return data;
        byte[] key = ObjectKey(num, gen, method);
        if (method == CryptMethod.AesV2 || method == CryptMethod.AesV3)
        {
            if (data.Length < 32) return Array.Empty<byte>(); // IV + at least one block
            try
            {
                using var aes = Aes.Create();
                aes.Key = key;
                aes.Mode = CipherMode.CBC; aes.Padding = PaddingMode.None;
                aes.IV = data[..16];
                using var dec = aes.CreateDecryptor();
                int cipherLen = (data.Length - 16) / 16 * 16;
                byte[] plain = dec.TransformFinalBlock(data, 16, cipherLen);
                // Strip PKCS#7 padding.
                if (plain.Length > 0)
                {
                    int pad = plain[^1];
                    if (pad >= 1 && pad <= 16 && pad <= plain.Length) plain = plain[..^pad];
                }
                return plain;
            }
            catch (CryptographicException) { return Array.Empty<byte>(); }
        }
        return Rc4(key, data);
    }

    private static byte[] Rc4(byte[] key, byte[] data)
    {
        var s = new byte[256];
        for (int i = 0; i < 256; i++) s[i] = (byte)i;
        int j = 0;
        for (int i = 0; i < 256; i++)
        {
            j = (j + s[i] + key[i % key.Length]) & 0xFF;
            (s[i], s[j]) = (s[j], s[i]);
        }
        var outp = new byte[data.Length];
        int x = 0, y = 0;
        for (int k = 0; k < data.Length; k++)
        {
            x = (x + 1) & 0xFF;
            y = (y + s[x]) & 0xFF;
            (s[x], s[y]) = (s[y], s[x]);
            outp[k] = (byte)(data[k] ^ s[(s[x] + s[y]) & 0xFF]);
        }
        return outp;
    }

    /// <summary>Decrypt all strings and stream data reachable within an object.</summary>
    public PdfObject DecryptObject(PdfObject o, int num, int gen)
    {
        switch (o)
        {
            case PdfString s:
                return new PdfString(DecryptBytes(s.Bytes, num, gen, _str));
            case PdfArray a:
                var na = new PdfArray();
                foreach (var it in a.Items) na.Items.Add(DecryptObject(it, num, gen));
                return na;
            case PdfDict d:
                return DecryptDict(d, num, gen);
            case PdfStream st:
                var nd = (PdfDict)DecryptDict(st.Dict, num, gen);
                return new PdfStream(nd, DecryptStreamData(st, num, gen));
            default:
                return o;
        }
    }

    private PdfObject DecryptDict(PdfDict d, int num, int gen)
    {
        var nd = new PdfDict();
        foreach (var kv in d.Map) nd.Map[kv.Key] = DecryptObject(kv.Value, num, gen);
        return nd;
    }

    /// <summary>The raw payload of a stream, decrypted by whichever crypt filter governs it.</summary>
    private byte[] DecryptStreamData(PdfStream st, int num, int gen)
    {
        string? type = st.Dict.Get("Type").AsName();
        // Cross-reference streams are never encrypted; metadata only when /EncryptMetadata says so.
        if (type == "XRef") return st.RawData;
        if (type == "Metadata" && !_encryptMetadata) return st.RawData;

        var method = _stm;
        if (type == "EmbeddedFile") method = _eff;
        if (CryptFilterName(st.Dict) is { } named)
            method = _filters.TryGetValue(named, out var m) ? m : CryptMethod.None;
        return DecryptBytes(st.RawData, num, gen, method);
    }

    /// <summary>The name from a stream's own <c>/Filter /Crypt</c> + <c>/DecodeParms /Name</c>, if it has one.</summary>
    private static string? CryptFilterName(PdfDict dict)
    {
        var filter = dict.Get("Filter");
        bool hasCrypt = filter is PdfName { Value: "Crypt" }
            || (filter is PdfArray fa && fa.Items.Any(i => i is PdfName { Value: "Crypt" }));
        if (!hasCrypt) return null;
        var parms = dict.Get("DecodeParms") ?? dict.Get("DP");
        PdfDict? pd = parms as PdfDict;
        if (pd is null && parms is PdfArray pa)
        {
            var filters = (filter as PdfArray)?.Items;
            if (filters is not null)
                for (int i = 0; i < filters.Count && i < pa.Items.Count; i++)
                    if (filters[i] is PdfName { Value: "Crypt" }) pd = pa.Items[i] as PdfDict;
        }
        return pd?.Get("Name") is PdfName n ? n.Value : "Identity";
    }

    /// <summary>Decrypt a stream object's raw data (dict kept as-is except nested strings).</summary>
    public PdfStream DecryptStreamObject(PdfStream st, int num, int gen) =>
        new(st.Dict, DecryptStreamData(st, num, gen));
}
