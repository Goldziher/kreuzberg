using Kreuzberg;

var config = new ExtractionConfig
{
    UseCache = true,
    EnableQualityProcessing = true,
    ForceOcr = false,
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
    PdfOptions = new PdfConfig
    {
        ExtractImages = true,
        ExtractMetadata = true
    },
    Images = new ImageExtractionConfig
    {
        ExtractImages = true,
        TargetDpi = 150,
        MaxImageDimension = 4096
    },
    Chunking = new ChunkingConfig
    {
        MaxChars = 1000,
        MaxOverlap = 200
    },
    TokenReduction = new TokenReductionConfig
    {
        Mode = "moderate",
        PreserveImportantWords = true
    },
    LanguageDetection = new LanguageDetectionConfig
    {
        Enabled = true,
        MinConfidence = 0.8,
        DetectMultiple = false
    },
    Postprocessor = new PostProcessorConfig
    {
        Enabled = true
    }
};

var result = KreuzbergClient.ExtractFileSync("document.pdf", config);
