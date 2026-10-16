namespace XRay.Content.Core;

/// <summary>
/// The ambient cancellation token of the extraction running on this logical call chain.
/// </summary>
/// <remarks>
/// <para>
/// The public entry points (<see cref="Extractor.Extract(ExtractInput, ExtractionConfig, CancellationToken)"/>
/// and <see cref="Extractor.ExtractAsync(ExtractInput, ExtractionConfig, CancellationToken)"/>)
/// take the token and install it here for the duration of the call. Every extractor, reader and
/// pipeline stage then polls it at the loop boundaries that bound latency — a page, a part, a
/// record, an archive entry, a row strip — instead of each signature growing a parameter. An
/// <see cref="AsyncLocal{T}"/> carries it, so work moved to the thread pool
/// (<see cref="Parallel"/> loops in the ONNX runtime, <see cref="Task.Run(Action)"/>) sees the same
/// token without being told about it.
/// </para>
/// <para>
/// The contract is the family's: cancellation surfaces as <see cref="OperationCanceledException"/>
/// and is never turned into a warning, an error item or a package exception. Two things keep that
/// true. Every best-effort <c>catch</c> in the library carries <see cref="NotCancelling"/> as a
/// filter, so a swallow-and-continue handler cannot eat the cancellation; and
/// <see cref="Extractor"/> rethrows it ahead of its own error mapping.
/// </para>
/// <para>
/// A caller that never supplies a token pays one <see cref="AsyncLocal{T}"/> read per poll, and a
/// poll is made every few hundred iterations in a hot loop, not every iteration.
/// </para>
/// </remarks>
internal static class Cancel
{
    private static readonly AsyncLocal<CancellationToken> Ambient = new();

    /// <summary>The token in force, or <see cref="CancellationToken.None"/>.</summary>
    public static CancellationToken Token => Ambient.Value;

    /// <summary>Whether the ambient token has been cancelled.</summary>
    public static bool Requested => Ambient.Value.IsCancellationRequested;

    /// <summary>
    /// Filter for a best-effort <c>catch</c>: <c>catch (Exception) when (Cancel.NotCancelling)</c>.
    /// While a cancellation is in flight the handler steps aside and the exception — the
    /// <see cref="OperationCanceledException"/> itself, or whatever the interrupted work threw on
    /// the way out — propagates to <see cref="Extractor"/>, which reports it as a cancellation.
    /// </summary>
    public static bool NotCancelling => !Ambient.Value.IsCancellationRequested;

    /// <summary>Throw <see cref="OperationCanceledException"/> if the ambient token is cancelled.</summary>
    public static void ThrowIfRequested() => Ambient.Value.ThrowIfCancellationRequested();

    /// <summary>
    /// Poll once every <paramref name="mask"/>+1 iterations: <c>Cancel.Every(i, 0xFF)</c>. The mask
    /// must be a power of two minus one.
    /// </summary>
    public static void Every(int iteration, int mask = 0x3F)
    {
        if ((iteration & mask) == 0) Ambient.Value.ThrowIfCancellationRequested();
    }

    /// <summary>The same, for a 64-bit counter.</summary>
    public static void Every(long iteration, long mask)
    {
        if ((iteration & mask) == 0) Ambient.Value.ThrowIfCancellationRequested();
    }

    /// <summary>
    /// Install <paramref name="token"/> for the extent of the returned scope. A token that cannot
    /// be cancelled leaves whatever is in force alone, so a nested call that passes
    /// <c>default</c> (an archive extracting its children, say) keeps honouring the outer one;
    /// two live tokens are linked so either cancels the call.
    /// </summary>
    public static Scope Begin(CancellationToken token)
    {
        var previous = Ambient.Value;
        if (!token.CanBeCanceled) return new Scope(previous, null, installed: false);
        if (!previous.CanBeCanceled || previous == token)
        {
            Ambient.Value = token;
            return new Scope(previous, null, installed: true);
        }
        var linked = CancellationTokenSource.CreateLinkedTokenSource(previous, token);
        Ambient.Value = linked.Token;
        return new Scope(previous, linked, installed: true);
    }

    internal readonly struct Scope : IDisposable
    {
        private readonly CancellationToken _previous;
        private readonly CancellationTokenSource? _linked;
        private readonly bool _installed;

        public Scope(CancellationToken previous, CancellationTokenSource? linked, bool installed)
        {
            _previous = previous;
            _linked = linked;
            _installed = installed;
        }

        public void Dispose()
        {
            if (_installed) Ambient.Value = _previous;
            _linked?.Dispose();
        }
    }
}
