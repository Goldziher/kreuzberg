using System.Buffers;
using System.Numerics.Tensors;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

namespace XRay.Content.Internal.Onnx.Ops;

/// <summary>
/// 2-D convolution and batch normalisation.
/// <para>
/// Convolution is lowered to matrix multiplication via <em>im2col</em>: each output pixel's
/// receptive field is unrolled into a column, turning the whole layer into one
/// <c>[M, C·kH·kW] x [C·kH·kW, outH·outW]</c> product that reuses the tuned kernel in
/// <see cref="Linear"/>. Two shapes bypass that entirely because they dominate these
/// backbones and the unrolled buffer would be pure overhead: a 1x1 stride-1 convolution is
/// already a matrix multiply against the input, and a depthwise convolution touches one
/// channel at a time, where im2col would inflate memory traffic by the kernel area for no
/// arithmetic gain.
/// </para>
/// </summary>
internal static class Convolution
{
    /// <summary>Padding as ONNX resolves it: explicit values, or derived from an auto_pad mode.</summary>
    private readonly record struct Padding(int Top, int Left, int Bottom, int Right);

    public static Tensor Conv(
        Tensor x, Tensor weight, Tensor? bias,
        long[]? strides, long[]? pads, long[]? dilations, long group, string autoPad,
        FusedActivation activation = FusedActivation.None)
    {
        var fx = x.AsFloat();
        var fw = weight.AsFloat();
        if (fx.Rank != 4 || fw.Rank != 4)
            throw new NotSupportedException("conv: only 2-D convolution is supported");

        int batch = fx.Shape[0], channels = fx.Shape[1], height = fx.Shape[2], width = fx.Shape[3];
        int filters = fw.Shape[0], inPerGroup = fw.Shape[1], kh = fw.Shape[2], kw = fw.Shape[3];
        int g = (int)Math.Max(group, 1);

        int strideH = (int)(strides is { Length: >= 2 } ? strides[0] : 1);
        int strideW = (int)(strides is { Length: >= 2 } ? strides[1] : 1);
        int dilationH = (int)(dilations is { Length: >= 2 } ? dilations[0] : 1);
        int dilationW = (int)(dilations is { Length: >= 2 } ? dilations[1] : 1);

        var pad = ResolvePadding(pads, autoPad, height, width, kh, kw, strideH, strideW, dilationH, dilationW);

        int effKh = (kh - 1) * dilationH + 1;
        int effKw = (kw - 1) * dilationW + 1;
        int outH = (height + pad.Top + pad.Bottom - effKh) / strideH + 1;
        int outW = (width + pad.Left + pad.Right - effKw) / strideW + 1;
        if (outH <= 0 || outW <= 0)
            throw new InvalidDataException($"conv: degenerate output size {outH}x{outW}");

        var result = Tensor.AllocateFloat(batch, filters, outH, outW);

        bool depthwise = g == channels && inPerGroup == 1 && filters % g == 0;
        bool pointwise = kh == 1 && kw == 1 && strideH == 1 && strideW == 1
                         && pad is { Top: 0, Left: 0, Bottom: 0, Right: 0 }
                         && dilationH == 1 && dilationW == 1;

        if (depthwise)
            ConvDepthwise(fx, fw, result, batch, channels, height, width, outH, outW,
                kh, kw, strideH, strideW, dilationH, dilationW, pad, filters / g);
        else if (pointwise)
            ConvPointwise(fx, fw, result, batch, channels, height * width, filters, g);
        else
            ConvImplicitGemm(fx, fw, result, batch, channels, height, width, outH, outW,
                kh, kw, strideH, strideW, dilationH, dilationW, pad, filters, inPerGroup, g);

        ApplyBiasAndActivation(result, bias?.AsFloat(), activation, batch, filters, outH * outW);
        return result;
    }

    /// <summary>
    /// Explicit pads win; otherwise <c>SAME_UPPER</c>/<c>SAME_LOWER</c> derive the total
    /// padding that keeps the output size equal to <c>ceil(input / stride)</c>, splitting an
    /// odd remainder toward the bottom-right or top-left respectively.
    /// </summary>
    private static Padding ResolvePadding(
        long[]? pads, string autoPad, int height, int width, int kh, int kw,
        int strideH, int strideW, int dilationH, int dilationW)
    {
        if (autoPad is "SAME_UPPER" or "SAME_LOWER")
        {
            int effKh = (kh - 1) * dilationH + 1;
            int effKw = (kw - 1) * dilationW + 1;
            int outH = (height + strideH - 1) / strideH;
            int outW = (width + strideW - 1) / strideW;
            int totalH = Math.Max((outH - 1) * strideH + effKh - height, 0);
            int totalW = Math.Max((outW - 1) * strideW + effKw - width, 0);
            bool upper = autoPad == "SAME_UPPER";
            int top = upper ? totalH / 2 : totalH - totalH / 2;
            int left = upper ? totalW / 2 : totalW - totalW / 2;
            return new Padding(top, left, totalH - top, totalW - left);
        }
        if (pads is { Length: >= 4 })
            return new Padding((int)pads[0], (int)pads[1], (int)pads[2], (int)pads[3]);
        return new Padding(0, 0, 0, 0);
    }

    /// <summary>Pointwise (1x1) convolution: one matrix multiply per batch item and group.</summary>
    private static void ConvPointwise(
        Tensor x, Tensor w, Tensor result, int batch, int channels, int spatial, int filters, int groups)
    {
        int inPerGroup = channels / groups;
        int outPerGroup = filters / groups;
        for (int n = 0; n < batch; n++)
        {
            for (int g = 0; g < groups; g++)
            {
                var weights = w.Floats.AsMemory(g * outPerGroup * inPerGroup, outPerGroup * inPerGroup);
                var input = x.Floats.AsMemory((n * channels + g * inPerGroup) * spatial, inPerGroup * spatial);
                var output = result.Floats.AsMemory((n * filters + g * outPerGroup) * spatial, outPerGroup * spatial);
                Linear.MultiplyInto(weights, input, output, outPerGroup, inPerGroup, spatial);
            }
        }
    }

    /// <summary>
    /// Depthwise convolution: each output channel reads exactly one input channel, so the
    /// whole layer is a stack of independent 2-D correlations with no channel reduction.
    /// The inner loop runs along a row, which keeps the input and output spans contiguous.
    /// </summary>
    private static void ConvDepthwise(
        Tensor x, Tensor w, Tensor result, int batch, int channels, int height, int width,
        int outH, int outW, int kh, int kw, int strideH, int strideW,
        int dilationH, int dilationW, Padding pad, int multiplier)
    {
        int planeIn = height * width;
        int planeOut = outH * outW;
        int filters = channels * multiplier;

        Parallel.For(0, batch * filters, Cancel.ParallelOptions, plane =>
        {
            int n = plane / filters;
            int f = plane % filters;
            int c = f / multiplier;
            var src = x.Floats.AsSpan((n * channels + c) * planeIn, planeIn);
            var dst = result.Floats.AsSpan((n * filters + f) * planeOut, planeOut);
            var kernel = w.Floats.AsSpan(f * kh * kw, kh * kw);
            dst.Clear();

            for (int ky = 0; ky < kh; ky++)
            {
                for (int kx = 0; kx < kw; kx++)
                {
                    float weight = kernel[ky * kw + kx];
                    if (weight == 0f) continue;
                    for (int oy = 0; oy < outH; oy++)
                    {
                        int iy = oy * strideH - pad.Top + ky * dilationH;
                        if ((uint)iy >= (uint)height) continue;
                        int rowIn = iy * width;
                        int rowOut = oy * outW;
                        for (int ox = 0; ox < outW; ox++)
                        {
                            int ix = ox * strideW - pad.Left + kx * dilationW;
                            if ((uint)ix >= (uint)width) continue;
                            dst[rowOut + ox] += weight * src[rowIn + ix];
                        }
                    }
                }
            }
        });
    }

    /// <summary>
    /// General convolution, as an implicit matrix multiply.
    /// <para>
    /// The receptive fields are never materialised. The multiply asks for its right-hand
    /// operand one packed panel at a time, and the packer gathers those columns straight out
    /// of the input image — so the expanded data, which for a 3x3 layer is nine times the
    /// input, is written once into a cache-resident panel instead of being spilled to a
    /// multi-megabyte buffer and read back.
    /// </para>
    /// </summary>
    private static void ConvImplicitGemm(
        Tensor x, Tensor w, Tensor result, int batch, int channels, int height, int width,
        int outH, int outW, int kh, int kw, int strideH, int strideW,
        int dilationH, int dilationW, Padding pad, int filters, int inPerGroup, int groups)
    {
        int outPerGroup = filters / groups;
        int patch = inPerGroup * kh * kw;
        int spatial = outH * outW;

        for (int n = 0; n < batch; n++)
        {
            for (int g = 0; g < groups; g++)
            {
                int planeBase = (n * channels + g * inPerGroup) * height * width;
                var weights = w.Floats.AsMemory(g * outPerGroup * patch, outPerGroup * patch);
                var output = result.Floats.AsMemory((n * filters + g * outPerGroup) * spatial, outPerGroup * spatial);
                var source = x.Floats;

                GemmKernel.Multiply(
                    weights,
                    (destination, offset, pc, countK, jc, countN) => PackReceptiveFields(
                        destination, offset, source, planeBase, height, width, outW,
                        kh, kw, strideH, strideW, dilationH, dilationW, pad,
                        pc, countK, jc, countN),
                    output, outPerGroup, patch, spatial, spatial,
                    parallel: true, ParallelThreshold);
            }
        }
    }

    /// <summary>Multiply-accumulates above which convolution spreads across cores.</summary>
    private const long ParallelThreshold = 1L << 16;

    /// <summary>
    /// Fill one packed panel directly from the image: for each kernel tap in
    /// <c>[pc, pc+countK)</c> and each block of sixteen output pixels in
    /// <c>[jc, jc+countN)</c>, gather the sixteen input values that tap contributes.
    /// <para>
    /// The arithmetic is arranged so the inner loop has none. Decomposing a flat tap index
    /// into channel and kernel offset costs three integer divisions, and doing that per block
    /// per tap dominated the gather; it is now computed once per panel into a small table.
    /// The output pixel's row and column likewise cost one division per block rather than one
    /// per tap.
    /// </para>
    /// <para>
    /// What is left is a stream of sixteen-float copies, and at that size the copy itself is
    /// the cost: sixteen floats are exactly one cache line, so a general-purpose
    /// <see cref="Span{T}.CopyTo"/> spends most of its time in call and length-dispatch
    /// overhead rather than moving data. The interior of the panel — every block that stays
    /// within one output row, reads contiguously, and has no tap leaving the image — therefore
    /// runs a dedicated loop whose body is one vector load and one vector store, with the
    /// edge conditions hoisted to a single test per block.
    /// </para>
    /// </summary>
    private static void PackReceptiveFields(
        float[] destination, int offset, float[] source, int planeBase,
        int height, int width, int outW,
        int kh, int kw, int strideH, int strideW, int dilationH, int dilationW, Padding pad,
        int pc, int countK, int jc, int countN)
    {
        const int BlockWidth = 16;
        int blocks = (countN + BlockWidth - 1) / BlockWidth;
        int plane = height * width;

        // One entry per tap in this depth slice: the input plane it reads and the vertical and
        // horizontal offsets it applies.
        Span<int> tapPlane = countK <= 256 ? stackalloc int[countK] : new int[countK];
        Span<int> tapRow = countK <= 256 ? stackalloc int[countK] : new int[countK];
        Span<int> tapColumn = countK <= 256 ? stackalloc int[countK] : new int[countK];

        // The extreme offsets over this depth slice decide, in one test per block, whether any
        // tap can leave the image — so the interior loop needs no bounds check at all.
        int minRow = int.MaxValue, maxRow = int.MinValue;
        int minColumn = int.MaxValue, maxColumn = int.MinValue;

        for (int kk = 0; kk < countK; kk++)
        {
            int tap = pc + kk;
            int kx = tap % kw;
            int ky = tap / kw % kh;
            int c = tap / (kw * kh);
            int row = ky * dilationH - pad.Top;
            int column = kx * dilationW - pad.Left;
            tapPlane[kk] = planeBase + c * plane;
            tapRow[kk] = row;
            tapColumn[kk] = column;
            if (row < minRow) minRow = row;
            if (row > maxRow) maxRow = row;
            if (column < minColumn) minColumn = column;
            if (column > maxColumn) maxColumn = column;
        }

        ref float sourceRef = ref MemoryMarshal.GetArrayDataReference(source);
        ref float destinationRef = ref MemoryMarshal.GetArrayDataReference(destination);
        int blockStride = countK * BlockWidth;

        // Classify the blocks once. Interior means: a whole block, staying inside one output
        // row, reading contiguously, with no tap in this depth slice leaving the image — so
        // the gather for it is one aligned vector move and needs no test of its own.
        Span<int> interiorBase = blocks <= 128 ? stackalloc int[blocks] : new int[blocks];
        bool anyInterior = false, anyEdge = false;
        for (int block = 0; block < blocks; block++)
        {
            int firstPixel = jc + block * BlockWidth;
            int pixels = Math.Min(BlockWidth, jc + countN - firstPixel);
            int oy0 = firstPixel / outW;
            int ox0 = firstPixel - oy0 * outW;
            int iy0 = oy0 * strideH;

            int ix0 = ox0 * strideW;
            bool interior = pixels == BlockWidth && ox0 + BlockWidth <= outW
                && iy0 + minRow >= 0 && iy0 + maxRow < height
                && ix0 + minColumn >= 0 && (ox0 + BlockWidth - 1) * strideW + maxColumn < width;

            interiorBase[block] = interior ? iy0 * width + ix0 : -1;
            anyInterior |= interior;
            anyEdge |= !interior;
        }

        // The interior runs depth-major even though the destination is laid out block-major.
        // Block-major would read each tap's source row once per block — eight scattered
        // sixteen-float reads across a row stride that for a 160x160 feature map is 100 KB, so
        // the same pages are revisited eight times and nothing prefetches. Depth-major reads
        // each tap's row once, straight through, and scatters only the writes.
        if (anyInterior)
        {
            for (int kk = 0; kk < countK; kk++)
            {
                int tapOffset = tapPlane[kk] + tapRow[kk] * width + tapColumn[kk];
                int to = offset + kk * BlockWidth;
                for (int block = 0; block < blocks; block++)
                {
                    int rowBase = interiorBase[block];
                    if (rowBase < 0) continue;
                    GemmKernel.GatherBlock(
                        ref sourceRef, source.Length, rowBase + tapOffset, strideW,
                        ref destinationRef, to + block * blockStride);
                }
            }
        }

        if (!anyEdge) return;

        for (int block = 0; block < blocks; block++)
        {
            if (interiorBase[block] >= 0) continue;

            int firstPixel = jc + block * BlockWidth;
            int pixels = Math.Min(BlockWidth, jc + countN - firstPixel);
            int blockBase = offset + block * countK * BlockWidth;

            int oy0 = firstPixel / outW;
            int ox0 = firstPixel - oy0 * outW;
            bool singleRow = ox0 + pixels <= outW;

            for (int kk = 0; kk < countK; kk++)
            {
                var target = destination.AsSpan(blockBase + kk * BlockWidth, BlockWidth);
                // Columns past the panel's real width are zero so the kernel runs one shape.
                if (pixels < BlockWidth) target[pixels..].Clear();

                if (singleRow)
                {
                    int iy = oy0 * strideH + tapRow[kk];
                    if ((uint)iy >= (uint)height)
                    {
                        target[..pixels].Clear();
                        continue;
                    }

                    int srcRow = tapPlane[kk] + iy * width;
                    int firstIx = ox0 * strideW + tapColumn[kk];
                    int lastIx = (ox0 + pixels - 1) * strideW + tapColumn[kk];

                    if (strideW == 1 && firstIx >= 0 && lastIx < width)
                        source.AsSpan(srcRow + firstIx, pixels).CopyTo(target);
                    else
                        GatherStrided(source, srcRow, target, pixels, ox0, strideW, tapColumn[kk], width);
                    continue;
                }

                // The block straddles an output row; walk it in per-row runs.
                int position = 0;
                while (position < pixels)
                {
                    int pixel = firstPixel + position;
                    int oy = pixel / outW;
                    int ox = pixel - oy * outW;
                    int run = Math.Min(outW - ox, pixels - position);

                    int iy = oy * strideH + tapRow[kk];
                    if ((uint)iy >= (uint)height)
                    {
                        target.Slice(position, run).Clear();
                        position += run;
                        continue;
                    }

                    int srcRow = tapPlane[kk] + iy * width;
                    int firstIx = ox * strideW + tapColumn[kk];
                    int lastIx = (ox + run - 1) * strideW + tapColumn[kk];

                    if (strideW == 1 && firstIx >= 0 && lastIx < width)
                        source.AsSpan(srcRow + firstIx, run).CopyTo(target.Slice(position, run));
                    else
                        GatherStrided(source, srcRow, target[position..], run, ox, strideW, tapColumn[kk], width);
                    position += run;
                }
            }
        }
    }

    /// <summary>Element-wise gather for the edges: a strided read, or one that leaves the image.</summary>
    private static void GatherStrided(
        float[] source, int srcRow, Span<float> target, int run, int ox, int strideW, int columnOffset, int width)
    {
        for (int i = 0; i < run; i++)
        {
            int ix = (ox + i) * strideW + columnOffset;
            target[i] = (uint)ix < (uint)width ? source[srcRow + ix] : 0f;
        }
    }

    /// <summary>
    /// Apply the per-channel bias and any fused activation in a single pass over each plane.
    /// <para>
    /// Keeping these together is the point of fusing them. As separate graph nodes the bias
    /// and the activation each stream the whole activation tensor through memory and write a
    /// fresh one back; done here, the plane is already in cache from the multiply that
    /// produced it, and no intermediate is materialised at all.
    /// </para>
    /// </summary>
    private static void ApplyBiasAndActivation(
        Tensor result, Tensor? bias, FusedActivation activation, int batch, int channels, int spatial)
    {
        if (bias is null && activation == FusedActivation.None) return;

        Parallel.For(0, batch * channels, Cancel.ParallelOptions, plane =>
        {
            var span = result.Floats.AsSpan(plane * spatial, spatial);
            if (bias is not null) TensorPrimitives.Add(span, bias.Floats[plane % channels], span);

            switch (activation)
            {
                case FusedActivation.Relu:
                    TensorPrimitives.Max(span, 0f, span);
                    break;
                case FusedActivation.Sigmoid:
                    TensorPrimitives.Sigmoid(span, span);
                    break;
                case FusedActivation.SiLU:
                    SiLUInPlace(span);
                    break;
            }
        });
    }

    /// <summary>
    /// <c>x * sigmoid(x)</c> in place, using a stack buffer for the sigmoid so the whole
    /// activation is never duplicated.
    /// </summary>
    private static void SiLUInPlace(Span<float> span)
    {
        const int ChunkSize = 1024;
        Span<float> scratch = stackalloc float[ChunkSize];
        for (int offset = 0; offset < span.Length; offset += ChunkSize)
        {
            var chunk = span.Slice(offset, Math.Min(ChunkSize, span.Length - offset));
            var buffer = scratch[..chunk.Length];
            TensorPrimitives.Sigmoid(chunk, buffer);
            TensorPrimitives.Multiply(chunk, buffer, chunk);
        }
    }

    /// <summary>
    /// Inference-mode batch normalisation, collapsed to one affine pass.
    /// <para>
    /// The four per-channel parameters fold into a single scale and shift —
    /// <c>a = scale / sqrt(var + eps)</c>, <c>b = B - mean * a</c> — so each plane costs one
    /// fused multiply-add rather than a subtract, a divide, a multiply and an add. The
    /// reciprocal square root is computed once per channel, not once per pixel.
    /// </para>
    /// </summary>
    public static Tensor BatchNormalization(Tensor x, Tensor scale, Tensor bias, Tensor mean, Tensor variance, float epsilon)
    {
        var fx = x.AsFloat();
        int channels = fx.Rank >= 2 ? fx.Shape[1] : 1;
        int batch = fx.Rank >= 1 ? fx.Shape[0] : 1;
        int spatial = 1;
        for (int i = 2; i < fx.Rank; i++) spatial *= fx.Shape[i];

        var result = Tensor.AllocateFloat(fx.Shape);
        var fs = scale.AsFloat();
        var fb = bias.AsFloat();
        var fm = mean.AsFloat();
        var fv = variance.AsFloat();

        for (int c = 0; c < channels; c++)
        {
            float a = fs.Floats[c] / MathF.Sqrt(fv.Floats[c] + epsilon);
            float b = fb.Floats[c] - fm.Floats[c] * a;
            for (int n = 0; n < batch; n++)
            {
                int offset = (n * channels + c) * spatial;
                var src = fx.Floats.AsSpan(offset, spatial);
                var dst = result.Floats.AsSpan(offset, spatial);
                TensorPrimitives.Multiply(src, a, dst);
                TensorPrimitives.Add(dst, b, dst);
            }
        }
        return result;
    }

    /// <summary>
    /// Layer normalisation over the trailing axes: normalise each row to zero mean and unit
    /// variance, then apply the learned affine. Exported graphs often spell this out as
    /// ReduceMean/Sub/Pow/Sqrt/Div nodes instead, which run through the generic kernels.
    /// </summary>
    public static Tensor LayerNormalization(Tensor x, Tensor scale, Tensor? bias, long axis, float epsilon)
    {
        var fx = x.AsFloat();
        int a = Shapes.NormalizeAxis(axis, fx.Rank);
        int rows = 1;
        for (int i = 0; i < a; i++) rows *= fx.Shape[i];
        int cols = 1;
        for (int i = a; i < fx.Rank; i++) cols *= fx.Shape[i];

        var result = Tensor.AllocateFloat(fx.Shape);
        var fs = scale.AsFloat();
        var fb = bias?.AsFloat();

        for (int r = 0; r < rows; r++)
        {
            var src = fx.Floats.AsSpan(r * cols, cols);
            var dst = result.Floats.AsSpan(r * cols, cols);
            float mean = TensorPrimitives.Sum(src) / cols;
            TensorPrimitives.Subtract(src, mean, dst);
            float variance = TensorPrimitives.SumOfSquares(dst) / cols;
            TensorPrimitives.Multiply(dst, 1f / MathF.Sqrt(variance + epsilon), dst);
            TensorPrimitives.Multiply(dst, fs.Floats, dst);
            if (fb is not null) TensorPrimitives.Add(dst, fb.Floats, dst);
        }
        return result;
    }
}
