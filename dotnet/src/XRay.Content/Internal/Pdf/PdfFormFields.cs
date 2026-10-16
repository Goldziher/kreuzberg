// AcroForm field extraction, ported from pdf_oxide's `FormExtractor::extract_fields`
// (extractors/forms.rs) plus the value mapping in crates/xberg/src/pdf/oxide/forms.rs
// (`map_form_field` / `map_field_value`).
//
// Distinct from the per-page widget scan in `PdfExtractor.CollectWidgetTextValues`: that one
// splices values into a page's text at extraction time, this one models the document-wide
// field tree with fully qualified names so filled values can be surfaced as document elements.
using System.Text;
using XRay.Content.Internal.PdfOxide.Fonts;
using XRay.Content.Types;

namespace XRay.Content.Internal.Pdf;

/// <summary>One AcroForm field: partial name, fully qualified name, and rendered value.</summary>
internal sealed class PdfAcroFormField
{
    public string Name = "";
    public string FullName = "";
    public string? Value;
}

internal static class PdfFormFields
{
    /// <summary>A /Parent or /Kids cycle would otherwise recurse forever on a malformed file.</summary>
    private const int MaxFieldDepth = 32;

    /// <summary>Walk <c>/Root /AcroForm /Fields</c> depth-first, in document order.</summary>
    public static List<PdfAcroFormField> Extract(PdfDocument pdf)
    {
        var result = new List<PdfAcroFormField>();
        var acroForm = pdf.Resolve(pdf.Catalog?.Get("AcroForm")).AsDict();
        var fields = pdf.Resolve(acroForm?.Get("Fields")).AsArray();
        if (fields is null) return result;
        foreach (var field in fields.Items) ExtractRecursive(pdf, field, "", result, 0);
        return result;
    }

    private static void ExtractRecursive(
        PdfDocument pdf, PdfObject? fieldRef, string parentName, List<PdfAcroFormField> result, int depth)
    {
        if (depth > MaxFieldDepth) return;
        var dict = pdf.Resolve(fieldRef).AsDict();
        if (dict is null) return;

        string partialName = pdf.Resolve(dict.Get("T")).AsStringBytes() is { } nameBytes
            ? DecodeTextString(nameBytes)
            : "";

        string fullName = parentName.Length == 0 ? partialName
            : partialName.Length == 0 ? parentName
            : parentName + "." + partialName;

        // Kids are visited before the parent is judged, so a grouping node that is itself
        // skipped still contributes its name to the children's qualified names.
        if (pdf.Resolve(dict.Get("Kids")).AsArray() is { } kids)
            foreach (var kid in kids.Items) ExtractRecursive(pdf, kid, fullName, result, depth + 1);

        string? fieldType = pdf.Resolve(dict.Get("FT")).AsName();
        // A parent carrying /T but no /FT (a logical grouping such as a multi-box SSN field)
        // is still surfaced; only a node with neither is nothing to report.
        if (fieldType is null && partialName.Length == 0) return;

        result.Add(new PdfAcroFormField
        {
            Name = partialName,
            FullName = fullName,
            Value = MapFieldValue(pdf.Resolve(dict.Get("V")), fieldType),
        });
    }

    /// <summary>
    /// The field's /V rendered as the single string the typed model carries: text verbatim,
    /// booleans as <c>true</c>/<c>false</c>, names as themselves, multi-select arrays joined.
    /// </summary>
    private static string? MapFieldValue(PdfObject? value, string? fieldType) => value switch
    {
        PdfString s => DecodeTextString(s.Bytes),
        // Checkbox and radio states are the only names read as booleans, and only the four
        // conventional on/off spellings; any other name is a radio group's export value.
        PdfName n when fieldType == "Btn" => n.Value switch
        {
            "Yes" or "On" => "true",
            "No" or "Off" => "false",
            _ => n.Value,
        },
        PdfName n => n.Value,
        PdfBool b => b.Value ? "true" : "false",
        PdfArray a => JoinArrayValue(a),
        _ => null,
    };

    private static string? JoinArrayValue(PdfArray array)
    {
        var parts = new List<string>();
        foreach (var item in array.Items)
        {
            if (item is PdfString s) parts.Add(DecodeTextString(s.Bytes));
            else if (item is PdfName n) parts.Add(n.Value);
        }
        return parts.Count == 0 ? null : string.Join(", ", parts);
    }

    /// <summary>
    /// UTF-16BE when the BOM says so, PDFDocEncoding otherwise (ISO 32000-1 §7.9.2.2).
    /// Bytes with no PDFDocEncoding character are dropped rather than replaced.
    /// </summary>
    internal static string DecodeTextString(byte[] bytes)
    {
        if (bytes.Length >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF)
        {
            var utf16 = new StringBuilder();
            for (int i = 2; i + 1 < bytes.Length; i += 2) utf16.Append((char)((bytes[i] << 8) | bytes[i + 1]));
            // Ill-formed UTF-16 has no decoding at all upstream, and half a surrogate pair
            // rendered as a replacement character would be worse than reporting no value.
            return IsWellFormedUtf16(utf16) ? utf16.ToString() : "";
        }
        var sb = new StringBuilder(bytes.Length);
        foreach (byte b in bytes)
            if (OxEncodingTables.PdfDocEncodingLookup(b) is { } c) sb.Append(c);
        return sb.ToString();
    }

    private static bool IsWellFormedUtf16(StringBuilder text)
    {
        for (int i = 0; i < text.Length; i++)
        {
            if (!char.IsSurrogate(text[i])) continue;
            if (!char.IsHighSurrogate(text[i]) || i + 1 >= text.Length || !char.IsLowSurrogate(text[i + 1]))
                return false;
            i++;
        }
        return true;
    }
}

/// <summary>
/// The typed form model: every terminal field with its inherited type, flags and value, its
/// options, and where its first widget sits. Built beside the upstream-shaped
/// <see cref="PdfFormFields.Extract"/> (which feeds the text injection and must keep its shape).
/// </summary>
internal static class PdfFormModel
{
    private const int MaxDepth = 32;
    private const int MaxFields = 100_000;

    // Field flag bits (ISO 32000-1 Tables 221, 226, 228, 230; bit n is 1 << (n - 1)).
    private const uint ReadOnlyBit = 1u << 0;
    private const uint RequiredBit = 1u << 1;
    private const uint RadioBit = 1u << 15;
    private const uint PushButtonBit = 1u << 16;
    private const uint ComboBit = 1u << 17;
    private const uint MultiSelectBit = 1u << 21;

    public static List<PdfFormField> Extract(PdfDocument pdf)
    {
        var result = new List<PdfFormField>();
        var acroForm = pdf.Resolve(pdf.Catalog?.Get("AcroForm")).AsDict();
        if (pdf.Resolve(acroForm?.Get("Fields")).AsArray() is not { } fields) return result;

        var widgetPages = WidgetPages(pdf);
        var visited = new HashSet<PdfDict>();
        foreach (var field in fields.Items)
            Walk(pdf, field, null, "", Inherited.Empty, widgetPages, visited, result, 0);
        return result;
    }

    /// <summary>Attributes a kid inherits from its ancestors.</summary>
    private readonly record struct Inherited(PdfObject? Ft, PdfObject? Ff, PdfObject? V, PdfObject? Dv, PdfObject? Opt)
    {
        public static readonly Inherited Empty = new(null, null, null, null, null);
    }

    private static Dictionary<PdfDict, int> WidgetPages(PdfDocument pdf)
    {
        var map = new Dictionary<PdfDict, int>(ReferenceEqualityComparer.Instance as IEqualityComparer<PdfDict>);
        int pageCount = pdf.PageCount;
        for (int i = 0; i < pageCount; i++)
        {
            Cancel.Every(i, 0x0F);
            foreach (var annot in PdfAnnotations.PageAnnotationDicts(pdf, i))
                if (pdf.Resolve(annot.Get("Subtype")).AsName() == "Widget") map.TryAdd(annot, i + 1);
        }
        return map;
    }

    private static void Walk(
        PdfDocument pdf, PdfObject? node, PdfDict? parent, string parentName, Inherited inherited,
        Dictionary<PdfDict, int> widgetPages, HashSet<PdfDict> visited, List<PdfFormField> result, int depth)
    {
        if (depth > MaxDepth || result.Count >= MaxFields) return;
        var dict = pdf.Resolve(node).AsDict();
        if (dict is null || !visited.Add(dict)) return;

        string partial = pdf.Resolve(dict.Get("T")).AsStringBytes() is { } t ? PdfFormFields.DecodeTextString(t) : "";
        string full = parentName.Length == 0 ? partial : partial.Length == 0 ? parentName : parentName + "." + partial;

        var here = new Inherited(
            dict.Get("FT") ?? inherited.Ft, dict.Get("Ff") ?? inherited.Ff, dict.Get("V") ?? inherited.V,
            dict.Get("DV") ?? inherited.Dv, dict.Get("Opt") ?? inherited.Opt);

        // A kid with its own /T is a sub-field; one without is only a widget of this field.
        var kids = pdf.Resolve(dict.Get("Kids")).AsArray();
        var subFields = new List<PdfObject>();
        var widgets = new List<PdfDict>();
        if (kids is not null)
            foreach (var kid in kids.Items)
            {
                var kd = pdf.Resolve(kid).AsDict();
                if (kd is null) continue;
                if (kd.Has("T") || (kd.Has("Kids") && !kd.Has("Rect"))) subFields.Add(kid);
                else widgets.Add(kd);
            }
        else if (dict.Has("Rect")) widgets.Add(dict);

        foreach (var sub in subFields)
            Walk(pdf, sub, dict, full, here, widgetPages, visited, result, depth + 1);

        // Only a field that is a leaf of the name hierarchy is reported; a pure grouping node
        // carries a name but no type of its own to describe.
        if (subFields.Count > 0) return;
        if (here.Ft is null && partial.Length == 0) return;
        result.Add(Describe(pdf, full, here, widgets, widgetPages));
    }

    private static PdfFormField Describe(
        PdfDocument pdf, string name, Inherited a, List<PdfDict> widgets, Dictionary<PdfDict, int> widgetPages)
    {
        string? ft = pdf.Resolve(a.Ft).AsName();
        uint flags = pdf.Resolve(a.Ff).AsLong() is { } f ? (uint)f : 0;
        var field = new PdfFormField
        {
            Name = name,
            Flags = flags,
            ReadOnly = (flags & ReadOnlyBit) != 0,
            Required = (flags & RequiredBit) != 0,
        };

        if (widgets.Count > 0)
        {
            var first = widgets[0];
            if (widgetPages.TryGetValue(first, out int page)) field.PageNumber = (uint)page;
            if (Numbers(pdf, first.Get("Rect")) is { Count: >= 4 } r)
                field.BoundingBox = new BoundingBox
                {
                    X0 = Math.Min(r[0], r[2]), Y0 = Math.Min(r[1], r[3]), X1 = Math.Max(r[0], r[2]), Y1 = Math.Max(r[1], r[3]),
                };
        }

        var value = pdf.Resolve(a.V);
        field.DefaultValue = Scalar(pdf, a.Dv);

        switch (ft)
        {
            case "Tx":
                field.FieldType = "text";
                field.Value = value switch
                {
                    PdfString s => PdfFormFields.DecodeTextString(s.Bytes),
                    PdfStream st => PdfFormFields.DecodeTextString(pdf.DecodeStream(st)),
                    _ => null,
                };
                break;

            case "Btn" when (flags & PushButtonBit) != 0:
                field.FieldType = "push_button";
                break;

            case "Btn" when (flags & RadioBit) != 0:
                field.FieldType = "radio";
                field.Options = widgets.SelectMany(w => ButtonStates(pdf, w)).Distinct()
                    .Select(s => new PdfFormOption { Export = s, Display = s }).ToList();
                string? chosen = value is PdfName vn && vn.Value != "Off" ? vn.Value : null;
                // A radio group states its choice in /V, or only in its widgets' /AS.
                chosen ??= widgets.Select(w => pdf.Resolve(w.Get("AS")).AsName()).FirstOrDefault(s => s is not null and not "Off");
                field.Value = chosen;
                if (chosen is not null) field.Values = new List<string> { chosen };
                break;

            case "Btn":
                field.FieldType = "checkbox";
                string? state = value is PdfName cn ? cn.Value
                    : widgets.Select(w => pdf.Resolve(w.Get("AS")).AsName()).FirstOrDefault(s => s is not null);
                field.Checked = state is not null && state != "Off";
                field.Value = field.Checked == true ? "true" : "false";
                if (state is not null) field.Values = new List<string> { state };
                break;

            case "Ch":
                field.FieldType = "choice";
                field.IsCombo = (flags & ComboBit) != 0;
                field.MultiSelect = (flags & MultiSelectBit) != 0;
                field.Options = ChoiceOptions(pdf, a.Opt);
                var exports = new List<string>();
                switch (value)
                {
                    case PdfString s: exports.Add(PdfFormFields.DecodeTextString(s.Bytes)); break;
                    case PdfName n: exports.Add(n.Value); break;
                    case PdfArray arr:
                        foreach (var item in arr.Items)
                            if (pdf.Resolve(item) is PdfString si) exports.Add(PdfFormFields.DecodeTextString(si.Bytes));
                        break;
                }
                if (exports.Count > 0)
                {
                    field.Values = exports;
                    field.Value = string.Join(", ", exports.Select(e =>
                        field.Options.FirstOrDefault(o => o.Export == e)?.Display ?? e));
                }
                break;

            case "Sig":
                field.FieldType = "signature";
                field.Value = value is PdfDict or PdfStream ? "signed" : null;
                break;

            default:
                field.FieldType = "unknown";
                field.Value = Scalar(pdf, a.V);
                break;
        }
        return field;
    }

    private static string? Scalar(PdfDocument pdf, PdfObject? o) => pdf.Resolve(o) switch
    {
        PdfString s => PdfFormFields.DecodeTextString(s.Bytes),
        PdfName n => n.Value,
        PdfArray arr => string.Join(", ", arr.Items.Select(i => pdf.Resolve(i) switch
        {
            PdfString s => PdfFormFields.DecodeTextString(s.Bytes),
            PdfName n => n.Value,
            _ => "",
        }).Where(x => x.Length > 0)) is { Length: > 0 } joined ? joined : null,
        _ => null,
    };

    /// <summary>The on-states of a button widget: the keys of its normal appearance, minus Off.</summary>
    private static IEnumerable<string> ButtonStates(PdfDocument pdf, PdfDict widget)
    {
        var ap = pdf.Resolve(widget.Get("AP")).AsDict();
        var normal = pdf.Resolve(ap?.Get("N")).AsDict();
        if (normal is null) yield break;
        foreach (var key in normal.Map.Keys)
            if (key != "Off") yield return key;
    }

    private static List<PdfFormOption> ChoiceOptions(PdfDocument pdf, PdfObject? opt)
    {
        var options = new List<PdfFormOption>();
        if (pdf.Resolve(opt).AsArray() is not { } array) return options;
        foreach (var item in array.Items)
        {
            switch (pdf.Resolve(item))
            {
                case PdfString s:
                {
                    string text = PdfFormFields.DecodeTextString(s.Bytes);
                    options.Add(new PdfFormOption { Export = text, Display = text });
                    break;
                }
                case PdfArray pair when pair.Items.Count >= 2:
                {
                    string export = Scalar(pdf, pair.Items[0]) ?? "";
                    options.Add(new PdfFormOption { Export = export, Display = Scalar(pdf, pair.Items[1]) ?? export });
                    break;
                }
            }
            if (options.Count >= 10_000) break;
        }
        return options;
    }

    private static List<double>? Numbers(PdfDocument pdf, PdfObject? array)
    {
        if (pdf.Resolve(array).AsArray() is not { } a) return null;
        var list = new List<double>(a.Items.Count);
        foreach (var item in a.Items)
            if (pdf.Resolve(item).AsNumber() is { } n) list.Add(n);
        return list;
    }
}
