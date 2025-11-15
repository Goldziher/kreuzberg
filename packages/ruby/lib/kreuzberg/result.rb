# frozen_string_literal: true

begin
  require 'json'
rescue LoadError
  require 'json/pure'
end

module Kreuzberg
  # Extraction result wrapper
  #
  # Provides structured access to extraction results from the native extension.
  #
  # @example
  #   result = Kreuzberg.extract_file_sync("document.pdf")
  #   puts result.content
  #   puts "MIME type: #{result.mime_type}"
  #   puts "Metadata: #{result.metadata.inspect}"
  #   result.tables.each { |table| puts table.inspect }
  #
  class Result
    attr_reader :content, :mime_type, :metadata, :metadata_json, :tables, :detected_languages, :chunks

    # Table structure
    #
    # @!attribute [r] cells
    #   @return [Array<Array<String>>] Table cells (2D array)
    # @!attribute [r] markdown
    #   @return [String] Markdown representation
    # @!attribute [r] page_number
    #   @return [Integer] Page number where table was found
    #
    Table = Struct.new(:cells, :markdown, :page_number, keyword_init: true) do
      def to_h
        { cells: cells, markdown: markdown, page_number: page_number }
      end
    end

    # Text chunk
    #
    # @!attribute [r] content
    #   @return [String] Chunk content
    # @!attribute [r] char_start
    #   @return [Integer] Starting character index
    # @!attribute [r] char_end
    #   @return [Integer] Ending character index
    # @!attribute [r] token_count
    #   @return [Integer, nil] Approximate token count (may be nil)
    #
    Chunk = Struct.new(:content, :char_start, :char_end, :token_count, keyword_init: true) do
      def to_h
        { content: content, char_start: char_start, char_end: char_end, token_count: token_count }
      end
    end

    # Initialize from native hash result
    #
    # @param hash [Hash] Hash returned from native extension
    #
    def initialize(hash)
      @content = hash['content'] || ''
      @mime_type = hash['mime_type'] || ''
      @metadata_json = hash['metadata_json'] || '{}'
      @metadata = parse_metadata(@metadata_json)
      @tables = parse_tables(hash['tables'])
      @detected_languages = parse_detected_languages(hash['detected_languages'])
      @chunks = parse_chunks(hash['chunks'])
    end

    # Convert to hash
    #
    # @return [Hash] Hash representation
    #
    def to_h
      {
        content: @content,
        mime_type: @mime_type,
        metadata: @metadata,
        tables: @tables.map(&:to_h),
        detected_languages: @detected_languages,
        chunks: @chunks&.map(&:to_h)
      }
    end

    # Convert to JSON
    #
    # @return [String] JSON representation
    #
    def to_json(*)
      to_h.to_json(*)
    end

    private

    def parse_metadata(metadata_json)
      JSON.parse(metadata_json)
    rescue JSON::ParserError
      {}
    end

    def parse_tables(tables_data)
      return [] if tables_data.nil? || tables_data.empty?

      tables_data.map do |table_hash|
        Table.new(
          cells: table_hash['cells'] || [],
          markdown: table_hash['markdown'] || '',
          page_number: table_hash['page_number'] || 0
        )
      end
    end

    def parse_detected_languages(langs_data)
      return nil if langs_data.nil?

      # Detected languages is now just an array of strings
      langs_data.is_a?(Array) ? langs_data : []
    end

    def parse_chunks(chunks_data)
      return nil if chunks_data.nil?

      chunks_data.map do |chunk_hash|
        Chunk.new(
          content: chunk_hash['content'],
          char_start: chunk_hash['char_start'],
          char_end: chunk_hash['char_end'],
          token_count: chunk_hash['token_count']
        )
      end
    end
  end
end
