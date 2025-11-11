# frozen_string_literal: true

RSpec.describe Kreuzberg::Config do
  describe Kreuzberg::Config::OCR do
    it 'creates with default values' do
      ocr = described_class.new

      expect(ocr.backend).to eq('tesseract')
      expect(ocr.language).to eq('eng')
      expect(ocr.tesseract_config).to be_nil
    end

    it 'creates with custom values' do
      ocr = described_class.new(
        backend: 'easyocr',
        language: 'deu'
      )

      expect(ocr.backend).to eq('easyocr')
      expect(ocr.language).to eq('deu')
    end

    it 'converts to hash' do
      ocr = described_class.new(backend: 'tesseract', language: 'fra')
      hash = ocr.to_h

      expect(hash).to be_a(Hash)
      expect(hash[:backend]).to eq('tesseract')
      expect(hash[:language]).to eq('fra')
    end
  end

  describe Kreuzberg::Config::Chunking do
    it 'creates with default values' do
      chunking = described_class.new

      expect(chunking.max_chars).to eq(1000)
      expect(chunking.max_overlap).to eq(200)
      expect(chunking.preset).to be_nil
      expect(chunking.embedding).to be_nil
    end

    it 'creates with custom values' do
      chunking = described_class.new(
        max_chars: 500,
        max_overlap: 100,
        preset: 'fast'
      )

      expect(chunking.max_chars).to eq(500)
      expect(chunking.max_overlap).to eq(100)
      expect(chunking.preset).to eq('fast')
    end

    it 'converts to hash' do
      chunking = described_class.new(max_chars: 750)
      hash = chunking.to_h

      expect(hash).to be_a(Hash)
      expect(hash[:max_chars]).to eq(750)
    end
  end

  describe Kreuzberg::Config::LanguageDetection do
    it 'creates with default values' do
      lang = described_class.new

      expect(lang.enabled).to be false
      expect(lang.min_confidence).to eq(0.5)
    end

    it 'creates with custom values' do
      lang = described_class.new(enabled: true, min_confidence: 0.9)

      expect(lang.enabled).to be true
      expect(lang.min_confidence).to eq(0.9)
    end

    it 'converts to hash' do
      lang = described_class.new(enabled: true, min_confidence: 0.75)
      hash = lang.to_h

      expect(hash).to be_a(Hash)
      expect(hash[:enabled]).to be true
      expect(hash[:min_confidence]).to eq(0.75)
    end
  end

  describe Kreuzberg::Config::PDF do
    it 'creates with default values' do
      pdf = described_class.new

      expect(pdf.extract_images).to be false
      expect(pdf.passwords).to be_nil
      expect(pdf.extract_metadata).to be true
    end

    it 'creates with custom values' do
      pdf = described_class.new(
        extract_images: true,
        passwords: %w[secret backup]
      )

      expect(pdf.extract_images).to be true
      expect(pdf.passwords).to eq(%w[secret backup])
    end

    it 'converts to hash' do
      pdf = described_class.new(extract_images: true, passwords: ['test'])
      hash = pdf.to_h

      expect(hash).to be_a(Hash)
      expect(hash[:extract_images]).to be true
      expect(hash[:passwords]).to eq(['test'])
    end
  end

  describe Kreuzberg::Config::Extraction do
    it 'creates with default values' do
      config = described_class.new

      expect(config.use_cache).to be true
      expect(config.enable_quality_processing).to be false
      expect(config.force_ocr).to be false
      expect(config.ocr).to be_nil
      expect(config.chunking).to be_nil
      expect(config.language_detection).to be_nil
      expect(config.pdf_options).to be_nil
    end

    it 'creates with custom values' do
      ocr = Kreuzberg::Config::OCR.new(backend: 'easyocr')
      chunking = Kreuzberg::Config::Chunking.new(max_chars: 500)
      lang = Kreuzberg::Config::LanguageDetection.new(enabled: true)
      pdf = Kreuzberg::Config::PDF.new(extract_images: true)

      config = described_class.new(
        use_cache: false,
        enable_quality_processing: true,
        force_ocr: true,
        ocr: ocr,
        chunking: chunking,
        language_detection: lang,
        pdf_options: pdf
      )

      expect(config.use_cache).to be false
      expect(config.enable_quality_processing).to be true
      expect(config.force_ocr).to be true
      expect(config.ocr).to eq(ocr)
      expect(config.chunking).to eq(chunking)
      expect(config.language_detection).to eq(lang)
      expect(config.pdf_options).to eq(pdf)
    end

    it 'accepts hash for nested configs' do
      config = described_class.new(
        ocr: { backend: 'tesseract', language: 'eng' },
        chunking: { max_chars: 500 }
      )

      expect(config.ocr).to be_a(Kreuzberg::Config::OCR)
      expect(config.ocr.backend).to eq('tesseract')
      expect(config.chunking).to be_a(Kreuzberg::Config::Chunking)
      expect(config.chunking.max_chars).to eq(500)
    end

    it 'converts to hash' do
      config = described_class.new(
        use_cache: false,
        ocr: { backend: 'tesseract' }
      )
      hash = config.to_h

      expect(hash).to be_a(Hash)
      expect(hash[:use_cache]).to be false
      expect(hash[:ocr]).to be_a(Hash)
      expect(hash[:ocr][:backend]).to eq('tesseract')
    end

    it 'raises error for invalid config type' do
      expect do
        described_class.new(ocr: 'invalid')
      end.to raise_error(ArgumentError, /Expected.*OCR/)
    end
  end
end
