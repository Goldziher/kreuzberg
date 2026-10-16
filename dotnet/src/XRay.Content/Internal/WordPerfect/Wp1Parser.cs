namespace XRay.Content.Internal.WordPerfect;

/// <summary>
/// Macintosh WordPerfect 1.x parser, reimplemented from libwpd's <c>WP1Parser</c>.
/// </summary>
/// <remarks>
/// <para>
/// Like WordPerfect 4.2 for DOS, this format has no header, so it is identified structurally. The
/// two are easy to confuse — both classify bytes the same way and both delimit multi-byte groups
/// with a repeated opening byte — and the difference is in the variable-length form: 1.x writes a
/// big-endian 32-bit length before and after the payload, where 4.2 simply scans for the closing
/// byte. Checking that length pair is what tells them apart.
/// </para>
/// <para>
/// Being a Macintosh format, its extended characters index Mac Roman rather than a WordPerfect
/// character set.
/// </para>
/// </remarks>
internal static class Wp1Parser
{
    /// <summary>Size of each function group from 0xC0 to 0xFE, or -1 for a variable-length one.</summary>
    private static readonly int[] FunctionGroupSize =
    {
        10, 4, 4, 7, 7, 6, 4, 6, 8, -1, 4, 6, 6, 3, 6, 3,          // 0xC0
        6, -1, -1, 4, 4, 4, 6, -1, 4, 4, 4, 4, -1, 24, 6, -1,      // 0xD0
        4, 3, -1, 150, 6, 23, 11, 3, 3, -1, -1, 32, 5, -1, 44, 18, // 0xE0
        6, 106, -1, 196, 4, -1, 5, 4, 4, 8, -1, 4, -1, -1, -1,     // 0xF0
    };

    private const byte HeaderFooterGroup = 0xD1;
    private const byte ExtendedCharacterGroup = 0xE1;

    /// <summary>
    /// Whether this looks like a Macintosh WordPerfect 1.x document.
    /// </summary>
    /// <remarks>
    /// The variable-length groups are what carry the evidence: their payload length is written
    /// twice, once before and once after, and both copies have to agree and be followed by the
    /// group's own byte. As with 4.2, at least one function group is required so a plain text file
    /// is not claimed.
    /// </remarks>
    public static bool LooksLikeWp1(byte[] bytes)
    {
        var reader = new WpdReader(bytes);
        int functionGroups = 0;

        while (!reader.AtEnd)
        {
            Cancel.ThrowIfRequested();
            byte value = reader.ReadU8();

            if (value <= 0x7F) continue;
            if (value <= 0xBF) { functionGroups++; continue; }
            if (value == 0xFF) return false;

            int size = FunctionGroupSize[value - 0xC0];
            if (size == -1)
            {
                uint length = reader.ReadU32(bigEndian: true);
                if (length == 0 || length > int.MaxValue / 2) return false;
                reader.Skip((int)length);
                uint closingLength = reader.ReadU32(bigEndian: true);
                if (length != closingLength) return false;
                if (reader.AtEnd || reader.ReadU8() != value) return false;
            }
            else
            {
                int expected = reader.Position + size - 2;
                if (expected >= reader.Length) return false;
                reader.Seek(expected);
                if (reader.ReadU8() != value) return false;
            }
            functionGroups++;
        }

        return functionGroups > 0;
    }

    /// <summary>Parse a Macintosh WordPerfect 1.x document into an event stream.</summary>
    public static WpdDocument Parse(byte[] bytes)
    {
        var document = new WpdDocument();
        var sink = new WpdEventSink(document.Events);
        ParseBody(new WpdReader(bytes), sink);
        sink.Finish();
        return document;
    }

    private static void ParseBody(WpdReader reader, WpdEventSink sink, int depth = 0)
    {
        if (depth > 8) return;

        while (!reader.AtEnd)
        {
            Cancel.ThrowIfRequested();
            byte value = reader.ReadU8();

            if (value < 0x20)
            {
                switch (value)
                {
                    case 0x09: sink.Tab(); break;
                    case 0x0A: sink.ParagraphEnd(); break;   // hard return
                    case 0x0B: sink.ParagraphEnd(); break;   // soft page break
                    case 0x0C: sink.ParagraphEnd(); break;   // hard page break
                    case 0x0D: sink.Character(' '); break;   // soft return
                }
            }
            else if (value <= 0x7F)
            {
                sink.Character((char)value);
            }
            else if (value <= 0xBF)
            {
                ParseSingleByteFunction(sink, value);
            }
            else if (value <= 0xFE)
            {
                ParseFunctionGroup(reader, sink, value, depth);
            }
        }
    }

    private static void ParseSingleByteFunction(WpdEventSink sink, byte function)
    {
        switch (function)
        {
            case 0x92: sink.AttributeChange(WpdEventKind.StrikethroughStart, WpdEventKind.StrikethroughEnd, on: true); break;
            case 0x93: sink.AttributeChange(WpdEventKind.StrikethroughStart, WpdEventKind.StrikethroughEnd, on: false); break;
            case 0x94: sink.AttributeChange(WpdEventKind.UnderlineStart, WpdEventKind.UnderlineEnd, on: true); break;
            case 0x95: sink.AttributeChange(WpdEventKind.UnderlineStart, WpdEventKind.UnderlineEnd, on: false); break;
            case 0x9C: sink.AttributeChange(WpdEventKind.BoldStart, WpdEventKind.BoldEnd, on: false); break;
            case 0x9D: sink.AttributeChange(WpdEventKind.BoldStart, WpdEventKind.BoldEnd, on: true); break;
            case 0xB2: sink.AttributeChange(WpdEventKind.ItalicStart, WpdEventKind.ItalicEnd, on: true); break;
            case 0xB3: sink.AttributeChange(WpdEventKind.ItalicStart, WpdEventKind.ItalicEnd, on: false); break;
            // Superscript and subscript are the one place this format is not symmetric: the
            // opening codes sit above the closing ones rather than beside them.
            case 0xBC: sink.AttributeChange(WpdEventKind.SuperscriptStart, WpdEventKind.SuperscriptEnd, on: true); break;
            case 0xB9: sink.AttributeChange(WpdEventKind.SuperscriptStart, WpdEventKind.SuperscriptEnd, on: false); break;
            case 0xBD: sink.AttributeChange(WpdEventKind.SubscriptStart, WpdEventKind.SubscriptEnd, on: true); break;
            case 0xB8: sink.AttributeChange(WpdEventKind.SubscriptStart, WpdEventKind.SubscriptEnd, on: false); break;
            // Redline, shadow and outline have no counterpart in the event stream.
        }
    }

    private static void ParseFunctionGroup(WpdReader reader, WpdEventSink sink, byte group, int depth)
    {
        int start = reader.Position;
        int size = FunctionGroupSize[group - 0xC0];

        switch (group)
        {
            case ExtendedCharacterGroup:
            {
                // Anything at or below 0x20 is a space; the rest indexes Mac Roman.
                byte character = reader.ReadU8();
                sink.Text(character <= 0x20 ? " " : WpCharacterMap.MacRomanCharacter(character));
                break;
            }

            case HeaderFooterGroup when size == -1:
            {
                // A variable-length group's payload is bracketed by its own length.
                reader.Seek(start);
                uint length = reader.ReadU32(bigEndian: true);
                int bodyStart = reader.Position;
                if (length > 0 && length < int.MaxValue / 2)
                {
                    sink.AttributeChange(WpdEventKind.HeaderStart, WpdEventKind.HeaderEnd, on: true);
                    ParseBody(new WpdReader(reader.Slice(bodyStart, (int)length)), sink, depth + 1);
                    sink.AttributeChange(WpdEventKind.HeaderStart, WpdEventKind.HeaderEnd, on: false);
                }
                break;
            }
        }

        SkipToGroupEnd(reader, group, start, size);
    }

    /// <summary>Position the cursor just past a group's closing byte.</summary>
    private static void SkipToGroupEnd(WpdReader reader, byte group, int start, int size)
    {
        if (size != -1)
        {
            reader.Seek(start + size - 1);
            return;
        }

        // Variable length: the payload is framed by a repeated big-endian length.
        reader.Seek(start);
        uint length = reader.ReadU32(bigEndian: true);
        if (length == 0 || length > int.MaxValue / 2) { reader.Seek(start); return; }
        reader.Skip((int)length);
        reader.Skip(4);   // the closing length
        reader.Skip(1);   // the closing group byte
    }
}
