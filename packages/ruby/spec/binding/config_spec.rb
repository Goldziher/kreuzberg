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
    describe '.from_file' do
      it 'loads configuration from TOML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.toml')
        config = described_class.from_file(config_path)

        expect(config.use_cache).to be false
        expect(config.enable_quality_processing).to be true
        expect(config.force_ocr).to be true
      end

      it 'loads OCR config from TOML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.toml')
        config = described_class.from_file(config_path)

        expect(config.ocr).to be_a(Kreuzberg::Config::OCR)
        expect(config.ocr.backend).to eq('tesseract')
        expect(config.ocr.language).to eq('deu')
      end

      it 'loads chunking config from TOML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.toml')
        config = described_class.from_file(config_path)

        expect(config.chunking).to be_a(Kreuzberg::Config::Chunking)
        expect(config.chunking.max_chars).to eq(500)
        expect(config.chunking.max_overlap).to eq(100)
        expect(config.chunking.preset).to eq('fast')
      end

      it 'loads language detection config from TOML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.toml')
        config = described_class.from_file(config_path)

        expect(config.language_detection).to be_a(Kreuzberg::Config::LanguageDetection)
        expect(config.language_detection.enabled).to be true
        expect(config.language_detection.min_confidence).to eq(0.9)
      end

      it 'loads PDF options from TOML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.toml')
        config = described_class.from_file(config_path)

        expect(config.pdf_options).to be_a(Kreuzberg::Config::PDF)
        expect(config.pdf_options.extract_images).to be true
        expect(config.pdf_options.passwords).to eq(%w[secret backup])
        expect(config.pdf_options.extract_metadata).to be true
      end

      it 'loads configuration from YAML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.yaml')
        config = described_class.from_file(config_path)

        expect(config.use_cache).to be false
        expect(config.enable_quality_processing).to be true
        expect(config.force_ocr).to be true
      end

      it 'loads OCR config from YAML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.yaml')
        config = described_class.from_file(config_path)

        expect(config.ocr).to be_a(Kreuzberg::Config::OCR)
        expect(config.ocr.backend).to eq('tesseract')
        expect(config.ocr.language).to eq('fra')
      end

      it 'loads chunking config from YAML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.yaml')
        config = described_class.from_file(config_path)

        expect(config.chunking).to be_a(Kreuzberg::Config::Chunking)
        expect(config.chunking.max_chars).to eq(750)
        expect(config.chunking.max_overlap).to eq(150)
        expect(config.chunking.preset).to eq('balanced')
      end

      it 'works with absolute paths' do
        config_path = File.expand_path('../fixtures/config.toml', __dir__)
        config = described_class.from_file(config_path)

        expect(config.use_cache).to be false
      end

      it 'works with relative paths' do
        config_path = File.join(__dir__, '..', 'fixtures', 'config.yaml')
        config = described_class.from_file(config_path)

        expect(config.use_cache).to be false
      end

      it 'raises error for non-existent file' do
        expect do
          described_class.from_file('/path/to/nonexistent/config.toml')
        end.to raise_error(Kreuzberg::Errors::ValidationError, /Failed to read config file/)
      end

      it 'raises error for invalid TOML file' do
        config_path = File.join(__dir__, '..', 'fixtures', 'invalid_config.toml')
        expect do
          described_class.from_file(config_path)
        end.to raise_error(Kreuzberg::Errors::ValidationError, /Invalid TOML/)
      end

      it 'detects file format from extension' do
        toml_path = File.join(__dir__, '..', 'fixtures', 'config.toml')
        yaml_path = File.join(__dir__, '..', 'fixtures', 'config.yaml')

        toml_config = described_class.from_file(toml_path)
        yaml_config = described_class.from_file(yaml_path)

        expect(toml_config.ocr.language).to eq('deu')
        expect(yaml_config.ocr.language).to eq('fra')
      end
    end

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

  describe 'ExtractionConfig alias' do
    it 'exists at module level' do
      expect(Kreuzberg.const_defined?(:ExtractionConfig)).to be true
    end

    it 'is the same class as Config::Extraction' do
      expect(Kreuzberg::ExtractionConfig).to eq(Kreuzberg::Config::Extraction)
    end

    it 'can be instantiated using the alias' do
      config = Kreuzberg::ExtractionConfig.new(use_cache: false)

      expect(config).to be_a(Kreuzberg::Config::Extraction)
      expect(config.use_cache).to be false
    end

    it 'supports all methods through the alias' do
      config = Kreuzberg::ExtractionConfig.new(
        use_cache: false,
        force_ocr: true,
        ocr: { backend: 'tesseract', language: 'eng' }
      )

      expect(config.use_cache).to be false
      expect(config.force_ocr).to be true
      expect(config.ocr).to be_a(Kreuzberg::Config::OCR)
      expect(config.ocr.backend).to eq('tesseract')

      hash = config.to_h
      expect(hash[:use_cache]).to be false
      expect(hash[:force_ocr]).to be true
    end

    it 'supports from_file through the alias' do
      config_path = File.join(__dir__, '..', 'fixtures', 'config.toml')
      config = Kreuzberg::ExtractionConfig.from_file(config_path)

      expect(config).to be_a(Kreuzberg::Config::Extraction)
      expect(config.use_cache).to be false
      expect(config.enable_quality_processing).to be true
    end
  end
end
