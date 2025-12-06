# frozen_string_literal: true

RSpec.describe Kreuzberg::CLI do
  describe '.extract' do
    it 'extracts content from a file' do
      path = test_document_path('documents/simple.odt')
      output = described_class.extract(path)

      expect(output).to be_a(String)
      expect(output).not_to be_empty
    end

    it 'accepts output format option' do
      path = test_document_path('documents/simple.odt')
      output = described_class.extract(path, output: 'json')

      expect(output).to be_a(String)
      expect(output).not_to be_empty
    end

    it 'accepts OCR option' do
      path = test_document_path('pdfs/100_g_networking_technology_overview_slides_toronto_august_2016.pdf')
      output = described_class.extract(path, ocr: false)

      expect(output).to be_a(String)
      expect(output).not_to be_empty
    end
  end

  describe '.detect' do
    it 'detects MIME type' do
      path = test_document_path('documents/simple.odt')
      mime_type = described_class.detect(path)

      expect(mime_type).to be_a(String)
      expect(mime_type).not_to be_empty
    end
  end

  describe '.version' do
    it 'returns version string' do
      version = described_class.version
      expect(version).to be_a(String)
      expect(version).to match(/\d+\.\d+/)
    end
  end

  describe '.help' do
    it 'returns help text' do
      help_text = described_class.help
      expect(help_text).to be_a(String)
      expect(help_text).to include('kreuzberg')
    end
  end
end
