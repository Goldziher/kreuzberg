# frozen_string_literal: true

require 'open3'
require 'pathname'

module Kreuzberg
  # API server proxy
  #
  # Starts and manages the Kreuzberg API server (Litestar/Python-based or Rust-based).
  #
  # @example Start the server
  #   server = Kreuzberg::APIProxy.new(port: 8000)
  #   server.start
  #   # Server runs in background
  #   server.stop
  #
  # @example With block
  #   Kreuzberg::APIProxy.run(port: 8000) do |server|
  #     # Server runs while block executes
  #     response = Net::HTTP.get(URI('http://localhost:8000/health'))
  #   end
  #
  module APIProxy
    Error = Class.new(Kreuzberg::Errors::Error)
    MissingBinaryError = Class.new(Error)
    ServerError = Class.new(Error)

    # API server instance
    class Server
      attr_reader :port, :host, :pid

      # Initialize server
      #
      # @param port [Integer] Port to run on (default: 8000)
      # @param host [String] Host to bind to (default: "0.0.0.0")
      #
      def initialize(port: 8000, host: '0.0.0.0')
        @port = port
        @host = host
        @pid = nil
        @process = nil
      end

      # Start the server in the background
      #
      # @return [Integer] Process ID
      # @raise [ServerError] If server fails to start
      #
      def start
        binary = APIProxy.find_api_binary
        @pid = spawn(
          binary.to_s,
          'api',
          '--host', @host,
          '--port', @port.to_s,
          out: $stdout,
          err: $stderr
        )
        Process.detach(@pid)
        sleep 1 # Give server time to start
        @pid
      end

      # Stop the server
      #
      # @return [void]
      #
      def stop
        return unless @pid

        Process.kill('TERM', @pid)
        Process.wait(@pid)
      rescue Errno::ESRCH, Errno::ECHILD
        # Process already dead
      ensure
        @pid = nil
      end

      # Check if server is running
      #
      # @return [Boolean]
      #
      def running?
        return false unless @pid

        Process.kill(0, @pid)
        true
      rescue Errno::ESRCH, Errno::EPERM
        false
      end
    end

    module_function

    # Run server with a block
    #
    # @param port [Integer] Port to run on
    # @param host [String] Host to bind to
    # @yield [Server] Yields server instance
    # @return [Object] Block result
    #
    # @example
    #   Kreuzberg::APIProxy.run(port: 8000) do |server|
    #     # Make API requests
    #   end
    #
    def run(port: 8000, host: '0.0.0.0')
      server = Server.new(port: port, host: host)
      server.start
      yield server
    ensure
      server&.stop
    end

    # Find the API binary
    #
    # @return [Pathname] Path to binary
    # @raise [MissingBinaryError] If not found
    #
    def find_api_binary
      # API might be served by kreuzberg CLI or a separate binary
      binary_name = Gem.win_platform? ? 'kreuzberg.exe' : 'kreuzberg'
      found = CLIProxy.search_paths(binary_name).find(&:file?)
      return found if found

      raise MissingBinaryError, missing_binary_message
    end

    # Error message for missing binary
    #
    # @return [String]
    #
    def missing_binary_message
      <<~MSG.strip
        kreuzberg binary not found for API server. Build it with:
        `cargo build --release --package kreuzberg-cli`

        Or ensure kreuzberg is installed with API support.
      MSG
    end
  end
end
