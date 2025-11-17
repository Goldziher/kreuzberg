# frozen_string_literal: true

# Basic smoke tests to verify package structure and imports work
require 'stringio'

RSpec.describe 'Kreuzberg package' do
  describe 'import and structure' do
    it 'can be required without errors' do
      expect { require 'kreuzberg' }.not_to raise_error
    end

    it 'has a version constant' do
      expect(Kreuzberg::VERSION).not_to be_nil
      expect(Kreuzberg::VERSION).to be_a(String)
      expect(Kreuzberg::VERSION).to match(/^\d+\.\d+\.\d+/)
    end
  end

  describe 'public API exports' do
    describe 'configuration classes' do
      it 'exports Config::Extraction' do
        expect(defined?(Kreuzberg::Config::Extraction)).to eq('constant')
      end

      it 'exports Config::OCR' do
        expect(defined?(Kreuzberg::Config::OCR)).to eq('constant')
      end

      it 'exports Config::Chunking' do
        expect(defined?(Kreuzberg::Config::Chunking)).to eq('constant')
      end

      it 'exports Config::LanguageDetection' do
        expect(defined?(Kreuzberg::Config::LanguageDetection)).to eq('constant')
      end

      it 'exports Config::PDF' do
        expect(defined?(Kreuzberg::Config::PDF)).to eq('constant')
      end

      it 'exports Config::HtmlOptions' do
        expect(defined?(Kreuzberg::Config::HtmlOptions)).to eq('constant')
      end

      it 'exports Config::Keywords' do
        expect(defined?(Kreuzberg::Config::Keywords)).to eq('constant')
      end
    end

    describe 'result classes' do
      it 'exports Result' do
        expect(defined?(Kreuzberg::Result)).to eq('constant')
      end

      it 'exports Result::Table' do
        expect(defined?(Kreuzberg::Result::Table)).to eq('constant')
      end

      it 'exports Result::Chunk' do
        expect(defined?(Kreuzberg::Result::Chunk)).to eq('constant')
      end

      it 'exports Result::Image' do
        expect(defined?(Kreuzberg::Result::Image)).to eq('constant')
      end
    end

    describe 'exception classes' do
      it 'exports Errors::Error' do
        expect(defined?(Kreuzberg::Errors::Error)).to eq('constant')
      end

      it 'exports Errors::ValidationError' do
        expect(defined?(Kreuzberg::Errors::ValidationError)).to eq('constant')
      end

      it 'exports Errors::ParsingError' do
        expect(defined?(Kreuzberg::Errors::ParsingError)).to eq('constant')
      end

      it 'exports Errors::OCRError' do
        expect(defined?(Kreuzberg::Errors::OCRError)).to eq('constant')
      end

      it 'exports Errors::MissingDependencyError' do
        expect(defined?(Kreuzberg::Errors::MissingDependencyError)).to eq('constant')
      end

      it 'exports Errors::IOError' do
        expect(defined?(Kreuzberg::Errors::IOError)).to eq('constant')
      end

      it 'exports Errors::PluginError' do
        expect(defined?(Kreuzberg::Errors::PluginError)).to eq('constant')
      end
    end

    describe 'extraction functions (sync)' do
      it 'exports extract_file_sync' do
        expect(Kreuzberg).to respond_to(:extract_file_sync)
      end

      it 'exports extract_bytes_sync' do
        expect(Kreuzberg).to respond_to(:extract_bytes_sync)
      end

      it 'exports batch_extract_files_sync' do
        expect(Kreuzberg).to respond_to(:batch_extract_files_sync)
      end
    end

    describe 'extraction functions (async)' do
      it 'exports extract_file' do
        expect(Kreuzberg).to respond_to(:extract_file)
      end

      it 'exports extract_bytes' do
        expect(Kreuzberg).to respond_to(:extract_bytes)
      end

      it 'exports batch_extract_files' do
        expect(Kreuzberg).to respond_to(:batch_extract_files)
      end
    end

    describe 'utility modules' do
      it 'exports CLI' do
        expect(defined?(Kreuzberg::CLI)).to eq('constant')
      end

      it 'exports CLIProxy' do
        expect(defined?(Kreuzberg::CLIProxy)).to eq('constant')
      end

      it 'exports APIProxy' do
        expect(defined?(Kreuzberg::APIProxy)).to eq('constant')
      end

      it 'exports MCPProxy' do
        expect(defined?(Kreuzberg::MCPProxy)).to eq('constant')
      end
    end
  end

  describe 'module structure' do
    it 'defines Kreuzberg as a module' do
      expect(Kreuzberg).to be_a(Module)
    end

    it 'defines Kreuzberg::Config as a module' do
      expect(Kreuzberg::Config).to be_a(Module)
    end

    it 'defines Kreuzberg::Errors as a module' do
      expect(Kreuzberg::Errors).to be_a(Module)
    end
  end

  describe 'basic extraction smoke tests' do
    it 'extracts inline text via bytes API' do
      bytes = StringIO.new('Hello from Kreuzberg')
      result = Kreuzberg.extract_bytes_sync(bytes.string, 'text/plain')

      expect(result.content).to include('Hello')
      expect(result.mime_type).to eq('text/plain')
    end

    it 'extracts from small temp file via sync API' do
      file = create_test_file('Simple document for smoke testing')
      result = Kreuzberg.extract_file_sync(file)

      expect(result.content).to include('Simple document')
      expect(result.mime_type).to eq('text/plain')
    ensure
      File.delete(file) if file && File.exist?(file)
    end
  end
end
