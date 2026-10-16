using System.IO.Compression;
using System.Security.Cryptography;
using System.Text;

namespace XRay.Content.Tests;

/// <summary>What <see cref="SyntheticPdf"/> encrypts a document with.</summary>
internal sealed class PdfEncryptionSpec
{
    /// <summary>Security handler revision: 2, 3, 4, 5 or 6.</summary>
    public int R { get; init; } = 3;
    /// <summary>For R4: AESV2 when true, RC4 through the V2 crypt filter when false.</summary>
    public bool Aes { get; init; } = true;
    public string UserPassword { get; init; } = "";
    public string OwnerPassword { get; init; } = "owner";
    public int Permissions { get; init; } = unchecked((int)0xFFFFF0C0u) | 0x4 | 0x10; // print + copy
    public bool EncryptMetadata { get; init; } = true;
}

/// <summary>
/// Builds small, valid PDFs in test code: numbered objects, optional Flate streams, and — the
/// reason this exists — encryption by the standard security handler (RC4-40, RC4-128, AESV2,
/// AESV3 R5 and R6), implemented on the writing side straight from ISO 32000-1/-2 so the reader
/// is tested against an independent statement of the algorithms.
/// </summary>
internal sealed class SyntheticPdf
{
    private static readonly byte[] Pad =
    {
        0x28,0xBF,0x4E,0x5E,0x4E,0x75,0x8A,0x41,0x64,0x00,0x4E,0x56,0xFF,0xFA,0x01,0x08,
        0x2E,0x2E,0x00,0xB6,0xD0,0x68,0x3E,0x80,0x2F,0x0C,0xA9,0xFE,0x64,0x53,0x69,0x7A,
    };

    private readonly List<Func<int, Writer, (string Dict, byte[]? Stream)>> _objects = new();
    private string _version = "1.7";

    public SyntheticPdf Version(string version) { _version = version; return this; }

    /// <summary>Add an object whose body is fixed; returns its number (1-based).</summary>
    public int Add(string body)
    {
        _objects.Add((_, _) => (body, null));
        return _objects.Count;
    }

    /// <summary>Add an object whose body depends on its number or needs strings encrypted.</summary>
    public int Add(Func<int, Writer, string> body)
    {
        _objects.Add((n, w) => (body(n, w), null));
        return _objects.Count;
    }

    /// <summary>Add a stream; <paramref name="entries"/> are extra dictionary entries.</summary>
    public int AddStream(string entries, byte[] data, bool flate = false)
    {
        _objects.Add((_, _) =>
        {
            byte[] payload = data;
            string extra = entries;
            if (flate)
            {
                using var ms = new MemoryStream();
                using (var z = new ZLibStream(ms, CompressionLevel.SmallestSize, leaveOpen: true)) z.Write(data);
                payload = ms.ToArray();
                extra += "/Filter/FlateDecode";
            }
            return (extra, payload);
        });
        return _objects.Count;
    }

    public int AddStream(string entries, string text, bool flate = false) =>
        AddStream(entries, Encoding.Latin1.GetBytes(text), flate);

    /// <summary>
    /// A one-page document: <paramref name="content"/> is the page's content stream and
    /// <paramref name="pageExtras"/> adds entries to the page (<c>/Annots[...]</c>). Returns the
    /// builder with the catalog as object 1 (pre-reserved), pages 2, page 3, font 4, content 5.
    /// </summary>
    public static SyntheticPdf OnePage(
        string content, string pageExtras = "", string catalogExtras = "", int pageCount = 1)
    {
        var pdf = new SyntheticPdf();
        pdf.Add($"<</Type/Catalog/Pages 2 0 R{catalogExtras}>>");
        pdf.Add("<</Type/Pages/Kids[3 0 R]/Count 1>>");
        pdf.Add($"<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R{pageExtras}>>");
        pdf.Add("<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>");
        pdf.AddStream("", content);
        return pdf;
    }

    /// <summary>Object number of the Info dictionary, or 0 for none.</summary>
    public int Info { get; set; }

    public byte[] Build(PdfEncryptionSpec? encryption = null)
    {
        byte[] id = Enumerable.Range(0, 16).Select(i => (byte)(i * 7 + 3)).ToArray();
        EncryptionState? state = encryption is null ? null : EncryptionState.Create(encryption, id);

        using var output = new MemoryStream();
        void Write(string s) => output.Write(Encoding.Latin1.GetBytes(s));
        Write($"%PDF-{_version}\n%âãÏÓ\n");

        var offsets = new List<long>();
        int count = _objects.Count;
        for (int i = 0; i < count; i++)
        {
            int number = i + 1;
            var writer = new Writer(state, number);
            var (dict, stream) = _objects[i](number, writer);
            offsets.Add(output.Position);
            Write($"{number} 0 obj\n");
            if (stream is null)
            {
                Write(dict);
                Write("\nendobj\n");
            }
            else
            {
                byte[] payload = state is null ? stream : state.EncryptBytes(stream, number, forStream: true);
                Write($"<<{dict}/Length {payload.Length}>>\nstream\n");
                output.Write(payload);
                Write("\nendstream\nendobj\n");
            }
        }

        int extra = count;
        int infoNumber = Info;
        int encryptNumber = 0;
        if (state is not null)
        {
            encryptNumber = ++extra;
            offsets.Add(output.Position);
            Write($"{encryptNumber} 0 obj\n{state.EncryptDictionary}\nendobj\n");
        }

        long xref = output.Position;
        Write($"xref\n0 {extra + 1}\n0000000000 65535 f \n");
        foreach (long off in offsets) Write($"{off:D10} 00000 n \n");
        Write($"trailer\n<</Size {extra + 1}/Root 1 0 R");
        if (infoNumber > 0) Write($"/Info {infoNumber} 0 R");
        if (encryptNumber > 0) Write($"/Encrypt {encryptNumber} 0 R");
        Write($"/ID[<{Convert.ToHexString(id)}><{Convert.ToHexString(id)}>]>>\nstartxref\n{xref}\n%%EOF");
        return output.ToArray();
    }

    /// <summary>Hands an object's body the means to encrypt its strings.</summary>
    internal sealed class Writer
    {
        private readonly EncryptionState? _state;
        private readonly int _number;
        public Writer(EncryptionState? state, int number) { _state = state; _number = number; }

        /// <summary>A text string as a PDF hex string, encrypted when the document is.</summary>
        public string Hex(string text)
        {
            byte[] raw = Encoding.Latin1.GetBytes(text);
            if (_state is not null) raw = _state.EncryptBytes(raw, _number, forStream: false);
            return "<" + Convert.ToHexString(raw) + ">";
        }
    }

    // ------------------------------------------------------------------------------------
    // The security handler, writing side.
    // ------------------------------------------------------------------------------------

    internal sealed class EncryptionState
    {
        private readonly byte[] _fileKey;
        private readonly int _r;
        private readonly bool _aes;
        private readonly bool _encryptMetadata;
        public string EncryptDictionary { get; private set; } = "";

        private EncryptionState(byte[] fileKey, int r, bool aes, bool encMeta)
        {
            _fileKey = fileKey; _r = r; _aes = aes; _encryptMetadata = encMeta;
        }

        public static EncryptionState Create(PdfEncryptionSpec spec, byte[] id)
        {
            int r = spec.R;
            byte[] user = PasswordBytes(spec.UserPassword, r);
            byte[] owner = PasswordBytes(spec.OwnerPassword, r);
            int p = spec.Permissions;

            if (r >= 5) return CreateV5(spec, user, owner, p);

            int keyLen = r == 2 ? 5 : 16;
            // O (algorithm 3): the user password, RC4-encrypted under a key from the owner password.
            byte[] ownerHash = MD5.HashData(PadPassword(owner.Length == 0 ? user : owner));
            if (r >= 3) for (int i = 0; i < 50; i++) ownerHash = MD5.HashData(ownerHash);
            byte[] ownerKey = ownerHash[..keyLen];
            byte[] o = Rc4(ownerKey, PadPassword(user));
            if (r >= 3)
                for (int i = 1; i <= 19; i++) o = Rc4(Xor(ownerKey, i), o);

            // Key (algorithm 2) and U (algorithms 4 and 5).
            var input = new List<byte>(PadPassword(user));
            input.AddRange(o);
            input.AddRange(BitConverter.GetBytes(p));
            input.AddRange(id);
            if (r >= 4 && !spec.EncryptMetadata) input.AddRange(new byte[] { 0xFF, 0xFF, 0xFF, 0xFF });
            byte[] key = MD5.HashData(input.ToArray());
            if (r >= 3) for (int i = 0; i < 50; i++) key = MD5.HashData(key[..keyLen]);
            key = key[..keyLen];

            byte[] u;
            if (r == 2) u = Rc4(key, Pad);
            else
            {
                byte[] h = MD5.HashData(Concat(Pad, id));
                u = Rc4(key, h);
                for (int i = 1; i <= 19; i++) u = Rc4(Xor(key, i), u);
                u = Concat(u, new byte[16]);
            }

            var state = new EncryptionState(key, r, r == 4 && spec.Aes, spec.EncryptMetadata);
            string cf = r == 4
                ? $"/CF<</StdCF<</CFM/{(spec.Aes ? "AESV2" : "V2")}/AuthEvent/DocOpen/Length 16>>>>/StmF/StdCF/StrF/StdCF"
                  + (spec.EncryptMetadata ? "" : "/EncryptMetadata false")
                : "";
            int v = r == 2 ? 1 : r == 3 ? 2 : 4;
            string length = r == 2 ? "" : "/Length 128";
            state.EncryptDictionary =
                $"<</Filter/Standard/V {v}/R {r}{length}{cf}/O<{Convert.ToHexString(o)}>/U<{Convert.ToHexString(u)}>/P {p}>>";
            return state;
        }

        private static EncryptionState CreateV5(PdfEncryptionSpec spec, byte[] user, byte[] owner, int p)
        {
            int r = spec.R;
            byte[] fileKey = RandomNumberGenerator.GetBytes(32);
            byte[] uValSalt = RandomNumberGenerator.GetBytes(8), uKeySalt = RandomNumberGenerator.GetBytes(8);
            byte[] oValSalt = RandomNumberGenerator.GetBytes(8), oKeySalt = RandomNumberGenerator.GetBytes(8);

            byte[] u = Concat(Hash(Concat(user, uValSalt), user, Array.Empty<byte>(), r), uValSalt, uKeySalt);
            byte[] ue = AesNoPad(Hash(Concat(user, uKeySalt), user, Array.Empty<byte>(), r), fileKey);
            byte[] o = Concat(Hash(Concat(owner, oValSalt, u), owner, u, r), oValSalt, oKeySalt);
            byte[] oe = AesNoPad(Hash(Concat(owner, oKeySalt, u), owner, u, r), fileKey);

            // /Perms: the permission word, encrypted under the file key with AES-ECB.
            var perms = new byte[16];
            BitConverter.GetBytes(p).CopyTo(perms, 0);
            perms[4] = perms[5] = perms[6] = perms[7] = 0xFF;
            perms[8] = spec.EncryptMetadata ? (byte)'T' : (byte)'F';
            perms[9] = (byte)'a'; perms[10] = (byte)'d'; perms[11] = (byte)'b';
            using var ecb = Aes.Create();
            ecb.Key = fileKey; ecb.Mode = CipherMode.ECB; ecb.Padding = PaddingMode.None;
            byte[] encPerms = ecb.CreateEncryptor().TransformFinalBlock(perms, 0, 16);

            var state = new EncryptionState(fileKey, r, aes: true, spec.EncryptMetadata);
            state.EncryptDictionary =
                $"<</Filter/Standard/V 5/R {r}/Length 256/CF<</StdCF<</CFM/AESV3/AuthEvent/DocOpen/Length 32>>>>"
                + "/StmF/StdCF/StrF/StdCF"
                + (spec.EncryptMetadata ? "" : "/EncryptMetadata false")
                + $"/O<{Convert.ToHexString(o)}>/OE<{Convert.ToHexString(oe)}>"
                + $"/U<{Convert.ToHexString(u)}>/UE<{Convert.ToHexString(ue)}>"
                + $"/Perms<{Convert.ToHexString(encPerms)}>/P {p}>>";
            return state;
        }

        public byte[] EncryptBytes(byte[] data, int objectNumber, bool forStream)
        {
            if (_r >= 5) return AesCbcWithIv(_fileKey, data);
            var keyInput = new List<byte>(_fileKey)
            {
                (byte)objectNumber, (byte)(objectNumber >> 8), (byte)(objectNumber >> 16), 0, 0,
            };
            if (_aes) keyInput.AddRange(new byte[] { 0x73, 0x41, 0x6C, 0x54 });
            byte[] objectKey = MD5.HashData(keyInput.ToArray())[..Math.Min(_fileKey.Length + 5, 16)];
            return _aes ? AesCbcWithIv(objectKey, data) : Rc4(objectKey, data);
        }

        private static byte[] AesCbcWithIv(byte[] key, byte[] data)
        {
            byte[] iv = RandomNumberGenerator.GetBytes(16);
            using var aes = Aes.Create();
            aes.Key = key; aes.Mode = CipherMode.CBC; aes.Padding = PaddingMode.PKCS7; aes.IV = iv;
            byte[] cipher = aes.CreateEncryptor().TransformFinalBlock(data, 0, data.Length);
            return Concat(iv, cipher);
        }

        private static byte[] AesNoPad(byte[] key, byte[] data)
        {
            using var aes = Aes.Create();
            aes.Key = key; aes.Mode = CipherMode.CBC; aes.Padding = PaddingMode.None; aes.IV = new byte[16];
            return aes.CreateEncryptor().TransformFinalBlock(data, 0, data.Length);
        }

        /// <summary>R5: SHA-256; R6: algorithm 2.B.</summary>
        private static byte[] Hash(byte[] input, byte[] password, byte[] udata, int r)
        {
            byte[] k = SHA256.HashData(input);
            if (r < 6) return k;
            for (int round = 0; ; round++)
            {
                var k1 = new List<byte>();
                for (int i = 0; i < 64; i++) { k1.AddRange(password); k1.AddRange(k); k1.AddRange(udata); }
                using var aes = Aes.Create();
                aes.Key = k[..16]; aes.IV = k[16..32]; aes.Mode = CipherMode.CBC; aes.Padding = PaddingMode.None;
                byte[] e = aes.CreateEncryptor().TransformFinalBlock(k1.ToArray(), 0, k1.Count);
                int mod = 0; for (int i = 0; i < 16; i++) mod += e[i];
                k = (mod % 3) switch { 0 => SHA256.HashData(e), 1 => SHA384.HashData(e), _ => SHA512.HashData(e) };
                if (round >= 63 && e[^1] <= round + 1 - 32) break;
            }
            return k[..32];
        }

        private static byte[] PasswordBytes(string password, int r) =>
            r >= 5 ? Encoding.UTF8.GetBytes(password) : Encoding.Latin1.GetBytes(password);

        private static byte[] PadPassword(byte[] password)
        {
            var padded = new byte[32];
            int n = Math.Min(password.Length, 32);
            Array.Copy(password, padded, n);
            Array.Copy(Pad, 0, padded, n, 32 - n);
            return padded;
        }

        private static byte[] Xor(byte[] key, int value) => key.Select(b => (byte)(b ^ value)).ToArray();

        private static byte[] Concat(params byte[][] parts) => parts.SelectMany(p => p).ToArray();

        private static byte[] Rc4(byte[] key, byte[] data)
        {
            var s = Enumerable.Range(0, 256).Select(i => (byte)i).ToArray();
            int j = 0;
            for (int i = 0; i < 256; i++)
            {
                j = (j + s[i] + key[i % key.Length]) & 0xFF;
                (s[i], s[j]) = (s[j], s[i]);
            }
            var result = new byte[data.Length];
            int a = 0, b = 0;
            for (int k = 0; k < data.Length; k++)
            {
                a = (a + 1) & 0xFF; b = (b + s[a]) & 0xFF;
                (s[a], s[b]) = (s[b], s[a]);
                result[k] = (byte)(data[k] ^ s[(s[a] + s[b]) & 0xFF]);
            }
            return result;
        }
    }
}
