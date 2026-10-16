// Content-stream interpreter: tracks the graphics/text state (ISO 32000-1 §8–9),
// executes text-showing operators (Tj/TJ/'/"), and emits positioned text spans.
// Approximates pdf_oxide's TextExtractor span output.
using System.Text;

namespace XRay.Content.Internal.Pdf;

public struct Matrix
{
    public double A, B, C, D, E, F;
    public Matrix(double a, double b, double c, double d, double e, double f) { A = a; B = b; C = c; D = d; E = e; F = f; }
    public static readonly Matrix Identity = new(1, 0, 0, 1, 0, 0);

    public (double x, double y) Transform(double x, double y) => (A * x + C * y + E, B * x + D * y + F);

    // Result applies `this` first, then `m`  (result = this · m).
    public Matrix Multiply(in Matrix m) => new(
        m.A * A + m.C * B,
        m.B * A + m.D * B,
        m.A * C + m.C * D,
        m.B * C + m.D * D,
        m.A * E + m.C * F + m.E,
        m.B * E + m.D * F + m.F);

    // Single-precision twins of the two above, for the path pipeline. pdf_oxide's graphics
    // state is `f32`, so it rounds after every product and every sum; folding the same
    // expression at double width and rounding once at the end lands a fraction of an ulp
    // away, and that drift survives into the table bounding boxes the corpus compares.
    public (double x, double y) TransformSingle(double x, double y)
    {
        float a = (float)A, b = (float)B, c = (float)C, d = (float)D, e = (float)E, f = (float)F;
        float fx = (float)x, fy = (float)y;
        return (a * fx + c * fy + e, b * fx + d * fy + f);
    }

    public Matrix MultiplySingle(in Matrix m)
    {
        float a = (float)A, b = (float)B, c = (float)C, d = (float)D, e = (float)E, f = (float)F;
        float ma = (float)m.A, mb = (float)m.B, mc = (float)m.C;
        float md = (float)m.D, me = (float)m.E, mf = (float)m.F;
        return new Matrix(
            a * ma + b * mc,
            a * mb + b * md,
            c * ma + d * mc,
            c * mb + d * md,
            e * ma + f * mc + me,
            e * mb + f * md + mf);
    }
}

public sealed class TextSpan
{
    public string Text = "";
    public double X, Y, Width, Height;
    public double FontSize;
    public bool IsBold;
    public bool IsItalic;
    public bool IsMonospace;

    /// <summary>The font's <c>/BaseFont</c> name. A change of it across a span boundary is a
    /// font-run transition, which producers frequently leave without an explicit space glyph.</summary>
    public string FontName = "";

    /// <summary>Text-matrix rotation in degrees; 0 for upright text.</summary>
    public double RotationDegrees;

    /// <summary>
    /// The Text Rise operator's shift (ISO 32000-1 §9.3.7 `Ts`) as a fraction of the font size.
    /// A producer that raises or lowers a run this way has said outright that it is a super- or
    /// subscript, whatever its size or characters.
    /// </summary>
    public double TextRiseRatio;

    /// <summary>Emission order within the page, used to break ties in a stable way.</summary>
    public int Sequence;

    // Geometry accessors mirroring pdf_oxide Rect (PDF coords: Y grows up).
    public double Left => X;
    public double Right => X + Width;
    public double Top => Y;      // pdf_oxide Rect::top() == y (lower edge); larger Y = higher on page
    public double Bottom => Y + Height;
}

public sealed class PdfContentExtractor
{
    private readonly PdfDocument _doc;
    private readonly List<TextSpan> _spans = new();

    private sealed class GState
    {
        public Matrix Ctm = Matrix.Identity;
        public double CharSpace, WordSpace;
        public double Hscale = 100.0;
        public double Leading;
        public double FontSize;
        public double Rise;
        public PdfFont? Font;
        public double LineWidth = 1.0;
        public int LineCap;
        // pdf_oxide's path extractor carries its own f32 graphics state, and rounding
        // the product after every concatenation is what makes `.23999999 3.125 cm cm`
        // land on exactly 0.75. Keeping it separate leaves the text CTM alone.
        public Matrix PathCtm = Matrix.Identity;
        public GState Clone() => (GState)MemberwiseClone();
    }

    private GState _gs = new();
    private readonly Stack<GState> _stack = new();
    private Matrix _tm = Matrix.Identity;
    private Matrix _tlm = Matrix.Identity;
    private readonly long _deadline;

    // Show-op span buffer, mirroring pdf_oxide's TjBuffer (extractors/text.rs).
    // A whole TJ array — and consecutive Tj operators — accumulate into ONE
    // span; only large negative TJ offsets (< SpaceInsertionThreshold) split
    // it. Emitting one span per show-string scrambles kerned runs once spans
    // are sorted by position (senate-expenditures fixture).
    private sealed class TjBuffer
    {
        public StringBuilder Text = new();
        public double StartX, StartY;          // user-space origin at buffer start
        public double RiseRatio;               // Ts shift ÷ font size at buffer start
        public double AccumWidth;              // text-space width (incl. Tz)
        public double UserHScale;              // sqrt(a²+c²) of combined matrix
        public double EffFontSize;             // font_size * sqrt(b²+d²)
        public bool IsBold, IsItalic, IsMonospace;
        public string FontName = "";
        public bool HasRtl;
        public double RotationDegrees;         // atan2(b, a) of the combined matrix
    }

    private TjBuffer? _buf;

    // pdf_oxide TextExtractionConfig::default().space_insertion_threshold.
    private const double SpaceInsertionThreshold = -120.0;

    // Painted paths, in device space. Ruling-line table detection reads these; text
    // extraction ignores them, so collection costs one list append per painting operator.
    private readonly List<PdfPath> _paths = new();
    private List<PathOp> _currentOps = new();
    private (double X, double Y)? _subpathStart;

    /// <summary>Paths painted by the content processed so far (pdf_oxide `extract_paths`).</summary>
    public List<PdfPath> Paths => _paths;

    public PdfContentExtractor(PdfDocument doc, long deadlineTicks) { _doc = doc; _deadline = deadlineTicks; }

    public List<TextSpan> Extract(byte[] content, PdfDict? resources)
    {
        Run(content, resources, 0);
        FlushBuffer();
        return _spans;
    }

    private void Run(byte[] content, PdfDict? resources, int depth)
    {
        if (depth > 12) return;
        var fonts = LoadFonts(resources);
        var xobjects = _doc.Resolve(resources?.Get("XObject")).AsDict();

        var lex = new PdfLexer(content, 0, null);
        var operands = new List<PdfObject>();
        int operatorCount = 0;
        while (lex.Pos < lex.Length)
        {
            Cancel.Every(operatorCount++, 0xFF);
            if (DateTime.UtcNow.Ticks > _deadline) return;
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
            if (op == null) break;
            if (op == "BI") { SkipInlineImage(lex); operands.Clear(); continue; }
            try { Execute(op, operands, fonts, xobjects, depth); }
            catch (Exception) when (Cancel.NotCancelling) { /* keep going */ }
            operands.Clear();
        }
    }

    // Per-document font cache: resolved font dicts are reference-cached by
    // PdfDocument, so the same PdfDict instance recurs across pages. Parsing
    // a font (ToUnicode CMap, widths, encoding) is expensive; without this
    // cache large books re-parse every font on every page (bayesian/intel
    // fixtures tripped the 25s guard).
    private static readonly System.Runtime.CompilerServices.ConditionalWeakTable<PdfDocument, Dictionary<PdfDict, PdfFont>> FontCaches = new();

    private Dictionary<string, PdfFont> LoadFonts(PdfDict? resources)
    {
        var map = new Dictionary<string, PdfFont>(StringComparer.Ordinal);
        var fontsDict = _doc.Resolve(resources?.Get("Font")).AsDict();
        if (fontsDict == null) return map;
        var cache = FontCaches.GetOrCreateValue(_doc);
        foreach (var kv in fontsDict.Map)
        {
            var fd = _doc.Resolve(kv.Value).AsDict();
            if (fd != null)
            {
                if (cache.TryGetValue(fd, out var cached)) { map[kv.Key] = cached; continue; }
                try
                {
                    var font = PdfFont.Load(fd, _doc);
                    cache[fd] = font;
                    map[kv.Key] = font;
                }
                catch (Exception) when (Cancel.NotCancelling) { }
            }
        }
        return map;
    }

    private static double Num(List<PdfObject> ops, int i) => i < ops.Count ? (ops[i].AsNumber() ?? 0) : 0;

    private static float NumF(List<PdfObject> ops, int i) => (float)Num(ops, i);

    private void Execute(string op, List<PdfObject> ops, Dictionary<string, PdfFont> fonts, PdfDict? xobjects, int depth)
    {
        switch (op)
        {
            case "q": _stack.Push(_gs.Clone()); break;
            case "Q": FlushBuffer(); if (_stack.Count > 0) _gs = _stack.Pop(); break;
            case "cm":
                if (ops.Count >= 6)
                {
                    FlushBuffer();
                    var cmMatrix = new Matrix(Num(ops, 0), Num(ops, 1), Num(ops, 2), Num(ops, 3), Num(ops, 4), Num(ops, 5));
                    _gs.Ctm = cmMatrix.Multiply(_gs.Ctm);
                    _gs.PathCtm =
                        new Matrix(NumF(ops, 0), NumF(ops, 1), NumF(ops, 2), NumF(ops, 3), NumF(ops, 4), NumF(ops, 5))
                            .MultiplySingle(_gs.PathCtm);
                }
                break;
            case "BT": FlushBuffer(); _tm = Matrix.Identity; _tlm = Matrix.Identity; break;
            case "ET": FlushBuffer(); break;
            case "Td":
                if (ops.Count >= 2) { FlushBuffer(); _tlm = new Matrix(1, 0, 0, 1, Num(ops, 0), Num(ops, 1)).Multiply(_tlm); _tm = _tlm; }
                break;
            case "TD":
                if (ops.Count >= 2)
                {
                    FlushBuffer();
                    _gs.Leading = -Num(ops, 1);
                    _tlm = new Matrix(1, 0, 0, 1, Num(ops, 0), Num(ops, 1)).Multiply(_tlm); _tm = _tlm;
                }
                break;
            case "Tm":
                if (ops.Count >= 6) { FlushBuffer(); _tlm = new Matrix(Num(ops, 0), Num(ops, 1), Num(ops, 2), Num(ops, 3), Num(ops, 4), Num(ops, 5)); _tm = _tlm; }
                break;
            case "T*":
                FlushBuffer();
                _tlm = new Matrix(1, 0, 0, 1, 0, -_gs.Leading).Multiply(_tlm); _tm = _tlm;
                break;
            case "Tc": _gs.CharSpace = Num(ops, 0); break;
            case "Tw": _gs.WordSpace = Num(ops, 0); break;
            case "Tz": _gs.Hscale = Num(ops, 0); break;
            case "TL": _gs.Leading = Num(ops, 0); break;
            case "Ts": _gs.Rise = Num(ops, 0); break;
            case "Tf":
                FlushBuffer();
                if (ops.Count >= 2 && ops[0] is PdfName fn)
                {
                    fonts.TryGetValue(fn.Value, out var font);
                    _gs.Font = font;
                    _gs.FontSize = Num(ops, 1);
                }
                break;
            case "Tj":
                if (ops.Count >= 1 && ops[^1] is PdfString s) ShowText(s.Bytes);
                break;
            case "'":
                FlushBuffer();
                _tlm = new Matrix(1, 0, 0, 1, 0, -_gs.Leading).Multiply(_tlm); _tm = _tlm;
                if (ops.Count >= 1 && ops[^1] is PdfString s2) ShowText(s2.Bytes);
                break;
            case "\"":
                if (ops.Count >= 3)
                {
                    FlushBuffer();
                    _gs.WordSpace = Num(ops, 0);
                    _gs.CharSpace = Num(ops, 1);
                    _tlm = new Matrix(1, 0, 0, 1, 0, -_gs.Leading).Multiply(_tlm); _tm = _tlm;
                    if (ops[^1] is PdfString s3) ShowText(s3.Bytes);
                }
                break;
            case "TJ":
                FlushBuffer();
                if (ops.Count >= 1 && ops[^1] is PdfArray arr) ShowTextArray(arr);
                break;
            case "Do":
                FlushBuffer();
                if (ops.Count >= 1 && ops[0] is PdfName xn) DoXObject(xn.Value, xobjects, depth);
                break;

            // ── Path construction and painting (pdf_oxide `extractors::paths`) ────────
            case "w": _gs.LineWidth = (float)Num(ops, 0); break;
            case "J": _gs.LineCap = (int)Num(ops, 0); break;
            // Path operands are read as f32 upstream, so round them here too: a
            // rectangle's far corner is `x + width` in single precision, and a double
            // sum rounded afterwards is not the same number.
            case "m": PathMoveTo(NumF(ops, 0), NumF(ops, 1)); break;
            case "l": PathLineTo(NumF(ops, 0), NumF(ops, 1)); break;
            case "c": PathCurveTo(NumF(ops, 0), NumF(ops, 1), NumF(ops, 2), NumF(ops, 3), NumF(ops, 4), NumF(ops, 5)); break;
            case "v": PathCurveToV(NumF(ops, 0), NumF(ops, 1), NumF(ops, 2), NumF(ops, 3)); break;
            case "y": PathCurveToY(NumF(ops, 0), NumF(ops, 1), NumF(ops, 2), NumF(ops, 3)); break;
            case "re": PathRectangle(NumF(ops, 0), NumF(ops, 1), NumF(ops, 2), NumF(ops, 3)); break;
            case "h": PathClose(); break;
            case "S": FinalizePath(stroke: true, fill: false); break;
            case "s": PathClose(); FinalizePath(stroke: true, fill: false); break;
            case "f": case "F": case "f*": FinalizePath(stroke: false, fill: true); break;
            case "b": PathClose(); FinalizePath(stroke: true, fill: true); break;
            // `B`, `B*` and `b*` paint nothing here, and that is deliberate. The path
            // extractor upstream (`document.rs:17590`) answers only `S`, `f`/`F`, `f*`,
            // `b` and `n`; every other painting operator falls to its catch-all arm, which
            // neither emits the path nor clears the operations it built up. A run of
            // fill-and-stroke subpaths therefore accumulates until the next clip
            // (`W n` / `W* n`) discards it or a handled operator paints all of it as one
            // path. Painting them individually is what a renderer would do, but it is not
            // what the detectors downstream are calibrated against: the ruling-line tiers
            // read this list, and on a page whose producer draws every cell border as
            // `m l l l h B*` it turns 27 primitives into 231, flooding the edge grid with
            // rules upstream never sees.
            case "B": case "B*": case "b*": break;
            case "n": EndPath(); break;
        }
    }

    // ── Path construction ────────────────────────────────────────────────────────
    // Points are CTM-transformed as they are recorded, matching pdf_oxide's
    // `PathExtractor`: the path list is device-space geometry with no matrix attached.

    // pdf_oxide's path pipeline is f32 end to end, and the edge coordinates a table's
    // bounding box is built from come straight out of it.
    private (double x, double y) TransformPath(double x, double y) =>
        _gs.PathCtm.TransformSingle(x, y);

    private static Matrix RoundToSingle(in Matrix m) =>
        new((float)m.A, (float)m.B, (float)m.C, (float)m.D, (float)m.E, (float)m.F);

    /// <summary>
    /// `line_width * Matrix::stroke_scale()` (pdf_oxide `content/graphics_state.rs:93`).
    /// The `w` operand is user-space while the recorded path is already CTM-transformed
    /// (§8.4.3.2), so it is scaled by sqrt(|det|) — the uniform-scale approximation
    /// renderers use — to keep the width and the bbox in one coordinate space. Computed in
    /// single precision because the rendered bbox this feeds is compared against a
    /// reference produced by an f32 pipeline, and a double-width residue here shows up as
    /// a one-ulp drift in every table bounding box on the page.
    /// </summary>
    private static double StrokeWidthOf(GState gs)
    {
        float a = (float)gs.PathCtm.A, b = (float)gs.PathCtm.B;
        float c = (float)gs.PathCtm.C, d = (float)gs.PathCtm.D;
        return (float)gs.LineWidth * MathF.Sqrt(MathF.Abs(a * d - b * c));
    }

    private void PathMoveTo(double x, double y)
    {
        var (tx, ty) = TransformPath(x, y);
        _currentOps.Add(PathOp.MoveTo(tx, ty));
        _pathCurrent = (tx, ty);
        _subpathStart = (tx, ty);
    }

    private void PathLineTo(double x, double y)
    {
        var (tx, ty) = TransformPath(x, y);
        _currentOps.Add(PathOp.LineTo(tx, ty));
        _pathCurrent = (tx, ty);
    }

    private void PathCurveTo(double x1, double y1, double x2, double y2, double x3, double y3)
    {
        var p1 = TransformPath(x1, y1);
        var p2 = TransformPath(x2, y2);
        var p3 = TransformPath(x3, y3);
        _currentOps.Add(PathOp.CurveTo(p1.x, p1.y, p2.x, p2.y, p3.x, p3.y));
        _pathCurrent = (p3.x, p3.y);
    }

    private void PathCurveToV(double x2, double y2, double x3, double y3)
    {
        var p1 = _pathCurrent ?? (0.0, 0.0);
        var p2 = TransformPath(x2, y2);
        var p3 = TransformPath(x3, y3);
        _currentOps.Add(PathOp.CurveTo(p1.Item1, p1.Item2, p2.x, p2.y, p3.x, p3.y));
        _pathCurrent = (p3.x, p3.y);
    }

    private void PathCurveToY(double x1, double y1, double x3, double y3)
    {
        var p1 = TransformPath(x1, y1);
        var p3 = TransformPath(x3, y3);
        _currentOps.Add(PathOp.CurveTo(p1.x, p1.y, p3.x, p3.y, p3.x, p3.y));
        _pathCurrent = (p3.x, p3.y);
    }

    private void PathRectangle(double x, double y, double w, double h)
    {
        var p1 = TransformPath(x, y);
        var p2 = TransformPath((float)x + (float)w, (float)y + (float)h);
        _currentOps.Add(PathOp.Rect(p1.x, p1.y, (float)(p2.x - p1.x), (float)(p2.y - p1.y)));
        _pathCurrent = (p1.x, p1.y);
        _subpathStart = (p1.x, p1.y);
    }

    private void PathClose()
    {
        _currentOps.Add(PathOp.Close);
        if (_subpathStart is { } s) _pathCurrent = s;
    }

    private void EndPath()
    {
        _currentOps.Clear();
        _pathCurrent = null;
        _subpathStart = null;
    }

    private void FinalizePath(bool stroke, bool fill)
    {
        if (_currentOps.Count == 0) return;
        // Cap the collected set: a chart or map can paint tens of thousands of paths,
        // none of which is a table rule, and the detector is quadratic in edge count.
        if (_paths.Count < MaxPaths)
        {
            var candidate = new PdfPath
            {
                Operations = _currentOps,
                Bbox = PdfPath.ComputeBbox(_currentOps),
                Stroked = stroke,
                Filled = fill,
                LineCap = stroke ? _gs.LineCap : 0,
                StrokeWidth = stroke ? StrokeWidthOf(_gs) : 0.0,
            };
            // Only rules and boxes are ever consulted; glyph outlines and chart fills
            // would otherwise dominate the list on graphics-heavy pages. Rule candidates
            // are kept alongside primitives because the borderless-table gate counts
            // drawn rules over a slightly wider thickness band.
            if (candidate.IsTablePrimitive() || candidate.IsRuleCandidate()) _paths.Add(candidate);
        }
        _currentOps = new List<PathOp>();
        _pathCurrent = null;
        _subpathStart = null;
    }

    private const int MaxPaths = 20000;
    private (double, double)? _pathCurrent;

    // TJ array (pdf_oxide process_tj_array_tiebreaker): accumulate all strings
    // into one span; only offsets below SpaceInsertionThreshold split the span
    // (flush + synthetic space span, mirroring insert_space_as_span).
    private void ShowTextArray(PdfArray arr)
    {
        for (int i = 0; i < arr.Items.Count; i++)
        {
            var it = arr.Items[i];
            if (it is PdfString s) ShowText(s.Bytes);
            else if (it is PdfNumber n)
            {
                if (n.Value < SpaceInsertionThreshold)
                {
                    bool bufEndsWithSpace = _buf != null && _buf.Text.Length > 0
                        && char.IsWhiteSpace(_buf.Text[^1]);
                    FlushBuffer();
                    bool nextStartsWithSpace = i + 1 < arr.Items.Count
                        && arr.Items[i + 1] is PdfString ns && ns.Bytes.Length > 0
                        && (ns.Bytes[0] == 0x20 || ns.Bytes[0] == 0x09 || ns.Bytes[0] == 0x0A || ns.Bytes[0] == 0x0D);
                    if (!bufEndsWithSpace && !nextStartsWithSpace) InsertSpaceSpan();
                }
                // TJ adjustment: subtract n/1000 * fontSize * Tz from text position.
                double adj = -n.Value / 1000.0 * _gs.FontSize * (_gs.Hscale / 100.0);
                _tm = new Matrix(1, 0, 0, 1, adj, 0).Multiply(_tm);
            }
        }
    }

    /// <summary>Text-matrix rotation in degrees. Zero for the overwhelming majority of spans;
    /// a sideways table or caption carries 90/180/270.</summary>
    private static double RotationOf(in Matrix m)
    {
        double deg = Math.Atan2(m.B, m.A) * (180.0 / Math.PI);
        // Snap float noise around an upright matrix to exactly zero so the "is this page
        // rotated at all" gate is not tripped by rounding.
        return Math.Abs(deg) < 1e-6 ? 0.0 : deg;
    }

    // Synthetic space span for a large TJ offset (pdf_oxide insert_space_as_span).
    private void InsertSpaceSpan()
    {
        var combined = _tm.Multiply(_gs.Ctm);
        double effSize = _gs.FontSize * Math.Sqrt(combined.D * combined.D + combined.B * combined.B);
        double spaceAdvance = (250.0 * _gs.FontSize / 1000.0 + _gs.WordSpace) * (_gs.Hscale / 100.0);
        _spans.Add(new TextSpan
        {
            Text = " ",
            X = combined.E,
            Y = combined.F,
            Width = spaceAdvance,
            Height = effSize,
            FontSize = effSize,
            IsBold = false,
            IsItalic = _gs.Font?.IsItalic ?? false,
            IsMonospace = false,
            FontName = _gs.Font?.BaseFont ?? "",
            RotationDegrees = RotationOf(combined),
            Sequence = _spans.Count,
        });
    }

    private void StartBuffer()
    {
        var combined = _tm.Multiply(_gs.Ctm);
        _buf = new TjBuffer
        {
            StartX = combined.E,
            StartY = combined.F,
            EffFontSize = _gs.FontSize * Math.Sqrt(combined.D * combined.D + combined.B * combined.B),
            UserHScale = Math.Sqrt(combined.A * combined.A + combined.C * combined.C),
            RotationDegrees = RotationOf(combined),
            IsBold = _gs.Font?.IsBold ?? false,
            IsItalic = _gs.Font?.IsItalic ?? false,
            IsMonospace = _gs.Font?.IsMonospace ?? false,
            FontName = _gs.Font?.BaseFont ?? "",
            RiseRatio = _gs.FontSize != 0 ? _gs.Rise / _gs.FontSize : 0,
        };
    }

    private void FlushBuffer()
    {
        var b = _buf;
        _buf = null;
        if (b == null || b.Text.Length == 0) return;
        string text = b.Text.ToString();
        // RTL correction (pdf_oxide flush_tj_buffer): text within a buffer is
        // in visual LTR draw order; when it contains RTL characters and the
        // buffer advanced left-to-right, reverse to logical order.
        if (text.Length > 1 && b.HasRtl && b.AccumWidth > 0.0)
            text = PdfBidi.ReverseRtlKeepNumbers(text);
        _spans.Add(new TextSpan
        {
            Text = text,
            X = b.StartX,
            Y = b.StartY,
            Width = b.AccumWidth * b.UserHScale,
            Height = b.EffFontSize,
            FontSize = b.EffFontSize,
            IsBold = b.IsBold,
            IsItalic = b.IsItalic,
            IsMonospace = b.IsMonospace,
            FontName = b.FontName,
            RotationDegrees = b.RotationDegrees,
            TextRiseRatio = b.RiseRatio,
            Sequence = _spans.Count,
        });
    }

    private void ShowText(byte[] bytes)
    {
        var font = _gs.Font;
        double fontSize = _gs.FontSize;
        double hs = _gs.Hscale / 100.0;
        double fmA = font?.FontMatrixA ?? 0.001;

        if (_buf == null) StartBuffer();
        var buf = _buf!;

        var codes = font != null ? font.DecodeCodes(bytes) : DefaultCodes(bytes);
        foreach (int code in codes)
        {
            string u = font != null ? font.CharToUnicode(code) : ((code >= 32 && code < 127) ? ((char)code).ToString() : "");
            double w0 = (font?.GlyphWidth(code) ?? 500.0) * fmA; // text-space width (em fraction)
            double tx = (w0 * fontSize + _gs.CharSpace + (code == 32 ? _gs.WordSpace : 0)) * hs;

            // Filter control chars like pdf_oxide (skip NUL and C0 except tab/newline/cr).
            foreach (var ch in u)
            {
                if (ch == '\0' || (char.IsControl(ch) && ch != '\t' && ch != '\n' && ch != '\r')) continue;
                buf.Text.Append(ch);
                if (!buf.HasRtl && PdfBidi.IsRtlText(ch)) buf.HasRtl = true;
            }

            // Advance text matrix.
            _tm = new Matrix(1, 0, 0, 1, tx, 0).Multiply(_tm);
            buf.AccumWidth += tx;
        }
    }

    private static IEnumerable<int> DefaultCodes(byte[] bytes)
    {
        foreach (var b in bytes) yield return b;
    }

    private void DoXObject(string name, PdfDict? xobjects, int depth)
    {
        var xobj = _doc.Resolve(xobjects?.Get(name));
        if (xobj is not PdfStream st) return;
        if (_doc.Resolve(st.Dict.Get("Subtype")).AsName() != "Form") return;
        // Save state, apply form Matrix, run form content with its resources.
        _stack.Push(_gs.Clone());
        // A Form XObject is path-isolated at both boundaries: pdf_oxide discards whatever
        // path construction is pending on the way in and on the way out
        // (`process_form_xobject_paths`, document.rs:18025 and :18204). Without that, a form
        // whose subpaths are painted only by an operator the extractor does not answer
        // (`B`, `B*`, `b*`) leaves them in the buffer, and the next `S` on the page emits
        // them as one path — a single primitive spanning the whole form, which bridges
        // unrelated ruling-line clusters and widens the table bounding boxes built from them.
        EndPath();
        var savedTm = _tm; var savedTlm = _tlm;
        if (_doc.Resolve(st.Dict.Get("Matrix")).AsArray() is PdfArray fm && fm.Items.Count >= 6)
        {
            var m = new Matrix(fm.Items[0].AsNumber() ?? 1, fm.Items[1].AsNumber() ?? 0,
                fm.Items[2].AsNumber() ?? 0, fm.Items[3].AsNumber() ?? 1,
                fm.Items[4].AsNumber() ?? 0, fm.Items[5].AsNumber() ?? 0);
            _gs.Ctm = m.Multiply(_gs.Ctm);
            _gs.PathCtm = RoundToSingle(m).MultiplySingle(_gs.PathCtm);
        }
        var formRes = _doc.Resolve(st.Dict.Get("Resources")).AsDict();
        try { Run(_doc.DecodeStream(st), formRes, depth + 1); } catch (Exception) when (Cancel.NotCancelling) { }
        EndPath();
        FlushBuffer();
        if (_stack.Count > 0) _gs = _stack.Pop();
        _tm = savedTm; _tlm = savedTlm;
    }

    private void SkipInlineImage(PdfLexer lex)
    {
        // Skip until "EI" delimiter.
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
}
