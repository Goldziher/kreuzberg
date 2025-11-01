# frozen_string_literal: true

RSpec.describe Kreuzberg::Errors do
  describe Kreuzberg::Errors::Error do
    it 'is a StandardError subclass' do
      expect(described_class).to be < StandardError
    end

    it 'can be raised with a message' do
      expect do
        raise described_class, 'Test error'
      end.to raise_error(described_class, 'Test error')
    end
  end

  describe Kreuzberg::Errors::ValidationError do
    it 'is an Error subclass' do
      expect(described_class).to be < Kreuzberg::Errors::Error
    end
  end

  describe Kreuzberg::Errors::ParsingError do
    it 'is an Error subclass' do
      expect(described_class).to be < Kreuzberg::Errors::Error
    end

    it 'stores context' do
      error = described_class.new('Parsing failed', context: { file: 'test.pdf' })
      expect(error.context).to eq({ file: 'test.pdf' })
    end
  end

  describe Kreuzberg::Errors::OCRError do
    it 'is an Error subclass' do
      expect(described_class).to be < Kreuzberg::Errors::Error
    end

    it 'stores context' do
      error = described_class.new('OCR failed', context: { page: 1 })
      expect(error.context).to eq({ page: 1 })
    end
  end

  describe Kreuzberg::Errors::MissingDependencyError do
    it 'is an Error subclass' do
      expect(described_class).to be < Kreuzberg::Errors::Error
    end

    it 'stores dependency name' do
      error = described_class.new('Tesseract not found', dependency: 'tesseract')
      expect(error.dependency).to eq('tesseract')
    end
  end

  describe Kreuzberg::Errors::IOError do
    it 'is an Error subclass' do
      expect(described_class).to be < Kreuzberg::Errors::Error
    end
  end

  describe Kreuzberg::Errors::PluginError do
    it 'is an Error subclass' do
      expect(described_class).to be < Kreuzberg::Errors::Error
    end
  end
end
