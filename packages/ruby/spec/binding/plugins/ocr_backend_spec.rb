# frozen_string_literal: true

require 'spec_helper'

RSpec.describe 'OCR Backend Plugin System' do
  let(:test_image) { test_document_path('images/invoice_image.png') }

  describe 'registering custom OCR backend' do
    it 'registers and uses custom OCR backend class' do
      class MockOcrBackend
        include Kreuzberg::OcrBackendProtocol

        attr_reader :process_called, :received_config

        def initialize
          @process_called = false
          @received_config = nil
        end

        def name
          'mock-ocr'
        end

        def process_image(_image_bytes, config)
          @process_called = true
          @received_config = config
          'Mocked OCR text from custom backend'
        end
      end

      backend = MockOcrBackend.new
      Kreuzberg.register_ocr_backend('mock-ocr', backend)

      config = Kreuzberg::Config::Extraction.new(
        force_ocr: true,
        ocr: Kreuzberg::Config::Ocr.new(backend: 'mock-ocr')
      )

      result = Kreuzberg.extract_file_sync(test_image, config: config)

      expect(backend.process_called).to be true
      expect(result.content).to include('Mocked OCR text')
    end

    it 'passes correct configuration to OCR backend' do
      class ConfigCapturingBackend
        include Kreuzberg::OcrBackendProtocol

        attr_reader :received_config

        def name
          'config-capture'
        end

        def process_image(_image_bytes, config)
          @received_config = config
          'OCR result'
        end
      end

      backend = ConfigCapturingBackend.new
      Kreuzberg.register_ocr_backend('config-capture', backend)

      config = Kreuzberg::Config::Extraction.new(
        force_ocr: true,
        ocr: Kreuzberg::Config::Ocr.new(
          backend: 'config-capture',
          language: 'eng'
        )
      )

      Kreuzberg.extract_file_sync(test_image, config: config)

      expect(backend.received_config).to be_a(Hash)
      expect(backend.received_config['backend']).to eq('config-capture')
      expect(backend.received_config['language']).to eq('eng')
    end
  end

  describe 'OCR backend receives correct parameters' do
    it 'receives image bytes as binary data' do
      class BytesCapturingBackend
        include Kreuzberg::OcrBackendProtocol

        attr_accessor :received_bytes

        def name
          'bytes-capture'
        end

        def process_image(image_bytes, _config)
          self.class.instance_variable_set(:@received_bytes, image_bytes)
          'OCR result'
        end
      end

      backend = BytesCapturingBackend.new
      Kreuzberg.register_ocr_backend('bytes-capture', backend)

      config = Kreuzberg::Config::Extraction.new(
        force_ocr: true,
        ocr: Kreuzberg::Config::Ocr.new(backend: 'bytes-capture')
      )

      Kreuzberg.extract_file_sync(test_image, config: config)

      received_bytes = BytesCapturingBackend.instance_variable_get(:@received_bytes)
      expect(received_bytes).to be_a(String)
      expect(received_bytes.encoding).to eq(Encoding::BINARY)
      expect(received_bytes.length).to be_positive
    end

    it 'backend can return extracted text' do
      class SimpleOcrBackend
        include Kreuzberg::OcrBackendProtocol

        def name
          'simple-ocr'
        end

        def process_image(_image_bytes, _config)
          'Invoice Total: $1,234.56'
        end
      end

      backend = SimpleOcrBackend.new
      Kreuzberg.register_ocr_backend('simple-ocr', backend)

      config = Kreuzberg::Config::Extraction.new(
        force_ocr: true,
        ocr: Kreuzberg::Config::Ocr.new(backend: 'simple-ocr')
      )

      result = Kreuzberg.extract_file_sync(test_image, config: config)

      expect(result.content).to include('Invoice Total')
      expect(result.content).to include('1,234.56')
    end
  end

  describe 'OCR backend with stateful processing' do
    it 'maintains state across multiple invocations' do
      class StatefulOcrBackend
        include Kreuzberg::OcrBackendProtocol

        attr_reader :call_count

        def initialize
          @call_count = 0
        end

        def name
          'stateful-ocr'
        end

        def process_image(_image_bytes, _config)
          @call_count += 1
          "OCR call number #{@call_count}"
        end
      end

      backend = StatefulOcrBackend.new
      Kreuzberg.register_ocr_backend('stateful-ocr', backend)

      config = Kreuzberg::Config::Extraction.new(
        force_ocr: true,
        ocr: Kreuzberg::Config::Ocr.new(backend: 'stateful-ocr')
      )

      Kreuzberg.extract_file_sync(test_image, config: config)
      Kreuzberg.extract_file_sync(test_image, config: config)

      expect(backend.call_count).to be >= 1
    end
  end

  describe 'error handling' do
    it 'propagates errors from OCR backend' do
      class FailingOcrBackend
        include Kreuzberg::OcrBackendProtocol

        def name
          'failing-ocr'
        end

        def process_image(_image_bytes, _config)
          raise StandardError, 'OCR processing failed'
        end
      end

      backend = FailingOcrBackend.new
      Kreuzberg.register_ocr_backend('failing-ocr', backend)

      config = Kreuzberg::Config::Extraction.new(
        force_ocr: true,
        ocr: Kreuzberg::Config::Ocr.new(backend: 'failing-ocr')
      )

      expect do
        Kreuzberg.extract_file_sync(test_image, config: config)
      end.to raise_error(StandardError, /OCR processing failed/)
    end

    it 'handles missing OCR backend gracefully' do
      config = Kreuzberg::Config::Extraction.new(
        force_ocr: true,
        ocr: Kreuzberg::Config::Ocr.new(backend: 'nonexistent-backend')
      )

      expect do
        Kreuzberg.extract_file_sync(test_image, config: config)
      end.to raise_error
    end
  end

  describe 'OCR backend protocol implementation' do
    it 'requires name method' do
      class InvalidBackendNoName
        def process_image(_image_bytes, _config)
          'text'
        end
      end

      backend = InvalidBackendNoName.new

      expect do
        Kreuzberg.register_ocr_backend('invalid', backend)
      end.to raise_error
    end

    it 'requires process_image method' do
      class InvalidBackendNoProcess
        def name
          'invalid'
        end
      end

      backend = InvalidBackendNoProcess.new

      expect do
        Kreuzberg.register_ocr_backend('invalid', backend)
      end.to raise_error
    end
  end

  describe 'OCR backend management' do
    describe '.list_ocr_backends' do
      it 'returns an array of backend names' do
        backends = Kreuzberg.list_ocr_backends
        expect(backends).to be_an(Array)
      end

      it 'includes registered backends' do
        class ListTestBackend
          include Kreuzberg::OcrBackendProtocol

          def name
            'list-test-backend'
          end

          def process_image(_image_bytes, _config)
            'test'
          end
        end

        backend = ListTestBackend.new
        Kreuzberg.register_ocr_backend('list-test-backend', backend)

        backends = Kreuzberg.list_ocr_backends
        expect(backends).to include('list-test-backend')

        Kreuzberg.unregister_ocr_backend('list-test-backend')
      end
    end

    describe '.unregister_ocr_backend' do
      it 'removes backend from registry' do
        class UnregisterTestBackend
          include Kreuzberg::OcrBackendProtocol

          def name
            'unregister-test'
          end

          def process_image(_image_bytes, _config)
            'test'
          end
        end

        backend = UnregisterTestBackend.new
        Kreuzberg.register_ocr_backend('unregister-test', backend)

        backends = Kreuzberg.list_ocr_backends
        expect(backends).to include('unregister-test')

        Kreuzberg.unregister_ocr_backend('unregister-test')

        backends = Kreuzberg.list_ocr_backends
        expect(backends).not_to include('unregister-test')
      end

      it 'accepts nonexistent backend name without error' do
        expect { Kreuzberg.unregister_ocr_backend('nonexistent-backend-xyz') }.not_to raise_error
      end
    end
  end
end
