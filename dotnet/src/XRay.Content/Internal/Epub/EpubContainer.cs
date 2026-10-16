// Ported from crates/xberg/src/extractors/epub/parsing.rs
// EPUB ZIP archive and container.xml / href-resolution utilities.

using System.IO.Compression;
using System.Text;
using System.Xml.Linq;

namespace XRay.Content.Internal.Epub;

/// <summary>Raised for fatal EPUB parsing failures (mirrors Rust `XbergError::Parsing`).</summary>
public sealed class EpubParseException : Exception
{
    public EpubParseException(string message) : base(message) { }
}

/// <summary>A resolved, package-relative href plus an optional fragment. Mirrors Rust `CanonicalHref`.</summary>
public readonly struct CanonicalHref
{
    public string Path { get; }
    public string? Fragment { get; }
    public CanonicalHref(string path, string? fragment)
    {
        Path = path;
        Fragment = fragment;
    }
}

/// <summary>Low-level container/ZIP/href helpers for EPUB extraction.</summary>
internal static class EpubContainer
{
    /// <summary>Parse container.xml to find the OPF (rootfile full-path). Mirrors `parse_container_xml`.</summary>
    public static string ParseContainerXml(string xml)
    {
        XDocument doc;
        try
        {
            doc = Cancel.ParseXml(xml, LoadOptions.PreserveWhitespace);
        }
        catch (Exception e) when (Cancel.NotCancelling)
        {
            throw new EpubParseException($"Failed to parse container.xml: {e.Message}");
        }

        foreach (var node in doc.Descendants())
        {
            if (node.Name.LocalName == "rootfile")
            {
                var fullPath = node.Attribute("full-path")?.Value;
                if (fullPath is not null)
                    return fullPath;
            }
        }

        throw new EpubParseException("No rootfile found in container.xml");
    }

    /// <summary>Read a UTF-8 text entry from the ZIP. Throws when the entry is missing. Mirrors `read_file_from_zip`.</summary>
    public static string ReadFileFromZip(ZipArchive archive, string path)
    {
        var entry = archive.GetEntry(path);
        if (entry is null)
            throw new EpubParseException($"File not found in EPUB: {path}");

        using var stream = entry.Open();
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return Cancel.ReadToEnd(reader);
    }

    /// <summary>Read a binary entry from the ZIP, or an empty array if missing/unreadable.</summary>
    public static byte[] ReadBytesFromZip(ZipArchive archive, string path)
    {
        var entry = archive.GetEntry(path);
        if (entry is null)
            return Array.Empty<byte>();
        try
        {
            using var stream = entry.Open();
            using var ms = new MemoryStream();
            Cancel.CopyTo(stream, ms);
            return ms.ToArray();
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            return Array.Empty<byte>();
        }
    }

    private static (string Path, string? Fragment) SplitHref(string href)
    {
        int hash = href.IndexOf('#');
        if (hash < 0) return (href, null);
        return (href.Substring(0, hash), href.Substring(hash + 1));
    }

    /// <summary>
    /// Resolve an EPUB href relative to the OPF directory. Returns false (with an error message)
    /// when the href escapes the package root or resolves to nothing. Mirrors `resolve_path`.
    /// </summary>
    public static bool TryResolvePath(string baseDir, string href, out CanonicalHref result, out string error)
    {
        result = default;
        error = "";
        var (relativePath, fragment) = SplitHref(href);

        string combined;
        if (relativePath.StartsWith('/'))
            combined = relativePath.TrimStart('/');
        else if (baseDir.Length == 0 || baseDir == ".")
            combined = relativePath;
        else
            combined = $"{baseDir.TrimEnd('/')}/{relativePath}";

        var normalized = new List<string>();
        foreach (var segment in combined.Split('/'))
        {
            switch (segment)
            {
                case "":
                case ".":
                    break;
                case "..":
                    if (normalized.Count == 0)
                    {
                        error = $"EPUB href '{href}' escapes the package root";
                        return false;
                    }
                    normalized.RemoveAt(normalized.Count - 1);
                    break;
                default:
                    normalized.Add(segment);
                    break;
            }
        }

        string path = string.Join("/", normalized);
        if (path.Length == 0)
        {
            error = $"EPUB href '{href}' does not contain a resolvable path";
            return false;
        }

        string? frag = string.IsNullOrEmpty(fragment) ? null : fragment;
        result = new CanonicalHref(path, frag);
        return true;
    }
}
