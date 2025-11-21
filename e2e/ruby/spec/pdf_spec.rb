# frozen_string_literal: true

# Auto-generated tests for pdf fixtures.

# rubocop:disable Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'pdf fixtures' do
  it 'pdf_assembly_technical' do
    E2ERuby.run_fixture(
      'pdf_assembly_technical',
      'pdfs/assembly_language_for_beginners_al4_b_en.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 5_000)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[assembly register instruction])
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'pdf_bayesian_data_analysis' do
    E2ERuby.run_fixture(
      'pdf_bayesian_data_analysis',
      'pdfs/bayesian_data_analysis_third_edition_13th_feb_2020.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10_000)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[Bayesian probability distribution])
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'pdf_code_and_formula' do
    E2ERuby.run_fixture(
      'pdf_code_and_formula',
      'pdfs/code_and_formula.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
    end
  end

  it 'pdf_deep_learning' do
    E2ERuby.run_fixture(
      'pdf_deep_learning',
      'pdfs/fundamentals_of_deep_learning_2014.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 1_000)
      E2ERuby::Assertions.assert_content_contains_any(result, ['neural', 'network', 'deep learning'])
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'pdf_embedded_images' do
    E2ERuby.run_fixture(
      'pdf_embedded_images',
      'pdfs/embedded_images_tables.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
      E2ERuby::Assertions.assert_table_count(result, 0, nil)
    end
  end

  it 'pdf_google_doc' do
    E2ERuby.run_fixture(
      'pdf_google_doc',
      'pdfs/google_doc_document.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'pdf_large_ciml' do
    E2ERuby.run_fixture(
      'pdf_large_ciml',
      'pdfs/a_course_in_machine_learning_ciml_v0_9_all.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10_000)
      E2ERuby::Assertions.assert_content_contains_any(result, ['machine learning', 'algorithm', 'training'])
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'pdf_non_english_german' do
    E2ERuby.run_fixture(
      'pdf_non_english_german',
      'pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[Intel paging])
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'pdf_right_to_left' do
    E2ERuby.run_fixture(
      'pdf_right_to_left',
      'pdfs/right_to_left_01.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'pdf_simple_text' do
    E2ERuby.run_fixture(
      'pdf_simple_text',
      'pdfs/fake_memo.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
      E2ERuby::Assertions.assert_content_contains_any(result, ['May 5, 2023', 'To Whom it May Concern', 'Mallori'])
    end
  end

  it 'pdf_tables_large' do
    E2ERuby.run_fixture(
      'pdf_tables_large',
      'pdfs_with_tables/large.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 500)
    end
  end

  it 'pdf_tables_medium' do
    E2ERuby.run_fixture(
      'pdf_tables_medium',
      'pdfs_with_tables/medium.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
    end
  end

  it 'pdf_tables_small' do
    E2ERuby.run_fixture(
      'pdf_tables_small',
      'pdfs_with_tables/tiny.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'pdf_technical_stat_learning' do
    E2ERuby.run_fixture(
      'pdf_technical_stat_learning',
      'pdfs/an_introduction_to_statistical_learning_with_applications_in_r_islr_sixth_printing.pdf',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10_000)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[statistical regression learning])
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
