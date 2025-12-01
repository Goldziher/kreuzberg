# frozen_string_literal: true

require 'json'

module Kreuzberg
  # ErrorContext module provides access to FFI error introspection functions.
  # Retrieve the last error code and panic context information from errors.
  module ErrorContext
    class << self
      def last_error_code
        Kreuzberg._last_error_code_native
      rescue StandardError
        0
      end

      def last_panic_context
        json_str = Kreuzberg._last_panic_context_json_native
        return nil unless json_str

        Errors::PanicContext.from_json(json_str)
      rescue StandardError
        nil
      end

      def last_panic_context_json
        Kreuzberg._last_panic_context_json_native
      rescue StandardError
        nil
      end
    end
  end
end
