using XRay.Content.Internal.Cfb;

namespace XRay.Content.Internal.Doc;

/// <summary>
/// Paragraph properties (<c>PAPX</c>) for legacy <c>.doc</c>: which paragraphs are bound to an
/// automatic list, at what depth, and which carry a heading style.
/// </summary>
/// <remarks>
/// <para>
/// [MS-DOC] stores paragraph formatting in <c>PAPX</c> grpprls held in 512-byte <c>PapxFkp</c>
/// pages, indexed by byte offset (<c>FC</c>) through the FIB's <c>PlcfBtePapx</c>. This reads only
/// what the element stream needs: list membership (<c>sprmPIlfo</c>), nesting depth
/// (<c>sprmPIlvl</c>), and the paragraph's style index. Resolving the <em>number</em> Word paints
/// needs a counter walk over <c>PlfLst</c>/<c>PlfLfo</c> and is deliberately out of scope.
/// </para>
/// <para>
/// Ports upstream's <c>extraction/doc/papx.rs</c> (xberg-io/xberg#1550, #1553). PAPX is
/// FC-addressed and text is CP-addressed, so binding one to the other needs the piece table's
/// mapping — which is why none of this was reachable until the <c>fcClx</c> pair was read from
/// the offset the spec gives it (#1551).
/// </para>
/// </remarks>
internal static class DocPapx
{
    // ── FibRgFcLcb97 pair indices ─────────────────────────────────────────────

    /// <summary><c>fcStshf</c> — the style sheet.</summary>
    private const int FibFcLcbIdxStshf = 1;

    /// <summary><c>fcPlcfBtePapx</c> — the paragraph-property bin table.</summary>
    private const int FibFcLcbIdxPlcfBtePapx = 13;

    /// <summary><c>fcPlfLst</c> — the list definition table.</summary>
    private const int FibFcLcbIdxPlfLst = 73;

    /// <summary><c>fcPlfLfo</c> — the list format overrides <c>ilfo</c> indexes into.</summary>
    private const int FibFcLcbIdxPlfLfo = 74;

    // ── record sizes ──────────────────────────────────────────────────────────

    /// <summary><c>LSTF</c> record size in the <c>PlfLst</c> array.</summary>
    private const int LstfLen = 28;

    /// <summary><c>LFO</c> record size in the <c>PlfLfo</c> array.</summary>
    private const int LfoLen = 16;

    /// <summary><c>LVLF</c> header size, before the two grpprls and the <c>Xst</c> after it.</summary>
    private const int LvlfLen = 28;

    /// <summary>Every <c>PapxFkp</c> is exactly one 512-byte page; its entry count is the last byte.</summary>
    private const int FkpPageLen = 512;

    /// <summary><c>BxPap</c> in the Word 97 FKP layout: a 1-byte word offset then 12 reserved bytes.</summary>
    private const int BxPapLen = 13;

    // ── numbering formats ─────────────────────────────────────────────────────

    /// <summary>A level with this <c>nfc</c> paints a bullet glyph ([MS-DOC] 2.9.131 MSONFC).</summary>
    private const byte NfcBullet = 23;

    /// <summary>A level with this <c>nfc</c> paints nothing.</summary>
    private const byte NfcNone = 255;

    /// <summary>How many <c>LVL</c> records a non-simple list carries, one per outline level.</summary>
    private const int LvlCountMultilevel = 9;

    // ── styles ────────────────────────────────────────────────────────────────

    /// <summary><c>STD.istdBase</c> value meaning "this style has no base style".</summary>
    private const ushort IstdNoBase = 0x0FFF;

    /// <summary>
    /// Built-in style identifiers 1..9 are <c>heading 1</c>..<c>heading 9</c>. [MS-DOC] reserves
    /// the low <c>sti</c> range for fixed built-in styles, so this mapping is specified rather
    /// than conventional.
    /// </summary>
    private const ushort StiHeadingMin = 1;
    private const ushort StiHeadingMax = 9;

    /// <summary>
    /// How far a style's base chain is followed. Chains are shallow in practice; the bound exists
    /// so a malformed self- or cycle-referencing sheet cannot spin.
    /// </summary>
    private const int MaxStyleBaseDepth = 8;

    // ── sprms ─────────────────────────────────────────────────────────────────

    /// <summary>
    /// <c>sprmPIlfo</c> — the 1-based list index binding a paragraph to a list. Zero means the
    /// paragraph is not in a list. <c>spra</c> 2, so a 2-byte operand.
    /// </summary>
    private const ushort SprmPIlfo = 0x460B;

    /// <summary><c>sprmPIlvl</c> — zero-based nesting depth. <c>spra</c> 1, so a 1-byte operand.</summary>
    private const ushort SprmPIlvl = 0x260A;

    /// <summary>A paragraph's binding to an automatic list.</summary>
    /// <param name="Ilfo">
    /// 1-based index into the list-format overrides. Never zero: a zero <c>ilfo</c> means "not in
    /// a list" and yields no binding at all.
    /// </param>
    /// <param name="Ilvl">Zero-based nesting depth. An absent <c>sprmPIlvl</c> means level 0.</param>
    internal readonly record struct ListBinding(ushort Ilfo, byte Ilvl);

    /// <summary>
    /// Operand length in bytes for a sprm, from its <c>spra</c> field (bits 13-15).
    /// </summary>
    /// <remarks>
    /// <c>spra</c> 6 is variable-length, where the first operand byte is the count; returning null
    /// lets the caller read it. Any other value is a malformed sprm and stops the walk rather than
    /// guessing a stride.
    /// </remarks>
    internal static int? SprmOperandLen(ushort sprm) => (sprm >> 13) switch
    {
        0 or 1 => 1,
        2 or 4 or 5 => 2,
        3 => 4,
        7 => 3,
        _ => null,
    };

    /// <summary>Scan a <c>grpprl</c> for the two sprms that describe list membership.</summary>
    internal static ListBinding? ListBindingFromGrpprl(ReadOnlySpan<byte> grpprl)
    {
        ushort? ilfo = null;
        byte ilvl = 0;
        int pos = 0;

        while (pos + 2 <= grpprl.Length)
        {
            Cancel.ThrowIfRequested();
            ushort sprm = (ushort)(grpprl[pos] | (grpprl[pos + 1] << 8));
            pos += 2;

            int len;
            if (SprmOperandLen(sprm) is { } fixedLen) len = fixedLen;
            else if (sprm >> 13 == 6 && pos < grpprl.Length) len = grpprl[pos] + 1;
            else break;

            if (pos + len > grpprl.Length) break;
            var operand = grpprl.Slice(pos, len);
            pos += len;

            if (sprm == SprmPIlfo && operand.Length >= 2) ilfo = (ushort)(operand[0] | (operand[1] << 8));
            else if (sprm == SprmPIlvl && operand.Length >= 1) ilvl = operand[0];
        }

        return ilfo is { } value && value != 0 ? new ListBinding(value, ilvl) : null;
    }

    /// <summary>
    /// Extract one <c>BxPap</c> entry's <c>GrpPrlAndIstd</c>: the paragraph's style index and the
    /// grpprl that overrides it.
    /// </summary>
    /// <remarks>
    /// <paramref name="bOffset"/> is a <em>word</em> offset into the page; zero means the paragraph
    /// has no <c>PAPX</c> at all. That is returned as absent rather than defaulted, so a document
    /// that does have one is not silently attributed style 0.
    /// </remarks>
    private static bool TryStyleAndGrpprlAt(byte[] page, byte bOffset, out ushort istd, out int grpprlStart, out int grpprlLen)
    {
        istd = 0;
        grpprlStart = 0;
        grpprlLen = 0;
        if (bOffset == 0) return false;

        int at = bOffset * 2;
        if (at >= page.Length) return false;
        byte cb = page[at];

        int start, len;
        if (cb != 0)
        {
            start = at + 1;
            len = cb * 2 - 1;
        }
        else
        {
            if (at + 1 >= page.Length) return false;
            start = at + 2;
            len = page[at + 1] * 2;
        }

        if (len < 2 || start + len > page.Length) return false;
        istd = (ushort)(page[start] | (page[start + 1] << 8));
        grpprlStart = start + 2;
        grpprlLen = len - 2;
        return true;
    }

    /// <summary>Read one <c>FibRgFcLcb97</c> pair, rejecting an absent or empty structure.</summary>
    private static bool TryReadFcLcb(byte[] wordDoc, int rgFcLcbOffset, int index, out int fc, out int lcb)
    {
        fc = 0;
        lcb = 0;
        int at = rgFcLcbOffset + index * 8;
        if (at < 0 || at + 8 > wordDoc.Length) return false;
        fc = (int)OleUtil.U32(wordDoc, at);
        lcb = (int)OleUtil.U32(wordDoc, at + 4);
        return lcb > 0 && fc >= 0;
    }

    /// <summary>
    /// Whether each list level paints numbers or bullets, resolved through
    /// <c>ilfo</c> → <c>LFO</c> → <c>lsid</c> → <c>LSTF</c> → <c>LVL[ilvl]</c> → <c>nfc</c>.
    /// </summary>
    /// <remarks>
    /// A list item carries an <c>ordered</c> flag, so this is not optional detail: emitting every
    /// list-bound paragraph as ordered would mislabel every bulleted list. Resolving <c>nfc</c> is
    /// a lookup, distinct from the counter walk that would paint the number itself.
    /// </remarks>
    internal sealed class ListFormats
    {
        private readonly Dictionary<(ushort Ilfo, byte Ilvl), bool> _orderedByLevel = new();

        public static ListFormats Build(byte[] wordDoc, byte[] tableStream, int rgFcLcbOffset)
        {
            var formats = new ListFormats();
            if (!TryReadFcLcb(wordDoc, rgFcLcbOffset, FibFcLcbIdxPlfLst, out int lstFc, out int lstLcb)) return formats;
            if (!TryReadFcLcb(wordDoc, rgFcLcbOffset, FibFcLcbIdxPlfLfo, out int lfoFc, out int lfoLcb)) return formats;
            if (lstFc >= tableStream.Length || lfoFc < 0 || lfoFc + lfoLcb > tableStream.Length) return formats;

            // `lcbPlfLst` covers only the `cLst` count and the `LSTF` array; the variable-length
            // `LVL` blocks follow immediately *after* it in the table stream. Slicing to `lcb`
            // cuts every `LVL` off, which yields an empty map that `IsOrdered`'s default then
            // papers over — a bulleted list would report as ordered.
            var plfLst = tableStream.AsSpan(lstFc);
            var plfLfo = tableStream.AsSpan(lfoFc, lfoLcb);

            var nfcByLsid = ParsePlfLst(plfLst, lstLcb);

            // PlfLfo: cLfo as u32, then that many 16-byte LFO records whose first field is the
            // lsid of the list they override. `ilfo` is 1-based.
            if (plfLfo.Length < 4) return formats;
            long cLfo = (uint)(plfLfo[0] | (plfLfo[1] << 8) | (plfLfo[2] << 16) | (plfLfo[3] << 24));
            for (long i = 0; i < cLfo; i++)
            {
                long at = 4 + i * LfoLen;
                if (at + LfoLen > plfLfo.Length) break;
                var record = plfLfo.Slice((int)at, LfoLen);
                uint lsid = (uint)(record[0] | (record[1] << 8) | (record[2] << 16) | (record[3] << 24));
                if (!nfcByLsid.TryGetValue(lsid, out var levels)) continue;

                ushort ilfo = i + 1 > ushort.MaxValue ? ushort.MaxValue : (ushort)(i + 1);
                for (int ilvl = 0; ilvl < levels.Count; ilvl++)
                {
                    byte level = ilvl > byte.MaxValue ? byte.MaxValue : (byte)ilvl;
                    formats._orderedByLevel[(ilfo, level)] = levels[ilvl] != NfcBullet && levels[ilvl] != NfcNone;
                }
            }

            return formats;
        }

        /// <summary>
        /// Whether the level paints an ordered number. An unknown level defaults to ordered, which
        /// matches the automatic numbering this exists for; a bulleted list whose tables could not
        /// be read is the rarer case.
        /// </summary>
        public bool IsOrdered(ushort ilfo, byte ilvl) =>
            !_orderedByLevel.TryGetValue((ilfo, ilvl), out bool ordered) || ordered;
    }

    /// <summary>Map each list's <c>lsid</c> to its per-level <c>nfc</c> values.</summary>
    /// <remarks>
    /// <c>PlfLst</c> is <c>cLst</c> as a 16-bit count, then that many fixed-size <c>LSTF</c>
    /// records, then the variable-length <c>LVL</c> blocks for each list in the same order — one
    /// for a simple list, nine otherwise. The <c>LVL</c>s must be walked in sequence because each
    /// one's length depends on its own header.
    /// </remarks>
    private static Dictionary<uint, List<byte>> ParsePlfLst(ReadOnlySpan<byte> plfLst, int lcb)
    {
        var nfcByLsid = new Dictionary<uint, List<byte>>();
        if (plfLst.Length < 2) return nfcByLsid;

        int cLst = plfLst[0] | (plfLst[1] << 8);
        // The declared lcb must actually hold the LSTF array it claims to.
        if (2 + (long)cLst * LstfLen > lcb) return nfcByLsid;

        var lists = new List<(uint Lsid, int LevelCount)>(cLst);
        for (int i = 0; i < cLst; i++)
        {
            int at = 2 + i * LstfLen;
            if (at + LstfLen > plfLst.Length) return nfcByLsid;
            var lstf = plfLst.Slice(at, LstfLen);
            uint lsid = (uint)(lstf[0] | (lstf[1] << 8) | (lstf[2] << 16) | (lstf[3] << 24));
            bool simple = (lstf[26] & 0x01) != 0;
            lists.Add((lsid, simple ? 1 : LvlCountMultilevel));
        }

        int pos = 2 + cLst * LstfLen;
        foreach (var (lsid, levelCount) in lists)
        {
            var nfcs = new List<byte>(levelCount);
            for (int i = 0; i < levelCount; i++)
            {
                if (pos + LvlfLen > plfLst.Length) return nfcByLsid;
                var lvlf = plfLst.Slice(pos, LvlfLen);
                nfcs.Add(lvlf[4]);
                pos += LvlfLen + lvlf[25] + lvlf[24]; // cbGrpprlPapx + cbGrpprlChpx
                // Xst: a 16-bit character count followed by that many UTF-16 units.
                if (pos < 0 || pos + 2 > plfLst.Length) return nfcByLsid;
                pos += 2 + (plfLst[pos] | (plfLst[pos + 1] << 8)) * 2;
            }
            nfcByLsid[lsid] = nfcs;
        }

        return nfcByLsid;
    }

    /// <summary>
    /// The document's style sheet, reduced to the one question asked of it: does a given
    /// <c>istd</c> denote a heading, and at what level.
    /// </summary>
    /// <remarks>
    /// Resolution follows <c>istdBase</c>, so a custom style derived from a built-in heading
    /// resolves too. That case is real: a corpus document carries <c>istd</c> 97 = <c>TOC
    /// Heading</c>, whose <c>sti</c> is 46 but whose base is <c>heading 1</c>. Testing
    /// <c>sti</c> 1..9 alone would miss it.
    /// </remarks>
    internal sealed class StyleSheet
    {
        private readonly record struct StyleEntry(ushort Sti, ushort IstdBase);

        /// <summary>Indexed by <c>istd</c>; absent for an unparseable or empty slot.</summary>
        private readonly List<StyleEntry?> _styles = new();

        public static StyleSheet Build(byte[] wordDoc, byte[] tableStream, int rgFcLcbOffset)
        {
            var sheet = new StyleSheet();
            if (!TryReadFcLcb(wordDoc, rgFcLcbOffset, FibFcLcbIdxStshf, out int fc, out int lcb)) return sheet;
            if (fc < 0 || fc + lcb > tableStream.Length) return sheet;
            var stsh = tableStream.AsSpan(fc, lcb);

            // STSH: cbStshi, then STSHI (whose first field is cstd), then one LPStd per style —
            // a 16-bit length followed by that many bytes.
            if (stsh.Length < 4) return sheet;
            int cbStshi = stsh[0] | (stsh[1] << 8);
            int cstd = stsh[2] | (stsh[3] << 8);

            int pos = 2 + cbStshi;
            for (int i = 0; i < cstd; i++)
            {
                if (pos < 0 || pos + 2 > stsh.Length) break;
                int cbStd = stsh[pos] | (stsh[pos + 1] << 8);
                pos += 2;
                if (cbStd == 0)
                {
                    // An empty slot: the istd exists but names no style.
                    sheet._styles.Add(null);
                    continue;
                }
                if (pos + cbStd > stsh.Length) break;
                var std = stsh.Slice(pos, cbStd);
                pos += cbStd;

                // STDFBase's first two 16-bit words: sti in the low 12 bits of the first,
                // istdBase in the high 12 bits of the second.
                sheet._styles.Add(std.Length >= 4
                    ? new StyleEntry(
                        (ushort)((std[0] | (std[1] << 8)) & 0x0FFF),
                        (ushort)(((std[2] | (std[3] << 8)) >> 4) & 0x0FFF))
                    : null);
            }

            return sheet;
        }

        /// <summary>Heading level for a paragraph style, or null when it is not a heading.</summary>
        public byte? HeadingLevel(ushort istd)
        {
            ushort current = istd;
            for (int i = 0; i < MaxStyleBaseDepth; i++)
            {
                if (current >= _styles.Count) return null;
                if (_styles[current] is not { } entry) return null;
                if (entry.Sti >= StiHeadingMin && entry.Sti <= StiHeadingMax) return (byte)entry.Sti;
                if (entry.IstdBase == IstdNoBase || entry.IstdBase == current) return null;
                current = entry.IstdBase;
            }
            return null;
        }
    }

    /// <summary>
    /// List bindings and style indices for a document, keyed by the byte offset (<c>FC</c>) one
    /// past each paragraph mark.
    /// </summary>
    internal sealed class ParagraphListIndex
    {
        private readonly Dictionary<uint, ListBinding> _byEndFc = new();

        /// <summary>Style index for every paragraph carrying a <c>PAPX</c>, not only bound ones.</summary>
        private readonly Dictionary<uint, ushort> _styleByEndFc = new();

        /// <summary>
        /// Build the index from the <c>PlcfBtePapx</c> plex and the FKP pages it names.
        /// </summary>
        /// <remarks>
        /// Returns an empty index rather than throwing whenever the structures are absent or
        /// malformed: list membership is additive, and a document whose paragraph properties
        /// cannot be read must still yield its text.
        /// </remarks>
        public static ParagraphListIndex Build(byte[] wordDoc, byte[] tableStream, int rgFcLcbOffset)
        {
            var index = new ParagraphListIndex();
            if (!TryReadFcLcb(wordDoc, rgFcLcbOffset, FibFcLcbIdxPlcfBtePapx, out int fc, out int lcb)) return index;
            if (lcb < 4 || fc < 0 || fc + lcb > tableStream.Length) return index;
            var plex = tableStream.AsSpan(fc, lcb);

            // PlcfBtePapx: (n+1) FCs then n 4-byte page numbers, whose low 22 bits are the FKP
            // page index.
            int n = (lcb - 4) / 8;
            for (int i = 0; i < n; i++)
            {
                int pnAt = (n + 1) * 4 + i * 4;
                if (pnAt + 4 > plex.Length) break;
                int pn = (int)((uint)(plex[pnAt] | (plex[pnAt + 1] << 8) | (plex[pnAt + 2] << 16)
                    | (plex[pnAt + 3] << 24)) & 0x003F_FFFF);
                index.AbsorbFkpPage(wordDoc, pn);
            }

            return index;
        }

        private void AbsorbFkpPage(byte[] wordDoc, int pn)
        {
            long start = (long)pn * FkpPageLen;
            if (start < 0 || start + FkpPageLen > wordDoc.Length) return;
            var page = new byte[FkpPageLen];
            Array.Copy(wordDoc, start, page, 0, FkpPageLen);

            int crun = page[FkpPageLen - 1];
            if (crun == 0) return;

            // rgfc holds crun+1 FCs, then rgbx holds crun BxPap entries.
            int rgbxAt = (crun + 1) * 4;
            if (rgbxAt + crun * BxPapLen > FkpPageLen - 1) return;

            for (int i = 0; i < crun; i++)
            {
                // The FKP's rgfc entry i+1 is the FC one past the paragraph's last character,
                // that is, just past its paragraph mark.
                uint endFc = OleUtil.U32(page, (i + 1) * 4);
                byte bOffset = page[rgbxAt + i * BxPapLen];
                if (!TryStyleAndGrpprlAt(page, bOffset, out ushort istd, out int grpprlStart, out int grpprlLen))
                    continue;

                _styleByEndFc[endFc] = istd;
                if (ListBindingFromGrpprl(page.AsSpan(grpprlStart, grpprlLen)) is { } binding)
                    _byEndFc[endFc] = binding;
            }
        }

        public ListBinding? BindingForParagraphEnd(uint endFc) =>
            _byEndFc.TryGetValue(endFc, out var binding) ? binding : null;

        public ushort? StyleForParagraphEnd(uint endFc) =>
            _styleByEndFc.TryGetValue(endFc, out ushort istd) ? istd : null;
    }

    /// <summary>
    /// The three structures a paragraph needs: which paragraphs are bound to a list, whether each
    /// level paints numbers or bullets, and which styles are headings.
    /// </summary>
    internal sealed class ListTables
    {
        public required ParagraphListIndex Index { get; init; }
        public required ListFormats Formats { get; init; }
        public required StyleSheet Styles { get; init; }

        public static ListTables Build(byte[] wordDoc, byte[] tableStream, int rgFcLcbOffset) => new()
        {
            Index = ParagraphListIndex.Build(wordDoc, tableStream, rgFcLcbOffset),
            Formats = ListFormats.Build(wordDoc, tableStream, rgFcLcbOffset),
            Styles = StyleSheet.Build(wordDoc, tableStream, rgFcLcbOffset),
        };

        /// <summary>Declared heading level for the paragraph whose mark ends at <paramref name="endFc"/>.</summary>
        public byte? HeadingLevelForParagraphEnd(uint endFc) =>
            Index.StyleForParagraphEnd(endFc) is { } istd ? Styles.HeadingLevel(istd) : null;

        /// <summary>List membership for the paragraph whose mark ends at <paramref name="endFc"/>.</summary>
        public (byte Level, bool Ordered)? MembershipForParagraphEnd(uint endFc) =>
            Index.BindingForParagraphEnd(endFc) is { } binding
                ? (binding.Ilvl, Formats.IsOrdered(binding.Ilfo, binding.Ilvl))
                : null;
    }
}

/// <summary>One Word paragraph: its normalized text, and what the document says it is.</summary>
internal sealed class DocParagraph
{
    public required string Content { get; init; }

    /// <summary>The automatic list this paragraph belongs to, if any.</summary>
    public (byte Level, bool Ordered)? List { get; init; }

    /// <summary>The heading level the paragraph's own style declares, if any.</summary>
    public byte? HeadingLevel { get; init; }
}
