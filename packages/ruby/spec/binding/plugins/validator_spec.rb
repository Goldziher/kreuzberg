# frozen_string_literal: true

require 'spec_helper'

RSpec.describe 'Validator Plugin System' do
  let(:test_pdf) { test_document_path('text/contract_test.txt') }

  after do
    Kreuzberg.clear_validators
  end

  describe 'registering validator as Proc' do
    it 'registers and executes Proc validator during extraction' do
      validator_called = false
      validator = lambda do |_result|
        validator_called = true
      end

      Kreuzberg.register_validator('check_called', validator)
      Kreuzberg.extract_file_sync(test_pdf)

      expect(validator_called).to be true
    end

    it 'allows extraction to proceed when validator passes' do
      validator = lambda do |result|
        # Validation passes - do nothing
      end

      Kreuzberg.register_validator('pass_validator', validator)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content).not_to be_empty
    end

    it 'prevents extraction when validator raises ValidationError' do
      validator = lambda do |result|
        if result['content'].length < 10_000_000
          raise Kreuzberg::Errors::ValidationError, 'Content too short for this test'
        end
      end

      Kreuzberg.register_validator('min_length', validator)

      expect do
        Kreuzberg.extract_file_sync(test_pdf)
      end.to raise_error(Kreuzberg::Errors::ValidationError, /Content too short/)
    end
  end

  describe 'registering validator as class' do
    it 'registers and executes class-based validator' do
      class MinimumLengthValidator
        include Kreuzberg::ValidatorProtocol

        def initialize(min_length)
          @min_length = min_length
        end

        def call(result)
          return unless result['content'].length < @min_length

          raise Kreuzberg::Errors::ValidationError, "Content too short: #{result['content'].length} < #{@min_length}"
        end
      end

      validator = MinimumLengthValidator.new(10)
      Kreuzberg.register_validator('min_length', validator)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result).to be_a(Kreuzberg::Result)
      expect(result.content.length).to be >= 10
    end

    it 'validates based on content characteristics' do
      class NonEmptyValidator
        include Kreuzberg::ValidatorProtocol

        def call(result)
          return unless result['content'].strip.empty?

          raise Kreuzberg::Errors::ValidationError, 'Content cannot be empty'
        end
      end

      validator = NonEmptyValidator.new
      Kreuzberg.register_validator('non_empty', validator)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(result.content.strip).not_to be_empty
    end
  end

  describe 'validator receives correct parameters' do
    it 'receives result hash with all required fields' do
      received_result = nil
      validator = lambda do |result|
        received_result = result
      end

      Kreuzberg.register_validator('capture', validator)
      Kreuzberg.extract_file_sync(test_pdf)

      expect(received_result).to be_a(Hash)
      expect(received_result).to have_key('content')
      expect(received_result).to have_key('mime_type')
      expect(received_result).to have_key('metadata')
      expect(received_result).to have_key('tables')
    end

    it 'receives correct content in result hash' do
      received_content = nil
      validator = lambda do |result|
        received_content = result['content']
      end

      Kreuzberg.register_validator('capture_content', validator)
      result = Kreuzberg.extract_file_sync(test_pdf)

      expect(received_content).to eq(result.content)
    end
  end

  describe 'multiple validators' do
    it 'executes all registered validators' do
      validator1_called = false
      validator2_called = false

      validator1 = lambda do |_result|
        validator1_called = true
      end

      validator2 = lambda do |_result|
        validator2_called = true
      end

      Kreuzberg.register_validator('val1', validator1)
      Kreuzberg.register_validator('val2', validator2)
      Kreuzberg.extract_file_sync(test_pdf)

      expect(validator1_called).to be true
      expect(validator2_called).to be true
    end

    it 'stops execution if any validator fails' do
      validator1 = lambda do |_result|
        raise Kreuzberg::Errors::ValidationError, 'First validator failed'
      end

      validator2 = lambda do |_result|
        raise StandardError, 'This should not be reached'
      end

      Kreuzberg.register_validator('fail_first', validator1)
      Kreuzberg.register_validator('never_reached', validator2)

      expect do
        Kreuzberg.extract_file_sync(test_pdf)
      end.to raise_error(Kreuzberg::Errors::ValidationError, /First validator failed/)
    end
  end

  describe 'unregister_validator' do
    it 'removes a registered validator by name' do
      validator = lambda do |_result|
        raise Kreuzberg::Errors::ValidationError, 'Should not be called'
      end

      Kreuzberg.register_validator('removable', validator)
      Kreuzberg.unregister_validator('removable')

      expect do
        Kreuzberg.extract_file_sync(test_pdf)
      end.not_to raise_error
    end

    it 'does not affect other registered validators' do
      validator1_called = false
      validator3_called = false

      validator1 = lambda do |_result|
        validator1_called = true
      end

      validator2 = lambda do |_result|
        raise Kreuzberg::Errors::ValidationError, 'Should not be called'
      end

      validator3 = lambda do |_result|
        validator3_called = true
      end

      Kreuzberg.register_validator('keep1', validator1)
      Kreuzberg.register_validator('remove', validator2)
      Kreuzberg.register_validator('keep3', validator3)

      Kreuzberg.unregister_validator('remove')
      Kreuzberg.extract_file_sync(test_pdf)

      expect(validator1_called).to be true
      expect(validator3_called).to be true
    end
  end

  describe 'clear_validators' do
    it 'removes all registered validators' do
      validator1 = lambda do |_result|
        raise Kreuzberg::Errors::ValidationError, 'Should not be called 1'
      end

      validator2 = lambda do |_result|
        raise Kreuzberg::Errors::ValidationError, 'Should not be called 2'
      end

      Kreuzberg.register_validator('val1', validator1)
      Kreuzberg.register_validator('val2', validator2)

      Kreuzberg.clear_validators

      expect do
        Kreuzberg.extract_file_sync(test_pdf)
      end.not_to raise_error
    end
  end

  describe 'list_validators' do
    it 'returns empty array when no validators registered' do
      Kreuzberg.clear_validators
      validators = Kreuzberg.list_validators
      expect(validators).to be_an(Array)
      expect(validators).to be_empty
    end

    it 'returns validator names after registration' do
      Kreuzberg.clear_validators
      validator = ->(result) { }
      Kreuzberg.register_validator('test-validator', validator)
      validators = Kreuzberg.list_validators
      expect(validators).to include('test-validator')
      Kreuzberg.clear_validators
    end

    it 'returns all registered validator names' do
      Kreuzberg.clear_validators
      validator1 = ->(result) { }
      validator2 = ->(result) { }
      validator3 = ->(result) { }

      Kreuzberg.register_validator('validator-one', validator1)
      Kreuzberg.register_validator('validator-two', validator2)
      Kreuzberg.register_validator('validator-three', validator3)

      validators = Kreuzberg.list_validators
      expect(validators).to contain_exactly('validator-one', 'validator-two', 'validator-three')
      Kreuzberg.clear_validators
    end

    it 'reflects changes after unregistration' do
      Kreuzberg.clear_validators
      validator = ->(result) { }
      Kreuzberg.register_validator('temp-validator', validator)

      validators_before = Kreuzberg.list_validators
      expect(validators_before).to include('temp-validator')

      Kreuzberg.unregister_validator('temp-validator')

      validators_after = Kreuzberg.list_validators
      expect(validators_after).not_to include('temp-validator')
      Kreuzberg.clear_validators
    end
  end
end
