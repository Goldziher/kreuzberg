# frozen_string_literal: true

require 'open3'
require 'pathname'
require 'json'

module Kreuzberg
  # MCP (Model Context Protocol) server proxy
  #
  # Starts and manages the Kreuzberg MCP server for Claude Desktop integration.
  #
  # @example Start MCP server
  #   server = Kreuzberg::MCPProxy.new
  #   server.start
  #
  module MCPProxy
    Error = Class.new(Kreuzberg::Errors::Error)
    MissingBinaryError = Class.new(Error)
    ServerError = Class.new(Error)

    # MCP server instance
    class Server
      attr_reader :pid, :transport

      # Initialize MCP server
      #
      # @param transport [String] Transport method ("stdio" or "sse")
      #
      def initialize(transport: 'stdio')
        @transport = transport
        @pid = nil
        @stdin = nil
        @stdout = nil
        @stderr = nil
      end

      # Start the MCP server
      #
      # @return [Integer, nil] Process ID (for SSE) or nil (for stdio)
      #
      def start
        binary = MCPProxy.find_mcp_binary

        case @transport
        when 'stdio'
          start_stdio(binary)
        when 'sse'
          start_sse(binary)
        else
          raise ServerError, "Unknown transport: #{@transport}"
        end
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
        close_pipes
      end

      # Send a message to the server (stdio only)
      #
      # @param message [Hash] JSON-RPC message
      # @return [void]
      #
      def send_message(message)
        raise ServerError, 'Can only send messages in stdio mode' unless @transport == 'stdio'
        raise ServerError, 'Server not started' unless @stdin

        @stdin.puts(JSON.generate(message))
        @stdin.flush
      end

      # Read a message from the server (stdio only)
      #
      # @return [Hash] JSON-RPC message
      #
      def read_message
        raise ServerError, 'Can only read messages in stdio mode' unless @transport == 'stdio'
        raise ServerError, 'Server not started' unless @stdout

        line = @stdout.gets
        JSON.parse(line) if line
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

      private

      def start_stdio(binary)
        @stdin, @stdout, @stderr, wait_thr = Open3.popen3(binary.to_s, 'mcp', '--transport', 'stdio')
        @pid = wait_thr.pid
        nil
      end

      def start_sse(binary)
        @pid = spawn(
          binary.to_s,
          'mcp',
          '--transport', 'sse',
          out: $stdout,
          err: $stderr
        )
        Process.detach(@pid)
        sleep 1 # Give server time to start
        @pid
      end

      def close_pipes
        @stdin&.close
        @stdout&.close
        @stderr&.close
        @stdin = @stdout = @stderr = nil
      end
    end

    module_function

    # Run MCP server with a block
    #
    # @param transport [String] Transport method
    # @yield [Server] Yields server instance
    # @return [Object] Block result
    #
    # @example
    #   Kreuzberg::MCPProxy.run(transport: 'stdio') do |server|
    #     server.send_message({ method: 'tools/list' })
    #     response = server.read_message
    #   end
    #
    def run(transport: 'stdio')
      server = Server.new(transport: transport)
      server.start
      yield server
    ensure
      server&.stop
    end

    # Find the MCP binary
    #
    # @return [Pathname] Path to binary
    # @raise [MissingBinaryError] If not found
    #
    def find_mcp_binary
      # MCP is served by kreuzberg CLI
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
        kreuzberg binary not found for MCP server. Build it with:
        `cargo build --release --package kreuzberg-cli`

        Or ensure kreuzberg is installed with MCP support.
      MSG
    end
  end
end
