package com.kreuzberg.e2e;

// CHECKSTYLE.OFF: UnusedImports - generated code
// CHECKSTYLE.OFF: LineLength - generated code
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import org.junit.jupiter.api.Test;

import java.util.Arrays;
import java.util.Collections;
import java.util.List;
import java.util.Map;
// CHECKSTYLE.ON: UnusedImports
// CHECKSTYLE.ON: LineLength

/** Auto-generated tests for pdf fixtures. */
public class PdfTest {
    private static final ObjectMapper MAPPER = new ObjectMapper();

    @Test
    public void pdfAssemblyTechnical() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_assembly_technical",
            "pdfs/assembly_language_for_beginners_al4_b_en.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 5000);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("assembly", "register", "instruction"));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void pdfBayesianDataAnalysis() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_bayesian_data_analysis",
            "pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10000);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("Bayesian", "probability", "distribution"));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void pdfCodeAndFormula() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_code_and_formula",
            "pdfs/code_and_formula.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
            }
        );
    }

    @Test
    public void pdfDeepLearning() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_deep_learning",
            "pdfs/fundamentals_of_deep_learning_2014.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 1000);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("neural", "network", "deep learning"));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void pdfEmbeddedImages() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_embedded_images",
            "pdfs/embedded_images_tables.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
                E2EHelpers.Assertions.assertTableCount(result, 0, null);
            }
        );
    }

    @Test
    public void pdfGoogleDoc() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_google_doc",
            "pdfs/google_doc_document.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void pdfLargeCiml() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_large_ciml",
            "pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10000);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("machine learning", "algorithm", "training"));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void pdfNonEnglishGerman() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_non_english_german",
            "pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("Intel", "paging"));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void pdfRightToLeft() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_right_to_left",
            "pdfs/right_to_left_01.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

    @Test
    public void pdfSimpleText() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_simple_text",
            "pdfs/fake_memo.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 50);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("May 5, 2023", "To Whom it May Concern", "Mallori"));
            }
        );
    }

    @Test
    public void pdfTablesLarge() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_tables_large",
            "pdfs_with_tables/large.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 500);
            }
        );
    }

    @Test
    public void pdfTablesMedium() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_tables_medium",
            "pdfs_with_tables/medium.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 100);
            }
        );
    }

    @Test
    public void pdfTablesSmall() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_tables_small",
            "pdfs_with_tables/tiny.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10);
            }
        );
    }

    @Test
    public void pdfTechnicalStatLearning() throws Exception {
        JsonNode config = null;
        E2EHelpers.runFixture(
            "pdf_technical_stat_learning",
            "pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf",
            config,
            Collections.emptyList(),
            null,
            true,
            result -> {
                E2EHelpers.Assertions.assertExpectedMime(result, Arrays.asList("application/pdf"));
                E2EHelpers.Assertions.assertMinContentLength(result, 10000);
                E2EHelpers.Assertions.assertContentContainsAny(result, Arrays.asList("statistical", "regression", "learning"));
                E2EHelpers.Assertions.assertMetadataExpectation(result, "format_type", Map.of("eq", "pdf"));
            }
        );
    }

}
