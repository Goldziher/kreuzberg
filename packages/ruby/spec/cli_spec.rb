# frozen_string_literal: true

RSpec.describe Kreuzberg::CLI do
  describe '.extract', :skip do
    it 'extracts content from a file' do
      # Skip in environments without CLI binary
      path = create_test_file('CLI test content')
      output = described_class.extract(path)

      expect(output).to be_a(String)
      expect(output).to include('CLI test content')
    end

    it 'accepts output format option' do
      path = create_test_file('JSON output test')
      output = described_class.extract(path, output: 'json')

      expect(output).to be_a(String)
    end

    it 'accepts OCR option' do
      path = create_test_file('OCR test')
      output = described_class.extract(path, ocr: true)

      expect(output).to be_a(String)
    end
  end

  describe '.detect', :skip do
    it 'detects MIME type' do
      path = create_test_file('MIME detection test')
      mime_type = described_class.detect(path)

      expect(mime_type).to be_a(String)
      expect(mime_type).not_to be_empty
    end
  end

  describe '.version', :skip do
    it 'returns version string' do
      version = described_class.version
      expect(version).to be_a(String)
      expect(version).to match(/\d+\.\d+/)
    end
  end

  describe '.help', :skip do
    it 'returns help text' do
      help_text = described_class.help
      expect(help_text).to be_a(String)
      expect(help_text).to include('kreuzberg')
    end
  end
end
