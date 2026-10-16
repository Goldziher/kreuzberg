using System.Numerics;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Runtime.Intrinsics;
using System.Runtime.Intrinsics.X86;

namespace XRay.Content.Internal.Onnx.Ops;

/// <summary>
/// The single-precision matrix multiply, structured after ONNX Runtime's MLAS.
/// <para>
/// The shape of this file is not invention. Profiling put convolution at well over half of
/// inference, convolution lowers onto this multiply, and the multiply was running at roughly
/// a fifth of the machine's measured ceiling — so MLAS's own SGEMM was read to find what it
/// does differently. Three things, all reproduced here.
/// </para>
/// <para>
/// <strong>The right-hand operand is packed.</strong> Before any arithmetic, a panel of
/// <c>B</c> is copied into a buffer where each group of sixteen columns is made physically
/// contiguous — <c>[columnBlock][k][16]</c>. The kernel then walks it strictly forwards
/// instead of jumping a full row stride per step, which is what turns its operand reads into
/// a stream the prefetcher can follow. The copy is paid once per panel and amortised over
/// every row block that consumes it.
/// </para>
/// <para>
/// <strong>The register block is six rows by two vectors.</strong> Twelve accumulators, two
/// operand vectors, and a <em>single</em> broadcast register reused across the rows — fifteen
/// live values, which is what fits AVX2's sixteen registers and what MLAS itself uses. An
/// earlier attempt at eight rows measured markedly slower, and reading MLAS explains why the
/// ceiling is where it is rather than higher.
/// </para>
/// <para>
/// <strong>The depth loop accumulates in place.</strong> The first depth panel writes the
/// destination and later ones add to it, so nothing pre-clears <c>C</c> and each output
/// element is touched once per panel rather than twice.
/// </para>
/// </summary>
internal static class GemmKernel
{
    /// <summary>Columns per packed block. Sixteen matches the 512-bit vector width and is the
    /// unit MLAS packs in; the 256-bit kernel simply consumes each block as two vectors.</summary>
    private const int BlockWidth = 16;

    /// <summary>Output rows per register block.</summary>
    private const int RowBlock = 6;

    /// <summary>Output rows for the AVX-512 kernel, which has registers for twice as many.</summary>
    private const int WideRowBlock = 12;

    /// <summary>
    /// Panel extents, matching MLAS's <c>MLAS_SGEMM_STRIDEN</c> and <c>STRIDEK</c>. A
    /// 128x128 float panel is 64 KB, which stays resident while every row block reads it.
    /// </summary>
    private const int StrideN = 128;

    /// <inheritdoc cref="StrideN"/>
    private const int StrideK = 128;

    private static bool Use512 { get; } = Avx512F.IsSupported;

    /// <summary>Scratch for the packed panel, one per thread and reused across calls.</summary>
    [ThreadStatic]
    private static float[]? _packed;

    /// <summary>
    /// <c>C[m,n] = A[m,k] * B[k,n]</c>, where <c>C</c>'s rows are
    /// <paramref name="ldc"/> apart.
    /// </summary>
    /// <param name="parallel">
    /// Spread column panels across threads. Turned off by callers that are already running
    /// one unit of work per thread, so the two levels do not oversubscribe.
    /// </param>
    public static void Multiply(
        ReadOnlyMemory<float> a, ReadOnlyMemory<float> b, Memory<float> c,
        int m, int k, int n, int ldc, bool parallel, long parallelThreshold)
    {
        var source = b;
        int ldb = n;
        Multiply(a, (destination, offset, pc, countK, jc, countN) =>
            PackB(destination, offset, source.Span, ldb, pc, countK, jc, countN),
            c, m, k, n, ldc, parallel, parallelThreshold);
    }

    /// <summary>
    /// As <see cref="Multiply(ReadOnlyMemory{float}, ReadOnlyMemory{float}, Memory{float}, int, int, int, int, bool, long)"/>,
    /// but with the caller supplying the right-hand operand a panel at a time.
    /// <para>
    /// This exists for convolution. Materialising the unrolled receptive fields into a buffer
    /// and then packing that buffer moves the expanded data — nine times the input for a 3x3
    /// layer — through memory twice. A caller that can produce the packed panel directly
    /// writes it once, and the intermediate disappears entirely.
    /// </para>
    /// </summary>
    public static void Multiply(
        ReadOnlyMemory<float> a, PanelPacker packB, Memory<float> c,
        int m, int k, int n, int ldc, bool parallel, long parallelThreshold)
    {
        if (m == 0 || n == 0 || k == 0) return;

        var (baseStrideN, strideK) = PanelExtents(k, n);
        var (strideN, panels) = ChoosePanels(n, baseStrideN, parallel);

        long work = (long)m * n * k;
        bool spread = parallel && work >= parallelThreshold;

        // The column panel is the natural unit of parallel work, but a narrow output does not
        // have enough of them to fill the machine: the decoder's projections are 256 columns
        // wide over 8400 rows, which is two panels and therefore two busy cores out of four.
        // Where that happens the rows are split as well. It costs one repacking of the panel
        // per extra chunk, which is only affordable because few panels means a small operand —
        // so the split is confined to exactly that case.
        int rowChunks = 1, rowsPerChunk = m;
        int workers = Environment.ProcessorCount;
        if (spread && panels < workers && m >= 2 * WideRowBlock)
        {
            int wanted = Math.Min((workers + panels - 1) / panels, m / WideRowBlock);
            rowsPerChunk = RoundUp((m + wanted - 1) / wanted, WideRowBlock);
            rowChunks = (m + rowsPerChunk - 1) / rowsPerChunk;
        }

        int tiles = panels * rowChunks;
        if (spread && tiles > 1)
        {
            int capturedStrideN = strideN, capturedRows = rowsPerChunk, capturedChunks = rowChunks;
            Parallel.For(0, tiles, Cancel.ParallelOptions, tile =>
            {
                int index = tile / capturedChunks;
                int jc = index * capturedStrideN;
                int chunk = tile % capturedChunks;
                ColumnPanel(
                    a.Span, packB, c.Span, k, ldc,
                    chunk * capturedRows, Math.Min(m, (chunk + 1) * capturedRows),
                    jc, Math.Min(capturedStrideN, n - jc), strideK);
            });
        }
        else
        {
            for (int index = 0; index < panels; index++)
                ColumnPanel(
                    a.Span, packB, c.Span, k, ldc, 0, m,
                    index * strideN, Math.Min(strideN, n - index * strideN), strideK);
        }
    }

    private static int RoundUp(int value, int multiple) => (value + multiple - 1) / multiple * multiple;

    /// <summary>
    /// How wide a column panel should be, and how many there are.
    /// <para>
    /// Filling each panel to the target width and leaving whatever remains is the wrong split
    /// twice over: the last panel can be a sliver, and the panel count decides how many
    /// parallel rounds there are. 1600 columns at the natural 128 is thirteen panels — four
    /// rounds on four cores, the last of them one thread wide, with a 64-column runt. Twelve
    /// panels of 144 is three full rounds.
    /// </para>
    /// <para>
    /// So the panel count is chosen to minimise how long the slowest core works: the number of
    /// rounds times the width of a panel. Widening a panel costs cache residency, hence the
    /// cap at twice the natural width.
    /// </para>
    /// </summary>
    private static (int StrideN, int Panels) ChoosePanels(int n, int baseStrideN, bool parallel)
    {
        int natural = (n + baseStrideN - 1) / baseStrideN;
        int workers = parallel ? Environment.ProcessorCount : 1;

        int bestPanels = natural;
        int bestStride = Math.Min(baseStrideN, RoundUp((n + natural - 1) / natural, BlockWidth));
        long bestCost = Makespan(bestPanels, bestStride, workers);

        for (int panels = Math.Max(1, natural - workers); panels <= natural + workers; panels++)
        {
            int stride = RoundUp((n + panels - 1) / panels, BlockWidth);
            if (stride > 2 * baseStrideN) continue;

            // Rounding the width up can leave fewer panels than asked for; cost the real split.
            int actual = (n + stride - 1) / stride;
            long cost = Makespan(actual, stride, workers);
            if (cost < bestCost) (bestCost, bestPanels, bestStride) = (cost, actual, stride);
        }

        return (bestStride, bestPanels);
    }

    private static long Makespan(int panels, int strideN, int workers) =>
        (long)((panels + workers - 1) / workers) * strideN;

    /// <summary>
    /// The column and depth extents of one panel.
    /// <para>
    /// MLAS reshapes the panel when one dimension is small: a shallow reduction wants wider
    /// column panels, a narrow output wants deeper ones, so the panel stays a similar size
    /// either way.
    /// </para>
    /// </summary>
    internal static (int StrideN, int StrideK) PanelExtents(int k, int n)
    {
        int strideN = StrideN, strideK = StrideK;
        if (n >= k)
        {
            while (strideK / 2 >= k && strideK > 16) { strideN *= 2; strideK /= 2; }
        }
        else
        {
            while (strideN > BlockWidth && strideN / 2 >= n) { strideK *= 2; strideN /= 2; }
        }
        return (strideN, strideK);
    }

    /// <summary>
    /// Fills <paramref name="destination"/> with the packed form of
    /// <c>B[pc..pc+countK, jc..jc+countN]</c>, laid out as <c>[columnBlock][k][16]</c> with the
    /// trailing block zero-filled.
    /// </summary>
    internal delegate void PanelPacker(float[] destination, int offset, int pc, int countK, int jc, int countN);

    /// <summary>
    /// Compute one tile of the destination — a range of rows within one column panel — by
    /// packing each depth slice of the operand once and running every row block against it.
    /// </summary>
    private static void ColumnPanel(
        ReadOnlySpan<float> a, PanelPacker packB, Span<float> c,
        int k, int ldc, int rowStart, int rowEnd, int jc, int countN, int strideK)
    {
        int blocks = (countN + BlockWidth - 1) / BlockWidth;
        var (packed, packedOffset) = RentPacked(blocks * strideK * BlockWidth);

        for (int pc = 0; pc < k; pc += strideK)
        {
            int countK = Math.Min(strideK, k - pc);
            packB(packed, packedOffset, pc, countK, jc, countN);

            // The first depth slice establishes the destination; later ones add to it.
            bool first = pc == 0;
            RowBlockKernel(a, packed, packedOffset, c, k, ldc, rowStart, rowEnd, pc, countK, jc, countN, first);
        }
    }

    /// <summary>Cache line the packed panel is aligned to.</summary>
    private const int CacheLineBytes = 64;

    private const int AlignmentSlack = CacheLineBytes / sizeof(float);

    /// <summary>Offset into <see cref="_packed"/> at which the aligned data starts.</summary>
    [ThreadStatic]
    private static int _packedOffset;

    /// <summary>
    /// The packed panel, aligned to a cache line.
    /// <para>
    /// Alignment is worth arranging deliberately because .NET does not provide it: a
    /// <c>float[]</c> lands at an essentially arbitrary offset within a cache line, and the
    /// panel's rows are exactly one line apart, so the base offset decides whether
    /// <em>every</em> 512-bit load in the kernel is a split load or none of them are. The
    /// array is allocated pinned so its address cannot change after the offset is computed.
    /// </para>
    /// </summary>
    private static (float[] Buffer, int Offset) RentPacked(int elements)
    {
        var buffer = _packed;
        if (buffer is null || buffer.Length < elements + AlignmentSlack)
        {
            _packed = buffer = GC.AllocateArray<float>(elements + AlignmentSlack, pinned: true);
            _packedOffset = AlignmentOffset(buffer);
        }
        return (buffer, _packedOffset);
    }

    private static unsafe int AlignmentOffset(float[] pinned)
    {
        nuint address = (nuint)Unsafe.AsPointer(ref MemoryMarshal.GetArrayDataReference(pinned));
        nuint past = address % CacheLineBytes;
        return past == 0 ? 0 : (int)((CacheLineBytes - past) / sizeof(float));
    }

    /// <summary>
    /// Copy <c>B[pc..pc+countK, jc..jc+countN]</c> into the packed layout
    /// <c>[block][k][16]</c>, zero-filling the final block when the panel is not a whole
    /// number of blocks wide — which lets the kernel run one uniform shape and ignore edges.
    /// <para>
    /// The loop runs depth-major, not block-major, even though the destination is laid out
    /// block-major. Doing it the other way round reads each row of <c>B</c> once per column
    /// block — eight separate sixteen-float reads scattered across a row stride that for a
    /// 160x160 feature map is 100 KB, so the same pages are revisited eight times and nothing
    /// prefetches. This way each row is read once, sequentially, and it is the writes that
    /// scatter — across only eight destinations, which is what a write-combining buffer is for.
    /// </para>
    /// </summary>
    internal static void PackB(
        float[] packed, int offset, ReadOnlySpan<float> b, int ldb, int pc, int countK, int jc, int countN)
    {
        int blocks = (countN + BlockWidth - 1) / BlockWidth;
        int wholeBlocks = countN / BlockWidth;
        int blockStride = countK * BlockWidth;
        ref float source = ref MemoryMarshal.GetReference(b);
        ref float destination = ref MemoryMarshal.GetArrayDataReference(packed);

        for (int kk = 0; kk < countK; kk++)
        {
            int from = (pc + kk) * ldb + jc;
            int to = offset + kk * BlockWidth;

            for (int block = 0; block < wholeBlocks; block++)
                CopyBlock(ref source, from + block * BlockWidth, ref destination, to + block * blockStride);

            if (wholeBlocks < blocks)
            {
                int column = wholeBlocks * BlockWidth;
                int width = countN - column;
                var target = packed.AsSpan(to + wholeBlocks * blockStride, BlockWidth);
                b.Slice(from + column, width).CopyTo(target);
                target[width..].Clear();
            }
        }
    }

    /// <summary>Lane selector picking every other float from a pair of vectors.</summary>
    private static readonly Vector512<int> EvenLanes =
        Vector512.Create(0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30);

    /// <summary>
    /// Move one packed block whose source elements are <paramref name="step"/> apart.
    /// <para>
    /// A convolution that downsamples reads every <c>step</c>-th pixel, and there are enough of
    /// those in a ResNet backbone — every stage transition — that they cannot go down the
    /// element-at-a-time path. Stride two is the case that occurs, and it is exactly one pair
    /// of loads and a two-source permute.
    /// </para>
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    internal static void GatherBlock(
        ref float source, int length, int from, int step, ref float destination, int to)
    {
        if (step == 1)
        {
            CopyBlock(ref source, from, ref destination, to);
            return;
        }

        // The wide read consumes 2x16 floats to produce 16; near the end of the last plane
        // that would run past the tensor, so the scalar form covers the final block.
        if (Avx512F.IsSupported && step == 2 && from + 2 * BlockWidth <= length)
        {
            var lower = Vector512.LoadUnsafe(ref source, (nuint)from);
            var upper = Vector512.LoadUnsafe(ref source, (nuint)(from + BlockWidth));
            Avx512F.PermuteVar16x32x2(lower, EvenLanes, upper).StoreUnsafe(ref destination, (nuint)to);
            return;
        }

        for (int i = 0; i < BlockWidth; i++)
            Unsafe.Add(ref destination, to + i) = Unsafe.Add(ref source, from + i * step);
    }

    /// <summary>
    /// Move one packed block — sixteen floats, exactly one cache line — as a single vector
    /// operation.
    /// <para>
    /// At this size the copy itself is the cost: a general-purpose <see cref="Span{T}.CopyTo"/>
    /// dispatches on length and calls out to <c>Buffer.Memmove</c>, which for one cache line is
    /// mostly overhead. The width tests below are JIT intrinsics folded to constants, so only
    /// one arm survives in the compiled code.
    /// </para>
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    internal static void CopyBlock(ref float source, int from, ref float destination, int to)
    {
        // Vector512.IsHardwareAccelerated reports false on this runtime even where AVX-512 is
        // present, so the capability is asked of the instruction set directly.
        if (Avx512F.IsSupported)
        {
            Vector512.LoadUnsafe(ref source, (nuint)from)
                     .StoreUnsafe(ref destination, (nuint)to);
        }
        else if (Vector256.IsHardwareAccelerated)
        {
            Vector256.LoadUnsafe(ref source, (nuint)from)
                     .StoreUnsafe(ref destination, (nuint)to);
            Vector256.LoadUnsafe(ref source, (nuint)(from + 8))
                     .StoreUnsafe(ref destination, (nuint)(to + 8));
        }
        else
        {
            for (int i = 0; i < BlockWidth; i++)
                Unsafe.Add(ref destination, to + i) = Unsafe.Add(ref source, from + i);
        }
    }

    /// <summary>
    /// Run a range of output rows, taking the widest kernel that fits at each step.
    /// <para>
    /// The register kernels are given only the columns covered by whole sixteen-wide blocks.
    /// Letting them handle a ragged final block instead costs far more than the handful of
    /// columns involved: the branch needed to store a partial block is what makes the
    /// accumulators address-exposed, and then every multiply-add in the innermost loop carries
    /// a store with it. The ragged columns are computed separately, one at a time.
    /// </para>
    /// </summary>
    private static void RowBlockKernel(
        ReadOnlySpan<float> a, float[] packed, int packedOffset, Span<float> c,
        int k, int ldc, int rowStart, int rowEnd, int pc, int countK, int jc, int countN, bool first)
    {
        int fullN = countN / BlockWidth * BlockWidth;
        if (fullN < countN)
            TailColumns(a, packed, packedOffset, c, k, ldc, rowStart, rowEnd, pc, countK, jc, fullN, countN, first);
        if (fullN == 0) return;
        countN = fullN;

        int i = rowStart;
        if (Use512)
        {
            for (; rowEnd - i >= WideRowBlock; i += WideRowBlock)
                TwelveRows512(a, packed, packedOffset, c, k, ldc, i, pc, countK, jc, countN, first);
            for (; rowEnd - i >= RowBlock; i += RowBlock)
                SixRows512(a, packed, packedOffset, c, k, ldc, i, pc, countK, jc, countN, first, RowBlock);
            if (i < rowEnd)
                SixRows512(a, packed, packedOffset, c, k, ldc, i, pc, countK, jc, countN, first, rowEnd - i);
        }
        else
        {
            for (; rowEnd - i >= RowBlock; i += RowBlock)
                SixRowsVector(a, packed, packedOffset, c, k, ldc, i, pc, countK, jc, countN, first, RowBlock);
            if (i < rowEnd)
                SixRowsVector(a, packed, packedOffset, c, k, ldc, i, pc, countK, jc, countN, first, rowEnd - i);
        }
    }

    /// <summary>
    /// The columns past the last whole block: at most fifteen of them, and only in the final
    /// panel of a product whose width is not a multiple of sixteen.
    /// <para>
    /// The whole block is accumulated even though only some of its lanes are live — packing
    /// zero-fills the rest — because a dot product down the ragged columns instead reads the
    /// packed panel with a stride of sixteen and cannot be vectorised at all. Doing it that way
    /// cost more than the twelve columns were worth: a 256x300 product fell to a third of its
    /// rate. Only the merge into the destination is per-column.
    /// </para>
    /// </summary>
    private static void TailColumns(
        ReadOnlySpan<float> a, float[] packed, int packedOffset, Span<float> c,
        int k, int ldc, int rowStart, int rowEnd, int pc, int countK, int jc, int fullN, int countN, bool first)
    {
        ref float aRef = ref MemoryMarshal.GetReference(a);
        ref float pRef = ref Unsafe.Add(ref MemoryMarshal.GetArrayDataReference(packed), packedOffset);
        ref float cRef = ref MemoryMarshal.GetReference(c);
        int blockBase = BlockOffset(fullN / BlockWidth, countK);
        int width = Vector<float>.Count;

        Span<float> block = stackalloc float[BlockWidth];
        ref float blockRef = ref MemoryMarshal.GetReference(block);

        for (int i = rowStart; i < rowEnd; i++)
        {
            int a0 = i * k + pc;
            for (int lane = 0; lane < BlockWidth; lane += width)
            {
                Vector<float> acc = default;
                for (int kk = 0; kk < countK; kk++)
                    acc = Vector.FusedMultiplyAdd(
                        new Vector<float>(Unsafe.Add(ref aRef, a0 + kk)),
                        Vector.LoadUnsafe(ref pRef, (nuint)(blockBase + kk * BlockWidth + lane)),
                        acc);
                acc.StoreUnsafe(ref blockRef, (nuint)lane);
            }

            int c0 = i * ldc + jc;
            for (int column = fullN; column < countN; column++)
            {
                ref float target = ref Unsafe.Add(ref cRef, c0 + column);
                float value = Unsafe.Add(ref blockRef, column - fullN);
                target = first ? value : target + value;
            }
        }
    }

    /// <summary>Flat offset of a packed block's first element.</summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static int BlockOffset(int block, int countK) => block * countK * BlockWidth;


    /// <summary>
    /// Twelve output rows against two 16-float packed blocks — MLAS's own main kernel shape.
    /// <para>
    /// Twenty-four accumulators plus two operand vectors and one broadcast is twenty-seven of
    /// AVX-512's thirty-two registers, and halves how often the packed panel is re-read
    /// compared with six rows. This only became worthwhile once the panel was packed: an
    /// earlier wide kernel over the unpacked operand measured slower, because the operand
    /// reads, not the register pressure, were the constraint.
    /// </para>
    /// </summary>
    private static void TwelveRows512(
        ReadOnlySpan<float> a, float[] packed, int packedOffset, Span<float> c,
        int k, int ldc, int i0, int pc, int countK, int jc, int countN, bool first)
    {
        ref float aRef = ref MemoryMarshal.GetReference(a);
        ref float pRef = ref Unsafe.Add(ref MemoryMarshal.GetArrayDataReference(packed), packedOffset);
        ref float cRef = ref MemoryMarshal.GetReference(c);

        int a0 = i0 * k + pc;
        int c0 = i0 * ldc + jc;
        int blocks = (countN + BlockWidth - 1) / BlockWidth;

        // One cursor per row, so a broadcast is [row + 4*kk] — a single addressing mode.
        // Recomputing i0*k + j*k + pc + kk inside the loop instead cost a lea, an add and a
        // sign extension per row per iteration, which is more integer work than there is
        // arithmetic to hide it behind.
        ref float aRow0 = ref Unsafe.Add(ref aRef, a0);
        ref float aRow1 = ref Unsafe.Add(ref aRow0, k);
        ref float aRow2 = ref Unsafe.Add(ref aRow1, k);
        ref float aRow3 = ref Unsafe.Add(ref aRow2, k);
        ref float aRow4 = ref Unsafe.Add(ref aRow3, k);
        ref float aRow5 = ref Unsafe.Add(ref aRow4, k);
        ref float aRow6 = ref Unsafe.Add(ref aRow5, k);
        ref float aRow7 = ref Unsafe.Add(ref aRow6, k);
        ref float aRow8 = ref Unsafe.Add(ref aRow7, k);
        ref float aRow9 = ref Unsafe.Add(ref aRow8, k);
        ref float aRow10 = ref Unsafe.Add(ref aRow9, k);
        ref float aRow11 = ref Unsafe.Add(ref aRow10, k);

        for (int block = 0; block + 2 <= blocks; block += 2)
        {
            int p0 = BlockOffset(block, countK);
            int p1 = BlockOffset(block + 1, countK);
            int column = block * BlockWidth;

            Vector512<float> r00 = Vector512<float>.Zero, r01 = Vector512<float>.Zero;
            Vector512<float> r10 = Vector512<float>.Zero, r11 = Vector512<float>.Zero;
            Vector512<float> r20 = Vector512<float>.Zero, r21 = Vector512<float>.Zero;
            Vector512<float> r30 = Vector512<float>.Zero, r31 = Vector512<float>.Zero;
            Vector512<float> r40 = Vector512<float>.Zero, r41 = Vector512<float>.Zero;
            Vector512<float> r50 = Vector512<float>.Zero, r51 = Vector512<float>.Zero;
            Vector512<float> r60 = Vector512<float>.Zero, r61 = Vector512<float>.Zero;
            Vector512<float> r70 = Vector512<float>.Zero, r71 = Vector512<float>.Zero;
            Vector512<float> r80 = Vector512<float>.Zero, r81 = Vector512<float>.Zero;
            Vector512<float> r90 = Vector512<float>.Zero, r91 = Vector512<float>.Zero;
            Vector512<float> r100 = Vector512<float>.Zero, r101 = Vector512<float>.Zero;
            Vector512<float> r110 = Vector512<float>.Zero, r111 = Vector512<float>.Zero;

            for (int kk = 0; kk < countK; kk++)
            {
                var b0 = Vector512.LoadUnsafe(ref pRef, (nuint)(p0 + kk * BlockWidth));
                var b1 = Vector512.LoadUnsafe(ref pRef, (nuint)(p1 + kk * BlockWidth));
                Vector512<float> s;
                s = Vector512.Create(Unsafe.Add(ref aRow0, kk));
                r00 = Vector512.FusedMultiplyAdd(s, b0, r00);
                r01 = Vector512.FusedMultiplyAdd(s, b1, r01);
                s = Vector512.Create(Unsafe.Add(ref aRow1, kk));
                r10 = Vector512.FusedMultiplyAdd(s, b0, r10);
                r11 = Vector512.FusedMultiplyAdd(s, b1, r11);
                s = Vector512.Create(Unsafe.Add(ref aRow2, kk));
                r20 = Vector512.FusedMultiplyAdd(s, b0, r20);
                r21 = Vector512.FusedMultiplyAdd(s, b1, r21);
                s = Vector512.Create(Unsafe.Add(ref aRow3, kk));
                r30 = Vector512.FusedMultiplyAdd(s, b0, r30);
                r31 = Vector512.FusedMultiplyAdd(s, b1, r31);
                s = Vector512.Create(Unsafe.Add(ref aRow4, kk));
                r40 = Vector512.FusedMultiplyAdd(s, b0, r40);
                r41 = Vector512.FusedMultiplyAdd(s, b1, r41);
                s = Vector512.Create(Unsafe.Add(ref aRow5, kk));
                r50 = Vector512.FusedMultiplyAdd(s, b0, r50);
                r51 = Vector512.FusedMultiplyAdd(s, b1, r51);
                s = Vector512.Create(Unsafe.Add(ref aRow6, kk));
                r60 = Vector512.FusedMultiplyAdd(s, b0, r60);
                r61 = Vector512.FusedMultiplyAdd(s, b1, r61);
                s = Vector512.Create(Unsafe.Add(ref aRow7, kk));
                r70 = Vector512.FusedMultiplyAdd(s, b0, r70);
                r71 = Vector512.FusedMultiplyAdd(s, b1, r71);
                s = Vector512.Create(Unsafe.Add(ref aRow8, kk));
                r80 = Vector512.FusedMultiplyAdd(s, b0, r80);
                r81 = Vector512.FusedMultiplyAdd(s, b1, r81);
                s = Vector512.Create(Unsafe.Add(ref aRow9, kk));
                r90 = Vector512.FusedMultiplyAdd(s, b0, r90);
                r91 = Vector512.FusedMultiplyAdd(s, b1, r91);
                s = Vector512.Create(Unsafe.Add(ref aRow10, kk));
                r100 = Vector512.FusedMultiplyAdd(s, b0, r100);
                r101 = Vector512.FusedMultiplyAdd(s, b1, r101);
                s = Vector512.Create(Unsafe.Add(ref aRow11, kk));
                r110 = Vector512.FusedMultiplyAdd(s, b0, r110);
                r111 = Vector512.FusedMultiplyAdd(s, b1, r111);
            }

            Store512(ref cRef, c0 + 0 * ldc + column, r00, r01, first);
            Store512(ref cRef, c0 + 1 * ldc + column, r10, r11, first);
            Store512(ref cRef, c0 + 2 * ldc + column, r20, r21, first);
            Store512(ref cRef, c0 + 3 * ldc + column, r30, r31, first);
            Store512(ref cRef, c0 + 4 * ldc + column, r40, r41, first);
            Store512(ref cRef, c0 + 5 * ldc + column, r50, r51, first);
            Store512(ref cRef, c0 + 6 * ldc + column, r60, r61, first);
            Store512(ref cRef, c0 + 7 * ldc + column, r70, r71, first);
            Store512(ref cRef, c0 + 8 * ldc + column, r80, r81, first);
            Store512(ref cRef, c0 + 9 * ldc + column, r90, r91, first);
            Store512(ref cRef, c0 + 10 * ldc + column, r100, r101, first);
            Store512(ref cRef, c0 + 11 * ldc + column, r110, r111, first);
        }

        // An odd trailing block is handled here rather than delegated: the narrower kernels
        // index packed blocks from zero, so handing them the last block would read the first.
        if ((blocks & 1) != 0)
        {
            int lastBlock = blocks - 1;
            int p0 = BlockOffset(lastBlock, countK);
            int column = lastBlock * BlockWidth;

            Vector512<float> t0 = Vector512<float>.Zero;
            Vector512<float> t1 = Vector512<float>.Zero;
            Vector512<float> t2 = Vector512<float>.Zero;
            Vector512<float> t3 = Vector512<float>.Zero;
            Vector512<float> t4 = Vector512<float>.Zero;
            Vector512<float> t5 = Vector512<float>.Zero;
            Vector512<float> t6 = Vector512<float>.Zero;
            Vector512<float> t7 = Vector512<float>.Zero;
            Vector512<float> t8 = Vector512<float>.Zero;
            Vector512<float> t9 = Vector512<float>.Zero;
            Vector512<float> t10 = Vector512<float>.Zero;
            Vector512<float> t11 = Vector512<float>.Zero;

            for (int kk = 0; kk < countK; kk++)
            {
                var b0 = Vector512.LoadUnsafe(ref pRef, (nuint)(p0 + kk * BlockWidth));
                Vector512<float> s;
                s = Vector512.Create(Unsafe.Add(ref aRow0, kk));
                t0 = Vector512.FusedMultiplyAdd(s, b0, t0);
                s = Vector512.Create(Unsafe.Add(ref aRow1, kk));
                t1 = Vector512.FusedMultiplyAdd(s, b0, t1);
                s = Vector512.Create(Unsafe.Add(ref aRow2, kk));
                t2 = Vector512.FusedMultiplyAdd(s, b0, t2);
                s = Vector512.Create(Unsafe.Add(ref aRow3, kk));
                t3 = Vector512.FusedMultiplyAdd(s, b0, t3);
                s = Vector512.Create(Unsafe.Add(ref aRow4, kk));
                t4 = Vector512.FusedMultiplyAdd(s, b0, t4);
                s = Vector512.Create(Unsafe.Add(ref aRow5, kk));
                t5 = Vector512.FusedMultiplyAdd(s, b0, t5);
                s = Vector512.Create(Unsafe.Add(ref aRow6, kk));
                t6 = Vector512.FusedMultiplyAdd(s, b0, t6);
                s = Vector512.Create(Unsafe.Add(ref aRow7, kk));
                t7 = Vector512.FusedMultiplyAdd(s, b0, t7);
                s = Vector512.Create(Unsafe.Add(ref aRow8, kk));
                t8 = Vector512.FusedMultiplyAdd(s, b0, t8);
                s = Vector512.Create(Unsafe.Add(ref aRow9, kk));
                t9 = Vector512.FusedMultiplyAdd(s, b0, t9);
                s = Vector512.Create(Unsafe.Add(ref aRow10, kk));
                t10 = Vector512.FusedMultiplyAdd(s, b0, t10);
                s = Vector512.Create(Unsafe.Add(ref aRow11, kk));
                t11 = Vector512.FusedMultiplyAdd(s, b0, t11);
            }

            StoreOne512(ref cRef, c0 + 0 * ldc + column, t0, first);
            StoreOne512(ref cRef, c0 + 1 * ldc + column, t1, first);
            StoreOne512(ref cRef, c0 + 2 * ldc + column, t2, first);
            StoreOne512(ref cRef, c0 + 3 * ldc + column, t3, first);
            StoreOne512(ref cRef, c0 + 4 * ldc + column, t4, first);
            StoreOne512(ref cRef, c0 + 5 * ldc + column, t5, first);
            StoreOne512(ref cRef, c0 + 6 * ldc + column, t6, first);
            StoreOne512(ref cRef, c0 + 7 * ldc + column, t7, first);
            StoreOne512(ref cRef, c0 + 8 * ldc + column, t8, first);
            StoreOne512(ref cRef, c0 + 9 * ldc + column, t9, first);
            StoreOne512(ref cRef, c0 + 10 * ldc + column, t10, first);
            StoreOne512(ref cRef, c0 + 11 * ldc + column, t11, first);
        }
    }

    /// <summary>
    /// Up to six output rows against two 16-float packed blocks, in 512-bit vectors.
    /// <para>
    /// <paramref name="liveRows"/> exists so the leftover rows of a range that does not divide
    /// by the block size come through here rather than one at a time. A single-row kernel
    /// re-reads the entire packed panel to produce one row, and does one multiply-add per
    /// operand load; four such rows after a 64-row layer's five full blocks cost as much as
    /// forty rows of this kernel, and dropped that shape from 228 to 142 GFLOP/s. Rows past
    /// the live count read a repeat of the last live row and are simply not stored.
    /// </para>
    /// </summary>
    private static void SixRows512(
        ReadOnlySpan<float> a, float[] packed, int packedOffset, Span<float> c,
        int k, int ldc, int i0, int pc, int countK, int jc, int countN, bool first, int liveRows)
    {
        ref float aRef = ref MemoryMarshal.GetReference(a);
        ref float pRef = ref Unsafe.Add(ref MemoryMarshal.GetArrayDataReference(packed), packedOffset);
        ref float cRef = ref MemoryMarshal.GetReference(c);

        int a0 = i0 * k + pc;
        int c0 = i0 * ldc + jc;
        int blocks = (countN + BlockWidth - 1) / BlockWidth;
        int block = 0;

        // One cursor per row; see the note in the twelve-row kernel. A cursor past the live
        // rows repeats the last live one, which keeps every read inside the operand.
        ref float aRow0 = ref Unsafe.Add(ref aRef, a0);
        ref float aRow1 = ref Unsafe.Add(ref aRow0, liveRows > 1 ? k : 0);
        ref float aRow2 = ref Unsafe.Add(ref aRow1, liveRows > 2 ? k : 0);
        ref float aRow3 = ref Unsafe.Add(ref aRow2, liveRows > 3 ? k : 0);
        ref float aRow4 = ref Unsafe.Add(ref aRow3, liveRows > 4 ? k : 0);
        ref float aRow5 = ref Unsafe.Add(ref aRow4, liveRows > 5 ? k : 0);

        for (; block + 2 <= blocks; block += 2)
        {
            int p0 = BlockOffset(block, countK);
            int p1 = BlockOffset(block + 1, countK);
            int column = block * BlockWidth;

            Vector512<float> r00 = Vector512<float>.Zero, r01 = Vector512<float>.Zero;
            Vector512<float> r10 = Vector512<float>.Zero, r11 = Vector512<float>.Zero;
            Vector512<float> r20 = Vector512<float>.Zero, r21 = Vector512<float>.Zero;
            Vector512<float> r30 = Vector512<float>.Zero, r31 = Vector512<float>.Zero;
            Vector512<float> r40 = Vector512<float>.Zero, r41 = Vector512<float>.Zero;
            Vector512<float> r50 = Vector512<float>.Zero, r51 = Vector512<float>.Zero;

            for (int kk = 0; kk < countK; kk++)
            {
                var b0 = Vector512.LoadUnsafe(ref pRef, (nuint)(p0 + kk * BlockWidth));
                var b1 = Vector512.LoadUnsafe(ref pRef, (nuint)(p1 + kk * BlockWidth));

                // One broadcast register, reused down the rows — the shape that keeps twelve
                // accumulators plus two operands inside the register file.
                var s = Vector512.Create(Unsafe.Add(ref aRow0, kk));
                r00 = Vector512.FusedMultiplyAdd(s, b0, r00);
                r01 = Vector512.FusedMultiplyAdd(s, b1, r01);
                s = Vector512.Create(Unsafe.Add(ref aRow1, kk));
                r10 = Vector512.FusedMultiplyAdd(s, b0, r10);
                r11 = Vector512.FusedMultiplyAdd(s, b1, r11);
                s = Vector512.Create(Unsafe.Add(ref aRow2, kk));
                r20 = Vector512.FusedMultiplyAdd(s, b0, r20);
                r21 = Vector512.FusedMultiplyAdd(s, b1, r21);
                s = Vector512.Create(Unsafe.Add(ref aRow3, kk));
                r30 = Vector512.FusedMultiplyAdd(s, b0, r30);
                r31 = Vector512.FusedMultiplyAdd(s, b1, r31);
                s = Vector512.Create(Unsafe.Add(ref aRow4, kk));
                r40 = Vector512.FusedMultiplyAdd(s, b0, r40);
                r41 = Vector512.FusedMultiplyAdd(s, b1, r41);
                s = Vector512.Create(Unsafe.Add(ref aRow5, kk));
                r50 = Vector512.FusedMultiplyAdd(s, b0, r50);
                r51 = Vector512.FusedMultiplyAdd(s, b1, r51);
            }

            Store512(ref cRef, c0 + 0 * ldc + column, r00, r01, first);
            if (liveRows > 1) Store512(ref cRef, c0 + 1 * ldc + column, r10, r11, first);
            if (liveRows > 2) Store512(ref cRef, c0 + 2 * ldc + column, r20, r21, first);
            if (liveRows > 3) Store512(ref cRef, c0 + 3 * ldc + column, r30, r31, first);
            if (liveRows > 4) Store512(ref cRef, c0 + 4 * ldc + column, r40, r41, first);
            if (liveRows > 5) Store512(ref cRef, c0 + 5 * ldc + column, r50, r51, first);
        }

        for (; block < blocks; block++)
        {
            int p0 = BlockOffset(block, countK);
            int column = block * BlockWidth;

            Vector512<float> r0 = Vector512<float>.Zero, r1 = Vector512<float>.Zero;
            Vector512<float> r2 = Vector512<float>.Zero, r3 = Vector512<float>.Zero;
            Vector512<float> r4 = Vector512<float>.Zero, r5 = Vector512<float>.Zero;

            for (int kk = 0; kk < countK; kk++)
            {
                var b0 = Vector512.LoadUnsafe(ref pRef, (nuint)(p0 + kk * BlockWidth));
                r0 = Vector512.FusedMultiplyAdd(Vector512.Create(Unsafe.Add(ref aRow0, kk)), b0, r0);
                r1 = Vector512.FusedMultiplyAdd(Vector512.Create(Unsafe.Add(ref aRow1, kk)), b0, r1);
                r2 = Vector512.FusedMultiplyAdd(Vector512.Create(Unsafe.Add(ref aRow2, kk)), b0, r2);
                r3 = Vector512.FusedMultiplyAdd(Vector512.Create(Unsafe.Add(ref aRow3, kk)), b0, r3);
                r4 = Vector512.FusedMultiplyAdd(Vector512.Create(Unsafe.Add(ref aRow4, kk)), b0, r4);
                r5 = Vector512.FusedMultiplyAdd(Vector512.Create(Unsafe.Add(ref aRow5, kk)), b0, r5);
            }

            StoreOne512(ref cRef, c0 + 0 * ldc + column, r0, first);
            if (liveRows > 1) StoreOne512(ref cRef, c0 + 1 * ldc + column, r1, first);
            if (liveRows > 2) StoreOne512(ref cRef, c0 + 2 * ldc + column, r2, first);
            if (liveRows > 3) StoreOne512(ref cRef, c0 + 3 * ldc + column, r3, first);
            if (liveRows > 4) StoreOne512(ref cRef, c0 + 4 * ldc + column, r4, first);
            if (liveRows > 5) StoreOne512(ref cRef, c0 + 5 * ldc + column, r5, first);
        }
    }

    /// <summary>
    /// Write two accumulators back, adding to what is there unless this is the first depth
    /// panel.
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static void Store512(ref float cRef, int offset, Vector512<float> v0, Vector512<float> v1, bool first)
    {
        StoreOne512(ref cRef, offset, v0, first);
        StoreOne512(ref cRef, offset + BlockWidth, v1, first);
    }

    /// <summary>
    /// Write one accumulator back. That this is inlined, and that it handles only whole
    /// blocks, decides whether the kernel's accumulators live in registers or in memory —
    /// which is worth more than everything else in this file put together.
    /// <para>
    /// A <c>Vector512</c> handed by value to a real call is passed by hidden reference, so the
    /// caller's local must be materialised on the stack and becomes address-exposed. RyuJIT
    /// then writes it through to memory after <em>every</em> multiply-add: twenty-three extra
    /// 64-byte stores per iteration of the innermost loop, to maintain a stack copy that
    /// nothing ever reads. A single partial-tail branch anywhere in here was enough to cause
    /// it, however cold that branch was, so the ragged columns are not handled here at all.
    /// </para>
    /// </summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static void StoreOne512(ref float cRef, int offset, Vector512<float> v, bool first)
    {
        if (!first) v += Vector512.LoadUnsafe(ref cRef, (nuint)offset);
        v.StoreUnsafe(ref cRef, (nuint)offset);
    }

    /// <summary>
    /// Six output rows against one 16-float packed block, using the portable vector width.
    /// On a 256-bit target the block is consumed as two vectors, giving the same twelve
    /// accumulators.
    /// </summary>
    private static void SixRowsVector(
        ReadOnlySpan<float> a, float[] packed, int packedOffset, Span<float> c,
        int k, int ldc, int i0, int pc, int countK, int jc, int countN, bool first, int liveRows)
    {
        int width = Vector<float>.Count;
        int halves = BlockWidth / width;          // 2 on AVX2, 1 where the vector is already 16 wide
        ref float aRef = ref MemoryMarshal.GetReference(a);
        ref float pRef = ref Unsafe.Add(ref MemoryMarshal.GetArrayDataReference(packed), packedOffset);
        ref float cRef = ref MemoryMarshal.GetReference(c);

        int a0 = i0 * k + pc;
        int c0 = i0 * ldc + jc;
        int blocks = (countN + BlockWidth - 1) / BlockWidth;

        // One cursor per row, clamped past the live rows; see SixRows512.
        ref float aRow0 = ref Unsafe.Add(ref aRef, a0);
        ref float aRow1 = ref Unsafe.Add(ref aRow0, liveRows > 1 ? k : 0);
        ref float aRow2 = ref Unsafe.Add(ref aRow1, liveRows > 2 ? k : 0);
        ref float aRow3 = ref Unsafe.Add(ref aRow2, liveRows > 3 ? k : 0);
        ref float aRow4 = ref Unsafe.Add(ref aRow3, liveRows > 4 ? k : 0);
        ref float aRow5 = ref Unsafe.Add(ref aRow4, liveRows > 5 ? k : 0);

        for (int block = 0; block < blocks; block++)
        {
            int p0 = BlockOffset(block, countK);
            int column = block * BlockWidth;

            for (int half = 0; half < halves; half++)
            {
                int lane = half * width;
                Vector<float> r0 = default, r1 = default, r2 = default;
                Vector<float> r3 = default, r4 = default, r5 = default;

                for (int kk = 0; kk < countK; kk++)
                {
                    var bv = Vector.LoadUnsafe(ref pRef, (nuint)(p0 + kk * BlockWidth + lane));
                    r0 = Vector.FusedMultiplyAdd(new Vector<float>(Unsafe.Add(ref aRow0, kk)), bv, r0);
                    r1 = Vector.FusedMultiplyAdd(new Vector<float>(Unsafe.Add(ref aRow1, kk)), bv, r1);
                    r2 = Vector.FusedMultiplyAdd(new Vector<float>(Unsafe.Add(ref aRow2, kk)), bv, r2);
                    r3 = Vector.FusedMultiplyAdd(new Vector<float>(Unsafe.Add(ref aRow3, kk)), bv, r3);
                    r4 = Vector.FusedMultiplyAdd(new Vector<float>(Unsafe.Add(ref aRow4, kk)), bv, r4);
                    r5 = Vector.FusedMultiplyAdd(new Vector<float>(Unsafe.Add(ref aRow5, kk)), bv, r5);
                }

                int at = column + lane;
                StoreVector(ref cRef, c0 + 0 * ldc + at, r0, first);
                if (liveRows > 1) StoreVector(ref cRef, c0 + 1 * ldc + at, r1, first);
                if (liveRows > 2) StoreVector(ref cRef, c0 + 2 * ldc + at, r2, first);
                if (liveRows > 3) StoreVector(ref cRef, c0 + 3 * ldc + at, r3, first);
                if (liveRows > 4) StoreVector(ref cRef, c0 + 4 * ldc + at, r4, first);
                if (liveRows > 5) StoreVector(ref cRef, c0 + 5 * ldc + at, r5, first);
            }
        }
    }

    /// <inheritdoc cref="StoreOne512"/>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    private static void StoreVector(ref float cRef, int offset, Vector<float> v, bool first)
    {
        if (!first) v += Vector.LoadUnsafe(ref cRef, (nuint)offset);
        v.StoreUnsafe(ref cRef, (nuint)offset);
    }
}
