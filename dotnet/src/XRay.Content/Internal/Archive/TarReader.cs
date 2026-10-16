// Ported from Rust `crates/xberg/src/extraction/archive/tar.rs` (the Rust code delegates
// to the `tar` crate; here we parse the ustar/GNU header format directly).

using System.Text;

namespace XRay.Content.Internal.Archive;

internal static class TarReader
{
    private const int BlockSize = 512;

    /// <summary>Read metadata, text contents and raw bytes from a (uncompressed) TAR archive.</summary>
    public static ArchiveReadResult Read(byte[] bytes)
    {
        var result = new ArchiveReadResult { Info = { Format = "TAR" } };
        ulong totalSize = 0;
        int offset = 0;
        string? pendingLongName = null;

        while (offset + BlockSize <= bytes.Length)
        {
            Cancel.ThrowIfRequested();
            // A block of all zeros marks the end of the archive.
            if (IsZeroBlock(bytes, offset))
                break;

            string rawName = ReadString(bytes, offset, 100);
            string prefix = ReadString(bytes, offset + 345, 155);
            char typeFlag = (char)bytes[offset + 156];
            ulong size = ParseOctal(bytes, offset + 124, 12);

            int dataOffset = offset + BlockSize;
            int dataBlocks = (int)((size + BlockSize - 1) / BlockSize);

            // GNU long-name entry: the following entry's real path is this block's data.
            if (typeFlag == 'L')
            {
                pendingLongName = ReadString(bytes, dataOffset, (int)Math.Min(size, (ulong)(bytes.Length - dataOffset)));
                offset = dataOffset + dataBlocks * BlockSize;
                continue;
            }
            // Pax extended headers / global headers: skip (path resolution not needed here).
            if (typeFlag == 'x' || typeFlag == 'g')
            {
                offset = dataOffset + dataBlocks * BlockSize;
                continue;
            }

            string path = pendingLongName ?? (prefix.Length > 0 ? prefix + "/" + rawName : rawName);
            pendingLongName = null;

            bool isDir = typeFlag == '5' || path.EndsWith("/", StringComparison.Ordinal);
            if (!isDir) totalSize += size;

            result.Info.FileList.Add(new ArchiveFileEntry(path, size, isDir));

            if (!isDir && size > 0 && dataOffset + (int)size <= bytes.Length)
            {
                byte[] data = new byte[(int)size];
                Array.Copy(bytes, dataOffset, data, 0, (int)size);
                result.FileBytes.Add(new KeyValuePair<string, byte[]>(path, data));

                if (ArchiveConstants.IsTextFile(path))
                {
                    result.TextContents.Add(new KeyValuePair<string, string>(path, ZipReader.DecodeArchiveText(data)));
                }
            }
            else if (!isDir)
            {
                result.FileBytes.Add(new KeyValuePair<string, byte[]>(path, Array.Empty<byte>()));
            }

            offset = dataOffset + dataBlocks * BlockSize;
        }

        if (result.Info.FileList.Count == 0)
            throw new InvalidDataException("Failed to read TAR archive: no entries");

        result.Info.FileCount = result.Info.FileList.Count;
        result.Info.TotalSize = totalSize;
        return result;
    }

    /// <summary>Detects a TAR archive by the "ustar" magic at offset 257.</summary>
    public static bool IsTarArchive(byte[] data) =>
        data.Length > 262
        && data[257] == (byte)'u' && data[258] == (byte)'s' && data[259] == (byte)'t'
        && data[260] == (byte)'a' && data[261] == (byte)'r';

    private static bool IsZeroBlock(byte[] b, int offset)
    {
        for (int i = 0; i < BlockSize; i++)
            if (b[offset + i] != 0) return false;
        return true;
    }

    private static string ReadString(byte[] b, int offset, int len)
    {
        int end = offset;
        int limit = offset + len;
        while (end < limit && b[end] != 0) end++;
        return Encoding.UTF8.GetString(b, offset, end - offset);
    }

    private static ulong ParseOctal(byte[] b, int offset, int len)
    {
        ulong value = 0;
        for (int i = 0; i < len; i++)
        {
            byte c = b[offset + i];
            if (c == 0 || c == (byte)' ') continue;
            if (c < (byte)'0' || c > (byte)'7') continue;
            value = value * 8 + (ulong)(c - (byte)'0');
        }
        return value;
    }
}
