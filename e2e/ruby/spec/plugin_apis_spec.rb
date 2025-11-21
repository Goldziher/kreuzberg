# frozen_string_literal: true

# Auto-generated plugin API tests for Ruby binding.
#
# E2E tests for plugin registration and management APIs.
#
# Tests all plugin types:
# - Validators
# - Post-processors
# - OCR backends
# - Document extractors
#
# Tests all management operations:
# - Registration
# - Unregistration
# - Listing
# - Clearing

require 'spec_helper'
require 'tempfile'
require 'fileutils'

RSpec.describe 'Validator APIs' do
  it 'lists all registered validators' do
    validators = Kreuzberg.list_validators
    expect(validators).to be_an(Array)
    expect(validators).to all(be_a(String))
  end

  it 'clears all validators' do
    Kreuzberg.clear_validators
    validators = Kreuzberg.list_validators
    expect(validators).to be_empty
  end
end

RSpec.describe 'Post-processor APIs' do
  it 'lists all registered post-processors' do
    processors = Kreuzberg.list_post_processors
    expect(processors).to be_an(Array)
    expect(processors).to all(be_a(String))
  end

  it 'clears all post-processors' do
    Kreuzberg.clear_post_processors
    processors = Kreuzberg.list_post_processors
    expect(processors).to be_empty
  end
end

RSpec.describe 'OCR Backend APIs' do
  it 'lists all registered OCR backends' do
    backends = Kreuzberg.list_ocr_backends
    expect(backends).to be_an(Array)
    expect(backends).to all(be_a(String))
    # Should include built-in backends
    expect(backends).to include('tesseract')
  end

  it 'unregisters an OCR backend' do
    # Should handle nonexistent backend gracefully
    expect { Kreuzberg.unregister_ocr_backend('nonexistent-backend-xyz') }.not_to raise_error
  end

  it 'clears all OCR backends' do
    Kreuzberg.clear_ocr_backends
    backends = Kreuzberg.list_ocr_backends
    expect(backends).to be_empty
  end
end

RSpec.describe 'Document Extractor APIs' do
  it 'lists all registered document extractors' do
    extractors = Kreuzberg.list_document_extractors
    expect(extractors).to be_an(Array)
    expect(extractors).to all(be_a(String))
    # Should include built-in extractors
    expect(extractors.any? { |e| e.downcase.include?('pdf') }).to be true
  end

  it 'unregisters a document extractor' do
    # Should handle nonexistent extractor gracefully
    expect { Kreuzberg.unregister_document_extractor('nonexistent-extractor-xyz') }.not_to raise_error
  end

  it 'clears all document extractors' do
    Kreuzberg.clear_document_extractors
    extractors = Kreuzberg.list_document_extractors
    expect(extractors).to be_empty
  end
end

RSpec.describe 'Configuration APIs' do
  it 'loads configuration from a TOML file' do
    Dir.mktmpdir do |tmpdir|
      config_path = File.join(tmpdir, 'test_config.toml')
      File.write(config_path, <<~TOML)
        [chunking]
        max_chars = 100
        max_overlap = 20

        [language_detection]
        enabled = false
      TOML

      config = Kreuzberg::Config::Extraction.from_file(config_path)
      expect(config.chunking).not_to be_nil
      expect(config.chunking.max_chars).to eq(100)
      expect(config.chunking.max_overlap).to eq(20)
      expect(config.language_detection).not_to be_nil
      expect(config.language_detection.enabled).to be false
    end
  end

  it 'discovers configuration from current or parent directories' do
    Dir.mktmpdir do |tmpdir|
      config_path = File.join(tmpdir, 'kreuzberg.toml')
      File.write(config_path, <<~TOML)
        [chunking]
        max_chars = 50
      TOML

      # Create subdirectory
      subdir = File.join(tmpdir, 'subdir')
      FileUtils.mkdir_p(subdir)

      original_dir = Dir.pwd
      begin
        Dir.chdir(subdir)
        config = Kreuzberg::Config::Extraction.discover
        expect(config).not_to be_nil
        expect(config.chunking).not_to be_nil
        expect(config.chunking.max_chars).to eq(50)
      ensure
        Dir.chdir(original_dir)
      end
    end
  end
end

RSpec.describe 'MIME Utilities' do
  it 'detects MIME type from bytes' do
    # PDF magic bytes
    pdf_bytes = '%PDF-1.4\n'
    mime_type = Kreuzberg.detect_mime_type(pdf_bytes)
    expect(mime_type.downcase).to include('pdf')
  end

  it 'detects MIME type from file path' do
    Dir.mktmpdir do |tmpdir|
      test_file = File.join(tmpdir, 'test.txt')
      File.write(test_file, 'Hello, world!')

      mime_type = Kreuzberg.detect_mime_type_from_path(test_file)
      expect(mime_type.downcase).to include('text')
    end
  end

  it 'gets file extensions for a MIME type' do
    extensions = Kreuzberg.get_extensions_for_mime('application/pdf')
    expect(extensions).to be_an(Array)
    expect(extensions).to include('pdf')
  end
end
