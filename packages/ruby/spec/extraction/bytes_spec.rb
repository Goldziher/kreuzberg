# frozen_string_literal: true

require 'spec_helper'

RSpec.describe 'Byte Extraction APIs' do
  let(:test_pdf) do
    '/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf'
  end
  let(:test_text) { '/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/text/contract_test.txt' }
  let(:test_docx) { '/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/documents/contract.docx' }

  describe 'extract_bytes_sync' do
    it 'extracts content from PDF bytes' do
      bytes = File.binread(test_pdf)
      result = Kreuzberg.extract_bytes_sync(bytes, 'application/pdf')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
      expect(result.mime_type).to eq('application/pdf')
    end

    it 'extracts content from text bytes' do
      bytes = File.binread(test_text)
      result = Kreuzberg.extract_bytes_sync(bytes, 'text/plain')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
      expect(result.mime_type).to eq('text/plain')
    end

    it 'uses MIME type hint for extraction' do
      bytes = File.binread(test_pdf)
      result = Kreuzberg.extract_bytes_sync(bytes, 'application/pdf')

      expect(result.mime_type).to eq('application/pdf')
      expect(result.content).to include('Intel')
    end

    it 'extracts metadata from bytes' do
      bytes = File.binread(test_pdf)
      result = Kreuzberg.extract_bytes_sync(bytes, 'application/pdf')

      expect(result.metadata).to be_a(Hash)
      expect(result.metadata).not_to be_empty
    end

    it 'accepts extraction configuration' do
      bytes = File.binread(test_text)
      config = Kreuzberg::Config::Extraction.new(
        chunking: Kreuzberg::Config::Chunking.new(
          enabled: true,
          chunk_size: 100
        )
      )

      result = Kreuzberg.extract_bytes_sync(bytes, 'text/plain', config: config)

      expect(result.chunks).not_to be_nil
      expect(result.chunks).not_to be_empty
    end
  end

  describe 'extract_bytes (async)' do
    it 'extracts content from PDF bytes asynchronously' do
      bytes = File.binread(test_pdf)
      result = Kreuzberg.extract_bytes(bytes, 'application/pdf')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
      expect(result.mime_type).to eq('application/pdf')
    end

    it 'extracts content from text bytes asynchronously' do
      bytes = File.binread(test_text)
      result = Kreuzberg.extract_bytes(bytes, 'text/plain')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end

    it 'accepts extraction configuration' do
      bytes = File.binread(test_text)
      config = Kreuzberg::Config::Extraction.new(
        language_detection: Kreuzberg::Config::LanguageDetection.new(enabled: true)
      )

      result = Kreuzberg.extract_bytes(bytes, 'text/plain', config: config)

      expect(result.detected_languages).not_to be_nil
    end
  end

  describe 'batch_extract_bytes_sync' do
    it 'extracts content from multiple byte arrays' do
      bytes1 = File.binread(test_pdf)
      bytes2 = File.binread(test_text)

      results = Kreuzberg.batch_extract_bytes_sync(
        [bytes1, bytes2],
        ['application/pdf', 'text/plain']
      )

      expect(results).to be_an(Array)
      expect(results.length).to eq(2)
      expect(results[0]).to be_a(Kreuzberg::Result)
      expect(results[1]).to be_a(Kreuzberg::Result)
      expect(results[0].mime_type).to eq('application/pdf')
      expect(results[1].mime_type).to eq('text/plain')
    end

    it 'processes all files even if some fail' do
      bytes1 = File.binread(test_pdf)
      bytes2 = 'invalid bytes'
      bytes3 = File.binread(test_text)

      results = Kreuzberg.batch_extract_bytes_sync(
        [bytes1, bytes2, bytes3],
        ['application/pdf', 'application/pdf', 'text/plain']
      )

      expect(results).to be_an(Array)
      expect(results.length).to eq(3)
      expect(results[0].content).not_to be_empty
      expect(results[2].content).not_to be_empty
    end

    it 'accepts extraction configuration for batch processing' do
      bytes1 = File.binread(test_text)
      bytes2 = File.binread(test_text)

      config = Kreuzberg::Config::Extraction.new(
        chunking: Kreuzberg::Config::Chunking.new(
          enabled: true,
          chunk_size: 100
        )
      )

      results = Kreuzberg.batch_extract_bytes_sync(
        [bytes1, bytes2],
        ['text/plain', 'text/plain'],
        config: config
      )

      expect(results.length).to eq(2)
      results.each do |result|
        expect(result.chunks).not_to be_nil
        expect(result.chunks).not_to be_empty
      end
    end

    it 'requires matching array lengths' do
      bytes1 = File.binread(test_pdf)
      bytes2 = File.binread(test_text)

      expect do
        Kreuzberg.batch_extract_bytes_sync(
          [bytes1, bytes2],
          ['application/pdf']
        )
      end.to raise_error
    end
  end

  describe 'batch_extract_bytes (async)' do
    it 'extracts content from multiple byte arrays asynchronously' do
      bytes1 = File.binread(test_pdf)
      bytes2 = File.binread(test_text)

      results = Kreuzberg.batch_extract_bytes(
        [bytes1, bytes2],
        ['application/pdf', 'text/plain']
      )

      expect(results).to be_an(Array)
      expect(results.length).to eq(2)
      expect(results[0]).to be_a(Kreuzberg::Result)
      expect(results[1]).to be_a(Kreuzberg::Result)
      expect(results[0].content).not_to be_empty
      expect(results[1].content).not_to be_empty
    end

    it 'accepts extraction configuration for async batch processing' do
      bytes1 = File.binread(test_text)
      bytes2 = File.binread(test_text)

      config = Kreuzberg::Config::Extraction.new(
        language_detection: Kreuzberg::Config::LanguageDetection.new(enabled: true)
      )

      results = Kreuzberg.batch_extract_bytes(
        [bytes1, bytes2],
        ['text/plain', 'text/plain'],
        config: config
      )

      expect(results.length).to eq(2)
      results.each do |result|
        expect(result.detected_languages).not_to be_nil
      end
    end
  end

  describe 'error handling' do
    it 'raises error for invalid bytes with wrong MIME type' do
      bytes = 'not a valid PDF'

      expect do
        Kreuzberg.extract_bytes_sync(bytes, 'application/pdf')
      end.to raise_error
    end

    it 'raises error for empty bytes' do
      expect do
        Kreuzberg.extract_bytes_sync('', 'application/pdf')
      end.to raise_error
    end

    it 'raises error when MIME type is nil' do
      bytes = File.binread(test_pdf)

      expect do
        Kreuzberg.extract_bytes_sync(bytes, nil)
      end.to raise_error
    end
  end

  describe 'encoding handling' do
    it 'handles binary encoding correctly' do
      bytes = File.binread(test_pdf)
      expect(bytes.encoding).to eq(Encoding::BINARY)

      result = Kreuzberg.extract_bytes_sync(bytes, 'application/pdf')

      expect(result.content).to be_a(String)
      expect(result.content.encoding).to eq(Encoding::UTF_8)
    end

    it 'preserves binary data integrity' do
      original_bytes = File.binread(test_pdf)
      bytes_copy = original_bytes.dup

      Kreuzberg.extract_bytes_sync(bytes_copy, 'application/pdf')

      expect(bytes_copy).to eq(original_bytes)
    end
  end
end
