# frozen_string_literal: true

require 'spec_helper'

RSpec.describe 'Cache Management' do
  let(:test_pdf) do
    test_document_path('pdfs/5_level_paging_and_5_level_ept_intel_revision_1_1_may_2017.pdf')
  end
  let(:test_text) { test_document_path('text/contract_test.txt') }
  let(:test_docx) { test_document_path('documents/contract.docx') }

  before do
    Kreuzberg.clear_cache
  end

  after do
    Kreuzberg.clear_cache
  end

  describe 'clear_cache' do
    it 'removes all cached results' do
      Kreuzberg.extract_file_sync(test_pdf)
      Kreuzberg.extract_file_sync(test_text)

      stats_before = Kreuzberg.cache_stats
      expect(stats_before['total_entries']).to be_positive

      Kreuzberg.clear_cache

      stats_after = Kreuzberg.cache_stats
      expect(stats_after['total_entries']).to eq(0)
      expect(stats_after['total_size_bytes']).to eq(0)
    end

    it 'returns nil (void return)' do
      result = Kreuzberg.clear_cache
      expect(result).to be_nil
    end

    it 'can be called multiple times safely' do
      Kreuzberg.clear_cache
      Kreuzberg.clear_cache
      Kreuzberg.clear_cache

      stats = Kreuzberg.cache_stats
      expect(stats['total_entries']).to eq(0)
    end

    it 'does not affect future extractions' do
      Kreuzberg.extract_file_sync(test_pdf)
      Kreuzberg.clear_cache

      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end
  end

  describe 'cache_stats' do
    it 'returns hash with correct structure' do
      stats = Kreuzberg.cache_stats

      expect(stats).to be_a(Hash)
      expect(stats).to have_key('total_entries')
      expect(stats).to have_key('total_size_bytes')
    end

    it 'returns zero stats when cache is empty' do
      Kreuzberg.clear_cache
      stats = Kreuzberg.cache_stats

      expect(stats['total_entries']).to eq(0)
      expect(stats['total_size_bytes']).to eq(0)
    end

    it 'shows entries after extractions' do
      Kreuzberg.clear_cache

      Kreuzberg.extract_file_sync(test_pdf)
      stats = Kreuzberg.cache_stats

      expect(stats['total_entries']).to be_positive
    end

    it 'shows total size in bytes' do
      Kreuzberg.clear_cache

      Kreuzberg.extract_file_sync(test_pdf)
      stats = Kreuzberg.cache_stats

      expect(stats['total_size_bytes']).to be_positive
    end

    it 'increases stats with multiple extractions' do
      Kreuzberg.clear_cache

      Kreuzberg.extract_file_sync(test_pdf)
      stats_after_one = Kreuzberg.cache_stats

      Kreuzberg.extract_file_sync(test_text)
      stats_after_two = Kreuzberg.cache_stats

      expect(stats_after_two['total_entries']).to be >= stats_after_one['total_entries']
    end
  end

  describe 'cache behavior across extractions' do
    it 'caches extraction results' do
      Kreuzberg.clear_cache
      stats_initial = Kreuzberg.cache_stats
      expect(stats_initial['total_entries']).to eq(0)

      result1 = Kreuzberg.extract_file_sync(test_pdf)
      stats_after_first = Kreuzberg.cache_stats
      expect(stats_after_first['total_entries']).to be_positive

      result2 = Kreuzberg.extract_file_sync(test_pdf)
      stats_after_second = Kreuzberg.cache_stats

      expect(result1.content).to eq(result2.content)
      expect(stats_after_second['total_entries']).to eq(stats_after_first['total_entries'])
    end

    it 'tracks different files separately' do
      Kreuzberg.clear_cache

      Kreuzberg.extract_file_sync(test_pdf)
      stats_after_pdf = Kreuzberg.cache_stats

      Kreuzberg.extract_file_sync(test_text)
      stats_after_text = Kreuzberg.cache_stats

      expect(stats_after_text['total_entries']).to be >= stats_after_pdf['total_entries']
    end

    it 'second extraction of same file may use cache' do
      Kreuzberg.clear_cache

      Time.now
      result1 = Kreuzberg.extract_file_sync(test_pdf)
      Time.now

      Time.now
      result2 = Kreuzberg.extract_file_sync(test_pdf)
      Time.now

      expect(result1.content).to eq(result2.content)
      expect(result1.mime_type).to eq(result2.mime_type)
    end

    it 'clears cache between extractions when requested' do
      result1 = Kreuzberg.extract_file_sync(test_pdf)

      Kreuzberg.clear_cache

      result2 = Kreuzberg.extract_file_sync(test_pdf)

      expect(result1.content).to eq(result2.content)
    end
  end

  describe 'cache with different configurations' do
    it 'may cache based on config differences' do
      Kreuzberg.clear_cache

      config1 = Kreuzberg::Config::Extraction.new(
        chunking: Kreuzberg::Config::Chunking.new(
          enabled: true,
          chunk_size: 100
        )
      )

      config2 = Kreuzberg::Config::Extraction.new(
        chunking: Kreuzberg::Config::Chunking.new(
          enabled: true,
          chunk_size: 200
        )
      )

      result1 = Kreuzberg.extract_file_sync(test_text, config: config1)
      Kreuzberg.cache_stats

      result2 = Kreuzberg.extract_file_sync(test_text, config: config2)
      Kreuzberg.cache_stats

      expect(result1.chunks).not_to be_nil
      expect(result2.chunks).not_to be_nil
    end
  end

  describe 'cache stats consistency' do
    it 'stats remain consistent after clear' do
      Kreuzberg.extract_file_sync(test_pdf)
      Kreuzberg.extract_file_sync(test_text)

      Kreuzberg.clear_cache
      stats = Kreuzberg.cache_stats

      expect(stats['total_entries']).to eq(0)
      expect(stats['total_size_bytes']).to eq(0)
    end

    it 'stats update correctly after new extractions' do
      Kreuzberg.clear_cache

      Kreuzberg.extract_file_sync(test_pdf)
      Kreuzberg.cache_stats

      Kreuzberg.clear_cache

      Kreuzberg.extract_file_sync(test_text)
      stats2 = Kreuzberg.cache_stats

      expect(stats2['total_entries']).to be_positive
    end
  end

  describe 'integration with batch operations' do
    it 'caches batch extraction results' do
      Kreuzberg.clear_cache

      results = Kreuzberg.batch_extract_files_sync([test_pdf, test_text])
      stats = Kreuzberg.cache_stats

      expect(results.length).to eq(2)
      expect(stats['total_entries']).to be_positive
    end

    it 'clear_cache affects batch extractions' do
      Kreuzberg.batch_extract_files_sync([test_pdf, test_text])

      Kreuzberg.clear_cache

      stats = Kreuzberg.cache_stats
      expect(stats['total_entries']).to eq(0)
    end
  end
end
