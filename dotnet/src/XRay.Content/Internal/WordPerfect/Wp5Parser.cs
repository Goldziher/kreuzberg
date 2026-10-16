namespace XRay.Content.Internal.WordPerfect;

/// <summary>
/// WordPerfect 5.0 and 5.1 parser, reimplemented from libwpd's <c>WP5Parser</c>.
/// </summary>
/// <remarks>
/// <para>
/// The 5.x body starts at the header's document offset and is a byte stream classified by value:
/// control characters below 0x20, literal ASCII to 0x7E, single-byte functions from 0x80 to 0xBF,
/// fixed-length groups from 0xC0 to 0xCF, and variable-length groups from 0xD0 up.
/// </para>
/// <para>
/// Both group kinds are self-checking, and that is load-bearing rather than decorative: a fixed
/// group repeats its own opening byte at a known offset, and a variable one repeats its size, its
/// subgroup and its opening byte at the end. libwpd validates that before trusting a group, and
/// skips the byte when it fails, which is what keeps a corrupt document from derailing the whole
/// parse. Doing the same here is why the malformed CVE samples fail cleanly instead of producing
/// nonsense.
/// </para>
/// </remarks>
internal static class Wp5Parser
{
    /// <summary>Size of each fixed-length group from 0xC0 to 0xCF, counting both delimiters.</summary>
    private static readonly int[] FixedLengthGroupSize =
    {
        4, 9, 11, 3, 3, 5, 6, 7, 4, 5, 6, 6, 8, 10, 10, 12,
    };

    private const byte TopExtendedCharacter = 0xC0;
    private const byte TopTabGroup = 0xC1;
    private const byte TopIndentGroup = 0xC2;
    private const byte TopAttributeOn = 0xC3;
    private const byte TopAttributeOff = 0xC4;
    private const byte TopHeaderFooterGroup = 0xD5;
    private const byte TopFootnoteEndnoteGroup = 0xD6;
    private const byte TopDefinitionGroup = 0xD2;
    private const byte TopTableEolGroup = 0xDC;
    private const byte TopTableEopGroup = 0xDD;

    /// <summary>The definition group's subgroup that describes a table's columns.</summary>
    private const byte DefinitionGroupDefineTables = 0x0B;

    // Attribute identifiers, shared by the on and off groups.
    private const byte AttributeSuperscript = 0x05;
    private const byte AttributeSubscript = 0x06;
    private const byte AttributeItalics = 0x08;
    private const byte AttributeBold = 0x0C;
    private const byte AttributeStrikeOut = 0x0D;
    private const byte AttributeUnderline = 0x0E;

    /// <summary>Parse a WordPerfect 5.x document into an event stream.</summary>
    public static WpdDocument Parse(byte[] bytes, WpdHeader header)
    {
        var document = new WpdDocument();
        var sink = new WpdEventSink(document.Events);
        var reader = new WpdReader(bytes);
        reader.Seek((int)header.DocumentOffset);
        ParseBody(reader, sink);
        sink.Finish();
        return document;
    }

    /// <summary>Walk a body, which a sub-document re-enters recursively.</summary>
    private static void ParseBody(WpdReader reader, WpdEventSink sink, int depth = 0)
    {
        // Sub-documents nest — a note inside a header, say — but not deeply. The bound stops a
        // corrupt document whose group offsets point back at themselves from recursing forever.
        if (depth > 8) return;

        while (!reader.AtEnd)
        {
            Cancel.ThrowIfRequested();
            byte value = reader.ReadU8();

            if (value is 0 or 0x7F or 0xFF)
            {
                // Meaningless here, and most often just corruption.
            }
            else if (value <= 0x1F)
            {
                switch (value)
                {
                    case 0x0A: sink.ParagraphEnd(); break;              // hard return
                    case 0x0B: sink.Character(' '); sink.ParagraphEnd(); break;  // soft page break
                    case 0x0C: sink.ParagraphEnd(); break;              // hard page break
                    case 0x0D: sink.Character(' '); break;              // soft return
                }
            }
            else if (value <= 0x7E)
            {
                sink.Character((char)value);
            }
            else if (value <= 0xBF)
            {
                ParseSingleByteFunction(sink, value);
            }
            else if (value <= 0xCF)
            {
                ParseFixedLengthGroup(reader, sink, value);
            }
            else
            {
                ParseVariableLengthGroup(reader, sink, value, depth);
            }
        }
    }

    private static void ParseSingleByteFunction(WpdEventSink sink, byte function)
    {
        switch (function)
        {
            case 0x8C:                       // combination hard return / soft page
            case 0x90:                       // deletable return at end of line
            case 0x99:                       // dormant hard return
                sink.ParagraphEnd();
                break;

            case 0x93:                       // invisible return in line
            case 0x94:                       // invisible return at end of line
            case 0x95:                       // invisible return at end of page
                sink.Character(' ');
                break;

            case 0xA0:                       // hard space
                sink.Character(' ');
                break;

            case 0xA9:                       // hard hyphen in line
            case 0xAA:                       // hard hyphen at end of line
            case 0xAB:                       // hard hyphen at end of page
                sink.Character('-');
                break;

            case 0xAC:                       // soft hyphen in line
            case 0xAD:                       // soft hyphen at end of line
            case 0xAE:                       // soft hyphen at end of page
                sink.Character('­');
                break;
        }
    }

    /// <summary>Whether a fixed-length group closes with its own opening byte where it should.</summary>
    private static bool FixedGroupIsConsistent(WpdReader reader, byte group)
    {
        int start = reader.Position;
        int size = FixedLengthGroupSize[group - 0xC0];
        int closing = start + size - 2;
        return closing < reader.Length && reader.PeekAt(closing) == group;
    }

    private static void ParseFixedLengthGroup(WpdReader reader, WpdEventSink sink, byte group)
    {
        if (!FixedGroupIsConsistent(reader, group)) return;

        int start = reader.Position;
        switch (group)
        {
            case TopExtendedCharacter:
            {
                byte character = reader.ReadU8();
                byte characterSet = reader.ReadU8();
                sink.Text(WpCharacterMap.Wp5Extended(character, characterSet));
                break;
            }

            case TopTabGroup:
            case TopIndentGroup:
                sink.Tab();
                break;

            case TopAttributeOn:
                Attribute(sink, reader.ReadU8(), on: true);
                break;

            case TopAttributeOff:
                Attribute(sink, reader.ReadU8(), on: false);
                break;
        }

        // The recorded size counts the opening byte and the closing one.
        reader.Seek(start + FixedLengthGroupSize[group - 0xC0] - 1);
    }

    private static void Attribute(WpdEventSink sink, byte attribute, bool on)
    {
        switch (attribute)
        {
            case AttributeBold:
                sink.AttributeChange(WpdEventKind.BoldStart, WpdEventKind.BoldEnd, on);
                break;
            case AttributeItalics:
                sink.AttributeChange(WpdEventKind.ItalicStart, WpdEventKind.ItalicEnd, on);
                break;
            case AttributeUnderline:
                sink.AttributeChange(WpdEventKind.UnderlineStart, WpdEventKind.UnderlineEnd, on);
                break;
            case AttributeStrikeOut:
                sink.AttributeChange(WpdEventKind.StrikethroughStart, WpdEventKind.StrikethroughEnd, on);
                break;
            case AttributeSuperscript:
                sink.AttributeChange(WpdEventKind.SuperscriptStart, WpdEventKind.SuperscriptEnd, on);
                break;
            case AttributeSubscript:
                sink.AttributeChange(WpdEventKind.SubscriptStart, WpdEventKind.SubscriptEnd, on);
                break;
            // Size and colour attributes carry no counterpart in the event stream.
        }
    }

    /// <summary>
    /// Read a table definition, which names the table's columns and opens it.
    /// </summary>
    /// <remarks>
    /// 5.x writes the definition twice: the columns as they were, then the columns as they now
    /// are, with the first set's own column count deciding how far to skip to reach the second.
    /// Only the count matters here — the widths and alignments are presentation — but it has to
    /// be the second one, and reaching it means reading the first.
    /// </remarks>
    private static void ParseTableDefinition(WpdReader reader, WpdEventSink sink, int contentStart)
    {
        reader.Seek(contentStart);
        reader.Skip(2);
        int oldColumns = reader.ReadU16();
        reader.Skip(20 + 5 * oldColumns);

        reader.Skip(2);                                 // position flags, then a reserved byte
        int columns = reader.ReadU16();

        // 5.x allows no nested table, so a new definition closes whatever was open.
        sink.EndTable();
        sink.DefineTable();
        for (int i = 0; i < Math.Min(columns, MaxTableColumns); i++) sink.DefineTableColumn();
        sink.StartTable();
    }

    /// <summary>The most columns a 5.x table definition can describe.</summary>
    /// <remarks>libwpd refuses a larger count as corruption rather than trusting it.</remarks>
    private const int MaxTableColumns = 32;

    /// <summary>
    /// Read a cell, whose spans say whether it is a cell at all.
    /// </summary>
    /// <remarks>
    /// The high bit of the column span marks a grid slot the row above already covers. Read as a
    /// span it would claim a cell that is not there and push the rest of the row along by one.
    /// </remarks>
    private static void ParseTableCell(WpdReader reader, WpdEventSink sink, int contentStart)
    {
        reader.Seek(contentStart);
        reader.Skip(2);                                 // cell flags, then the column number
        byte spanning = reader.ReadU8();
        int rowSpan = reader.ReadU8();

        if ((spanning & 0x80) != 0) return;             // bound from above
        sink.InsertCell(spanning & 0x7F, rowSpan);
    }

    /// <summary>
    /// Read a variable-length group's header and check that its trailer agrees with it.
    /// </summary>
    /// <returns>The subgroup and total size, or <c>null</c> when the group is inconsistent.</returns>
    private static (byte SubGroup, int Size)? ReadVariableGroupHeader(WpdReader reader, byte group)
    {
        int start = reader.Position;
        byte subGroup = reader.ReadU8();
        int size = reader.ReadU16() + 4;   // the stored length excludes the four framing bytes

        int trailer = start + size - 5;
        if (size < 5 || trailer + 3 >= reader.Length) { reader.Seek(start); return null; }

        if (reader.PeekAt(trailer) + (reader.PeekAt(trailer + 1) << 8) + 4 != size
            || reader.PeekAt(trailer + 2) != subGroup
            || reader.PeekAt(trailer + 3) != group)
        {
            reader.Seek(start);
            return null;
        }

        return (subGroup, size);
    }

    private static void ParseVariableLengthGroup(WpdReader reader, WpdEventSink sink, byte group, int depth)
    {
        int start = reader.Position;
        if (ReadVariableGroupHeader(reader, group) is not var (subGroup, size))
        {
            // Inconsistent: skip only this byte and try to resynchronise at the next one.
            return;
        }

        switch (group)
        {
            case TopHeaderFooterGroup:
                ParseHeaderFooter(reader, sink, subGroup, size, depth);
                break;

            case TopFootnoteEndnoteGroup:
                ParseNote(reader, sink, subGroup, size, depth);
                break;

            case TopDefinitionGroup:
                if (subGroup == DefinitionGroupDefineTables)
                    ParseTableDefinition(reader, sink, start + 3);
                break;

            case TopTableEolGroup:
                switch (subGroup)
                {
                    case 0x00: ParseTableCell(reader, sink, start + 3); break;
                    case 0x01: sink.InsertRow(header: false); break;
                    case 0x02: sink.EndTable(); break;
                }
                break;

            case TopTableEopGroup:
                // A row that runs onto the next page, and the table ending on one.
                switch (subGroup)
                {
                    case 0x01: case 0x03: sink.InsertRow(header: false); break;
                    case 0x02: sink.EndTable(); break;
                }
                break;
        }

        reader.Seek(start + size - 1);
    }

    /// <summary>
    /// A header or footer carries its text as a sub-document after a 26-byte preamble.
    /// </summary>
    /// <remarks>
    /// Subgroups 0 and 1 are the two headers, 2 and 3 the two footers. The occurrence byte says
    /// which pages it appears on, and a zero there means the definition is empty.
    /// </remarks>
    private static void ParseHeaderFooter(
        WpdReader reader, WpdEventSink sink, byte subGroup, int size, int depth)
    {
        int bodyLength = size - 26;
        reader.Skip(7);
        byte occurrence = reader.ReadU8();
        if (occurrence == 0 || bodyLength <= 0) return;

        reader.Skip(10);
        bool footer = subGroup is 0x02 or 0x03;
        sink.Emit(WpdEvent.Simple(footer ? WpdEventKind.FooterStart : WpdEventKind.HeaderStart));
        ParseBody(new WpdReader(reader.Slice(reader.Position, bodyLength)), sink, depth + 1);
        sink.Emit(WpdEvent.Simple(footer ? WpdEventKind.FooterEnd : WpdEventKind.HeaderEnd));
    }

    /// <summary>
    /// A footnote or endnote carries its body as a sub-document after a variable preamble.
    /// </summary>
    /// <remarks>
    /// A footnote's preamble length depends on how many pages it continues onto, which is why the
    /// remaining size has to be computed rather than assumed.
    /// </remarks>
    private static void ParseNote(WpdReader reader, WpdEventSink sink, byte subGroup, int size, int depth)
    {
        int remaining = size - 8;
        reader.ReadU8();                      // flags
        reader.ReadU16();                     // note number
        remaining -= 3;

        bool endnote = subGroup == 0x01;
        if (!endnote)
        {
            int additionalPages = reader.ReadU8();
            remaining -= 1;
            int preamble = 2 * (additionalPages + 1) + 9;
            reader.Skip(preamble);
            remaining -= preamble;
        }
        else
        {
            reader.Skip(4);
            remaining -= 4;
        }

        if (remaining <= 0) return;

        sink.Emit(new WpdEvent(WpdEventKind.NoteStart) { Endnote = endnote });
        ParseBody(new WpdReader(reader.Slice(reader.Position, remaining)), sink, depth + 1);
        sink.Emit(WpdEvent.Simple(WpdEventKind.NoteEnd));
    }
}
