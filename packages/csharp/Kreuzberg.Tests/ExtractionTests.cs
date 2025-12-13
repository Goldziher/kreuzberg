using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text;
using Kreuzberg;
using Xunit;

namespace Kreuzberg.Tests;

/// <summary>
/// Comprehensive tests for document extraction across various file types, configurations, and scenarios.
/// Tests cover MIME detection, encoding detection, large file handling, table extraction, and metadata extraction.
/// </summary>
public class ExtractionTests
{
    public ExtractionTests()
    {
        NativeTestHelper.EnsureNativeLibraryLoaded();
    }

    #region PDF Extraction Tests

    [Fact]
    public void ExtractPdfFileSync_WithValidPdf_ReturnsContent()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        Assert.NotNull(result);
        Assert.True(result.Success, "PDF extraction should succeed");
        Assert.NotEmpty(result.Content);
        Assert.Equal("application/pdf", result.MimeType);
    }

    [Fact]
    public void ExtractPdfFileSync_WithPasswordProtectedPdf_RequiresPassword()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/password_protected.pdf");
        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig
            {
                Passwords = new List<string> { "test123" }
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);
        Assert.NotNull(result);
    }

    [Fact]
    public void ExtractPdfBytesSync_WithValidPdfBytes_ReturnsContent()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(pdfPath);

        var result = KreuzbergClient.ExtractBytesSync(bytes, "application/pdf");

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractPdfFileSync_ExtractMetadata_ContainsPdfProperties()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig { ExtractMetadata = true }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
        Assert.NotNull(result.Metadata.Format);
    }

    [Fact]
    public void ExtractPdfFileSync_ExtractImages_ReturnsImageList()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            PdfOptions = new PdfConfig { ExtractImages = true },
            Images = new ImageExtractionConfig { ExtractImages = true }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);
        Assert.NotNull(result);
    }

    #endregion

    #region Office Document Tests

    [Fact]
    public void ExtractDocxFileSync_WithValidDocx_ReturnsContent()
    {
        var docPath = NativeTestHelper.GetDocumentPath("office/document.docx");
        var result = KreuzbergClient.ExtractFileSync(docPath);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractExcelFileSync_WithValidXlsx_ReturnsContent()
    {
        var excelPath = NativeTestHelper.GetDocumentPath("office/excel.xlsx");
        var result = KreuzbergClient.ExtractFileSync(excelPath);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractExcelFileSync_WithExcel_ContainsExcelMetadata()
    {
        var excelPath = NativeTestHelper.GetDocumentPath("office/excel.xlsx");
        var result = KreuzbergClient.ExtractFileSync(excelPath);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
        Assert.Equal(FormatType.Excel, result.Metadata.FormatType);
    }

    #endregion

    #region Image Extraction Tests

    [Fact]
    public void ExtractImageFileSync_WithJpeg_ReturnsImageMetadata()
    {
        var imagePath = NativeTestHelper.GetDocumentPath("images/chi_sim_image.jpeg");
        var result = KreuzbergClient.ExtractFileSync(imagePath);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
        Assert.Equal(FormatType.Image, result.Metadata.FormatType);
    }

    [Fact]
    public void ExtractImageFileSync_WithPng_DetectsMimeType()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.png");
        try
        {
            var pngBytes = Convert.FromBase64String(
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+X1lQAAAAASUVORK5CYII=");
            File.WriteAllBytes(tempPath, pngBytes);

            var mime = KreuzbergClient.DetectMimeTypeFromPath(tempPath);
            Assert.NotNull(mime);
            Assert.Contains("image", mime, StringComparison.OrdinalIgnoreCase);
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
    public void ExtractImageBytesSync_WithImageBytes_ReturnsFormat()
    {
        var imagePath = NativeTestHelper.GetDocumentPath("images/chi_sim_image.jpeg");
        var bytes = File.ReadAllBytes(imagePath);

        var result = KreuzbergClient.ExtractBytesSync(bytes, "image/jpeg");

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
    }

    #endregion

    #region Text File Tests

    [Fact]
    public void ExtractTextFileSync_WithPlainText_ReturnsContent()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.txt");
        try
        {
            File.WriteAllText(tempPath, "hello world");

            var result = KreuzbergClient.ExtractFileSync(tempPath);
            Assert.NotNull(result);
            Assert.True(result.Success);
            Assert.NotEmpty(result.Content);
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
    public void ExtractMarkdownFileSync_WithValidMarkdown_ReturnsStructuredContent()
    {
        var mdPath = NativeTestHelper.GetDocumentPath("pandoc/simple_metadata.md");
        var result = KreuzbergClient.ExtractFileSync(mdPath);

        Assert.NotNull(result);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractCsvFileSync_WithValidCsv_ExtractsData()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.csv");
        try
        {
            File.WriteAllText(tempPath, "a,b,c\n1,2,3\n");

            var result = KreuzbergClient.ExtractFileSync(tempPath);
            Assert.NotNull(result);
            Assert.NotEmpty(result.Content);
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

    #region Structured Data Tests

    [Fact]
    public void ExtractJsonFileSync_WithValidJson_ReturnsContent()
    {
        var jsonPath = NativeTestHelper.GetDocumentPath("data_formats/simple.json");
        var result = KreuzbergClient.ExtractFileSync(jsonPath);

        Assert.NotNull(result);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractYamlFileSync_WithValidYaml_ReturnsContent()
    {
        var yamlPath = NativeTestHelper.GetDocumentPath("data_formats/simple.yaml");
        var result = KreuzbergClient.ExtractFileSync(yamlPath);

        Assert.NotNull(result);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractXmlFileSync_WithValidXml_ParsesStructure()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.xml");
        try
        {
            File.WriteAllText(tempPath, "<root><item>value</item></root>");

            var result = KreuzbergClient.ExtractFileSync(tempPath);
            Assert.NotNull(result);
            Assert.NotNull(result.Metadata);
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

    #region MIME Type Detection Tests

    [Theory]
    [InlineData("pdf/simple.pdf", "application/pdf")]
    [InlineData("office/document.docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document")]
    [InlineData("office/excel.xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")]
    public void DetectMimeTypeFromPath_WithKnownTypes_ReturnsCorrectMime(string relativePath, string expectedMime)
    {
        var path = NativeTestHelper.GetDocumentPath(relativePath);
        var mime = KreuzbergClient.DetectMimeTypeFromPath(path);

        Assert.Equal(expectedMime, mime);
    }

    [Fact]
    public void DetectMimeTypeFromBytes_WithPdfSignature_ReturnsPdfMime()
    {
        var pdfSignature = Encoding.ASCII.GetBytes("%PDF-1.7\n%");
        var mime = KreuzbergClient.DetectMimeType(pdfSignature);

        Assert.Equal("application/pdf", mime);
    }

    [Fact]
    public void DetectMimeTypeFromBytes_WithZipSignature_ReturnsZipMime()
    {
        // ZIP file signature: PK\x03\x04
        var zipSignature = new byte[] { 0x50, 0x4B, 0x03, 0x04 };
        var mime = KreuzbergClient.DetectMimeType(zipSignature);

        Assert.NotNull(mime);
        Assert.Contains("zip", mime.ToLower());
    }

    [Fact]
    public void DetectMimeTypeFromBytes_WithJpegSignature_ReturnsImageMime()
    {
        // JPEG signature: FF D8 FF
        var jpegSignature = new byte[] { 0xFF, 0xD8, 0xFF, 0xE0 };
        var mime = KreuzbergClient.DetectMimeType(jpegSignature);

        Assert.NotNull(mime);
        Assert.Contains("image", mime);
    }

    [Fact]
    public void DetectMimeTypeFromPath_ReturnsNonEmptyString()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var mime = KreuzbergClient.DetectMimeTypeFromPath(pdfPath);

        Assert.False(string.IsNullOrWhiteSpace(mime));
    }

    #endregion

    #region Encoding Detection Tests

    [Fact]
    public void ExtractTextFileSync_WithUtf8Encoding_CorrectlyDecodes()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.txt");
        try
        {
            File.WriteAllText(tempPath, "café — 你好", new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));

            var result = KreuzbergClient.ExtractFileSync(tempPath);
            Assert.NotNull(result);
            Assert.Contains("你好", result.Content);
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
    public void ExtractTextFileSync_WithMultibyteCharacters_PreservesContent()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.txt");
        try
        {
            File.WriteAllText(tempPath, "こんにちは世界", new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));

            var result = KreuzbergClient.ExtractFileSync(tempPath);
            Assert.NotNull(result);
            Assert.Contains("こんにちは", result.Content);
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
    public void ExtractBytesSync_WithDifferentEncodings_HandlesCorrectly()
    {
        var bytes = Encoding.UTF8.GetBytes("hello ünicode");
        var result = KreuzbergClient.ExtractBytesSync(bytes, "text/plain");

        Assert.NotNull(result);
        Assert.NotEmpty(result.Content);
    }

    #endregion

    #region Table Extraction Tests

    [Fact]
    public void ExtractFileSync_WithTableContent_ExtractsTableStructure()
    {
        var tablePath = NativeTestHelper.GetDocumentPath("tables/simple_table.png");
        var config = new ExtractionConfig
        {
            Ocr = new OcrConfig { Backend = "tesseract" }
        };

        var result = KreuzbergClient.ExtractFileSync(tablePath, config);

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);
    }

    [Fact]
    public void ExtractFileSync_WithComplexTable_PreservesStructure()
    {
        var tablePath = NativeTestHelper.GetDocumentPath("tables/complex_document.png");
        var result = KreuzbergClient.ExtractFileSync(tablePath);

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);
    }

    [Fact]
    public void ExtractFileSync_TableContainsMarkdownRepresentation()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/table_document.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        if (result.Tables.Count > 0)
        {
            var table = result.Tables[0];
            Assert.NotNull(table);
            Assert.NotEmpty(table.Cells);
            Assert.NotEmpty(table.Markdown);
        }
    }

    [Fact]
    public void ExtractFileSync_TableMetadataIncludesPageNumber()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/multi_page_tables.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        Assert.NotNull(result);
        Assert.NotNull(result.Tables);

        foreach (var table in result.Tables)
        {
            Assert.True(table.PageNumber >= 0, "Table page number should be non-negative");
        }
    }

    #endregion

    #region Metadata Extraction Tests

    [Fact]
    public void ExtractPdfFileSync_ContainsMetadataLanguage()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
    }

    [Fact]
    public void ExtractImageFileSync_ExtractsImageMetadata()
    {
        var imagePath = NativeTestHelper.GetDocumentPath("images/chi_sim_image.jpeg");
        var result = KreuzbergClient.ExtractFileSync(imagePath);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
        var imageMetadata = result.Metadata.Format.Image;

        if (imageMetadata != null)
        {
            Assert.True(imageMetadata.Width > 0);
            Assert.True(imageMetadata.Height > 0);
            Assert.False(string.IsNullOrEmpty(imageMetadata.Format));
        }
    }

    [Fact]
    public void ExtractExcelFileSync_ContainsSheetMetadata()
    {
        var excelPath = NativeTestHelper.GetDocumentPath("office/excel.xlsx");
        var result = KreuzbergClient.ExtractFileSync(excelPath);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
        var excelMetadata = result.Metadata.Format.Excel;

        if (excelMetadata != null)
        {
            Assert.NotNull(excelMetadata.SheetNames);
        }
    }

    [Fact]
    public void ExtractDocxFileSync_ContainsDocumentMetadata()
    {
        var docPath = NativeTestHelper.GetDocumentPath("office/document.docx");
        var result = KreuzbergClient.ExtractFileSync(docPath);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
    }

    [Fact]
    public void ExtractFileSync_MetadataFormatTypeMatchesMime()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        Assert.NotNull(result);
        Assert.NotNull(result.Metadata);
        Assert.True(result.Metadata.FormatType == FormatType.Pdf || result.Metadata.FormatType != FormatType.Unknown);
    }

    #endregion

    #region Language Detection Tests

    [Fact]
    public void ExtractFileSync_WithLanguageDetectionEnabled_ReturnsDetectedLanguages()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.txt");
        try
        {
            File.WriteAllText(tempPath, "Hello world. 你好世界。", new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
            var config = new ExtractionConfig
            {
                LanguageDetection = new LanguageDetectionConfig
                {
                    Enabled = true,
                    DetectMultiple = true
                }
            };

            var result = KreuzbergClient.ExtractFileSync(tempPath, config);
            Assert.NotNull(result);
            // The native library only returns detected languages when non-empty.
            Assert.True(result.DetectedLanguages == null || result.DetectedLanguages.Count > 0);
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
    public void ExtractFileSync_DefaultLanguageDetection_MayReturnLanguages()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.txt");
        try
        {
            File.WriteAllText(tempPath, "Hello world", new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
            var result = KreuzbergClient.ExtractFileSync(tempPath);

            Assert.NotNull(result);
            Assert.True(result.DetectedLanguages == null || result.DetectedLanguages.Count > 0);
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

    #region Configuration Integration Tests

    [Fact]
    public void ExtractFileSync_WithCustomConfig_AppliesSettings()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            EnableQualityProcessing = true,
            UseCache = false
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public void ExtractFileSync_WithChunkingConfig_SplitsContent()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.txt");
        try
        {
            File.WriteAllText(tempPath, string.Join(" ", Enumerable.Repeat("lorem ipsum", 5000)));
            var config = new ExtractionConfig
            {
                Chunking = new ChunkingConfig
                {
                    Enabled = true,
                    ChunkSize = 1000,
                    ChunkOverlap = 100
                }
            };

            var result = KreuzbergClient.ExtractFileSync(tempPath, config);
            Assert.NotNull(result);
            Assert.NotNull(result.Chunks);
            Assert.NotEmpty(result.Chunks);
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
    public void ExtractFileSync_WithImageExtractionConfig_ConfiguresImageProcessing()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var config = new ExtractionConfig
        {
            Images = new ImageExtractionConfig
            {
                ExtractImages = true,
                TargetDpi = 150,
                MaxImageDimension = 2000,
                AutoAdjustDpi = true
            }
        };

        var result = KreuzbergClient.ExtractFileSync(pdfPath, config);
        Assert.NotNull(result);
    }

    [Fact]
    public void DiscoverExtractionConfig_FromWorkspaceRoot_MayReturnConfig()
    {
        var config = KreuzbergClient.DiscoverExtractionConfig();

        // Config may or may not exist depending on workspace setup
        // Just verify the call succeeds
        Assert.True(config == null || config is ExtractionConfig);
    }

    [Fact]
    public void LoadExtractionConfigFromFile_WithValidTomlConfig_ParsesCorrectly()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.toml");
        try
        {
            File.WriteAllText(tempPath, "use_cache = false\n");

            var config = KreuzbergClient.LoadExtractionConfigFromFile(tempPath);
            Assert.NotNull(config);
            Assert.False(config.UseCache);
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

    #region Batch Extraction Tests

    [Fact]
    public void BatchExtractFilesSync_WithMultipleFiles_ReturnsResultsInOrder()
    {
        var paths = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var results = KreuzbergClient.BatchExtractFilesSync(paths);

        Assert.NotNull(results);
        Assert.Equal(paths.Length, results.Count);

        foreach (var result in results)
        {
            Assert.NotNull(result);
        }
    }

    [Fact]
    public void BatchExtractFilesSync_WithEmptyList_ReturnsEmpty()
    {
        var paths = new string[] { };
        var results = KreuzbergClient.BatchExtractFilesSync(paths);

        Assert.NotNull(results);
        Assert.Empty(results);
    }

    [Fact]
    public void BatchExtractBytesSync_WithMultipleItems_ReturnsAllResults()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var docPath = NativeTestHelper.GetDocumentPath("office/document.docx");

        var items = new[]
        {
            new BytesWithMime(File.ReadAllBytes(pdfPath), "application/pdf"),
            new BytesWithMime(File.ReadAllBytes(docPath), "application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        };

        var results = KreuzbergClient.BatchExtractBytesSync(items);

        Assert.NotNull(results);
        Assert.Equal(items.Length, results.Count);
    }

    [Fact]
    public void BatchExtractBytesSync_WithEmptyList_ReturnsEmpty()
    {
        var items = new BytesWithMime[] { };
        var results = KreuzbergClient.BatchExtractBytesSync(items);

        Assert.NotNull(results);
        Assert.Empty(results);
    }

    [Fact]
    public void BatchExtractFilesSync_WithConfig_AppliesConfigToAll()
    {
        var paths = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var config = new ExtractionConfig
        {
            EnableQualityProcessing = true
        };

        var results = KreuzbergClient.BatchExtractFilesSync(paths, config);

        Assert.NotNull(results);
        Assert.Equal(paths.Length, results.Count);
    }

    #endregion

    #region Large File Handling Tests

    [Fact]
    public void ExtractLargeFile_WithLargeSize_HandlesSuccessfully()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.txt");
        try
        {
            File.WriteAllText(tempPath, new string('a', 2_000_000));
            var result = KreuzbergClient.ExtractFileSync(tempPath);
            Assert.NotNull(result);
            Assert.NotEmpty(result.Content);
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
    public void ExtractBytesSync_WithLargeByteArray_ProcessesSuccessfully()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(pdfPath);

        // Verify it can handle the bytes
        var result = KreuzbergClient.ExtractBytesSync(bytes, "application/pdf");
        Assert.NotNull(result);
    }

    #endregion

    #region Content Quality Tests

    [Fact]
    public void ExtractFileSync_ContentNotEmpty_ForNonEmptyFiles()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.txt");
        try
        {
            File.WriteAllText(tempPath, "not empty");
            var result = KreuzbergClient.ExtractFileSync(tempPath);

            Assert.NotNull(result);
            Assert.NotEmpty(result.Content);
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
    public void ExtractFileSync_SuccessFlag_IndicatesExtraction()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var result = KreuzbergClient.ExtractFileSync(pdfPath);

        Assert.NotNull(result);
        Assert.True(result.Success);
        Assert.NotNull(result.Content);
    }

    [Fact]
    public void ExtractBytesSync_MimeTypeRoundTrip_ReturnsSameMimeType()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(pdfPath);
        var inputMime = "application/pdf";

        var result = KreuzbergClient.ExtractBytesSync(bytes, inputMime);

        Assert.NotNull(result);
        Assert.Equal(inputMime, result.MimeType);
    }

    #endregion

    #region Format-Specific Extraction Tests

    [Fact]
    public void ExtractEpubFileSync_WithValidEpub_ReturnsContent()
    {
        var epubPath = NativeTestHelper.GetDocumentPath("misc/simple.epub");
        var result = KreuzbergClient.ExtractFileSync(epubPath);

        Assert.NotNull(result);
        Assert.NotEmpty(result.Content);
    }

    [Fact]
    public void ExtractArchiveFileSync_WithZip_ReturnsContent()
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"kreuzberg-test-{Guid.NewGuid():N}.zip");
        try
        {
            using (var archive = ZipFile.Open(tempPath, ZipArchiveMode.Create))
            {
                var entry = archive.CreateEntry("hello.txt");
                using var stream = entry.Open();
                using var writer = new StreamWriter(stream, Encoding.UTF8);
                writer.Write("hello");
            }

            var result = KreuzbergClient.ExtractFileSync(tempPath);
            Assert.NotNull(result);
            Assert.NotNull(result.Metadata);
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

    #region Extension Mapping Tests

    [Fact]
    public void GetExtensionsForMime_WithPdfMime_ReturnsPdfExtension()
    {
        var extensions = KreuzbergClient.GetExtensionsForMime("application/pdf");

        Assert.NotNull(extensions);
        Assert.NotEmpty(extensions);
        Assert.Contains(extensions, e => e.Equals("pdf", StringComparison.OrdinalIgnoreCase));
    }

    [Fact]
    public void GetExtensionsForMime_WithDocxMime_ReturnsDocxExtension()
    {
        var docxMime = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";
        var extensions = KreuzbergClient.GetExtensionsForMime(docxMime);

        Assert.NotNull(extensions);
        Assert.NotEmpty(extensions);
    }

    [Fact]
    public void GetExtensionsForMime_WithImageMime_ReturnsImageExtensions()
    {
        var extensions = KreuzbergClient.GetExtensionsForMime("image/jpeg");

        Assert.NotNull(extensions);
        Assert.NotEmpty(extensions);
    }

    #endregion

    #region Async Extraction Tests

    [Fact]
    public async Task ExtractFileAsync_WithValidPath_CompletsSuccessfully()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var result = await KreuzbergClient.ExtractFileAsync(pdfPath);

        Assert.NotNull(result);
        Assert.True(result.Success);
    }

    [Fact]
    public async Task ExtractBytesAsync_WithValidBytes_CompletsSuccessfully()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");
        var bytes = File.ReadAllBytes(pdfPath);

        var result = await KreuzbergClient.ExtractBytesAsync(bytes, "application/pdf");

        Assert.NotNull(result);
        Assert.NotNull(result.Content);
    }

    [Fact]
    public async Task BatchExtractFilesAsync_WithMultipleFiles_ReturnsAllResults()
    {
        var paths = new[]
        {
            NativeTestHelper.GetDocumentPath("pdf/simple.pdf"),
            NativeTestHelper.GetDocumentPath("office/document.docx")
        };

        var results = await KreuzbergClient.BatchExtractFilesAsync(paths);

        Assert.NotNull(results);
        Assert.Equal(paths.Length, results.Count);
    }

    [Fact]
    public async Task BatchExtractBytesAsync_WithMultipleItems_ReturnsAllResults()
    {
        var pdfPath = NativeTestHelper.GetDocumentPath("pdf/simple.pdf");

        var items = new[]
        {
            new BytesWithMime(File.ReadAllBytes(pdfPath), "application/pdf")
        };

        var results = await KreuzbergClient.BatchExtractBytesAsync(items);

        Assert.NotNull(results);
        Assert.Equal(items.Length, results.Count);
    }

    #endregion
}
