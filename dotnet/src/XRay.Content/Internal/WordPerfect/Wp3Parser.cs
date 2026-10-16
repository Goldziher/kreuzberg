namespace XRay.Content.Internal.WordPerfect;

/// <summary>
/// Macintosh WordPerfect 2.x and 3.x parser, reimplemented from libwpd's <c>WP3Parser</c>.
/// </summary>
/// <remarks>
/// <para>
/// Structurally the closest of the families to 5.x — single-byte functions from 0x80, fixed-length
/// groups from 0xC0, variable-length ones from 0xD0 — but with two differences that matter. Its
/// variable-length group sizes are big-endian, being a Macintosh format, and its characters carry
/// both a Mac Roman byte and a WordPerfect character-set pair, with the Mac one preferred.
/// </para>
/// <para>
/// Control characters below 0x20 carry no meaning at all here, unlike every other family, where
/// the hard return lives at 0x0A.
/// </para>
/// </remarks>
internal static class Wp3Parser
{
    /// <summary>Size of each fixed-length group from 0xC0 to 0xCF, counting both delimiters.</summary>
    private static readonly int[] FixedLengthGroupSize =
    {
        5, 8, 7, 4, 4, 7, 10, 7, 4, 5, 6, 6, 7, 9, 7, 4,
    };

    private const byte ExtendedCharacterGroup = 0xC0;
    private const byte TabGroup = 0xC1;
    private const byte IndentGroup = 0xC2;
    private const byte AttributeGroup = 0xC3;
    private const byte UndoGroup = 0xCD;
    private const byte HeaderFooterGroup = 0xD5;
    private const byte FootnoteEndnoteGroup = 0xD6;
    private const byte EndOfLinePageGroup = 0xDC;
    private const byte TablesGroup = 0xE2;

    private const byte AttributeBold = 0;
    private const byte AttributeItalics = 1;
    private const byte AttributeUnderline = 2;
    private const byte AttributeStrikeOut = 9;
    private const byte AttributeSubscript = 10;
    private const byte AttributeSuperscript = 11;

    /// <summary>Parse a Macintosh WordPerfect document into an event stream.</summary>
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

    private static void ParseBody(WpdReader reader, WpdEventSink sink, int depth = 0)
    {
        if (depth > 8) return;

        while (!reader.AtEnd)
        {
            Cancel.ThrowIfRequested();
            byte value = reader.ReadU8();

            if (value is 0 or 0x7F or 0xFF)
            {
                // Meaningless, and most often just corruption.
            }
            else if (value <= 0x1F)
            {
                // Control characters carry nothing here; the hard return is a group instead.
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
            else if (value <= 0xEF)
            {
                ParseVariableLengthGroup(reader, sink, value, depth);
            }
        }
    }

    private static void ParseSingleByteFunction(WpdEventSink sink, byte function)
    {
        switch (function)
        {
            case 0x80: sink.ParagraphEnd(); break;   // condensed hard return
            case 0x81: sink.ParagraphEnd(); break;   // condensed hard page
            case 0x82: case 0x84: case 0x85: sink.Tab(); break;  // condensed tab and indents
            case 0x96: sink.Character('-'); break;   // hard hyphen
            case 0x97: sink.Character('­'); break;   // soft hyphen
            case 0xA0: sink.Character(' '); break;   // hard space
            // A back-tab (0x83) moves left, so it inserts nothing.
        }
    }

    private static void ParseFixedLengthGroup(WpdReader reader, WpdEventSink sink, byte group)
    {
        int size = FixedLengthGroupSize[group - 0xC0];
        int start = reader.Position;
        int closing = start + size - 2;
        if (closing >= reader.Length || reader.PeekAt(closing) != group) return;

        switch (group)
        {
            case ExtendedCharacterGroup:
            {
                byte macCharacter = reader.ReadU8();
                byte characterSet = reader.ReadU8();
                byte character = reader.ReadU8();

                // The Mac byte wins where it has a value; the WordPerfect pair is the fallback.
                // A set of 0xFF with a character of 0xFE or 0xFF means "nothing", not a character.
                if (macCharacter >= 0x20)
                    sink.Text(WpCharacterMap.MacRomanCharacter(macCharacter));
                else if (characterSet != 0xFF || (character != 0xFE && character != 0xFF))
                    sink.Text(WpCharacterMap.Wp5Extended(character, characterSet));
                break;
            }

            case TabGroup:
            case IndentGroup:
                sink.Tab();
                break;

            case AttributeGroup:
            {
                // The attribute comes first and its state second; the low bit of the state is
                // what says on or off.
                byte attribute = reader.ReadU8();
                byte state = reader.ReadU8();
                Attribute(sink, attribute, on: (state & 0x01) == 0x01);
                break;
            }

            case UndoGroup:
            {
                byte undoType = reader.ReadU8();
                if (undoType == 0) sink.Discarding = true;
                else if (undoType == 1) sink.Discarding = false;
                break;
            }
        }

        reader.Seek(start + size - 1);
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
    /// Read a variable-length group, whose size is big-endian and whose trailer repeats the size,
    /// the subgroup and the opening byte.
    /// </summary>
    private static void ParseVariableLengthGroup(
        WpdReader reader, WpdEventSink sink, byte group, int depth)
    {
        int start = reader.Position;
        byte subGroup = reader.ReadU8();
        int size = reader.ReadU16(bigEndian: true);

        int trailer = start + size - 1;
        if (size < 4 || trailer + 3 >= reader.Length
            || (reader.PeekAt(trailer) << 8) + reader.PeekAt(trailer + 1) != size
            || reader.PeekAt(trailer + 2) != subGroup
            || reader.PeekAt(trailer + 3) != group)
        {
            reader.Seek(start);
            return;
        }

        switch (group)
        {
            case EndOfLinePageGroup:
                ParseEndOfLinePage(sink, subGroup);
                break;

            case TablesGroup:
                // Subgroup 1 is the table definition, which is what actually opens a table. The
                // rest set cell borders, colours and alignment, none of which is text.
                if (subGroup == 0x01 && !sink.TableOpen)
                {
                    sink.StartTable();
                    sink.InsertRow(header: false);
                    sink.InsertCell(1, 1);
                }
                break;

            case HeaderFooterGroup:
            case FootnoteEndnoteGroup:
                // Both hold their bodies in the resource fork, addressed separately from the
                // document stream. That fork is not read here.
                break;
        }

        reader.Seek(start + size + 3);
    }

    /// <summary>
    /// Dispatch an end-of-line or end-of-page group by subgroup.
    /// </summary>
    /// <remarks>
    /// These mappings — a soft end of line reading as a space, a hard one ending a paragraph —
    /// come from the format documentation rather than from inspection, and libwpd's source says
    /// as much.
    /// </remarks>
    private static void ParseEndOfLinePage(WpdEventSink sink, byte subGroup)
    {
        switch (subGroup)
        {
            case 0x00: case 0x01:                      // soft end of line / page
                sink.Character(' ');
                break;

            case 0x02: case 0x03: case 0x04:           // hard end of line, temporary end of line
            case 0x06:                                 // dormant hard return
            case 0x0A: case 0x0B:                      // hard end of line outside columns
            // A page or column break closes the paragraph too: libwpd's listener ends the current
            // one before starting the new page or column, so the break is a paragraph boundary in
            // the event stream even though it carries no text of its own.
            case 0x05: case 0x07: case 0x08: case 0x09:
            case 0x14: case 0x15:
                sink.ParagraphEnd();
                break;

            case 0x0C: case 0x0D: sink.Character('-'); break;  // hard hyphen at a break
            case 0x0E: case 0x0F: sink.Character('­'); break;  // soft hyphen at a break

            // A cell or row boundary only means a cell or row when a table is actually open.
            // Outside one — which is how a Mac document lays out columns with tabs — the same
            // codes are ordinary paragraph structure, and treating them as table events would
            // drop the break entirely.
            case 0x16:                                 // hard end of table cell
                if (sink.TableOpen) sink.InsertCell(1, 1);
                break;

            case 0x18:                                 // hard end of table row and cell
                if (sink.TableOpen)
                {
                    sink.InsertRow(header: false);
                    sink.InsertCell(1, 1);
                }
                else sink.ParagraphEnd();
                break;

        }
    }
}
