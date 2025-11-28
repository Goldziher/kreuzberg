using Kreuzberg;

class Program
{
    static async Task Main()
    {
        try
        {
            // Create comprehensive extraction configuration
            var config = new ExtractionConfig
            {
                UseCache = true,
                EnableQualityProcessing = true,
                ForceOcr = false,

                // OCR configuration
                Ocr = new OcrConfig
                {
                    Backend = "tesseract",
                    Language = "eng+fra",
                    TesseractConfig = new TesseractConfig
                    {
                        Psm = 3,
                        Oem = 3,
                        MinConfidence = 0.8,
                        Preprocessing = new ImagePreprocessingConfig
                        {
                            TargetDpi = 300,
                            Denoise = true,
                            Deskew = true,
                            ContrastEnhance = true
                        },
                        EnableTableDetection = true
                    }
                },

                // PDF configuration
                PdfOptions = new PdfConfig
                {
                    ExtractImages = true,
                    ExtractMetadata = true
                },

                // Image extraction
                Images = new ImageExtractionConfig
                {
                    ExtractImages = true,
                    TargetDpi = 150,
                    MaxImageDimension = 4096
                },

                // Chunking
                Chunking = new ChunkingConfig
                {
                    MaxChars = 1000,
                    MaxOverlap = 200,
                    Preset = "default"
                },

                // Token reduction
                TokenReduction = new TokenReductionConfig
                {
                    Mode = "moderate",
                    PreserveImportantWords = true
                },

                // Language detection
                LanguageDetection = new LanguageDetectionConfig
                {
                    Enabled = true,
                    MinConfidence = 0.8,
                    DetectMultiple = false
                },

                // Post-processing
                Postprocessor = new PostProcessorConfig
                {
                    Enabled = true
                }
            };

            // Extract with configuration
            var result = await KreuzbergClient.ExtractFileAsync(
                "document.pdf",
                config
            );

            Console.WriteLine($"Content length: {result.Content.Length}");
            Console.WriteLine($"MIME type: {result.MimeType}");
            Console.WriteLine($"Format type: {result.Metadata.FormatType}");

            if (result.Tables.Any())
            {
                Console.WriteLine($"Found {result.Tables.Count} tables");
            }

            if (result.Chunks?.Any() == true)
            {
                Console.WriteLine($"Created {result.Chunks.Count} chunks");
            }
        }
        catch (KreuzbergException ex)
        {
            Console.WriteLine($"Extraction error: {ex.Message}");
        }
    }
}
