using System.IO.Compression;
using System.Text;
using XRay.Content.Core;
using XRay.Content.Internal.Pdf;
using Xunit;

namespace XRay.Content.Tests;

/// <summary>The building blocks of <see cref="Cancel"/>: scopes, polling, and the cancellable readers.</summary>
public class CancelUtilityTests
{
    [Fact]
    public void ScopesNestAndRestoreTheOuterToken()
    {
        using var outer = new CancellationTokenSource();
        using var inner = new CancellationTokenSource();

        using (Cancel.Begin(outer.Token))
        {
            Assert.Equal(outer.Token, Cancel.Token);

            // A nested call with a token that cannot be cancelled leaves the outer one in force.
            using (Cancel.Begin(CancellationToken.None))
                Assert.Equal(outer.Token, Cancel.Token);

            // Two live tokens are linked: either one cancels the work inside.
            using (Cancel.Begin(inner.Token))
            {
                Assert.False(Cancel.Requested);
                outer.Cancel();
                Assert.True(Cancel.Requested);
            }
            Assert.Equal(outer.Token, Cancel.Token);
        }
        Assert.False(Cancel.Token.CanBeCanceled);
    }

    [Fact]
    public void TheNotCancellingFilterStepsAsideOnlyWhenCancelled()
    {
        using var cts = new CancellationTokenSource();
        using var scope = Cancel.Begin(cts.Token);

        int Swallow()
        {
            try { throw new InvalidOperationException(); }
            catch (Exception) when (Cancel.NotCancelling) { return 1; }
        }

        Assert.Equal(1, Swallow());
        cts.Cancel();
        Assert.Throws<InvalidOperationException>(() => Swallow());
    }

    [Fact]
    public void EveryPollsOnlyOnTheMask()
    {
        using var cts = new CancellationTokenSource();
        cts.Cancel();
        using var scope = Cancel.Begin(cts.Token);

        Cancel.Every(1, 0xFF);                                              // not on the mask
        Assert.Throws<OperationCanceledException>(() => Cancel.Every(256, 0xFF));
        Assert.Throws<OperationCanceledException>(() => Cancel.Every(0, 0xFF));
    }

    [Fact]
    public void CopyBoundedStopsAtTheCeilingAndReportsIt()
    {
        using var source = new MemoryStream(new byte[300_000]);
        using var sink = new MemoryStream();

        Assert.False(Cancel.CopyBounded(source, sink, 100_000));
        Assert.True(sink.Length <= 100_000);

        source.Position = 0;
        sink.SetLength(0);
        Assert.True(Cancel.CopyBounded(source, sink, 300_000));
        Assert.Equal(300_000, sink.Length);
    }

    [Fact]
    public void CopyToAndReadToEndStopWhenCancelled()
    {
        using var cts = new CancellationTokenSource();
        using var scope = Cancel.Begin(cts.Token);
        cts.Cancel();

        Assert.Throws<OperationCanceledException>(() => Cancel.CopyTo(new MemoryStream(new byte[10]), Stream.Null));
        Assert.Throws<OperationCanceledException>(() => Cancel.ReadToEnd(new StringReader("text")));
        Assert.Throws<OperationCanceledException>(() => Cancel.ParseXml("<a/>"));
        var polled = Cancel.Poll(new MemoryStream(new byte[10]));
        Assert.Throws<OperationCanceledException>(() => polled.ReadByte());
    }

    [Fact]
    public void ParseXmlMatchesXDocumentParse()
    {
        const string xml = "<root a=\"1\">\n  <child>text &amp; more</child>\n  <!-- c -->\n</root>";

        Assert.Equal(
            System.Xml.Linq.XDocument.Parse(xml).ToString(),
            Cancel.ParseXml(xml).ToString());
        Assert.Equal(
            System.Xml.Linq.XDocument.Parse(xml, System.Xml.Linq.LoadOptions.PreserveWhitespace).ToString(),
            Cancel.ParseXml(xml, System.Xml.Linq.LoadOptions.PreserveWhitespace).ToString());
    }

    [Fact]
    public void FlateDecodingIsBoundedByTheCeiling()
    {
        byte[] zlib;
        using (var ms = new MemoryStream())
        {
            using (var z = new ZLibStream(ms, CompressionLevel.SmallestSize, leaveOpen: true)) z.Write(new byte[2_000_000]);
            zlib = ms.ToArray();
        }

        Assert.Equal(2_000_000, PdfFilters.Inflate(zlib).Length);
        Assert.Throws<SecurityException>(() => PdfFilters.Inflate(zlib, 100_000));
        Assert.Equal(2_000_000, PdfFilters.Inflate(zlib, 2_000_000).Length);
    }

    [Fact]
    public async Task AWorkerThreadInheritsTheAmbientToken()
    {
        using var cts = new CancellationTokenSource();
        using var scope = Cancel.Begin(cts.Token);

        var seen = await Task.Run(() => Cancel.Token);
        Assert.Equal(cts.Token, seen);

        // Parallel loops carry it too, and stop handing out iterations once it is cancelled.
        cts.Cancel();
        Assert.ThrowsAny<OperationCanceledException>(
            () => Parallel.For(0, 1000, Cancel.ParallelOptions, _ => { }));
    }
}
