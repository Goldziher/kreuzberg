# frozen_string_literal: true

require 'json'

module Kreuzberg
  # Error code constants matching kreuzberg-ffi error codes
  ERROR_CODE_SUCCESS = 0
  ERROR_CODE_GENERIC = 1
  ERROR_CODE_PANIC = 2
  ERROR_CODE_INVALID_ARGUMENT = 3
  ERROR_CODE_IO = 4
  ERROR_CODE_PARSING = 5
  ERROR_CODE_OCR = 6
  ERROR_CODE_MISSING_DEPENDENCY = 7

  module Errors
    # Panic context information from FFI error introspection
    class PanicContext
      attr_reader :file, :line, :function, :message, :timestamp_secs

      def initialize(file:, line:, function:, message:, timestamp_secs:)
        @file = file
        @line = line
        @function = function
        @message = message
        @timestamp_secs = timestamp_secs
      end

      def to_s
        "#{file}:#{line}:#{function}: #{message}"
      end

      def to_h
        {
          file:,
          line:,
          function:,
          message:,
          timestamp_secs:
        }
      end

      def self.from_json(json_string)
        return nil if json_string.nil? || json_string.empty?

        data = JSON.parse(json_string, symbolize_names: true)
        sliced = data.slice(:file, :line, :function, :message, :timestamp_secs)
        new(**with_defaults(sliced))
      rescue JSON::ParserError
        nil
      end

      def self.with_defaults(sliced)
        {
          file: sliced[:file] || '',
          line: sliced[:line] || 0,
          function: sliced[:function] || '',
          message: sliced[:message] || '',
          timestamp_secs: sliced[:timestamp_secs] || 0
        }
      end
      private_class_method :with_defaults
    end

    # Base error class for all Kreuzberg errors
    class Error < StandardError
      attr_reader :panic_context, :error_code

      def initialize(message, panic_context: nil, error_code: nil)
        super(message)
        @panic_context = panic_context
        @error_code = error_code
      end
    end

    # Raised when validation fails
    class ValidationError < Error; end

    # Raised when document parsing fails
    class ParsingError < Error
      attr_reader :context

      def initialize(message, context: nil, panic_context: nil, error_code: nil)
        super(message, panic_context:, error_code:)
        @context = context
      end
    end

    # Raised when OCR processing fails
    class OCRError < Error
      attr_reader :context

      def initialize(message, context: nil, panic_context: nil, error_code: nil)
        super(message, panic_context:, error_code:)
        @context = context
      end
    end

    # Raised when a required dependency is missing
    class MissingDependencyError < Error
      attr_reader :dependency

      def initialize(message, dependency: nil, panic_context: nil, error_code: nil)
        super(message, panic_context:, error_code:)
        @dependency = dependency
      end
    end

    # Raised when an I/O operation fails
    class IOError < Error; end

    # Raised when plugin operations fail
    class PluginError < Error; end

    # Raised when an unsupported file format or MIME type is encountered
    class UnsupportedFormatError < Error; end
  end
end
