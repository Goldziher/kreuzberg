# frozen_string_literal: true

require 'open3'
require 'pathname'

module Kreuzberg
  # CLI binary proxy
  #
  # Provides access to the Kreuzberg CLI binary built from crates/kreuzberg-cli.
  #
  # @example
  #   output = Kreuzberg::CLIProxy.call(['extract', 'document.pdf'])
  #   puts output
  #
  module CLIProxy
    Error = Class.new(Kreuzberg::Errors::Error)
    MissingBinaryError = Class.new(Error)

    # CLI execution error with stderr and exit status
    class CLIExecutionError < Error
      attr_reader :stderr, :status

      def initialize(message, stderr:, status:)
        super(message)
        @stderr = stderr
        @status = status
      end
    end

    module_function

    # Execute the Kreuzberg CLI with given arguments
    #
    # @param argv [Array<String>] Command-line arguments
    # @return [String] Standard output from the CLI
    # @raise [CLIExecutionError] If the CLI exits with non-zero status
    # @raise [MissingBinaryError] If the CLI binary is not found
    #
    # @example Extract a file
    #   output = Kreuzberg::CLIProxy.call(['extract', 'document.pdf'])
    #
    # @example Detect file type
    #   output = Kreuzberg::CLIProxy.call(['detect', 'document.pdf'])
    #
    def call(argv)
      binary = find_cli_binary
      args = Array(argv).map(&:to_s)
      stdout, stderr, status = Open3.capture3(binary.to_s, *args)
      return stdout if status.success?

      raise CLIExecutionError.new(
        "kreuzberg CLI exited with status #{status.exitstatus}",
        stderr: stderr,
        status: status.exitstatus
      )
    end

    # Find the kreuzberg CLI binary
    #
    # Searches in multiple locations:
    # - crates/kreuzberg-cli/target/release/
    # - packages/ruby/lib/bin/
    # - workspace root target/release/
    #
    # @return [Pathname] Path to the CLI binary
    # @raise [MissingBinaryError] If binary not found
    #
    def find_cli_binary
      binary_name = Gem.win_platform? ? 'kreuzberg.exe' : 'kreuzberg'
      found = search_paths(binary_name).find(&:file?)
      return found if found

      raise MissingBinaryError, missing_binary_message
    end

    # Get the root path of the Ruby package
    #
    # @return [Pathname] Root path
    #
    def root_path
      @root_path ||= Pathname(__dir__ || '.').join('../..').expand_path
    end

    # Get the lib path
    #
    # @return [Pathname] Lib path
    #
    def lib_path
      @lib_path ||= Pathname(__dir__ || '.').join('..').expand_path
    end

    # Search paths for the CLI binary
    #
    # @param binary_name [String] Name of the binary
    # @return [Array<Pathname>] List of paths to search
    #
    def search_paths(binary_name)
      paths = [
        # In lib/bin (for packaged gems)
        lib_path.join('bin', binary_name),
        lib_path.join(binary_name),
        # In local development (packages/ruby)
        root_path.join('../../crates/kreuzberg-cli/target/release', binary_name),
        root_path.join('../../target/release', binary_name)
      ]

      # Try workspace root
      workspace_root = root_path.parent&.parent
      paths << workspace_root.join('target', 'release', binary_name) if workspace_root

      paths
    end

    # Error message when binary is missing
    #
    # @return [String] Error message
    #
    def missing_binary_message
      <<~MSG.strip
        kreuzberg CLI binary not found. Build it with:
        `cargo build --release --package kreuzberg-cli`

        Or install the gem with pre-built binaries.
      MSG
    end
  end
end
