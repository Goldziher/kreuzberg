using XRay.Content.Core;
using XRay.Content.Extractors;
using XRay.Content.Internal.Pdf;
using XRay.Content.Types;
using Xunit;

namespace XRay.Content.Tests;

/// <summary>
/// The standard security handler: every revision opens with the right password, user or owner,
/// and a document no password opens fails as "needs a password", not as "corrupt".
/// </summary>
/// <remarks>
/// The synthetic documents are encrypted by <see cref="SyntheticPdf"/>, which implements the
/// writing side of each algorithm independently of the reader, so a reader bug cannot cancel
/// against a matching writer bug. The corpus tests then read documents encrypted by qpdf.
/// </remarks>
public class PdfEncryptionTests
{
    private const string Secret = "Top secret text";
    private const string UserPassword = "user-pass";
    private const string OwnerPassword = "owner-pass";

    public static TheoryData<string, int, bool, string, int> Variants() => new()
    {
        { "rc4-40", 2, true, "RC4", 40 },
        { "rc4-128", 3, true, "RC4", 128 },
        { "v4-rc4", 4, false, "RC4", 128 },
        { "aes-128", 4, true, "AESV2", 128 },
        { "aes-256-r5", 5, true, "AESV3", 256 },
        { "aes-256-r6", 6, true, "AESV3", 256 },
    };

    private static PdfEncryptionSpec Spec(int r, bool aes = true, string user = UserPassword, bool metadata = true) =>
        new() { R = r, Aes = aes, UserPassword = user, OwnerPassword = OwnerPassword, EncryptMetadata = metadata };

    private static byte[] Build(PdfEncryptionSpec spec)
    {
        var pdf = SyntheticPdf.OnePage($"BT /F1 24 Tf 72 700 Td ({Secret}) Tj ET");
        pdf.Info = pdf.Add((_, w) => $"<</Title{w.Hex("Secret title")}/Author{w.Hex("Alice")}>>");
        return pdf.Build(spec);
    }

    private static ExtractedDocument Extract(byte[] bytes, PdfOptions? options)
    {
        var result = new Extractor().Extract(
            ExtractInput.FromBytes(bytes, "application/pdf"), new ExtractionConfig { Pdf = options });
        Assert.Empty(result.Errors);
        return Assert.Single(result.Results);
    }

    [Theory]
    [MemberData(nameof(Variants))]
    public void TheUserPasswordOpensTheDocument(string name, int r, bool aes, string algorithm, int bits)
    {
        var doc = Extract(Build(Spec(r, aes)), new PdfOptions { Password = UserPassword });

        Assert.Contains(Secret, doc.Content, StringComparison.Ordinal);
        Assert.Equal("Secret title", doc.Metadata.Title);
        Assert.Equal(new[] { "Alice" }, doc.Metadata.Authors);
        var pdf = Assert.IsType<PdfMetadata>(doc.Metadata.Format!.Payload);
        Assert.True(pdf.IsEncrypted);
        Assert.True(pdf.RequiresPassword);
        Assert.Equal("user", pdf.AuthenticatedAs);
        Assert.Equal(algorithm, pdf.EncryptionAlgorithm);
        Assert.Equal(bits, pdf.EncryptionKeyBits);
        Assert.NotNull(name);
    }

    [Theory]
    [MemberData(nameof(Variants))]
    public void TheOwnerPasswordOpensTheDocumentAndIsReportedAsOwner(
        string name, int r, bool aes, string algorithm, int bits)
    {
        var doc = Extract(Build(Spec(r, aes)), new PdfOptions { Password = OwnerPassword });

        Assert.Contains(Secret, doc.Content, StringComparison.Ordinal);
        var pdf = Assert.IsType<PdfMetadata>(doc.Metadata.Format!.Payload);
        Assert.Equal("owner", pdf.AuthenticatedAs);
        Assert.Equal(algorithm, pdf.EncryptionAlgorithm);
        Assert.NotNull(name);
        Assert.NotEqual(0, bits);
    }

    [Theory]
    [MemberData(nameof(Variants))]
    public void AnEmptyUserPasswordNeedsNoPasswordAtAll(string name, int r, bool aes, string algorithm, int bits)
    {
        var open = new PdfEncryptionSpec { R = r, Aes = aes, UserPassword = "", OwnerPassword = OwnerPassword };
        var doc = Extract(Build(open), options: null);

        Assert.Contains(Secret, doc.Content, StringComparison.Ordinal);
        var pdf = Assert.IsType<PdfMetadata>(doc.Metadata.Format!.Payload);
        Assert.True(pdf.IsEncrypted);
        Assert.False(pdf.RequiresPassword);
        Assert.Equal("user", pdf.AuthenticatedAs);
        Assert.Equal(algorithm, pdf.EncryptionAlgorithm);
        Assert.NotNull(name);
        Assert.NotEqual(0, bits);
    }

    [Theory]
    [MemberData(nameof(Variants))]
    public void NoPasswordIsPasswordRequiredNotCorrupt(string name, int r, bool aes, string algorithm, int bits)
    {
        byte[] bytes = Build(Spec(r, aes));

        var ex = Assert.Throws<PdfPasswordRequiredException>(
            () => new PdfExtractor().Extract(bytes, "application/pdf", new ExtractionConfig()));
        Assert.False(ex.PasswordWasSupplied);
        Assert.Equal(algorithm, ex.Encryption!.Algorithm);

        // Through the public envelope it is an error item with its own type, not "other".
        var result = new Extractor().Extract(ExtractInput.FromBytes(bytes, "application/pdf"), new ExtractionConfig());
        var error = Assert.Single(result.Errors);
        Assert.Equal("password_required", error.ErrorType);
        Assert.Equal(1010u, error.Code);
        Assert.NotNull(name);
        Assert.NotEqual(0, bits);
    }

    [Theory]
    [MemberData(nameof(Variants))]
    public void AWrongPasswordIsInvalidPassword(string name, int r, bool aes, string algorithm, int bits)
    {
        byte[] bytes = Build(Spec(r, aes));

        var ex = Assert.Throws<PdfPasswordRequiredException>(() => new PdfExtractor().Extract(
            bytes, "application/pdf", new ExtractionConfig { Pdf = new PdfOptions { Password = "nope" } }));
        Assert.True(ex.PasswordWasSupplied);

        var result = new Extractor().Extract(
            ExtractInput.FromBytes(bytes, "application/pdf"),
            new ExtractionConfig { Pdf = new PdfOptions { Password = "nope" } });
        Assert.Equal("invalid_password", Assert.Single(result.Errors).ErrorType);
        Assert.NotNull(name);
        Assert.NotNull(algorithm);
        Assert.NotEqual(0, bits);
    }

    [Fact]
    public void ThePasswordListIsTriedInOrderAndAnyMemberMayWin()
    {
        var doc = Extract(Build(Spec(6)), new PdfOptions { Passwords = new List<string> { "a", "b", UserPassword } });
        Assert.Contains(Secret, doc.Content, StringComparison.Ordinal);

        doc = Extract(Build(Spec(3)), new PdfOptions { Password = "wrong", OwnerPassword = OwnerPassword });
        var pdf = Assert.IsType<PdfMetadata>(doc.Metadata.Format!.Payload);
        Assert.Equal("owner", pdf.AuthenticatedAs);
    }

    [Fact]
    public void APasswordWhoseOwnerAndUserPasswordsCoincideIsStillOpened()
    {
        var spec = new PdfEncryptionSpec { R = 6, UserPassword = "same", OwnerPassword = "same" };
        var doc = Extract(Build(spec), new PdfOptions { Password = "same" });
        Assert.Contains(Secret, doc.Content, StringComparison.Ordinal);
    }

    [Theory]
    [InlineData(3, true)]
    [InlineData(4, false)]
    [InlineData(6, true)]
    public void MetadataLeftInTheClearIsStillReadable(int r, bool aes)
    {
        var doc = Extract(Build(Spec(r, aes, metadata: false)), new PdfOptions { Password = UserPassword });
        Assert.Contains(Secret, doc.Content, StringComparison.Ordinal);
    }

    [Fact]
    public void PermissionFlagsAreReportedFromTheEncryptionDictionary()
    {
        // Print and copy allowed; modify, annotate and the rest not.
        var spec = new PdfEncryptionSpec
        {
            R = 6, UserPassword = "", OwnerPassword = OwnerPassword,
            Permissions = unchecked((int)0xFFFFF0C0u) | (1 << 2) | (1 << 4),
        };
        var doc = Extract(Build(spec), options: null);

        var permissions = Assert.IsType<PdfMetadata>(doc.Metadata.Format!.Payload).Permissions!;
        Assert.True(permissions.Print);
        Assert.True(permissions.Copy);
        Assert.False(permissions.Modify);
        Assert.False(permissions.Annotate);
        Assert.False(permissions.FillForms);
        Assert.False(permissions.Assemble);
    }

    [Fact]
    public void AnUnencryptedDocumentReportsNoEncryptionDetail()
    {
        byte[] bytes = SyntheticPdf.OnePage($"BT /F1 24 Tf 72 700 Td ({Secret}) Tj ET").Build();
        var doc = Extract(bytes, options: null);

        var pdf = Assert.IsType<PdfMetadata>(doc.Metadata.Format!.Payload);
        Assert.False(pdf.IsEncrypted);
        Assert.Null(pdf.RequiresPassword);
        Assert.Null(pdf.EncryptionAlgorithm);
        Assert.Null(pdf.Permissions);
        Assert.Null(pdf.AuthenticatedAs);
    }

    [Fact]
    public void ACorruptDocumentIsStillInvalidData()
    {
        byte[] bytes = new byte[] { 1, 2, 3, 4, 5 };
        var result = new Extractor().Extract(ExtractInput.FromBytes(bytes, "application/pdf"), new ExtractionConfig());
        Assert.NotEqual("password_required", Assert.Single(result.Errors).ErrorType);
        Assert.NotEqual("invalid_password", result.Errors[0].ErrorType);
    }

    [Fact]
    public void TheInspectionApiSeesEncryptionWithoutAPassword()
    {
        var pdf = PdfDocument.Open(Build(Spec(4)));
        Assert.True(pdf.IsEncrypted);
        Assert.Null(pdf.Decryptor);
        Assert.Equal("AESV2", pdf.Encryption!.Algorithm);
        Assert.True(pdf.Encryption.RequiresPassword);
        Assert.Equal(PdfAuthentication.None, pdf.Encryption.Authenticated);
    }

    // ---- real documents, encrypted by qpdf (test_documents/pdf/encrypted) ----

    private static string? Fixture(string name)
    {
        var roots = new List<string> { CorpusPresenceTests.CorpusRoot };
        if (Environment.GetEnvironmentVariable("XRAY_TEST_DOCUMENTS") is { Length: > 0 } env) roots.Add(env);
        return roots.Select(r => Path.Combine(r, "pdf", "encrypted", name)).FirstOrDefault(File.Exists);
    }

    private static string Text(byte[] bytes, PdfOptions? options)
    {
        var doc = Extract(bytes, options);
        return doc.Content;
    }

    [Theory]
    [InlineData("rc4-40", "RC4", 40)]
    [InlineData("rc4-128", "RC4", 128)]
    [InlineData("aes-128", "AESV2", 128)]
    [InlineData("aes-256", "AESV3", 256)]
    public void QpdfEncryptedFixturesDecryptToTheOriginalsText(string stem, string algorithm, int bits)
    {
        if (Fixture("original.pdf") is not { } originalPath) return;
        string expected = Text(File.ReadAllBytes(originalPath), null);
        Assert.Contains("Encrypted fixture, page one", expected, StringComparison.Ordinal);

        // Password needed.
        string userPath = Fixture($"{stem}-user.pdf")!;
        byte[] user = File.ReadAllBytes(userPath);
        Assert.Throws<PdfPasswordRequiredException>(
            () => new PdfExtractor().Extract(user, "application/pdf", new ExtractionConfig()));
        Assert.Equal(expected, Text(user, new PdfOptions { Password = UserPassword }));
        Assert.Equal(expected, Text(user, new PdfOptions { Password = OwnerPassword }));

        var meta = Assert.IsType<PdfMetadata>(
            Extract(user, new PdfOptions { Password = UserPassword }).Metadata.Format!.Payload);
        Assert.Equal(algorithm, meta.EncryptionAlgorithm);
        Assert.Equal(bits, meta.EncryptionKeyBits);
        Assert.True(meta.RequiresPassword);

        // Empty user password; the owner password only restricts print and copy.
        byte[] open = File.ReadAllBytes(Fixture($"{stem}-open.pdf")!);
        Assert.Equal(expected, Text(open, null));
        var openMeta = Assert.IsType<PdfMetadata>(Extract(open, null).Metadata.Format!.Payload);
        Assert.False(openMeta.RequiresPassword);
        Assert.False(openMeta.Permissions!.Copy);
    }

    [Theory]
    [InlineData("aes-128-cleartext-metadata.pdf")]
    [InlineData("rc4-128-v4-user.pdf")]
    [InlineData("aes-256-same-passwords.pdf")]
    public void QpdfEncryptedVariantsDecryptToTheOriginalsText(string name)
    {
        if (Fixture("original.pdf") is not { } originalPath) return;
        string expected = Text(File.ReadAllBytes(originalPath), null);

        Assert.Equal(expected, Text(File.ReadAllBytes(Fixture(name)!), new PdfOptions { Password = UserPassword }));
    }
}
