# frozen_string_literal: true

RSpec.describe Kreuzberg do
  describe 'version' do
    it 'has a version number' do
      expect(Kreuzberg::VERSION).not_to be_nil
      expect(Kreuzberg::VERSION).to match(/^\d+\.\d+\.\d+/)
    end
  end

  describe '.extract_file_sync' do
    it 'extracts content from a text file' do
      path = create_test_file('Hello, World!')
      result = described_class.extract_file_sync(path)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Hello, World!')
      expect(result.mime_type).to eq('text/plain')
    end

    it 'includes metadata for text files' do
      path = create_test_file("Line 1\nLine 2\nLine 3")
      result = described_class.extract_file_sync(path)

      expect(result.metadata_json).to be_a(String)
      metadata = JSON.parse(result.metadata_json)
      expect(metadata['format_type']).to eq('text')
      expect(metadata).to have_key('line_count')
      expect(metadata).to have_key('word_count')
      expect(metadata).to have_key('character_count')
    end

    it 'accepts a MIME type hint' do
      path = create_test_file('Test content')
      result = described_class.extract_file_sync(path, mime_type: 'text/plain')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Test content')
    end

    it 'accepts a configuration hash' do
      path = create_test_file('Cached content')
      result = described_class.extract_file_sync(path, config: { use_cache: false })

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Cached content')
    end

    it 'accepts a Config::Extraction object' do
      path = create_test_file('Config object test')
      config = Kreuzberg::Config::Extraction.new(use_cache: false)
      result = described_class.extract_file_sync(path, config: config)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Config object test')
    end

    it 'raises an error for non-existent files' do
      expect do
        described_class.extract_file_sync('/nonexistent/file.txt')
      end.to raise_error(StandardError)
    end

    it 'works with cache disabled' do
      path = create_test_file('No cache test')
      config = Kreuzberg::Config::Extraction.new(use_cache: false)
      result = described_class.extract_file_sync(path, config: config)

      expect(result.content).to include('No cache test')
    end
  end

  describe '.extract_bytes_sync' do
    it 'extracts content from bytes' do
      data = 'Binary content test'
      result = described_class.extract_bytes_sync(data, 'text/plain')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Binary content test')
      expect(result.mime_type).to eq('text/plain')
    end

    it 'accepts a configuration hash' do
      data = 'Bytes with config'
      result = described_class.extract_bytes_sync(data, 'text/plain', config: { use_cache: false })

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Bytes with config')
    end
  end

  describe '.batch_extract_files_sync' do
    it 'extracts content from multiple files' do
      paths = [
        create_test_file('First file', filename: 'file1.txt'),
        create_test_file('Second file', filename: 'file2.txt'),
        create_test_file('Third file', filename: 'file3.txt')
      ]

      results = described_class.batch_extract_files_sync(paths)

      expect(results).to be_an(Array)
      expect(results.size).to eq(3)
      expect(results).to all(be_a(Kreuzberg::Result))
      expect(results.map(&:content)).to include(
        match(/First file/),
        match(/Second file/),
        match(/Third file/)
      )
    end

    it 'accepts a configuration hash' do
      paths = [create_test_file('Batch test')]
      results = described_class.batch_extract_files_sync(paths, config: { use_cache: false })

      expect(results).to be_an(Array)
      expect(results.size).to eq(1)
    end
  end

  describe '.extract_file (async)' do
    it 'extracts content asynchronously' do
      path = create_test_file('Async content')
      result = described_class.extract_file(path)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Async content')
    end
  end

  describe '.extract_bytes (async)' do
    it 'extracts content from bytes asynchronously' do
      data = 'Async bytes'
      result = described_class.extract_bytes(data, 'text/plain')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Async bytes')
    end
  end

  describe '.batch_extract_files (async)' do
    it 'extracts content from multiple files asynchronously' do
      paths = [
        create_test_file('Async file 1', filename: 'async1.txt'),
        create_test_file('Async file 2', filename: 'async2.txt')
      ]

      results = described_class.batch_extract_files(paths)

      expect(results).to be_an(Array)
      expect(results.size).to eq(2)
      expect(results).to all(be_a(Kreuzberg::Result))
    end
  end

  describe '.clear_cache' do
    it 'clears the cache without error' do
      expect { described_class.clear_cache }.not_to raise_error
    end
  end

  describe '.cache_stats' do
    it 'returns cache statistics' do
      stats = described_class.cache_stats

      expect(stats).to be_a(Hash)
      expect(stats).to have_key(:total_entries)
      expect(stats).to have_key(:total_size_bytes)
      expect(stats[:total_entries]).to be_a(Integer)
      expect(stats[:total_size_bytes]).to be_a(Integer)
    end
  end

  describe 'version information' do
    it 'has a version number' do
      expect(Kreuzberg::VERSION).not_to be_nil
      expect(Kreuzberg::VERSION).to match(/^\d+\.\d+\.\d+/)
    end
  end
end
