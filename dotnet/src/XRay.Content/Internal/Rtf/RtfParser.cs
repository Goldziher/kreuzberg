// Ported from crates/xberg/src/extractors/rtf/parser.rs
// Core RTF parsing: tokenizer/control-word handling, text extraction, table and image
// detection, formatting spans, hyperlinks, footnotes, and metadata-per-paragraph.
// All byte offsets are UTF-8 byte offsets into the extracted text (matching Rust).

using System.Text;
using XRay.Content.Types;

namespace XRay.Content.Internal.Rtf;

/// <summary>Metadata for a single paragraph extracted from RTF.</summary>
internal sealed class ParagraphMeta
{
    public byte HeadingLevel;       // 1-based heading level; 0 = not a heading
    public byte? ListLevel;         // list nesting level (0-based); null = not a list item
    public ushort? ListId;          // \lsN list override id
    public bool IsTable;            // table-row placeholder
    public bool Ordered;            // ordered (numbered/lettered) list item
}

/// <summary>A formatting span tracked during RTF parsing (byte offsets into output text).</summary>
internal sealed class RtfFormattingSpan
{
    public int Start;
    public int End;
    public bool Bold;
    public bool Italic;
    public bool Underline;
    public bool Strikethrough;
    public ushort ColorIndex;
}

/// <summary>RTF formatting metadata extracted alongside text.</summary>
internal sealed class RtfFormattingData
{
    public List<RtfFormattingSpan> Spans = new();
    public List<string> ColorTable = new();
    public string? HeaderText;
    public string? FooterText;
    public List<(int Start, int End, string Url)> Hyperlinks = new();
}

/// <summary>Result bundle of <see cref="RtfParser.ExtractTextFromRtf"/>.</summary>
internal sealed class RtfTextResult
{
    public string Text = "";
    public List<Table> Tables = new();
    public List<RtfImage> Images = new();
    public List<ParagraphMeta> ParaMetas = new();
    public RtfFormattingData Formatting = new();
}

internal static class RtfParser
{
    // ------------------------------------------------------------------
    // FormattingTracker: byte-offset-aligned formatting spans.
    // ------------------------------------------------------------------
    private struct FmtState
    {
        public bool Bold;
        public bool Italic;
        public bool Underline;
        public bool Strikethrough;
        public ushort ColorIdx;
    }

    private sealed class FormattingTracker
    {
        public FmtState Fmt;
        private readonly List<FmtState> _stack = new();
        private int _spanStart;
        public readonly List<RtfFormattingSpan> Spans = new();

        public void Push() => _stack.Add(Fmt);

        public void Pop(int textOffset)
        {
            if (_stack.Count == 0) return;
            var parent = _stack[^1];
            _stack.RemoveAt(_stack.Count - 1);
            bool changed = Fmt.Bold != parent.Bold
                || Fmt.Italic != parent.Italic
                || Fmt.Underline != parent.Underline
                || Fmt.Strikethrough != parent.Strikethrough
                || Fmt.ColorIdx != parent.ColorIdx;
            if (changed)
            {
                if (textOffset > _spanStart)
                    EmitSpan(textOffset);
                _spanStart = textOffset;
                Fmt = parent;
            }
        }

        public void UpdateBold(int off, bool v) { if (v != Fmt.Bold) { CloseSpan(off); Fmt.Bold = v; } }
        public void UpdateItalic(int off, bool v) { if (v != Fmt.Italic) { CloseSpan(off); Fmt.Italic = v; } }
        public void UpdateUnderline(int off, bool v) { if (v != Fmt.Underline) { CloseSpan(off); Fmt.Underline = v; } }
        public void UpdateStrikethrough(int off, bool v) { if (v != Fmt.Strikethrough) { CloseSpan(off); Fmt.Strikethrough = v; } }
        public void UpdateColor(int off, ushort v) { if (v != Fmt.ColorIdx) { CloseSpan(off); Fmt.ColorIdx = v; } }

        public void ResetAll(int off)
        {
            if (Fmt.Bold || Fmt.Italic || Fmt.Underline || Fmt.Strikethrough || Fmt.ColorIdx != 0)
            {
                CloseSpan(off);
                Fmt = default;
            }
        }

        private void CloseSpan(int textOffset)
        {
            if (textOffset > _spanStart)
                EmitSpan(textOffset);
            _spanStart = textOffset;
        }

        public void Finalize(int textOffset)
        {
            if (textOffset > _spanStart
                && (Fmt.Bold || Fmt.Italic || Fmt.Underline || Fmt.Strikethrough || Fmt.ColorIdx != 0))
                EmitSpan(textOffset);
        }

        private void EmitSpan(int end) => Spans.Add(new RtfFormattingSpan
        {
            Start = _spanStart,
            End = end,
            Bold = Fmt.Bold,
            Italic = Fmt.Italic,
            Underline = Fmt.Underline,
            Strikethrough = Fmt.Strikethrough,
            ColorIndex = Fmt.ColorIdx,
        });

        public void RemapSpans(List<(int Old, int New)> mapping)
        {
            foreach (var span in Spans)
            {
                span.Start = RtfFormatting.MapOffset(mapping, span.Start);
                span.End = RtfFormatting.MapOffset(mapping, span.End);
            }
            Spans.RemoveAll(s => !(s.Start < s.End));
        }
    }

    // ------------------------------------------------------------------
    // Color table
    // ------------------------------------------------------------------
    private static List<string> ParseRtfColorTable(string content)
    {
        var colors = new List<string>();
        int start = content.IndexOf("{\\colortbl", StringComparison.Ordinal);
        if (start < 0) return colors;

        string rest = content.Substring(start);
        int depth = 0;
        var tableContent = new StringBuilder();
        foreach (var ch in rest)
        {
            if (ch == '{') depth += 1;
            else if (ch == '}') { depth -= 1; if (depth == 0) break; }
            if (depth > 0) tableContent.Append(ch);
        }

        string tc = tableContent.ToString();
        string tableBody = tc.StartsWith("{\\colortbl", StringComparison.Ordinal) ? tc.Substring("{\\colortbl".Length) : tc;

        foreach (var rawEntry in tableBody.Split(';'))
        {
            string entry = rawEntry.Trim();
            if (entry.Length == 0)
            {
                colors.Add("");
                continue;
            }
            byte r = 0, g = 0, b = 0;
            foreach (var rawPart in entry.Split('\\'))
            {
                string part = rawPart.Trim();
                if (part.StartsWith("red", StringComparison.Ordinal))
                    r = byte.TryParse(part.Substring(3), out var vr) ? vr : (byte)0;
                else if (part.StartsWith("green", StringComparison.Ordinal))
                    g = byte.TryParse(part.Substring(5), out var vg) ? vg : (byte)0;
                else if (part.StartsWith("blue", StringComparison.Ordinal))
                    b = byte.TryParse(part.Substring(4), out var vb) ? vb : (byte)0;
            }
            colors.Add($"#{r:x2}{g:x2}{b:x2}");
        }
        return colors;
    }

    // ------------------------------------------------------------------
    // extract_rtf_formatting: header/footer + hyperlink + span pass
    // ------------------------------------------------------------------
    private static readonly HashSet<string> FormattingSkipDests = new(StringComparer.Ordinal)
    {
        "fonttbl", "stylesheet", "info", "listtable", "listoverridetable", "generator", "filetbl",
        "revtbl", "rsidtbl", "xmlnstbl", "mmathPr", "themedata", "colorschememapping", "datastore",
        "latentstyles", "datafield", "objdata", "objclass", "panose", "bkmkstart", "bkmkend",
        "wgrffmtfilter", "fcharset", "pgdsctbl", "colortbl", "pict",
    };

    public static RtfFormattingData ExtractRtfFormatting(string content)
    {
        var colorTable = ParseRtfColorTable(content);
        var spans = new List<RtfFormattingSpan>();
        var hyperlinks = new List<(int, int, string)>();
        int textOffset = 0;
        int spanStart = 0;

        bool inHeader = false, inFooter = false;
        int headerDepth = 0, footerDepth = 0;
        var headerBuf = new StringBuilder();
        var footerBuf = new StringBuilder();

        bool inFldinst = false;
        int fldinstDepth = 0;
        var fldinstContent = new StringBuilder();
        bool inFldrslt = false;
        int fldrsltDepth = 0;
        int fldrsltStart = 0;
        string? pendingHyperlinkUrl = null;

        FmtState fmt = default;
        var fmtStack = new List<FmtState>();

        int groupDepth = 0;
        int skipDepth = 0;
        var chars = new CharCursor(content);
        bool expectDestination = false;
        bool ignorablePending = false;

        var groupHasText = new List<bool>();
        bool pendingBoundarySpace = false;

        void EmitSpanIfChanged(FmtState f)
        {
            if (textOffset > spanStart)
                spans.Add(new RtfFormattingSpan
                {
                    Start = spanStart, End = textOffset,
                    Bold = f.Bold, Italic = f.Italic, Underline = f.Underline,
                    Strikethrough = f.Strikethrough, ColorIndex = f.ColorIdx,
                });
        }

        int rtfTick = 0;
        while (true)
        {
            Cancel.Every(rtfTick++, 0xFFF);
            int ci = chars.Next();
            if (ci < 0) break;

            if (ci == '{')
            {
                groupDepth += 1;
                expectDestination = true;
                fmtStack.Add(fmt);
                groupHasText.Add(false);
                pendingBoundarySpace = false;
            }
            else if (ci == '}')
            {
                groupDepth -= 1;
                expectDestination = false;
                ignorablePending = false;
                if (fmtStack.Count > 0)
                {
                    var parent = fmtStack[^1];
                    fmtStack.RemoveAt(fmtStack.Count - 1);
                    bool changed = fmt.Bold != parent.Bold || fmt.Italic != parent.Italic
                        || fmt.Underline != parent.Underline || fmt.Strikethrough != parent.Strikethrough
                        || fmt.ColorIdx != parent.ColorIdx;
                    if (changed)
                    {
                        EmitSpanIfChanged(fmt);
                        spanStart = textOffset;
                        fmt = parent;
                    }
                }
                if (skipDepth > 0 && groupDepth < skipDepth) skipDepth = 0;
                if (inHeader && groupDepth < headerDepth) inHeader = false;
                if (inFooter && groupDepth < footerDepth) inFooter = false;
                if (inFldinst && groupDepth < fldinstDepth)
                {
                    inFldinst = false;
                    var url = ParseHyperlinkUrl(fldinstContent.ToString());
                    if (url is not null) pendingHyperlinkUrl = url;
                    fldinstContent.Clear();
                }
                if (inFldrslt && groupDepth < fldrsltDepth)
                {
                    inFldrslt = false;
                    if (pendingHyperlinkUrl is not null)
                    {
                        hyperlinks.Add((fldrsltStart, textOffset, pendingHyperlinkUrl));
                        pendingHyperlinkUrl = null;
                    }
                }
                bool producedText = PopBool(groupHasText);
                if (producedText && skipDepth == 0) pendingBoundarySpace = true;
            }
            else if (ci == '\\')
            {
                int next = chars.Peek();
                if (next == '\\' || next == '{' || next == '}')
                {
                    chars.Next();
                    expectDestination = false;
                    char nc = (char)next;
                    if (inFldinst) fldinstContent.Append(nc);
                    if (skipDepth > 0) continue;
                    if (pendingBoundarySpace && textOffset > 0) textOffset += 1;
                    pendingBoundarySpace = false;
                    textOffset += RtfChars.Utf8Len(next);
                    MarkLast(groupHasText);
                    if (inHeader) headerBuf.Append(nc);
                    if (inFooter) footerBuf.Append(nc);
                }
                else if (next == '\'')
                {
                    chars.Next();
                    expectDestination = false;
                    chars.Next();
                    chars.Next();
                    if (skipDepth > 0) continue;
                    if (pendingBoundarySpace && textOffset > 0) textOffset += 1;
                    pendingBoundarySpace = false;
                    textOffset += 1;
                    MarkLast(groupHasText);
                }
                else if (next == '*')
                {
                    chars.Next();
                    ignorablePending = true;
                }
                else
                {
                    var (word, param) = RtfEncoding.ParseRtfControlWord(chars);

                    if (expectDestination || ignorablePending)
                    {
                        expectDestination = false;

                        if (ignorablePending)
                        {
                            ignorablePending = false;
                            if (word == "fldinst")
                            {
                                inFldinst = true;
                                fldinstDepth = groupDepth;
                                if (skipDepth == 0) skipDepth = groupDepth;
                                continue;
                            }
                            if (skipDepth == 0) skipDepth = groupDepth;
                            continue;
                        }

                        if (word == "fldinst")
                        {
                            inFldinst = true;
                            fldinstDepth = groupDepth;
                            if (skipDepth == 0) skipDepth = groupDepth;
                            continue;
                        }
                        if (word == "fldrslt")
                        {
                            inFldrslt = true;
                            fldrsltDepth = groupDepth;
                            fldrsltStart = textOffset;
                            continue;
                        }

                        if (FormattingSkipDests.Contains(word))
                        {
                            if (skipDepth == 0) skipDepth = groupDepth;
                            continue;
                        }
                    }

                    if (inFldinst) fldinstContent.Append(word);
                    if (skipDepth > 0) continue;

                    switch (word)
                    {
                        case "b":
                        {
                            bool nv = (param ?? 1) != 0;
                            if (nv != fmt.Bold) { EmitSpanIfChanged(fmt); spanStart = textOffset; fmt.Bold = nv; }
                            break;
                        }
                        case "i":
                        {
                            bool nv = (param ?? 1) != 0;
                            if (nv != fmt.Italic) { EmitSpanIfChanged(fmt); spanStart = textOffset; fmt.Italic = nv; }
                            break;
                        }
                        case "ul":
                        {
                            bool nv = (param ?? 1) != 0;
                            if (nv != fmt.Underline) { EmitSpanIfChanged(fmt); spanStart = textOffset; fmt.Underline = nv; }
                            break;
                        }
                        case "ulnone":
                        {
                            if (fmt.Underline) { EmitSpanIfChanged(fmt); spanStart = textOffset; fmt.Underline = false; }
                            break;
                        }
                        case "strike":
                        {
                            bool nv = (param ?? 1) != 0;
                            if (nv != fmt.Strikethrough) { EmitSpanIfChanged(fmt); spanStart = textOffset; fmt.Strikethrough = nv; }
                            break;
                        }
                        case "cf":
                        {
                            ushort nv = (ushort)(param ?? 0);
                            if (nv != fmt.ColorIdx) { EmitSpanIfChanged(fmt); spanStart = textOffset; fmt.ColorIdx = nv; }
                            break;
                        }
                        case "plain" when fmt.Bold || fmt.Italic || fmt.Underline || fmt.Strikethrough || fmt.ColorIdx != 0:
                        {
                            EmitSpanIfChanged(fmt);
                            spanStart = textOffset;
                            fmt.Bold = fmt.Italic = fmt.Underline = fmt.Strikethrough = false;
                            fmt.ColorIdx = 0;
                            break;
                        }
                        case "header":
                        case "headerl":
                        case "headerr":
                        case "headerf":
                            inHeader = true; headerDepth = groupDepth; break;
                        case "footer":
                        case "footerl":
                        case "footerr":
                        case "footerf":
                            inFooter = true; footerDepth = groupDepth; break;
                        case "par":
                        case "line":
                            textOffset += 1;
                            if (inHeader) headerBuf.Append('\n');
                            if (inFooter) footerBuf.Append('\n');
                            break;
                        case "tab":
                            textOffset += 1; break;
                        case "bullet": textOffset += RtfChars.Utf8Len(0x2022); break;
                        case "lquote": textOffset += RtfChars.Utf8Len(0x2018); break;
                        case "rquote": textOffset += RtfChars.Utf8Len(0x2019); break;
                        case "ldblquote": textOffset += RtfChars.Utf8Len(0x201C); break;
                        case "rdblquote": textOffset += RtfChars.Utf8Len(0x201D); break;
                        case "endash": textOffset += RtfChars.Utf8Len(0x2013); break;
                        case "emdash": textOffset += RtfChars.Utf8Len(0x2014); break;
                        case "u":
                        {
                            if (param is int codeNum)
                            {
                                long codeU = codeNum < 0 ? codeNum + 65536 : codeNum;
                                if (RtfChars.IsValidScalar(codeU))
                                {
                                    textOffset += RtfChars.Utf8Len((int)codeU);
                                    if (inHeader) RtfChars.AppendCp(headerBuf, (int)codeU);
                                    if (inFooter) RtfChars.AppendCp(footerBuf, (int)codeU);
                                }
                            }
                            int pk = chars.Peek();
                            if (pk >= 0 && pk != '\\' && pk != '{' && pk != '}')
                                chars.Next();
                            break;
                        }
                    }
                }
            }
            else if (ci == '\n' || ci == '\r')
            {
                // not significant
            }
            else if (ci == ' ' || ci == '\t')
            {
                if (inFldinst) fldinstContent.Append(' ');
                if (skipDepth > 0) continue;
                if (textOffset > 0)
                {
                    textOffset += 1;
                    MarkLast(groupHasText);
                }
            }
            else
            {
                if (inFldinst) { RtfChars.AppendCp(fldinstContent, ci); continue; }
                if (skipDepth > 0) continue;
                if (pendingBoundarySpace && textOffset > 0) textOffset += 1;
                pendingBoundarySpace = false;
                textOffset += RtfChars.Utf8Len(ci);
                MarkLast(groupHasText);
                if (inHeader) RtfChars.AppendCp(headerBuf, ci);
                if (inFooter) RtfChars.AppendCp(footerBuf, ci);
            }
        }

        if (textOffset > spanStart && (fmt.Bold || fmt.Italic || fmt.Underline || fmt.Strikethrough || fmt.ColorIdx != 0))
            EmitSpanIfChanged(fmt);

        string headerTrimmed = headerBuf.ToString().Trim();
        string footerTrimmed = footerBuf.ToString().Trim();

        return new RtfFormattingData
        {
            Spans = spans,
            ColorTable = colorTable,
            HeaderText = headerTrimmed.Length == 0 ? null : headerTrimmed,
            FooterText = footerTrimmed.Length == 0 ? null : footerTrimmed,
            Hyperlinks = hyperlinks,
        };
    }

    private static string? ParseHyperlinkUrl(string fldinstContent)
    {
        string trimmed = fldinstContent.Trim();
        if (!trimmed.StartsWith("HYPERLINK", StringComparison.Ordinal))
            return null;
        string rest = trimmed.Substring("HYPERLINK".Length);
        string url = rest.Trim().Trim('"').Trim();
        if (url.StartsWith("\\l ", StringComparison.Ordinal))
            url = "#" + url.Substring(3).Trim().Trim('"');
        else if (url.StartsWith("\\l\"", StringComparison.Ordinal))
            url = "#" + url.Substring(3).Trim('"');
        return url.Length == 0 ? null : url;
    }

    // ------------------------------------------------------------------
    // spans_to_annotations
    // ------------------------------------------------------------------
    public static List<TextAnnotation> SpansToAnnotations(int paraStart, int paraEnd, RtfFormattingData formatting)
    {
        var annotations = new List<TextAnnotation>();
        foreach (var span in formatting.Spans)
        {
            if (span.End <= paraStart || span.Start >= paraEnd) continue;
            int annStart = Math.Max(span.Start, paraStart) - paraStart;
            int annEnd = Math.Min(span.End, paraEnd) - paraStart;
            if (annStart >= annEnd) continue;
            uint s = (uint)annStart;
            uint e = (uint)annEnd;
            if (span.Bold)
                annotations.Add(new TextAnnotation { Start = s, End = e, Kind = AnnotationKind.Bold });
            if (span.Italic)
                annotations.Add(new TextAnnotation { Start = s, End = e, Kind = AnnotationKind.Italic });
            if (span.Underline)
                annotations.Add(new TextAnnotation { Start = s, End = e, Kind = new AnnotationKind { Which = AnnotationKind.Tag.Underline } });
            if (span.Strikethrough)
                annotations.Add(new TextAnnotation { Start = s, End = e, Kind = new AnnotationKind { Which = AnnotationKind.Tag.Strikethrough } });
            if (span.ColorIndex > 0 && span.ColorIndex < formatting.ColorTable.Count)
            {
                string color = formatting.ColorTable[span.ColorIndex];
                if (color.Length > 0 && color != "#000000")
                    annotations.Add(new TextAnnotation { Start = s, End = e, Kind = new AnnotationKind { Which = AnnotationKind.Tag.Color, Value = color } });
            }
        }

        foreach (var (linkStart, linkEnd, url) in formatting.Hyperlinks)
        {
            if (linkEnd <= paraStart || linkStart >= paraEnd) continue;
            uint s = (uint)(Math.Max(linkStart, paraStart) - paraStart);
            uint e = (uint)(Math.Min(linkEnd, paraEnd) - paraStart);
            if (s < e)
                annotations.Add(new TextAnnotation { Start = s, End = e, Kind = new AnnotationKind { Which = AnnotationKind.Tag.Link, Url = url, Title = null } });
        }

        return annotations;
    }

    // ------------------------------------------------------------------
    // Known destinations to skip entirely (excludes field/fldinst).
    // ------------------------------------------------------------------
    private static readonly HashSet<string> SkipDestinations = new(StringComparer.Ordinal)
    {
        "fonttbl", "colortbl", "stylesheet", "info", "listtable", "listoverridetable", "generator",
        "filetbl", "revtbl", "rsidtbl", "xmlnstbl", "mmathPr", "themedata", "colorschememapping",
        "datastore", "latentstyles", "datafield", "objdata", "objclass", "panose", "bkmkstart",
        "bkmkend", "wgrffmtfilter", "fcharset", "pgdsctbl",
    };

    // ------------------------------------------------------------------
    // Mutable state shared between the main loop and handle_control_word.
    // ------------------------------------------------------------------
    private sealed class TextState
    {
        public Utf8Buf Result = new();
        public CharCursor Chars = null!;
        public TableState? Table;
        public List<Table> Tables = new();
        public List<RtfImage> Images = new();
        public bool Plain;
        public List<bool> GroupHasText = new();
        public byte CurHeadingLevel;
        public byte? CurListLevel;
        public ushort? CurListId;
        public bool CurOrdered;
        public List<ParagraphMeta> ParaMetas = new();
        public bool ParaMetaEmitted;
        public List<int> UcStack = new() { 1 };
        public int FootnoteCount;
        public bool PendingBoundarySpace;
        public List<bool> HiddenStack = new() { false };
        public FormattingTracker Fmt = new();

        /// <summary>ANSI codepage for `\'hh` escapes. RTF defaults to 1252 unless `\ansicpgN` says
        /// otherwise; scoped like the other document properties.</summary>
        public List<uint> AnsiCodepageStack = new() { 1252u };

        /// <summary>Active font id, set by `\fN`. Its `\fcharsetN` outranks `\ansicpg`.</summary>
        public List<ushort?> FontIdStack = new() { null };

        /// <summary>Document default font from `\deffN` — declared once, so not scoped.</summary>
        public ushort? DefaultFontId;

        /// <summary>Font id → Windows codepage, from the font table's `\fcharsetN`/`\cpgN`.</summary>
        public Dictionary<ushort, uint> FontCharsets = new();

        public bool Hidden => HiddenStack.Count > 0 && HiddenStack[^1];
        public int Uc => UcStack.Count > 0 ? UcStack[^1] : 1;

        /// <summary>
        /// The codepage a `\'hh` run decodes with: the active font's charset (an explicit `\fN`
        /// in scope, else the `\deffN` default), then `\ansicpg`, then RTF's default of 1252.
        /// </summary>
        public uint DecodeCodepage()
        {
            ushort? fontId = FontIdStack.Count > 0 ? FontIdStack[^1] : null;
            fontId ??= DefaultFontId;
            if (fontId is { } id && FontCharsets.TryGetValue(id, out uint fromFont)) return fromFont;
            return AnsiCodepageStack.Count > 0 ? AnsiCodepageStack[^1] : 1252u;
        }
    }

    private static void EnsureTable(TextState st)
    {
        st.Table ??= new TableState();
    }

    private static void FinalizeTable(TextState st)
    {
        if (st.Table is null) return;
        var state = st.Table;
        st.Table = null;
        var table = state.FinalizeWithFormat(st.Plain);
        if (table is not null) st.Tables.Add(table);
    }

    // ------------------------------------------------------------------
    // extract_text_from_rtf
    // ------------------------------------------------------------------
    public static RtfTextResult ExtractTextFromRtf(string content, bool plain)
    {
        var colorTable = ParseRtfColorTable(content);
        var st = new TextState { Plain = plain, Chars = new CharCursor(content) };
        st.FontCharsets = RtfEncoding.ParseFontCharsetTable(content);
        var chars = st.Chars;

        // list-marker destination tracking
        bool inListtext = false;
        int listtextDepth = 0;
        var listtextBuf = new StringBuilder();

        // hyperlink fields
        bool inFldinst = false;
        int fldinstDepth = 0;
        var fldinstContent = new StringBuilder();
        bool inFldrslt = false;
        int fldrsltDepth = 0;
        int fldrsltStart = 0;
        string? pendingHyperlinkUrl = null;
        var hyperlinks = new List<(int, int, string)>();

        // footnotes
        bool inFootnote = false;
        int footnoteDepth = 0;
        var footnoteBuf = new StringBuilder();
        var footnotes = new List<string>();

        int groupDepth = 0;
        int skipDepth = 0;
        bool ignorablePending = false;
        bool expectDestination = false;

        while (true)
        {
            int ci = chars.Next();
            if (ci < 0) break;

            if (ci == '{')
            {
                groupDepth += 1;
                expectDestination = true;
                st.GroupHasText.Add(false);
                st.UcStack.Add(st.Uc);
                st.HiddenStack.Add(st.Hidden);
                st.AnsiCodepageStack.Add(st.AnsiCodepageStack.Count > 0 ? st.AnsiCodepageStack[^1] : 1252u);
                st.FontIdStack.Add(st.FontIdStack.Count > 0 ? st.FontIdStack[^1] : null);
                st.Fmt.Push();
                st.PendingBoundarySpace = false;
            }
            else if (ci == '}')
            {
                groupDepth -= 1;
                expectDestination = false;
                ignorablePending = false;
                st.Fmt.Pop(st.Result.Len);
                if (st.UcStack.Count > 1) st.UcStack.RemoveAt(st.UcStack.Count - 1);
                if (st.HiddenStack.Count > 1) st.HiddenStack.RemoveAt(st.HiddenStack.Count - 1);
                if (st.AnsiCodepageStack.Count > 1) st.AnsiCodepageStack.RemoveAt(st.AnsiCodepageStack.Count - 1);
                if (st.FontIdStack.Count > 1) st.FontIdStack.RemoveAt(st.FontIdStack.Count - 1);
                if (skipDepth > 0 && groupDepth < skipDepth) skipDepth = 0;

                if (inListtext && groupDepth < listtextDepth)
                {
                    inListtext = false;
                    if (IsOrderedMarker(listtextBuf.ToString().Trim()))
                        st.CurOrdered = true;
                    listtextBuf.Clear();
                }
                if (inFldinst && groupDepth < fldinstDepth)
                {
                    inFldinst = false;
                    var url = ParseHyperlinkUrl(fldinstContent.ToString());
                    if (url is not null) pendingHyperlinkUrl = url;
                    fldinstContent.Clear();
                }
                if (inFldrslt && groupDepth < fldrsltDepth)
                {
                    inFldrslt = false;
                    if (pendingHyperlinkUrl is not null)
                    {
                        hyperlinks.Add((fldrsltStart, st.Result.Len, pendingHyperlinkUrl));
                        pendingHyperlinkUrl = null;
                    }
                }
                if (inFootnote && groupDepth < footnoteDepth)
                {
                    inFootnote = false;
                    string note = footnoteBuf.ToString().Trim();
                    if (note.Length > 0) footnotes.Add(note);
                    footnoteBuf.Clear();
                }

                bool producedText = PopBool(st.GroupHasText);
                if (producedText && skipDepth == 0) st.PendingBoundarySpace = true;
            }
            else if (ci == '\\')
            {
                int next = chars.Peek();
                if (next == '\n' || next == '\r')
                {
                    chars.Next();
                    if (next == '\r' && chars.Peek() == '\n') chars.Next();
                    expectDestination = false;
                    if (skipDepth > 0) continue;
                    HandleControlWord(st, "par", null);
                }
                else if (next == '\\' || next == '{' || next == '}')
                {
                    chars.Next();
                    expectDestination = false;
                    char nc = (char)next;
                    if (inFldinst) fldinstContent.Append(nc);
                    if (inFootnote) footnoteBuf.Append(nc);
                    if (skipDepth > 0) continue;
                    if (st.Hidden) continue;
                    if (st.PendingBoundarySpace && !st.Result.IsEmpty && !st.Result.EndsWith(' ') && !st.Result.EndsWith('\n'))
                        st.Result.PushChar(' ');
                    st.PendingBoundarySpace = false;
                    st.ParaMetaEmitted = false;
                    st.Result.PushChar(nc);
                    MarkLast(st.GroupHasText);
                }
                else if (next == '\'')
                {
                    chars.Next();
                    expectDestination = false;
                    int h1 = chars.Next();
                    int h2 = chars.Next();
                    byte? decodedByte = (h1 >= 0 && h2 >= 0)
                        ? RtfEncoding.ParseHexByte((byte)(h1 & 0xFF), (byte)(h2 & 0xFF))
                        : null;
                    // A run of adjacent escapes is decoded as one byte string: a multi-byte
                    // codepage such as 932 spells one character across two `\'hh` escapes, and
                    // decoding them one at a time turns it into two replacement characters.
                    List<byte>? runBytes = null;
                    if (decodedByte is byte first)
                    {
                        runBytes = new List<byte> { first };
                        while (RtfEncoding.ConsumeAdjacentHexEscape(chars) is byte more) runBytes.Add(more);
                    }
                    string? decodedRun = runBytes is null
                        ? null
                        : RtfEncoding.DecodeAnsiBytes(runBytes, st.DecodeCodepage());
                    if (inFootnote && decodedRun is not null)
                        footnoteBuf.Append(decodedRun);
                    if (skipDepth > 0) continue;
                    if (st.Hidden) continue;
                    if (decodedRun is not null)
                    {
                        if (st.Table is { InRow: true } ts)
                        {
                            ts.CurrentCell.Append(decodedRun);
                        }
                        else
                        {
                            if (st.PendingBoundarySpace && !st.Result.IsEmpty && !st.Result.EndsWith(' ') && !st.Result.EndsWith('\n'))
                                st.Result.PushChar(' ');
                            st.PendingBoundarySpace = false;
                            st.ParaMetaEmitted = false;
                            foreach (char dc in decodedRun) st.Result.PushChar(dc);
                            MarkLast(st.GroupHasText);
                        }
                    }
                }
                else if (next == '*')
                {
                    chars.Next();
                    ignorablePending = true;
                }
                else
                {
                    var (controlWord, param) = RtfEncoding.ParseRtfControlWord(chars);

                    if (expectDestination || ignorablePending)
                    {
                        expectDestination = false;

                        if (ignorablePending)
                        {
                            ignorablePending = false;
                            if (controlWord == "fldinst")
                            {
                                inFldinst = true;
                                fldinstDepth = groupDepth;
                                if (skipDepth == 0) skipDepth = groupDepth;
                                continue;
                            }
                            if (controlWord == "listtext" || controlWord == "pntext")
                            {
                                inListtext = true;
                                listtextDepth = groupDepth;
                                listtextBuf.Clear();
                                if (skipDepth == 0) skipDepth = groupDepth;
                                continue;
                            }
                            if (controlWord != "shppict")
                            {
                                if (skipDepth == 0) skipDepth = groupDepth;
                                continue;
                            }
                        }

                        if (controlWord == "listtext" || controlWord == "pntext")
                        {
                            inListtext = true;
                            listtextDepth = groupDepth;
                            listtextBuf.Clear();
                            if (skipDepth == 0) skipDepth = groupDepth;
                            continue;
                        }
                        if (controlWord == "fldinst")
                        {
                            inFldinst = true;
                            fldinstDepth = groupDepth;
                            if (skipDepth == 0) skipDepth = groupDepth;
                            continue;
                        }
                        if (controlWord == "fldrslt")
                        {
                            inFldrslt = true;
                            fldrsltDepth = groupDepth;
                            fldrsltStart = st.Result.Len;
                            continue;
                        }
                        if (controlWord == "footnote")
                        {
                            inFootnote = true;
                            footnoteDepth = groupDepth;
                            footnoteBuf.Clear();
                            if (skipDepth == 0) skipDepth = groupDepth;
                            continue;
                        }
                        if (SkipDestinations.Contains(controlWord))
                        {
                            if (skipDepth == 0) skipDepth = groupDepth;
                            continue;
                        }
                    }

                    // Codepage selection is document state, tracked whether or not the group it
                    // appears in contributes text — `\ansicpg` sits in the header, and `\deff`
                    // often does too.
                    if (controlWord == "ansicpg" && param is int cpv && cpv > 0 && st.AnsiCodepageStack.Count > 0)
                        st.AnsiCodepageStack[^1] = (uint)cpv;
                    if (controlWord == "f" && param is int fv && st.FontIdStack.Count > 0)
                        st.FontIdStack[^1] = (ushort)Math.Max(0, fv);
                    if (controlWord == "deff" && param is int dfv)
                        st.DefaultFontId = (ushort)Math.Max(0, dfv);

                    if (skipDepth > 0)
                    {
                        if (controlWord == "uc" && param is int ucv && st.UcStack.Count > 0)
                            st.UcStack[^1] = Math.Max(0, ucv);
                        if (inFootnote && controlWord == "u" && param is int codeNum)
                        {
                            long codeU = codeNum < 0 ? codeNum + 65536 : codeNum;
                            if (RtfChars.IsValidScalar(codeU))
                                RtfChars.AppendCp(footnoteBuf, (int)codeU);
                            int ucCount = st.Uc;
                            for (int k = 0; k < ucCount; k++)
                            {
                                int pk = chars.Peek();
                                if (pk >= 0 && pk != '\\' && pk != '{' && pk != '}')
                                    chars.Next();
                            }
                        }
                        if (inFootnote && (controlWord == "par" || controlWord == "line"))
                            footnoteBuf.Append(' ');
                        continue;
                    }

                    HandleControlWord(st, controlWord, param);
                }
            }
            else if (ci == '\n' || ci == '\r')
            {
                // source line breaks are insignificant
            }
            else if (ci == ' ' || ci == '\t')
            {
                if (inFldinst) fldinstContent.Append(' ');
                if (inFootnote) footnoteBuf.Append(' ');
                if (skipDepth > 0 && !inFootnote) continue;
                if (inFootnote) continue;
                if (st.Table is { InRow: true } ts)
                {
                    if (!ts.CurrentCellEndsWith(' '))
                        ts.CurrentCell.Append(' ');
                }
                else if (!st.Result.IsEmpty && !st.Result.EndsWith(' ') && !st.Result.EndsWith('\n'))
                {
                    st.Result.PushChar(' ');
                    MarkLast(st.GroupHasText);
                }
            }
            else
            {
                expectDestination = false;
                if (inFldinst) RtfChars.AppendCp(fldinstContent, ci);
                if (inFootnote) RtfChars.AppendCp(footnoteBuf, ci);
                if (inListtext) RtfChars.AppendCp(listtextBuf, ci);
                if (skipDepth > 0) continue;
                if (st.Hidden) continue;
                if (st.Table is { InRow: false, Rows.Count: > 0 })
                    FinalizeTable(st);
                if (st.Table is { InRow: true } ts)
                {
                    RtfChars.AppendCp(ts.CurrentCell, ci);
                }
                else
                {
                    if (st.PendingBoundarySpace && !st.Result.IsEmpty && !st.Result.EndsWith(' ') && !st.Result.EndsWith('\n'))
                        st.Result.PushChar(' ');
                    st.PendingBoundarySpace = false;
                    st.ParaMetaEmitted = false;
                    st.Result.PushCp(ci);
                    MarkLast(st.GroupHasText);
                }
            }
        }

        if (st.Table is not null)
            FinalizeTable(st);

        st.Fmt.Finalize(st.Result.Len);

        var (normalized, mapping) = RtfFormatting.NormalizeWhitespaceWithMapping(st.Result.ToString());
        string finalText = normalized.TrimEnd();
        if (finalText.Length > 0)
        {
            int paraCount = CountNonEmptyParagraphs(normalized);
            while (st.ParaMetas.Count < paraCount)
            {
                st.ParaMetas.Add(new ParagraphMeta
                {
                    HeadingLevel = st.CurHeadingLevel,
                    ListLevel = st.CurListLevel,
                    ListId = st.CurListId,
                    IsTable = false,
                    Ordered = st.CurOrdered,
                });
            }
        }

        var finalResult = new StringBuilder(normalized);
        if (footnotes.Count > 0)
        {
            if (!(normalized.Length > 0 && normalized[^1] == '\n'))
            {
                finalResult.Append('\n');
                finalResult.Append('\n');
            }
            for (int i = 0; i < footnotes.Count; i++)
            {
                finalResult.Append($"[^{i + 1}]: {footnotes[i].Trim()}");
                finalResult.Append('\n');
                finalResult.Append('\n');
            }
        }

        st.Fmt.RemapSpans(mapping);

        var remappedLinks = new List<(int, int, string)>();
        foreach (var (s0, e0, url) in hyperlinks)
        {
            int ns = RtfFormatting.MapOffset(mapping, s0);
            int ne = RtfFormatting.MapOffset(mapping, e0);
            if (ns < ne) remappedLinks.Add((ns, ne, url));
        }

        return new RtfTextResult
        {
            Text = finalResult.ToString(),
            Tables = st.Tables,
            Images = st.Images,
            ParaMetas = st.ParaMetas,
            Formatting = new RtfFormattingData
            {
                Spans = st.Fmt.Spans,
                ColorTable = colorTable,
                HeaderText = null,
                FooterText = null,
                Hyperlinks = remappedLinks,
            },
        };
    }

    // ------------------------------------------------------------------
    // handle_control_word
    // ------------------------------------------------------------------
    private static void HandleControlWord(TextState st, string controlWord, int? param)
    {
        var chars = st.Chars;
        switch (controlWord)
        {
            case "v":
            {
                bool hidden = (param ?? 1) != 0;
                if (st.HiddenStack.Count > 0) st.HiddenStack[^1] = hidden;
                break;
            }
            case "pard":
            {
                bool inTableRow = st.Table is { InRow: true };
                if (!inTableRow)
                {
                    if (!st.Result.IsEmpty && !st.Result.EndsWith('\n') && !st.ParaMetaEmitted)
                    {
                        st.ParaMetas.Add(new ParagraphMeta
                        {
                            HeadingLevel = st.CurHeadingLevel,
                            ListLevel = st.CurListLevel,
                            ListId = st.CurListId,
                            IsTable = false,
                            Ordered = st.CurOrdered,
                        });
                        st.Result.PushChar('\n');
                        st.Result.PushChar('\n');
                        MarkLast(st.GroupHasText);
                    }
                }
                st.ParaMetaEmitted = false;
                st.CurHeadingLevel = 0;
                st.CurListLevel = null;
                st.CurListId = null;
                st.CurOrdered = false;
                break;
            }
            case "outlinelevel":
                if (param is int lvl) st.CurHeadingLevel = (byte)(lvl + 1);
                break;
            case "ilvl":
                st.CurListLevel = (byte)(param ?? 0);
                break;
            case "ls":
                st.CurListId = (ushort)(param ?? 0);
                break;
            case "uc":
                if (param is int ucv && st.UcStack.Count > 0)
                    st.UcStack[^1] = Math.Max(0, ucv);
                break;
            case "u":
            {
                if (param is int codeNum)
                {
                    long codeU = codeNum < 0 ? codeNum + 65536 : codeNum;
                    if (RtfChars.IsValidScalar(codeU))
                    {
                        int cp = (int)codeU;
                        if (st.Table is { InRow: true } ts)
                        {
                            RtfChars.AppendCp(ts.CurrentCell, cp);
                        }
                        else
                        {
                            if (st.PendingBoundarySpace && !st.Result.IsEmpty && !st.Result.EndsWith(' ') && !st.Result.EndsWith('\n'))
                                st.Result.PushChar(' ');
                            st.PendingBoundarySpace = false;
                            st.Result.PushCp(cp);
                            MarkLast(st.GroupHasText);
                        }
                    }
                    int ucCount = st.Uc;
                    int skipped = 0;
                    while (skipped < ucCount)
                    {
                        int nxt = chars.Peek();
                        if (nxt < 0) break;
                        if (nxt == '\\')
                        {
                            chars.Next(); // consume '\'
                            int apos = chars.Peek();
                            if (apos < 0) break;
                            if (apos == '\'')
                            {
                                chars.Next(); // '\''
                                chars.Next(); // hex1
                                chars.Next(); // hex2
                                skipped += 1;
                                continue;
                            }
                            break;
                        }
                        else if (nxt == '{' || nxt == '}')
                        {
                            break;
                        }
                        else
                        {
                            chars.Next();
                            skipped += 1;
                        }
                    }
                }
                break;
            }
            case "chftn":
            {
                st.FootnoteCount += 1;
                string marker = $"[^{st.FootnoteCount}]";
                if (st.Table is { InRow: true } ts)
                {
                    ts.CurrentCell.Append(marker);
                }
                else
                {
                    st.Result.PushStr(marker);
                    MarkLast(st.GroupHasText);
                }
                break;
            }
            case "pict":
            {
                var (imageMetadata, rtfImage) = RtfImages.ExtractPictImage(chars);
                if (rtfImage is not null) st.Images.Add(rtfImage);
                if (imageMetadata.Length > 0 && !st.Plain)
                {
                    string imgMd = $"![image]({imageMetadata}) ";
                    if (st.Table is { InRow: true } ts)
                    {
                        ts.CurrentCell.Append(imgMd);
                    }
                    else
                    {
                        MarkLast(st.GroupHasText);
                        st.Result.PushStr(imgMd);
                    }
                }
                break;
            }
            case "par":
            case "line":
            {
                st.PendingBoundarySpace = false;
                bool inTableRow = st.Table is { InRow: true };
                if (inTableRow)
                {
                    var ts = st.Table!;
                    if (ts.CurrentCell.Length != 0 && !ts.CurrentCellEndsWith(' '))
                        ts.CurrentCell.Append(' ');
                }
                else
                {
                    bool stillInTable = st.Table is { ExpectingNextRow: true };
                    if (st.Table is not null && !stillInTable)
                        FinalizeTable(st);
                    if (!st.Result.IsEmpty && !st.Result.EndsWith('\n'))
                    {
                        if (!st.ParaMetaEmitted)
                        {
                            st.ParaMetas.Add(new ParagraphMeta
                            {
                                HeadingLevel = st.CurHeadingLevel,
                                ListLevel = st.CurListLevel,
                                ListId = st.CurListId,
                                IsTable = false,
                                Ordered = st.CurOrdered,
                            });
                            st.ParaMetaEmitted = true;
                        }
                        st.Result.PushChar('\n');
                        st.Result.PushChar('\n');
                    }
                    MarkLast(st.GroupHasText);
                }
                break;
            }
            case "tab":
                if (st.Table is { InRow: true } tabTs)
                {
                    tabTs.CurrentCell.Append('\t');
                }
                else
                {
                    st.Result.PushChar('\t');
                    MarkLast(st.GroupHasText);
                }
                break;
            case "bullet": st.Result.PushCp(0x2022); MarkLast(st.GroupHasText); break;
            case "lquote": st.Result.PushCp(0x2018); MarkLast(st.GroupHasText); break;
            case "rquote": st.Result.PushCp(0x2019); MarkLast(st.GroupHasText); break;
            case "ldblquote": st.Result.PushCp(0x201C); MarkLast(st.GroupHasText); break;
            case "rdblquote": st.Result.PushCp(0x201D); MarkLast(st.GroupHasText); break;
            case "endash": st.Result.PushCp(0x2013); MarkLast(st.GroupHasText); break;
            case "emdash": st.Result.PushCp(0x2014); MarkLast(st.GroupHasText); break;
            case "trowd":
                EnsureTable(st);
                st.Table!.StartRow();
                break;
            case "cell":
                if (st.Table is { InRow: true } cellTs)
                    cellTs.PushCell();
                break;
            case "row":
            {
                EnsureTable(st);
                var ts = st.Table!;
                if (ts.InRow || ts.CurrentCell.Length != 0)
                    ts.PushRow();
                if (!st.Result.IsEmpty && !st.Result.EndsWith('\n'))
                {
                    st.Result.PushChar('\n');
                    st.Result.PushChar('\n');
                }
                st.Result.PushStr("[TABLE_ROW]");
                st.Result.PushChar('\n');
                st.Result.PushChar('\n');
                MarkLast(st.GroupHasText);
                st.ParaMetaEmitted = true;
                st.ParaMetas.Add(new ParagraphMeta { IsTable = true });
                break;
            }
            case "intbl":
                EnsureTable(st);
                if (st.Table is { InRow: false } intblTs)
                    intblTs.StartRow();
                break;
            case "b":
                st.Fmt.UpdateBold(st.Result.Len, (param ?? 1) != 0);
                break;
            case "i":
                st.Fmt.UpdateItalic(st.Result.Len, (param ?? 1) != 0);
                break;
            case "ul":
                st.Fmt.UpdateUnderline(st.Result.Len, (param ?? 1) != 0);
                break;
            case "ulnone":
                st.Fmt.UpdateUnderline(st.Result.Len, false);
                break;
            case "strike":
                st.Fmt.UpdateStrikethrough(st.Result.Len, (param ?? 1) != 0);
                break;
            case "cf":
                st.Fmt.UpdateColor(st.Result.Len, (ushort)(param ?? 0));
                break;
            case "plain":
                if (st.HiddenStack.Count > 0) st.HiddenStack[^1] = false;
                st.Fmt.ResetAll(st.Result.Len);
                break;
        }
    }

    // ------------------------------------------------------------------
    // helpers
    // ------------------------------------------------------------------
    private static void MarkLast(List<bool> flags)
    {
        if (flags.Count > 0) flags[^1] = true;
    }

    private static bool PopBool(List<bool> flags)
    {
        if (flags.Count == 0) return false;
        bool v = flags[^1];
        flags.RemoveAt(flags.Count - 1);
        return v;
    }

    private static bool IsOrderedMarker(string lt)
    {
        string? prefix = null;
        if (lt.EndsWith('.')) prefix = lt.Substring(0, lt.Length - 1);
        else if (lt.EndsWith(')')) prefix = lt.Substring(0, lt.Length - 1);
        if (prefix is null) return false;
        string p = prefix.Trim();
        if (p.Length == 0) return false;
        if (p.All(c => c >= '0' && c <= '9')) return true;
        if (p.All(c => (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'))) return true;
        return false;
    }

    private static int CountNonEmptyParagraphs(string text)
    {
        int count = 0;
        int start = 0;
        int idx;
        while ((idx = text.IndexOf("\n\n", start, StringComparison.Ordinal)) >= 0)
        {
            if (text.Substring(start, idx - start).Trim().Length > 0) count++;
            start = idx + 2;
        }
        if (text.Substring(start).Trim().Length > 0) count++;
        return count;
    }
}
