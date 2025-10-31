# frozen_string_literal: true

module Kreuzberg
  # Command-line interface wrapper
  #
  # Provides a Ruby API for the Kreuzberg CLI commands.
  #
  # @example Extract a file
  #   Kreuzberg::CLI.extract('document.pdf', output: 'text')
  #
  # @example Detect file type
  #   mime_type = Kreuzberg::CLI.detect('document.pdf')
  #
  module CLI
    module_function

    # Extract content from a file using the CLI
    #
    # @param path [String] Path to the file
    # @param output [String] Output format ("text", "json", "markdown")
    # @param ocr [Boolean] Enable OCR
    # @return [String] Extracted content
    #
    def extract(path, output: 'text', ocr: false)
      args = ['extract', path, '--output', output]
      args << '--ocr' if ocr
      CLIProxy.call(args)
    end

    # Detect MIME type of a file using the CLI
    #
    # @param path [String] Path to the file
    # @return [String] MIME type
    #
    def detect(path)
      CLIProxy.call(['detect', path]).strip
    end

    # Get CLI version
    #
    # @return [String] Version string
    #
    def version
      CLIProxy.call(['--version']).strip
    end

    # Get CLI help text
    #
    # @return [String] Help text
    #
    def help
      CLIProxy.call(['--help'])
    end
  end
end
