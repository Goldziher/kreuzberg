from __future__ import annotations

from pathlib import Path

TEST_DOCS_ROOT = Path(__file__).parent.parent.parent / "test_documents"

BENCHMARK_TEST_FILES: dict[str, list[str]] = {
    "small_text": [
        "text/book_war_and_peace_1p.txt",
        "text/selected_pdfs.txt",
        "text/world_war_ii.md",
        "text/readme.rst",
        "pandoc/restructured_text.rst",
        "misc/readme.org",
        "pandoc/latex_document.tex",
    ],
    "small_web": [
        "web/complex_table.html",
        "web/html.html",
    ],
    "small_office": [
        "office/document.docx",
        "documents/fake.odt",
        "documents/simple.odt",
    ],
    "medium_pdf": [
        "pdfs/test_article.pdf",
        "pdfs/copy_protected.pdf",
        "pdfs_with_tables/medium.pdf",
        "pdfs/a_comparison_of_programming_languages_in_economics_16_jun_2014.pdf",
        "pdfs/image_only_german_pdf.pdf",
        "pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf",
        "pdfs/a_brief_introduction_to_the_standard_annotation_language_sal_2006.pdf",
    ],
    "medium_images": [
        "images/2305_03393v1_pg9_img.png",
        "images/english_and_korean.png",
        "images/layout_parser_paper_with_table.jpg",
        "images/layout_parser_ocr.jpg",
        "images/invoice_image.png",
        "images/chi_sim_image.jpeg",
        "images/jpn_vert.jpeg",
        "images/bmp_24.bmp",
    ],
    "large_pdf": [
        "pdfs/sharable_web_guide.pdf",
        "pdfs_with_tables/large.pdf",
        "gmft/tatr.pdf",
        "pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf",
        "pdfs/xerox_alta_link_series_mfp_sag_en_us_2.pdf",
        "pdfs/a_catalogue_of_optimizing_transformations_1971_allen_catalog.pdf",
        "pdfs/a_brief_introduction_to_neural_networks_neuronalenetze_en_zeta2_2col_dkrieselcom.pdf",
        "pdfs/assembly_language_for_beginners_al4_b_en.pdf",
        "pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf",
    ],
    "large_web": [
        "web/taylor_swift.html",
        "web/beijing_chinese.html",
        "web/world_war_ii.html",
        "web/germany_german.html",
        "web/crohns_disease.html",
        "web/berlin_german.html",
    ],
    "xlarge_pdf": [
        "pdfs/algebra_topology_differential_calculus_and_optimization_theory_for_computer_science_and_machine_learning_2019_math_deep.pdf",
        "pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf",
        "pdfs/intel_64_and_ia_32_architectures_software_developer_s_manual_combined_volumes_1_4_june_2021_325462_sdm_vol_1_2abcd_3abcd.pdf",
        "pdfs/fundamentals_of_deep_learning_2014.pdf",
        "pdfs/proof_of_concept_or_gtfo_v13_october_18th_2016.pdf",
    ],
    "spreadsheets": [
        "office/excel.xlsx",
        "spreadsheets/stanley_cups.xlsx",
        "spreadsheets/excel_multi_sheet.xlsx",
        "spreadsheets/test_01.xlsx",
        "spreadsheets/test_excel.xls",
        "spreadsheets/tests_example.xls",
        "spreadsheets/stanley_cups.csv",
        "pandoc/data_table.csv",
    ],
    "presentations": [
        "presentations/simple.pptx",
        "presentations/powerpoint_sample.pptx",
        "presentations/powerpoint_bad_text.pptx",
        "presentations/powerpoint_with_image.pptx",
        "presentations/pitch_deck_presentation.pptx",
    ],
    "data_formats": [
        "json/test_01.json",
        "json/test_02.json",
        "json/test_03.json",
        "json/real_world/github_emojis.json",
        "json/real_world/earthquakes.geojson",
        "data_formats/simple.json",
        "data_formats/test_results.json",
        "yaml/config.yaml",
        "data_formats/simple.yaml",
    ],
    "emails": [
        "email/html_only.eml",
        "email/multipart_email.eml",
        "email/fake_email.eml",
        "email/sample_email.eml",
        "email/complex_headers.eml",
        "email/eml/with_attachments/mixed_content_types.eml",
        "email/eml/with_attachments/mailgun_pdf_attachment.eml",
        "email/fake_email.msg",
        "email/fake_email_attachment.msg",
        "email/msg/simple/simple_msg.msg",
        "email/msg/with_attachments/msg_with_png_attachment.msg",
    ],
    "other_docs": [
        "misc/simple.epub",
        "misc/winter_sports.epub",
    ],
}


def get_benchmark_files() -> list[tuple[str, Path]]:
    test_files = []

    for category, files in BENCHMARK_TEST_FILES.items():
        for file_path in files:
            full_path = TEST_DOCS_ROOT / file_path
            if full_path.exists():
                file_name = Path(file_path).name
                test_id = f"{category}_{file_name}"
                test_files.append((test_id, full_path))

    return test_files
