package com.kreuzberg.e2e;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;

/** Auto-generated tests for ocr fixtures. */
public class OcrTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void ocrImageHelloWorld() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_image_hello_world",
            "images/test_hello_world.png",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Requires Tesseract OCR for image text extraction.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/png"));
                E2EHelpers.Assertions.assertMinContentLength(result, 5);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("hello", "world"));
            }
        );
    }

    @Test
    public void ocrImageNoText() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_image_no_text",
            "images/flower_no_text.jpg",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Skip when Tesseract is unavailable.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("image/jpeg"));
                E2EHelpers.Assertions.assertMaxContentLength(result, 200);
            }
        );
    }

    @Test
    public void ocrPdfImageOnlyGerman() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_pdf_image_only_german",
            "pdfs/image_only_german_pdf.pdf",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Skip if OCR backend unavailable.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void ocrPdfRotated90() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_pdf_rotated_90",
            "pdfs/ocr_test_rotated_90.pdf",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Skip automatically when OCR backend is missing.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void ocrPdfTesseract() throws Exception {
        JsonNode config = MAPPER.readTree("{\"force_ocr\":true,\"ocr\":{\"backend\":\"tesseract\",\"language\":\"eng\"}}");
        E2EHelpers.runFixture(
            "ocr_pdf_tesseract",
            "pdfs/ocr_test.pdf",
            config,
            Arrays.asList("tesseract", "tesseract"),
            "Skip automatically if OCR backend is unavailable.",
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 20);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("Docling", "Markdown", "JSON"));
            }
        );
    }

}
