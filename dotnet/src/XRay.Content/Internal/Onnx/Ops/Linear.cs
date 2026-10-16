using System.Numerics.Tensors;
using System.Numerics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

namespace XRay.Content.Internal.Onnx.Ops;

/// <summary>
/// Matrix multiplication — the arithmetic core of both halves of these graphs, since
/// convolution is lowered onto it too.
/// <para>
/// This layer handles ONNX's shape rules — 1-D promotion, batch broadcasting, transposes and
/// the alpha/beta scaling — and hands the actual arithmetic to <see cref="GemmKernel"/>.
/// </para>
/// </summary>
internal static class Linear
{
    /// <summary>Multiply-accumulate operations above which the work is worth spreading across cores.</summary>
    private const long ParallelThreshold = 1L << 18;

    /// <summary>
    /// ONNX MatMul, including the 1-D promotions and leading-dimension broadcasting.
    /// </summary>
    public static Tensor MatMul(Tensor a, Tensor b)
    {
        var fa = a.AsFloat();
        var fb = b.AsFloat();

        // A 1-D operand is promoted to a matrix for the multiply, then the added dimension
        // is removed again — exactly the numpy rule ONNX inherits.
        bool promotedA = fa.Rank == 1;
        bool promotedB = fb.Rank == 1;
        if (promotedA) fa = fa.Reshaped(1, fa.Shape[0]);
        if (promotedB) fb = fb.Reshaped(fb.Shape[0], 1);

        int m = fa.Shape[^2], k = fa.Shape[^1];
        int k2 = fb.Shape[^2], n = fb.Shape[^1];
        if (k != k2)
            throw new InvalidDataException($"matmul: inner dimensions differ ({k} vs {k2})");

        // Broadcast the batch dimensions that precede the two matrix axes.
        var batchA = fa.Shape[..^2];
        var batchB = fb.Shape[..^2];
        var batchShape = Broadcast.ResultShape(batchA, batchB);
        int batchRank = batchShape.Length;
        int batches = Tensor.ElementCount(batchShape);
        var strideA = Broadcast.StridesFor(batchA, batchRank);
        var strideB = Broadcast.StridesFor(batchB, batchRank);

        var outShape = new int[batchRank + 2];
        batchShape.CopyTo(outShape, 0);
        outShape[batchRank] = m;
        outShape[batchRank + 1] = n;
        var result = Tensor.AllocateFloat(outShape);

        int sizeA = m * k, sizeB = k * n, sizeC = m * n;

        // Resolve each batch's operand offsets up front so the batches can be dispatched in
        // any order.
        var offsetsA = new int[batches];
        var offsetsB = new int[batches];
        {
            var index = new int[Math.Max(batchRank, 1)];
            int offsetA = 0, offsetB = 0;
            for (int batch = 0; batch < batches; batch++)
            {
                offsetsA[batch] = offsetA;
                offsetsB[batch] = offsetB;
                for (int d = batchRank - 1; d >= 0; d--)
                {
                    index[d]++;
                    offsetA += strideA[d];
                    offsetB += strideB[d];
                    if (index[d] < batchShape[d]) break;
                    offsetA -= strideA[d] * index[d];
                    offsetB -= strideB[d] * index[d];
                    index[d] = 0;
                }
            }
        }

        // Attention layers issue many small batched products — eight heads of a few hundred
        // rows each. Parallelising inside every one of those pays the thread-pool handshake
        // per batch on a few milliseconds of work, which is why the decoder's multiplies
        // measured an order of magnitude below the backbone's. Spreading the batches instead
        // gives one dispatch for the whole node and leaves each product running straight
        // through.
        if (batches > 1)
        {
            Parallel.For(0, batches, Cancel.ParallelOptions, batch => MultiplyInto(
                fa.Floats.AsMemory(offsetsA[batch] * sizeA, sizeA),
                fb.Floats.AsMemory(offsetsB[batch] * sizeB, sizeB),
                result.Floats.AsMemory(batch * sizeC, sizeC),
                m, k, n, n, parallel: false));
        }
        else
        {
            MultiplyInto(
                fa.Floats.AsMemory(offsetsA[0] * sizeA, sizeA),
                fb.Floats.AsMemory(offsetsB[0] * sizeB, sizeB),
                result.Floats.AsMemory(0, sizeC),
                m, k, n);
        }

        // Undo the 1-D promotions on the result.
        if (promotedA && promotedB) return result.Reshaped();
        if (promotedA)
        {
            var shape = outShape.Where((_, i) => i != batchRank).ToArray();
            return result.Reshaped(shape);
        }
        if (promotedB) return result.Reshaped(outShape[..^1]);
        return result;
    }

    /// <summary>
    /// One <c>[m,k] x [k,n] -> [m,n]</c> product.
    /// </summary>
    public static void MultiplyInto(
        ReadOnlyMemory<float> a, ReadOnlyMemory<float> b, Memory<float> c, int m, int k, int n) =>
        GemmKernel.Multiply(a, b, c, m, k, n, n, parallel: true, ParallelThreshold);

    /// <summary>
    /// As above, but writing into a destination whose rows are
    /// <paramref name="destinationStride"/> apart rather than packed.
    /// <para>
    /// Tiled convolution needs this: it computes a slice of output <em>columns</em> at a time
    /// and scatters each into the full-width result, so the destination rows are strided by
    /// the whole spatial extent. <paramref name="parallel"/> is turned off when the caller is
    /// already running one tile per thread, so the two levels do not oversubscribe.
    /// </para>
    /// </summary>
    public static void MultiplyInto(
        ReadOnlyMemory<float> a, ReadOnlyMemory<float> b, Memory<float> c,
        int m, int k, int n, int destinationStride, bool parallel) =>
        GemmKernel.Multiply(a, b, c, m, k, n, destinationStride, parallel, ParallelThreshold);

    /// <summary>
    /// Gemm: <c>alpha * A' * B' + beta * C</c>, with optional transposes and a broadcast bias.
    /// </summary>
    public static Tensor Gemm(Tensor a, Tensor b, Tensor? c, float alpha, float beta, bool transA, bool transB)
    {
        var fa = a.AsFloat();
        var fb = b.AsFloat();
        if (fa.Rank != 2 || fb.Rank != 2) throw new InvalidDataException("gemm: A and B must be rank 2");

        if (transA) fa = Shapes.Transpose(fa, [1, 0]);
        if (transB) fb = Shapes.Transpose(fb, [1, 0]);

        int m = fa.Shape[0], k = fa.Shape[1], n = fb.Shape[1];
        if (fb.Shape[0] != k) throw new InvalidDataException($"gemm: inner dimensions differ ({k} vs {fb.Shape[0]})");

        var result = Tensor.AllocateFloat(m, n);
        MultiplyInto(fa.Floats, fb.Floats, result.Floats, m, k, n);

        if (alpha != 1f) TensorPrimitives.Multiply(result.Floats, alpha, result.Floats);

        if (c is not null && beta != 0f)
        {
            var bias = c.AsFloat();
            var scaled = beta == 1f ? bias : Elementwise.Binary(bias, Tensor.Scalar(beta), BinaryKind.Mul);
            return Elementwise.Binary(result, scaled, BinaryKind.Add);
        }
        return result;
    }
}
