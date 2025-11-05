# frozen_string_literal: true

module Kreuzberg
  module Errors
    # Base error class for all Kreuzberg errors
    class Error < StandardError; end

    # Raised when validation fails
    class ValidationError < Error; end

    # Raised when document parsing fails
    class ParsingError < Error
      attr_reader :context

      def initialize(message, context: nil)
        super(message)
        @context = context
      end
    end

    # Raised when OCR processing fails
    class OCRError < Error
      attr_reader :context

      def initialize(message, context: nil)
        super(message)
        @context = context
      end
    end

    # Raised when a required dependency is missing
    class MissingDependencyError < Error
      attr_reader :dependency

      def initialize(message, dependency: nil)
        super(message)
        @dependency = dependency
      end
    end

    # Raised when an I/O operation fails
    class IOError < Error; end

    # Raised when plugin operations fail
    class PluginError < Error; end
  end
end
