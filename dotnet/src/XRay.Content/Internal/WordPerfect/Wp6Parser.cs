namespace XRay.Content.Internal.WordPerfect;

/// <summary>
/// WordPerfect 6.x parser, reimplemented from libwpd's <c>WP6Parser</c>.
/// </summary>
/// <remarks>
/// <para>
/// The 6.x body classifies bytes differently from 5.x: 0x01 to 0x20 are an international
/// character shorthand rather than control codes, ASCII runs to 0x7F, single-byte functions reach
/// all the way to 0xCF, variable-length groups occupy 0xD0 to 0xEF, and fixed-length groups sit
/// at the top from 0xF0.
/// </para>
/// <para>
/// The single-byte function block is where 6.x puts most of its structure: table rows, cells and
/// table-off are single bytes, with a multi-byte variant used only when a row needs non-default
/// values. That is why so much of the table handling here is a switch over byte values rather
/// than group parsing.
/// </para>
/// </remarks>
internal static class Wp6Parser
{
    /// <summary>Size of each fixed-length group from 0xF0 to 0xFE, counting both delimiters.</summary>
    /// <remarks>0xFF has no size: it is reserved and never opens a group.</remarks>
    private static readonly int[] FixedLengthGroupSize =
    {
        4, 5, 3, 3, 3, 3, 4, 4, 4, 5, 5, 6, 6, 8, 8,
    };

    /// <summary>
    /// The shorthand for accented characters that 6.x encodes in the 0x01 to 0x20 range.
    /// </summary>
    /// <remarks>
    /// These are Latin-1 code points, so the byte's own value has nothing to do with the
    /// character it names.
    /// </remarks>
    private static readonly ushort[] ExtendedInternational =
    {
        229, 197, 230, 198, 228, 196, 225, 224, 226, 227, 195, 231, 199, 235, 233, 201,
        232, 234, 237, 241, 209, 248, 216, 245, 213, 246, 214, 252, 220, 250, 249, 223,
    };

    private const byte TopExtendedCharacter = 0xF0;
    private const byte TopUndoGroup = 0xF1;
    private const byte TopAttributeOn = 0xF2;
    private const byte TopAttributeOff = 0xF3;

    private const byte TopEolGroup = 0xD0;
    private const byte TopParagraphGroup = 0xD3;
    private const byte TopCharacterGroup = 0xD4;
    private const byte TopFootnoteEndnoteGroup = 0xD7;
    private const byte TopDisplayNumberReferenceGroup = 0xDA;
    private const byte TopTabGroup = 0xE0;

    /// <summary>Set in a variable-length group's flags when prefix IDs follow.</summary>
    private const byte VariableGroupPrefixIdBit = 0x80;

    /// <summary>Set in a group's flags when the document wants the function ignored.</summary>
    private const byte VariableGroupIgnoreBit = 0x40;

    private const int TabBack = 0x00;
    private const int TabTable = 0x01;
    private const int TabBar = 0x04;
    private const int TabCenter = 0x0A;
    private const int TabRight = 0x12;
    private const int TabDecimal = 0x1A;

    private const byte FootnoteOn = 0x00;
    private const byte FootnoteOff = 0x01;
    private const byte EndnoteOn = 0x02;
    private const byte EndnoteOff = 0x03;

    private const byte CharacterGroupTableDefinitionOn = 0x2A;
    private const byte CharacterGroupTableDefinitionOff = 0x2B;
    private const byte CharacterGroupTableColumn = 0x2C;

    private const byte AttributeSuperscript = 5;
    private const byte AttributeSubscript = 6;
    private const byte AttributeItalics = 8;
    private const byte AttributeBold = 12;
    private const byte AttributeStrikeOut = 13;
    private const byte AttributeUnderline = 14;

    /// <summary>How deep a packet may pull in another before the nesting is refused.</summary>
    /// <remarks>
    /// Packets address each other by number, so nothing in the format stops one from naming
    /// itself. The limit is generous enough that a note inside a boxed note inside a table
    /// still reads, and small enough that a cycle cannot exhaust the stack.
    /// </remarks>
    private const int MaxPacketDepth = 8;

    /// <summary>What the parse carries across a group boundary.</summary>
    private sealed class Wp6State
    {
        public required Wp6PrefixData Prefix { get; init; }

        /// <summary>How many packets deep this parse already is.</summary>
        public int Depth { get; init; }

        /// <summary>The packet holding the body of the note currently being read.</summary>
        public int PendingNotePacket { get; set; }
    }

    /// <summary>Parse a WordPerfect 6.x document into an event stream.</summary>
    public static WpdDocument Parse(byte[] bytes, WpdHeader header)
    {
        var document = new WpdDocument();
        var sink = new WpdEventSink(document.Events);
        var reader = new WpdReader(bytes);
        reader.Seek((int)header.DocumentOffset);
        ParseBody(reader, sink, new Wp6State { Prefix = Wp6PrefixData.Read(bytes) });
        sink.Finish();
        return document;
    }

    private static void ParseBody(WpdReader reader, WpdEventSink sink, Wp6State state)
    {
        while (!reader.AtEnd)
        {
            Cancel.ThrowIfRequested();
            byte value = reader.ReadU8();

            if (value == 0)
            {
                // Meaningless, and most often just corruption.
            }
            else if (value <= 0x20)
            {
                sink.Character((char)ExtendedInternational[value - 1]);
            }
            else if (value <= 0x7F)
            {
                sink.Character((char)value);
            }
            else if (value <= 0xCF)
            {
                ParseSingleByteFunction(sink, value);
            }
            else if (value <= 0xEF)
            {
                ParseVariableLengthGroup(reader, sink, value, state);
            }
            else if (value < 0xFF)
            {
                ParseFixedLengthGroup(reader, sink, value);
            }
        }
    }

    private static void ParseSingleByteFunction(WpdEventSink sink, byte function)
    {
        switch (function)
        {
            case 0x80:                                  // soft space
            case 0xCD: case 0xCE: case 0xCF:            // soft end of line
                sink.Character(' ');
                break;

            case 0x81: sink.Character(' '); break; // hard space
            case 0x82: case 0x83: sink.Character('­'); break; // soft hyphen
            case 0x84: sink.Character('-'); break;      // hard hyphen

            case 0x87:                                  // dormant hard return
            case 0xB7: case 0xB8: case 0xB9:            // deletable hard end of line
            case 0xCA: case 0xCB: case 0xCC:            // hard end of line
                sink.ParagraphEnd();
                break;

            // Column and page breaks (0xC7-0xC9 and their deletable forms 0xB4-0xB6) end no
            // paragraph and produce no text, and the deletable soft end-of-line codes
            // (0xBA-0xBC) are removed content that libwpd does not dispatch at all.

            case 0xBD: case 0xBE: case 0xBF:            // table off
                sink.EndTable();
                break;

            case 0xC0: case 0xC1: case 0xC2:
            case 0xC3: case 0xC4: case 0xC5:            // table row
                // The single-byte form exists only for a row with default values, and it
                // carries the row's first cell with it — a row code is never on its own.
                sink.InsertRow(header: false);
                sink.InsertCell(1, 1);
                break;

            case 0xC6:                                  // table cell
                sink.InsertCell(1, 1);
                break;
        }
    }

    /// <summary>Whether a fixed-length group repeats its own opening byte where it should.</summary>
    private static bool FixedGroupIsConsistent(WpdReader reader, byte group)
    {
        int closing = reader.Position + FixedLengthGroupSize[group - 0xF0] - 2;
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
                sink.Text(WpCharacterMap.Wp6Extended(character, characterSet));
                break;
            }

            case TopUndoGroup:
            {
                // WordPerfect keeps deleted text in the file so an undo can restore it, bracketed
                // by this group. Reading it would put text the author removed into the output.
                byte undoType = reader.ReadU8();
                if (undoType == 0) sink.Discarding = true;        // invalid text starts
                else if (undoType == 1) sink.Discarding = false;  // invalid text ends
                break;
            }

            case TopAttributeOn: Attribute(sink, reader.ReadU8(), on: true); break;
            case TopAttributeOff: Attribute(sink, reader.ReadU8(), on: false); break;
        }

        reader.Seek(start + FixedLengthGroupSize[group - 0xF0] - 1);
    }

    /// <summary>
    /// What an end-of-line group's embedded sub-functions said about the row or cell it opens.
    /// </summary>
    private struct EolInfo
    {
        public bool IsHeaderRow;
        public int ColSpan;
        public int RowSpan;

        /// <summary>Whether this grid slot is covered by a cell in an earlier row.</summary>
        public bool BoundFromAbove;

        public static EolInfo Default => new() { ColSpan = 1, RowSpan = 1 };
    }

    /// <summary>
    /// Read the sub-functions packed into an end-of-line group's non-deletable data.
    /// </summary>
    /// <remarks>
    /// A row or cell code carries its properties here rather than in fixed fields: the row's
    /// header flag and a cell's spans are what decide the table's shape, so they have to be
    /// read even though everything else in this block is presentation.
    /// </remarks>
    private static EolInfo ParseEolSubFunctions(WpdReader reader, int contentStart, int sizeNonDeletable)
    {
        var info = EolInfo.Default;

        reader.Seek(contentStart);
        int sizeDeletable = reader.ReadU16();
        if (sizeDeletable > sizeNonDeletable) return info;
        reader.Skip(sizeDeletable);

        int end = contentStart + sizeNonDeletable;
        while (reader.Position < end && !reader.AtEnd)
        {
            Cancel.ThrowIfRequested();
            byte function = reader.ReadU8();
            int after = reader.Position;
            int size;

            switch (function)
            {
                case 128:                                   // row information
                    size = 5;
                    byte rowFlags = reader.ReadU8();
                    if ((rowFlags & 0x04) != 0) info.IsHeaderRow = true;
                    break;

                case 129:                                   // cell formula, length-prefixed
                case 0x8E: case 0x8F:                       // undocumented, also length-prefixed
                    size = reader.ReadU16();
                    break;

                case 130: case 131: size = 4; break;        // gutter spacing
                case 132: size = 9; break;                  // cell information
                case 133:                                   // cell spanning information
                    size = 4;
                    info.ColSpan = reader.ReadU8();
                    info.RowSpan = reader.ReadU8();
                    // A span of 128 or more is the marker for a slot the row above covers.
                    if (info.ColSpan >= 128) info.BoundFromAbove = true;
                    break;

                case 134: size = 10; break;                 // cell fill colours
                case 135: size = 6; break;                  // cell line colour
                case 136: size = 6; break;                  // cell number type
                case 137: size = 11; break;                 // cell floating point number
                case 139: size = 3; break;                  // cell prefix flag
                case 140: size = 3; break;                  // recalculation error number
                case 141: size = 1; break;                  // don't end a paragraph style

                default:
                    // An unrecognised sub-function means the group is not what it claims to
                    // be; libwpd abandons the whole group here rather than guessing a length.
                    return info;
            }

            if (after + size - 1 < reader.Position) return info;
            reader.Seek(after + size - 1);
        }

        return info;
    }

    /// <summary>
    /// Dispatch an end-of-line group by its subgroup.
    /// </summary>
    /// <remarks>
    /// This one group covers everything from a soft line wrap to a table row: 6.x folds the whole
    /// family into subgroups of 0xD0, so treating the group as a single paragraph break turns
    /// every wrapped line into a paragraph and every table row into two.
    /// </remarks>
    private static void ParseEolGroup(WpdEventSink sink, byte subGroup, in EolInfo info)
    {
        switch (subGroup)
        {
            // A soft break is where the text merely wrapped, so it reads as a space.
            case 0x01: case 0x02: case 0x03:
                sink.Character(' ');
                break;

            // Hard returns, and the deletable forms of them, end a paragraph.
            case 0x04: case 0x05: case 0x06:
            case 0x17: case 0x18: case 0x19: case 0x1C:
                sink.ParagraphEnd();
                break;

            case 0x0A:
                if (!info.BoundFromAbove) sink.InsertCell(info.ColSpan, info.RowSpan);
                break;

            case 0x0B: case 0x0C: case 0x0D: case 0x0E: case 0x0F: case 0x10:
                sink.InsertRow(info.IsHeaderRow);
                if (!info.BoundFromAbove) sink.InsertCell(info.ColSpan, info.RowSpan);
                break;

            case 0x11: case 0x12: case 0x13:
                sink.EndTable();
                break;

            // Column and page breaks (0x07-0x09, 0x1A, 0x1B) produce no text, and the deletable
            // soft forms (0x14-0x16) are removed content.
        }
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
        }
    }

    /// <summary>
    /// Read a variable-length group, whose header is subgroup, size, flags, optional prefix IDs
    /// and a non-deletable size, and whose trailer repeats the size and the opening byte.
    /// </summary>
    /// <remarks>
    /// The prefix IDs are how 6.x addresses content held elsewhere in the file — a header's text,
    /// a note's body — rather than inline as 5.x does. Those packets are not read here, which is
    /// the one deliberate gap in this parser.
    /// </remarks>
    private static void ParseVariableLengthGroup(
        WpdReader reader, WpdEventSink sink, byte group, Wp6State state)
    {
        int start = reader.Position;

        // Consistency first: the trailer has to agree before the header is trusted.
        byte subGroup = reader.ReadU8();
        int size = reader.ReadU16();
        int trailer = start + size - 4;
        if (size == 0 || trailer + 2 >= reader.Length
            || reader.PeekAt(trailer) + (reader.PeekAt(trailer + 1) << 8) != size
            || reader.PeekAt(trailer + 2) != group)
        {
            reader.Seek(start);
            return;
        }

        byte flags = reader.ReadU8();
        int firstPrefixId = 0;
        if ((flags & VariableGroupPrefixIdBit) != 0)
        {
            int prefixIdCount = reader.ReadU8();
            for (int i = 0; i < prefixIdCount; i++)
            {
                int prefixId = reader.ReadU16();
                if (i == 0) firstPrefixId = prefixId;
            }
        }

        int sizeNonDeletable = reader.ReadU16();
        int contentStart = reader.Position;

        // A non-deletable block claiming to be larger than the group, or with the high bit set,
        // is corruption; libwpd abandons the group rather than reading past it.
        bool contentsUsable = sizeNonDeletable <= size && (sizeNonDeletable & 0x8000) == 0;

        switch (group)
        {
            case TopTabGroup:
                // A tab the document marked as ignored is a position record, not a tab stop
                // anyone typed; emitting it indents a line that is not indented.
                if ((flags & VariableGroupIgnoreBit) == 0) InsertTab(sink, subGroup);
                break;

            case TopDisplayNumberReferenceGroup:
                // The subgroups pair up: an even one starts a number the document writes out
                // literally, the odd one after it ends it. What lies between is the number
                // itself, which belongs to whatever owns it rather than to the running text.
                if ((subGroup & 1) == 0) sink.NumberTextOn();
                else sink.NumberTextOff();
                break;

            case TopFootnoteEndnoteGroup:
                switch (subGroup)
                {
                    case FootnoteOn:
                    case EndnoteOn:
                        sink.NoteOn();
                        state.PendingNotePacket = firstPrefixId;
                        break;
                    case FootnoteOff: EmitNote(sink, state, endnote: false); break;
                    case EndnoteOff: EmitNote(sink, state, endnote: true); break;
                }
                break;

            case TopEolGroup:
            {
                var info = contentsUsable
                    ? ParseEolSubFunctions(reader, contentStart, sizeNonDeletable)
                    : EolInfo.Default;
                ParseEolGroup(sink, subGroup, info);
                break;
            }

            case TopCharacterGroup:
                if (contentsUsable) ParseCharacterGroup(reader, sink, subGroup, contentStart);
                break;

            // The paragraph group carries justification and spacing: formatting only, with no
            // text and no structure the event stream models.
            case TopParagraphGroup:
                break;
        }

        reader.Seek(start + size - 1);
    }

    /// <summary>
    /// Emit a note, pulling its body out of the packet the anchor named.
    /// </summary>
    /// <remarks>
    /// The body is a WordPerfect document in its own right, so it is parsed the same way the
    /// main body is — which is what lets a note hold its own formatting, and a table.
    /// </remarks>
    private static void EmitNote(WpdEventSink sink, Wp6State state, bool endnote)
    {
        int packetId = state.PendingNotePacket;
        state.PendingNotePacket = 0;

        sink.NoteBegin(endnote);

        var body = packetId > 0 && state.Depth < MaxPacketDepth
            ? state.Prefix.TextPacket(packetId)
            : null;
        if (body is { Length: > 0 })
        {
            ParseBody(
                new WpdReader(body),
                sink,
                new Wp6State { Prefix = state.Prefix, Depth = state.Depth + 1 });
        }

        sink.NoteFinish();
    }

    /// <summary>
    /// Emit a tab, or the indent it stands for.
    /// </summary>
    /// <remarks>
    /// A tab before any text on the line is not a tab at all: WordPerfect uses the tab codes to
    /// express a paragraph's first-line indent and its left margin, and only a tab that lands
    /// mid-line is a tab stop the reader sees. Emitting the indent forms would put a stray tab
    /// at the head of every indented paragraph.
    /// </remarks>
    private static void InsertTab(WpdEventSink sink, byte subGroup)
    {
        int type = (subGroup & 0xF8) >> 3;

        // A back tab is a hanging indent; it is never a tab stop.
        if (type == TabBack) return;

        bool alwaysATab = type is TabTable or TabBar or TabCenter or TabRight or TabDecimal;
        if (!sink.ParagraphStarted && !alwaysATab) return;

        sink.Tab();

        // A bar tab draws a vertical rule, which has no character of its own.
        if (type == TabBar) sink.Character('|');
    }

    /// <summary>
    /// Dispatch a character group by its subgroup.
    /// </summary>
    /// <remarks>
    /// Most of this group is font and colour changes the event stream does not model. What it
    /// does carry is the table definition — the columns a table has, which is what tells a later
    /// row how wide it should be and which of its slots a vertical span already covers.
    /// </remarks>
    private static void ParseCharacterGroup(
        WpdReader reader, WpdEventSink sink, byte subGroup, int contentStart)
    {
        switch (subGroup)
        {
            case CharacterGroupTableDefinitionOn:
                sink.DefineTable();
                break;

            case CharacterGroupTableColumn:
                sink.DefineTableColumn();
                break;

            case CharacterGroupTableDefinitionOff:
                // The definition is complete, so the table itself opens here.
                sink.StartTable();
                break;
        }

        reader.Seek(contentStart);
    }
}
