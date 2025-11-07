# frozen_string_literal: true

require 'spec_helper'

RSpec.describe 'PostProcessor Plugin System' do
  let(:test_pdf) { '/Users/naamanhirschfeld/workspace/kreuzberg/test_documents/text/contract_test.txt' }

  after do
    Kreuzberg.clear_post_processors
  end

  describe 'registering post-processor as Proc' do
    it 'registers and executes Proc post-processor during extraction' do
      processor_called = false
      processor = lambda do |result|
        processor_called = true
        result['content'] = result['content'].upcase
        result
      end

      Kreuzberg.register_post_processor('upcase', processor)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(processor_called).to be true
      expect(result.content).to eq(result.content.upcase)
    end

    it 'allows post-processor to modify result content' do
      processor = lambda do |result|
        result['content'] = "[PROCESSED] #{result['content']}"
        result
      end

      Kreuzberg.register_post_processor('prefix', processor)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.content).to start_with('[PROCESSED]')
    end

    it 'allows post-processor to add metadata' do
      processor = lambda do |result|
        result['metadata']['custom_field'] = 'custom_value'
        result['metadata']['word_count'] = result['content'].split.length
        result
      end

      Kreuzberg.register_post_processor('metadata_adder', processor)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.metadata['custom_field']).to eq('custom_value')
      expect(result.metadata['word_count']).to be > 0
    end
  end

  describe 'registering post-processor as class' do
    it 'registers and executes class-based post-processor' do
      class WordCountProcessor
        include Kreuzberg::PostProcessorProtocol

        def call(result)
          word_count = result['content'].split.length
          result['metadata']['word_count'] = word_count
          result['metadata']['processor_name'] = 'WordCountProcessor'
          result
        end
      end

      processor = WordCountProcessor.new
      Kreuzberg.register_post_processor('word_count', processor)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.metadata['word_count']).to be > 0
      expect(result.metadata['processor_name']).to eq('WordCountProcessor')
    end

    it 'allows class-based processor to transform content' do
      class TruncateProcessor
        include Kreuzberg::PostProcessorProtocol

        def initialize(max_length)
          @max_length = max_length
        end

        def call(result)
          result['content'] = "#{result['content'][0...@max_length]}..." if result['content'].length > @max_length
          result
        end
      end

      processor = TruncateProcessor.new(50)
      Kreuzberg.register_post_processor('truncate', processor)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.content.length).to be <= 53
    end
  end

  describe 'multiple post-processors' do
    it 'executes multiple registered post-processors in order' do
      processor1 = lambda do |result|
        result['metadata']['processor1'] = 'executed'
        result
      end

      processor2 = lambda do |result|
        result['metadata']['processor2'] = 'executed'
        result
      end

      Kreuzberg.register_post_processor('proc1', processor1)
      Kreuzberg.register_post_processor('proc2', processor2)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.metadata['processor1']).to eq('executed')
      expect(result.metadata['processor2']).to eq('executed')
    end
  end

  describe 'unregister_post_processor' do
    it 'removes a registered post-processor by name' do
      processor = lambda do |result|
        result['metadata']['should_not_appear'] = 'value'
        result
      end

      Kreuzberg.register_post_processor('removable', processor)
      Kreuzberg.unregister_post_processor('removable')
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.metadata['should_not_appear']).to be_nil
    end

    it 'does not affect other registered post-processors' do
      processor1 = lambda do |result|
        result['metadata']['keep1'] = 'value1'
        result
      end

      processor2 = lambda do |result|
        result['metadata']['remove'] = 'value2'
        result
      end

      processor3 = lambda do |result|
        result['metadata']['keep3'] = 'value3'
        result
      end

      Kreuzberg.register_post_processor('keep1', processor1)
      Kreuzberg.register_post_processor('remove', processor2)
      Kreuzberg.register_post_processor('keep3', processor3)

      Kreuzberg.unregister_post_processor('remove')
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.metadata['keep1']).to eq('value1')
      expect(result.metadata['remove']).to be_nil
      expect(result.metadata['keep3']).to eq('value3')
    end
  end

  describe 'clear_post_processors' do
    it 'removes all registered post-processors' do
      processor1 = lambda do |result|
        result['metadata']['proc1'] = 'value1'
        result
      end

      processor2 = lambda do |result|
        result['metadata']['proc2'] = 'value2'
        result
      end

      Kreuzberg.register_post_processor('proc1', processor1)
      Kreuzberg.register_post_processor('proc2', processor2)

      Kreuzberg.clear_post_processors
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.metadata['proc1']).to be_nil
      expect(result.metadata['proc2']).to be_nil
    end
  end

  describe 'error handling' do
    it 'propagates errors from post-processor' do
      processor = lambda do |_result|
        raise StandardError, 'Post-processor error'
      end

      Kreuzberg.register_post_processor('failing', processor)

      expect do
        Kreuzberg.extract_file_sync(test_pdf)
      end.to raise_error(StandardError, /Post-processor error/)
    end

    it 'handles post-processor that returns invalid result' do
      processor = lambda do |_result|
        'invalid return value'
      end

      Kreuzberg.register_post_processor('invalid', processor)

      expect do
        Kreuzberg.extract_file_sync(test_pdf)
      end.to raise_error
    end
  end
end
