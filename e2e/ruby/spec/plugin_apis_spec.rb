# frozen_string_literal: true

# Auto-generated from fixtures/plugin_api/ - DO NOT EDIT

# E2E tests for plugin/config/utility APIs.
#
# Generated from plugin API fixtures.
# To regenerate: cargo run -p kreuzberg-e2e-generator -- generate --lang ruby

# rubocop:disable Metrics/BlockLength

require 'spec_helper'
require 'tmpdir'
require 'fileutils'

RSpec.describe 'Configuration' do
  it 'Discover configuration from current or parent directories' do
    Dir.mktmpdir do |tmpdir|
      config_path = File.join(tmpdir, 'kreuzberg.toml')
      File.write(config_path, <<~TOML)
        [chunking]
        max_chars = 50
      TOML

      subdir = File.join(tmpdir, 'subdir')
      FileUtils.mkdir_p(subdir)

      FileUtils.cd(subdir) do
        config = Kreuzberg::Config::Extraction.discover

        expect(config.chunking).not_to be_nil
        expect(config.chunking.max_chars).to eq(50)
      end
    end
  end

  it 'Load configuration from a TOML file' do
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
      expect(config.language_detection.enabled).to eq(false)
    end
  end

end

RSpec.describe 'Document Extractor Management' do
  it 'Clear all document extractors and verify list is empty' do
    Kreuzberg.clear_document_extractors
    result = Kreuzberg.list_document_extractors
    expect(result).to be_empty
  end

  it 'List all registered document extractors' do
    result = Kreuzberg.list_document_extractors
    expect(result).to be_an(Array)
    expect(result).to all(be_a(String))
  end

  it 'Unregister nonexistent document extractor gracefully' do
    expect { Kreuzberg.unregister_document_extractor('nonexistent-extractor-xyz') }.not_to raise_error
  end

end

RSpec.describe 'Mime Utilities' do
  it 'Detect MIME type from file bytes' do
    test_bytes = '%PDF-1.4\\n'.dup.force_encoding('ASCII-8BIT')
    result = Kreuzberg.detect_mime_type(test_bytes)
    expect(result.downcase).to include('pdf')
  end

  it 'Detect MIME type from file path' do
    Dir.mktmpdir do |tmpdir|
      test_file = File.join(tmpdir, 'test.txt')
      File.write(test_file, 'Hello, world!')

      result = Kreuzberg.detect_mime_type_from_path(test_file)
      expect(result.downcase).to include('text')
    end
  end

  it 'Get file extensions for a MIME type' do
    result = Kreuzberg.get_extensions_for_mime('application/pdf')
    expect(result).to be_an(Array)
    expect(result).to include('pdf')
  end

end

RSpec.describe 'Ocr Backend Management' do
  it 'Clear all OCR backends and verify list is empty' do
    Kreuzberg.clear_ocr_backends
    result = Kreuzberg.list_ocr_backends
    expect(result).to be_empty
  end

  it 'List all registered OCR backends' do
    result = Kreuzberg.list_ocr_backends
    expect(result).to be_an(Array)
    expect(result).to all(be_a(String))
  end

  it 'Unregister nonexistent OCR backend gracefully' do
    expect { Kreuzberg.unregister_ocr_backend('nonexistent-backend-xyz') }.not_to raise_error
  end

end

RSpec.describe 'Post Processor Management' do
  it 'Clear all post-processors and verify list is empty' do
    Kreuzberg.clear_post_processors
    result = Kreuzberg.list_post_processors
    expect(result).to be_empty
  end

  it 'List all registered post-processors' do
    result = Kreuzberg.list_post_processors
    expect(result).to be_an(Array)
    expect(result).to all(be_a(String))
  end

end

RSpec.describe 'Validator Management' do
  it 'Clear all validators and verify list is empty' do
    Kreuzberg.clear_validators
    result = Kreuzberg.list_validators
    expect(result).to be_empty
  end

  it 'List all registered validators' do
    result = Kreuzberg.list_validators
    expect(result).to be_an(Array)
    expect(result).to all(be_a(String))
  end

end

# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
