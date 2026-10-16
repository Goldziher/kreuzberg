using System.IO.Compression;
using System.Xml.Linq;

namespace XRay.Content.Internal.Ooxml;

/// <summary>
/// Thin wrapper over a ZIP-based Office Open XML package. Provides case-sensitive
/// part lookup (matching Rust <c>zip</c>/<c>ZipArchive::by_name</c>) plus helpers to read
/// a part as bytes / UTF-8 string / parsed <see cref="XDocument"/>.
/// </summary>
public sealed class OoxmlPackage : IDisposable
{
    private readonly ZipArchive _archive;
    private readonly Dictionary<string, ZipArchiveEntry> _entries = new(StringComparer.Ordinal);

    public OoxmlPackage(ReadOnlySpan<byte> content)
    {
        // ZipArchive needs a seekable stream; copy the bytes into a MemoryStream.
        var ms = new MemoryStream(content.ToArray(), writable: false);
        _archive = new ZipArchive(ms, ZipArchiveMode.Read);
        foreach (var e in _archive.Entries)
            _entries[e.FullName] = e; // later duplicates win, like a dict insert
    }

    public bool Has(string partName) => _entries.ContainsKey(Normalize(partName));

    public byte[]? ReadBytes(string partName)
    {
        if (!_entries.TryGetValue(Normalize(partName), out var entry)) return null;
        using var s = entry.Open();
        using var ms = new MemoryStream();
        Cancel.CopyTo(s, ms);
        return ms.ToArray();
    }

    public string? ReadString(string partName)
    {
        var bytes = ReadBytes(partName);
        if (bytes is null) return null;
        return DecodeXmlText(bytes);
    }

    public XDocument? ReadXml(string partName)
    {
        var text = ReadString(partName);
        if (text is null) return null;
        try { return Cancel.ParseXml(text, LoadOptions.PreserveWhitespace); }
        catch (Exception) when (Cancel.NotCancelling) { return null; }
    }

    /// <summary>All part names, in archive order.</summary>
    public IEnumerable<string> PartNames => _archive.Entries.Select(e => e.FullName);

    public void Dispose() => _archive.Dispose();

    private static string Normalize(string name) => name.StartsWith('/') ? name[1..] : name;

    /// <summary>Decode part bytes to text, honoring a UTF-8/UTF-16 BOM.</summary>
    internal static string DecodeXmlText(byte[] bytes)
    {
        if (bytes.Length >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF)
            return System.Text.Encoding.UTF8.GetString(bytes, 3, bytes.Length - 3);
        if (bytes.Length >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE)
            return System.Text.Encoding.Unicode.GetString(bytes, 2, bytes.Length - 2);
        if (bytes.Length >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF)
            return System.Text.Encoding.BigEndianUnicode.GetString(bytes, 2, bytes.Length - 2);
        return System.Text.Encoding.UTF8.GetString(bytes);
    }
}
