using SixLabors.ImageSharp;
using SixLabors.ImageSharp.PixelFormats;
using SixLabors.ImageSharp.Processing;

namespace XRay.Content.Internal.Qr;

/// <summary>One decoded QR code: its payload and where it sat in the image.</summary>
internal readonly record struct QrResult(string Payload, int X, int Y, int Width, int Height);

/// <summary>
/// Find and decode the QR codes in an image, ported from rqrr 0.10.1.
/// </summary>
/// <remarks>
/// Tolerant throughout: malformed bytes, an image that decodes to nothing, an undetected grid and
/// a grid that fails its error correction all yield nothing rather than an error. The caller
/// distinguishes "ran and found none" from "did not run".
/// </remarks>
internal static class QrScanner
{
    /// <summary>Decode every QR code in the given image bytes.</summary>
    public static List<QrResult> Detect(ReadOnlySpan<byte> imageBytes)
    {
        if (imageBytes.Length == 0) return new List<QrResult>();

        byte[] grey;
        int width, height;
        try
        {
            using var image = Image.Load<L8>(imageBytes);
            width = image.Width;
            height = image.Height;
            grey = new byte[width * height];
            image.CopyPixelDataTo(grey);
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            // An undecodable image is not an error here; the post-processor reports "found none".
            return new List<QrResult>();
        }

        return DetectFromGreyscale(grey, width, height);
    }

    /// <summary>Decode every QR code in an 8-bit greyscale buffer.</summary>
    public static List<QrResult> DetectFromGreyscale(byte[] grey, int width, int height)
    {
        var results = new List<QrResult>();
        if (width <= 0 || height <= 0) return results;

        var prepared = QrPreparedImage.Prepare(grey, width, height);
        foreach (var (grid, bounds) in DetectGrids(prepared))
        {
            byte[]? payload = QrDecoder.Decode(grid);
            if (payload is null) continue;

            // Upstream decodes lossily rather than rejecting a payload that is not UTF-8 — a QR
            // code can legitimately carry Shift-JIS or raw bytes.
            string text = System.Text.Encoding.UTF8.GetString(payload);

            int minX = bounds.Min(p => p.X), maxX = bounds.Max(p => p.X);
            int minY = bounds.Min(p => p.Y), maxY = bounds.Max(p => p.Y);
            results.Add(new QrResult(
                text,
                Math.Max(minX, 0),
                Math.Max(minY, 0),
                Math.Max(maxX - minX, 0),
                Math.Max(maxY - minY, 0)));
        }

        return results;
    }

    /// <summary>
    /// Group the capstones into grids that look like codes, and locate each one.
    /// </summary>
    /// <remarks>
    /// A candidate grouping is tried against a <em>copy</em> of the image, because locating a grid
    /// mutates it — claiming an alignment pattern, recolouring regions — and a wrong guess would
    /// otherwise poison the search for the right one.
    /// </remarks>
    private static List<(RefGridImage Grid, QrPoint[] Bounds)> DetectGrids(QrPreparedImage img)
    {
        var result = new List<(RefGridImage, QrPoint[])>();
        var stones = QrDetector.CapstonesFromImage(img);
        var groups = FindGroupings(img, stones);

        foreach (var group in groups)
        {
            var location = SkewedGridLocation.FromGroup(img, group);
            if (location is null) continue;

            double n = location.GridSize + 1.0;
            var bounds = new[]
            {
                location.C.Map(0.0, 0.0),
                location.C.Map(n, 0.0),
                location.C.Map(n, n),
                location.C.Map(0.0, n),
            };
            result.Add((new RefGridImage(location, img), bounds));
        }

        return result;
    }

    private static List<CapStoneGroup> FindGroupings(QrPreparedImage img, List<CapStone> capstones)
    {
        var used = new HashSet<int>();
        var groups = new List<CapStoneGroup>();

        for (int idx = 0; idx < capstones.Count; idx++)
        {
            Cancel.ThrowIfRequested();
            if (used.Contains(idx)) continue;

            foreach (var (h, v) in QrDetector.FindAndRankPossibleNeighbors(capstones, idx))
            {
                if (used.Contains(h) || used.Contains(v)) continue;

                var candidate = new CapStoneGroup(
                    capstones[h].Clone(), capstones[idx].Clone(), capstones[v].Clone());

                // Confirm on a copy, so a rejected grouping leaves the real image untouched.
                var imageCopy = img.Clone();
                var testGroup = new CapStoneGroup(
                    candidate.A.Clone(), candidate.B.Clone(), candidate.C.Clone());
                if (SkewedGridLocation.FromGroup(imageCopy, testGroup) is null) continue;

                groups.Add(candidate);
                used.Add(h);
                used.Add(idx);
                used.Add(v);
            }
        }

        return groups;
    }
}
