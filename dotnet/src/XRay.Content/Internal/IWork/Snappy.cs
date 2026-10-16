// Raw (frameless) Snappy block decoder, as used by Apple's IWA chunk framing.
//
// Rust reaches this through the `snap` crate (`snap::raw::Decoder` /
// `snap::raw::decompress_len`), which the port has no equivalent of; the format is
// small enough to decode directly. Only decompression is needed — nothing here writes
// Snappy.

namespace XRay.Content.Internal.IWork;

internal static class Snappy
{
    /// <summary>Thrown for a malformed Snappy block; mirrors `snap`'s decode errors.</summary>
    internal sealed class FormatException(string message) : System.Exception(message);

    /// <summary>The uncompressed length recorded in the block's varint preamble.</summary>
    public static int DecompressedLength(ReadOnlySpan<byte> source)
    {
        var (length, _) = ReadPreamble(source);
        return length;
    }

    public static byte[] Decompress(ReadOnlySpan<byte> source)
    {
        var (length, headerLength) = ReadPreamble(source);
        var output = new byte[length];
        int written = 0;
        int position = headerLength;

        while (position < source.Length)
        {
            Cancel.ThrowIfRequested();
            byte tag = source[position++];
            if ((tag & 0x03) == 0)
            {
                position += CopyLiteral(source, position, tag, output, ref written);
            }
            else
            {
                var (copyLength, offset) = ReadCopy(source, ref position, tag);
                CopyReference(output, ref written, copyLength, offset);
            }
        }

        if (written != length)
            throw new FormatException($"Snappy block declared {length} bytes but produced {written}");
        return output;
    }

    private static (int Length, int HeaderLength) ReadPreamble(ReadOnlySpan<byte> source)
    {
        long value = 0;
        int shift = 0;
        int position = 0;
        while (true)
        {
            if (position >= source.Length || shift > 28)
                throw new FormatException("Snappy block has no valid length preamble");
            byte b = source[position++];
            value |= (long)(b & 0x7F) << shift;
            if ((b & 0x80) == 0) break;
            shift += 7;
        }
        if (value > int.MaxValue)
            throw new FormatException("Snappy block length does not fit this platform");
        return ((int)value, position);
    }

    /// <summary>Returns the number of source bytes consumed after the tag byte.</summary>
    private static int CopyLiteral(ReadOnlySpan<byte> source, int position, byte tag, byte[] output, ref int written)
    {
        int length = tag >> 2;
        int extra = 0;
        if (length >= 60)
        {
            extra = length - 59;
            if (position + extra > source.Length)
                throw new FormatException("Snappy literal length is truncated");
            long parsed = 0;
            for (int i = 0; i < extra; i++) parsed |= (long)source[position + i] << (8 * i);
            length = checked((int)parsed);
        }
        length += 1;

        int start = position + extra;
        if (start + length > source.Length)
            throw new FormatException("Snappy literal runs past the end of the block");
        if (written + length > output.Length)
            throw new FormatException("Snappy literal overruns the declared output length");
        source.Slice(start, length).CopyTo(output.AsSpan(written));
        written += length;
        return extra + length;
    }

    private static (int Length, int Offset) ReadCopy(ReadOnlySpan<byte> source, ref int position, byte tag)
    {
        switch (tag & 0x03)
        {
            case 1:
                Require(source, position, 1);
                return (4 + ((tag >> 2) & 0x07), ((tag >> 5) << 8) | source[position++]);
            case 2:
            {
                Require(source, position, 2);
                int offset = source[position] | (source[position + 1] << 8);
                position += 2;
                return ((tag >> 2) + 1, offset);
            }
            default:
            {
                Require(source, position, 4);
                long offset = (uint)(source[position] | (source[position + 1] << 8)
                    | (source[position + 2] << 16) | (source[position + 3] << 24));
                position += 4;
                if (offset > int.MaxValue) throw new FormatException("Snappy copy offset is out of range");
                return ((tag >> 2) + 1, (int)offset);
            }
        }
    }

    /// <summary>
    /// Byte-at-a-time on purpose: a copy may overlap itself (offset smaller than the copy
    /// length), which is how Snappy encodes runs, so a block move would read bytes it has
    /// not written yet.
    /// </summary>
    private static void CopyReference(byte[] output, ref int written, int length, int offset)
    {
        if (offset <= 0 || offset > written)
            throw new FormatException("Snappy copy points outside the decoded output");
        if (written + length > output.Length)
            throw new FormatException("Snappy copy overruns the declared output length");
        int start = written - offset;
        for (int i = 0; i < length; i++) output[written + i] = output[start + i];
        written += length;
    }

    private static void Require(ReadOnlySpan<byte> source, int position, int count)
    {
        if (position + count > source.Length) throw new FormatException("Snappy copy tag is truncated");
    }
}
