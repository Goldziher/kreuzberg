// Port of pdf_oxide 0.3.77 `src/content/parser.rs` — the content-stream
// tokenizer plus `parse_content_stream`, `parse_content_stream_paths_only`,
// `parse_content_stream_text_only`, `parse_and_execute_text_only`,
// `parse_content_stream_images_only`, and their supporting scanners
// (`scan_graphics_region`, `parse_text_operator_fast`, `parse_inline_image`,
// the prescan/`forward_scan_ctm` path and the raw skip helpers).
//
// Rust threads byte slices through every helper; this port threads absolute
// indices into the same array instead, so `-1` stands in for the `None` /
// `Err` returns of the Rust originals.
//
// The generic operand reader replaces `crate::parser::parse_object` +
// `crate::lexer::token`, which are not part of the content module but are the
// tokenizer the content parser calls into.
using System.Globalization;

namespace XRay.Content.Internal.PdfOxide.Content;

internal static class OxContentParser
{
    /// <summary>
    /// Default cap on operators parsed from one content stream. Bounds the cost
    /// of pathological inputs (e.g. Isartor 6.1.12).
    /// </summary>
    internal const int MaxOperators = 1_000_000;

    /// <summary>
    /// Maximum consecutive byte-skips before bailing out. Skipping this many
    /// bytes without finding a valid operator means the remainder is junk, not
    /// a parseable content stream.
    /// </summary>
    internal const int MaxConsecutiveErrors = 1024;

    /// <summary>0 means "use <see cref="MaxOperators"/>". Written from one thread while
    /// extraction may run on another, so accessed through Interlocked/Volatile.</summary>
    private static int _maxOperatorsOverride;

    /// <summary>
    /// Override the global operator cap; null restores the default. Returns the
    /// previous override, or null if the default was active. Raise it only for
    /// trusted inputs — large technical PDFs can legitimately exceed 1,000,000
    /// operators in a single stream.
    /// </summary>
    internal static int? SetMaxOpsPerStream(int? limit)
    {
        int previous = Interlocked.Exchange(ref _maxOperatorsOverride, limit ?? 0);
        return previous == 0 ? null : previous;
    }

    internal static int EffectiveMaxOperators
    {
        get
        {
            int over = Volatile.Read(ref _maxOperatorsOverride);
            return over == 0 ? MaxOperators : over;
        }
    }

    // ══ Entry points ═══════════════════════════════════════════════════════

    /// <summary>
    /// Parse a content stream into a sequence of operators. Content streams use
    /// postfix notation, so operands precede their operator.
    /// </summary>
    internal static List<OxOperator> ParseContentStream(byte[] data)
    {
        var operators = new List<OxOperator>(Math.Min(data.Length / 20, 100_000));
        int i = 0;
        int consecutiveErrors = 0;
        int cap = EffectiveMaxOperators;

        int cancelTick = 0;
        while (i < data.Length)
        {
            Cancel.Every(cancelTick++, 0xFF);
            i = SkipMultispace(data, i);
            if (i >= data.Length)
            {
                break;
            }

            if (TryParseOperatorWithOperands(data, i, out int next, out OxOperator? op))
            {
                operators.Add(op!);
                i = next;
                consecutiveErrors = 0;

                if (operators.Count >= cap)
                {
                    break;
                }
            }
            else
            {
                consecutiveErrors++;
                if (consecutiveErrors >= MaxConsecutiveErrors)
                {
                    break;
                }

                // Skipping the offending byte is what keeps malformed streams
                // yielding their remaining good operators instead of aborting.
                if (data.Length - i > 1)
                {
                    i++;
                }
                else
                {
                    break;
                }
            }
        }

        return operators;
    }

    /// <summary>
    /// Parse a content stream for path extraction, skipping BT/ET text blocks
    /// and parsing common path/state/colour operators straight from the bytes.
    /// </summary>
    internal static List<OxOperator> ParseContentStreamPathsOnly(byte[] data)
    {
        var operators = new List<OxOperator>(Math.Min(data.Length / 20, 100_000));
        int len = data.Length;
        int i = 0;
        int operandStart = 0;
        int consecutiveErrors = 0;
        int cap = EffectiveMaxOperators;

        while (true)
        {
            while (i < len && ByteClass[data[i]] == ScanSkip)
            {
                i++;
            }

            if (i >= len)
            {
                break;
            }

            if (operators.Count >= cap)
            {
                break;
            }

            switch (ByteClass[data[i]])
            {
                case ScanAlpha:
                {
                    byte firstByte = data[i];
                    bool secondIsNonAlpha = i + 1 >= len || ByteClass[data[i + 1]] != ScanAlpha;

                    if (secondIsNonAlpha)
                    {
                        OxOperator? op = null;
                        bool alreadyConsumed = false;
                        switch (firstByte)
                        {
                            case (byte)'S': op = OxOperator.Stroke.Instance; break;
                            case (byte)'n': op = OxOperator.EndPath.Instance; break;
                            case (byte)'h': op = OxOperator.ClosePath.Instance; break;
                            case (byte)'q': op = OxOperator.SaveState.Instance; break;
                            case (byte)'Q': op = OxOperator.RestoreState.Instance; break;
                            case (byte)'m':
                                op = ParseFloats(data, operandStart, i, 2, out float[] m)
                                    ? new OxOperator.MoveTo(m[0], m[1]) : null;
                                break;
                            case (byte)'l':
                                op = ParseFloats(data, operandStart, i, 2, out float[] l)
                                    ? new OxOperator.LineTo(l[0], l[1]) : null;
                                break;
                            case (byte)'c':
                                op = ParseFloats(data, operandStart, i, 6, out float[] c)
                                    ? new OxOperator.CurveTo(c[0], c[1], c[2], c[3], c[4], c[5]) : null;
                                break;
                            case (byte)'v':
                                op = ParseFloats(data, operandStart, i, 4, out float[] v)
                                    ? new OxOperator.CurveToV(v[0], v[1], v[2], v[3]) : null;
                                break;
                            case (byte)'y':
                                op = ParseFloats(data, operandStart, i, 4, out float[] y)
                                    ? new OxOperator.CurveToY(y[0], y[1], y[2], y[3]) : null;
                                break;
                            case (byte)'w':
                                op = ParseFloats(data, operandStart, i, 1, out float[] w)
                                    ? new OxOperator.SetLineWidth(w[0]) : null;
                                break;
                            case (byte)'J':
                                op = ParseFloats(data, operandStart, i, 1, out float[] jc)
                                    ? new OxOperator.SetLineCap((byte)jc[0]) : null;
                                break;
                            case (byte)'j':
                                op = ParseFloats(data, operandStart, i, 1, out float[] jj)
                                    ? new OxOperator.SetLineJoin((byte)jj[0]) : null;
                                break;
                            case (byte)'M':
                                op = ParseFloats(data, operandStart, i, 1, out float[] ml)
                                    ? new OxOperator.SetMiterLimit(ml[0]) : null;
                                break;
                            case (byte)'g':
                                op = ParseFloats(data, operandStart, i, 1, out float[] gf)
                                    ? new OxOperator.SetFillGray(gf[0]) : null;
                                break;
                            case (byte)'G':
                                op = ParseFloats(data, operandStart, i, 1, out float[] gs)
                                    ? new OxOperator.SetStrokeGray(gs[0]) : null;
                                break;
                            case (byte)'f':
                            case (byte)'F': op = OxOperator.Fill.Instance; break;
                            case (byte)'B': op = OxOperator.FillStroke.Instance; break;
                            case (byte)'b': op = OxOperator.CloseFillStroke.Instance; break;
                            case (byte)'s':
                                // `s` = close path then stroke; no single variant models it.
                                operators.Add(OxOperator.ClosePath.Instance);
                                op = OxOperator.Stroke.Instance;
                                break;
                            case (byte)'W': op = OxOperator.ClipNonZero.Instance; break;
                            case (byte)'i':
                                // Flatness carries an operand but no path effect.
                                operandStart = i + 1;
                                i++;
                                consecutiveErrors = 0;
                                alreadyConsumed = true;
                                break;
                        }

                        if (alreadyConsumed)
                        {
                            continue;
                        }

                        if (op is not null)
                        {
                            operators.Add(op);
                            i++;
                            operandStart = i;
                            consecutiveErrors = 0;
                            continue;
                        }
                    }

                    int opStart = i;
                    while (i < len && IsOperatorNameByte(data[i]))
                    {
                        i++;
                    }

                    if (Matches(data, opStart, i, "true") || Matches(data, opStart, i, "false") || Matches(data, opStart, i, "null"))
                    {
                        consecutiveErrors = 0;
                        continue;
                    }

                    consecutiveErrors = 0;

                    if (Matches(data, opStart, i, "BT"))
                    {
                        int afterEt = ScanToEt(data, i);
                        if (afterEt < 0)
                        {
                            i = len;
                            break;
                        }

                        i = afterEt;
                        operandStart = i;
                        continue;
                    }

                    OxOperator? fastOp = null;
                    bool skipOp = false;
                    if (Matches(data, opStart, i, "cm"))
                    {
                        fastOp = ParseFloats(data, operandStart, opStart, 6, out float[] cm)
                            ? new OxOperator.Cm(cm[0], cm[1], cm[2], cm[3], cm[4], cm[5]) : null;
                    }
                    else if (Matches(data, opStart, i, "re"))
                    {
                        fastOp = ParseFloats(data, operandStart, opStart, 4, out float[] re)
                            ? new OxOperator.Rectangle(re[0], re[1], re[2], re[3]) : null;
                    }
                    else if (Matches(data, opStart, i, "rg"))
                    {
                        fastOp = ParseFloats(data, operandStart, opStart, 3, out float[] rg)
                            ? new OxOperator.SetFillRgb(rg[0], rg[1], rg[2]) : null;
                    }
                    else if (Matches(data, opStart, i, "RG"))
                    {
                        fastOp = ParseFloats(data, operandStart, opStart, 3, out float[] sg)
                            ? new OxOperator.SetStrokeRgb(sg[0], sg[1], sg[2]) : null;
                    }
                    else if (Matches(data, opStart, i, "k"))
                    {
                        fastOp = ParseFloats(data, operandStart, opStart, 4, out float[] fk)
                            ? new OxOperator.SetFillCmyk(fk[0], fk[1], fk[2], fk[3]) : null;
                    }
                    else if (Matches(data, opStart, i, "K"))
                    {
                        fastOp = ParseFloats(data, operandStart, opStart, 4, out float[] sk)
                            ? new OxOperator.SetStrokeCmyk(sk[0], sk[1], sk[2], sk[3]) : null;
                    }
                    else if (Matches(data, opStart, i, "f*"))
                    {
                        fastOp = OxOperator.FillEvenOdd.Instance;
                    }
                    else if (Matches(data, opStart, i, "B*"))
                    {
                        fastOp = OxOperator.FillStrokeEvenOdd.Instance;
                    }
                    else if (Matches(data, opStart, i, "b*"))
                    {
                        fastOp = OxOperator.CloseFillStrokeEvenOdd.Instance;
                    }
                    else if (Matches(data, opStart, i, "W*"))
                    {
                        fastOp = OxOperator.ClipEvenOdd.Instance;
                    }
                    else if (IsPathIrrelevantOp(data, opStart, i))
                    {
                        skipOp = true;
                    }

                    if (skipOp)
                    {
                        operandStart = i;
                        continue;
                    }

                    if (fastOp is not null)
                    {
                        operators.Add(fastOp);
                        operandStart = i;
                        continue;
                    }

                    // Slow path for Do, gs, d, BI/ID/EI and friends.
                    if (TryParseOperatorWithOperands(data, operandStart, out int next, out OxOperator? slowOp))
                    {
                        operators.Add(slowOp!);
                        i = next;
                        operandStart = i;
                    }
                    else
                    {
                        operandStart = i;
                    }

                    break;
                }

                case ScanParen:
                {
                    int end = SkipLiteralStringRaw(data, i);
                    if (end >= 0)
                    {
                        i = end;
                        consecutiveErrors = 0;
                    }
                    else
                    {
                        i++;
                        consecutiveErrors++;
                    }

                    break;
                }

                case ScanAngle:
                {
                    if (i + 1 < len && data[i + 1] == (byte)'<')
                    {
                        // Dictionaries do not appear as bare path operands.
                        i += 2;
                    }
                    else
                    {
                        int end = SkipHexStringRaw(data, i);
                        if (end >= 0)
                        {
                            i = end;
                            consecutiveErrors = 0;
                        }
                        else
                        {
                            i++;
                            consecutiveErrors++;
                        }
                    }

                    break;
                }

                case ScanBracket:
                {
                    i++;
                    uint depth = 1;
                    while (i < len && depth > 0)
                    {
                        byte b = data[i];
                        if (b == (byte)'[')
                        {
                            depth++;
                        }
                        else if (b == (byte)']')
                        {
                            depth--;
                        }
                        else if (b == (byte)'(')
                        {
                            int end = SkipLiteralStringRaw(data, i);
                            if (end >= 0)
                            {
                                i = end;
                                continue;
                            }
                        }

                        i++;
                    }

                    break;
                }

                case ScanSlash:
                    i = SkipNameRaw(data, i);
                    break;

                case ScanPercent:
                    while (i < len && data[i] != (byte)'\n' && data[i] != (byte)'\r')
                    {
                        i++;
                    }

                    break;

                default:
                    i++;
                    consecutiveErrors++;
                    if (consecutiveErrors >= MaxConsecutiveErrors)
                    {
                        return operators;
                    }

                    break;
            }
        }

        return operators;
    }

    /// <summary>
    /// Parse a content stream for text extraction, skipping pure graphics
    /// operators. Inside BT/ET blocks this is identical to
    /// <see cref="ParseContentStream"/>; outside them operands are skipped at
    /// the byte level.
    /// </summary>
    internal static List<OxOperator> ParseContentStreamTextOnly(byte[] data)
    {
        var operators = new List<OxOperator>(Math.Min(data.Length / 40, 50_000));
        int i = 0;
        int consecutiveErrors = 0;
        bool insideText = false;
        int cap = EffectiveMaxOperators;

        int cancelTick = 0;
        while (i < data.Length)
        {
            Cancel.Every(cancelTick++, 0xFF);
            i = SkipMultispace(data, i);
            if (i >= data.Length)
            {
                break;
            }

            if (operators.Count >= cap)
            {
                break;
            }

            if (insideText)
            {
                if (TryParseOperatorWithOperands(data, i, out int next, out OxOperator? op))
                {
                    if (op is OxOperator.EndText)
                    {
                        insideText = false;
                    }

                    operators.Add(op!);
                    i = next;
                    consecutiveErrors = 0;
                }
                else
                {
                    consecutiveErrors++;
                    if (consecutiveErrors >= MaxConsecutiveErrors)
                    {
                        break;
                    }

                    if (data.Length - i > 1)
                    {
                        i++;
                    }
                    else
                    {
                        break;
                    }
                }

                continue;
            }

            ScanResult scan = ScanGraphicsRegion(data, i, ref consecutiveErrors);
            switch (scan.Kind)
            {
                case ScanKind.EndOfData:
                    return operators;

                case ScanKind.FoundBt:
                    operators.Add(OxOperator.BeginText.Instance);
                    i = scan.Rest;
                    insideText = true;
                    break;

                case ScanKind.InlineImage:
                    i = TryParseInlineImage(data, scan.Rest, out int afterImage, out _) ? afterImage : scan.Rest;
                    break;

                case ScanKind.NeedFullParse:
                    if (TryParseOperatorWithOperands(data, scan.OperandStart, out int nextFull, out OxOperator? fullOp))
                    {
                        operators.Add(fullOp!);
                        i = nextFull;
                    }
                    else
                    {
                        i = scan.AfterOp;
                    }

                    break;

                case ScanKind.DeferredThenText:
                {
                    // Re-parse the deferred q/cm/Q region so CTM-affecting operators
                    // survive. The trigger itself is left for the next iteration,
                    // which re-enters the scanner and returns it.
                    int remaining = scan.DeferredStart;
                    while (remaining < scan.TriggerStart)
                    {
                        if (TryParseOperatorWithOperands(data, remaining, out int nextDeferred, out OxOperator? deferredOp))
                        {
                            operators.Add(deferredOp!);
                            remaining = nextDeferred;
                        }
                        else if (data.Length - remaining > 1)
                        {
                            remaining++;
                        }
                        else
                        {
                            break;
                        }
                    }

                    i = scan.TriggerStart;
                    consecutiveErrors = 0;
                    break;
                }

                case ScanKind.SimpleOp:
                    operators.Add(scan.Op!);
                    i = scan.Rest;
                    break;

                default:
                    return operators;
            }
        }

        return operators;
    }

    /// <summary>
    /// Streaming text-only parser: operators are handed to <paramref name="handler"/>
    /// as they are parsed, with no intermediate operator list (which reaches
    /// tens of megabytes on graphics-heavy pages). Returning false from the
    /// handler stops parsing, mirroring the Rust handler's error return.
    /// </summary>
    internal static void ParseAndExecuteTextOnly(byte[] data, Func<OxOperator, bool> handler)
    {
        // Above 256 KB the prescan locates the text-bearing regions instead of
        // walking megabytes of path/colour operators byte by byte.
        if (data.Length > 256 * 1024)
        {
            PrescanResult? prescan = PrescanTextRegions(data);
            if (prescan is not null)
            {
                if (prescan.Regions.Count == 0)
                {
                    return;
                }

                for (int r = 0; r < prescan.Regions.Count; r++)
                {
                    (int start, int end) = prescan.Regions[r];
                    if (prescan.RegionStates is null)
                    {
                        if (!ParseRegionTextOnly(Slice(data, start, end), handler))
                        {
                            return;
                        }

                        continue;
                    }

                    // Inject the graphics state the forward scan recorded, wrapped in
                    // q/Q so one region's state cannot leak into the next.
                    PrescanState state = prescan.RegionStates[r];
                    if (!handler(OxOperator.SaveState.Instance))
                    {
                        return;
                    }

                    if (!handler(new OxOperator.Cm(state.A, state.B, state.C, state.D, state.E, state.F)))
                    {
                        return;
                    }

                    // BT blocks that inherit Tf from an earlier scope need it re-issued.
                    if (state.FontName is not null && !handler(new OxOperator.Tf(state.FontName, state.FontSize)))
                    {
                        return;
                    }

                    if (!ParseRegionTextOnly(Slice(data, start, end), handler))
                    {
                        return;
                    }

                    if (!handler(OxOperator.RestoreState.Instance))
                    {
                        return;
                    }
                }

                return;
            }
        }

        RunTextOnly(data, handler, EffectiveMaxOperators);
    }

    /// <summary>
    /// Image-only parser: skips BT/ET blocks entirely and fully parses only
    /// what image extraction needs (cm, q, Q, Do, BI/ID/EI).
    /// </summary>
    internal static List<OxOperator> ParseContentStreamImagesOnly(byte[] data)
    {
        var operators = new List<OxOperator>(256);
        int i = 0;
        int consecutiveErrors = 0;
        bool insideText = false;
        int cap = EffectiveMaxOperators;

        int cancelTick = 0;
        while (i < data.Length)
        {
            Cancel.Every(cancelTick++, 0xFF);
            i = SkipMultispace(data, i);
            if (i >= data.Length)
            {
                break;
            }

            if (operators.Count >= cap)
            {
                break;
            }

            if (insideText)
            {
                int afterEt = ScanToEt(data, i);
                if (afterEt < 0)
                {
                    break;
                }

                i = afterEt;
                insideText = false;
                consecutiveErrors = 0;
                continue;
            }

            ScanResult scan = ScanGraphicsRegion(data, i, ref consecutiveErrors);
            switch (scan.Kind)
            {
                case ScanKind.EndOfData:
                    return operators;

                case ScanKind.FoundBt:
                    i = scan.Rest;
                    insideText = true;
                    break;

                case ScanKind.InlineImage:
                    if (TryParseInlineImage(data, scan.Rest, out int afterImage, out OxOperator? image))
                    {
                        operators.Add(image!);
                        i = afterImage;
                    }
                    else
                    {
                        i = scan.Rest;
                    }

                    break;

                case ScanKind.NeedFullParse:
                    if (TryParseOperatorWithOperands(data, scan.OperandStart, out int nextFull, out OxOperator? fullOp))
                    {
                        operators.Add(fullOp!);
                        i = nextFull;
                    }
                    else
                    {
                        i = scan.AfterOp;
                    }

                    break;

                case ScanKind.DeferredThenText:
                {
                    int remaining = scan.DeferredStart;
                    while (remaining < scan.TriggerStart)
                    {
                        if (TryParseOperatorWithOperands(data, remaining, out int nextDeferred, out OxOperator? deferredOp))
                        {
                            operators.Add(deferredOp!);
                            remaining = nextDeferred;
                        }
                        else if (data.Length - remaining > 1)
                        {
                            remaining++;
                        }
                        else
                        {
                            break;
                        }
                    }

                    i = scan.TriggerStart;
                    consecutiveErrors = 0;
                    break;
                }

                case ScanKind.SimpleOp:
                    operators.Add(scan.Op!);
                    i = scan.Rest;
                    break;

                default:
                    return operators;
            }
        }

        return operators;
    }

    // ══ Streaming text-only core ═══════════════════════════════════════════

    /// <summary>Shared body of the streaming text-only walk. Returns false when the
    /// handler asked to stop.</summary>
    private static bool RunTextOnly(byte[] data, Func<OxOperator, bool> handler, int cap)
    {
        int i = 0;
        int consecutiveErrors = 0;
        bool insideText = false;
        int opCount = 0;

        int cancelTick3 = 0;
        while (i < data.Length)
        {
            Cancel.Every(cancelTick3++, 0xFF);
            while (i < data.Length && IsAsciiWhitespace(data[i]))
            {
                i++;
            }

            if (i >= data.Length)
            {
                break;
            }

            if (opCount >= cap)
            {
                break;
            }

            if (insideText)
            {
                if (TryParseTextOperatorFast(data, i, out int fastNext, out OxOperator? fastOp))
                {
                    if (fastOp is OxOperator.EndText)
                    {
                        insideText = false;
                    }

                    if (!handler(fastOp!))
                    {
                        return false;
                    }

                    opCount++;
                    i = fastNext;
                    consecutiveErrors = 0;
                }
                else if (TryParseOperatorWithOperands(data, i, out int next, out OxOperator? op))
                {
                    if (op is OxOperator.EndText)
                    {
                        insideText = false;
                    }

                    if (!handler(op!))
                    {
                        return false;
                    }

                    opCount++;
                    i = next;
                    consecutiveErrors = 0;
                }
                else
                {
                    consecutiveErrors++;
                    if (consecutiveErrors >= MaxConsecutiveErrors)
                    {
                        break;
                    }

                    if (data.Length - i > 1)
                    {
                        i++;
                    }
                    else
                    {
                        break;
                    }
                }

                continue;
            }

            ScanResult scan = ScanGraphicsRegion(data, i, ref consecutiveErrors);
            switch (scan.Kind)
            {
                case ScanKind.EndOfData:
                    return true;

                case ScanKind.FoundBt:
                    if (!handler(OxOperator.BeginText.Instance))
                    {
                        return false;
                    }

                    opCount++;
                    i = scan.Rest;
                    insideText = true;
                    break;

                case ScanKind.InlineImage:
                    i = TryParseInlineImage(data, scan.Rest, out int afterImage, out _) ? afterImage : scan.Rest;
                    break;

                case ScanKind.NeedFullParse:
                    if (TryParseOperatorWithOperands(data, scan.OperandStart, out int nextFull, out OxOperator? fullOp))
                    {
                        if (!handler(fullOp!))
                        {
                            return false;
                        }

                        opCount++;
                        i = nextFull;
                    }
                    else
                    {
                        i = scan.AfterOp;
                    }

                    break;

                case ScanKind.DeferredThenText:
                {
                    int remaining = scan.DeferredStart;
                    while (remaining < scan.TriggerStart)
                    {
                        if (TryParseOperatorWithOperands(data, remaining, out int nextDeferred, out OxOperator? deferredOp))
                        {
                            if (!handler(deferredOp!))
                            {
                                return false;
                            }

                            opCount++;
                            remaining = nextDeferred;
                        }
                        else if (data.Length - remaining > 1)
                        {
                            remaining++;
                        }
                        else
                        {
                            break;
                        }
                    }

                    i = scan.TriggerStart;
                    consecutiveErrors = 0;
                    break;
                }

                case ScanKind.SimpleOp:
                    if (!handler(scan.Op!))
                    {
                        return false;
                    }

                    opCount++;
                    i = scan.Rest;
                    break;

                default:
                    return true;
            }
        }

        return true;
    }

    /// <summary>Parse one prescan-identified text region.</summary>
    private static bool ParseRegionTextOnly(byte[] region, Func<OxOperator, bool> handler) =>
        RunTextOnly(region, handler, EffectiveMaxOperators);

    // ══ Prescan ════════════════════════════════════════════════════════════

    /// <summary>Graphics state captured at a BT/Do position by the forward scan.</summary>
    private sealed class PrescanState
    {
        internal float A, B, C, D, E, F;
        internal string? FontName;
        internal float FontSize;
    }

    private sealed class PrescanResult
    {
        internal List<(int Start, int End)> Regions = new();

        /// <summary>Null when the backward scan alone supplied full CTM context.</summary>
        internal List<PrescanState>? RegionStates;
    }

    /// <summary>
    /// Locate text-bearing regions. When the backward scan can capture every
    /// enclosing q/cm within 4 KB the regions stand on their own; otherwise a
    /// forward CTM scan supplies the graphics state per region. Returns null to
    /// tell the caller to fall back to a full scan.
    /// </summary>
    private static PrescanResult? PrescanTextRegions(byte[] data)
    {
        int len = data.Length;
        var textPositions = new List<int>();

        for (int pos = 0; pos + 1 < len; pos++)
        {
            byte b = data[pos];
            if (b != (byte)'B' && b != (byte)'D')
            {
                continue;
            }

            bool isBt = b == (byte)'B' && data[pos + 1] == (byte)'T';
            bool isDo = b == (byte)'D' && data[pos + 1] == (byte)'o';
            if (!isBt && !isDo)
            {
                continue;
            }

            bool beforeOk = pos == 0 || IsPrescanBoundary(data[pos - 1]);
            bool afterOk = pos + 2 >= len || IsPrescanBoundary(data[pos + 2]);
            if (beforeOk && afterOk)
            {
                textPositions.Add(pos);
            }
        }

        if (textPositions.Count == 0)
        {
            return new PrescanResult();
        }

        // Chart/figure streams where Do dominates BT would merge regions across
        // the whole stream, so drop the Do positions in that case.
        int btCount = textPositions.Count(p => p + 1 < len && data[p] == (byte)'B');
        int doCount = textPositions.Count - btCount;
        if (doCount > 50 && doCount > btCount * 10)
        {
            textPositions.RemoveAll(p => !(p + 1 < len && data[p] == (byte)'B'));
            if (textPositions.Count == 0)
            {
                return new PrescanResult();
            }
        }

        var regions = new List<(int Start, int End)>();
        bool needsForwardCtm = false;

        foreach (int tp in textPositions)
        {
            (int regionStart, bool hitLimit) = FindRegionStart(data, tp);
            if (hitLimit)
            {
                needsForwardCtm = true;
            }

            int regionEnd;
            if (data[tp] == (byte)'B')
            {
                int et = FindMatchingEt(data, tp + 2);
                regionEnd = et < 0 ? len : et;
            }
            else
            {
                regionEnd = tp + 2;
            }

            regions.Add((regionStart, Math.Min(regionEnd, len)));
        }

        if (regions.Count == 0)
        {
            return new PrescanResult();
        }

        if (needsForwardCtm)
        {
            // Some BT sat too far from the stream start for the backward scan to
            // capture the enclosing CTM; the forward scan gets the whole state.
            List<PrescanState>? states = ForwardScanCtm(data, textPositions);
            if (states is null)
            {
                return null;
            }

            // Regions start at the BT/Do itself (not the backward-scanned q) to
            // avoid q/Q nesting issues with the injected save/restore, but are
            // extended over any preceding BDC/BMC and following EMC so marked
            // content survives in tagged PDFs.
            var indexed = new List<((int Start, int End) Region, PrescanState State)>();
            for (int idx = 0; idx < textPositions.Count; idx++)
            {
                int tp = textPositions[idx];
                int regionStart = FindPrecedingMarkedContent(data, tp);
                int regionEnd;
                if (data[tp] == (byte)'B')
                {
                    int et = FindMatchingEt(data, tp + 2);
                    regionEnd = FindFollowingEmc(data, et < 0 ? len : et);
                }
                else
                {
                    regionEnd = tp + 2;
                }

                indexed.Add(((regionStart, Math.Min(regionEnd, len)), states[idx]));
            }

            indexed.Sort((x, y) => x.Region.Start.CompareTo(y.Region.Start));

            var mergedCtm = new PrescanResult { RegionStates = new List<PrescanState>() };
            foreach (var entry in indexed)
            {
                if (mergedCtm.Regions.Count > 0)
                {
                    (int lastStart, int lastEnd) = mergedCtm.Regions[^1];
                    if (entry.Region.Start <= lastEnd)
                    {
                        // Merged — the first region's state stays authoritative.
                        mergedCtm.Regions[^1] = (lastStart, Math.Max(lastEnd, entry.Region.End));
                        continue;
                    }
                }

                mergedCtm.Regions.Add(entry.Region);
                mergedCtm.RegionStates!.Add(entry.State);
            }

            return mergedCtm;
        }

        regions.Sort((x, y) => x.Start.CompareTo(y.Start));
        var merged = new PrescanResult();
        foreach (var r in regions)
        {
            if (merged.Regions.Count > 0)
            {
                (int lastStart, int lastEnd) = merged.Regions[^1];
                if (r.Start <= lastEnd)
                {
                    merged.Regions[^1] = (lastStart, Math.Max(lastEnd, r.End));
                    continue;
                }
            }

            merged.Regions.Add(r);
        }

        return merged;
    }

    /// <summary>
    /// Track q/Q/cm/Tf across the whole stream and record the accumulated state
    /// at each position in <paramref name="textPositions"/>. Far cheaper than a
    /// full parse; numeric operands ride a rolling buffer so cm always has its
    /// six floats available.
    /// </summary>
    private static List<PrescanState>? ForwardScanCtm(byte[] data, List<int> textPositions)
    {
        var results = new List<PrescanState>(textPositions.Count);
        if (textPositions.Count == 0)
        {
            return results;
        }

        for (int k = 0; k < textPositions.Count; k++)
        {
            results.Add(new PrescanState());
        }

        // Font table indexed from the CTM stack so q/Q never clones a string.
        var fontTable = new List<(string Name, float Size)>();
        int? currentFontIdx = null;
        var ctmStack = new List<(OxMatrix Ctm, int? FontIdx)>();
        OxMatrix ctm = OxMatrix.Identity;

        float[] numBuf = new float[6];
        int numCount = 0;
        string? lastName = null;

        var sorted = new List<(int OrigIdx, int Pos)>(textPositions.Count);
        for (int k = 0; k < textPositions.Count; k++)
        {
            sorted.Add((k, textPositions[k]));
        }

        sorted.Sort((x, y) => x.Pos.CompareTo(y.Pos));
        int nextTp = 0;

        int len = data.Length;
        int i = 0;

        void Record(int origIdx)
        {
            var state = new PrescanState { A = ctm.A, B = ctm.B, C = ctm.C, D = ctm.D, E = ctm.E, F = ctm.F };
            if (currentFontIdx is int fi)
            {
                state.FontName = fontTable[fi].Name;
                state.FontSize = fontTable[fi].Size;
            }

            results[origIdx] = state;
        }

        int cancelTick2 = 0;
        while (i < len)
        {
            Cancel.Every(cancelTick2++, 0xFF);
            while (nextTp < sorted.Count && sorted[nextTp].Pos <= i)
            {
                Record(sorted[nextTp].OrigIdx);
                nextTp++;
            }

            if (nextTp >= sorted.Count)
            {
                break;
            }

            byte b = data[i];

            if (IsAsciiWhitespace(b))
            {
                i++;
                continue;
            }

            if (IsAsciiDigit(b) || b == (byte)'-' || b == (byte)'+'
                || (b == (byte)'.' && i + 1 < len && IsAsciiDigit(data[i + 1])))
            {
                int start = i;
                i++;
                while (i < len && (IsAsciiDigit(data[i]) || data[i] == (byte)'.'))
                {
                    i++;
                }

                if (TryParseFloatToken(data, start, i, out float val))
                {
                    if (numCount < 6)
                    {
                        numBuf[numCount] = val;
                        numCount++;
                    }
                    else
                    {
                        Array.Copy(numBuf, 1, numBuf, 0, 5);
                        numBuf[5] = val;
                    }
                }

                continue;
            }

            if (IsAsciiAlpha(b))
            {
                int opStart = i;
                i++;
                while (i < len && (IsAsciiAlpha(data[i]) || data[i] == (byte)'*' || data[i] == (byte)'\'' || data[i] == (byte)'"'))
                {
                    i++;
                }

                if (Matches(data, opStart, i, "BI"))
                {
                    // Inline-image binary data can contain q/Q/cm-shaped ASCII
                    // that would corrupt the CTM stack, so skip the whole block
                    // to the first whitespace-bounded EI (§8.9.7).
                    numCount = 0;
                    int j = i;
                    while (j + 1 < len)
                    {
                        if (data[j] == (byte)'E' && data[j + 1] == (byte)'I')
                        {
                            bool beforeOk = j == 0 || IsAsciiWhitespace(data[j - 1]);
                            bool afterOk = j + 2 >= len || IsAsciiWhitespace(data[j + 2])
                                || data[j + 2] is (byte)'(' or (byte)'<' or (byte)'[' or (byte)'/' or (byte)'%';
                            if (beforeOk && afterOk)
                            {
                                j += 2;
                                break;
                            }
                        }

                        j++;
                    }

                    i = j;
                    continue;
                }

                if (Matches(data, opStart, i, "q"))
                {
                    ctmStack.Add((ctm, currentFontIdx));
                    numCount = 0;
                }
                else if (Matches(data, opStart, i, "Q"))
                {
                    if (ctmStack.Count > 0)
                    {
                        (ctm, currentFontIdx) = ctmStack[^1];
                        ctmStack.RemoveAt(ctmStack.Count - 1);
                    }

                    numCount = 0;
                }
                else if (Matches(data, opStart, i, "cm"))
                {
                    if (numCount >= 6)
                    {
                        int b0 = numCount - 6;
                        var newCtm = new OxMatrix(numBuf[b0], numBuf[b0 + 1], numBuf[b0 + 2], numBuf[b0 + 3], numBuf[b0 + 4], numBuf[b0 + 5]);
                        ctm = newCtm.Multiply(ctm);
                    }

                    numCount = 0;
                }
                else if (Matches(data, opStart, i, "Tf"))
                {
                    if (numCount >= 1 && lastName is not null)
                    {
                        fontTable.Add((lastName, numBuf[numCount - 1]));
                        currentFontIdx = fontTable.Count - 1;
                    }

                    numCount = 0;
                    lastName = null;
                }
                else
                {
                    numCount = 0;
                }

                continue;
            }

            if (b == (byte)'(')
            {
                i++;
                uint depth = 1;
                while (i < len && depth > 0)
                {
                    byte c = data[i];
                    if (c == (byte)'\\')
                    {
                        i++;
                    }
                    else if (c == (byte)'(')
                    {
                        depth++;
                    }
                    else if (c == (byte)')')
                    {
                        depth--;
                    }

                    i++;
                }

                numCount = 0;
                continue;
            }

            if (b == (byte)'<')
            {
                if (i + 1 < len && data[i + 1] == (byte)'<')
                {
                    i += 2;
                    uint depth = 1;
                    while (i + 1 < len && depth > 0)
                    {
                        if (data[i] == (byte)'<' && data[i + 1] == (byte)'<')
                        {
                            depth++;
                            i += 2;
                        }
                        else if (data[i] == (byte)'>' && data[i + 1] == (byte)'>')
                        {
                            depth--;
                            i += 2;
                        }
                        else
                        {
                            i++;
                        }
                    }
                }
                else
                {
                    i++;
                    while (i < len && data[i] != (byte)'>')
                    {
                        i++;
                    }

                    if (i < len)
                    {
                        i++;
                    }
                }

                numCount = 0;
                continue;
            }

            if (b == (byte)'/')
            {
                int nameStart = i + 1;
                i++;
                while (i < len && !IsAsciiWhitespace(data[i]) && !IsNameDelimiter(data[i]))
                {
                    i++;
                }

                lastName = AsciiString(data, nameStart, i);
                numCount = 0;
                continue;
            }

            if (b == (byte)'%')
            {
                while (i < len && data[i] != (byte)'\n' && data[i] != (byte)'\r')
                {
                    i++;
                }

                continue;
            }

            i++;
        }

        while (nextTp < sorted.Count)
        {
            Record(sorted[nextTp].OrigIdx);
            nextTp++;
        }

        return results;
    }

    /// <summary>
    /// Scan backwards for the nearest unmatched q within a 4 KB window.
    /// Returns the region start and whether the window stopped short of the
    /// stream start — in which case enclosing q/cm operators may be missing.
    /// </summary>
    private static (int Start, bool HitLimit) FindRegionStart(byte[] data, int pos)
    {
        int scanStart = Math.Max(0, pos - 4096);
        int qDepth = 0;
        int bestQPos = pos;

        for (int i = pos - 1; i >= scanStart; i--)
        {
            byte b = data[i];
            if (b != (byte)'q' && b != (byte)'Q')
            {
                continue;
            }

            bool beforeOk = i == scanStart || IsAsciiWhitespace(data[i - 1])
                || data[i - 1] is (byte)')' or (byte)'>' or (byte)']';
            bool afterOk = i + 1 >= pos || IsAsciiWhitespace(data[i + 1])
                || data[i + 1] is (byte)'(' or (byte)'<' or (byte)'[' or (byte)'/' or (byte)'%'
                || IsAsciiDigit(data[i + 1]) || data[i + 1] == (byte)'-' || data[i + 1] == (byte)'.';

            if (!beforeOk || !afterOk)
            {
                continue;
            }

            if (b == (byte)'Q')
            {
                qDepth++;
            }
            else if (qDepth > 0)
            {
                qDepth--;
            }
            else
            {
                bestQPos = i;
                break;
            }
        }

        // Complete CTM context is only guaranteed when the scan reached the very
        // start of the data: an unmatched q inside the window may still sit under
        // further enclosing q/cm scaling transforms.
        return (bestQPos, scanStart > 0);
    }

    /// <summary>Position of a BDC/BMC immediately preceding <paramref name="pos"/>
    /// (within 256 bytes), else <paramref name="pos"/>.</summary>
    private static int FindPrecedingMarkedContent(byte[] data, int pos)
    {
        int scanStart = Math.Max(0, pos - 256);
        for (int i = pos - 1; i > scanStart; i--)
        {
            if (data[i] != (byte)'C' || i < 2 || data[i - 2] != (byte)'B'
                || (data[i - 1] != (byte)'D' && data[i - 1] != (byte)'M'))
            {
                continue;
            }

            int opStart = i - 2;
            bool beforeOk = opStart == 0 || !IsAsciiAlphanumeric(data[opStart - 1]);
            bool afterOk = i + 1 >= data.Length || !IsAsciiAlphanumeric(data[i + 1]);
            if (!beforeOk || !afterOk)
            {
                continue;
            }

            // Back up to the start of the line so a BDC's tag and property dict
            // ("/Span << /MCID 0 >> BDC") come along.
            int lineStart = opStart;
            while (lineStart > scanStart && data[lineStart - 1] != (byte)'\n' && data[lineStart - 1] != (byte)'\r')
            {
                lineStart--;
            }

            return lineStart;
        }

        return pos;
    }

    /// <summary>Position after an EMC immediately following <paramref name="pos"/>
    /// (within 256 bytes), else <paramref name="pos"/>.</summary>
    private static int FindFollowingEmc(byte[] data, int pos)
    {
        int scanEnd = Math.Min(pos + 256, data.Length);
        for (int i = pos; i + 2 < scanEnd; i++)
        {
            if (data[i] != (byte)'E' || data[i + 1] != (byte)'M' || data[i + 2] != (byte)'C')
            {
                continue;
            }

            bool beforeOk = i == 0 || IsAsciiWhitespace(data[i - 1]);
            bool afterOk = i + 3 >= data.Length || IsAsciiWhitespace(data[i + 3]);
            if (beforeOk && afterOk)
            {
                return i + 3;
            }
        }

        return pos;
    }

    /// <summary>Position after the ET matching a BT, or -1.</summary>
    private static int FindMatchingEt(byte[] data, int start)
    {
        int len = data.Length;
        for (int pos = start; pos + 1 < len; pos++)
        {
            if (data[pos] != (byte)'E' || data[pos + 1] != (byte)'T')
            {
                continue;
            }

            bool beforeOk = pos == 0 || IsAsciiWhitespace(data[pos - 1])
                || data[pos - 1] is (byte)')' or (byte)'>' or (byte)']' or (byte)'}' or (byte)'/' or (byte)'%';
            bool afterOk = pos + 2 >= len || IsAsciiWhitespace(data[pos + 2])
                || data[pos + 2] is (byte)'(' or (byte)'<' or (byte)'[' or (byte)'/' or (byte)'%';
            if (beforeOk && afterOk)
            {
                return pos + 2;
            }
        }

        return -1;
    }

    // ══ Generic operator parsing ═══════════════════════════════════════════

    /// <summary>
    /// Parse one operator together with the operands that precede it.
    /// Returns false when the input runs out or an operand cannot be read.
    /// </summary>
    private static bool TryParseOperatorWithOperands(byte[] data, int pos, out int next, out OxOperator? op)
    {
        var operands = new List<OxOperand>(6);
        int remaining = pos;

        while (true)
        {
            remaining = SkipMultispace(data, remaining);
            if (remaining >= data.Length)
            {
                next = remaining;
                op = null;
                return false;
            }

            if (IsOperatorStart(data[remaining]))
            {
                int nameStart = remaining;
                while (remaining < data.Length && IsOperatorNameByte(data[remaining]))
                {
                    remaining++;
                }

                string name = AsciiString(data, nameStart, remaining);

                if (name == "BI")
                {
                    return TryParseInlineImage(data, remaining, out next, out op);
                }

                op = BuildOperator(name, operands);
                next = remaining;
                return true;
            }

            if (!TryParseOperand(data, remaining, out int afterOperand, out OxOperand? obj))
            {
                next = remaining;
                op = null;
                return false;
            }

            operands.Add(obj!);
            remaining = afterOperand;
        }
    }

    /// <summary>Convert an operator name plus its operands into a typed operator.</summary>
    private static OxOperator BuildOperator(string name, List<OxOperand> operands)
    {
        switch (name)
        {
            // Text positioning
            case "Td": return new OxOperator.Td(Number(operands, 0, 0f), Number(operands, 1, 0f));
            case "TD": return new OxOperator.TD(Number(operands, 0, 0f), Number(operands, 1, 0f));
            case "Tm":
                return new OxOperator.Tm(
                    Number(operands, 0, 1f), Number(operands, 1, 0f), Number(operands, 2, 0f),
                    Number(operands, 3, 1f), Number(operands, 4, 0f), Number(operands, 5, 0f));
            case "T*": return OxOperator.TStar.Instance;

            // Text showing
            case "Tj": return new OxOperator.Tj(StringBytes(operands, 0));
            case "TJ": return new OxOperator.TJ(TextElements(operands));
            case "'": return new OxOperator.Quote(StringBytes(operands, 0));
            case "\"":
                return new OxOperator.DoubleQuote(Number(operands, 0, 0f), Number(operands, 1, 0f), StringBytes(operands, 2));

            // Text state
            case "Tc": return new OxOperator.Tc(Number(operands, 0, 0f));
            case "Tw": return new OxOperator.Tw(Number(operands, 0, 0f));
            case "Tz": return new OxOperator.Tz(Number(operands, 0, 100f));
            case "TL": return new OxOperator.TL(Number(operands, 0, 0f));
            case "Tf": return new OxOperator.Tf(Name(operands, 0, ""), Number(operands, 1, 12f));
            case "Tr": return new OxOperator.Tr((byte)Integer(operands, 0, 0));
            case "Ts": return new OxOperator.Ts(Number(operands, 0, 0f));

            // Graphics state
            case "q": return OxOperator.SaveState.Instance;
            case "Q": return OxOperator.RestoreState.Instance;
            case "cm":
                return new OxOperator.Cm(
                    Number(operands, 0, 1f), Number(operands, 1, 0f), Number(operands, 2, 0f),
                    Number(operands, 3, 1f), Number(operands, 4, 0f), Number(operands, 5, 0f));

            // Colour
            case "rg": return new OxOperator.SetFillRgb(Number(operands, 0, 0f), Number(operands, 1, 0f), Number(operands, 2, 0f));
            case "RG": return new OxOperator.SetStrokeRgb(Number(operands, 0, 0f), Number(operands, 1, 0f), Number(operands, 2, 0f));
            case "g": return new OxOperator.SetFillGray(Number(operands, 0, 0f));
            case "G": return new OxOperator.SetStrokeGray(Number(operands, 0, 0f));
            case "k":
                return new OxOperator.SetFillCmyk(Number(operands, 0, 0f), Number(operands, 1, 0f), Number(operands, 2, 0f), Number(operands, 3, 0f));
            case "K":
                return new OxOperator.SetStrokeCmyk(Number(operands, 0, 0f), Number(operands, 1, 0f), Number(operands, 2, 0f), Number(operands, 3, 0f));

            // Colour space
            case "cs": return new OxOperator.SetFillColorSpace(Name(operands, 0, "DeviceGray"));
            case "CS": return new OxOperator.SetStrokeColorSpace(Name(operands, 0, "DeviceGray"));
            case "sc": return new OxOperator.SetFillColor(NumericComponents(operands));
            case "SC": return new OxOperator.SetStrokeColor(NumericComponents(operands));
            case "scn": return new OxOperator.SetFillColorN(NumericComponents(operands), TrailingPatternName(operands));
            case "SCN": return new OxOperator.SetStrokeColorN(NumericComponents(operands), TrailingPatternName(operands));

            // Text objects
            case "BT": return OxOperator.BeginText.Instance;
            case "ET": return OxOperator.EndText.Instance;

            // XObjects
            case "Do":
                // Per ISO 32000-1:2008 §7.8.2 operands shall immediately precede
                // their operator and none shall be left over. If stray operands
                // accumulated (e.g. a dropped cm before this Do) the XObject name
                // is still the LAST operand, not the first.
                return new OxOperator.Do(Name(operands, operands.Count - 1, ""));

            // Path construction
            case "m": return new OxOperator.MoveTo(Number(operands, 0, 0f), Number(operands, 1, 0f));
            case "l": return new OxOperator.LineTo(Number(operands, 0, 0f), Number(operands, 1, 0f));
            case "c":
                return new OxOperator.CurveTo(
                    Number(operands, 0, 0f), Number(operands, 1, 0f), Number(operands, 2, 0f),
                    Number(operands, 3, 0f), Number(operands, 4, 0f), Number(operands, 5, 0f));
            case "v":
                return new OxOperator.CurveToV(Number(operands, 0, 0f), Number(operands, 1, 0f), Number(operands, 2, 0f), Number(operands, 3, 0f));
            case "y":
                return new OxOperator.CurveToY(Number(operands, 0, 0f), Number(operands, 1, 0f), Number(operands, 2, 0f), Number(operands, 3, 0f));
            case "h": return OxOperator.ClosePath.Instance;
            case "re":
                return new OxOperator.Rectangle(Number(operands, 0, 0f), Number(operands, 1, 0f), Number(operands, 2, 0f), Number(operands, 3, 0f));
            case "S": return OxOperator.Stroke.Instance;
            case "f":
            case "F": return OxOperator.Fill.Instance; // F is the obsolete spelling of f
            case "f*": return OxOperator.FillEvenOdd.Instance;
            case "b": return OxOperator.CloseFillStroke.Instance;
            case "b*": return OxOperator.CloseFillStrokeEvenOdd.Instance;
            case "B": return OxOperator.FillStroke.Instance;
            case "B*": return OxOperator.FillStrokeEvenOdd.Instance;
            case "n": return OxOperator.EndPath.Instance;
            case "W": return OxOperator.ClipNonZero.Instance;
            case "W*": return OxOperator.ClipEvenOdd.Instance;

            // Non-text graphics state
            case "w": return new OxOperator.SetLineWidth(Number(operands, 0, 1f));
            case "d":
            {
                var array = new List<float>();
                if (operands.Count > 0 && operands[0] is OxOperand.Array arr)
                {
                    foreach (OxOperand item in arr.Items)
                    {
                        if (item.AsNumber is float f)
                        {
                            array.Add(f);
                        }
                    }
                }

                return new OxOperator.SetDash(array, Number(operands, 1, 0f));
            }

            case "J": return new OxOperator.SetLineCap((byte)Integer(operands, 0, 0));
            case "j": return new OxOperator.SetLineJoin((byte)Integer(operands, 0, 0));
            case "M": return new OxOperator.SetMiterLimit(Number(operands, 0, 10f));
            case "ri": return new OxOperator.SetRenderingIntent(Name(operands, 0, "RelativeColorimetric"));
            case "i": return new OxOperator.SetFlatness(Number(operands, 0, 1f));
            case "gs": return new OxOperator.SetExtGState(Name(operands, 0, ""));
            case "sh": return new OxOperator.PaintShading(Name(operands, 0, ""));

            // Marked content (ISO 32000-1 §14.6)
            case "BMC": return new OxOperator.BeginMarkedContent(Name(operands, 0, ""));
            case "BDC":
                return new OxOperator.BeginMarkedContentDict(
                    Name(operands, 0, ""),
                    operands.Count > 1 ? operands[1] : OxOperand.Null.Instance);
            case "EMC": return OxOperator.EndMarkedContent.Instance;

            default: return new OxOperator.Other(name, operands);
        }
    }

    private static float Number(List<OxOperand> operands, int index, float fallback) =>
        index >= 0 && index < operands.Count && operands[index].AsNumber is float f ? f : fallback;

    // `as_integer` in Rust accepts Integer only, so a real-valued Tr/J/j falls
    // back to the default rather than truncating.
    private static long Integer(List<OxOperand> operands, int index, long fallback) =>
        index >= 0 && index < operands.Count && operands[index].AsInteger is long v ? v : fallback;

    private static byte[] StringBytes(List<OxOperand> operands, int index) =>
        index >= 0 && index < operands.Count && operands[index].AsString is byte[] b ? b : Array.Empty<byte>();

    private static string Name(List<OxOperand> operands, int index, string fallback) =>
        index >= 0 && index < operands.Count && operands[index].AsName is string s ? s : fallback;

    private static List<OxTextElement> TextElements(List<OxOperand> operands)
    {
        var elements = new List<OxTextElement>();
        if (operands.Count == 0 || operands[0].AsArray is not List<OxOperand> array)
        {
            return elements;
        }

        foreach (OxOperand item in array)
        {
            switch (item)
            {
                case OxOperand.Str s:
                    elements.Add(new OxTextElement.Str(s.Bytes));
                    break;
                case OxOperand.Integer n:
                    elements.Add(new OxTextElement.Offset(n.Value));
                    break;
                case OxOperand.Real r:
                    elements.Add(new OxTextElement.Offset((float)r.Value));
                    break;
            }
        }

        return elements;
    }

    private static List<float> NumericComponents(List<OxOperand> operands)
    {
        var components = new List<float>(operands.Count);
        foreach (OxOperand operand in operands)
        {
            if (operand.AsNumber is float f)
            {
                components.Add(f);
            }
        }

        return components;
    }

    /// <summary>Pattern name of an scn/SCN: the last operand, when it is a name.</summary>
    private static string? TrailingPatternName(List<OxOperand> operands) =>
        operands.Count > 0 ? operands[^1].AsName : null;

    // ══ Inline images (ISO 32000-1 §8.9.7) ═════════════════════════════════

    /// <summary>
    /// Parse a BI &lt;dict&gt; ID &lt;binary&gt; EI sequence starting just after BI.
    /// The hard part is finding EI, since those two bytes can occur inside the
    /// image data — per spec the real EI is preceded by whitespace and followed
    /// by whitespace, a delimiter, or end of stream.
    /// </summary>
    private static bool TryParseInlineImage(byte[] data, int pos, out int next, out OxOperator? op)
    {
        var dict = new Dictionary<string, OxOperand>(StringComparer.Ordinal);
        int remaining = pos;
        next = pos;
        op = null;

        while (true)
        {
            remaining = SkipMultispace(data, remaining);
            if (remaining >= data.Length)
            {
                return false;
            }

            if (data.Length - remaining >= 2 && data[remaining] == (byte)'I' && data[remaining + 1] == (byte)'D')
            {
                if (data.Length - remaining == 2 || (data.Length - remaining > 2 && IsWhitespace(data[remaining + 2])))
                {
                    remaining += 2;
                    break;
                }
            }

            if (!TryParseOperand(data, remaining, out int afterKey, out OxOperand? key))
            {
                return false;
            }

            remaining = SkipMultispace(data, afterKey);
            if (!TryParseOperand(data, remaining, out int afterValue, out OxOperand? value))
            {
                return false;
            }

            remaining = afterValue;

            if (key!.AsName is string keyName)
            {
                dict[keyName] = value!;
            }
        }

        remaining = SkipMultispace(data, remaining);

        int eiPos = FindEiOperator(data, remaining);
        if (eiPos < 0)
        {
            return false;
        }

        byte[] imageData = Slice(data, remaining, eiPos);

        // Upstream advances by 2 from the whitespace before EI, which lands on
        // the "I" rather than past it; the leftover byte becomes a harmless
        // Other("I") operator. Kept as-is so operator streams match pdf_oxide.
        next = eiPos + 2;
        op = new OxOperator.InlineImage(dict, imageData);
        return true;
    }

    /// <summary>Index of the whitespace byte immediately before a well-formed EI, or -1.</summary>
    private static int FindEiOperator(byte[] data, int start)
    {
        for (int i = start; i + 2 < data.Length; i++)
        {
            if (!IsWhitespace(data[i]) || data[i + 1] != (byte)'E' || data[i + 2] != (byte)'I')
            {
                continue;
            }

            if (i + 3 >= data.Length || IsWhitespaceOrDelimiter(data[i + 3]))
            {
                return i;
            }
        }

        return -1;
    }

    // ══ Fast BT/ET operator parser ═════════════════════════════════════════

    private enum FastKind
    {
        None,
        Number,
        StringBytes,
        Name,
        TextArray,
    }

    private struct FastOperand
    {
        internal FastKind Kind;
        internal float Number;
        internal byte[]? Bytes;
        internal string? Name;
        internal List<OxTextElement>? TextArray;
    }

    /// <summary>
    /// Hand-rolled parser for the operators that appear inside a text block.
    /// Returns false to hand the input back to the generic parser.
    /// </summary>
    private static bool TryParseTextOperatorFast(byte[] data, int pos, out int next, out OxOperator? op)
    {
        // Eight slots covers every standard PDF operator's operand count.
        var operands = new FastOperand[8];
        int opCount = 0;
        next = pos;
        op = null;

        while (true)
        {
            while (pos < data.Length && IsWhitespace(data[pos]))
            {
                pos++;
            }

            if (pos >= data.Length)
            {
                return false;
            }

            byte b = data[pos];

            if (IsAsciiDigit(b) || b == (byte)'.' || b == (byte)'+' || b == (byte)'-')
            {
                // A lone sign is not a number — hand back to the generic parser.
                if ((b == (byte)'-' || b == (byte)'+')
                    && (pos + 1 >= data.Length || (!IsAsciiDigit(data[pos + 1]) && data[pos + 1] != (byte)'.')))
                {
                    return false;
                }

                if (!TryParseFloatFast(data, pos, out float num, out int consumed))
                {
                    return false;
                }

                if (opCount < 8)
                {
                    operands[opCount] = new FastOperand { Kind = FastKind.Number, Number = num };
                    opCount++;
                }

                pos += consumed;
                continue;
            }

            if (b == (byte)'(')
            {
                if (!TryParseLiteralStringFast(data, pos, out byte[]? bytes, out int end))
                {
                    return false;
                }

                if (opCount < 8)
                {
                    operands[opCount] = new FastOperand { Kind = FastKind.StringBytes, Bytes = bytes };
                    opCount++;
                }

                pos = end;
                continue;
            }

            if (b == (byte)'<')
            {
                if (pos + 1 < data.Length && data[pos + 1] == (byte)'<')
                {
                    return false; // dictionary operand — generic parser
                }

                if (!TryParseHexStringFast(data, pos, out byte[]? bytes, out int end))
                {
                    return false;
                }

                if (opCount < 8)
                {
                    operands[opCount] = new FastOperand { Kind = FastKind.StringBytes, Bytes = bytes };
                    opCount++;
                }

                pos = end;
                continue;
            }

            if (b == (byte)'/')
            {
                (string name, int end) = ParseNameFast(data, pos);
                if (opCount < 8)
                {
                    operands[opCount] = new FastOperand { Kind = FastKind.Name, Name = name };
                    opCount++;
                }

                pos = end;
                continue;
            }

            if (b == (byte)'[')
            {
                if (!TryParseTjArrayFast(data, pos, out List<OxTextElement>? elements, out int end))
                {
                    return false;
                }

                if (opCount < 8)
                {
                    operands[opCount] = new FastOperand { Kind = FastKind.TextArray, TextArray = elements };
                    opCount++;
                }

                pos = end;
                continue;
            }

            if (!IsOperatorStart(b))
            {
                return false;
            }

            int opStart = pos;
            while (pos < data.Length && IsOperatorNameByte(data[pos]))
            {
                pos++;
            }

            if (Matches(data, opStart, pos, "true") || Matches(data, opStart, pos, "false") || Matches(data, opStart, pos, "null"))
            {
                continue; // operand keyword, not an operator
            }

            string opName = AsciiString(data, opStart, pos);
            op = BuildFastOperator(opName, operands, opCount);
            if (op is null)
            {
                return false;
            }

            next = pos;
            return true;
        }
    }

    private static OxOperator? BuildFastOperator(string name, FastOperand[] operands, int opCount)
    {
        float Num(int index, float fallback) =>
            operands[index].Kind == FastKind.Number ? operands[index].Number : fallback;

        byte[] Bytes(int index) =>
            operands[index].Kind == FastKind.StringBytes ? operands[index].Bytes! : Array.Empty<byte>();

        string Nm(int index, string fallback) =>
            operands[index].Kind == FastKind.Name ? operands[index].Name! : fallback;

        List<float> Components()
        {
            var list = new List<float>(opCount);
            for (int i = 0; i < opCount; i++)
            {
                if (operands[i].Kind == FastKind.Number)
                {
                    list.Add(operands[i].Number);
                }
            }

            return list;
        }

        string? TrailingName() =>
            operands[Math.Max(opCount - 1, 0)].Kind == FastKind.Name ? operands[Math.Max(opCount - 1, 0)].Name : null;

        switch (name)
        {
            case "ET": return OxOperator.EndText.Instance;
            case "BT": return OxOperator.BeginText.Instance;
            case "Tf": return new OxOperator.Tf(Nm(0, string.Empty), Num(1, 12f));
            case "Td": return new OxOperator.Td(Num(0, 0f), Num(1, 0f));
            case "TD": return new OxOperator.TD(Num(0, 0f), Num(1, 0f));
            case "Tm": return new OxOperator.Tm(Num(0, 1f), Num(1, 0f), Num(2, 0f), Num(3, 1f), Num(4, 0f), Num(5, 0f));
            case "T*": return OxOperator.TStar.Instance;
            case "Tj": return new OxOperator.Tj(Bytes(0));
            case "TJ":
                return new OxOperator.TJ(operands[0].Kind == FastKind.TextArray ? operands[0].TextArray! : new List<OxTextElement>());
            case "'": return new OxOperator.Quote(Bytes(0));
            case "\"": return new OxOperator.DoubleQuote(Num(0, 0f), Num(1, 0f), Bytes(2));
            case "Tc": return new OxOperator.Tc(Num(0, 0f));
            case "Tw": return new OxOperator.Tw(Num(0, 0f));
            case "Tz": return new OxOperator.Tz(Num(0, 100f));
            case "TL": return new OxOperator.TL(Num(0, 0f));
            case "Tr": return new OxOperator.Tr(operands[0].Kind == FastKind.Number ? (byte)operands[0].Number : (byte)0);
            case "Ts": return new OxOperator.Ts(Num(0, 0f));
            case "q": return OxOperator.SaveState.Instance;
            case "Q": return OxOperator.RestoreState.Instance;
            case "cm": return new OxOperator.Cm(Num(0, 1f), Num(1, 0f), Num(2, 0f), Num(3, 1f), Num(4, 0f), Num(5, 0f));
            case "rg": return new OxOperator.SetFillRgb(Num(0, 0f), Num(1, 0f), Num(2, 0f));
            case "RG": return new OxOperator.SetStrokeRgb(Num(0, 0f), Num(1, 0f), Num(2, 0f));
            case "g": return new OxOperator.SetFillGray(Num(0, 0f));
            case "G": return new OxOperator.SetStrokeGray(Num(0, 0f));
            case "k": return new OxOperator.SetFillCmyk(Num(0, 0f), Num(1, 0f), Num(2, 0f), Num(3, 0f));
            case "K": return new OxOperator.SetStrokeCmyk(Num(0, 0f), Num(1, 0f), Num(2, 0f), Num(3, 0f));
            case "cs": return new OxOperator.SetFillColorSpace(Nm(0, "DeviceGray"));
            case "CS": return new OxOperator.SetStrokeColorSpace(Nm(0, "DeviceGray"));
            case "sc": return new OxOperator.SetFillColor(Components());
            case "SC": return new OxOperator.SetStrokeColor(Components());
            case "scn": return new OxOperator.SetFillColorN(Components(), TrailingName());
            case "SCN": return new OxOperator.SetStrokeColorN(Components(), TrailingName());
            case "gs": return new OxOperator.SetExtGState(Nm(0, string.Empty));
            case "Do":
                // Same last-operand rule as the generic parser's Do arm.
                return new OxOperator.Do(TrailingName() ?? string.Empty);
            case "w": return new OxOperator.SetLineWidth(Num(0, 1f));
            case "J": return new OxOperator.SetLineCap(operands[0].Kind == FastKind.Number ? (byte)operands[0].Number : (byte)0);
            case "j": return new OxOperator.SetLineJoin(operands[0].Kind == FastKind.Number ? (byte)operands[0].Number : (byte)0);
            case "i": return new OxOperator.SetFlatness(Num(0, 0f));
            default: return null; // unknown inside BT/ET — generic parser
        }
    }

    private static bool TryParseFloatFast(byte[] data, int start, out float value, out int consumed)
    {
        int i = start;
        bool negative = false;
        if (i < data.Length && (data[i] == (byte)'-' || data[i] == (byte)'+'))
        {
            negative = data[i] == (byte)'-';
            i++;
        }

        int digitsStart = i;
        double intPart = 0;
        while (i < data.Length && IsAsciiDigit(data[i]))
        {
            intPart = (intPart * 10) + (data[i] - (byte)'0');
            i++;
        }

        double fracPart = 0;
        double fracScale = 1;
        if (i < data.Length && data[i] == (byte)'.')
        {
            i++;
            while (i < data.Length && IsAsciiDigit(data[i]))
            {
                fracPart = (fracPart * 10) + (data[i] - (byte)'0');
                fracScale *= 10;
                i++;
            }
        }

        if (i == digitsStart)
        {
            value = 0f;
            consumed = 0;
            return false;
        }

        double result = intPart + (fracPart / fracScale);
        value = (float)(negative ? -result : result);
        consumed = i - start;
        return true;
    }

    private static bool TryParseLiteralStringFast(byte[] data, int start, out byte[]? value, out int end)
    {
        int i = start + 1;
        uint depth = 1;

        // Most PDF strings are plain ASCII with no escapes or nesting; take them
        // without building an intermediate buffer.
        int scanStart = i;
        while (i < data.Length)
        {
            byte b = data[i];
            if (b == (byte)')')
            {
                value = Slice(data, scanStart, i);
                end = i + 1;
                return true;
            }

            if (b == (byte)'\\' || b == (byte)'(')
            {
                break;
            }

            i++;
        }

        i = scanStart;
        var result = new List<byte>();
        while (i < data.Length && depth > 0)
        {
            byte b = data[i];
            if (b == (byte)'\\' && i + 1 < data.Length)
            {
                byte esc = data[i + 1];
                switch (esc)
                {
                    case (byte)'n': result.Add((byte)'\n'); i += 2; break;
                    case (byte)'r': result.Add((byte)'\r'); i += 2; break;
                    case (byte)'t': result.Add((byte)'\t'); i += 2; break;
                    case (byte)'b': result.Add(0x08); i += 2; break;
                    case (byte)'f': result.Add(0x0C); i += 2; break;
                    case (byte)'(': result.Add((byte)'('); i += 2; break;
                    case (byte)')': result.Add((byte)')'); i += 2; break;
                    case (byte)'\\': result.Add((byte)'\\'); i += 2; break;
                    case (byte)'\r':
                        i += 2;
                        if (i < data.Length && data[i] == (byte)'\n')
                        {
                            i++;
                        }

                        break;
                    case (byte)'\n': i += 2; break;
                    default:
                        if (esc >= (byte)'0' && esc <= (byte)'7')
                        {
                            uint octal = (uint)(esc - (byte)'0');
                            int j = i + 2;
                            for (int k = 0; k < 2; k++)
                            {
                                if (j < data.Length && data[j] >= (byte)'0' && data[j] <= (byte)'7')
                                {
                                    octal = (octal * 8) + (uint)(data[j] - (byte)'0');
                                    j++;
                                }
                                else
                                {
                                    break;
                                }
                            }

                            result.Add((byte)(octal & 0xFF));
                            i = j;
                        }
                        else
                        {
                            result.Add(esc);
                            i += 2;
                        }

                        break;
                }

                continue;
            }

            if (b == (byte)'(')
            {
                depth++;
                result.Add((byte)'(');
                i++;
            }
            else if (b == (byte)')')
            {
                depth--;
                if (depth > 0)
                {
                    result.Add((byte)')');
                }

                i++;
            }
            else
            {
                result.Add(b);
                i++;
            }
        }

        if (depth != 0)
        {
            value = null;
            end = i;
            return false;
        }

        value = result.ToArray();
        end = i;
        return true;
    }

    private static bool TryParseHexStringFast(byte[] data, int start, out byte[]? value, out int end)
    {
        int i = start + 1;
        var result = new List<byte>();
        int highNibble = -1;

        while (i < data.Length)
        {
            byte b = data[i];
            if (b == (byte)'>')
            {
                // An odd digit count pads the trailing nibble with 0 (§7.3.4.3).
                if (highNibble >= 0)
                {
                    result.Add((byte)(highNibble << 4));
                }

                value = result.ToArray();
                end = i + 1;
                return true;
            }

            int nibble = HexNibble(b);
            if (nibble >= 0)
            {
                if (highNibble < 0)
                {
                    highNibble = nibble;
                }
                else
                {
                    result.Add((byte)((highNibble << 4) | nibble));
                    highNibble = -1;
                }
            }

            i++;
        }

        value = null;
        end = i;
        return false;
    }

    private static int HexNibble(byte b) => b switch
    {
        >= (byte)'0' and <= (byte)'9' => b - (byte)'0',
        >= (byte)'a' and <= (byte)'f' => b - (byte)'a' + 10,
        >= (byte)'A' and <= (byte)'F' => b - (byte)'A' + 10,
        _ => -1,
    };

    private static bool TryParseTjArrayFast(byte[] data, int start, out List<OxTextElement>? elements, out int end)
    {
        int i = start + 1;
        var list = new List<OxTextElement>();

        while (true)
        {
            while (i < data.Length && IsWhitespace(data[i]))
            {
                i++;
            }

            if (i >= data.Length)
            {
                elements = null;
                end = i;
                return false;
            }

            byte b = data[i];
            if (b == (byte)']')
            {
                elements = list;
                end = i + 1;
                return true;
            }

            if (b == (byte)'(')
            {
                if (!TryParseLiteralStringFast(data, i, out byte[]? bytes, out int strEnd))
                {
                    elements = null;
                    end = i;
                    return false;
                }

                list.Add(new OxTextElement.Str(bytes!));
                i = strEnd;
            }
            else if (b == (byte)'<')
            {
                if (!TryParseHexStringFast(data, i, out byte[]? bytes, out int hexEnd))
                {
                    elements = null;
                    end = i;
                    return false;
                }

                list.Add(new OxTextElement.Str(bytes!));
                i = hexEnd;
            }
            else if (IsAsciiDigit(b) || b == (byte)'.' || b == (byte)'+' || b == (byte)'-')
            {
                if (!TryParseFloatFast(data, i, out float num, out int consumed))
                {
                    elements = null;
                    end = i;
                    return false;
                }

                list.Add(new OxTextElement.Offset(num));
                i += consumed;
            }
            else
            {
                i++; // unknown token inside the array
            }
        }
    }

    /// <summary>
    /// Read a /Name at the byte level. Unlike the generic operand reader this
    /// does not decode #XX escapes — matching the Rust fast path, whose output
    /// feeds resource lookups that use the same raw spelling.
    /// </summary>
    private static (string Name, int End) ParseNameFast(byte[] data, int start)
    {
        int i = start + 1;
        int nameStart = i;
        while (i < data.Length && !IsWhitespaceOrDelimiter(data[i]))
        {
            i++;
        }

        return (Latin1String(data, nameStart, i), i);
    }

    // ══ Graphics-region scanner ════════════════════════════════════════════

    private enum ScanKind
    {
        EndOfData,
        FoundBt,
        InlineImage,
        NeedFullParse,
        DeferredThenText,
        SimpleOp,
        TooManyErrors,
    }

    private readonly struct ScanResult
    {
        internal ScanResult(ScanKind kind, int rest = 0, int operandStart = 0, int afterOp = 0,
            int deferredStart = 0, int triggerStart = 0, OxOperator? op = null)
        {
            Kind = kind;
            Rest = rest;
            OperandStart = operandStart;
            AfterOp = afterOp;
            DeferredStart = deferredStart;
            TriggerStart = triggerStart;
            Op = op;
        }

        internal ScanKind Kind { get; }

        /// <summary>Index just past the recognised operator (FoundBt / InlineImage / SimpleOp).</summary>
        internal int Rest { get; }

        /// <summary>Index of the first pending operand, for a full re-parse.</summary>
        internal int OperandStart { get; }

        /// <summary>Fallback index past the operator name when the full parse fails.</summary>
        internal int AfterOp { get; }

        /// <summary>Index of the first deferred q, so its q/cm/Q run can be replayed.</summary>
        internal int DeferredStart { get; }

        /// <summary>Index of the operand start of the operator that ended the deferral.</summary>
        internal int TriggerStart { get; }

        internal OxOperator? Op { get; }
    }

    // Byte classes for the graphics scan. Over 80% of bytes in a graphics-heavy
    // stream are digits, dots and whitespace belonging to path coordinates, so
    // they get a single bulk-skip class.
    private const byte ScanSkip = 0;
    private const byte ScanAlpha = 1;
    private const byte ScanParen = 2;
    private const byte ScanAngle = 3;
    private const byte ScanBracket = 4;
    private const byte ScanSlash = 5;
    private const byte ScanPercent = 6;
    private const byte ScanOther = 7;

    private static readonly byte[] ByteClass = BuildByteClass();

    private static byte[] BuildByteClass()
    {
        var t = new byte[256];
        Array.Fill(t, ScanOther);

        foreach (byte ws in new byte[] { (byte)' ', (byte)'\t', (byte)'\n', (byte)'\r', 0x00, 0x0C })
        {
            t[ws] = ScanSkip;
        }

        for (byte c = (byte)'0'; c <= (byte)'9'; c++)
        {
            t[c] = ScanSkip;
        }

        t[(byte)'.'] = ScanSkip;
        t[(byte)'+'] = ScanSkip;
        t[(byte)'-'] = ScanSkip;

        for (byte c = (byte)'A'; c <= (byte)'Z'; c++)
        {
            t[c] = ScanAlpha;
        }

        for (byte c = (byte)'a'; c <= (byte)'z'; c++)
        {
            t[c] = ScanAlpha;
        }

        t[(byte)'\''] = ScanAlpha;
        t[(byte)'"'] = ScanAlpha;
        t[(byte)'*'] = ScanAlpha;

        t[(byte)'('] = ScanParen;
        t[(byte)'<'] = ScanAngle;
        t[(byte)'['] = ScanBracket;
        t[(byte)'/'] = ScanSlash;
        t[(byte)'%'] = ScanPercent;
        return t;
    }

    private static ScanResult ScanGraphicsRegion(byte[] data, int start, ref int consecutiveErrors)
    {
        int i = start;
        int operandStart = start;
        uint deferredDepth = 0;
        int deferredStart = start;
        int len = data.Length;

        while (true)
        {
            while (i < len && ByteClass[data[i]] == ScanSkip)
            {
                i++;
            }

            if (i >= len)
            {
                return new ScanResult(ScanKind.EndOfData);
            }

            switch (ByteClass[data[i]])
            {
                case ScanAlpha:
                {
                    byte firstByte = data[i];
                    bool secondIsNonAlpha = i + 1 >= len || ByteClass[data[i + 1]] != ScanAlpha;

                    // Fast path for single-char operators that never matter to text
                    // extraction. q/Q are excluded because they drive the deferred
                    // depth, and g/G/k/K because they mutate persistent colour state
                    // that must reach a later BT/Tj (see IsColorOp).
                    if (secondIsNonAlpha && IsSkippableSingleCharOp(firstByte))
                    {
                        i++;
                        consecutiveErrors = 0;
                        operandStart = i;
                        continue;
                    }

                    int opStart = i;
                    while (i < len && IsOperatorNameByte(data[i]))
                    {
                        i++;
                    }

                    if (Matches(data, opStart, i, "true") || Matches(data, opStart, i, "false") || Matches(data, opStart, i, "null"))
                    {
                        consecutiveErrors = 0;
                        continue;
                    }

                    consecutiveErrors = 0;

                    if (Matches(data, opStart, i, "q"))
                    {
                        if (deferredDepth == 0)
                        {
                            deferredStart = operandStart;
                        }

                        deferredDepth++;
                        operandStart = i;
                        continue;
                    }

                    if (Matches(data, opStart, i, "Q"))
                    {
                        if (deferredDepth > 0)
                        {
                            deferredDepth--;
                            operandStart = i;
                            continue;
                        }

                        // Unmatched Q has no operands, so emitting it directly avoids
                        // a full nom-style re-parse for a trivial operator.
                        return new ScanResult(ScanKind.SimpleOp, rest: i, op: OxOperator.RestoreState.Instance);
                    }

                    if (deferredDepth > 0)
                    {
                        if (Matches(data, opStart, i, "cm") || Matches(data, opStart, i, "gs")
                            || IsSkippableGraphicsOp(data, opStart, i))
                        {
                            operandStart = i;
                            continue;
                        }

                        return new ScanResult(ScanKind.DeferredThenText, deferredStart: deferredStart, triggerStart: operandStart);
                    }

                    if (Matches(data, opStart, i, "BT"))
                    {
                        return new ScanResult(ScanKind.FoundBt, rest: i);
                    }

                    if (Matches(data, opStart, i, "BI"))
                    {
                        return new ScanResult(ScanKind.InlineImage, rest: i);
                    }

                    if (Matches(data, opStart, i, "cm"))
                    {
                        if (ParseFloats(data, operandStart, opStart, 6, out float[] m))
                        {
                            return new ScanResult(ScanKind.SimpleOp, rest: i, op: new OxOperator.Cm(m[0], m[1], m[2], m[3], m[4], m[5]));
                        }

                        return new ScanResult(ScanKind.NeedFullParse, operandStart: operandStart, afterOp: i);
                    }

                    if (IsColorOp(data, opStart, i))
                    {
                        // Outside any q/Q scope nothing reverts this colour before the
                        // next BT — per §8.4 graphics state persists across BT/ET — so
                        // it must reach the handler instead of being dropped.
                        return new ScanResult(ScanKind.NeedFullParse, operandStart: operandStart, afterOp: i);
                    }

                    if (IsSkippableGraphicsOp(data, opStart, i))
                    {
                        operandStart = i;
                        continue;
                    }

                    return new ScanResult(ScanKind.NeedFullParse, operandStart: operandStart, afterOp: i);
                }

                case ScanParen:
                {
                    int end = SkipLiteralStringRaw(data, i);
                    if (end >= 0)
                    {
                        i = end;
                        consecutiveErrors = 0;
                    }
                    else
                    {
                        i++;
                        consecutiveErrors++;
                    }

                    break;
                }

                case ScanAngle:
                {
                    int end = i + 1 < len && data[i + 1] == (byte)'<'
                        ? SkipDictRaw(data, i)
                        : SkipHexStringRaw(data, i);
                    if (end >= 0)
                    {
                        i = end;
                        consecutiveErrors = 0;
                    }
                    else
                    {
                        i++;
                        consecutiveErrors++;
                    }

                    break;
                }

                case ScanBracket:
                {
                    int end = SkipArrayRaw(data, i);
                    if (end >= 0)
                    {
                        i = end;
                        consecutiveErrors = 0;
                    }
                    else
                    {
                        i++;
                        consecutiveErrors++;
                    }

                    break;
                }

                case ScanSlash:
                    i = SkipNameRaw(data, i);
                    consecutiveErrors = 0;
                    break;

                case ScanPercent:
                    while (i < len && data[i] != (byte)'\n' && data[i] != (byte)'\r')
                    {
                        i++;
                    }

                    consecutiveErrors = 0;
                    break;

                default:
                    i++;
                    consecutiveErrors++;
                    break;
            }

            if (consecutiveErrors >= MaxConsecutiveErrors)
            {
                return new ScanResult(ScanKind.TooManyErrors, rest: i);
            }
        }
    }

    private static bool IsSkippableSingleCharOp(byte b) => b is (byte)'m' or (byte)'l' or (byte)'c'
        or (byte)'v' or (byte)'y' or (byte)'h' or (byte)'f' or (byte)'F' or (byte)'B' or (byte)'b'
        or (byte)'S' or (byte)'s' or (byte)'n' or (byte)'W' or (byte)'w' or (byte)'d' or (byte)'i'
        or (byte)'J' or (byte)'j' or (byte)'M';

    /// <summary>
    /// Pure-graphics operators that text extraction can drop. The colour
    /// operators are included, which is only sound inside a deferred q/Q scope
    /// where a matching Q reverts the change before it can reach a BT — outside
    /// one, use <see cref="IsColorOp"/> instead.
    /// </summary>
    private static bool IsSkippableGraphicsOp(byte[] data, int start, int end)
    {
        int n = end - start;
        if (n is < 1 or > 3)
        {
            return false;
        }

        return MatchesAny(data, start, end, SkippableGraphicsOps);
    }

    private static readonly string[] SkippableGraphicsOps =
    {
        "m", "l", "c", "v", "y", "h", "re",                      // path construction
        "S", "s", "f", "F", "f*", "B", "B*", "b", "b*", "n",     // path painting
        "W", "W*",                                                // clipping
        "w", "J", "j", "M", "d", "i", "ri", "sh",                // non-text graphics state
        "rg", "RG", "g", "G", "k", "K",                          // colour
        "cs", "CS", "sc", "SC", "scn", "SCN",                    // colour space / components
    };

    /// <summary>
    /// Operators that set persistent fill/stroke colour state. Discarding these
    /// at the top level left the graphics state stuck on default black whenever
    /// a document set the fill colour before opening the text object — a common
    /// BDC, colour, BT, Tf, Tm, Tj pattern.
    /// </summary>
    private static bool IsColorOp(byte[] data, int start, int end)
    {
        int n = end - start;
        if (n is < 1 or > 3)
        {
            return false;
        }

        return MatchesAny(data, start, end, ColorOps);
    }

    private static readonly string[] ColorOps =
    {
        "rg", "RG", "g", "G", "k", "K", "cs", "CS", "sc", "SC", "scn", "SCN",
    };

    /// <summary>Text/colour-space/shading operators the paths-only parser drops.</summary>
    private static bool IsPathIrrelevantOp(byte[] data, int start, int end) =>
        MatchesAny(data, start, end, PathIrrelevantOps);

    private static readonly string[] PathIrrelevantOps =
    {
        "ET", "Tc", "Tw", "Tz", "TL", "Tf", "Tr", "Ts", "Td", "TD", "Tm", "Tj", "TJ", "T*",
        "cs", "CS", "sc", "SC", "scn", "SCN", "ri", "sh", "EI",
    };

    /// <summary>Compare an operator token against a set without materialising a string —
    /// this runs once per token across megabytes of path data.</summary>
    private static bool MatchesAny(byte[] data, int start, int end, string[] candidates)
    {
        foreach (string candidate in candidates)
        {
            if (Matches(data, start, end, candidate))
            {
                return true;
            }
        }

        return false;
    }

    /// <summary>Skip to just past the next real ET operator, or -1 if there is none.</summary>
    private static int ScanToEt(byte[] data, int start)
    {
        int i = start;
        while (i + 1 < data.Length)
        {
            if (data[i] == (byte)'E' && data[i + 1] == (byte)'T')
            {
                bool beforeOk = i == 0 || IsAsciiWhitespace(data[i - 1]) || data[i - 1] == (byte)')' || data[i - 1] == (byte)'>';
                bool afterOk = i + 2 >= data.Length || IsAsciiWhitespace(data[i + 2]) || data[i + 2] == (byte)'%';
                if (beforeOk && afterOk)
                {
                    return i + 2;
                }
            }

            // Strings can contain "ET" — step over them so it is not mistaken
            // for the operator.
            if (data[i] == (byte)'(')
            {
                i++;
                int depth = 1;
                while (i < data.Length && depth > 0)
                {
                    byte b = data[i];
                    if (b == (byte)'(')
                    {
                        depth++;
                    }
                    else if (b == (byte)')')
                    {
                        depth--;
                    }
                    else if (b == (byte)'\\')
                    {
                        i++;
                    }

                    i++;
                }

                continue;
            }

            if (data[i] == (byte)'<' && (i + 1 >= data.Length || data[i + 1] != (byte)'<'))
            {
                i++;
                while (i < data.Length && data[i] != (byte)'>')
                {
                    i++;
                }

                if (i < data.Length)
                {
                    i++;
                }

                continue;
            }

            i++;
        }

        return -1;
    }

    // ══ Raw skippers ═══════════════════════════════════════════════════════

    private static int SkipLiteralStringRaw(byte[] data, int i)
    {
        i++; // past '('
        uint depth = 1;
        while (i < data.Length && depth > 0)
        {
            byte b = data[i];
            if (b == (byte)'\\' && i + 1 < data.Length)
            {
                i += 2;
            }
            else if (b == (byte)'(')
            {
                depth++;
                i++;
            }
            else if (b == (byte)')')
            {
                depth--;
                i++;
            }
            else
            {
                i++;
            }
        }

        return depth == 0 ? i : -1;
    }

    private static int SkipHexStringRaw(byte[] data, int i)
    {
        i++; // past '<'
        while (i < data.Length)
        {
            if (data[i] == (byte)'>')
            {
                return i + 1;
            }

            i++;
        }

        return -1;
    }

    private static int SkipNameRaw(byte[] data, int i)
    {
        i++; // past '/'
        while (i < data.Length && !IsWhitespaceOrDelimiter(data[i]))
        {
            i++;
        }

        return i;
    }

    private static int SkipArrayRaw(byte[] data, int i)
    {
        int pos = i + 1; // past '['
        uint depth = 1;
        while (pos < data.Length && depth > 0)
        {
            byte b = data[pos];
            if (b == (byte)'[')
            {
                depth++;
                pos++;
            }
            else if (b == (byte)']')
            {
                depth--;
                pos++;
            }
            else if (b == (byte)'(')
            {
                pos++;
                uint strDepth = 1;
                while (pos < data.Length && strDepth > 0)
                {
                    byte c = data[pos];
                    if (c == (byte)'\\' && pos + 1 < data.Length)
                    {
                        pos += 2;
                    }
                    else if (c == (byte)'(')
                    {
                        strDepth++;
                        pos++;
                    }
                    else if (c == (byte)')')
                    {
                        strDepth--;
                        pos++;
                    }
                    else
                    {
                        pos++;
                    }
                }
            }
            else if (b == (byte)'<' && pos + 1 < data.Length && data[pos + 1] == (byte)'<')
            {
                pos += 2;
                uint dictDepth = 1;
                while (pos + 1 < data.Length && dictDepth > 0)
                {
                    if (data[pos] == (byte)'<' && data[pos + 1] == (byte)'<')
                    {
                        dictDepth++;
                        pos += 2;
                    }
                    else if (data[pos] == (byte)'>' && data[pos + 1] == (byte)'>')
                    {
                        dictDepth--;
                        pos += 2;
                    }
                    else
                    {
                        pos++;
                    }
                }
            }
            else if (b == (byte)'<')
            {
                pos++;
                while (pos < data.Length && data[pos] != (byte)'>')
                {
                    pos++;
                }

                if (pos < data.Length)
                {
                    pos++;
                }
            }
            else
            {
                pos++;
            }
        }

        return depth == 0 ? pos : -1;
    }

    private static int SkipDictRaw(byte[] data, int i)
    {
        int pos = i + 2; // past '<<'
        uint depth = 1;
        while (pos < data.Length && depth > 0)
        {
            if (pos + 1 < data.Length && data[pos] == (byte)'<' && data[pos + 1] == (byte)'<')
            {
                depth++;
                pos += 2;
            }
            else if (pos + 1 < data.Length && data[pos] == (byte)'>' && data[pos + 1] == (byte)'>')
            {
                depth--;
                pos += 2;
            }
            else if (data[pos] == (byte)'(')
            {
                pos++;
                uint strDepth = 1;
                while (pos < data.Length && strDepth > 0)
                {
                    byte c = data[pos];
                    if (c == (byte)'\\' && pos + 1 < data.Length)
                    {
                        pos += 2;
                    }
                    else if (c == (byte)'(')
                    {
                        strDepth++;
                        pos++;
                    }
                    else if (c == (byte)')')
                    {
                        strDepth--;
                        pos++;
                    }
                    else
                    {
                        pos++;
                    }
                }
            }
            else if (data[pos] == (byte)'<')
            {
                pos++;
                while (pos < data.Length && data[pos] != (byte)'>')
                {
                    pos++;
                }

                if (pos < data.Length)
                {
                    pos++;
                }
            }
            else
            {
                pos++;
            }
        }

        return depth == 0 ? pos : -1;
    }

    // ══ Generic operand reader (replaces lexer::token + parser::parse_object) ══

    /// <summary>
    /// Read one PDF object from the operand position. Mirrors
    /// `crate::parser::parse_object`, including its `n g R` lookahead — such a
    /// reference is illegal inside a content stream but the lookahead changes
    /// how many tokens are consumed, so it is reproduced here.
    /// </summary>
    private static bool TryParseOperand(byte[] data, int pos, out int next, out OxOperand? obj)
    {
        int i = SkipWhitespaceAndComments(data, pos);
        next = i;
        obj = null;

        if (i >= data.Length)
        {
            return false;
        }

        byte b = data[i];

        switch (b)
        {
            case (byte)'/':
            {
                (string name, int end) = ParseNameWithEscapes(data, i);
                obj = new OxOperand.Name(name);
                next = end;
                return true;
            }

            case (byte)'[':
                return TryParseArray(data, i + 1, out next, out obj);

            case (byte)'<':
                if (i + 1 < data.Length && data[i + 1] == (byte)'<')
                {
                    return TryParseDictionary(data, i + 2, out next, out obj);
                }

                int hexEnd = SkipHexStringRaw(data, i);
                if (hexEnd < 0)
                {
                    return false;
                }

                obj = new OxOperand.Str(DecodeHexString(data, i + 1, hexEnd - 1));
                next = hexEnd;
                return true;

            case (byte)'(':
            {
                int end = SkipLiteralStringRaw(data, i);
                if (end < 0)
                {
                    return false;
                }

                obj = new OxOperand.Str(DecodeLiteralStringEscapes(data, i + 1, end - 1));
                next = end;
                return true;
            }
        }

        if (IsAsciiDigit(b) || b == (byte)'+' || b == (byte)'-' || b == (byte)'.')
        {
            if (!TryParseNumberToken(data, i, out int numEnd, out OxOperand? number))
            {
                return false;
            }

            // Indirect-reference lookahead: `int int R`.
            if (number is OxOperand.Integer id)
            {
                int afterFirst = SkipWhitespaceAndComments(data, numEnd);
                if (afterFirst < data.Length && (IsAsciiDigit(data[afterFirst]) || data[afterFirst] == (byte)'+' || data[afterFirst] == (byte)'-')
                    && TryParseNumberToken(data, afterFirst, out int genEnd, out OxOperand? gen)
                    && gen is OxOperand.Integer generation)
                {
                    int afterGen = SkipWhitespaceAndComments(data, genEnd);
                    if (afterGen < data.Length && data[afterGen] == (byte)'R'
                        && (afterGen + 1 >= data.Length || !IsAsciiAlpha(data[afterGen + 1])))
                    {
                        obj = new OxOperand.Reference((uint)id.Value, (ushort)generation.Value);
                        next = afterGen + 1;
                        return true;
                    }
                }
            }

            obj = number;
            next = numEnd;
            return true;
        }

        if (StartsWith(data, i, "true"))
        {
            obj = new OxOperand.Bool(true);
            next = i + 4;
            return true;
        }

        if (StartsWith(data, i, "false"))
        {
            obj = new OxOperand.Bool(false);
            next = i + 5;
            return true;
        }

        if (StartsWith(data, i, "null"))
        {
            obj = OxOperand.Null.Instance;
            next = i + 4;
            return true;
        }

        return false;
    }

    private static bool TryParseArray(byte[] data, int pos, out int next, out OxOperand? obj)
    {
        var items = new List<OxOperand>();
        int i = pos;

        while (true)
        {
            i = SkipWhitespaceAndComments(data, i);
            if (i >= data.Length)
            {
                // Unclosed array at EOF: keep what was read rather than failing.
                obj = new OxOperand.Array(items);
                next = i;
                return true;
            }

            if (data[i] == (byte)']')
            {
                obj = new OxOperand.Array(items);
                next = i + 1;
                return true;
            }

            if (!TryParseOperand(data, i, out int afterItem, out OxOperand? item))
            {
                obj = null;
                next = i;
                return false;
            }

            items.Add(item!);
            i = afterItem;
        }
    }

    private static bool TryParseDictionary(byte[] data, int pos, out int next, out OxOperand? obj)
    {
        var entries = new Dictionary<string, OxOperand>(StringComparer.Ordinal);
        int i = pos;

        while (true)
        {
            i = SkipWhitespaceAndComments(data, i);
            if (i >= data.Length)
            {
                obj = new OxOperand.Dict(entries);
                next = i;
                return true;
            }

            if (i + 1 < data.Length && data[i] == (byte)'>' && data[i + 1] == (byte)'>')
            {
                obj = new OxOperand.Dict(entries);
                next = i + 2;
                return true;
            }

            if (data[i] != (byte)'/')
            {
                obj = null;
                next = i;
                return false;
            }

            (string key, int afterKey) = ParseNameWithEscapes(data, i);
            i = SkipWhitespaceAndComments(data, afterKey);

            if (TryParseOperand(data, i, out int afterValue, out OxOperand? value))
            {
                entries[key] = value!;
                i = afterValue;
                continue;
            }

            // Malformed PDFs write bare words where a name belongs (OBJR for
            // /OBJR); accept one as a name rather than dropping the dictionary.
            if (i < data.Length && IsAsciiAlphanumeric(data[i]))
            {
                int wordStart = i;
                while (i < data.Length && IsAsciiAlphanumeric(data[i]))
                {
                    i++;
                }

                entries[key] = new OxOperand.Name(AsciiString(data, wordStart, i));
                continue;
            }

            if (i >= data.Length)
            {
                obj = new OxOperand.Dict(entries);
                next = i;
                return true;
            }

            obj = null;
            next = i;
            return false;
        }
    }

    private static bool TryParseNumberToken(byte[] data, int pos, out int next, out OxOperand? number)
    {
        int i = pos;
        bool negative = false;
        bool hasSign = false;
        if (i < data.Length && (data[i] == (byte)'+' || data[i] == (byte)'-'))
        {
            negative = data[i] == (byte)'-';
            hasSign = true;
            i++;
        }

        int intStart = i;
        while (i < data.Length && IsAsciiDigit(data[i]))
        {
            i++;
        }

        int intEnd = i;
        bool hasFraction = false;
        int fracStart = i;
        int fracEnd = i;
        if (i < data.Length && data[i] == (byte)'.')
        {
            hasFraction = true;
            i++;
            fracStart = i;
            while (i < data.Length && IsAsciiDigit(data[i]))
            {
                i++;
            }

            fracEnd = i;
        }

        if (intEnd == intStart && !hasFraction)
        {
            // A bare sign shows up as a TJ offset in malformed streams like
            // `(v)-(e)`; the Rust lexer reads it as integer 0.
            if (hasSign)
            {
                number = new OxOperand.Integer(0);
                next = i;
                return true;
            }

            number = null;
            next = pos;
            return false;
        }

        if (hasFraction)
        {
            string text = (negative ? "-" : string.Empty)
                + (intEnd > intStart ? AsciiString(data, intStart, intEnd) : "0")
                + "."
                + (fracEnd > fracStart ? AsciiString(data, fracStart, fracEnd) : "0");
            if (!double.TryParse(text, NumberStyles.Float, CultureInfo.InvariantCulture, out double real))
            {
                number = null;
                next = pos;
                return false;
            }

            // ISO 32000-1 Annex C.2 Table C.1 bounds real values to ~±3.403e38;
            // an oversized literal saturates to infinity, which poisons later
            // arithmetic into NaN, so clamp to the implementation limit instead.
            if (double.IsInfinity(real))
            {
                real = Math.CopySign(float.MaxValue, real);
            }

            number = new OxOperand.Real(real);
            next = i;
            return true;
        }

        if (!long.TryParse(AsciiString(data, intStart, intEnd), NumberStyles.Integer, CultureInfo.InvariantCulture, out long value))
        {
            number = null;
            next = pos;
            return false;
        }

        number = new OxOperand.Integer(negative ? -value : value);
        next = i;
        return true;
    }

    /// <summary>
    /// Read a /Name, decoding #XX escapes per ISO 32000-1 §7.3.5. Invalid
    /// escapes are preserved verbatim.
    /// </summary>
    private static (string Name, int End) ParseNameWithEscapes(byte[] data, int start)
    {
        int i = start + 1;
        int nameStart = i;
        while (i < data.Length && !IsNameTerminator(data[i]))
        {
            i++;
        }

        string raw = Latin1String(data, nameStart, i);
        if (raw.IndexOf('#') < 0)
        {
            return (raw, i);
        }

        var sb = new System.Text.StringBuilder(raw.Length);
        for (int k = 0; k < raw.Length; k++)
        {
            if (raw[k] != '#')
            {
                sb.Append(raw[k]);
                continue;
            }

            if (k + 2 < raw.Length
                && byte.TryParse(raw.AsSpan(k + 1, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out byte decoded))
            {
                sb.Append((char)decoded);
                k += 2;
                continue;
            }

            sb.Append('#');
            if (k + 1 < raw.Length)
            {
                sb.Append(raw[k + 1]);
                k++;
                if (k + 1 < raw.Length)
                {
                    sb.Append(raw[k + 1]);
                    k++;
                }
            }
        }

        return (sb.ToString(), i);
    }

    /// <summary>
    /// Decode a hex string body. Whitespace is dropped; every other non-hex
    /// character counts as a nibble of value 0 and an odd digit count pads the
    /// last byte, per ISO 32000-1 §7.3.4.3.
    /// </summary>
    private static byte[] DecodeHexString(byte[] data, int start, int end)
    {
        var digits = new List<byte>(Math.Max(end - start, 0));
        for (int k = start; k < end; k++)
        {
            if (!IsSplitWhitespace(data[k]))
            {
                int nibble = HexNibble(data[k]);
                digits.Add((byte)(nibble < 0 ? 0 : nibble));
            }
        }

        var result = new byte[(digits.Count + 1) / 2];
        for (int k = 0; k < result.Length; k++)
        {
            int hi = digits[k * 2];
            int lo = (k * 2) + 1 < digits.Count ? digits[(k * 2) + 1] : 0;
            result[k] = (byte)((hi << 4) | lo);
        }

        return result;
    }

    /// <summary>
    /// Decode the escape sequences of a literal string body (ISO 32000-1
    /// §7.3.4.2). An unknown escape keeps its backslash, as the spec allows.
    /// </summary>
    private static byte[] DecodeLiteralStringEscapes(byte[] data, int start, int end)
    {
        var result = new List<byte>(end - start);
        int i = start;

        while (i < end)
        {
            if (data[i] != (byte)'\\' || i + 1 >= end)
            {
                result.Add(data[i]);
                i++;
                continue;
            }

            byte esc = data[i + 1];
            switch (esc)
            {
                case (byte)'n': result.Add((byte)'\n'); i += 2; break;
                case (byte)'r': result.Add((byte)'\r'); i += 2; break;
                case (byte)'t': result.Add((byte)'\t'); i += 2; break;
                case (byte)'b': result.Add(8); i += 2; break;
                case (byte)'f': result.Add(12); i += 2; break;
                case (byte)'(': result.Add((byte)'('); i += 2; break;
                case (byte)')': result.Add((byte)')'); i += 2; break;
                case (byte)'\\': result.Add((byte)'\\'); i += 2; break;
                case (byte)'\n': i += 2; break; // line continuation
                case (byte)'\r':
                    i += 2;
                    if (i < end && data[i] == (byte)'\n')
                    {
                        i++;
                    }

                    break;
                default:
                    if (esc >= (byte)'0' && esc < (byte)'8')
                    {
                        uint octal = 0;
                        int len = 0;
                        for (int j = 0; j < 3; j++)
                        {
                            int at = i + 1 + j;
                            if (at >= end || data[at] < (byte)'0' || data[at] >= (byte)'8')
                            {
                                break;
                            }

                            octal = (octal * 8) + (uint)(data[at] - (byte)'0');
                            len++;
                        }

                        if (len > 0)
                        {
                            result.Add((byte)(octal & 0xFF));
                            i += 1 + len;
                        }
                        else
                        {
                            result.Add((byte)'\\');
                            i++;
                        }
                    }
                    else
                    {
                        result.Add((byte)'\\');
                        i++;
                    }

                    break;
            }
        }

        return result.ToArray();
    }

    // ══ Byte helpers ═══════════════════════════════════════════════════════

    /// <summary>Token boundary for the prescan's BT/Do detection.</summary>
    private static bool IsPrescanBoundary(byte b) => IsSplitWhitespace(b) || IsNameDelimiter(b);

    /// <summary>Whitespace per PDF Table 1 (null, tab, LF, FF, CR, space).</summary>
    private static bool IsWhitespace(byte b) =>
        b is 0x00 or (byte)'\t' or (byte)'\r' or (byte)'\n' or 0x0C or (byte)' ';

    private static bool IsWhitespaceOrDelimiter(byte b) => IsWhitespace(b) || IsNameDelimiter(b);

    private static bool IsNameDelimiter(byte b) =>
        b is (byte)'(' or (byte)')' or (byte)'<' or (byte)'>' or (byte)'[' or (byte)']'
        or (byte)'{' or (byte)'}' or (byte)'/' or (byte)'%';

    private static bool IsNameTerminator(byte b) => IsAsciiWhitespace(b) || b == 0x00 || b == 0x0C || IsNameDelimiter(b);

    /// <summary>The whitespace set of the operator loop's leading skip: space, tab, CR, LF.</summary>
    private static bool IsAsciiWhitespace(byte b) => b is (byte)' ' or (byte)'\t' or (byte)'\r' or (byte)'\n';

    private static bool IsAsciiDigit(byte b) => b >= (byte)'0' && b <= (byte)'9';

    private static bool IsAsciiAlpha(byte b) =>
        (b >= (byte)'A' && b <= (byte)'Z') || (b >= (byte)'a' && b <= (byte)'z');

    private static bool IsAsciiAlphanumeric(byte b) => IsAsciiAlpha(b) || IsAsciiDigit(b);

    /// <summary>A byte that can begin an operator name.</summary>
    private static bool IsOperatorStart(byte b) =>
        IsAsciiAlpha(b) || b == (byte)'\'' || b == (byte)'"' || b == (byte)'*';

    private static bool IsOperatorNameByte(byte b) =>
        IsAsciiAlphanumeric(b) || b == (byte)'\'' || b == (byte)'"' || b == (byte)'*';

    private static int SkipMultispace(byte[] data, int i)
    {
        while (i < data.Length && IsAsciiWhitespace(data[i]))
        {
            i++;
        }

        return i;
    }

    private static int SkipWhitespaceAndComments(byte[] data, int i)
    {
        while (i < data.Length)
        {
            if (IsWhitespace(data[i]))
            {
                i++;
                continue;
            }

            if (data[i] == (byte)'%')
            {
                while (i < data.Length && data[i] != (byte)'\r' && data[i] != (byte)'\n')
                {
                    i++;
                }

                continue;
            }

            break;
        }

        return i;
    }

    private static bool Matches(byte[] data, int start, int end, string literal)
    {
        if (end - start != literal.Length)
        {
            return false;
        }

        for (int k = 0; k < literal.Length; k++)
        {
            if (data[start + k] != (byte)literal[k])
            {
                return false;
            }
        }

        return true;
    }

    private static bool StartsWith(byte[] data, int start, string literal)
    {
        if (data.Length - start < literal.Length)
        {
            return false;
        }

        for (int k = 0; k < literal.Length; k++)
        {
            if (data[start + k] != (byte)literal[k])
            {
                return false;
            }
        }

        return true;
    }

    private static string AsciiString(byte[] data, int start, int end) => Latin1String(data, start, end);

    private static string Latin1String(byte[] data, int start, int end)
    {
        if (end <= start)
        {
            return string.Empty;
        }

        return string.Create(end - start, (data, start), static (span, state) =>
        {
            for (int k = 0; k < span.Length; k++)
            {
                span[k] = (char)state.data[state.start + k];
            }
        });
    }

    private static byte[] Slice(byte[] data, int start, int end)
    {
        if (end <= start)
        {
            return Array.Empty<byte>();
        }

        var slice = new byte[end - start];
        Buffer.BlockCopy(data, start, slice, 0, slice.Length);
        return slice;
    }

    /// <summary>
    /// Read the first <paramref name="count"/> whitespace-separated floats from
    /// a raw operand run. Extra tokens beyond the requested count are ignored;
    /// a missing or unparseable one fails the whole read.
    /// </summary>
    private static bool ParseFloats(byte[] data, int start, int end, int count, out float[] values)
    {
        values = new float[count];
        int i = start;
        int produced = 0;

        while (produced < count)
        {
            while (i < end && IsSplitWhitespace(data[i]))
            {
                i++;
            }

            if (i >= end)
            {
                return false;
            }

            int tokenStart = i;
            while (i < end && !IsSplitWhitespace(data[i]))
            {
                i++;
            }

            if (!TryParseFloatToken(data, tokenStart, i, out values[produced]))
            {
                return false;
            }

            produced++;
        }

        return true;
    }

    /// <summary>ASCII whitespace as Rust's `u8::is_ascii_whitespace` defines it:
    /// space, tab, LF, FF, CR — notably not NUL or vertical tab.</summary>
    private static bool IsSplitWhitespace(byte b) =>
        b is (byte)' ' or (byte)'\t' or (byte)'\n' or (byte)'\r' or 0x0C;

    private static bool TryParseFloatToken(byte[] data, int start, int end, out float value)
    {
        value = 0f;
        if (end <= start)
        {
            return false;
        }

        for (int k = start; k < end; k++)
        {
            // Non-ASCII bytes make the run un-decodable, matching the Rust
            // `from_utf8(...).ok()?` bail-out.
            if (data[k] >= 0x80)
            {
                return false;
            }
        }

        return float.TryParse(AsciiString(data, start, end), NumberStyles.Float, CultureInfo.InvariantCulture, out value);
    }
}
