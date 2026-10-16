// The operator dispatch and the Form XObject walk, ported from pdf_oxide-0.3.77
// src/extractors/text.rs:
//   5510-6820  execute_operator
//   6821-6875  MAX_XOBJECT_DEPTH / MAX_XOBJECT_DECODES / resolve_xobject_ref
//   6879-7402  process_xobject
//   7404-7410  current_artifact_type
//
// Two helpers the dispatch reaches outside text.rs come with it, because the modules that
// own them upstream are not ported: `cmyk_to_rgb` (color/mod.rs:545-609, the 16-corner
// interpolation the whole port must agree on for span colour) and
// `PdfDocument::may_contain_text` (document.rs:9578, the pre-decode gate that keeps
// graphics-only forms from being parsed).
//
// The colour-component application is shared by sc/scn and SC/SCN. Upstream writes that
// switch out four times — the two `…ColorN` copies differ only in dropped log lines — so the
// port keeps one fill and one stroke copy and both operators call it. Every branch, its guard
// and its component-count fallback are as written.
using System;
using System.Collections.Generic;
using XRay.Content.Internal.Pdf;
using XRay.Content.Internal.PdfOxide.Content;
using XRay.Content.Internal.PdfOxide.Fonts;

namespace XRay.Content.Internal.PdfOxide.Text;

/// <summary>
/// The document-scoped Form XObject caches (`xobject_text_free_cache`,
/// `xobject_stream_cache`, `xobject_spans_cache`). They hang off `PdfDocument` upstream, and
/// the port's document layer is not pdf_oxide's, so the extractor reaches them through this
/// seam; the default implementation is per-extractor, which is correct but only ever warm
/// within one page.
/// </summary>
internal interface IOxXObjectCaches
{
    /// <summary>Forms already known to paint no text, so a second `Do` skips the decode.</summary>
    bool IsTextFree((int Number, int Generation) reference);

    void MarkTextFree((int Number, int Generation) reference);

    /// <summary>The decoded content stream of a form, avoiding a repeated FlateDecode.</summary>
    byte[]? GetStream((int Number, int Generation) reference);

    void PutStream((int Number, int Generation) reference, byte[] data);

    /// <summary>
    /// Spans a self-contained form produced under one CTM. The outer bool reports whether the
    /// pair was walked at all; a walked form that painted nothing caches null spans, which is
    /// what stops it being walked again.
    /// </summary>
    bool TryGetSpans(
        (int Number, int Generation, long M0, long M1, long M2, long M3, long M4, long M5) key,
        out List<OxTextSpan>? spans);

    void PutSpans(
        (int Number, int Generation, long M0, long M1, long M2, long M3, long M4, long M5) key,
        List<OxTextSpan>? spans);
}

internal sealed partial class OxTextExtractor
{
    // ---- execute_operator (text.rs:5510) ----------------------------------------

    /// <summary>
    /// Execute one content-stream operator, updating the graphics state and extracting text
    /// as appropriate.
    /// </summary>
    private int _cancelTick;

    internal void ExecuteOperator(OxOperator op)
    {
        Cancel.Every(_cancelTick++, 0xFF);
        switch (op)
        {
            // ── Text state ────────────────────────────────────────────────────

            case OxOperator.Tf tf:
            {
                // Many PDFs re-select the font they already have (a Tf after every q/Q), and
                // the flush plus lookup is only needed when something actually changed.
                var current = StateStack.Current;
                bool sameFont = current.FontSize == tf.Size && current.FontName == tf.Font;
                if (!sameFont)
                {
                    // The open buffer decodes its bytes with the font it was created under, so
                    // a font change has to end it — otherwise the tail of the run is read
                    // through the wrong /ToUnicode CMap.
                    FlushTjSpanBuffer();

                    CachedCurrentFont = Fonts.TryGetValue(tf.Font, out var font) ? font : null;

                    // Writing mode is cached on the graphics state so the advance path reads a
                    // single primitive instead of dereferencing the font per glyph.
                    byte newWmode = CachedCurrentFont?.Wmode ?? 0;

                    var state = StateStack.Current;
                    state.FontName = tf.Font;
                    state.FontSize = tf.Size;
                    state.TextWMode = newWmode;
                }
                break;
            }

            // ── Text positioning ──────────────────────────────────────────────

            case OxOperator.Tm tm:
            {
                // Many producers position every glyph with its own Tm+Tj, which would emit
                // thousands of one-character spans per page. A Tm that stays on the same line
                // under the same transform keeps the open buffer instead of flushing it.
                //
                // The baseline tolerance is scale-relative rather than a rounded compare:
                // Word emits each glyph in its own BT/Tm/Tj block with a few points of
                // sinusoidal baseline jitter, and §9.4 leaves logical reading order to the
                // extractor, so a delta far below the line's own height is the same visual
                // line. Only a delta on the order of the font size is a real line break.
                float curFontSize = StateStack.Current.FontSize;
                bool isContinuation = false;
                if (MergingConfig.MergeTmTjRuns && TjSpanBuffer is { } buffer && !buffer.IsEmpty)
                {
                    var start = buffer.StartMatrix;
                    float tolerance = MathF.Max(MathF.Abs(curFontSize * start.D) * 0.5f, 0.5f);
                    if (MathF.Abs(tm.F - start.F) <= tolerance
                        && tm.A == start.A
                        && tm.B == start.B
                        && tm.C == start.C
                        && tm.D == start.D
                        && tm.E >= start.E)
                    {
                        // Same line, same transform, left-to-right progression: the buffer's
                        // width becomes the run's actual visual extent.
                        buffer.AccumulatedWidth = tm.E - start.E;
                        isContinuation = true;
                    }
                }

                if (!isContinuation)
                {
                    FlushTjSpanBuffer();
                }

                var state = StateStack.Current;
                state.TextMatrix = new OxMatrix(tm.A, tm.B, tm.C, tm.D, tm.E, tm.F);
                state.TextLineMatrix = state.TextMatrix;
                break;
            }

            case OxOperator.Td td:
            {
                FlushTjSpanBuffer();
                var state = StateStack.Current;

                // §9.4.2 Table 108: Tlm' = T(tx,ty) × Tlm. The translation is in text-line
                // space, so it is pre-multiplied and picks up the existing Tlm transform.
                state.TextLineMatrix = OxMatrix.Translation(td.Tx, td.Ty).Multiply(state.TextLineMatrix);
                state.TextMatrix = state.TextLineMatrix;
                break;
            }

            case OxOperator.TD tdl:
            {
                FlushTjSpanBuffer();

                // TD is Td plus the leading it implies.
                var state = StateStack.Current;
                state.Leading = -tdl.Ty;
                state.TextLineMatrix = OxMatrix.Translation(tdl.Tx, tdl.Ty).Multiply(state.TextLineMatrix);
                state.TextMatrix = state.TextLineMatrix;
                break;
            }

            case OxOperator.TStar:
            {
                FlushTjSpanBuffer();

                float leading = StateStack.Current.Leading;
                var state = StateStack.Current;
                state.TextLineMatrix = OxMatrix.Translation(0.0f, -leading).Multiply(state.TextLineMatrix);
                state.TextMatrix = state.TextLineMatrix;
                break;
            }

            // ── Text showing ──────────────────────────────────────────────────

            case OxOperator.Tj tj:
            {
                // /Artifact content is deliberately NOT skipped here: many PDFs mark real page
                // content as an artifact, and a tagged PDF's structure tree already excludes
                // true artifacts through the MCID mapping.
                (string? currentAt, bool alreadyEmitted) = PeekCurrentActualText();
                if (currentAt is { } actualText)
                {
                    if (alreadyEmitted)
                    {
                        // A later showing operator in the same scope: its glyphs are already
                        // covered by the one replacement the first one emitted. Positioning
                        // still advances so outer-scope text lands correctly.
                        float w = AdvancePositionForString(tj.Text);
                        if (TjSpanBuffer is { } buffer)
                        {
                            buffer.AccumulatedWidth += w;
                        }
                    }
                    else
                    {
                        MarkActualTextEmitted();
                        if (ExtractSpans)
                        {
                            // The replacement is already Unicode, so it goes straight into the
                            // buffer rather than through font character mapping.
                            TjSpanBuffer ??= OxTextDecoding.NewTjBuffer(
                                StateStack.Current, CurrentMcid, CachedCurrentFont);
                            TjSpanBuffer.Unicode.Append(actualText);
                        }
                        else
                        {
                            // Character mode maps through the font, which the replacement does
                            // not need; show_text is reached anyway so positioning holds.
                            ShowText(System.Text.Encoding.UTF8.GetBytes(actualText));
                        }

                        // The original text still sets the advance, so layout is unchanged.
                        float w = AdvancePositionForString(tj.Text);
                        if (TjSpanBuffer is { } buffer)
                        {
                            buffer.AccumulatedWidth += w;
                        }
                    }
                }
                else if (ExtractSpans)
                {
                    // Consecutive Tj operators accumulate into one span: §9.4.4 NOTE 6 asks
                    // for text strings as long as possible.
                    TjSpanBuffer ??= OxTextDecoding.NewTjBuffer(
                        StateStack.Current, CurrentMcid, CachedCurrentFont);
                    AppendAndAdvance(tj.Text);
                }
                else
                {
                    ShowText(tj.Text);
                }
                break;
            }

            case OxOperator.TJ tjArray:
            {
                (string? currentAt, bool alreadyEmitted) = PeekCurrentActualText();
                if (currentAt is { } actualText)
                {
                    if (!alreadyEmitted)
                    {
                        MarkActualTextEmitted();
                        if (ExtractSpans)
                        {
                            var buffer = OxTextDecoding.NewTjBuffer(
                                StateStack.Current, CurrentMcid, CachedCurrentFont);
                            buffer.Unicode.Append(actualText);
                            FlushTjBuffer(buffer);
                        }
                        else
                        {
                            ShowText(System.Text.Encoding.UTF8.GetBytes(actualText));
                        }
                    }

                    // First or later, the whole array still advances the position.
                    foreach (var element in tjArray.Array)
                    {
                        switch (element)
                        {
                            case OxTextElement.Str s:
                            {
                                float w = AdvancePositionForString(s.Bytes);
                                if (TjSpanBuffer is { } buffer)
                                {
                                    buffer.AccumulatedWidth += w;
                                }
                                break;
                            }

                            case OxTextElement.Offset offset:
                                AdvancePositionForOffset(offset.Value);
                                break;
                        }
                    }
                }
                else if (ExtractSpans)
                {
                    // One span per logical text unit rather than one per array element
                    // (§9.4.4 NOTE 6).
                    ProcessTjArray(tjArray.Array);
                }
                else
                {
                    foreach (var element in tjArray.Array)
                    {
                        switch (element)
                        {
                            case OxTextElement.Str s:
                                ShowText(s.Bytes);
                                break;

                            case OxTextElement.Offset offsetElement:
                                ShowTjOffsetAsChar(offsetElement.Value);
                                break;
                        }
                    }
                }
                break;
            }

            case OxOperator.Quote quote:
            {
                // ' is T* followed by Tj, so the pending run ends at the line break.
                FlushTjSpanBuffer();

                float leading = StateStack.Current.Leading;
                {
                    var state = StateStack.Current;
                    state.TextLineMatrix = OxMatrix.Translation(0.0f, -leading).Multiply(state.TextLineMatrix);
                    state.TextMatrix = state.TextLineMatrix;
                }

                if (ExtractSpans)
                {
                    TjSpanBuffer ??= OxTextDecoding.NewTjBuffer(
                        StateStack.Current, CurrentMcid, CachedCurrentFont);
                    AppendAndAdvance(quote.Text);
                }
                else
                {
                    ShowText(quote.Text);
                }
                break;
            }

            case OxOperator.DoubleQuote dq:
            {
                // " sets the two spacings, then behaves as '.
                FlushTjSpanBuffer();

                {
                    var state = StateStack.Current;
                    state.WordSpace = dq.WordSpace;
                    state.CharSpace = dq.CharSpace;
                    float leading = state.Leading;
                    state.TextLineMatrix = OxMatrix.Translation(0.0f, -leading).Multiply(state.TextLineMatrix);
                    state.TextMatrix = state.TextLineMatrix;
                }

                if (ExtractSpans)
                {
                    TjSpanBuffer ??= OxTextDecoding.NewTjBuffer(
                        StateStack.Current, CurrentMcid, CachedCurrentFont);
                    AppendAndAdvance(dq.Text);
                }
                else
                {
                    ShowText(dq.Text);
                }
                break;
            }

            // ── Text state parameters ─────────────────────────────────────────

            case OxOperator.Tc tc:
                StateStack.Current.CharSpace = tc.CharSpace;
                break;

            case OxOperator.Tw tw:
                StateStack.Current.WordSpace = tw.WordSpace;
                break;

            case OxOperator.Tz tz:
                StateStack.Current.HorizontalScaling = tz.Scale;
                break;

            case OxOperator.TL tl:
                StateStack.Current.Leading = tl.Leading;
                break;

            case OxOperator.Ts ts:
                StateStack.Current.TextRise = ts.Rise;
                break;

            case OxOperator.Tr tr:
                StateStack.Current.RenderMode = tr.Render;
                break;

            // ── Graphics state ────────────────────────────────────────────────

            case OxOperator.SaveState:
                // q/Q wraps a graphics-state block, and the matching Q can restore an earlier
                // CTM — leaving the user-space position the buffer captured out of step with
                // the active one. Each block therefore emits its own cluster.
                FlushTjSpanBuffer();
                StateStack.Save();
                break;

            case OxOperator.RestoreState:
            {
                FlushTjSpanBuffer();
                StateStack.Restore();

                var restored = StateStack.Current;
                CachedCurrentFont = restored.FontName is { } fontName && Fonts.TryGetValue(fontName, out var font)
                    ? font
                    : null;

                // The restored colour space may or may not be an excluded ink.
                if (ExcludedInks.Count > 0)
                {
                    InsideExcludedInk = IsExcludedInkColorSpace(restored.FillColorSpace);
                }
                break;
            }

            case OxOperator.Cm cm:
            {
                // The buffer captured its user-space position and horizontal scale from the
                // CTM in force when it was created. Non-conforming producers do issue cm
                // inside a text object — figure and chart text alternates cm for position with
                // text operators in one BT/ET block — and without a flush the following glyphs
                // are positioned under the new CTM while the buffer still reports the old one,
                // which drops the cluster off the page. §9.4 does not formally allow cm there,
                // but a conforming reader must process it.
                FlushTjSpanBuffer();
                var state = StateStack.Current;

                // §8.3.4: cm concatenates as M_cm × CTM.
                state.Ctm = new OxMatrix(cm.A, cm.B, cm.C, cm.D, cm.E, cm.F).Multiply(state.Ctm);
                break;
            }

            // ── Colour ────────────────────────────────────────────────────────

            case OxOperator.SetFillRgb rgb:
                // rg implicitly selects DeviceRGB, a process colour.
                InsideExcludedInk = false;
                StateStack.Current.FillColorRgb = (rgb.R, rgb.G, rgb.B);
                break;

            case OxOperator.SetStrokeRgb srgb:
                StateStack.Current.StrokeColorRgb = (srgb.R, srgb.G, srgb.B);
                break;

            case OxOperator.SetFillGray gray:
                // g implicitly selects DeviceGray, so any active ink exclusion is cleared.
                InsideExcludedInk = false;
                StateStack.Current.FillColorRgb = (gray.Gray, gray.Gray, gray.Gray);
                break;

            case OxOperator.SetStrokeGray sgray:
                StateStack.Current.StrokeColorRgb = (sgray.Gray, sgray.Gray, sgray.Gray);
                break;

            case OxOperator.SetFillCmyk cmyk:
            {
                // k implicitly selects DeviceCMYK, a process colour.
                InsideExcludedInk = false;
                var state = StateStack.Current;
                state.FillColorCmyk = (cmyk.C, cmyk.M, cmyk.Y, cmyk.K);
                state.FillColorRgb = CmykToRgb(cmyk.C, cmyk.M, cmyk.Y, cmyk.K);
                break;
            }

            case OxOperator.SetStrokeCmyk scmyk:
            {
                var state = StateStack.Current;
                state.StrokeColorCmyk = (scmyk.C, scmyk.M, scmyk.Y, scmyk.K);
                state.StrokeColorRgb = CmykToRgb(scmyk.C, scmyk.M, scmyk.Y, scmyk.K);
                break;
            }

            case OxOperator.SetFillColorSpace cs:
            {
                // Resolved before the state is mutated, because it reads the resources.
                InsideExcludedInk = IsExcludedInkColorSpace(cs.Name);

                var state = StateStack.Current;
                state.FillColorSpace = cs.Name;

                // Changing colour space resets the colour.
                state.FillColorRgb = (0.0f, 0.0f, 0.0f);
                state.FillColorCmyk = null;
                break;
            }

            case OxOperator.SetStrokeColorSpace scs:
            {
                var state = StateStack.Current;
                state.StrokeColorSpace = scs.Name;
                state.StrokeColorRgb = (0.0f, 0.0f, 0.0f);
                state.StrokeColorCmyk = null;
                break;
            }

            case OxOperator.SetFillColor fc:
                ApplyFillColorComponents(fc.Components);
                break;

            case OxOperator.SetStrokeColor sc:
                ApplyStrokeColorComponents(sc.Components);
                break;

            case OxOperator.SetFillColorN fcn:
                // A pattern colour space names its pattern instead of giving components; the
                // text extractor has no use for the pattern itself.
                if (fcn.Name is null)
                {
                    ApplyFillColorComponents(fcn.Components);
                }
                break;

            case OxOperator.SetStrokeColorN scn:
                if (scn.Name is null)
                {
                    ApplyStrokeColorComponents(scn.Components);
                }
                break;

            // ── Line style ────────────────────────────────────────────────────

            case OxOperator.SetLineCap lineCap:
                StateStack.Current.LineCap = lineCap.CapStyle;
                break;

            case OxOperator.SetLineJoin lineJoin:
                StateStack.Current.LineJoin = lineJoin.JoinStyle;
                break;

            case OxOperator.SetMiterLimit miter:
                StateStack.Current.MiterLimit = miter.Limit;
                break;

            case OxOperator.SetRenderingIntent intent:
                StateStack.Current.RenderingIntent = intent.Intent;
                break;

            case OxOperator.SetFlatness flatness:
                StateStack.Current.Flatness = flatness.Tolerance;
                break;

            // ── Text objects ──────────────────────────────────────────────────

            case OxOperator.BeginText:
            {
                // §9.4.1: Tm and Tlm are the identity at the start of a text object.
                var state = StateStack.Current;
                state.TextMatrix = OxMatrix.Identity;
                state.TextLineMatrix = OxMatrix.Identity;
                break;
            }

            case OxOperator.EndText:
                FlushTjSpanBuffer();
                break;

            // ── Marked content (§14.6) ────────────────────────────────────────

            case OxOperator.BeginMarkedContent bmc:
            {
                // The buffer is flushed at every marked-content boundary. Without it,
                // consecutive Tj operators straddling a BMC/BDC/EMC glue into one span whose
                // MCID is only the first one's — fusing two structurally distinct elements and
                // breaking everything downstream that relies on MCID identity: structure-tree
                // reading order, tree-scope /ActualText suppression, table-cell membership.
                FlushTjSpanBuffer();

                if (bmc.Tag == "ReversedChars")
                {
                    SawReversedChars = true;
                }

                // BMC carries no properties, but the tag alone can mark an artifact.
                bool isArtifact = bmc.Tag == "Artifact";
                bool isPlacedPdf = bmc.Tag == "PlacedPDF";
                MarkedContentStack.Add(new OxMarkedContentContext
                {
                    Tag = bmc.Tag,
                    IsArtifact = isArtifact,
                    IsPlacedPdf = isPlacedPdf,
                });
                UpdateArtifactState();
                UpdateLayerState();
                break;
            }

            case OxOperator.BeginMarkedContentDict bdc:
            {
                // Same boundary reasoning as BMC.
                FlushTjSpanBuffer();

                string? actualText = null;
                (OxArtifactType Type, OxPaginationSubtype? Subtype)? artifactType = null;
                string? expansion = null;
                int? ownMcid = null;
                bool isExcludedLayer = false;

                if (ResolveBdcProperties(bdc.Properties) is { } propsDict)
                {
                    if (propsDict.Get("MCID").AsLong() is { } mcid)
                    {
                        ownMcid = (int)mcid;
                        CurrentMcid = (int)mcid;
                    }

                    if (propsDict.Get("ActualText").AsStringBytes() is { } actualTextBytes)
                    {
                        actualText = DecodePdfTextString(actualTextBytes);

                        // The in-stream /ActualText is the authoritative replacement for this
                        // MCID, so an ancestor StructElem's must not override it later.
                        if (CurrentMcid is { } currentMcid)
                        {
                            McActualTextMcids.Add(currentMcid);
                        }
                    }

                    // /E expands an abbreviation or acronym (§14.9.5).
                    if (propsDict.Get("E").AsStringBytes() is { } expansionBytes)
                    {
                        expansion = DecodePdfTextString(expansionBytes);
                    }

                    if (bdc.Tag == "Artifact")
                    {
                        artifactType = ParseArtifactType(propsDict);
                    }

                    // Optional content (§8.11.2): a direct OCG carries /Name, an OCMD carries
                    // /OCGs with a visibility policy.
                    if (bdc.Tag == "OC" && ExcludedLayers.Count > 0)
                    {
                        isExcludedLayer = CheckOcgExcluded(propsDict);
                    }
                }

                bool isArtifact = bdc.Tag == "Artifact";
                bool isPlacedPdf = bdc.Tag == "PlacedPDF";
                MarkedContentStack.Add(new OxMarkedContentContext
                {
                    Tag = bdc.Tag,
                    IsArtifact = isArtifact,
                    ArtifactType = artifactType?.Type,
                    ActualText = actualText,
                    Expansion = expansion,
                    IsExcludedLayer = isExcludedLayer,
                    IsPlacedPdf = isPlacedPdf,
                    OwnMcid = ownMcid,
                });
                UpdateArtifactState();
                UpdateLayerState();
                break;
            }

            case OxOperator.EndMarkedContent:
            {
                FlushTjSpanBuffer();

                // Pop first, then restore the MCID from the nearest enclosing BDC that carried
                // one. Marked-content sequences nest (§14.6), so a Tj issued after an inner EMC
                // still belongs to its enclosing scope; blanking the MCID here would orphan
                // that span.
                if (MarkedContentStack.Count > 0)
                {
                    MarkedContentStack.RemoveAt(MarkedContentStack.Count - 1);
                    UpdateArtifactState();
                    UpdateLayerState();
                }

                int? restored = null;
                for (int i = MarkedContentStack.Count - 1; i >= 0; i--)
                {
                    if (MarkedContentStack[i].OwnMcid is { } mcid)
                    {
                        restored = mcid;
                        break;
                    }
                }
                CurrentMcid = restored;
                break;
            }

            // ── XObjects ──────────────────────────────────────────────────────

            case OxOperator.Do doOp:
                // `ProcessXObject` applies the form's /Matrix to the CTM (§8.10.1) and may run
                // cm/Tm inside the form, so the position the buffer captured would no longer
                // match the CTM the form's text is emitted under.
                FlushTjSpanBuffer();
                ProcessXObject(doOp.Name);
                break;

            // Everything else — path construction and painting, inline images, gs and sh —
            // carries no text and leaves no state the extractor reads.
            default:
                break;
        }
    }

    /// <summary>
    /// The character-mode handling of a TJ numeric offset (text.rs:5771-5896): a significant
    /// negative offset becomes a space glyph.
    ///
    /// PDFs commonly write inter-word space as a TJ offset rather than a space character —
    /// `[(Text1) -200 (Text2)] TJ` — and §9.4.4 defines the positioning but not where a word
    /// boundary is, so the threshold is font-metric derived rather than fixed.
    /// </summary>
    private void ShowTjOffsetAsChar(float offset)
    {
        var state = StateStack.Current;
        float tx = -offset / 1000.0f * state.FontSize * state.HorizontalScaling / 100.0f;

        float threshold = CalculateAdaptiveTjThreshold();
        if (offset < threshold)
        {
            var textMatrix = state.TextMatrix;
            var ctm = state.Ctm;
            string? fontName = state.FontName;
            float fontSize = state.FontSize;
            var fillColorRgb = state.FillColorRgb;

            // The rendered size includes the CTM and text-matrix scaling.
            var combined = ctm.Multiply(textMatrix);
            float effectiveFontSize = fontSize * MathF.Sqrt((combined.D * combined.D) + (combined.B * combined.B));

            OxFontInfo? font = null;
            if (fontName is not null)
            {
                Fonts.TryGetValue(fontName, out font);
            }
            var fontWeight = font is not null && font.IsBold() ? OxFontWeight.Bold : OxFontWeight.Normal;

            var textPos = textMatrix.TransformPoint(0.0f, 0.0f);
            var pos = ctm.TransformPoint(textPos.X, textPos.Y);
            bool isItalicSpace = font is not null && font.IsItalic();
            string fontNameStr = fontName ?? "";
            var finalMatrix = ctm.Multiply(textMatrix);
            float rotationDegrees = MathF.Atan2(finalMatrix.B, finalMatrix.A) * (180.0f / MathF.PI);

            var spaceChar = new OxTextChar
            {
                Char = ' ',
                Bbox = new OxRect(pos.X, pos.Y, MathF.Abs(tx), effectiveFontSize),
                FontName = fontNameStr,
                FontSize = effectiveFontSize,
                FontWeight = fontWeight,
                Color = new OxColor(fillColorRgb.R, fillColorRgb.G, fillColorRgb.B),
                Mcid = CurrentMcid,
                IsItalic = isItalicSpace,
                IsMonospace = false,
                OriginX = pos.X,
                OriginY = pos.Y,
                RotationDegrees = rotationDegrees,
                AdvanceWidth = MathF.Abs(tx),
                RenderedAdvance = MathF.Abs(tx),
                Ascent = (font?.Ascent ?? 0.95f) * effectiveFontSize,
                Descent = (font?.Descent ?? -0.35f) * effectiveFontSize,
                Matrix = new[]
                {
                    finalMatrix.A, finalMatrix.B, finalMatrix.C,
                    finalMatrix.D, finalMatrix.E, finalMatrix.F,
                },
            };
            if (!IsContentSuppressed())
            {
                Chars.Add(spaceChar);
            }
        }

        // Routed through the state's own advance so the horizontal/vertical axis swap lives in
        // one place: §9.4.4 shifts a TJ offset along the active writing axis.
        StateStack.Current.AdvanceTextMatrix(tx);
    }

    // ---- colour components (text.rs:6074-6511) ----------------------------------

    /// <summary>Set the fill colour from components in the current fill colour space.</summary>
    private void ApplyFillColorComponents(List<float> components)
    {
        var state = StateStack.Current;
        switch (state.FillColorSpace)
        {
            case "DeviceGray" or "CalGray" when components.Count == 1:
                state.FillColorRgb = (components[0], components[0], components[0]);
                break;

            case "DeviceRGB" or "CalRGB" when components.Count == 3:
                state.FillColorRgb = (components[0], components[1], components[2]);
                break;

            case "Lab" when components.Count == 3:
                // A faithful L*a*b* conversion needs the whitepoint; lightness alone is a
                // grayscale approximation.
                state.FillColorRgb = (components[0] / 100.0f, components[0] / 100.0f, components[0] / 100.0f);
                break;

            case "DeviceCMYK" when components.Count == 4:
                state.FillColorCmyk = (components[0], components[1], components[2], components[3]);
                state.FillColorRgb = CmykToRgb(components[0], components[1], components[2], components[3]);
                break;

            case "ICCBased":
                // The ICC profile itself is not processed; the component count says which
                // device space it stands in for.
                if (components.Count == 3)
                {
                    state.FillColorRgb = (components[0], components[1], components[2]);
                }
                else if (components.Count == 1)
                {
                    state.FillColorRgb = (components[0], components[0], components[0]);
                }
                else if (components.Count == 4)
                {
                    state.FillColorCmyk = (components[0], components[1], components[2], components[3]);
                    state.FillColorRgb = CmykToRgb(components[0], components[1], components[2], components[3]);
                }
                break;

            case "Separation" when components.Count == 1:
                // The component is a tint: no ink is paper, full ink is solid.
                state.FillColorRgb = (1.0f - components[0], 1.0f - components[0], 1.0f - components[0]);
                break;

            case "DeviceN" when components.Count > 0:
                if (components.Count == 4)
                {
                    state.FillColorCmyk = (components[0], components[1], components[2], components[3]);
                    state.FillColorRgb = CmykToRgb(components[0], components[1], components[2], components[3]);
                }
                else
                {
                    state.FillColorRgb = (1.0f - components[0], 1.0f - components[0], 1.0f - components[0]);
                }
                break;

            default:
                // A named colour space ("Cs1") or an unknown one: fall back by component count.
                switch (components.Count)
                {
                    case 1:
                        state.FillColorRgb = (components[0], components[0], components[0]);
                        break;
                    case 3:
                        state.FillColorRgb = (components[0], components[1], components[2]);
                        break;
                    case 4:
                        state.FillColorCmyk = (components[0], components[1], components[2], components[3]);
                        state.FillColorRgb = CmykToRgb(components[0], components[1], components[2], components[3]);
                        break;
                }
                break;
        }
    }

    /// <summary>Set the stroke colour from components in the current stroke colour space.</summary>
    private void ApplyStrokeColorComponents(List<float> components)
    {
        var state = StateStack.Current;
        switch (state.StrokeColorSpace)
        {
            case "DeviceGray" or "CalGray" when components.Count == 1:
                state.StrokeColorRgb = (components[0], components[0], components[0]);
                break;

            case "DeviceRGB" or "CalRGB" when components.Count == 3:
                state.StrokeColorRgb = (components[0], components[1], components[2]);
                break;

            case "Lab" when components.Count == 3:
                state.StrokeColorRgb = (components[0] / 100.0f, components[0] / 100.0f, components[0] / 100.0f);
                break;

            case "DeviceCMYK" when components.Count == 4:
                state.StrokeColorCmyk = (components[0], components[1], components[2], components[3]);
                state.StrokeColorRgb = CmykToRgb(components[0], components[1], components[2], components[3]);
                break;

            case "ICCBased":
                if (components.Count == 3)
                {
                    state.StrokeColorRgb = (components[0], components[1], components[2]);
                }
                else if (components.Count == 1)
                {
                    state.StrokeColorRgb = (components[0], components[0], components[0]);
                }
                else if (components.Count == 4)
                {
                    state.StrokeColorCmyk = (components[0], components[1], components[2], components[3]);
                    state.StrokeColorRgb = CmykToRgb(components[0], components[1], components[2], components[3]);
                }
                break;

            case "Separation" when components.Count == 1:
                state.StrokeColorRgb = (1.0f - components[0], 1.0f - components[0], 1.0f - components[0]);
                break;

            case "DeviceN" when components.Count > 0:
                if (components.Count == 4)
                {
                    state.StrokeColorCmyk = (components[0], components[1], components[2], components[3]);
                    state.StrokeColorRgb = CmykToRgb(components[0], components[1], components[2], components[3]);
                }
                else
                {
                    state.StrokeColorRgb = (1.0f - components[0], 1.0f - components[0], 1.0f - components[0]);
                }
                break;

            default:
                switch (components.Count)
                {
                    case 1:
                        state.StrokeColorRgb = (components[0], components[0], components[0]);
                        break;
                    case 3:
                        state.StrokeColorRgb = (components[0], components[1], components[2]);
                        break;
                    case 4:
                        state.StrokeColorCmyk = (components[0], components[1], components[2], components[3]);
                        state.StrokeColorRgb = CmykToRgb(components[0], components[1], components[2], components[3]);
                        break;
                }
                break;
        }
    }

    /// <summary>
    /// The measured corners of the CMYK cube (color/mod.rs:545). A per-channel `1-x`
    /// conversion turns rich blacks into muddy grey; interpolating between the corners a
    /// press actually prints keeps composite black black and cyan cyan.
    /// </summary>
    private static readonly float[][] CmykCorners =
    {
        new[] { 1.0f, 1.0f, 1.0f },          // 0000 paper
        new[] { 0.1373f, 0.1216f, 0.1255f }, // 000K
        new[] { 1.0f, 0.9490f, 0.0f },       // 00Y0 yellow
        new[] { 0.1098f, 0.1020f, 0.0f },    // 00YK
        new[] { 0.9255f, 0.0f, 0.5490f },    // 0M00 magenta
        new[] { 0.1412f, 0.0f, 0.0f },       // 0M0K
        new[] { 0.9294f, 0.1098f, 0.1412f }, // 0MY0 red
        new[] { 0.1333f, 0.0f, 0.0f },       // 0MYK
        new[] { 0.0f, 0.6784f, 0.9373f },    // C000 cyan
        new[] { 0.0f, 0.0588f, 0.1412f },    // C00K
        new[] { 0.0f, 0.6510f, 0.3137f },    // C0Y0 green
        new[] { 0.0f, 0.0745f, 0.0f },       // C0YK
        new[] { 0.1804f, 0.1922f, 0.5725f }, // CM00 blue
        new[] { 0.0f, 0.0f, 0.0078f },       // CM0K
        new[] { 0.2118f, 0.2118f, 0.2235f }, // CMY0 composite black
        new[] { 0.0f, 0.0f, 0.0f },          // CMYK registration
    };

    /// <summary>Quadrilinear interpolation over <see cref="CmykCorners"/> (color/mod.rs:594).</summary>
    private static (float R, float G, float B) CmykToRgb(float c, float m, float y, float k)
    {
        c = Math.Clamp(c, 0.0f, 1.0f);
        m = Math.Clamp(m, 0.0f, 1.0f);
        y = Math.Clamp(y, 0.0f, 1.0f);
        k = Math.Clamp(k, 0.0f, 1.0f);

        var acc = new float[3];
        for (int i = 0; i < CmykCorners.Length; i++)
        {
            float w = ((i & 8) != 0 ? c : 1.0f - c)
                * ((i & 4) != 0 ? m : 1.0f - m)
                * ((i & 2) != 0 ? y : 1.0f - y)
                * ((i & 1) != 0 ? k : 1.0f - k);
            if (w == 0.0f)
            {
                continue;
            }
            for (int j = 0; j < 3; j++)
            {
                acc[j] += w * CmykCorners[i][j];
            }
        }
        return (Math.Clamp(acc[0], 0.0f, 1.0f), Math.Clamp(acc[1], 0.0f, 1.0f), Math.Clamp(acc[2], 0.0f, 1.0f));
    }

    // ---- Form XObjects (text.rs:6821-7402) --------------------------------------

    /// <summary>
    /// Recursion limit. Text is rarely nested more than two or three forms deep; deeper
    /// nesting is complex vector graphics with no text in it.
    /// </summary>
    private const uint MaxXObjectDepth = 10;

    /// <summary>Per-page decode budget.</summary>
    private const uint MaxXObjectDecodes = 500;

    /// <summary>Tolerance on the /BBox clip, in points, so glyphs sitting on the clip edge
    /// survive float rounding. Conformant clipping is exact; this is far below any real
    /// margin.</summary>
    private const float BBoxClipTolerance = 1.0f;

    /// <summary>
    /// The XObject caches. Replaceable so a document-scoped owner can supply caches that stay
    /// warm across pages, which is what upstream's `PdfDocument`-held maps do.
    /// </summary>
    internal IOxXObjectCaches XObjectCaches
    {
        get => _xobjectCaches ??= new InMemoryXObjectCaches();
        set => _xobjectCaches = value;
    }

    private IOxXObjectCaches? _xobjectCaches;

    /// <summary>
    /// Loads the fonts of a page's or Form XObject's own /Resources into the extractor
    /// (`PdfDocument::load_fonts`, document.rs:19130). Upstream that method wraps one loop in
    /// several layers of cross-page font caching; the two that decide what a page's font
    /// aliases resolve to are ported below, the rest arrive through the same seam.
    /// </summary>
    internal Action<OxTextExtractor, PdfObject> LoadFontsForResources { get; set; } = DefaultLoadFonts;

    /// <summary>
    /// The two font-set caches a document carries: pdf_oxide's `font_set_cache`, keyed by the
    /// /Font dictionary's object reference (document.rs:19167), and its `font_fingerprint_cache`,
    /// keyed by the (alias &#8594; object reference) mapping the dictionary spells out
    /// (document.rs:19188). Pages that share a /Font dictionary — and the span and glyph passes
    /// over one page — then parse each embedded font program once between them.
    /// </summary>
    private sealed class FontSetCache
    {
        public readonly System.Collections.Concurrent.ConcurrentDictionary<
            (int Number, int Generation), List<(string Name, OxFontInfo Font)>> ByRef = new();

        public readonly System.Collections.Concurrent.ConcurrentDictionary<
            string, List<(string Name, OxFontInfo Font)>> ByFingerprint = new();
    }

    /// <remarks>
    /// Weak-keyed on the document so a cache dies with the document it describes; upstream
    /// hangs its equivalent off the document too, behind a mutex, which the concurrent map
    /// stands in for.
    /// </remarks>
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<
        PdfDocument, FontSetCache> FontSetCaches = new();

    private static void DefaultLoadFonts(OxTextExtractor extractor, PdfObject resources)
    {
        var doc = extractor.Document;
        PdfObject? fontEntry = Ox.Dict(doc, resources)?.Get("Font");
        if (fontEntry is null)
        {
            return;
        }

        var key = fontEntry as PdfRef;
        var cache = doc is not null ? FontSetCaches.GetOrCreateValue(doc) : null;

        if (cache is not null && key is not null
            && cache.ByRef.TryGetValue((key.Number, key.Generation), out var cachedByRef))
        {
            ApplyCachedFontSet(extractor, cachedByRef);
            return;
        }

        var fontDict = Ox.Dict(doc, fontEntry);
        if (fontDict is null)
        {
            return;
        }

        // Sorted by alias: which font donates a TrueType cmap to which depends on the order
        // they are loaded, so the dictionary's own ordering must not decide the text.
        var entries = new List<KeyValuePair<string, PdfObject>>(fontDict.Map);
        entries.Sort(static (a, b) => string.CompareOrdinal(a.Key, b.Key));

        string fingerprint = FontDictFingerprint(entries);
        if (cache is not null && cache.ByFingerprint.TryGetValue(fingerprint, out var cachedByPrint))
        {
            ApplyCachedFontSet(extractor, cachedByPrint);
            return;
        }

        foreach (var entry in entries)
        {
            if (OxFontInfo.FromDict(entry.Value, doc) is { } font)
            {
                extractor.AddFontShared(entry.Key, font);
            }
        }
        extractor.ShareTrueTypeCmaps();

        if (cache is not null)
        {
            // What both layers store is the extractor's whole font set, not just this call's
            // additions: a /Font dictionary loaded on top of a page's fonts caches the page's
            // fonts with it, and a later resources dictionary that fingerprints the same way
            // inherits all of them.
            var fontSet = extractor.GetFontSet();
            if (key is not null)
            {
                cache.ByRef[(key.Number, key.Generation)] = fontSet;
            }
            cache.ByFingerprint[fingerprint] = fontSet;
        }
    }

    private static void ApplyCachedFontSet(
        OxTextExtractor extractor, List<(string Name, OxFontInfo Font)> set)
    {
        foreach ((string name, OxFontInfo font) in set)
        {
            extractor.AddFontShared(name, font);
        }
        extractor.ShareTrueTypeCmaps();
    }

    /// <summary>
    /// The alias &#8594; object-reference mapping a /Font dictionary spells out, as a key. An
    /// entry that is not a reference contributes its alias alone, so two dictionaries whose
    /// inline font objects differ still fingerprint alike — the identity upstream hashes.
    /// </summary>
    private static string FontDictFingerprint(List<KeyValuePair<string, PdfObject>> sortedEntries)
    {
        var sb = new System.Text.StringBuilder();
        foreach (var entry in sortedEntries)
        {
            sb.Append(entry.Key);
            if (entry.Value is PdfRef r)
            {
                sb.Append(':').Append(r.Number).Append(':').Append(r.Generation);
            }
            sb.Append('\u0000');
        }
        return sb.ToString();
    }

    /// <summary>
    /// Resolve an XObject name to its object reference, populating the whole name-to-reference
    /// map for this resources context on the first miss — the resources/XObject dictionary
    /// chain is expensive to walk once per `Do`.
    /// </summary>
    internal (int Number, int Generation)? ResolveXObjectRef(string name)
    {
        if (CachedXObjectRefs.TryGetValue(name, out var cached))
        {
            return cached;
        }

        if (Resources is null || Document is null)
        {
            return null;
        }

        var resourcesDict = Ox.Dict(Document, Resources);
        if (resourcesDict is null)
        {
            return null;
        }

        var xobjectEntry = resourcesDict.Get("XObject");
        if (xobjectEntry is null)
        {
            return null;
        }

        var xobjectDict = Ox.Dict(Document, xobjectEntry);
        if (xobjectDict is null)
        {
            return null;
        }

        foreach (var entry in xobjectDict.Map)
        {
            CachedXObjectRefs[entry.Key] = Ox.RefOf(entry.Value);
        }

        return CachedXObjectRefs.TryGetValue(name, out var resolved) ? resolved : null;
    }

    /// <summary>
    /// Walk a Form XObject's content stream, extracting the text it paints. Image XObjects and
    /// anything that cannot be decoded are skipped.
    /// </summary>
    internal void ProcessXObject(string name)
    {
        if (XObjectDepth >= MaxXObjectDepth)
        {
            return;
        }
        if (XObjectDecodeCount >= MaxXObjectDecodes)
        {
            return;
        }

        if (ResolveXObjectRef(name) is not { } xobjectRef)
        {
            return;
        }

        // The dedup key is the reference AND the CTM at the `Do`. Keying on the reference
        // alone wrongly blocked the same form stamped at a second position (a header form used
        // at two Y positions, or a page whose content stream sets a different `cm` before each
        // `Do`). Recursion is still stopped, because a truly recursive call re-enters with the
        // same reference under the same CTM, and the depth limiter backs that up.
        //
        // Rounded to millipoints rather than truncated, so float noise in one logical CTM
        // still hashes to one key.
        var currentCtm = StateStack.Current.Ctm;
        var xobjKey = (
            xobjectRef.Number,
            xobjectRef.Generation,
            (long)MathF.Round(currentCtm.A * 1000.0f),
            (long)MathF.Round(currentCtm.B * 1000.0f),
            (long)MathF.Round(currentCtm.C * 1000.0f),
            (long)MathF.Round(currentCtm.D * 1000.0f),
            (long)MathF.Round(currentCtm.E * 1000.0f),
            (long)MathF.Round(currentCtm.F * 1000.0f));

        // Each (XObject, CTM) pair is walked at most once per page.
        if (ProcessedXObjects.Contains(xobjKey))
        {
            return;
        }
        ProcessedXObjects.Add(xobjKey);

        if (Document is not { } doc)
        {
            return;
        }

        if (XObjectCaches.IsTextFree(xobjectRef))
        {
            return;
        }

        // Image XObjects are megabytes of pixel data; discovering Subtype=Image after loading
        // one is the expensive way to find out.
        if (!IsFormXObject(doc, xobjectRef))
        {
            return;
        }

        // Cached spans are keyed by (reference, CTM) too, so a form reused across pages under
        // different translations cannot hand back the first page's coordinates.
        bool hasFilters = ExcludedLayers.Count > 0 || ExcludedInks.Count > 0;
        if (ExtractSpans && !hasFilters && XObjectCaches.TryGetSpans(xobjKey, out var cachedSpans))
        {
            if (cachedSpans is not null)
            {
                foreach (var span in cachedSpans)
                {
                    Spans.Add(span.Clone());
                }
            }
            return;
        }

        var xobject = doc.LoadObject(xobjectRef.Number, xobjectRef.Generation);
        var xobjectDict = xobject.AsDict();
        if (xobjectDict is null)
        {
            return;
        }

        string? subtype = xobjectDict.Get("Subtype").AsName();
        if (subtype != "Form")
        {
            // An Image XObject has no text, and an unknown subtype is not walked either.
            return;
        }

        // A form whose own /Resources has neither /Font nor /XObject can neither draw text nor
        // reach a nested form, so it is skipped without paying for the FlateDecode.
        var xobjResourcesEntry = xobjectDict.Get("Resources");
        if (xobjResourcesEntry is not null)
        {
            if (Ox.Dict(doc, xobjResourcesEntry) is { } resDict)
            {
                if (!resDict.Has("Font") && !resDict.Has("XObject"))
                {
                    XObjectCaches.MarkTextFree(xobjectRef);
                    return;
                }
            }
        }

        // With no /Resources at all the form inherits the page's fonts, and only its content
        // stream can say whether it uses them.
        XObjectDecodeCount++;
        byte[]? streamData = XObjectCaches.GetStream(xobjectRef);
        if (streamData is null)
        {
            streamData = Ox.StreamData(doc, xobject);
            if (streamData is null)
            {
                return;
            }
            XObjectCaches.PutStream(xobjectRef, streamData);
        }

        if (!MayContainText(streamData))
        {
            XObjectCaches.MarkTextFree(xobjectRef);
            return;
        }

        // §8.10.1: /Matrix defaults to the identity.
        var formMatrix = OxMatrix.Identity;
        if (xobjectDict.Get("Matrix").AsArray() is { } matrixArray)
        {
            float MatrixEntry(int i)
            {
                if (i < matrixArray.Items.Count && matrixArray.Items[i].AsNumber() is { } v)
                {
                    return (float)v;
                }
                return i is 0 or 3 ? 1.0f : 0.0f;
            }

            formMatrix = new OxMatrix(
                MatrixEntry(0), MatrixEntry(1), MatrixEntry(2),
                MatrixEntry(3), MatrixEntry(4), MatrixEntry(5));
        }

        // §8.10.1 clips a form's painting to its /BBox, so text it draws outside is invisible
        // in a conformant renderer and must not be extracted. Null disables the clip — /BBox
        // is required, but malformed dictionaries exist.
        float[]? formBbox = null;
        if (xobjectDict.Get("BBox").AsArray() is { } bboxArray && bboxArray.Items.Count >= 4)
        {
            float? B(int i) => bboxArray.Items[i].AsNumber() is { } v ? (float)v : null;
            if (B(0) is { } b0 && B(1) is { } b1 && B(2) is { } b2 && B(3) is { } b3
                && float.IsFinite(b0) && float.IsFinite(b1) && float.IsFinite(b2) && float.IsFinite(b3))
            {
                // Normalized so [x0,y0] is the min corner.
                formBbox = new[]
                {
                    MathF.Min(b0, b2), MathF.Min(b1, b3),
                    MathF.Max(b0, b2), MathF.Max(b1, b3),
                };
            }
        }

        // Fonts and resources are saved only when the form brings its own; a form that inherits
        // the page's should not pay for a copy of the font map.
        bool hasOwnResources = xobjectDict.Has("Resources");

        Dictionary<string, OxFontInfo>? savedFonts = null;
        PdfObject? savedResources = null;
        Dictionary<string, (int Number, int Generation)?>? savedXobjCache = null;

        if (hasOwnResources)
        {
            savedFonts = new Dictionary<string, OxFontInfo>(Fonts);
            savedResources = Resources;
            savedXobjCache = new Dictionary<string, (int Number, int Generation)?>(CachedXObjectRefs);
            CachedXObjectRefs.Clear();

            var xobjResources = xobjectDict.Get("Resources")!;

            // A /Resources reference that will not resolve leaves the reference itself in
            // place, as upstream's `load_object(..).unwrap_or(clone)` does.
            var xobjRes = Ox.Resolve(doc, xobjResources) ?? xobjResources;

            LoadFontsForResources(this, xobjRes);
            Resources = xobjRes;
        }

        int spansBefore = Spans.Count;

        // §8.10.1: the form is painted as if inside q … Q.
        StateStack.Save();
        StateStack.Current.Ctm = formMatrix.Multiply(StateStack.Current.Ctm);

        // The effective form-to-page transform every span inside the form is drawn under; the
        // /BBox clip maps the box through the same one so both live in one coordinate space.
        var formCtm = StateStack.Current.Ctm;

        // §14.7.4.3: every MCID inside this form belongs to the form's namespace, not the
        // page's, so two forms that both emit MCID 0 stay distinct.
        McidScopeStack.Add(OxMcidScope.Form(xobjectRef.Number, xobjectRef.Generation));

        XObjectDepth++;
        if (ExcludedInks.Count == 0)
        {
            OxContentParser.ParseAndExecuteTextOnly(streamData, innerOp =>
            {
                ExecuteOperator(innerOp);
                return true;
            });
        }
        else
        {
            foreach (var innerOp in OxContentParser.ParseContentStream(streamData))
            {
                ExecuteOperator(innerOp);
            }
        }
        XObjectDepth--;

        // Popped whatever the walk did, so the parent stream's scope is restored even after a
        // partial parse.
        McidScopeStack.RemoveAt(McidScopeStack.Count - 1);

        ApplyFormBBoxClip(formBbox, formCtm, spansBefore);

        // Caching needs `hasOwnResources`, so that the glyph mappings behind these spans are
        // self-contained; a form inheriting page fonts would produce spans that depend on the
        // caller's context.
        if (hasOwnResources && ExtractSpans && !hasFilters)
        {
            List<OxTextSpan>? newSpans = null;
            if (Spans.Count > spansBefore)
            {
                newSpans = new List<OxTextSpan>(Spans.Count - spansBefore);
                for (int i = spansBefore; i < Spans.Count; i++)
                {
                    newSpans.Add(Spans[i].Clone());
                }
            }
            XObjectCaches.PutSpans(xobjKey, newSpans);
        }

        // The implicit Q.
        StateStack.Restore();
        var restoredState = StateStack.Current;
        CachedCurrentFont = restoredState.FontName is { } restoredFont && Fonts.TryGetValue(restoredFont, out var font2)
            ? font2
            : null;

        if (savedFonts is not null)
        {
            Fonts.Clear();
            foreach (var entry in savedFonts)
            {
                Fonts[entry.Key] = entry.Value;
            }
        }
        if (savedResources is not null)
        {
            Resources = savedResources;
        }
        if (savedXobjCache is not null)
        {
            CachedXObjectRefs.Clear();
            foreach (var entry in savedXobjCache)
            {
                CachedXObjectRefs[entry.Key] = entry.Value;
            }
        }

        // An excluded ink the form set must not leak into the caller's scope, and the resources
        // the name resolves against have changed back.
        if (ExcludedInks.Count > 0)
        {
            InsideExcludedInk = IsExcludedInkColorSpace(StateStack.Current.FillColorSpace);
        }

        // The reference stays in ProcessedXObjects for good: re-walking a form yields the same
        // text, and keeping it prevents combinatorial fan-out on pages with deep XObject trees.
    }

    /// <summary>
    /// Drop the spans a form painted outside its /BBox (§8.10.1). Some producers — a pdfTeX
    /// \includegraphics of a figure PDF that kept a whole draft-galley page — paint a redundant
    /// copy of the body outside the figure's box, which surfaces as duplicate text over the
    /// real page. Runs before the span cache, so cached results are already clipped.
    /// </summary>
    private void ApplyFormBBoxClip(float[]? formBbox, in OxMatrix formCtm, int spansBefore)
    {
        if (formBbox is null || Spans.Count <= spansBefore)
        {
            return;
        }

        float bx0 = formBbox[0], by0 = formBbox[1], bx1 = formBbox[2], by1 = formBbox[3];
        if (bx1 <= bx0 || by1 <= by0)
        {
            return;
        }

        // The corners are mapped through the form CTM and bounded axis-aligned — a superset for
        // a rotated form, so it never over-clips.
        var corners = new[]
        {
            formCtm.TransformPoint(bx0, by0),
            formCtm.TransformPoint(bx1, by0),
            formCtm.TransformPoint(bx1, by1),
            formCtm.TransformPoint(bx0, by1),
        };
        float minX = float.PositiveInfinity, maxX = float.NegativeInfinity;
        float minY = float.PositiveInfinity, maxY = float.NegativeInfinity;
        foreach (var p in corners)
        {
            minX = MathF.Min(minX, p.X);
            maxX = MathF.Max(maxX, p.X);
            minY = MathF.Min(minY, p.Y);
            maxY = MathF.Max(maxY, p.Y);
        }
        if (!float.IsFinite(minX) || !float.IsFinite(maxX) || !float.IsFinite(minY) || !float.IsFinite(maxY))
        {
            return;
        }

        bool Inside(OxTextSpan s)
        {
            float cx = s.Bbox.X + (s.Bbox.Width * 0.5f);
            float cy = s.Bbox.Y + (s.Bbox.Height * 0.5f);
            return cx >= minX - BBoxClipTolerance
                && cx <= maxX + BBoxClipTolerance
                && cy >= minY - BBoxClipTolerance
                && cy <= maxY + BBoxClipTolerance;
        }

        // Where every span the form painted is already inside its box — the conformant majority,
        // and where the clip is a no-op anyway — nothing is rebuilt.
        bool anyOutside = false;
        for (int i = spansBefore; i < Spans.Count; i++)
        {
            if (!Inside(Spans[i]))
            {
                anyOutside = true;
                break;
            }
        }
        if (!anyOutside)
        {
            return;
        }

        // Out-of-BBox spans exist. A real figure form's stray text is a draft-galley underlay
        // safe to drop; a full-page content-frame wrapper whose declared BBox happens to exclude
        // body text is not — a conformant renderer clips both, but every text extractor keeps a
        // wrapper's body, which may be the only copy of it. Coverage discriminates: a figure
        // occupies a sub-region of the page, a wrapper covers most of it.
        float clipArea = (maxX - minX) * (maxY - minY);
        int pageIdx = McidScopeStack.Count > 0 && McidScopeStack[0].PageIndex is { } p0 ? p0 : 0;
        float? pageArea = null;
        if (Document is { } doc)
        {
            (double llx, double lly, double urx, double ury) = doc.GetPageMediaBox(pageIdx);
            float area = (float)Math.Abs((urx - llx) * (ury - lly));
            if (area > 0.0f)
            {
                pageArea = area;
            }
        }

        // At 60% or more of the page this is a content-frame wrapper, not a figure (measured
        // figures reach 27%, wrappers start at 82%).
        bool isPageWrapper = pageArea is { } pa && clipArea >= 0.6f * pa;
        if (isPageWrapper)
        {
            return;
        }

        var kept = new List<OxTextSpan>(Spans.Count - spansBefore);
        for (int i = spansBefore; i < Spans.Count; i++)
        {
            if (Inside(Spans[i]))
            {
                kept.Add(Spans[i]);
            }
        }
        Spans.RemoveRange(spansBefore, Spans.Count - spansBefore);
        Spans.AddRange(kept);
    }

    /// <summary>
    /// Whether an XObject is a Form. Upstream peeks the object header in the file to avoid
    /// loading an image (document.rs:2576); the port's document layer has no such peek, so the
    /// object is resolved and its /Subtype read, and anything unresolvable is treated as a form
    /// — the same conservative answer the peek gives when it cannot decide.
    /// </summary>
    private static bool IsFormXObject(PdfDocument doc, (int Number, int Generation) reference)
    {
        var obj = doc.LoadObject(reference.Number, reference.Generation);
        var dict = obj.AsDict();
        if (dict is null)
        {
            return true;
        }
        string? subtype = dict.Get("Subtype").AsName();
        return subtype is null || subtype == "Form";
    }

    /// <summary>
    /// Whether a decoded content stream can contribute text (document.rs:9578): it holds a BT,
    /// or a Do that may reach a form that does. §9.4.3 confines the showing operators to
    /// BT…ET, but a page's only text can live inside a form it invokes.
    /// </summary>
    internal static bool MayContainText(byte[] data)
    {
        static bool IsBoundary(byte b) =>
            b is (byte)' ' or (byte)'\t' or (byte)'\n' or (byte)'\r' or 0x0B or 0x0C
                or (byte)'(' or (byte)')' or (byte)'<' or (byte)'>' or (byte)'['
                or (byte)']' or (byte)'{' or (byte)'}' or (byte)'/' or (byte)'%';

        int len = data.Length;
        for (int i = 0; i + 1 < len; i++)
        {
            bool candidate = (data[i] == (byte)'B' && data[i + 1] == (byte)'T')
                || (data[i] == (byte)'D' && data[i + 1] == (byte)'o');
            if (!candidate)
            {
                continue;
            }

            bool beforeOk = i == 0 || IsBoundary(data[i - 1]);
            bool afterOk = i + 2 >= len || IsBoundary(data[i + 2]);
            if (beforeOk && afterOk)
            {
                return true;
            }
        }
        return false;
    }

    // ---- current_artifact_type (text.rs:7404) -----------------------------------

    /// <summary>The innermost artifact classification on the marked-content stack.</summary>
    internal OxArtifactType? CurrentArtifactType()
    {
        for (int i = MarkedContentStack.Count - 1; i >= 0; i--)
        {
            if (MarkedContentStack[i].ArtifactType is { } artifactType)
            {
                return artifactType;
            }
        }
        return null;
    }

    /// <summary>
    /// The default, extractor-local implementation of the XObject caches. One extractor walks
    /// one page, so the stream and span caches only ever pay off within it; the text-free set
    /// still saves a decode when one page invokes the same graphics-only form many times.
    /// </summary>
    private sealed class InMemoryXObjectCaches : IOxXObjectCaches
    {
        /// <summary>Stream cache budget, matching upstream's 50 MB ceiling.</summary>
        private const int MaxStreamCacheBytes = 50 * 1024 * 1024;

        private readonly HashSet<(int, int)> _textFree = new();
        private readonly Dictionary<(int, int), byte[]> _streams = new();
        private readonly Dictionary<
            (int Number, int Generation, long M0, long M1, long M2, long M3, long M4, long M5),
            List<OxTextSpan>?> _spans = new();
        private int _streamBytes;

        public bool IsTextFree((int Number, int Generation) reference) => _textFree.Contains(reference);

        public void MarkTextFree((int Number, int Generation) reference) => _textFree.Add(reference);

        public byte[]? GetStream((int Number, int Generation) reference) =>
            _streams.TryGetValue(reference, out byte[]? data) ? data : null;

        public void PutStream((int Number, int Generation) reference, byte[] data)
        {
            if (_streamBytes + data.Length <= MaxStreamCacheBytes)
            {
                _streamBytes += data.Length;
                _streams[reference] = data;
            }
        }

        public bool TryGetSpans(
            (int Number, int Generation, long M0, long M1, long M2, long M3, long M4, long M5) key,
            out List<OxTextSpan>? spans) => _spans.TryGetValue(key, out spans);

        public void PutSpans(
            (int Number, int Generation, long M0, long M1, long M2, long M3, long M4, long M5) key,
            List<OxTextSpan>? spans) => _spans[key] = spans;
    }
}
