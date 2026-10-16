namespace XRay.Content.Internal.Onnx.Ops;

/// <summary>
/// Spatial resampling: <c>Resize</c> and <c>GridSample</c>.
/// <para>
/// Both are dominated by their coordinate conventions rather than their arithmetic, and the
/// conventions are where a re-implementation silently goes wrong: a half-pixel offset
/// applied in the wrong place shifts every feature by half a cell, which survives training-
/// free inference as plausible-looking but consistently displaced boxes. Each mode is
/// therefore spelled out rather than folded into a shared formula.
/// </para>
/// </summary>
internal static class Sampling
{
    /// <summary>
    /// Map an output coordinate back to input space under an ONNX
    /// <c>coordinate_transformation_mode</c>.
    /// </summary>
    private static float SourceCoordinate(int outIndex, float scale, int outLength, int inLength, string mode) => mode switch
    {
        "asymmetric" => outIndex / scale,
        "align_corners" => outLength == 1 ? 0f : outIndex * (inLength - 1f) / (outLength - 1f),
        // pytorch_half_pixel degenerates to 0 for a single-element output, unlike half_pixel.
        "pytorch_half_pixel" => outLength > 1 ? (outIndex + 0.5f) / scale - 0.5f : 0f,
        _ => (outIndex + 0.5f) / scale - 0.5f, // half_pixel, the ONNX default
    };

    /// <summary>Round an input coordinate to a source index under a <c>nearest_mode</c>.</summary>
    private static int NearestIndex(float coordinate, string mode) => mode switch
    {
        "floor" => (int)MathF.Floor(coordinate),
        "ceil" => (int)MathF.Ceiling(coordinate),
        "round_prefer_ceil" => (int)MathF.Floor(coordinate + 0.5f),
        // round_prefer_floor, the ONNX default: an exact .5 rounds down.
        _ => (int)MathF.Ceiling(coordinate - 0.5f),
    };

    /// <summary>
    /// Resize the two spatial axes of an <c>[N,C,H,W]</c> tensor. Target geometry comes from
    /// explicit <paramref name="sizes"/> when present, otherwise from <paramref name="scales"/>.
    /// </summary>
    public static Tensor Resize(
        Tensor x, Tensor? scales, Tensor? sizes, string mode, string coordinateMode, string nearestMode)
    {
        var fx = x.AsFloat();
        if (fx.Rank != 4) throw new NotSupportedException("resize: only 4-D NCHW input is supported");

        int batch = fx.Shape[0], channels = fx.Shape[1], height = fx.Shape[2], width = fx.Shape[3];
        int outH, outW;
        float scaleH, scaleW;

        if (sizes is { Count: >= 4 })
        {
            outH = (int)sizes.GetLong(2);
            outW = (int)sizes.GetLong(3);
            scaleH = height == 0 ? 1f : (float)outH / height;
            scaleW = width == 0 ? 1f : (float)outW / width;
        }
        else if (scales is { Count: >= 4 })
        {
            scaleH = scales.GetFloat(2);
            scaleW = scales.GetFloat(3);
            outH = (int)MathF.Floor(height * scaleH);
            outW = (int)MathF.Floor(width * scaleW);
        }
        else
        {
            throw new InvalidDataException("resize: neither scales nor sizes were provided");
        }

        var result = Tensor.AllocateFloat(batch, channels, outH, outW);
        int planeIn = height * width;
        int planeOut = outH * outW;

        // Source coordinates depend only on the output index, so they are computed once per
        // axis and reused across every plane instead of once per output pixel.
        var srcY = new float[outH];
        var srcX = new float[outW];
        for (int oy = 0; oy < outH; oy++) srcY[oy] = SourceCoordinate(oy, scaleH, outH, height, coordinateMode);
        for (int ox = 0; ox < outW; ox++) srcX[ox] = SourceCoordinate(ox, scaleW, outW, width, coordinateMode);

        bool linear = mode == "linear";

        Parallel.For(0, batch * channels, Cancel.ParallelOptions, plane =>
        {
            var src = fx.Floats.AsSpan(plane * planeIn, planeIn);
            var dst = result.Floats.AsSpan(plane * planeOut, planeOut);

            for (int oy = 0; oy < outH; oy++)
            {
                for (int ox = 0; ox < outW; ox++)
                {
                    if (!linear)
                    {
                        int iy = Math.Clamp(NearestIndex(srcY[oy], nearestMode), 0, height - 1);
                        int ix = Math.Clamp(NearestIndex(srcX[ox], nearestMode), 0, width - 1);
                        dst[oy * outW + ox] = src[iy * width + ix];
                        continue;
                    }

                    float fy = Math.Clamp(srcY[oy], 0f, height - 1f);
                    float fxCoord = Math.Clamp(srcX[ox], 0f, width - 1f);
                    int y0 = (int)MathF.Floor(fy), x0 = (int)MathF.Floor(fxCoord);
                    int y1 = Math.Min(y0 + 1, height - 1), x1 = Math.Min(x0 + 1, width - 1);
                    float wy = fy - y0, wx = fxCoord - x0;

                    float top = src[y0 * width + x0] * (1 - wx) + src[y0 * width + x1] * wx;
                    float bottom = src[y1 * width + x0] * (1 - wx) + src[y1 * width + x1] * wx;
                    dst[oy * outW + ox] = top * (1 - wy) + bottom * wy;
                }
            }
        });
        return result;
    }

    /// <summary>
    /// GridSample: read <c>[N,C,H,W]</c> input at the normalised coordinates in
    /// <c>grid[N,outH,outW,2]</c>, where the last axis is <c>(x, y)</c> in <c>[-1, 1]</c>.
    /// <para>
    /// RT-DETR's deformable attention is built on this: every sampling offset the model
    /// predicts lands here, so its coordinate convention is load-bearing for box accuracy.
    /// </para>
    /// </summary>
    public static Tensor GridSample(Tensor x, Tensor grid, string mode, string paddingMode, bool alignCorners)
    {
        var fx = x.AsFloat();
        var fg = grid.AsFloat();
        if (fx.Rank != 4 || fg.Rank != 4) throw new NotSupportedException("grid_sample: only 4-D input is supported");

        int batch = fx.Shape[0], channels = fx.Shape[1], height = fx.Shape[2], width = fx.Shape[3];
        int outH = fg.Shape[1], outW = fg.Shape[2];
        var result = Tensor.AllocateFloat(batch, channels, outH, outW);

        int planeIn = height * width;
        int planeOut = outH * outW;
        bool nearest = mode == "nearest";

        Parallel.For(0, batch, Cancel.ParallelOptions, n =>
        {
            for (int oy = 0; oy < outH; oy++)
            {
                for (int ox = 0; ox < outW; ox++)
                {
                    int gridPos = ((n * outH + oy) * outW + ox) * 2;
                    float gx = fg.Floats[gridPos];
                    float gy = fg.Floats[gridPos + 1];

                    float px = Denormalize(gx, width, alignCorners);
                    float py = Denormalize(gy, height, alignCorners);

                    if (paddingMode == "border")
                    {
                        px = Math.Clamp(px, 0f, width - 1f);
                        py = Math.Clamp(py, 0f, height - 1f);
                    }
                    else if (paddingMode == "reflection")
                    {
                        px = Reflect(px, width, alignCorners);
                        py = Reflect(py, height, alignCorners);
                    }

                    int outPos = oy * outW + ox;
                    if (nearest)
                    {
                        int ix = (int)MathF.Round(px, MidpointRounding.ToEven);
                        int iy = (int)MathF.Round(py, MidpointRounding.ToEven);
                        for (int c = 0; c < channels; c++)
                        {
                            float value = InBounds(ix, iy, width, height)
                                ? fx.Floats[(n * channels + c) * planeIn + iy * width + ix]
                                : 0f;
                            result.Floats[(n * channels + c) * planeOut + outPos] = value;
                        }
                        continue;
                    }

                    int x0 = (int)MathF.Floor(px), y0 = (int)MathF.Floor(py);
                    int x1 = x0 + 1, y1 = y0 + 1;
                    float wx = px - x0, wy = py - y0;
                    float w00 = (1 - wx) * (1 - wy);
                    float w10 = wx * (1 - wy);
                    float w01 = (1 - wx) * wy;
                    float w11 = wx * wy;

                    for (int c = 0; c < channels; c++)
                    {
                        int planeBase = (n * channels + c) * planeIn;
                        // Out-of-bounds taps contribute zero under "zeros" padding; under the
                        // other modes the coordinates were already brought back in range.
                        float value =
                            Sample(fx.Floats, planeBase, x0, y0, width, height) * w00 +
                            Sample(fx.Floats, planeBase, x1, y0, width, height) * w10 +
                            Sample(fx.Floats, planeBase, x0, y1, width, height) * w01 +
                            Sample(fx.Floats, planeBase, x1, y1, width, height) * w11;
                        result.Floats[(n * channels + c) * planeOut + outPos] = value;
                    }
                }
            }
        });
        return result;
    }

    /// <summary>
    /// Normalised <c>[-1,1]</c> to pixel coordinates. With <c>align_corners</c> the extremes
    /// name the centres of the first and last pixels; without it they name the outer edges of
    /// the image, which is the half-pixel convention RT-DETR's export uses.
    /// </summary>
    private static float Denormalize(float coordinate, int length, bool alignCorners) =>
        alignCorners
            ? (coordinate + 1f) / 2f * (length - 1)
            : ((coordinate + 1f) * length - 1f) / 2f;

    private static bool InBounds(int x, int y, int width, int height) =>
        (uint)x < (uint)width && (uint)y < (uint)height;

    private static float Sample(float[] data, int planeBase, int x, int y, int width, int height) =>
        InBounds(x, y, width, height) ? data[planeBase + y * width + x] : 0f;

    /// <summary>Fold an out-of-range coordinate back inside by mirroring at the boundaries.</summary>
    private static float Reflect(float coordinate, int length, bool alignCorners)
    {
        float low = alignCorners ? 0f : -0.5f;
        float high = alignCorners ? length - 1f : length - 0.5f;
        float span = high - low;
        if (span <= 0f) return low;

        float shifted = coordinate - low;
        // Reflecting has period 2*span; fold into that window, then mirror the far half.
        shifted = MathF.Abs(shifted);
        shifted = MathF.IEEERemainder(shifted, 2f * span);
        if (shifted < 0f) shifted += 2f * span;
        if (shifted > span) shifted = 2f * span - shifted;
        return Math.Clamp(shifted + low, alignCorners ? 0f : 0f, length - 1f);
    }
}
