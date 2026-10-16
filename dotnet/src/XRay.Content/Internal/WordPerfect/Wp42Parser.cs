namespace XRay.Content.Internal.WordPerfect;

/// <summary>
/// WordPerfect 4.2 parser, reimplemented from libwpd's <c>WP42Parser</c>.
/// </summary>
/// <remarks>
/// <para>
/// The 4.2 document is the whole file: there is no header and no index, just a byte stream where
/// the value of each byte says what it is. Under 0x20 is a control character, 0x20 to 0x7F is
/// literal ASCII, 0x80 to 0xBF is a single-byte function, and 0xC0 upward opens a multi-byte
/// function group.
/// </para>
/// <para>
/// A multi-byte group is delimited by its own opening byte repeated at the end, and
/// <see cref="FunctionGroupSize"/> says how long each is — or that it is variable, in which case
/// the closing byte is what ends it. That self-delimiting structure is also what makes the format
/// recognisable without a header.
/// </para>
/// </remarks>
internal static class Wp42Parser
{
    /// <summary>
    /// Size of each function group from 0xC0 to 0xFE, or -1 for a variable-length group.
    /// </summary>
    /// <remarks>
    /// Straight from libwpd's table, including its note that the documented size of 0xEB is wrong.
    /// </remarks>
    private static readonly int[] FunctionGroupSize =
    {
        6, 4, 3, 5, 5, 6, 4, 6, 8, 42, 3, 6, 4, 3, 4, 3,          // 0xC0
        6, -1, -1, 4, 4, 4, 6, -1, 4, 4, 4, 4, -1, 24, 4, -1,     // 0xD0
        4, 3, -1, 150, 6, 23, 11, 3, 3, -1, -1, -1, 4, -1, 44, 18, // 0xE0
        6, 106, -1, 100, 4, -1, 5, -1, -1, -1, -1, -1, -1, -1, -1, // 0xF0
    };

    private const byte ExtendedCharacterGroup = 0xE1;
    private const byte HeaderFooterGroup = 0xD1;

    /// <summary>
    /// Whether this looks like a WordPerfect 4.2 document.
    /// </summary>
    /// <remarks>
    /// Port of libwpd's <c>WP42Heuristics</c>: walk the whole file and require that every
    /// multi-byte group closes with its own opening byte at exactly the right place. A plain text
    /// file passes the byte-range checks but contains no function group at all, so at least one is
    /// required — otherwise this would claim every text file.
    /// </remarks>
    public static bool LooksLikeWp42(byte[] bytes)
    {
        var reader = new WpdReader(bytes);
        int functionGroups = 0;

        while (!reader.AtEnd)
        {
            Cancel.ThrowIfRequested();
            byte value = reader.ReadU8();

            if (value <= 0x7F) continue;            // control characters and literal ASCII
            if (value <= 0xBF) { functionGroups++; continue; }   // single-byte functions
            if (value == 0xFF) return false;        // never a group opener

            int size = FunctionGroupSize[value - 0xC0];
            if (size == -1)
            {
                int closing = -1;
                while (!reader.AtEnd)
                {
                    Cancel.ThrowIfRequested();
                    closing = reader.ReadU8();
                    if (closing == value) break;
                }
                if (closing != value) return false;
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

    /// <summary>Parse a WordPerfect 4.2 document into an event stream.</summary>
    public static WpdDocument Parse(byte[] bytes)
    {
        var document = new WpdDocument();
        var sink = new WpdEventSink(document.Events);
        ParseBody(new WpdReader(bytes), sink);
        sink.Finish();
        return document;
    }

    /// <summary>Walk a document body, which a header or footer group re-enters recursively.</summary>
    private static void ParseBody(WpdReader reader, WpdEventSink sink)
    {
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
                    // Anything else here is undocumented and libwpd ignores it too.
                }
            }
            else if (value <= 0x7F)
            {
                sink.Character((char)value);
            }
            else if (value <= 0xBF)
            {
                switch (value)
                {
                    case 0x92: sink.AttributeChange(WpdEventKind.StrikethroughStart, WpdEventKind.StrikethroughEnd, on: true); break;
                    case 0x93: sink.AttributeChange(WpdEventKind.StrikethroughStart, WpdEventKind.StrikethroughEnd, on: false); break;
                    case 0x94: sink.AttributeChange(WpdEventKind.UnderlineStart, WpdEventKind.UnderlineEnd, on: true); break;
                    case 0x95: sink.AttributeChange(WpdEventKind.UnderlineStart, WpdEventKind.UnderlineEnd, on: false); break;
                    case 0x9C: sink.AttributeChange(WpdEventKind.BoldStart, WpdEventKind.BoldEnd, on: false); break;
                    case 0x9D: sink.AttributeChange(WpdEventKind.BoldStart, WpdEventKind.BoldEnd, on: true); break;
                    case 0xB2: sink.AttributeChange(WpdEventKind.ItalicStart, WpdEventKind.ItalicEnd, on: true); break;
                    case 0xB3: sink.AttributeChange(WpdEventKind.ItalicStart, WpdEventKind.ItalicEnd, on: false); break;
                    // Redline and shadow have no counterpart in the event stream.
                }
            }
            else if (value <= 0xFE)
            {
                ParseFunctionGroup(reader, sink, value);
            }
            // 0xFF only ever terminates a variable-length group's payload.
        }
    }

    /// <summary>Read one multi-byte function group and act on the few that carry content.</summary>
    private static void ParseFunctionGroup(WpdReader reader, WpdEventSink sink, byte group)
    {
        int start = reader.Position;

        switch (group)
        {
            case ExtendedCharacterGroup:
                sink.Text(WpCharacterMap.Wp42Extended(reader.ReadU8()));
                break;

            case HeaderFooterGroup:
                ParseHeaderFooter(reader, sink, start);
                break;
        }

        SkipToGroupEnd(reader, group, start);
    }

    /// <summary>
    /// A header or footer group carries its text inline, as a sub-document ending at 0xFF.
    /// </summary>
    /// <remarks>
    /// The four leading bytes are the definition, and the payload runs to the first 0xFF. Emitting
    /// it as a header keeps running text out of the body flow, which is where libwpd puts it.
    /// </remarks>
    private static void ParseHeaderFooter(WpdReader reader, WpdEventSink sink, int start)
    {
        reader.Skip(4);
        int bodyStart = reader.Position;

        int end = bodyStart;
        while (end < reader.Length && reader.PeekAt(end) != 0xFF) end++;
        if (end - bodyStart <= 2) return;

        sink.AttributeChange(WpdEventKind.HeaderStart, WpdEventKind.HeaderEnd, on: true);
        ParseBody(new WpdReader(reader.Slice(bodyStart, end - bodyStart)), sink);
        sink.AttributeChange(WpdEventKind.HeaderStart, WpdEventKind.HeaderEnd, on: false);
    }

    /// <summary>Position the cursor just past a group's closing byte.</summary>
    private static void SkipToGroupEnd(WpdReader reader, byte group, int start)
    {
        int size = FunctionGroupSize[group - 0xC0];
        if (size != -1)
        {
            // The recorded size counts the opening and closing bytes.
            reader.Seek(start + size - 1);
            return;
        }

        reader.Seek(start);
        while (!reader.AtEnd && reader.ReadU8() != group) { }
    }
}
