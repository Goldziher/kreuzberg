# frozen_string_literal: true

# Auto-generated tests for smoke fixtures.

# rubocop:disable Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'smoke fixtures' do
  it 'smoke_docx_basic' do
    E2ERuby.run_fixture(
      'smoke_docx_basic',
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
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[Lorem ipsum document text])
    end
  end

  it 'smoke_html_basic' do
    E2ERuby.run_fixture(
      'smoke_html_basic',
      'web/simple_table.html',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['text/html']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
      E2ERuby::Assertions.assert_content_contains_any(result, ['#', '**', 'simple', 'HTML'])
    end
  end

  it 'smoke_image_png' do
    E2ERuby.run_fixture(
      'smoke_image_png',
      'images/sample.png',
      nil,
      requirements: [],
      notes: 'Image extraction requires image processing dependencies',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/png']
      )
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format', 'PNG')
    end
  end

  it 'smoke_json_basic' do
    E2ERuby.run_fixture(
      'smoke_json_basic',
      'data_formats/simple.json',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/json']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 5)
    end
  end

  it 'smoke_pdf_basic' do
    E2ERuby.run_fixture(
      'smoke_pdf_basic',
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
      E2ERuby::Assertions.assert_content_contains_any(result, ['May 5, 2023', 'To Whom it May Concern'])
    end
  end

  it 'smoke_txt_basic' do
    E2ERuby.run_fixture(
      'smoke_txt_basic',
      'text/report.txt',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['text/plain']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 5)
    end
  end

  it 'smoke_xlsx_basic' do
    E2ERuby.run_fixture(
      'smoke_xlsx_basic',
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
      E2ERuby::Assertions.assert_min_content_length(result, 10)
      E2ERuby::Assertions.assert_table_count(result, 1, nil)
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
