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

    /// <summary>
    /// <see cref="System.Threading.Tasks.ParallelOptions"/> carrying the ambient token, for the
    /// <c>Parallel.For</c> loops of the inference runtime: a cancelled loop stops handing out
    /// iterations and surfaces as an <see cref="OperationCanceledException"/>.
    /// </summary>
    public static ParallelOptions ParallelOptions => new() { CancellationToken = Ambient.Value };

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
    /// <see cref="Stream.CopyTo(Stream)"/> that notices a cancellation between chunks, so reading a
    /// large decompressed entry is interruptible instead of running to the end of the entry.
    /// </summary>
    public static void CopyTo(Stream source, Stream destination)
    {
        CopyBounded(source, destination, long.MaxValue);
    }

    /// <summary>
    /// Copy at most <paramref name="maxBytes"/> (plus one byte, enough to tell the limit was
    /// crossed), polling for cancellation between chunks.
    /// </summary>
    /// <returns>False when the source held more than <paramref name="maxBytes"/>.</returns>
    public static bool CopyBounded(Stream source, Stream destination, long maxBytes)
    {
        var buffer = new byte[81920];
        long copied = 0;
        int read;
        while ((read = source.Read(buffer, 0, buffer.Length)) > 0)
        {
            Ambient.Value.ThrowIfCancellationRequested();
            copied += read;
            if (copied > maxBytes) return false;
            destination.Write(buffer, 0, read);
        }
        return true;
    }

    /// <summary>
    /// <c>XDocument.Parse</c> that polls for cancellation as the parser pulls text, so parsing one
    /// very large part is interruptible. Same reader settings <c>XDocument.Parse</c> uses.
    /// </summary>
    public static System.Xml.Linq.XDocument ParseXml(
        string xml, System.Xml.Linq.LoadOptions options = System.Xml.Linq.LoadOptions.None)
    {
        var settings = new System.Xml.XmlReaderSettings
        {
            DtdProcessing = System.Xml.DtdProcessing.Parse,
            IgnoreWhitespace = (options & System.Xml.Linq.LoadOptions.PreserveWhitespace) == 0,
        };
        using var reader = System.Xml.XmlReader.Create(new PollingTextReader(xml), settings);
        return System.Xml.Linq.XDocument.Load(reader, options);
    }

    /// <summary>
    /// A read-only view of <paramref name="stream"/> that polls for cancellation on every read, for
    /// handing to a parser that pulls from a stream (decompression included) on its own.
    /// </summary>
    public static Stream Poll(Stream stream) => new PollingStream(stream);

    private sealed class PollingStream : Stream
    {
        private readonly Stream _inner;
        public PollingStream(Stream inner) => _inner = inner;
        public override bool CanRead => true;
        public override bool CanSeek => _inner.CanSeek;
        public override bool CanWrite => false;
        public override long Length => _inner.Length;
        public override long Position { get => _inner.Position; set => _inner.Position = value; }
        public override int Read(byte[] buffer, int offset, int count)
        {
            Ambient.Value.ThrowIfCancellationRequested();
            return _inner.Read(buffer, offset, count);
        }
        public override int Read(Span<byte> buffer)
        {
            Ambient.Value.ThrowIfCancellationRequested();
            return _inner.Read(buffer);
        }
        public override long Seek(long offset, SeekOrigin origin) => _inner.Seek(offset, origin);
        public override void Flush() { }
        public override void SetLength(long value) => throw new NotSupportedException();
        public override void Write(byte[] buffer, int offset, int count) => throw new NotSupportedException();
        protected override void Dispose(bool disposing)
        {
            if (disposing) _inner.Dispose();
            base.Dispose(disposing);
        }
    }

    /// <summary>A <see cref="StringReader"/> that polls the ambient token on every chunk it hands out.</summary>
    private sealed class PollingTextReader : TextReader
    {
        private readonly StringReader _inner;
        public PollingTextReader(string text) => _inner = new StringReader(text);
        public override int Peek() => _inner.Peek();
        public override int Read() => _inner.Read();
        public override int Read(char[] buffer, int index, int count)
        {
            Ambient.Value.ThrowIfCancellationRequested();
            return _inner.Read(buffer, index, count);
        }
        public override int Read(Span<char> buffer)
        {
            Ambient.Value.ThrowIfCancellationRequested();
            return _inner.Read(buffer);
        }
        protected override void Dispose(bool disposing)
        {
            if (disposing) _inner.Dispose();
            base.Dispose(disposing);
        }
    }

    /// <summary><see cref="TextReader.ReadToEnd"/>, polling for cancellation between chunks.</summary>
    public static string ReadToEnd(TextReader reader)
    {
        var sb = new System.Text.StringBuilder();
        var buffer = new char[32768];
        int read;
        while ((read = reader.Read(buffer, 0, buffer.Length)) > 0)
        {
            Ambient.Value.ThrowIfCancellationRequested();
            sb.Append(buffer, 0, read);
        }
        return sb.ToString();
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
