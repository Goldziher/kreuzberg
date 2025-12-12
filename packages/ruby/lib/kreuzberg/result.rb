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
  # rubocop:disable Metrics/ClassLength
  class Result
    attr_reader :content, :mime_type, :metadata, :metadata_json, :tables,
                :detected_languages, :chunks, :images, :pages

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
    # @!attribute [r] byte_start
    #   @return [Integer] Starting byte offset (UTF-8)
    # @!attribute [r] byte_end
    #   @return [Integer] Ending byte offset (UTF-8)
    # @!attribute [r] token_count
    #   @return [Integer, nil] Approximate token count (may be nil)
    # @!attribute [r] first_page
    #   @return [Integer, nil] First page number (1-indexed)
    # @!attribute [r] last_page
    #   @return [Integer, nil] Last page number (1-indexed)
    #
    Chunk = Struct.new(
      :content,
      :byte_start,
      :byte_end,
      :token_count,
      :chunk_index,
      :total_chunks,
      :first_page,
      :last_page,
      :embedding,
      keyword_init: true
    ) do
      def to_h
        {
          content: content,
          byte_start: byte_start,
          byte_end: byte_end,
          token_count: token_count,
          chunk_index: chunk_index,
          total_chunks: total_chunks,
          first_page: first_page,
          last_page: last_page,
          embedding: embedding
        }
      end
    end

    Image = Struct.new(
      :data,
      :format,
      :image_index,
      :page_number,
      :width,
      :height,
      :colorspace,
      :bits_per_component,
      :is_mask,
      :description,
      :ocr_result,
      keyword_init: true
    ) do
      def to_h
        {
          data: data,
          format: format,
          image_index: image_index,
          page_number: page_number,
          width: width,
          height: height,
          colorspace: colorspace,
          bits_per_component: bits_per_component,
          is_mask: is_mask,
          description: description,
          ocr_result: ocr_result&.to_h
        }
      end
    end

    # Per-page content
    #
    # @!attribute [r] page_number
    #   @return [Integer] Page number (1-indexed)
    # @!attribute [r] content
    #   @return [String] Text content for this page
    # @!attribute [r] tables
    #   @return [Array<Table>] Tables on this page
    # @!attribute [r] images
    #   @return [Array<Image>] Images on this page
    #
    PageContent = Struct.new(:page_number, :content, :tables, :images, keyword_init: true) do
      def to_h
        {
          page_number: page_number,
          content: content,
          tables: tables.map(&:to_h),
          images: images.map(&:to_h)
        }
      end
    end

    # Initialize from native hash result
    #
    # @param hash [Hash] Hash returned from native extension
    #
    def initialize(hash)
      # Handle both string and symbol keys for flexibility
      @content = get_value(hash, 'content', '')
      @mime_type = get_value(hash, 'mime_type', '')
      @metadata_json = get_value(hash, 'metadata_json', '{}')
      @metadata = parse_metadata(@metadata_json)
      @tables = parse_tables(get_value(hash, 'tables'))
      @detected_languages = parse_detected_languages(get_value(hash, 'detected_languages'))
      @chunks = parse_chunks(get_value(hash, 'chunks'))
      @images = parse_images(get_value(hash, 'images'))
      @pages = parse_pages(get_value(hash, 'pages'))
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
        tables: serialize_tables,
        detected_languages: @detected_languages,
        chunks: serialize_chunks,
        images: serialize_images,
        pages: serialize_pages
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

    def serialize_tables
      @tables.map(&:to_h)
    end

    def serialize_chunks
      @chunks&.map(&:to_h)
    end

    def serialize_images
      @images&.map(&:to_h)
    end

    def serialize_pages
      @pages&.map(&:to_h)
    end

    def get_value(hash, key, default = nil)
      hash[key] || hash[key.to_sym] || default
    end

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
          byte_start: chunk_hash['byte_start'],
          byte_end: chunk_hash['byte_end'],
          token_count: chunk_hash['token_count'],
          chunk_index: chunk_hash['chunk_index'],
          total_chunks: chunk_hash['total_chunks'],
          first_page: chunk_hash['first_page'],
          last_page: chunk_hash['last_page'],
          embedding: chunk_hash['embedding']
        )
      end
    end

    def parse_images(images_data)
      return nil if images_data.nil?

      images_data.map do |image_hash|
        data = image_hash['data']
        data = data.dup.force_encoding(Encoding::BINARY) if data.respond_to?(:force_encoding)
        Image.new(
          data: data,
          format: image_hash['format'],
          image_index: image_hash['image_index'],
          page_number: image_hash['page_number'],
          width: image_hash['width'],
          height: image_hash['height'],
          colorspace: image_hash['colorspace'],
          bits_per_component: image_hash['bits_per_component'],
          is_mask: image_hash['is_mask'],
          description: image_hash['description'],
          ocr_result: image_hash['ocr_result'] ? Result.new(image_hash['ocr_result']) : nil
        )
      end
    end

    def parse_pages(pages_data)
      return nil if pages_data.nil?

      pages_data.map do |page_hash|
        PageContent.new(
          page_number: page_hash['page_number'],
          content: page_hash['content'],
          tables: parse_tables(page_hash['tables']),
          images: parse_images(page_hash['images'])
        )
      end
    end
  end
  # rubocop:enable Metrics/ClassLength
end
