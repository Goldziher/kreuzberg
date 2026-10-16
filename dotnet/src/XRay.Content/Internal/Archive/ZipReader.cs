// Ported from Rust `crates/xberg/src/extraction/archive/zip.rs`.
// ZIP metadata + text/byte extraction via System.IO.Compression.

using System.IO.Compression;
using System.Text;
using XRay.Content.Core;

namespace XRay.Content.Internal.Archive;

internal static class ZipReader
{
    /// <summary>Read metadata, text contents and raw bytes from a ZIP archive in one pass.</summary>
    /// <remarks>
    /// The central directory is checked against <paramref name="limits"/> before a single entry is
    /// decompressed, so a bomb is refused on its declared sizes rather than on the memory it has
    /// already taken. Upstream reports that refusal as a validation failure rather than a security
    /// one; the wrapping here is that distinction, not a swallowed error.
    /// </remarks>
    public static ArchiveReadResult Read(byte[] bytes, SecurityLimits? limits)
    {
        using var stream = new MemoryStream(bytes, writable: false);
        ZipArchive archive;
        try
        {
            archive = ZipBombValidator.OpenValidated(stream, limits);
        }
        catch (XRay.Content.Core.SecurityException e)
        {
            throw new ValidationException(e.Message);
        }
        using var _archive = archive;

        var result = new ArchiveReadResult { Info = { Format = "ZIP" } };
        ulong totalSize = 0;

        foreach (var entry in archive.Entries)
        {
            Cancel.ThrowIfRequested();
            string path = entry.FullName;
            bool isDir = path.EndsWith("/", StringComparison.Ordinal);
            ulong size = isDir ? 0 : (ulong)entry.Length;
            if (!isDir) totalSize += size;

            result.Info.FileList.Add(new ArchiveFileEntry(path, size, isDir));

            if (isDir) continue;

            // Raw bytes for recursive extraction.
            byte[] raw = ReadAll(entry);
            result.FileBytes.Add(new KeyValuePair<string, byte[]>(path, raw));

            // Text content for text-typed entries.
            if (ArchiveConstants.IsTextFile(path))
            {
                result.TextContents.Add(new KeyValuePair<string, string>(path, DecodeArchiveText(raw)));
            }
        }

        result.Info.FileCount = archive.Entries.Count;
        result.Info.TotalSize = totalSize;
        return result;
    }

    private static byte[] ReadAll(ZipArchiveEntry entry)
    {
        using var es = entry.Open();
        using var ms = new MemoryStream(entry.Length > 0 ? (int)Math.Min(entry.Length, int.MaxValue) : 0);
        Cancel.CopyTo(es, ms);
        return ms.ToArray();
    }

    /// <summary>Decode as UTF-8, returning null when the bytes are not valid UTF-8
    /// (mirrors Rust `read_to_string`/`String::from_utf8` which reject invalid UTF-8).</summary>
    internal static string? TryDecodeUtf8(byte[] data)
    {
        try
        {
            var enc = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);
            return enc.GetString(data);
        }
        catch (DecoderFallbackException)
        {
            return null;
        }
    }

    /// <summary>
    /// Port of `decode_archive_text`. A member that reaches here has already been judged
    /// textual by its extension, so failing to decode it is not grounds for dropping it —
    /// the bytes are substituted and the member still appears. An AppleDouble sidecar named
    /// `._something.txt` is the case that makes the difference visible: it is binary under a
    /// text extension, and skipping it loses a member the archive listing still announces.
    /// </summary>
    internal static string DecodeArchiveText(byte[] data)
    {
        string text = TryDecodeUtf8(data) ?? Encoding.UTF8.GetString(data);
        return text.Length > 0 && text[0] == '\uFEFF' ? text[1..] : text;
    }
}
