namespace XRay.Content.Internal.Heif;

/// <summary>What a HEIF-family container says about its primary image.</summary>
internal sealed record HeifImageInfo(uint Width, uint Height, byte[]? Exif);

/// <summary>
/// Reads the metadata of a HEIF-family container — HEIC, HEIF, AVIF.
/// </summary>
/// <remarks>
/// <para>
/// These files are ISO base media containers: a box tree describing items, their properties
/// and where their bytes live, with the coded picture itself sitting in <c>mdat</c> as an HEVC
/// or AV1 bitstream. Everything this reader needs — the primary image's dimensions and the EXIF
/// block — is in that description, so no picture is ever decoded.
/// </para>
/// <para>
/// That is the whole scope, and it is a deliberate one: decoding the picture would mean an HEVC
/// intra decoder for HEIC and an AV1 intra decoder for AVIF, each far larger than everything
/// else in this port put together. What is lost is pixels, and this extractor reports metadata.
/// </para>
/// </remarks>
internal static class HeifContainer
{
    /// <summary>The brands that mark a file as one of the HEIF family.</summary>
    /// <remarks>
    /// A brand is checked against both the major brand and the compatible-brand list, because a
    /// file written as <c>mif1</c> can still name <c>heic</c> among the brands it is compatible
    /// with, and it is the same container either way.
    /// </remarks>
    private static readonly string[] HeifBrands =
    {
        "heic", "heix", "hevc", "hevx", "heim", "heis", "hevm", "hevs",
        "mif1", "msf1", "avif", "avis", "avcs", "avci",
    };

    /// <summary>Whether these bytes open with a HEIF-family <c>ftyp</c> box.</summary>
    public static bool IsHeifContainer(ReadOnlySpan<byte> bytes)
    {
        if (bytes.Length < 12) return false;
        if (bytes[4] != 'f' || bytes[5] != 't' || bytes[6] != 'y' || bytes[7] != 'p') return false;

        uint size = ReadU32(bytes, 0);
        if (size < 16 || size > bytes.Length) size = (uint)Math.Min(bytes.Length, 64);

        // The major brand, then every compatible brand after the minor version.
        if (IsHeifBrand(bytes, 8)) return true;
        for (int offset = 16; offset + 4 <= size; offset += 4)
            if (IsHeifBrand(bytes, offset)) return true;

        return false;
    }

    private static bool IsHeifBrand(ReadOnlySpan<byte> bytes, int offset)
    {
        if (offset + 4 > bytes.Length) return false;
        string brand = BoxType(bytes, offset);
        foreach (var known in HeifBrands)
            if (brand == known) return true;
        return false;
    }

    /// <summary>Read the primary image's dimensions and EXIF, or <c>null</c> if they are absent.</summary>
    public static HeifImageInfo? TryRead(byte[] bytes)
    {
        if (FindBox(bytes, 0, bytes.Length, "meta") is not { } meta) return null;
        var (metaStart, metaEnd) = meta;
        if (metaEnd - metaStart < 4) return null;

        // meta is a full box: a version and flags precede its children.
        int childrenStart = metaStart + 4;

        var items = ReadItemTypes(bytes, childrenStart, metaEnd);
        var locations = ReadItemLocations(bytes, childrenStart, metaEnd);
        uint primaryId = ReadPrimaryItemId(bytes, childrenStart, metaEnd);
        if (primaryId == 0 && items.Count > 0) primaryId = items.Keys.Min();

        var size = ReadPrimarySize(bytes, childrenStart, metaEnd, primaryId, items, locations);
        if (size is null) return null;

        return new HeifImageInfo(size.Value.Width, size.Value.Height,
            ReadExif(bytes, items, locations));
    }

    // ------------------------------------------------------------------ Dimensions

    private static (uint Width, uint Height)? ReadPrimarySize(
        byte[] bytes, int metaStart, int metaEnd, uint primaryId,
        Dictionary<uint, string> items, Dictionary<uint, (long Offset, long Length)> locations)
    {
        // A grid item is not coded at all: it is a description of how the tiles that make it up
        // are laid out, and its own header carries the assembled size.
        if (items.TryGetValue(primaryId, out string? type) && type == "grid"
            && locations.TryGetValue(primaryId, out var grid))
        {
            var assembled = ReadGridSize(bytes, grid.Offset, grid.Length);
            if (assembled is not null) return assembled;
        }

        var properties = ReadPropertiesFor(bytes, metaStart, metaEnd, primaryId);
        if (properties.Count == 0) return null;

        uint width = 0, height = 0;
        foreach (var (boxType, start, end) in properties)
        {
            if (boxType != "ispe" || end - start < 12) continue;
            width = ReadU32(bytes, start + 4);
            height = ReadU32(bytes, start + 8);
            break;
        }
        if (width == 0 || height == 0) return null;

        // A clean aperture crops the coded picture down to what the author meant to show, and a
        // rotation turns it. Both are properties rather than pixels, and a decoder applies them
        // before reporting a size — so reporting the raw extent would disagree with every viewer.
        foreach (var (boxType, start, end) in properties)
        {
            switch (boxType)
            {
                case "clap" when end - start >= 32:
                {
                    var cropped = ReadCleanAperture(bytes, start);
                    if (cropped is not null) (width, height) = cropped.Value;
                    break;
                }

                case "irot" when end - start >= 1:
                    if ((bytes[start] & 0x03) % 2 == 1) (width, height) = (height, width);
                    break;
            }
        }

        return (width, height);
    }

    /// <summary>The visible extent a clean-aperture property selects, as whole pixels.</summary>
    private static (uint Width, uint Height)? ReadCleanAperture(byte[] bytes, int start)
    {
        int widthNumerator = (int)ReadU32(bytes, start);
        int widthDenominator = (int)ReadU32(bytes, start + 4);
        int heightNumerator = (int)ReadU32(bytes, start + 8);
        int heightDenominator = (int)ReadU32(bytes, start + 12);

        if (widthDenominator == 0 || heightDenominator == 0) return null;
        if (widthNumerator <= 0 || heightNumerator <= 0) return null;

        uint width = (uint)Math.Round((double)widthNumerator / widthDenominator,
            MidpointRounding.AwayFromZero);
        uint height = (uint)Math.Round((double)heightNumerator / heightDenominator,
            MidpointRounding.AwayFromZero);
        return width == 0 || height == 0 ? null : (width, height);
    }

    /// <summary>The assembled size a grid item's own header declares.</summary>
    private static (uint Width, uint Height)? ReadGridSize(byte[] bytes, long offset, long length)
    {
        if (offset < 0 || length < 8 || offset + length > bytes.Length) return null;

        int p = (int)offset;
        byte flags = bytes[p + 1];
        bool wide = (flags & 0x01) != 0;                 // four-byte fields rather than two
        int need = wide ? 12 : 8;
        if (length < need) return null;

        p += 4;                                          // version, flags, rows, columns
        uint width = wide ? ReadU32(bytes, p) : ReadU16(bytes, p);
        uint height = wide ? ReadU32(bytes, p + 4) : ReadU16(bytes, p + 2);
        return width == 0 || height == 0 ? null : (width, height);
    }

    // ------------------------------------------------------------------ EXIF

    /// <summary>The TIFF block of the EXIF item, or <c>null</c> when the file carries none.</summary>
    /// <remarks>
    /// The item's payload begins with an offset to the TIFF header rather than the header itself,
    /// so that a writer can put its own preamble in front. Handing the payload straight to an
    /// EXIF reader would have it parsing that offset as a byte-order mark.
    /// </remarks>
    private static byte[]? ReadExif(
        byte[] bytes, Dictionary<uint, string> items,
        Dictionary<uint, (long Offset, long Length)> locations)
    {
        foreach (var (id, type) in items)
        {
            if (type != "Exif") continue;
            if (!locations.TryGetValue(id, out var where)) continue;
            if (where.Offset < 0 || where.Length < 4) continue;
            if (where.Offset + where.Length > bytes.Length) continue;

            long headerOffset = ReadU32(bytes, (int)where.Offset);
            long start = where.Offset + 4 + headerOffset;
            long end = where.Offset + where.Length;
            if (headerOffset < 0 || start >= end) continue;

            var exif = new byte[end - start];
            Array.Copy(bytes, start, exif, 0, exif.Length);
            return exif;
        }

        return null;
    }

    // ------------------------------------------------------------------ Item description

    private static uint ReadPrimaryItemId(byte[] bytes, int start, int end)
    {
        if (FindBox(bytes, start, end, "pitm") is not { } pitm) return 0;
        var (boxStart, boxEnd) = pitm;
        if (boxEnd - boxStart < 6) return 0;
        return bytes[boxStart] == 0 ? ReadU16(bytes, boxStart + 4) : ReadU32(bytes, boxStart + 4);
    }

    /// <summary>Every item's four-character type, by item ID.</summary>
    private static Dictionary<uint, string> ReadItemTypes(byte[] bytes, int start, int end)
    {
        var items = new Dictionary<uint, string>();
        if (FindBox(bytes, start, end, "iinf") is not { } info) return items;
        var (infoStart, infoEnd) = info;
        if (infoEnd - infoStart < 6) return items;

        byte version = bytes[infoStart];
        int entriesStart = version == 0 ? infoStart + 6 : infoStart + 8;

        foreach (var (boxType, entryStart, entryEnd) in Children(bytes, entriesStart, infoEnd))
        {
            if (boxType != "infe" || entryEnd - entryStart < 8) continue;

            byte entryVersion = bytes[entryStart];
            if (entryVersion < 2) continue;              // pre-HEIF layout, no item type at all

            uint id;
            int p;
            if (entryVersion == 2)
            {
                id = ReadU16(bytes, entryStart + 4);
                p = entryStart + 6;
            }
            else
            {
                id = ReadU32(bytes, entryStart + 4);
                p = entryStart + 8;
            }

            if (p + 6 > entryEnd) continue;
            items[id] = BoxType(bytes, p + 2);
        }

        return items;
    }

    /// <summary>Where each item's bytes are, by item ID.</summary>
    /// <remarks>
    /// Only items stored in the file itself are recorded: an item can name a URL or another
    /// item as its source, and neither is a range of these bytes.
    /// </remarks>
    private static Dictionary<uint, (long Offset, long Length)> ReadItemLocations(
        byte[] bytes, int start, int end)
    {
        var locations = new Dictionary<uint, (long, long)>();
        if (FindBox(bytes, start, end, "iloc") is not { } loc) return locations;
        var (locStart, locEnd) = loc;
        if (locEnd - locStart < 8) return locations;

        byte version = bytes[locStart];
        int p = locStart + 4;

        int offsetSize = bytes[p] >> 4;
        int lengthSize = bytes[p] & 0x0F;
        int baseOffsetSize = bytes[p + 1] >> 4;
        int indexSize = version >= 1 ? bytes[p + 1] & 0x0F : 0;
        p += 2;

        int count;
        if (version < 2)
        {
            count = (int)ReadU16(bytes, p);
            p += 2;
        }
        else
        {
            count = (int)ReadU32(bytes, p);
            p += 4;
        }

        for (int i = 0; i < count && p < locEnd; i++)
        {
            uint id;
            if (version < 2) { id = ReadU16(bytes, p); p += 2; }
            else { id = ReadU32(bytes, p); p += 4; }

            int constructionMethod = 0;
            if (version >= 1)
            {
                constructionMethod = bytes[p + 1] & 0x0F;
                p += 2;
            }

            p += 2;                                      // data reference index
            long baseOffset = ReadVariable(bytes, ref p, baseOffsetSize);

            int extents = (int)ReadU16(bytes, p);
            p += 2;

            long firstOffset = 0, totalLength = 0;
            for (int e = 0; e < extents && p < locEnd; e++)
            {
                if (indexSize > 0) ReadVariable(bytes, ref p, indexSize);
                long extentOffset = ReadVariable(bytes, ref p, offsetSize);
                long extentLength = ReadVariable(bytes, ref p, lengthSize);
                if (e == 0) firstOffset = baseOffset + extentOffset;
                totalLength += extentLength;
            }

            // Construction method 0 is a range of the file; 1 points into an idat box and 2 at
            // another item, neither of which this reader follows.
            if (constructionMethod == 0 && extents > 0)
                locations[id] = (firstOffset, totalLength);
        }

        return locations;
    }

    /// <summary>The property boxes associated with one item, in the order the file lists them.</summary>
    private static List<(string Type, int Start, int End)> ReadPropertiesFor(
        byte[] bytes, int start, int end, uint itemId)
    {
        var found = new List<(string, int, int)>();
        if (FindBox(bytes, start, end, "iprp") is not { } properties) return found;
        var (propStart, propEnd) = properties;

        if (FindBox(bytes, propStart, propEnd, "ipco") is not { } ipco) return found;
        var container = Children(bytes, ipco.Start, ipco.End).ToList();

        if (FindBox(bytes, propStart, propEnd, "ipma") is not { } ipma) return found;
        var (assocStart, assocEnd) = ipma;
        if (assocEnd - assocStart < 8) return found;

        byte version = bytes[assocStart];
        uint flags = ReadU24(bytes, assocStart + 1);
        int p = assocStart + 4;
        int entries = (int)ReadU32(bytes, p);
        p += 4;

        for (int i = 0; i < entries && p < assocEnd; i++)
        {
            uint id;
            if (version < 1) { id = ReadU16(bytes, p); p += 2; }
            else { id = ReadU32(bytes, p); p += 4; }

            int associations = bytes[p];
            p++;

            for (int a = 0; a < associations && p < assocEnd; a++)
            {
                int index;
                if ((flags & 0x01) != 0)
                {
                    index = (int)(ReadU16(bytes, p) & 0x7FFF);
                    p += 2;
                }
                else
                {
                    index = bytes[p] & 0x7F;
                    p++;
                }

                // Property indices are one-based, and zero means no property at all.
                if (id == itemId && index >= 1 && index <= container.Count)
                    found.Add(container[index - 1]);
            }
        }

        return found;
    }

    // ------------------------------------------------------------------ Box walking

    /// <summary>The first box of this type among the children of the given range.</summary>
    private static (int Start, int End)? FindBox(byte[] bytes, int start, int end, string type)
    {
        foreach (var (boxType, boxStart, boxEnd) in Children(bytes, start, end))
            if (boxType == type)
                return (boxStart, boxEnd);
        return null;
    }

    /// <summary>Walk the boxes directly inside a range, yielding each one's payload.</summary>
    /// <remarks>
    /// A malformed size ends the walk rather than throwing: these files come off cameras and
    /// through transcoders, and a truncated tail should cost the boxes after it, not the file.
    /// </remarks>
    private static IEnumerable<(string Type, int Start, int End)> Children(
        byte[] bytes, int start, int end)
    {
        end = Math.Min(end, bytes.Length);
        int p = start;

        while (p + 8 <= end)
        {
            Cancel.ThrowIfRequested();
            long size = ReadU32(bytes, p);
            string type = BoxType(bytes, p + 4);
            int header = 8;

            if (size == 1)
            {
                if (p + 16 > end) yield break;
                size = (long)ReadU64(bytes, p + 8);
                header = 16;
            }
            else if (size == 0)
            {
                size = end - p;                          // a box that runs to the end of its parent
            }

            if (size < header || p + size > end) yield break;

            yield return (type, p + header, (int)(p + size));
            p += (int)size;
        }
    }

    private static string BoxType(ReadOnlySpan<byte> bytes, int offset)
    {
        if (offset + 4 > bytes.Length) return "";
        Span<char> chars = stackalloc char[4];
        for (int i = 0; i < 4; i++) chars[i] = (char)bytes[offset + i];
        return new string(chars);
    }

    private static long ReadVariable(byte[] bytes, ref int p, int size)
    {
        long value = 0;
        for (int i = 0; i < size; i++)
        {
            value = (value << 8) | (p < bytes.Length ? bytes[p] : (byte)0);
            p++;
        }
        return value;
    }

    private static uint ReadU16(ReadOnlySpan<byte> bytes, int offset) =>
        offset + 2 <= bytes.Length ? (uint)((bytes[offset] << 8) | bytes[offset + 1]) : 0;

    private static uint ReadU24(ReadOnlySpan<byte> bytes, int offset) =>
        offset + 3 <= bytes.Length
            ? (uint)((bytes[offset] << 16) | (bytes[offset + 1] << 8) | bytes[offset + 2])
            : 0;

    private static uint ReadU32(ReadOnlySpan<byte> bytes, int offset) =>
        offset + 4 <= bytes.Length
            ? ((uint)bytes[offset] << 24) | ((uint)bytes[offset + 1] << 16)
              | ((uint)bytes[offset + 2] << 8) | bytes[offset + 3]
            : 0;

    private static ulong ReadU64(ReadOnlySpan<byte> bytes, int offset)
    {
        ulong value = 0;
        for (int i = 0; i < 8 && offset + i < bytes.Length; i++)
            value = (value << 8) | bytes[offset + i];
        return value;
    }
}
