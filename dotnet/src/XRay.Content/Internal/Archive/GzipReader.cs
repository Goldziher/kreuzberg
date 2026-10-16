// Ported from Rust `crates/xberg/src/extraction/archive/gzip.rs`.
// Gzip decompression + embedded-TAR detection (.tar.gz delegates to the TAR reader).

using System.IO.Compression;
using System.Text;

namespace XRay.Content.Internal.Archive;

internal static class GzipReader
{
    /// <summary>Decompress a gzip stream and extract metadata/text/bytes. If the decompressed
    /// payload is a TAR archive, delegates to <see cref="TarReader"/>; Rust keeps the outer
    /// "GZIP" format label for .tar.gz, so we do too.</summary>
    public static ArchiveReadResult Read(byte[] bytes, XRay.Content.Core.SecurityLimits? limits = null)
    {
        byte[] decompressed = Decompress(bytes, (limits ?? new XRay.Content.Core.SecurityLimits()).MaxArchiveSize);

        if (TarReader.IsTarArchive(decompressed))
        {
            var tar = TarReader.Read(decompressed);
            tar.Info.Format = "GZIP";
            return tar;
        }

        string filename = ReadGzipFilename(bytes) ?? "compressed_content";
        ulong size = (ulong)decompressed.Length;

        var result = new ArchiveReadResult
        {
            Info = new ArchiveInfo
            {
                Format = "GZIP",
                FileList = { new ArchiveFileEntry(filename, size, false) },
                FileCount = 1,
                TotalSize = size,
            },
        };
        result.FileBytes.Add(new KeyValuePair<string, byte[]>(filename, decompressed));

        string? text = ZipReader.TryDecodeUtf8(decompressed);
        if (text is not null)
            result.TextContents.Add(new KeyValuePair<string, string>(filename, text));

        return result;
    }

    /// <summary>
    /// Decompress with a size ceiling, as upstream does (<c>decompress_gzip_limited</c>), so a few
    /// kilobytes of gzip cannot inflate to gigabytes; cancellable between chunks.
    /// </summary>
    private static byte[] Decompress(byte[] bytes, long maxSize)
    {
        using var input = new MemoryStream(bytes, writable: false);
        using var gz = new GZipStream(input, CompressionMode.Decompress);
        using var output = new MemoryStream();
        if (!Cancel.CopyBounded(gz, output, maxSize))
            throw new XRay.Content.Core.ValidationException($"Gzip decompressed size exceeds {maxSize} byte limit");
        return output.ToArray();
    }

    /// <summary>Extract the original filename from the gzip header (FNAME flag), if present.</summary>
    private static string? ReadGzipFilename(byte[] b)
    {
        // Header: ID1 ID2 CM FLG MTIME(4) XFL OS, then optional fields.
        if (b.Length < 10 || b[0] != 0x1F || b[1] != 0x8B) return null;
        byte flg = b[3];
        int pos = 10;

        const int FEXTRA = 1 << 2, FNAME = 1 << 3, FCOMMENT = 1 << 4, FHCRC = 1 << 1;

        if ((flg & FEXTRA) != 0)
        {
            if (pos + 2 > b.Length) return null;
            int xlen = b[pos] | (b[pos + 1] << 8);
            pos += 2 + xlen;
        }
        if ((flg & FNAME) != 0)
        {
            int start = pos;
            while (pos < b.Length && b[pos] != 0) pos++;
            if (pos > start && pos <= b.Length)
                return Encoding.UTF8.GetString(b, start, pos - start);
        }
        _ = FCOMMENT; _ = FHCRC;
        return null;
    }
}
