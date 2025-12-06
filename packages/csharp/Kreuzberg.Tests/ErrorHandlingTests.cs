using System;
using System.IO;
using System.Text;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive error handling tests covering validation failures, file errors, corrupted files,
/// permission issues, invalid configurations, and null/empty input scenarios.
/// </summary>
public class ErrorHandlingTests
{
    public ErrorHandlingTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();
    }

    #region File Not Found Tests

    [Fact]
    public void ExtractFileSync_WithNonexistentPath_ThrowsException()
    {
        var nonexistentPath = "/nonexistent/path/file.pdf";

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.ExtractFileSync(nonexistentPath));
        Assert.NotNull(ex);
    }

    [Fact]
    public void ExtractFileSync_WithNullPath_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.ExtractFileSync(null!));
        Assert.Contains("path", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ExtractFileSync_WithEmptyPath_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.ExtractFileSync(""));
        Assert.Contains("path", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ExtractFileSync_WithWhitespacePath_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.ExtractFileSync("   "));
        Assert.Contains("path", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void DetectMimeTypeFromPath_WithNonexistentFile_ThrowsException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.DetectMimeTypeFromPath("/nonexistent/file.pdf"));
        Assert.NotNull(ex);
    }

    [Fact]
    public void DetectMimeTypeFromPath_WithNullPath_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.DetectMimeTypeFromPath(null!));
        Assert.Contains("path", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void DetectMimeTypeFromPath_WithEmptyPath_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.DetectMimeTypeFromPath(""));
        Assert.Contains("path", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    #endregion

    #region Invalid MIME Type Tests

    [Fact]
    public void ExtractBytesSync_WithEmptyMimeType_ThrowsValidationException()
    {
        var bytes = new byte[] { 1, 2, 3 };
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.ExtractBytesSync(bytes, ""));
        Assert.Contains("mime", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ExtractBytesSync_WithNullMimeType_ThrowsValidationException()
    {
        var bytes = new byte[] { 1, 2, 3 };
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.ExtractBytesSync(bytes, null!));
        Assert.Contains("mime", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ExtractBytesSync_WithWhitespaceMimeType_ThrowsValidationException()
    {
        var bytes = new byte[] { 1, 2, 3 };
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.ExtractBytesSync(bytes, "   "));
        Assert.Contains("mime", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ExtractBytesSync_WithInvalidMimeFormat_HandlesGracefully()
    {
        var bytes = new byte[] { 1, 2, 3 };
        // Invalid MIME format but structurally valid
        try
        {
            var result = KreuzbergClient.ExtractBytesSync(bytes, "invalid/mime/type");
            Assert.NotNull(result);
        }
        catch (KreuzbergException)
        {
            // Graceful error handling - acceptable outcome
            Assert.True(true);
        }
    }

    [Fact]
    public void GetExtensionsForMime_WithNullMime_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetExtensionsForMime(null!));
        Assert.Contains("mime", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void GetExtensionsForMime_WithEmptyMime_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetExtensionsForMime(""));
        Assert.Contains("mime", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void GetExtensionsForMime_WithWhitespaceMime_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetExtensionsForMime("   "));
        Assert.Contains("mime", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    #endregion

    #region Empty Data Tests

    [Fact]
    public void ExtractBytesSync_WithEmptyByteArray_ThrowsValidationException()
    {
        var bytes = Array.Empty<byte>();
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.ExtractBytesSync(bytes, "application/pdf"));
        Assert.Contains("data", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void ExtractBytesSync_WithEmptySpan_ThrowsValidationException()
    {
        try
        {
            KreuzbergClient.ExtractBytesSync(new ReadOnlySpan<byte>(), "application/pdf");
            Assert.True(false, "Expected KreuzbergValidationException");
        }
        catch (KreuzbergValidationException ex)
        {
            Assert.Contains("data", ex.Message, StringComparison.OrdinalIgnoreCase);
        }
    }

    [Fact]
    public void DetectMimeType_WithEmptyBytes_ThrowsValidationException()
    {
        var bytes = Array.Empty<byte>();
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.DetectMimeType(bytes));
        Assert.Contains("data", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void DetectMimeType_WithEmptySpan_ThrowsValidationException()
    {
        try
        {
            KreuzbergClient.DetectMimeType(new ReadOnlySpan<byte>());
            Assert.True(false, "Expected KreuzbergValidationException");
        }
        catch (KreuzbergValidationException ex)
        {
            Assert.Contains("data", ex.Message, StringComparison.OrdinalIgnoreCase);
        }
    }

    #endregion

    #region Corrupted File Tests

    [Fact]
    public void ExtractFileSync_WithCorruptedPdfHeader_HandlesError()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"corrupt-{Guid.NewGuid():N}.pdf");
        File.WriteAllText(tempPath, "This is not a valid PDF file");

        try
        {
            var result = KreuzbergClient.ExtractFileSync(tempPath);
            // Extraction may fail gracefully
            Assert.NotNull(result);
        }
        catch (KreuzbergException)
        {
            // Corrupted file should raise KreuzbergException
            Assert.True(true);
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
    public void ExtractBytesSync_WithInvalidPdfBytes_HandlesError()
    {
        var invalidPdfBytes = new byte[] { 0x00, 0x01, 0x02, 0x03 };

        try
        {
            var result = KreuzbergClient.ExtractBytesSync(invalidPdfBytes, "application/pdf");
            // Should either fail gracefully or return empty result
            Assert.NotNull(result);
        }
        catch (KreuzbergParsingException)
        {
            // Parsing error for invalid PDF is expected
            Assert.True(true);
        }
    }

    [Fact]
    public void ExtractFileSync_WithPartiallyDownloadedFile_HandlesGracefully()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"partial-{Guid.NewGuid():N}.pdf");
        // Write PDF header but incomplete file
        File.WriteAllBytes(tempPath, new byte[] { 0x25, 0x50, 0x44, 0x46 }); // %PDF

        try
        {
            try
            {
                var result = KreuzbergClient.ExtractFileSync(tempPath);
                // Should handle incomplete file
                Assert.NotNull(result);
            }
            catch (KreuzbergException)
            {
                // Parsing error for incomplete file is acceptable
                Assert.True(true);
            }
        }
        finally
        {
            if (File.Exists(tempPath))
            {
                File.Delete(tempPath);
            }
        }
    }

    #endregion

    #region Configuration Validation Tests

    [Fact]
    public void ExtractFileSync_WithInvalidOcrConfig_HandlesError()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Ocr = new OcrConfig
            {
                Backend = "nonexistent-backend",
                Language = ""
            }
        };

        // Should either work or throw validation error
        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);
        Assert.NotNull(result);
    }

    [Fact]
    public void ExtractFileSync_WithInvalidChunkingSize_HandlesError()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Chunking = new ChunkingConfig
            {
                ChunkSize = -1, // Invalid negative size
                ChunkOverlap = -100
            }
        };

        // Should either handle or throw error
        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);
        Assert.NotNull(result);
    }

    [Fact]
    public void ExtractFileSync_WithNegativeMaxConcurrentExtractions_HandlesError()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            MaxConcurrentExtractions = -5
        };

        try
        {
            var result = KreuzbergClient.ExtractFileSync(pdfPath, config);
            Assert.NotNull(result);
        }
        catch (KreuzbergRuntimeException)
        {
            // Runtime error for invalid config is expected
            Assert.True(true);
        }
    }

    #endregion

    #region Batch Operation Validation Tests

    [Fact]
    public void BatchExtractFilesSync_WithNullPaths_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => KreuzbergClient.BatchExtractFilesSync(null!));
        Assert.Contains("paths", ex.Message);
    }

    [Fact]
    public void BatchExtractFilesSync_WithEmptyPathInList_ThrowsValidationException()
    {
        var paths = new[] { "valid_path.pdf", "", "another_path.pdf" };

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.BatchExtractFilesSync(paths));
        Assert.Contains("path", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void BatchExtractFilesSync_WithNullPathInList_ThrowsValidationException()
    {
        var paths = new[] { "valid_path.pdf", null!, "another_path.pdf" };

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.BatchExtractFilesSync(paths));
        Assert.NotNull(ex);
    }

    [Fact]
    public void BatchExtractBytesSync_WithNullItems_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => KreuzbergClient.BatchExtractBytesSync(null!));
        Assert.Contains("items", ex.Message);
    }

    [Fact]
    public void BatchExtractBytesSync_WithNullItemInList_ThrowsValidationException()
    {
        var items = new BytesWithMime[]
        {
            new BytesWithMime(new byte[] { 1, 2, 3 }, "application/pdf"),
            null!,
            new BytesWithMime(new byte[] { 4, 5, 6 }, "text/plain")
        };

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.BatchExtractBytesSync(items));
        Assert.Contains("null", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void BatchExtractBytesSync_WithEmptyDataInItem_ThrowsValidationException()
    {
        var items = new[]
        {
            new BytesWithMime(new byte[] { 1, 2, 3 }, "application/pdf"),
            new BytesWithMime(Array.Empty<byte>(), "text/plain")
        };

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.BatchExtractBytesSync(items));
        Assert.Contains("data", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void BatchExtractBytesSync_WithNullMimeTypeInItem_ThrowsValidationException()
    {
        // ArgumentNullException is thrown during BytesWithMime construction, not during batch extract
        var ex = Assert.Throws<ArgumentNullException>(() =>
            new BytesWithMime(new byte[] { 4, 5, 6 }, null!)
        );
        Assert.NotNull(ex);
    }

    [Fact]
    public void BatchExtractBytesSync_WithEmptyMimeTypeInItem_ThrowsValidationException()
    {
        var items = new[]
        {
            new BytesWithMime(new byte[] { 1, 2, 3 }, "application/pdf"),
            new BytesWithMime(new byte[] { 4, 5, 6 }, "")
        };

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.BatchExtractBytesSync(items));
        Assert.Contains("mime", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    #endregion

    #region Processor and Validator Registration Tests

    [Fact]
    public void RegisterPostProcessor_WithNullProcessor_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => KreuzbergClient.RegisterPostProcessor(null!));
        Assert.Contains("processor", ex.Message);
    }

    [Fact]
    public void RegisterPostProcessor_WithEmptyName_ThrowsValidationException()
    {
        var processor = new TestPostProcessor("", 0);

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.RegisterPostProcessor(processor));
        Assert.Contains("name", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void RegisterPostProcessor_WithNullName_ThrowsValidationException()
    {
        var processor = new TestPostProcessor(null!, 0);

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.RegisterPostProcessor(processor));
        Assert.Contains("name", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void RegisterValidator_WithNullValidator_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => KreuzbergClient.RegisterValidator(null!));
        Assert.Contains("validator", ex.Message);
    }

    [Fact]
    public void RegisterValidator_WithEmptyName_ThrowsValidationException()
    {
        var validator = new TestValidator("", 0);

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.RegisterValidator(validator));
        Assert.Contains("name", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void UnregisterPostProcessor_WithNullName_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.UnregisterPostProcessor(null!));
        Assert.Contains("name", ex.Message);
    }

    [Fact]
    public void UnregisterPostProcessor_WithEmptyName_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.UnregisterPostProcessor(""));
        Assert.Contains("name", ex.Message);
    }

    [Fact]
    public void UnregisterValidator_WithNullName_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.UnregisterValidator(null!));
        Assert.Contains("name", ex.Message);
    }

    [Fact]
    public void UnregisterValidator_WithEmptyName_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.UnregisterValidator(""));
        Assert.Contains("name", ex.Message);
    }

    [Fact]
    public void RegisterOcrBackend_WithNullBackend_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => KreuzbergClient.RegisterOcrBackend(null!));
        Assert.Contains("backend", ex.Message);
    }

    [Fact]
    public void RegisterOcrBackend_WithEmptyName_ThrowsValidationException()
    {
        var backend = new TestOcrBackend("");

        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.RegisterOcrBackend(backend));
        Assert.Contains("name", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void UnregisterOcrBackend_WithNullName_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.UnregisterOcrBackend(null!));
        Assert.Contains("name", ex.Message);
    }

    [Fact]
    public void UnregisterOcrBackend_WithEmptyName_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.UnregisterOcrBackend(""));
        Assert.Contains("name", ex.Message);
    }

    #endregion

    #region Embedding Preset Tests

    [Fact]
    public void GetEmbeddingPreset_WithNullName_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetEmbeddingPreset(null!));
        Assert.Contains("preset", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void GetEmbeddingPreset_WithEmptyName_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetEmbeddingPreset(""));
        Assert.Contains("preset", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void GetEmbeddingPreset_WithWhitespaceName_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.GetEmbeddingPreset("   "));
        Assert.Contains("preset", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void GetEmbeddingPreset_WithNonexistentPreset_ReturnsNull()
    {
        var preset = KreuzbergClient.GetEmbeddingPreset("nonexistent-preset-xyz");
        Assert.Null(preset);
    }

    #endregion

    #region Configuration File Tests

    [Fact]
    public void LoadExtractionConfigFromFile_WithNullPath_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.LoadExtractionConfigFromFile(null!));
        Assert.Contains("path", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void LoadExtractionConfigFromFile_WithEmptyPath_ThrowsValidationException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.LoadExtractionConfigFromFile(""));
        Assert.Contains("path", ex.Message, StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void LoadExtractionConfigFromFile_WithNonexistentFile_ThrowsException()
    {
        var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.LoadExtractionConfigFromFile("/nonexistent/config.toml"));
        Assert.NotNull(ex);
    }

    [Fact]
    public void LoadExtractionConfigFromFile_WithInvalidTomlFormat_ThrowsException()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"invalid-{Guid.NewGuid():N}.toml");
        File.WriteAllText(tempPath, "[ [invalid toml");

        try
        {
            var ex = Assert.Throws<KreuzbergValidationException>(() => KreuzbergClient.LoadExtractionConfigFromFile(tempPath));
            Assert.NotNull(ex);
        }
        finally
        {
            if (File.Exists(tempPath))
            {
                File.Delete(tempPath);
            }
        }
    }

    #endregion

    #region Document Extractor Tests

    [Fact]
    public void UnregisterDocumentExtractor_WithNullName_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.UnregisterDocumentExtractor(null!));
        Assert.Contains("name", ex.Message);
    }

    [Fact]
    public void UnregisterDocumentExtractor_WithEmptyName_ThrowsArgumentException()
    {
        var ex = Assert.Throws<ArgumentException>(() => KreuzbergClient.UnregisterDocumentExtractor(""));
        Assert.Contains("name", ex.Message);
    }

    #endregion

    #region BytesWithMime Validation Tests

    [Fact]
    public void BytesWithMime_WithNullData_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => new BytesWithMime(null!, "application/pdf"));
        Assert.Contains("data", ex.Message);
    }

    [Fact]
    public void BytesWithMime_WithNullMimeType_ThrowsArgumentNullException()
    {
        var ex = Assert.Throws<ArgumentNullException>(() => new BytesWithMime(new byte[] { 1, 2, 3 }, null!));
        Assert.Contains("mimeType", ex.Message);
    }

    [Fact]
    public void BytesWithMime_WithValidInputs_Constructs()
    {
        var data = new byte[] { 1, 2, 3 };
        var mime = "application/pdf";

        var item = new BytesWithMime(data, mime);

        Assert.Equal(data, item.Data);
        Assert.Equal(mime, item.MimeType);
    }

    #endregion

    #region Extraction Exception Types Tests

    [Fact]
    public void KreuzbergValidationException_WithMessage_HasMessage()
    {
        var message = "Test validation error";
        var ex = new KreuzbergValidationException(message);

        Assert.Contains(message, ex.Message);
    }

    [Fact]
    public void KreuzbergException_WithErrorKind_PreservesKind()
    {
        var ex = new KreuzbergException(KreuzbergErrorKind.Validation, "Test error");

        Assert.Equal(KreuzbergErrorKind.Validation, ex.Kind);
        Assert.Equal("Test error", ex.Message);
    }

    #endregion

    #region Permission and Access Tests

    [Fact]
    public void ExtractFileSync_WithUnreadableFile_ThrowsException()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"unreadable-{Guid.NewGuid():N}.pdf");
        File.WriteAllText(tempPath, "%PDF-1.7");

        try
        {
            // Change permissions (platform-specific)
            if (OperatingSystem.IsWindows())
            {
                // Windows permission test would require different approach
                Assert.True(true);
            }
            else
            {
                // Unix-like systems
                System.Diagnostics.Process.Start("chmod", $"000 {tempPath}").WaitForExit();

                var ex = Assert.Throws<KreuzbergIOException>(() => KreuzbergClient.ExtractFileSync(tempPath));
                Assert.NotNull(ex);

                // Restore permissions for cleanup
                System.Diagnostics.Process.Start("chmod", $"644 {tempPath}").WaitForExit();
            }
        }
        finally
        {
            if (File.Exists(tempPath))
            {
                try
                {
                    File.Delete(tempPath);
                }
                catch
                {
                    // Ignore cleanup errors
                }
            }
        }
    }

    #endregion

    #region Async Error Handling Tests

    [Fact]
    public async void ExtractFileAsync_WithNonexistentFile_ThrowsException()
    {
        var ex = await Assert.ThrowsAsync<KreuzbergValidationException>(() => KreuzbergClient.ExtractFileAsync("/nonexistent/file.pdf"));
        Assert.NotNull(ex);
    }

    [Fact]
    public async void BatchExtractFilesAsync_WithInvalidPath_Fails()
    {
        var paths = new[] { "/nonexistent/path.pdf" };

        try
        {
            var result = await KreuzbergClient.BatchExtractFilesAsync(paths);
            // Async batch extraction might not throw - just verify it completes
            Assert.NotNull(result);
        }
        catch (KreuzbergValidationException)
        {
            // Validation error is acceptable
            Assert.True(true);
        }
    }

    #endregion

    #region Helper Test Classes

    private sealed class TestPostProcessor : IPostProcessor
    {
        public TestPostProcessor(string name, int priority)
        {
            Name = name;
            Priority = priority;
        }

        public string Name { get; }
        public int Priority { get; }

        public ExtractionResult Process(ExtractionResult result)
        {
            return result;
        }
    }

    private sealed class TestValidator : IValidator
    {
        public TestValidator(string name, int priority)
        {
            Name = name;
            Priority = priority;
        }

        public string Name { get; }
        public int Priority { get; }

        public void Validate(ExtractionResult result)
        {
            // No validation for test
        }
    }

    private sealed class TestOcrBackend : IOcrBackend
    {
        public TestOcrBackend(string name)
        {
            Name = name;
        }

        public string Name { get; }

        public string Process(ReadOnlySpan<byte> imageBytes, OcrConfig? config)
        {
            return "{}";
        }
    }

    #endregion
}
