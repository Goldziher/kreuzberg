// Ported from crates/xberg/src/extraction/mathml.rs
// Presentation (and content) MathML → LaTeX converter.
//
// Converts `<math>` subtrees found in ODT/ODP embedded formula objects, EPUB XHTML content and
// raw HTML to LaTeX notation. Modeled on the OMML converter (`Internal/Ooxml/OmmlMath.cs`): the
// subtree is collected into an MmlNode tree, then recursively rendered. Unknown or unhandled
// elements degrade to their text content instead of failing the whole document.
//
// Callers either hold a parsed element already (the EPUB walker) or only the raw XML text (the
// ODT embedded-object reader, the HTML re-scan), so both entry points exist. The Rust
// SecurityBudget gating is omitted, as elsewhere in this port.

using System.Text;
using System.Xml.Linq;

// The namespace deliberately differs from the directory name: a `XRay.Content.Internal.Math` namespace
// would shadow `System.Math` for every file under `XRay.Content.Internal.*`.
namespace XRay.Content.Internal.MathMarkup;

internal static class MathMl
{
    /// <summary>
    /// Names of MathML elements that hold no rendered content and whose text (an alternate
    /// encoding annotation, e.g. StarMath or content MathML) must never leak into the output.
    /// </summary>
    private static readonly string[] AnnotationElements = { "annotation", "annotation-xml" };

    /// <summary>
    /// <c>annotation</c> encodings whose text is the LaTeX the author wrote. A document that
    /// ships one states the formula exactly, so it beats reconstructing LaTeX from the
    /// presentation tree, which can only approximate the author's spelling.
    /// </summary>
    private static readonly string[] TexAnnotationEncodings = { "application/x-tex", "text/x-tex", "tex", "latex" };

    // ── MmlNode tree ────────────────────────────────────────────────────────
    private abstract record MmlNode;

    /// <summary>LaTeX taken verbatim from a TeX <c>annotation</c> or a content-MathML branch.</summary>
    private sealed record Verbatim(string Latex) : MmlNode;

    /// <summary>Plain text from <c>mi</c>/<c>mn</c>/<c>mo</c>/<c>ms</c>.</summary>
    private sealed record Run(string Text) : MmlNode;

    /// <summary>Literal text from <c>mtext</c>: rendered as <c>\text{...}</c>.</summary>
    private sealed record TextRun(string Text) : MmlNode;

    /// <summary>A single blank space from <c>mspace</c>.</summary>
    private sealed record Space : MmlNode;

    private sealed record Frac(MmlNode Num, MmlNode Den) : MmlNode;
    private sealed record Sup(MmlNode Base, MmlNode Script) : MmlNode;
    private sealed record Sub(MmlNode Base, MmlNode Script) : MmlNode;
    private sealed record SubSup(MmlNode Base, MmlNode SubScript, MmlNode SupScript) : MmlNode;
    private sealed record Sqrt(MmlNode Body) : MmlNode;
    private sealed record Root(MmlNode Body, MmlNode Index) : MmlNode;

    /// <summary>Fenced group: <c>\left(open) a, b, ...\right(close)</c>.</summary>
    private sealed record Fenced(string Open, string Close, string Sep, List<MmlNode> Elements) : MmlNode;

    private sealed record Under(MmlNode Base, MmlNode UnderScript) : MmlNode;
    private sealed record Over(MmlNode Base, MmlNode OverScript) : MmlNode;
    private sealed record UnderOver(MmlNode Base, MmlNode UnderScript, MmlNode OverScript) : MmlNode;
    private sealed record Phantom(MmlNode Body) : MmlNode;
    private sealed record Table(List<List<MmlNode>> Rows) : MmlNode;

    /// <summary>Grouping container (<c>math</c>, <c>mrow</c>, unknown elements).</summary>
    private sealed record Group(List<MmlNode> Children) : MmlNode;

    // ── Entry points ─────────────────────────────────────────────────────────

    /// <summary>
    /// Convert a MathML XML fragment (a document whose root is <c>&lt;math&gt;</c>, or one nested
    /// anywhere inside the fragment) to LaTeX. Returns an empty string when the fragment does not
    /// parse.
    /// </summary>
    public static string ConvertMathmlStrToLatex(string xml)
    {
        // An embedded formula object carries its own DOCTYPE — OpenOffice writes
        // `<!DOCTYPE math:math PUBLIC "-//OpenOffice.org//DTD Modified W3C MathML 1.01//EN">` into
        // every one — and the XML reader rejects a DTD outright, so the formula would be dropped.
        // The declaration is removed rather than allowed: a formula needs none of it, and the
        // rejection still guards the entity expansion a hostile document would declare inside one.
        string stripped = StripDoctype(xml);
        XDocument doc;
        try
        {
            doc = XDocument.Parse(stripped, LoadOptions.PreserveWhitespace);
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            return "";
        }

        var root = doc.Root;
        if (root is null) return "";

        XElement mathNode = IsTag(root, "math")
            ? root
            : root.Descendants().FirstOrDefault(n => IsTag(n, "math")) ?? root;

        return ConvertMathmlNodeToLatex(mathNode);
    }

    /// <summary>Convert an already-parsed MathML element to LaTeX.</summary>
    public static string ConvertMathmlNodeToLatex(XElement node)
    {
        var output = new StringBuilder();
        RenderNode(CollectNode(node), output);
        return output.ToString();
    }

    /// <summary>Remove a <c>{\displaystyle ...}</c> / <c>{\textstyle ...}</c> wrapper around the
    /// whole expression.</summary>
    public static string StripStyleWrapper(string latex)
    {
        foreach (string prefix in new[] { "{\\displaystyle", "{\\textstyle", "{\\scriptstyle" })
        {
            if (!latex.StartsWith(prefix, StringComparison.Ordinal)) continue;
            string rest = latex[prefix.Length..];
            if (!rest.EndsWith('}')) continue;
            string inner = rest[..^1];

            // The wrapper must enclose everything: a brace that closes early means the tail
            // belongs to the formula.
            int depth = 1;
            foreach (char ch in inner)
            {
                if (ch == '{') depth++;
                else if (ch == '}') depth--;
                if (depth == 0) return latex;
            }
            return inner.Trim();
        }
        return latex;
    }

    /// <summary>
    /// Strip the math delimiters around a LaTeX source string
    /// (<c>extraction/derive.rs::strip_math_delimiters</c>).
    /// </summary>
    public static string StripMathDelimiters(string text)
    {
        string t = text.Trim();
        foreach (var (open, close) in new[] { ("$$", "$$"), ("\\[", "\\]"), ("$", "$") })
        {
            if (t.Length > open.Length + close.Length
                && t.StartsWith(open, StringComparison.Ordinal)
                && t.EndsWith(close, StringComparison.Ordinal))
            {
                string inner = t[open.Length..^close.Length];
                if (!ContainsUnescaped(inner, open) && !ContainsUnescaped(inner, close)) return inner.Trim();
            }
        }
        return t;
    }

    /// <summary>True when <paramref name="needle"/> occurs outside a backslash escape.
    /// <c>\$</c> is LaTeX for a literal dollar sign and does not end a math span.</summary>
    private static bool ContainsUnescaped(string text, string needle)
    {
        int from = 0;
        while (true)
        {
            int at = text.IndexOf(needle, from, StringComparison.Ordinal);
            if (at < 0) return false;
            if (at == 0 || text[at - 1] != '\\') return true;
            from = at + 1;
        }
    }

    // ── Tree builder ─────────────────────────────────────────────────────────

    private static bool IsTag(XElement element, string name) =>
        string.Equals(element.Name.LocalName, name, StringComparison.OrdinalIgnoreCase);

    /// <summary>Collect an element's children, dropping whitespace-only text nodes.</summary>
    private static List<MmlNode> CollectChildren(XElement parent)
    {
        var nodes = new List<MmlNode>();
        foreach (var child in parent.Nodes())
        {
            if (child is XElement e) nodes.Add(CollectNode(e));
            else if (child is XText t && t.Value.Trim().Length != 0) nodes.Add(new Run(t.Value));
        }
        return nodes;
    }

    /// <summary>Collect the Nth element child, or an empty group when there are fewer.</summary>
    private static MmlNode CollectNthChild(XElement parent, int index)
    {
        var child = parent.Elements().Skip(index).FirstOrDefault();
        return child is null ? new Group(new List<MmlNode>()) : CollectNode(child);
    }

    private static MmlNode CollectNode(XElement node)
    {
        string tag = node.Name.LocalName;
        if (AnnotationElements.Any(a => string.Equals(a, tag, StringComparison.OrdinalIgnoreCase)))
            return new Group(new List<MmlNode>());

        switch (tag.ToLowerInvariant())
        {
            case "mi":
            case "mn":
            case "ms":
            case "mo":
                return new Run(CollectText(node));
            case "mtext":
                return new TextRun(CollectText(node));
            case "mspace":
                return new Space();
            // Content MathML states meaning rather than layout, so it converts by operator. It
            // appears as a `math` child in content documents and inside `annotation-xml` in
            // mixed ones.
            case "apply":
            case "piecewise":
            case "matrix":
            case "vector":
            case "set":
            case "list":
            case "ci":
            case "cn":
            case "csymbol":
                return new Verbatim(ConvertContentNode(node));
            case "semantics":
                return CollectSemantics(node);
            case "mfrac":
                return new Frac(CollectNthChild(node, 0), CollectNthChild(node, 1));
            case "msup":
                return new Sup(CollectNthChild(node, 0), CollectNthChild(node, 1));
            case "msub":
                return new Sub(CollectNthChild(node, 0), CollectNthChild(node, 1));
            case "msubsup":
                return new SubSup(CollectNthChild(node, 0), CollectNthChild(node, 1), CollectNthChild(node, 2));
            case "msqrt":
                return new Sqrt(new Group(CollectChildren(node)));
            case "mroot":
                return new Root(CollectNthChild(node, 0), CollectNthChild(node, 1));
            case "mfenced":
                return CollectFenced(node);
            case "munder":
                return new Under(CollectNthChild(node, 0), CollectNthChild(node, 1));
            case "mover":
                return new Over(CollectNthChild(node, 0), CollectNthChild(node, 1));
            case "munderover":
                return new UnderOver(CollectNthChild(node, 0), CollectNthChild(node, 1), CollectNthChild(node, 2));
            case "mphantom":
                return new Phantom(new Group(CollectChildren(node)));
            case "mtable":
                return CollectTable(node);
            // Pure grouping/styling wrappers: their children render in sequence with no markup
            // of their own. An element the converter does not know degrades the same way, to its
            // content, rather than failing the document.
            case "math":
            case "mrow":
            case "mstyle":
            case "mpadded":
            case "merror":
            default:
                return new Group(CollectChildren(node));
        }
    }

    private static MmlNode CollectSemantics(XElement node)
    {
        if (TexAnnotation(node) is { } tex) return new Verbatim(tex);

        var children = node.Elements()
            .Where(c => !AnnotationElements.Any(a => string.Equals(a, c.Name.LocalName, StringComparison.OrdinalIgnoreCase)))
            .Select(CollectNode)
            .ToList();

        // A document may carry only the content branch, in which case the presentation side
        // renders to nothing and the meaning is all there is to work from.
        if (RenderNodes(children).Trim().Length == 0 && ContentAnnotation(node) is { } latex)
            return new Verbatim(latex);

        return new Group(children);
    }

    /// <summary>
    /// The LaTeX of a <c>semantics</c> child annotation that carries TeX, if any.
    /// </summary>
    /// <remarks>
    /// Renderers wrap the whole expression in <c>{\displaystyle ...}</c> or <c>{\textstyle ...}</c>
    /// to state the style the surrounding document set. That wrapper is presentation, not the
    /// formula, so it comes off; <c>$</c> delimiters come off for the same reason.
    /// </remarks>
    private static string? TexAnnotation(XElement node)
    {
        foreach (var child in node.Elements())
        {
            if (!IsTag(child, "annotation")) continue;
            string? encoding = child.Attribute("encoding")?.Value;
            if (encoding is null) continue;
            if (!TexAnnotationEncodings.Any(k => string.Equals(k, encoding.Trim(), StringComparison.OrdinalIgnoreCase)))
                continue;

            string latex = StripStyleWrapper(StripMathDelimiters(CollectText(child).Trim()));
            if (latex.Length != 0) return latex;
        }
        return null;
    }

    /// <summary>The LaTeX of a content-MathML <c>annotation-xml</c> child, if the element has one.</summary>
    private static string? ContentAnnotation(XElement node)
    {
        foreach (var child in node.Elements())
        {
            if (!IsTag(child, "annotation-xml")) continue;
            string? encoding = child.Attribute("encoding")?.Value;
            if (encoding is null || !string.Equals(encoding.Trim(), "MathML-Content", StringComparison.OrdinalIgnoreCase))
                continue;

            foreach (var inner in child.Elements())
            {
                string latex = ConvertContentNode(inner);
                if (latex.Trim().Length != 0) return latex.Trim();
            }
        }
        return null;
    }

    /// <summary>Collect the direct text content of a leaf element.</summary>
    private static string CollectText(XElement node)
    {
        var text = new StringBuilder();
        // Only real text nodes: MathML fixtures commonly annotate an entity with a comment
        // (`<mo>&#x222B;<!-- ∫ --></mo>`) whose body must not be rendered a second time.
        foreach (var child in node.Nodes())
        {
            if (child is not XText t) continue;
            foreach (var rune in t.Value.EnumerateRunes())
                if (!IsPrivateUse(rune)) text.Append(rune.ToString());
        }
        return text.ToString();
    }

    /// <summary>
    /// Report whether <paramref name="rune"/> sits in a Unicode private use area.
    /// </summary>
    /// <remarks>
    /// A private use codepoint carries no meaning outside the font that defines it. OpenOffice
    /// writes its stretchy fences that way, and a renderer shows a missing glyph or rejects the
    /// formula, so the character is dropped rather than passed through as LaTeX.
    /// </remarks>
    private static bool IsPrivateUse(Rune rune) =>
        rune.Value is (>= 0xE000 and <= 0xF8FF) or (>= 0xF0000 and <= 0xFFFFD) or (>= 0x100000 and <= 0x10FFFD);

    private static string StripPrivateUse(string value)
    {
        var sb = new StringBuilder(value.Length);
        foreach (var rune in value.EnumerateRunes())
            if (!IsPrivateUse(rune)) sb.Append(rune.ToString());
        return sb.ToString();
    }

    /// <summary>Collect an <c>mfenced</c>: open/close/separators attributes plus one element per
    /// fenced argument.</summary>
    private static MmlNode CollectFenced(XElement node)
    {
        // A fence may be a private use codepoint, which OpenOffice writes for the bracket shapes
        // of its own font. It has no meaning to a renderer, so it is dropped the way it is in
        // element text.
        string open = node.Attribute("open") is { } o ? StripPrivateUse(o.Value) : "(";
        string close = node.Attribute("close") is { } c ? StripPrivateUse(c.Value) : ")";

        string sep = ",";
        if (node.Attribute("separators") is { } s)
        {
            string stripped = StripPrivateUse(s.Value);
            foreach (var rune in stripped.EnumerateRunes())
            {
                sep = rune.ToString();
                break;
            }
        }

        var elements = node.Elements().Select(CollectNode).ToList();
        return new Fenced(open, close, sep, elements);
    }

    /// <summary>Collect an <c>mtable</c> into rows of cells (<c>mtr</c> &gt; <c>mtd</c>).</summary>
    private static MmlNode CollectTable(XElement node)
    {
        var rows = new List<List<MmlNode>>();
        foreach (var row in node.Elements().Where(e => IsTag(e, "mtr")))
        {
            var cells = row.Elements()
                .Where(c => IsTag(c, "mtd"))
                .Select(c => (MmlNode)new Group(CollectChildren(c)))
                .ToList();
            rows.Add(cells);
        }
        return new Table(rows);
    }

    // ── Content MathML ───────────────────────────────────────────────────────

    /// <summary>Content MathML operators that render as an infix chain.</summary>
    private static readonly (string Name, string Latex)[] InfixOperators =
    {
        ("plus", "+"),
        ("minus", "-"),
        ("times", " \\times "),
        ("divide", " \\div "),
        ("eq", "="),
        ("neq", " \\ne "),
        ("lt", " < "),
        ("gt", " > "),
        ("leq", " \\le "),
        ("geq", " \\ge "),
        ("equivalent", " \\equiv "),
        ("approx", " \\approx "),
        ("and", " \\land "),
        ("or", " \\lor "),
        ("implies", " \\implies "),
        ("in", " \\in "),
        ("notin", " \\notin "),
        ("subset", " \\subset "),
        ("prsubset", " \\subsetneq "),
        ("union", " \\cup "),
        ("intersect", " \\cap "),
        ("setdiff", " \\setminus "),
        ("cartesianproduct", " \\times "),
        ("compose", " \\circ "),
    };

    /// <summary>Content MathML operators that render as a named LaTeX function.</summary>
    private static readonly (string Name, string Latex)[] FunctionOperators =
    {
        ("sin", "\\sin"), ("cos", "\\cos"), ("tan", "\\tan"), ("sec", "\\sec"), ("csc", "\\csc"),
        ("cot", "\\cot"), ("arcsin", "\\arcsin"), ("arccos", "\\arccos"), ("arctan", "\\arctan"),
        ("sinh", "\\sinh"), ("cosh", "\\cosh"), ("tanh", "\\tanh"), ("exp", "\\exp"), ("ln", "\\ln"),
        ("log", "\\log"), ("det", "\\det"), ("gcd", "\\gcd"), ("max", "\\max"), ("min", "\\min"),
    };

    /// <summary>
    /// Convert a content-MathML <c>apply</c> subtree to LaTeX.
    /// </summary>
    /// <remarks>
    /// Content MathML states what a formula <em>means</em> (<c>&lt;apply&gt;&lt;plus/&gt;…</c>)
    /// rather than how it looks, so it converts by operator rather than by layout. An operator
    /// with no LaTeX spelling becomes <c>\operatorname{name}(args)</c>, which parses and still
    /// names what the source said.
    /// </remarks>
    private static string ConvertApply(XElement node)
    {
        var children = node.Elements().ToList();
        if (children.Count == 0) return "";

        string name = children[0].Name.LocalName.ToLowerInvariant();

        // `bvar`, `lowlimit`, `uplimit`, `degree` and `condition` qualify the operator;
        // everything else is an operand.
        var operands = new List<XElement>();
        XElement? bvar = null, lower = null, upper = null, degree = null;
        foreach (var child in children.Skip(1))
        {
            switch (child.Name.LocalName.ToLowerInvariant())
            {
                case "bvar": bvar = child; break;
                case "lowlimit":
                case "condition": lower = child; break;
                case "uplimit": upper = child; break;
                case "degree": degree = child; break;
                default: operands.Add(child); break;
            }
        }

        var rendered = operands.Select(ConvertContentNode).ToList();

        if (Array.Find(InfixOperators, op => op.Name == name) is { Name: not null } infix)
        {
            // Unary minus reads as negation rather than subtraction.
            if (name == "minus" && rendered.Count == 1) return "-" + rendered[0];
            return string.Join(infix.Latex, rendered);
        }
        if (Array.Find(FunctionOperators, op => op.Name == name) is { Name: not null } func)
            return $"{func.Latex}\\left({string.Join(", ", rendered)}\\right)";

        switch (name)
        {
            case "power" when rendered.Count == 2:
                return $"{rendered[0]}^{{{rendered[1]}}}";
            case "root":
            {
                string index = Qualifier(degree);
                string radicand = rendered.Count > 0 ? rendered[0] : "";
                return index.Length == 0 || index == "2"
                    ? $"\\sqrt{{{radicand}}}"
                    : $"\\sqrt[{index}]{{{radicand}}}";
            }
            case "abs":
                return $"\\left|{string.Join(", ", rendered)}\\right|";
            case "floor":
                return $"\\lfloor {string.Join(", ", rendered)}\\rfloor";
            case "ceiling":
                return $"\\lceil {string.Join(", ", rendered)}\\rceil";
            case "factorial":
                return string.Concat(rendered) + "!";
            case "sum":
            case "product":
            case "int":
            {
                string command = name switch { "sum" => "\\sum", "product" => "\\prod", _ => "\\int" };
                string var = Qualifier(bvar);
                string from = Qualifier(lower);
                string to = Qualifier(upper);
                var output = new StringBuilder(command);
                if (from.Length != 0)
                {
                    string start = var.Length == 0 ? from : $"{var}={from}";
                    output.Append($"_{{{start}}}");
                }
                else if (var.Length != 0)
                {
                    output.Append($"_{{{var}}}");
                }
                if (to.Length != 0) output.Append($"^{{{to}}}");
                output.Append(' ').Append(string.Concat(rendered));
                if (name == "int" && var.Length != 0) output.Append($"\\,d{var}");
                return output.ToString();
            }
            case "diff":
            {
                string var = Qualifier(bvar);
                string body = string.Concat(rendered);
                return var.Length == 0 ? $"\\frac{{d}}{{dx}}{body}" : $"\\frac{{d}}{{d{var}}}{body}";
            }
            // An operator the mapping does not name still parses and still says what the source
            // said.
            default:
                return $"\\operatorname{{{name}}}\\left({string.Join(", ", rendered)}\\right)";
        }

        static string Qualifier(XElement? qualifier) =>
            qualifier is null ? "" : string.Concat(qualifier.Elements().Select(ConvertContentNode));
    }

    /// <summary>Convert one content-MathML node to LaTeX.</summary>
    private static string ConvertContentNode(XElement node)
    {
        switch (node.Name.LocalName.ToLowerInvariant())
        {
            case "apply":
                return ConvertApply(node);
            case "ci":
            case "cn":
            case "csymbol":
            {
                var output = new StringBuilder();
                MathSymbols.RenderRunText(CollectText(node).Trim(), output);
                return output.ToString();
            }
            case "matrix":
            case "vector":
            {
                var rows = new List<string>();
                foreach (var row in node.Elements())
                {
                    var cells = row.Elements().Select(ConvertContentNode).ToList();
                    rows.Add(cells.Count == 0 ? ConvertContentNode(row) : string.Join(" & ", cells));
                }
                return $"\\begin{{pmatrix}}{string.Join(" \\\\ ", rows)}\\end{{pmatrix}}";
            }
            case "piecewise":
            {
                var rows = new List<string>();
                foreach (var piece in node.Elements())
                {
                    var parts = piece.Elements().Select(ConvertContentNode).ToList();
                    rows.Add(piece.Name.LocalName.ToLowerInvariant() == "otherwise"
                        ? $"{string.Concat(parts)} & \\text{{otherwise}}"
                        : string.Join(" & \\text{if }", parts));
                }
                return $"\\begin{{cases}}{string.Join(" \\\\ ", rows)}\\end{{cases}}";
            }
            case "list":
            case "set":
            {
                string inner = string.Join(", ", node.Elements().Select(ConvertContentNode));
                return IsTag(node, "set") ? $"\\{{{inner}\\}}" : $"\\left({inner}\\right)";
            }
            // A constant such as `<pi/>` or `<exponentiale/>` carries its meaning in its name.
            case "pi": return "\\pi";
            case "exponentiale": return "e";
            case "imaginaryi": return "i";
            case "infinity": return "\\infty";
            case "true": return "\\text{true}";
            case "false": return "\\text{false}";
            case "emptyset": return "\\emptyset";
            default:
                return CollectText(node).Trim();
        }
    }

    // ── Renderer ─────────────────────────────────────────────────────────────

    private static string RenderNodes(List<MmlNode> nodes)
    {
        var output = new StringBuilder();
        foreach (var node in nodes) RenderNode(node, output);
        return output.ToString();
    }

    private static void RenderNode(MmlNode node, StringBuilder output)
    {
        switch (node)
        {
            case Verbatim v:
                output.Append(v.Latex);
                break;
            case Run r:
                MathSymbols.RenderRunText(r.Text, output);
                break;
            case TextRun t:
                RenderTextContent(t.Text, output);
                break;
            case Space:
                output.Append(' ');
                break;
            case Frac f:
                output.Append("\\frac{");
                RenderNode(f.Num, output);
                output.Append("}{");
                RenderNode(f.Den, output);
                output.Append('}');
                break;
            case Sup s:
                RenderArg(s.Base, output);
                output.Append("^{");
                RenderNode(s.Script, output);
                output.Append('}');
                break;
            case Sub s:
                RenderArg(s.Base, output);
                output.Append("_{");
                RenderNode(s.Script, output);
                output.Append('}');
                break;
            case SubSup s:
                RenderArg(s.Base, output);
                output.Append("_{");
                RenderNode(s.SubScript, output);
                output.Append("}^{");
                RenderNode(s.SupScript, output);
                output.Append('}');
                break;
            case Sqrt s:
                output.Append("\\sqrt{");
                RenderNode(s.Body, output);
                output.Append('}');
                break;
            case Root r:
                output.Append("\\sqrt[");
                RenderNode(r.Index, output);
                output.Append("]{");
                RenderNode(r.Body, output);
                output.Append('}');
                break;
            case Fenced f:
                RenderFenced(f, output);
                break;
            case Under u:
                if (UnderScriptCommand(u.UnderScript) is { } underCmd)
                {
                    output.Append(underCmd).Append('{');
                    RenderNode(u.Base, output);
                    output.Append('}');
                }
                else
                {
                    output.Append("\\underset{");
                    RenderNode(u.UnderScript, output);
                    output.Append("}{");
                    RenderNode(u.Base, output);
                    output.Append('}');
                }
                break;
            case Over o:
                if (OverScriptCommand(o.OverScript, o.Base) is { } overCmd)
                {
                    output.Append(overCmd).Append('{');
                    RenderNode(o.Base, output);
                    output.Append('}');
                }
                else
                {
                    output.Append("\\overset{");
                    RenderNode(o.OverScript, output);
                    output.Append("}{");
                    RenderNode(o.Base, output);
                    output.Append('}');
                }
                break;
            case UnderOver uo:
                output.Append("\\overset{");
                RenderNode(uo.OverScript, output);
                output.Append("}{\\underset{");
                RenderNode(uo.UnderScript, output);
                output.Append("}{");
                RenderNode(uo.Base, output);
                output.Append("}}");
                break;
            case Phantom p:
                output.Append("\\phantom{");
                RenderNode(p.Body, output);
                output.Append('}');
                break;
            case Table t:
                output.Append("\\begin{matrix}");
                for (int i = 0; i < t.Rows.Count; i++)
                {
                    if (i > 0) output.Append(" \\\\ ");
                    for (int j = 0; j < t.Rows[i].Count; j++)
                    {
                        if (j > 0) output.Append(" & ");
                        RenderNode(t.Rows[i][j], output);
                    }
                }
                output.Append("\\end{matrix}");
                break;
            case Group g:
                output.Append(RenderNodes(g.Children));
                break;
        }
    }

    private static void RenderFenced(Fenced fenced, StringBuilder output)
    {
        // Authors use `mfenced` as plain grouping with operators as direct children; inserting the
        // spec-default comma separators there turns `(1 - x)` into `(1,-,x)`. Suppress separators
        // when any child is itself an infix operator.
        string sep = fenced.Elements.Any(IsOperatorChild) ? "" : fenced.Sep;
        string? left = FenceChrToLatex(fenced.Open);
        string? right = FenceChrToLatex(fenced.Close);

        if (left is not null && right is not null)
        {
            output.Append("\\left").Append(left);
            for (int i = 0; i < fenced.Elements.Count; i++)
            {
                if (i > 0) output.Append(sep);
                RenderNode(fenced.Elements[i], output);
            }
            output.Append("\\right").Append(right);
            return;
        }

        // A fence char LaTeX cannot use after `\left`: emit the fences as plain glyphs instead of
        // producing an unparseable string.
        MathSymbols.RenderRunText(fenced.Open, output);
        for (int i = 0; i < fenced.Elements.Count; i++)
        {
            if (i > 0) output.Append(sep);
            RenderNode(fenced.Elements[i], output);
        }
        MathSymbols.RenderRunText(fenced.Close, output);
    }

    /// <summary>
    /// True when <paramref name="s"/> is exactly one balanced brace group: the opening brace's
    /// closer is the final character. <c>{a}^{b}</c> starts with <c>{</c> and ends with <c>}</c>
    /// but is two atoms — treating it as pre-braced produces double scripts when another script
    /// attaches.
    /// </summary>
    private static bool IsSingleBraceGroup(string s)
    {
        if (!s.StartsWith('{') || !s.EndsWith('}')) return false;
        int depth = 0;
        bool escaped = false;
        for (int i = 0; i < s.Length; i++)
        {
            if (escaped) { escaped = false; continue; }
            switch (s[i])
            {
                case '\\': escaped = true; break;
                case '{': depth++; break;
                case '}':
                    if (depth > 0) depth--;
                    if (depth == 0) return i == s.Length - 1;
                    break;
            }
        }
        return false;
    }

    /// <summary>
    /// Render <c>mtext</c> content. Plain text goes inside <c>\text{...}</c> with text-mode
    /// escaping; characters that map to math commands (Greek letters, operators) are emitted
    /// <em>outside</em> the <c>\text</c> group, because commands like <c>\Delta</c> are
    /// math-mode-only and fail inside <c>\text{}</c>.
    /// </summary>
    private static void RenderTextContent(string text, StringBuilder output)
    {
        bool inText = false;
        foreach (var rune in text.EnumerateRunes())
        {
            if (MathSymbols.UnicodeToLatex(rune) is { } latex)
            {
                if (inText)
                {
                    output.Append('}');
                    inText = false;
                }
                output.Append(latex);
                continue;
            }
            if (!inText)
            {
                output.Append("\\text{");
                inText = true;
            }
            switch (rune.Value)
            {
                case '{': output.Append("\\{"); break;
                case '}': output.Append("\\}"); break;
                case '&': output.Append("\\&"); break;
                case '%': output.Append("\\%"); break;
                case '#': output.Append("\\#"); break;
                case '$': output.Append("\\$"); break;
                case '_': output.Append("\\_"); break;
                case '\\': output.Append("\\textbackslash "); break;
                case '^': output.Append("\\textasciicircum "); break;
                case '~': output.Append("\\textasciitilde "); break;
                default: output.Append(rune.ToString()); break;
            }
        }
        if (inText) output.Append('}');
    }

    /// <summary>The raw script text of an accent-like script node (<c>mo</c>/<c>mi</c> leaf,
    /// possibly inside grouping), or null when the script is real content.</summary>
    private static string? ScriptLeafText(MmlNode node) => node switch
    {
        Run r => r.Text.Trim(),
        Group g when g.Children.Count == 1 => ScriptLeafText(g.Children[0]),
        _ => null,
    };

    /// <summary>True when the base renders to a single glyph (possibly one LaTeX command), used
    /// to pick <c>\bar</c>/<c>\vec</c> over <c>\overline</c>/<c>\overrightarrow</c>.</summary>
    private static bool BaseIsSingleGlyph(MmlNode node)
    {
        var rendered = new StringBuilder();
        RenderNode(node, rendered);
        string t = rendered.ToString().Trim();
        return RuneCount(t) == 1 || (t.StartsWith('\\') && AllAsciiLetters(t, 1));
    }

    private static int RuneCount(string s)
    {
        int n = 0;
        foreach (var _ in s.EnumerateRunes()) n++;
        return n;
    }

    private static bool AllAsciiLetters(string s, int from)
    {
        for (int i = from; i < s.Length; i++)
            if (!char.IsAsciiLetter(s[i])) return false;
        return true;
    }

    /// <summary>
    /// Map an <c>mover</c> script char to a LaTeX accent command. MathML sources write accents as
    /// literal combining/spacing characters (<c>&lt;mover&gt;&lt;mi&gt;x&lt;/mi&gt;&lt;mo&gt;^&lt;/mo&gt;&lt;/mover&gt;</c>);
    /// <c>\overset{^}{x}</c> is not valid LaTeX (a bare <c>^</c> needs a group), so these must
    /// become accent macros.
    /// </summary>
    private static string? OverScriptCommand(MmlNode over, MmlNode baseNode)
    {
        string? script = ScriptLeafText(over);
        if (script is null) return null;
        return script switch
        {
            "^" or "ˆ" or "̂" => "\\hat",
            "~" or "˜" or "̃" or "∼" => "\\tilde",
            "˙" or "̇" => "\\dot",
            "¨" or "̈" => "\\ddot",
            "¯" or "‾" or "̄" or "̅" => BaseIsSingleGlyph(baseNode) ? "\\bar" : "\\overline",
            "→" or "⃗" => BaseIsSingleGlyph(baseNode) ? "\\vec" : "\\overrightarrow",
            "ˇ" or "̌" => "\\check",
            "˘" or "̆" => "\\breve",
            "´" or "́" => "\\acute",
            "`" or "̀" => "\\grave",
            "˚" or "̊" => "\\mathring",
            "⏞" => "\\overbrace",
            _ => null,
        };
    }

    /// <summary>Map an <c>munder</c> script char to a LaTeX command, like
    /// <see cref="OverScriptCommand"/>.</summary>
    private static string? UnderScriptCommand(MmlNode under)
    {
        string? script = ScriptLeafText(under);
        if (script is null) return null;
        return script switch
        {
            "_" or "̲" or "ˍ" or "¯" or "‾" => "\\underline",
            "⏟" => "\\underbrace",
            _ => null,
        };
    }

    /// <summary>
    /// Render an argument (sup/sub base), wrapping in braces unless it is a single atom (one
    /// character, one LaTeX command, or one brace group).
    /// </summary>
    /// <remarks>
    /// A compound base that already carries a script (<c>\lambda _{1}^{'}</c>) MUST be wrapped, or
    /// attaching the outer script produces a double superscript. An empty base (script-only markup
    /// like tensor <c>{}_{,\nu}</c>) renders as <c>{}</c> so the script cannot fuse onto the
    /// preceding atom.
    /// </remarks>
    private static void RenderArg(MmlNode node, StringBuilder output)
    {
        var buffer = new StringBuilder();
        RenderNode(node, buffer);
        string rendered = buffer.ToString();
        string trimmed = rendered.Trim();
        if (trimmed.Length == 0)
        {
            output.Append("{}");
            return;
        }

        bool singleChar = RuneCount(trimmed) == 1;
        bool singleCommand = trimmed.StartsWith('\\') && trimmed.Length > 1 && AllAsciiLetters(trimmed, 1);
        if (singleChar || singleCommand || IsSingleBraceGroup(trimmed)) output.Append(rendered);
        else output.Append('{').Append(trimmed).Append('}');
    }

    /// <summary>True when a fenced child renders to a bare infix operator, meaning the
    /// <c>mfenced</c> is grouping an expression, not listing arguments.</summary>
    private static bool IsOperatorChild(MmlNode node) => node is Run r && r.Text.Trim() switch
    {
        "+" or "-" or "−" or "=" or "±" or "×" or "⋅" or "/" or "<" or ">"
            or "≤" or "≥" => true,
        _ => false,
    };

    /// <summary>
    /// Map an <c>mfenced</c> open/close character to a LaTeX delimiter valid after
    /// <c>\left</c>/<c>\right</c>, or null for characters LaTeX cannot use there. Word-form
    /// commands carry a trailing space so following content never glues onto the control word
    /// (<c>\langle A</c>, not <c>\langleA</c>).
    /// </summary>
    private static string? FenceChrToLatex(string chr) => chr switch
    {
        "(" => "(",
        ")" => ")",
        "[" => "[",
        "]" => "]",
        "{" => "\\{",
        "}" => "\\}",
        "|" or "∣" => "|",
        "‖" or "∥" => "\\|",
        "〈" or "⟨" => "\\langle ",
        "〉" or "⟩" => "\\rangle ",
        "⌊" => "\\lfloor ",
        "⌋" => "\\rfloor ",
        "⌈" => "\\lceil ",
        "⌉" => "\\rceil ",
        "/" => "/",
        "\\" => "\\backslash ",
        "" => ".",
        _ => null,
    };

    /// <summary>Return <paramref name="xml"/> without its <c>&lt;!DOCTYPE ...&gt;</c> declaration
    /// (<c>utils/xml_utils.rs::strip_doctype</c>).</summary>
    private static string StripDoctype(string xml)
    {
        int start = xml.IndexOf("<!DOCTYPE", StringComparison.Ordinal);
        if (start < 0) return xml;

        string tail = xml[(start + "<!DOCTYPE".Length)..];
        // An internal subset may hold a `>` inside its brackets, so the scan tracks bracket depth
        // and takes the first `>` outside them.
        int depth = 0;
        int end = -1;
        for (int i = 0; i < tail.Length; i++)
        {
            char ch = tail[i];
            if (ch == '[') depth++;
            else if (ch == ']') { if (depth > 0) depth--; }
            else if (ch == '>' && depth == 0) { end = i; break; }
        }
        if (end < 0) return xml;

        return xml[..start] + tail[(end + 1)..];
    }
}
