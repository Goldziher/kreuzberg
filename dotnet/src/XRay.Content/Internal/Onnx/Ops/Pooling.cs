using System.Numerics.Tensors;

namespace XRay.Content.Internal.Onnx.Ops;

/// <summary>2-D pooling: max, average, and the global reductions.</summary>
internal static class Pooling
{
    public static Tensor GlobalAveragePool(Tensor x)
    {
        var fx = x.AsFloat();
        int batch = fx.Shape[0], channels = fx.Shape[1];
        int spatial = 1;
        for (int i = 2; i < fx.Rank; i++) spatial *= fx.Shape[i];

        var shape = new int[fx.Rank];
        shape[0] = batch;
        shape[1] = channels;
        for (int i = 2; i < fx.Rank; i++) shape[i] = 1;

        var result = Tensor.AllocateFloat(shape);
        for (int p = 0; p < batch * channels; p++)
            result.Floats[p] = TensorPrimitives.Sum(fx.Floats.AsSpan(p * spatial, spatial)) / spatial;
        return result;
    }

    public static Tensor MaxPool(Tensor x, long[]? kernel, long[]? strides, long[]? pads, long[]? dilations, string autoPad, bool ceilMode)
        => Pool(x, kernel, strides, pads, dilations, autoPad, ceilMode, countIncludePad: false, isMax: true);

    public static Tensor AveragePool(Tensor x, long[]? kernel, long[]? strides, long[]? pads, string autoPad, bool ceilMode, bool countIncludePad)
        => Pool(x, kernel, strides, pads, null, autoPad, ceilMode, countIncludePad, isMax: false);

    /// <summary>
    /// The shared pooling walk.
    /// <para>
    /// The subtlety is <c>count_include_pad</c> for average pooling: with it off — the ONNX
    /// default — a window overlapping the padding divides by the number of <em>real</em>
    /// elements it saw, not by the window area, so edge outputs are not silently dimmed.
    /// </para>
    /// </summary>
    private static Tensor Pool(
        Tensor x, long[]? kernel, long[]? strides, long[]? pads, long[]? dilations,
        string autoPad, bool ceilMode, bool countIncludePad, bool isMax)
    {
        var fx = x.AsFloat();
        if (fx.Rank != 4) throw new NotSupportedException("pool: only 2-D pooling is supported");

        int batch = fx.Shape[0], channels = fx.Shape[1], height = fx.Shape[2], width = fx.Shape[3];
        int kh = (int)(kernel is { Length: >= 2 } ? kernel[0] : 1);
        int kw = (int)(kernel is { Length: >= 2 } ? kernel[1] : 1);
        int strideH = (int)(strides is { Length: >= 2 } ? strides[0] : 1);
        int strideW = (int)(strides is { Length: >= 2 } ? strides[1] : 1);
        int dilationH = (int)(dilations is { Length: >= 2 } ? dilations[0] : 1);
        int dilationW = (int)(dilations is { Length: >= 2 } ? dilations[1] : 1);

        int padTop = 0, padLeft = 0, padBottom = 0, padRight = 0;
        int effKh = (kh - 1) * dilationH + 1;
        int effKw = (kw - 1) * dilationW + 1;

        if (autoPad is "SAME_UPPER" or "SAME_LOWER")
        {
            int outHSame = (height + strideH - 1) / strideH;
            int outWSame = (width + strideW - 1) / strideW;
            int totalH = Math.Max((outHSame - 1) * strideH + effKh - height, 0);
            int totalW = Math.Max((outWSame - 1) * strideW + effKw - width, 0);
            bool upper = autoPad == "SAME_UPPER";
            padTop = upper ? totalH / 2 : totalH - totalH / 2;
            padLeft = upper ? totalW / 2 : totalW - totalW / 2;
            padBottom = totalH - padTop;
            padRight = totalW - padLeft;
        }
        else if (pads is { Length: >= 4 })
        {
            padTop = (int)pads[0];
            padLeft = (int)pads[1];
            padBottom = (int)pads[2];
            padRight = (int)pads[3];
        }

        int spanH = height + padTop + padBottom - effKh;
        int spanW = width + padLeft + padRight - effKw;
        int outH = ceilMode ? CeilDiv(spanH, strideH) + 1 : spanH / strideH + 1;
        int outW = ceilMode ? CeilDiv(spanW, strideW) + 1 : spanW / strideW + 1;

        // Ceil mode may push a window to start entirely inside the padding; ONNX drops those.
        if (ceilMode)
        {
            if ((outH - 1) * strideH >= height + padTop) outH--;
            if ((outW - 1) * strideW >= width + padLeft) outW--;
        }

        var result = Tensor.AllocateFloat(batch, channels, outH, outW);
        int planeIn = height * width;
        int planeOut = outH * outW;

        // Max pooling needs none of the machinery below — no count of real elements, no
        // division — so it gets its own walk. The condition on the padding is what guarantees
        // every output window sees at least one real element, which is what lets that walk
        // start each row at negative infinity and never look at a count.
        bool separableMax = isMax
            && padTop < effKh && padBottom < effKh && padLeft < effKw && padRight < effKw;

        Parallel.For(0, batch * channels, Cancel.ParallelOptions, plane =>
        {
            var src = fx.Floats.AsSpan(plane * planeIn, planeIn);
            var dst = result.Floats.AsSpan(plane * planeOut, planeOut);

            if (separableMax)
            {
                MaxPlane(src, dst, height, width, outH, outW,
                    kh, kw, strideH, strideW, dilationH, dilationW, padTop, padLeft);
                return;
            }

            for (int oy = 0; oy < outH; oy++)
            {
                for (int ox = 0; ox < outW; ox++)
                {
                    float accumulator = isMax ? float.NegativeInfinity : 0f;
                    int seen = 0;
                    for (int ky = 0; ky < kh; ky++)
                    {
                        int iy = oy * strideH - padTop + ky * dilationH;
                        if ((uint)iy >= (uint)height) continue;
                        for (int kx = 0; kx < kw; kx++)
                        {
                            int ix = ox * strideW - padLeft + kx * dilationW;
                            if ((uint)ix >= (uint)width) continue;
                            float v = src[iy * width + ix];
                            accumulator = isMax ? MathF.Max(accumulator, v) : accumulator + v;
                            seen++;
                        }
                    }
                    dst[oy * outW + ox] = isMax
                        ? (seen == 0 ? 0f : accumulator)
                        : accumulator / (countIncludePad ? kh * kw : Math.Max(seen, 1));
                }
            }
        });
        return result;
    }

    /// <summary>
    /// One plane of max pooling, walked kernel tap by kernel tap.
    /// <para>
    /// Taking the kernel on the outside means each pass is a running maximum of one contiguous
    /// source row against a run of the output row, so at unit horizontal stride the work
    /// vectorises directly. The alternative — finishing one output pixel at a time, as the
    /// shared walk does — re-reads the same source rows once per pixel, tests bounds per
    /// element, and has nothing wider than a scalar to do.
    /// </para>
    /// <para>
    /// Padding costs nothing here beyond arithmetic: a tap that falls outside the plane is
    /// skipped for a whole row, and one that falls outside horizontally simply clips the run
    /// of output columns it applies to.
    /// </para>
    /// </summary>
    private static void MaxPlane(
        ReadOnlySpan<float> src, Span<float> dst, int height, int width, int outH, int outW,
        int kh, int kw, int strideH, int strideW, int dilationH, int dilationW, int padTop, int padLeft)
    {
        for (int oy = 0; oy < outH; oy++)
        {
            var row = dst.Slice(oy * outW, outW);
            row.Fill(float.NegativeInfinity);

            for (int ky = 0; ky < kh; ky++)
            {
                int iy = oy * strideH - padTop + ky * dilationH;
                if ((uint)iy >= (uint)height) continue;
                int sourceRow = iy * width;

                for (int kx = 0; kx < kw; kx++)
                {
                    // Output columns whose source column for this tap lands inside the row.
                    int shift = kx * dilationW - padLeft;
                    if (width - 1 - shift < 0) continue;
                    int first = shift >= 0 ? 0 : (-shift + strideW - 1) / strideW;
                    int last = Math.Min((width - 1 - shift) / strideW, outW - 1);
                    if (first > last) continue;

                    int run = last - first + 1;
                    var target = row.Slice(first, run);

                    if (strideW == 1)
                    {
                        TensorPrimitives.Max(target, src.Slice(sourceRow + shift + first, run), target);
                        continue;
                    }
                    for (int ox = first; ox <= last; ox++)
                        row[ox] = MathF.Max(row[ox], src[sourceRow + shift + ox * strideW]);
                }
            }
        }
    }

    private static int CeilDiv(int a, int b) => b == 0 ? 0 : (a + b - 1) / b;
}
