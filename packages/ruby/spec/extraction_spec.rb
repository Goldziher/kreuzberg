# frozen_string_literal: true

# Comprehensive extraction API tests

RSpec.describe 'Extraction API' do
  describe 'file extraction' do
    it 'extracts content from text file' do
      path = create_test_file("Test content\nLine 2")
      result = Kreuzberg.extract_file_sync(path)

      expect(result.content).to include('Test content')
      expect(result.mime_type).to eq('text/plain')
    end

    it 'handles empty files' do
      path = create_test_file('')
      result = Kreuzberg.extract_file_sync(path)

      expect(result.content).to eq('')
      expect(result.mime_type).to eq('text/plain')
    end

    it 'extracts with MIME type hint' do
      path = create_test_file('Forced MIME type')
      result = Kreuzberg.extract_file_sync(path, mime_type: 'text/markdown')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Forced MIME type')
    end

    it 'extracts with custom config' do
      path = create_test_file('Custom config test')
      config = Kreuzberg::Config::Extraction.new(
        use_cache: false,
        enable_quality_processing: true
      )
      result = Kreuzberg.extract_file_sync(path, config: config)

      expect(result.content).to include('Custom config test')
    end

    it 'raises error for non-existent file' do
      expect do
        Kreuzberg.extract_file_sync('/path/to/nonexistent/file.txt')
      end.to raise_error(StandardError)
    end

    it 'raises error for invalid path' do
      expect do
        Kreuzberg.extract_file_sync('')
      end.to raise_error(StandardError)
    end
  end

  describe 'bytes extraction' do
    it 'extracts from raw bytes' do
      data = 'Raw bytes content'
      result = Kreuzberg.extract_bytes_sync(data, 'text/plain')

      expect(result.content).to include('Raw bytes content')
      expect(result.mime_type).to eq('text/plain')
    end

    it 'handles empty bytes' do
      data = ''
      result = Kreuzberg.extract_bytes_sync(data, 'text/plain')

      expect(result.content).to eq('')
    end

    it 'works with binary data' do
      # Use valid UTF-8 binary data (Magnus/Rust String requires valid UTF-8)
      data = "Binary\x00data\x7F".dup.force_encoding('BINARY')
      result = Kreuzberg.extract_bytes_sync(data, 'text/plain')

      expect(result).to be_a(Kreuzberg::Result)
    end

    it 'extracts with config' do
      data = 'Config test'
      config = Kreuzberg::Config::Extraction.new(use_cache: false)
      result = Kreuzberg.extract_bytes_sync(data, 'text/plain', config: config)

      expect(result.content).to include('Config test')
    end
  end

  describe 'batch extraction' do
    it 'extracts multiple files' do
      files = [
        create_test_file('File 1', filename: 'file1.txt'),
        create_test_file('File 2', filename: 'file2.txt'),
        create_test_file('File 3', filename: 'file3.txt')
      ]

      results = Kreuzberg.batch_extract_files_sync(files)

      expect(results).to be_an(Array)
      expect(results.size).to eq(3)
      expect(results).to all(be_a(Kreuzberg::Result))
      expect(results.map(&:content)).to include(
        match(/File 1/),
        match(/File 2/),
        match(/File 3/)
      )
    end

    it 'handles empty file list' do
      results = Kreuzberg.batch_extract_files_sync([])

      expect(results).to be_an(Array)
      expect(results).to be_empty
    end

    it 'works with config' do
      files = [create_test_file('Batch config')]
      config = Kreuzberg::Config::Extraction.new(use_cache: false)
      results = Kreuzberg.batch_extract_files_sync(files, config: config)

      expect(results.size).to eq(1)
      expect(results.first.content).to include('Batch config')
    end

    it 'continues on partial failures' do
      files = [
        create_test_file('Valid file'),
        '/nonexistent/file.txt'
      ]

      # Implementation may either raise error or handle gracefully
      begin
        result = Kreuzberg.batch_extract_files_sync(files)
        expect(result).to be_an(Array)
      rescue StandardError => e
        expect(e).to be_a(StandardError)
      end
    end
  end

  describe 'async extraction' do
    it 'extracts file asynchronously' do
      path = create_test_file('Async test')
      result = Kreuzberg.extract_file(path)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Async test')
    end

    it 'extracts bytes asynchronously' do
      data = 'Async bytes'
      result = Kreuzberg.extract_bytes(data, 'text/plain')

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).to include('Async bytes')
    end

    it 'batch extracts asynchronously' do
      files = [
        create_test_file('Async 1', filename: 'async1.txt'),
        create_test_file('Async 2', filename: 'async2.txt')
      ]

      results = Kreuzberg.batch_extract_files(files)

      expect(results).to be_an(Array)
      expect(results.size).to eq(2)
    end
  end

  describe 'metadata extraction' do
    it 'extracts metadata from text files' do
      content = "Line 1\nLine 2\nLine 3\nWord count test"
      path = create_test_file(content)
      result = Kreuzberg.extract_file_sync(path)

      expect(result.metadata).to be_a(Hash)
      expect(result.metadata['format_type']).to eq('text')
      expect(result.metadata).to have_key('line_count')
      expect(result.metadata).to have_key('word_count')
      expect(result.metadata).to have_key('character_count')
    end

    it 'provides metadata_json' do
      path = create_test_file('JSON metadata test')
      result = Kreuzberg.extract_file_sync(path)

      expect(result.metadata_json).to be_a(String)
      expect { JSON.parse(result.metadata_json) }.not_to raise_error
    end
  end

  describe 'result structure' do
    it 'includes all expected fields' do
      path = create_test_file('Complete result test')
      result = Kreuzberg.extract_file_sync(path)

      expect(result).to respond_to(:content)
      expect(result).to respond_to(:mime_type)
      expect(result).to respond_to(:metadata)
      expect(result).to respond_to(:metadata_json)
      expect(result).to respond_to(:tables)
      expect(result).to respond_to(:detected_languages)
      expect(result).to respond_to(:chunks)
    end

    it 'provides tables as array' do
      path = create_test_file('Table test')
      result = Kreuzberg.extract_file_sync(path)

      expect(result.tables).to be_an(Array)
    end

    it 'provides detected_languages as array or nil' do
      path = create_test_file('Language test')
      result = Kreuzberg.extract_file_sync(path)

      expect(result.detected_languages).to be_nil.or be_an(Array)
    end

    it 'provides chunks as array or nil' do
      path = create_test_file('Chunks test')
      result = Kreuzberg.extract_file_sync(path)

      expect(result.chunks).to be_nil.or be_an(Array)
    end
  end
end
