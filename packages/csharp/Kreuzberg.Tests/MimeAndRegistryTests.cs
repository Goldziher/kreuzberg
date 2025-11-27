using System;
using System.IO;
using System.Linq;
using System.Text;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

public class MimeAndRegistryTests
{
    public MimeAndRegistryTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();
    }

    [Fact]
    public void DetectMimeTypeFromPath_ReturnsPdf()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"mime-{Guid.NewGuid():N}.pdf");
        File.WriteAllText(tempPath, "%PDF-1.7\n%");

        try
        {
            var mime = KreuzbergClient.DetectMimeTypeFromPath(tempPath);
            Assert.Equal("application/pdf", mime);
        }
        finally
        {
            if (File.Exists(tempPath))
            {
                File.Delete(tempPath);
            }
        }
    }

    [Fact]
    public void DetectMimeTypeFromBytes_ReturnsPdf()
    {
        var bytes = Encoding.ASCII.GetBytes("%PDF-1.7\n%");
        var mime = KreuzbergClient.DetectMimeType(bytes);
        Assert.Equal("application/pdf", mime);
    }

    [Fact]
    public void GetExtensionsForMime_ReturnsPdf()
    {
        var extensions = KreuzbergClient.GetExtensionsForMime("application/pdf");
        Assert.Contains(extensions, ext => string.Equals(ext, "pdf", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void ListDocumentExtractors_ReturnsDefaults()
    {
        var extractors = KreuzbergClient.ListDocumentExtractors();
        Assert.NotNull(extractors);
        Assert.NotEmpty(extractors);
    }

    [Fact]
    public void RegisterAndUnregisterOcrBackend_RoundTrips()
    {
        var name = $"dummy-ocr-{Guid.NewGuid():N}";
        var backend = new DummyOcrBackend(name);

        KreuzbergClient.RegisterOcrBackend(backend);
        var cleanupNeeded = true;
        try
        {
            var registered = KreuzbergClient.ListOcrBackends();
            Assert.Contains(name, registered);

            KreuzbergClient.UnregisterOcrBackend(name);
            cleanupNeeded = false;

            var after = KreuzbergClient.ListOcrBackends();
            Assert.DoesNotContain(name, after);
        }
        finally
        {
            if (cleanupNeeded)
            {
                try
                {
                    KreuzbergClient.UnregisterOcrBackend(name);
                }
                catch
                {
                    // best-effort cleanup
                }
            }
        }
    }

    [Fact]
    public void ListPostProcessorsAndValidators_NotEmpty()
    {
        Assert.NotEmpty(KreuzbergClient.ListPostProcessors());
        Assert.NotEmpty(KreuzbergClient.ListValidators());
    }

    private sealed class DummyOcrBackend : IOcrBackend
    {
        public DummyOcrBackend(string name)
        {
            Name = name;
        }

        public string Name { get; }

        public int Priority => 0;

        public string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config)
        {
            return $"ok:{imageBytes.Length}:{config?.Backend}";
        }
    }
}
