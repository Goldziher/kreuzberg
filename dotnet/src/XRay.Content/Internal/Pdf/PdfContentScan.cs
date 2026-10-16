namespace XRay.Content.Internal.Pdf;

/// <summary>
/// A light content-stream pass that gathers only what scanned-page detection needs: where
/// raster images land on the page, which codec dominates, and how much of the text layer is
/// drawn invisibly. Deliberately separate from <see cref="PdfContentExtractor"/> — it needs
/// neither fonts nor glyph positioning, and the text path must not change shape to serve it.
/// </summary>
internal static class PdfContentScan
{
    internal sealed class ScanWalk
    {
        /// <summary>Axis-aligned image boxes in page space, one per image placement.</summary>
        public List<(double X0, double Y0, double X1, double Y1)> ImageBoxes = new();
        /// <summary>Dominant codec: CCITT wins over anything, otherwise first seen.</summary>
        public ImageCodecClass Codec = ImageCodecClass.None;
        /// <summary>Total bytes passed to text-showing operators.</summary>
        public int TextBytes;
        /// <summary>Bytes shown while the text render mode was 3 (invisible).</summary>
        public int InvisibleBytes;
    }

    private const int MaxDepth = 12;

    public static ScanWalk Walk(PdfDocument doc, int pageIndex)
    {
        var walk = new ScanWalk();
        byte[] content;
        PdfDict? resources;
        try
        {
            content = doc.GetPageContent(pageIndex);
            resources = doc.Resolve(doc.Pages[pageIndex].Get("Resources")).AsDict();
        }
        catch (Exception) when (Cancel.NotCancelling) { return walk; }

        Run(doc, walk, content, resources, Matrix.Identity, 0);
        return walk;
    }

    private static void Run(PdfDocument doc, ScanWalk walk, byte[] content, PdfDict? resources,
        Matrix baseCtm, int depth)
    {
        if (depth > MaxDepth) return;
        var xobjects = doc.Resolve(resources?.Get("XObject")).AsDict();

        var lex = new PdfLexer(content, 0, null);
        var operands = new List<PdfObject>();
        var ctm = baseCtm;
        var ctmStack = new Stack<Matrix>();
        byte renderMode = 0;
        var renderStack = new Stack<byte>();

        int tokenCount = 0;
        while (lex.Pos < lex.Length)
        {
            Cancel.Every(tokenCount++, 0xFF);
            lex.SkipWhitespace();
            if (lex.Pos >= lex.Length) break;
            byte b = lex.Buffer[lex.Pos];
            if (b == (byte)'<' || b == (byte)'(' || b == (byte)'[' || b == (byte)'/' ||
                b == (byte)'+' || b == (byte)'-' || b == (byte)'.' || (b >= (byte)'0' && b <= (byte)'9'))
            {
                int before = lex.Pos;
                operands.Add(lex.ParseObject());
                if (lex.Pos == before) lex.Pos++;
                if (operands.Count > 64) operands.RemoveRange(0, operands.Count - 32);
                continue;
            }

            string? op = lex.ReadToken();
            if (op is null) break;

            try
            {
                switch (op)
                {
                    case "q":
                        ctmStack.Push(ctm);
                        renderStack.Push(renderMode);
                        break;
                    case "Q":
                        if (ctmStack.Count > 0) ctm = ctmStack.Pop();
                        if (renderStack.Count > 0) renderMode = renderStack.Pop();
                        break;
                    case "cm":
                        if (operands.Count >= 6)
                        {
                            var m = MatrixFrom(operands, operands.Count - 6);
                            ctm = m.Multiply(ctm);
                        }
                        break;
                    case "Tr":
                        if (operands.Count >= 1 && operands[^1].AsNumber() is { } rm)
                            renderMode = (byte)Math.Clamp((int)rm, 0, 7);
                        break;
                    case "Tj":
                    case "'":
                    case "\"":
                        if (operands.Count >= 1 && operands[^1] is PdfString s)
                            Count(walk, s.Bytes.Length, renderMode);
                        break;
                    case "TJ":
                        if (operands.Count >= 1 && operands[^1] is PdfArray arr)
                        {
                            int n = 0;
                            foreach (var it in arr.Items) if (it is PdfString ps) n += ps.Bytes.Length;
                            Count(walk, n, renderMode);
                        }
                        break;
                    case "Do":
                        if (operands.Count >= 1 && operands[^1] is PdfName xn)
                            DoXObject(doc, walk, xn.Value, xobjects, ctm, depth);
                        break;
                    case "BI":
                        // Inline image: its dictionary carries the same filter names, and it is
                        // painted in the current unit square exactly like an image XObject.
                        InlineImage(doc, walk, lex, ctm);
                        break;
                }
            }
            catch (Exception) when (Cancel.NotCancelling) { /* detection is advisory; keep walking */ }

            operands.Clear();
        }
    }

    private static void Count(ScanWalk walk, int bytes, byte renderMode)
    {
        walk.TextBytes += bytes;
        if (renderMode == 3) walk.InvisibleBytes += bytes;
    }

    private static Matrix MatrixFrom(List<PdfObject> ops, int at) => new(
        ops[at].AsNumber() ?? 1, ops[at + 1].AsNumber() ?? 0,
        ops[at + 2].AsNumber() ?? 0, ops[at + 3].AsNumber() ?? 1,
        ops[at + 4].AsNumber() ?? 0, ops[at + 5].AsNumber() ?? 0);

    private static void DoXObject(PdfDocument doc, ScanWalk walk, string name, PdfDict? xobjects,
        Matrix ctm, int depth)
    {
        if (doc.Resolve(xobjects?.Get(name)) is not PdfStream st) return;
        string? subtype = doc.Resolve(st.Dict.Get("Subtype")).AsName();

        if (subtype == "Image")
        {
            if (CarriesColorSpace(st.Dict, inline: false))
                AddImage(walk, ctm, CodecOf(doc, st.Dict));
            return;
        }
        if (subtype != "Form") return;

        // Recurse into form XObjects: a scanned page is often a single form wrapping the raster.
        var m = ctm;
        if (doc.Resolve(st.Dict.Get("Matrix")).AsArray() is PdfArray fm && fm.Items.Count >= 6)
            m = MatrixFrom(fm.Items, 0).Multiply(ctm);

        var formRes = doc.Resolve(st.Dict.Get("Resources")).AsDict();
        try { Run(doc, walk, doc.DecodeStream(st), formRes, m, depth + 1); } catch (Exception) when (Cancel.NotCancelling) { }
    }

    private static void InlineImage(PdfDocument doc, ScanWalk walk, PdfLexer lex, Matrix ctm)
    {
        // Parse the inline dictionary up to `ID`, then skip the data to `EI`.
        var dict = new PdfDict();
        while (lex.Pos < lex.Length)
        {
            lex.SkipWhitespace();
            if (lex.Pos >= lex.Length) return;
            if (lex.Buffer[lex.Pos] == (byte)'/')
            {
                var key = lex.ParseObject();
                var val = lex.ParseObject();
                if (key is PdfName kn) dict.Map[kn.Value] = val;
                continue;
            }
            string? tok = lex.ReadToken();
            if (tok is null) return;
            if (tok == "ID") break;
        }

        if (CarriesColorSpace(dict, inline: true))
            AddImage(walk, ctm, CodecOf(doc, dict, inline: true));
        SkipToEndOfInlineImage(lex);
    }

    private static void SkipToEndOfInlineImage(PdfLexer lex)
    {
        while (lex.Pos + 1 < lex.Length)
        {
            if (lex.Buffer[lex.Pos] == (byte)'E' && lex.Buffer[lex.Pos + 1] == (byte)'I'
                && (lex.Pos + 2 >= lex.Length || lex.Buffer[lex.Pos + 2] <= 32))
            {
                lex.Pos += 2;
                return;
            }
            lex.Pos++;
        }
    }

    /// <summary>An image is painted into the unit square, so its page-space box is the
    /// axis-aligned bounds of that square under the CTM.</summary>
    private static void AddImage(ScanWalk walk, Matrix ctm, ImageCodecClass codec)
    {
        var (x0, y0) = ctm.Transform(0, 0);
        var (x1, y1) = ctm.Transform(1, 0);
        var (x2, y2) = ctm.Transform(0, 1);
        var (x3, y3) = ctm.Transform(1, 1);

        walk.ImageBoxes.Add((
            Math.Min(Math.Min(x0, x1), Math.Min(x2, x3)),
            Math.Min(Math.Min(y0, y1), Math.Min(y2, y3)),
            Math.Max(Math.Max(x0, x1), Math.Max(x2, x3)),
            Math.Max(Math.Max(y0, y1), Math.Max(y2, y3))));

        // CCITT wins over anything already seen; otherwise the first codec sticks.
        walk.Codec = (walk.Codec, codec) switch
        {
            (ImageCodecClass.None, var x) => x,
            (_, ImageCodecClass.Ccitt) => ImageCodecClass.Ccitt,
            (var cur, _) => cur,
        };
    }

    /// <summary>
    /// Whether the image dictionary names a colour space, which is what decides whether the
    /// reference extracts the image at all.
    /// </summary>
    /// <remarks>
    /// A stencil mask (<c>/ImageMask true</c>) carries no <c>/ColorSpace</c>, and pdf_oxide's
    /// image extractor rejects any image dictionary without one, so masks reach neither the
    /// coverage sum nor the codec vote — a page of CCITT masks over a JPEG photo grades as a
    /// photo, not as a fax.
    /// </remarks>
    private static bool CarriesColorSpace(PdfDict dict, bool inline) =>
        dict.Has("ColorSpace") || (inline && dict.Has("CS"));

    private static ImageCodecClass CodecOf(PdfDocument doc, PdfDict dict, bool inline = false)
    {
        var filter = doc.Resolve(dict.Get(inline ? "F" : "Filter")) ?? doc.Resolve(dict.Get("Filter"));
        var names = new List<string>();
        if (filter is PdfName fn) names.Add(fn.Value);
        else if (filter is PdfArray fa)
            foreach (var it in fa.Items)
                if (doc.Resolve(it) is PdfName n) names.Add(n.Value);

        foreach (var n in names)
        {
            switch (n)
            {
                case "CCITTFaxDecode" or "CCF": return ImageCodecClass.Ccitt;
                case "DCTDecode" or "DCT" or "JPXDecode": return ImageCodecClass.Dct;
                // JBIG2 is deliberately absent. pdf_oxide's page classifier asks each image only
                // whether it carries CCITT parameters and whether its data decodes as JPEG, so a
                // JBIG2 image comes back as `Other` and never earns the bilevel-codec bonus.
                // Naming it here would score four scanned fixtures 0.10 above the reference.
            }
        }
        return ImageCodecClass.Other;
    }
}
