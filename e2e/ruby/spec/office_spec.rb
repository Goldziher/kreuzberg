# frozen_string_literal: true

# Auto-generated tests for office fixtures.

# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'office fixtures' do
  it 'office_doc_legacy' do
    E2ERuby.run_fixture(
      'office_doc_legacy',
      'legacy_office/unit_test_lists.doc',
      nil,
      requirements: %w[libreoffice libreoffice],
      notes: 'LibreOffice must be installed for conversion.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/msword']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_basic' do
    E2ERuby.run_fixture(
      'office_docx_basic',
      'office/document.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_docx_equations' do
    E2ERuby.run_fixture(
      'office_docx_equations',
      'documents/equations.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_fake' do
    E2ERuby.run_fixture(
      'office_docx_fake',
      'documents/fake.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_formatting' do
    E2ERuby.run_fixture(
      'office_docx_formatting',
      'documents/unit_test_formatting.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_headers' do
    E2ERuby.run_fixture(
      'office_docx_headers',
      'documents/unit_test_headers.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_lists' do
    E2ERuby.run_fixture(
      'office_docx_lists',
      'documents/unit_test_lists.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_docx_tables' do
    E2ERuby.run_fixture(
      'office_docx_tables',
      'documents/docx_tables.docx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.wordprocessingml.document']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
      E2ERuby::Assertions.assert_content_contains_all(result, ['Simple uniform table', 'Nested Table', 'merged cells', 'Header Col'])
      E2ERuby::Assertions.assert_table_count(result, 1, nil)
    end
  end

  it 'office_ppt_legacy' do
    E2ERuby.run_fixture(
      'office_ppt_legacy',
      'legacy_office/simple.ppt',
      nil,
      requirements: %w[libreoffice libreoffice],
      notes: 'Skip if LibreOffice conversion is unavailable.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.ms-powerpoint']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_pptx_basic' do
    E2ERuby.run_fixture(
      'office_pptx_basic',
      'presentations/simple.pptx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.presentationml.presentation']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 50)
    end
  end

  it 'office_pptx_images' do
    E2ERuby.run_fixture(
      'office_pptx_images',
      'presentations/powerpoint_with_image.pptx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.presentationml.presentation']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end

  it 'office_pptx_pitch_deck' do
    E2ERuby.run_fixture(
      'office_pptx_pitch_deck',
      'presentations/pitch_deck_presentation.pptx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.presentationml.presentation']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
    end
  end

  it 'office_xls_legacy' do
    E2ERuby.run_fixture(
      'office_xls_legacy',
      'spreadsheets/test_excel.xls',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.ms-excel']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'office_xlsx_basic' do
    E2ERuby.run_fixture(
      'office_xlsx_basic',
      'spreadsheets/stanley_cups.xlsx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
      E2ERuby::Assertions.assert_content_contains_all(result, ['Team', 'Location', 'Stanley Cups'])
      E2ERuby::Assertions.assert_table_count(result, 1, nil)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'sheet_count', { gte: 2 })
      E2ERuby::Assertions.assert_metadata_expectation(result, 'sheet_names', { contains: ['Stanley Cups'] })
    end
  end

  it 'office_xlsx_multi_sheet' do
    E2ERuby.run_fixture(
      'office_xlsx_multi_sheet',
      'spreadsheets/excel_multi_sheet.xlsx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'sheet_count', { gte: 2 })
    end
  end

  it 'office_xlsx_office_example' do
    E2ERuby.run_fixture(
      'office_xlsx_office_example',
      'office/excel.xlsx',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
