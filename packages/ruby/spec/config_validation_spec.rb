# frozen_string_literal: true

# Configuration validation tests

RSpec.describe 'Configuration Validation' do
  describe Kreuzberg::Config::Extraction do
    it 'accepts all valid parameters' do
      config = described_class.new(
        use_cache: true,
        enable_quality_processing: false,
        force_ocr: false,
        ocr: Kreuzberg::Config::OCR.new,
        chunking: Kreuzberg::Config::Chunking.new,
        language_detection: Kreuzberg::Config::LanguageDetection.new,
        pdf_options: Kreuzberg::Config::PDF.new
      )

      expect(config.use_cache).to be true
      expect(config.enable_quality_processing).to be false
      expect(config.force_ocr).to be false
      expect(config.ocr).to be_a(Kreuzberg::Config::OCR)
      expect(config.chunking).to be_a(Kreuzberg::Config::Chunking)
      expect(config.language_detection).to be_a(Kreuzberg::Config::LanguageDetection)
      expect(config.pdf_options).to be_a(Kreuzberg::Config::PDF)
    end

    it 'accepts hashes for nested configs' do
      config = described_class.new(
        ocr: { backend: 'tesseract', language: 'eng' },
        chunking: { max_chars: 500 }
      )

      expect(config.ocr).to be_a(Kreuzberg::Config::OCR)
      expect(config.ocr.backend).to eq('tesseract')
      expect(config.chunking).to be_a(Kreuzberg::Config::Chunking)
      expect(config.chunking.max_chars).to eq(500)
    end

    it 'validates ocr config type' do
      expect do
        described_class.new(ocr: 'invalid')
      end.to raise_error(ArgumentError, /Expected.*OCR/)
    end

    it 'validates chunking config type' do
      expect do
        described_class.new(chunking: 'invalid')
      end.to raise_error(ArgumentError, /Expected.*Chunking/)
    end

    it 'converts to hash correctly' do
      config = described_class.new(
        use_cache: false,
        force_ocr: true
      )
      hash = config.to_h

      expect(hash).to be_a(Hash)
      expect(hash[:use_cache]).to be false
      expect(hash[:force_ocr]).to be true
    end

    it 'omits nil values from hash' do
      config = described_class.new
      hash = config.to_h

      expect(hash[:ocr]).to be_nil
      expect(hash[:chunking]).to be_nil
    end
  end

  describe Kreuzberg::Config::OCR do
    it 'has sensible defaults' do
      config = described_class.new

      expect(config.backend).to eq('tesseract')
      expect(config.language).to eq('eng')
      expect(config.tesseract_config).to be_nil
    end

    it 'accepts custom values' do
      config = described_class.new(
        backend: 'easyocr',
        language: 'deu'
      )

      expect(config.backend).to eq('easyocr')
      expect(config.language).to eq('deu')
    end

    it 'coerces types correctly' do
      config = described_class.new(
        backend: :tesseract,
        language: 123
      )

      expect(config.backend).to eq('tesseract')
      expect(config.language).to eq('123')
    end
  end

  describe Kreuzberg::Config::Chunking do
    it 'has sensible defaults' do
      config = described_class.new

      expect(config.max_chars).to eq(1000)
      expect(config.max_overlap).to eq(200)
      expect(config.preset).to be_nil
    end

    it 'accepts custom chunk sizes' do
      config = described_class.new(
        max_chars: 500,
        max_overlap: 100
      )

      expect(config.max_chars).to eq(500)
      expect(config.max_overlap).to eq(100)
    end

    it 'supports different strategies' do
      config = described_class.new(preset: 'fast')
      expect(config.preset).to eq('fast')
    end
  end

  describe Kreuzberg::Config::LanguageDetection do
    it 'has sensible defaults' do
      config = described_class.new

      expect(config.enabled).to be false
      expect(config.min_confidence).to eq(0.5)
    end

    it 'accepts custom confidence thresholds' do
      config = described_class.new(
        enabled: true,
        min_confidence: 0.9
      )

      expect(config.enabled).to be true
      expect(config.min_confidence).to eq(0.9)
    end

    it 'coerces confidence to float' do
      config = described_class.new(min_confidence: '0.75')
      expect(config.min_confidence).to eq(0.75)
    end
  end

  describe Kreuzberg::Config::PDF do
    it 'has sensible defaults' do
      config = described_class.new

      expect(config.extract_images).to be false
      expect(config.passwords).to be_nil
      expect(config.extract_metadata).to be true
    end

    it 'accepts custom values' do
      config = described_class.new(
        extract_images: true,
        passwords: ['secret123']
      )

      expect(config.extract_images).to be true
      expect(config.passwords).to eq(['secret123'])
    end

    it 'converts password to string' do
      config = described_class.new(passwords: 12_345)
      expect(config.passwords).to eq(['12345'])
    end
  end

  describe 'config usage in extraction' do
    it 'works with OCR config' do
      path = create_test_file('OCR config test')
      config = Kreuzberg::Config::Extraction.new(
        ocr: Kreuzberg::Config::OCR.new(backend: 'tesseract', language: 'eng')
      )

      result = Kreuzberg.extract_file_sync(path, config: config)
      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'works with chunking config' do
      path = create_test_file('Chunking config test' * 50)
      config = Kreuzberg::Config::Extraction.new(
        chunking: Kreuzberg::Config::Chunking.new(max_chars: 50)
      )

      result = Kreuzberg.extract_file_sync(path, config: config)
      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'works with language detection config' do
      path = create_test_file('Language detection test')
      config = Kreuzberg::Config::Extraction.new(
        language_detection: Kreuzberg::Config::LanguageDetection.new(enabled: true)
      )

      result = Kreuzberg.extract_file_sync(path, config: config)
      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'works with combined configs' do
      path = create_test_file('Combined config test')
      config = Kreuzberg::Config::Extraction.new(
        use_cache: false,
        force_ocr: false,
        ocr: { backend: 'tesseract', language: 'eng' },
        language_detection: { enabled: false }
      )

      result = Kreuzberg.extract_file_sync(path, config: config)
      expect(result).to be_a(Kreuzberg::Result)
    end
  end
end
