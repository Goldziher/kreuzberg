// The bridge between the ported pdf_oxide code and the port's existing PDF object
// layer (`XRay.Content.Internal.Pdf`: xref, object streams, filters, decryption).
//
// pdf_oxide has its own `Object`/`PdfDocument`, but that half of it — file parsing —
// is not where the extraction divergence lives, and the port already has a working
// equivalent. These helpers give the ported modules the same shape of accessor the
// Rust reads with (`obj.as_dict()`, `doc.load_object(r)`), so a port stays readable
// against its source.
using System;
using System.Collections.Generic;
using XRay.Content.Internal.Pdf;

namespace XRay.Content.Internal.PdfOxide;

internal static class Ox
{
    public static PdfObject? Resolve(PdfDocument? doc, PdfObject? o) =>
        doc is null ? o : doc.Resolve(o);

    public static PdfDict? Dict(PdfDocument? doc, PdfObject? o) => Resolve(doc, o).AsDict();

    public static PdfArray? Arr(PdfDocument? doc, PdfObject? o) => Resolve(doc, o).AsArray();

    public static string? Name(PdfDocument? doc, PdfObject? o) => Resolve(doc, o).AsName();

    public static byte[]? Str(PdfDocument? doc, PdfObject? o) => Resolve(doc, o).AsStringBytes();

    public static long? Int(PdfDocument? doc, PdfObject? o) => Resolve(doc, o).AsLong();

    public static float? Num(PdfDocument? doc, PdfObject? o)
    {
        double? v = Resolve(doc, o).AsNumber();
        return v is null ? null : (float)v.Value;
    }

    /// <summary>Entry <paramref name="key"/> of <paramref name="d"/>, resolved.</summary>
    public static PdfObject? Get(PdfDocument? doc, PdfDict? d, string key) =>
        d is null ? null : Resolve(doc, d.Get(key));

    public static PdfDict? GetDict(PdfDocument? doc, PdfDict? d, string key) => Get(doc, d, key).AsDict();
    public static PdfArray? GetArr(PdfDocument? doc, PdfDict? d, string key) => Get(doc, d, key).AsArray();
    public static string? GetName(PdfDocument? doc, PdfDict? d, string key) => Get(doc, d, key).AsName();
    public static long? GetInt(PdfDocument? doc, PdfDict? d, string key) => Get(doc, d, key).AsLong();

    public static float? GetNum(PdfDocument? doc, PdfDict? d, string key)
    {
        double? v = Get(doc, d, key).AsNumber();
        return v is null ? null : (float)v.Value;
    }

    /// <summary>Decoded bytes of a stream, or null when the object is not one.</summary>
    public static byte[]? StreamData(PdfDocument? doc, PdfObject? o)
    {
        if (doc is null) return null;
        if (doc.Resolve(o) is not PdfStream st) return null;
        try { return doc.DecodeStream(st); }
        catch (Exception) when (Cancel.NotCancelling) { return null; }
    }

    /// <summary>Six numbers as a matrix, for /Matrix and /FontMatrix entries.</summary>
    public static OxMatrix? Matrix6(PdfDocument? doc, PdfObject? o)
    {
        var a = Arr(doc, o);
        if (a is null || a.Items.Count < 6) return null;
        var v = new float[6];
        for (int i = 0; i < 6; i++)
        {
            float? n = Num(doc, a.Items[i]);
            if (n is null) return null;
            v[i] = n.Value;
        }
        return new OxMatrix(v[0], v[1], v[2], v[3], v[4], v[5]);
    }

    /// <summary>
    /// The identity of an indirect reference, for the visited-set keys the extractor
    /// uses to stop Form XObject recursion.
    /// </summary>
    public static (int Number, int Generation)? RefOf(PdfObject? o) =>
        o is PdfRef r ? (r.Number, r.Generation) : null;
}
