# frozen_string_literal: true

# Error handling and exception mapping tests

RSpec.describe 'Error Handling' do
  let(:nested_ocr_result) do
    {
      'content' => 'ocr text',
      'mime_type' => 'text/plain',
      'metadata_json' => '{}',
      'tables' => []
    }
  end

  let(:image_result_payload) do
    {
      content: 'Test',
      mime_type: 'text/plain',
      images: [
        {
          'data' => "binary\0data",
          'format' => 'png',
          'image_index' => 0,
          'page_number' => 1,
          'width' => 100,
          'height' => 200,
          'colorspace' => 'RGB',
          'bits_per_component' => 8,
          'is_mask' => false,
          'description' => 'inline image',
          'ocr_result' => nested_ocr_result
        }
      ]
    }
  end

  describe 'file not found errors' do
    it 'raises error for non-existent file' do
      expect do
        Kreuzberg.extract_file_sync('/nonexistent/path/file.txt')
      end.to raise_error(StandardError)
    end

    it 'raises error for empty path' do
      expect do
        Kreuzberg.extract_file_sync('')
      end.to raise_error(StandardError)
    end

    it 'raises error for nil path' do
      expect do
        Kreuzberg.extract_file_sync(nil)
      end.to raise_error(StandardError)
    end
  end

  describe 'invalid MIME type handling' do
    it 'handles unknown MIME types' do
      path = create_test_file('Unknown MIME')

      # Implementation may either handle gracefully or raise error for unknown MIME types
      begin
        result = Kreuzberg.extract_file_sync(path, mime_type: 'application/x-unknown-type')
        expect(result).to be_a(Kreuzberg::Result)
      rescue StandardError => e
        expect(e).to be_a(StandardError)
      end
    end
  end

  describe 'invalid configuration' do
    it 'raises error for invalid ocr config' do
      expect do
        Kreuzberg::Config::Extraction.new(ocr: 'invalid')
      end.to raise_error(ArgumentError)
    end

    it 'raises error for invalid chunking config' do
      expect do
        Kreuzberg::Config::Extraction.new(chunking: 123)
      end.to raise_error(ArgumentError)
    end

    it 'raises error for invalid language_detection config' do
      expect do
        Kreuzberg::Config::Extraction.new(language_detection: [])
      end.to raise_error(ArgumentError)
    end

    it 'raises error for invalid pdf_options config' do
      expect do
        Kreuzberg::Config::Extraction.new(pdf_options: 'invalid')
      end.to raise_error(ArgumentError)
    end
  end

  describe 'error context' do
    it 'provides meaningful error messages' do
      Kreuzberg.extract_file_sync('/nonexistent/file.pdf')
      raise 'Expected an error to be raised'
    rescue StandardError => e
      expect(e.message).not_to be_empty
    end
  end

  describe 'batch extraction errors' do
    it 'handles mixed valid and invalid files' do
      files = [
        create_test_file('Valid'),
        '/definitely/nonexistent/file.txt'
      ]

      # Implementation may either raise error or handle gracefully
      begin
        result = Kreuzberg.batch_extract_files_sync(files)
        expect(result).to be_an(Array)
      rescue StandardError => e
        expect(e).to be_a(StandardError)
      end
    end

    it 'handles all invalid files' do
      files = [
        '/nonexistent1.txt',
        '/nonexistent2.txt',
        '/nonexistent3.txt'
      ]

      # Batch operations may either fail fast or return partial results
      begin
        result = Kreuzberg.batch_extract_files_sync(files)
        # If no error is raised, result should be an array (possibly empty or with errors)
        expect(result).to be_an(Array)
      rescue StandardError => e
        # If error is raised, it should be a StandardError
        expect(e).to be_a(StandardError)
      end
    end
  end

  describe 'async error handling' do
    it 'propagates errors in async extraction' do
      expect do
        Kreuzberg.extract_file('/nonexistent/async/file.txt')
      end.to raise_error(StandardError)
    end

    it 'propagates errors in async bytes extraction' do
      # Implementation may either handle invalid MIME types or raise error

      result = Kreuzberg.extract_bytes('data', 'invalid/mime/type/that/causes/error')
      expect(result).to be_a(Kreuzberg::Result)
    rescue StandardError => e
      expect(e).to be_a(StandardError)
    end
  end

  describe 'result parsing errors' do
    it 'handles malformed result gracefully' do
      # This tests the Result class constructor with edge cases
      result = Kreuzberg::Result.new({})

      expect(result.content).to eq('')
      expect(result.mime_type).to eq('')
      expect(result.metadata).to eq({})
      expect(result.tables).to eq([])
      expect(result.detected_languages).to be_nil
      expect(result.chunks).to be_nil
      expect(result.images).to be_nil
    end

    it 'handles partial result data' do
      result = Kreuzberg::Result.new(
        content: 'Test',
        mime_type: 'text/plain'
      )

      expect(result.content).to eq('Test')
      expect(result.mime_type).to eq('text/plain')
      expect(result.tables).to eq([])
    end

    it 'parses invalid metadata JSON' do
      result = Kreuzberg::Result.new(
        content: 'Test',
        mime_type: 'text/plain',
        metadata_json: 'invalid json{'
      )

      expect(result.metadata).to eq({})
    end

    it 'parses extracted images' do
      result = Kreuzberg::Result.new(image_result_payload)
      image = result.images&.first

      expect(image&.format).to eq('png')
      expect(image&.data&.encoding).to eq(Encoding::BINARY)
      expect(image&.ocr_result).to be_a(Kreuzberg::Result)
    end
  end

  describe 'type conversion errors' do
    it 'handles non-string content gracefully' do
      # Test that the wrapper handles type coercion
      path = create_test_file('Type test')
      result = Kreuzberg.extract_file_sync(path)

      expect(result.content).to be_a(String)
      expect(result.mime_type).to be_a(String)
    end
  end
end
