# frozen_string_literal: true

# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength
require_relative 'spec_helper'

RSpec.describe 'ocr fixtures' do
  it 'ocr_image_hello_world' do
    E2ERuby.run_fixture(
      'ocr_image_hello_world',
      'images/test_hello_world.png',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Requires Tesseract OCR for image text extraction.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/png']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 5)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[hello world])
    end
  end

  it 'ocr_image_no_text' do
    E2ERuby.run_fixture(
      'ocr_image_no_text',
      'images/flower_no_text.jpg',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Skip when Tesseract is unavailable.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/jpeg']
      )
      E2ERuby::Assertions.assert_max_content_length(result, 200)
    end
  end

  it 'ocr_pdf_image_only_german' do
    E2ERuby.run_fixture(
      'ocr_pdf_image_only_german',
      'pdfs/image_only_german_pdf.pdf',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Skip if OCR backend unavailable.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'format_type', { eq: 'pdf' })
    end
  end

  it 'ocr_pdf_rotated_90' do
    E2ERuby.run_fixture(
      'ocr_pdf_rotated_90',
      'pdfs/ocr_test_rotated_90.pdf',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Skip automatically when OCR backend is missing.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end

  it 'ocr_pdf_tesseract' do
    E2ERuby.run_fixture(
      'ocr_pdf_tesseract',
      'pdfs/ocr_test.pdf',
      { force_ocr: true, ocr: { backend: 'tesseract', language: 'eng' } },
      requirements: %w[tesseract tesseract],
      notes: 'Skip automatically if OCR backend is unavailable.',
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/pdf']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_content_contains_any(result, %w[Docling Markdown JSON])
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength
