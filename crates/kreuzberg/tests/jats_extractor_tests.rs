//! Comprehensive test suite for the JATS (Journal Article Tag Suite) extractor.
//!
//! This test suite validates all aspects of the JATS extractor including:
//! - Metadata extraction (title, authors, affiliations, DOI, keywords, dates)
//! - Article content extraction (sections, paragraphs)
//! - Table extraction with proper structure
//! - Citation handling
//! - Edge cases and error handling

#[cfg(all(test, feature = "xml"))]
mod jats_extractor_tests {
    use kreuzberg::core::config::ExtractionConfig;
    use kreuzberg::extractors::JatsExtractor;
    use kreuzberg::plugins::{DocumentExtractor, Plugin};
    use std::path::PathBuf;

    fn jats_fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../test_documents/jats")
            .join(name)
    }

    /// Test basic JATS article extraction with all key metadata fields
    #[tokio::test]
    async fn test_extract_complete_jats_article() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article xmlns:xlink="http://www.w3.org/1999/xlink" article-type="research-article">
  <front>
    <article-meta>
      <article-title>Effects of Caffeine on Human Health</article-title>
      <subtitle>A Systematic Review and Meta-Analysis</subtitle>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Smith</surname>
            <given-names>John A.</given-names>
          </name>
        </contrib>
        <contrib contrib-type="author">
          <name>
            <surname>Johnson</surname>
            <given-names>Jane B.</given-names>
          </name>
        </contrib>
      </contrib-group>
      <aff id="aff1">Department of Medicine, Harvard University, Cambridge, MA</aff>
      <article-id pub-id-type="doi">10.1371/journal.pmed.0020124</article-id>
      <article-id pub-id-type="pii">05-PLME-RA-0071R2</article-id>
      <pub-date pub-type="epub">
        <day>18</day>
        <month>04</month>
        <year>2005</year>
      </pub-date>
      <volume>2</volume>
      <issue>4</issue>
      <fpage>e124</fpage>
      <lpage>e132</lpage>
      <kwd-group>
        <kwd>caffeine</kwd>
        <kwd>meta-analysis</kwd>
        <kwd>systematic review</kwd>
      </kwd-group>
      <abstract>
        <sec>
          <title>Background</title>
          <p>Caffeine is one of the most widely consumed psychoactive substances.</p>
        </sec>
      </abstract>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Introduction</title>
      <p>This review examines the evidence for effects of caffeine.</p>
    </sec>
  </body>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        assert!(extraction.content.contains("Effects of Caffeine"));
        assert!(extraction.content.contains("Introduction"));

        assert!(extraction.metadata.subject.is_some());
        let subject = extraction.metadata.subject.unwrap();
        assert!(subject.contains("Effects of Caffeine"));

        assert!(subject.contains("10.1371"));

        assert!(subject.contains("caffeine") || subject.contains("Keywords"));
    }

    /// Test extraction of rich metadata including all author and affiliation data
    #[tokio::test]
    async fn test_extract_rich_author_affiliation_metadata() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Advanced Study</article-title>
      <contrib-group>
        <contrib contrib-type="author">
          <name>
            <surname>Alpha</surname>
            <given-names>First</given-names>
          </name>
          <xref ref-type="aff" rid="aff1">1</xref>
          <xref ref-type="aff" rid="aff2">2</xref>
        </contrib>
        <contrib contrib-type="author">
          <name>
            <surname>Beta</surname>
            <given-names>Second</given-names>
          </name>
          <xref ref-type="aff" rid="aff1">1</xref>
        </contrib>
        <contrib contrib-type="author">
          <name>
            <surname>Gamma</surname>
            <given-names>Third</given-names>
          </name>
          <xref ref-type="aff" rid="aff3">3</xref>
          <role>Correspondence</role>
        </contrib>
      </contrib-group>
      <aff id="aff1"><label>1</label>Department of Science, University A, City A</aff>
      <aff id="aff2"><label>2</label>Research Institute, City B</aff>
      <aff id="aff3"><label>3</label>Medical Center, City C</aff>
    </article-meta>
  </front>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        let subject = extraction.metadata.subject.unwrap();
        assert!(subject.contains("Alpha"));
        assert!(subject.contains("Beta"));
        assert!(subject.contains("Gamma"));
        assert!(subject.contains("Department of Science"));
    }

    /// Test section hierarchy extraction in article body
    #[tokio::test]
    async fn test_extract_section_hierarchy() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Research Article</article-title>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Introduction</title>
      <p>Introduction content here.</p>
    </sec>
    <sec id="s2">
      <title>Methods</title>
      <sec id="s2a">
        <title>Study Design</title>
        <p>Design content here.</p>
      </sec>
      <sec id="s2b">
        <title>Participants</title>
        <p>Participant content here.</p>
      </sec>
    </sec>
    <sec id="s3">
      <title>Results</title>
      <p>Results content here.</p>
    </sec>
    <sec id="s4">
      <title>Discussion</title>
      <p>Discussion content here.</p>
    </sec>
    <sec id="s5">
      <title>Conclusions</title>
      <p>Conclusion content here.</p>
    </sec>
  </body>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        assert!(extraction.content.contains("Introduction"));
        assert!(extraction.content.contains("Methods"));
        assert!(extraction.content.contains("Results"));
        assert!(extraction.content.contains("Discussion"));
        assert!(extraction.content.contains("Conclusions"));
        assert!(extraction.content.contains("Study Design"));
        assert!(extraction.content.contains("Participants"));
    }

    /// Test table extraction with headers and data rows
    #[tokio::test]
    async fn test_extract_tables_with_captions() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Study Results</article-title>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Results</title>
      <table-wrap id="tbl1">
        <label>Table 1</label>
        <caption>
          <title>Characteristics of Study Population</title>
          <p>Baseline characteristics of enrolled subjects.</p>
        </caption>
        <table>
          <thead>
            <tr>
              <th>Parameter</th>
              <th>Group A (n=50)</th>
              <th>Group B (n=50)</th>
              <th>P-value</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td>Age (years)</td>
              <td>45.3 ± 8.2</td>
              <td>44.9 ± 7.8</td>
              <td>0.45</td>
            </tr>
            <tr>
              <td>Sex (M/F)</td>
              <td>28/22</td>
              <td>26/24</td>
              <td>0.58</td>
            </tr>
            <tr>
              <td>BMI (kg/m²)</td>
              <td>25.1 ± 3.2</td>
              <td>24.8 ± 2.9</td>
              <td>0.62</td>
            </tr>
          </tbody>
        </table>
      </table-wrap>
    </sec>
  </body>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        assert_eq!(extraction.tables.len(), 1);
        let table = &extraction.tables[0];

        assert!(table.cells.len() >= 3);
        assert_eq!(table.cells[0].len(), 4);

        assert!(table.cells[0][0].contains("Parameter"));
        assert!(table.cells[1][0].contains("Age"));
        assert!(table.cells[2][0].contains("Sex"));
    }

    /// Test multiple tables extraction
    #[tokio::test]
    async fn test_extract_multiple_tables() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Multi-Table Article</article-title>
    </article-meta>
  </front>
  <body>
    <table-wrap id="tbl1">
      <table>
        <thead>
          <tr><th>A</th><th>B</th></tr>
        </thead>
        <tbody>
          <tr><td>1</td><td>2</td></tr>
        </tbody>
      </table>
    </table-wrap>
    <table-wrap id="tbl2">
      <table>
        <thead>
          <tr><th>X</th><th>Y</th><th>Z</th></tr>
        </thead>
        <tbody>
          <tr><td>a</td><td>b</td><td>c</td></tr>
        </tbody>
      </table>
    </table-wrap>
  </body>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        assert_eq!(extraction.tables.len(), 2);
        assert_eq!(extraction.tables[0].cells[0].len(), 2);
        assert_eq!(extraction.tables[1].cells[0].len(), 3);
    }

    /// Test citation extraction in text with xref elements
    #[tokio::test]
    async fn test_extract_citations_in_text() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Citation Study</article-title>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Introduction</title>
      <p>Previous research has shown effectiveness <xref ref-type="bibr" rid="ref1">1</xref>.
         Other studies confirm this finding <xref ref-type="bibr" rid="ref2">2</xref>.</p>
    </sec>
  </body>
  <back>
    <ref-list>
      <ref id="ref1">
        <element-citation publication-type="journal">
          <person-group person-group-type="author">
            <name>
              <surname>Author</surname>
              <given-names>First</given-names>
            </name>
          </person-group>
          <article-title>Original Research</article-title>
          <source>Journal Name</source>
          <year>2020</year>
        </element-citation>
      </ref>
      <ref id="ref2">
        <element-citation publication-type="journal">
          <person-group person-group-type="author">
            <name>
              <surname>Researcher</surname>
              <given-names>Second</given-names>
            </name>
          </person-group>
          <article-title>Confirmatory Study</article-title>
          <source>Other Journal</source>
          <year>2021</year>
        </element-citation>
      </ref>
    </ref-list>
  </back>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        assert!(extraction.content.contains("Previous research"));
        assert!(extraction.content.contains("Other studies"));
    }

    /// Test abstract extraction with structured sections
    #[tokio::test]
    async fn test_extract_structured_abstract() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Abstract Study</article-title>
      <abstract>
        <sec>
          <title>Background</title>
          <p>This is the background information of the study.</p>
        </sec>
        <sec>
          <title>Methods and Findings</title>
          <p>We used quantitative analysis to evaluate the hypothesis.</p>
        </sec>
        <sec>
          <title>Conclusions</title>
          <p>The study provides evidence that the hypothesis is correct.</p>
        </sec>
      </abstract>
    </article-meta>
  </front>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        let subject = extraction.metadata.subject.unwrap();
        assert!(subject.contains("background") || subject.contains("Background") || subject.contains("Abstract"));
    }

    /// Test corresponding author extraction
    #[tokio::test]
    async fn test_extract_corresponding_author() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Study</article-title>
      <author-notes>
        <corresp id="cor1"><label>*</label>Corresponding author. E-mail: john.smith@example.com</corresp>
      </author-notes>
    </article-meta>
  </front>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        assert!(extraction.metadata.subject.is_some());
    }

    /// Test publication date extraction in various formats
    #[tokio::test]
    async fn test_extract_publication_date() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Publication Test</article-title>
      <pub-date pub-type="epub">
        <day>15</day>
        <month>06</month>
        <year>2023</year>
      </pub-date>
    </article-meta>
  </front>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        assert!(extraction.metadata.date.is_some());
    }

    /// Test handling of empty/minimal JATS documents
    #[tokio::test]
    async fn test_extract_minimal_jats() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
    </article-meta>
  </front>
  <body>
  </body>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();
        assert!(extraction.content.is_empty() || extraction.content.trim().is_empty());
    }

    /// Test MIME type support
    #[test]
    fn test_jats_supported_mime_types() {
        let extractor = JatsExtractor::new();
        let mime_types = extractor.supported_mime_types();

        assert!(mime_types.contains(&"application/x-jats+xml"));
        assert!(mime_types.contains(&"text/jats"));
    }

    /// Test extractor priority value
    #[test]
    fn test_jats_extractor_priority() {
        let extractor = JatsExtractor::new();
        assert_eq!(extractor.priority(), 50);
    }

    /// Test plugin interface compliance
    #[test]
    fn test_jats_plugin_interface() {
        let extractor = JatsExtractor::new();
        assert_eq!(extractor.name(), "jats-extractor");
        assert!(!extractor.version().is_empty());
        assert!(extractor.initialize().is_ok());
        assert!(extractor.shutdown().is_ok());
    }

    /// Test mixed content with tables and paragraphs
    #[tokio::test]
    async fn test_extract_mixed_content() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Mixed Content</article-title>
    </article-meta>
  </front>
  <body>
    <sec id="s1">
      <title>Analysis</title>
      <p>First paragraph with data.</p>
      <table-wrap id="tbl1">
        <table>
          <thead>
            <tr><th>Data</th><th>Value</th></tr>
          </thead>
          <tbody>
            <tr><td>Sample</td><td>100</td></tr>
          </tbody>
        </table>
      </table-wrap>
      <p>Second paragraph after table.</p>
    </sec>
  </body>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        assert!(extraction.content.contains("First paragraph"));
        assert!(extraction.content.contains("Second paragraph"));
        assert_eq!(extraction.tables.len(), 1);
    }

    /// Test extraction with multiple keyword groups
    #[tokio::test]
    async fn test_extract_multiple_keywords() {
        let jats_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<article>
  <front>
    <article-meta>
      <article-title>Keyword Test</article-title>
      <kwd-group xml:lang="en">
        <kwd>primary keyword</kwd>
        <kwd>secondary keyword</kwd>
      </kwd-group>
      <kwd-group xml:lang="es">
        <kwd>palabra clave</kwd>
      </kwd-group>
    </article-meta>
  </front>
</article>"#;

        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();
        let result = extractor
            .extract_bytes(jats_content.as_bytes(), "application/x-jats+xml", &config)
            .await;

        assert!(result.is_ok());
        let extraction = result.unwrap();

        let subject = extraction.metadata.subject.unwrap();
        assert!(subject.contains("keyword") || subject.contains("Keyword"));
    }

    /// Test full extraction workflow on file
    #[tokio::test]
    async fn test_extract_jats_file() {
        let extractor = JatsExtractor::new();
        let config = ExtractionConfig::default();

        let test_file = jats_fixture("sample_article.jats");
        if test_file.exists() {
            let result = extractor
                .extract_file(&test_file, "application/x-jats+xml", &config)
                .await;

            assert!(result.is_ok());
            let extraction = result.unwrap();

            assert!(!extraction.content.is_empty());
            assert!(extraction.metadata.subject.is_some());
        }
    }
}
