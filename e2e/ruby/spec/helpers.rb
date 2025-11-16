# frozen_string_literal: true

# rubocop:disable Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/MethodLength, Metrics/PerceivedComplexity, Metrics/ParameterLists, Style/Documentation, Style/IfUnlessModifier, Layout/LineLength, Layout/EmptyLineAfterGuardClause

require 'json'
require 'pathname'
require 'rspec/expectations'
require 'kreuzberg'
require 'rspec/core'

module E2ERuby
  module_function

  WORKSPACE_ROOT = Pathname.new(__dir__).join('..', '..', '..').expand_path
  TEST_DOCUMENTS = WORKSPACE_ROOT.join('test_documents')

  def resolve_document(relative)
    TEST_DOCUMENTS.join(relative)
  end

  def build_config(raw)
    return nil unless raw.is_a?(Hash) && !raw.empty?

    symbolize_keys(raw)
  end

  def symbolize_keys(value)
    case value
    when Hash
      value.each_with_object({}) do |(key, val), acc|
        symbol_key = key.respond_to?(:to_sym) ? key.to_sym : key
        acc[symbol_key] = symbolize_keys(val)
      end
    when Array
      value.map { |item| symbolize_keys(item) }
    else
      value
    end
  end

  def skip_reason_for(error, fixture_id, requirements, notes = nil)
    message = error.message.to_s
    downcased = message.downcase
    requirement_hit = requirements.any? { |req| downcased.include?(req.downcase) }
    missing_dependency = error.is_a?(Kreuzberg::Errors::MissingDependencyError) || downcased.include?('missing dependency')
    unsupported_format = downcased.include?('unsupported format')

    return nil unless missing_dependency || unsupported_format || requirement_hit

    reason =
      if missing_dependency
        dependency = error.respond_to?(:dependency) ? error.dependency : nil
        if dependency && !dependency.to_s.empty?
          "missing dependency #{dependency}"
        else
          'missing dependency'
        end
      elsif unsupported_format
        'unsupported format'
      elsif requirements.any?
        "requires #{requirements.join(', ')}"
      else
        'environmental requirement'
      end

    details = "Skipping #{fixture_id}: #{reason}. #{error.class}: #{message}"
    details += " Notes: #{notes}" if notes
    warn(details)
    details
  end

  def run_fixture(fixture_id, relative_path, config_hash, requirements:, notes:, skip_if_missing: true)
    document_path = resolve_document(relative_path)

    if skip_if_missing && !document_path.exist?
      warn "Skipping #{fixture_id}: missing document at #{document_path}"
      raise RSpec::Core::Pending::SkipDeclaredInExample, 'missing document'
    end

    config = build_config(config_hash)
    result = nil
    begin
      result = Kreuzberg.extract_file_sync(document_path.to_s, config: config)
    rescue StandardError => e
      if (reason = skip_reason_for(e, fixture_id, requirements, notes))
        raise RSpec::Core::Pending::SkipDeclaredInExample, reason
      end
      raise
    end

    yield result
  end

  module Assertions
    extend RSpec::Matchers

    def self.assert_expected_mime(result, expected)
      return if expected.empty?

      expect(expected.any? { |token| result.mime_type.include?(token) }).to be(true)
    end

    def self.assert_min_content_length(result, minimum)
      expect(result.content.length).to be >= minimum
    end

    def self.assert_max_content_length(result, maximum)
      expect(result.content.length).to be <= maximum
    end

    def self.assert_content_contains_any(result, snippets)
      return if snippets.empty?

      lowered = result.content.downcase
      expect(snippets.any? { |snippet| lowered.include?(snippet.downcase) }).to be(true)
    end

    def self.assert_content_contains_all(result, snippets)
      return if snippets.empty?

      lowered = result.content.downcase
      expect(snippets.all? { |snippet| lowered.include?(snippet.downcase) }).to be(true)
    end

    def self.assert_table_count(result, minimum, maximum)
      tables = Array(result.tables)
      expect(tables.length).to be >= minimum if minimum
      expect(tables.length).to be <= maximum if maximum
    end

    def self.assert_detected_languages(result, expected, min_confidence)
      return if expected.empty?

      languages = result.detected_languages
      expect(languages).not_to be_nil
      expect(expected.all? { |lang| languages.include?(lang) }).to be(true)

      return unless min_confidence

      metadata = result.metadata || {}
      confidence = metadata['confidence'] || metadata[:confidence]
      expect(confidence).to be >= min_confidence if confidence
    end

    def self.assert_metadata_expectation(result, path, expectation)
      metadata = result.metadata || {}
      value = fetch_metadata_value(metadata, path)
      raise "Metadata path '#{path}' missing in #{metadata.inspect}" if value.nil?

      if expectation.key?(:eq)
        expect(values_equal?(value, expectation[:eq])).to be(true)
      end

      if expectation.key?(:gte)
        expect(convert_numeric(value)).to be >= convert_numeric(expectation[:gte])
      end

      if expectation.key?(:lte)
        expect(convert_numeric(value)).to be <= convert_numeric(expectation[:lte])
      end

      return unless expectation.key?(:contains)

      contains = expectation[:contains]
      if value.is_a?(String) && contains.is_a?(String)
        expect(value.include?(contains)).to be(true)
      elsif value.is_a?(Array) && contains.is_a?(Array)
        expect(contains.all? { |item| value.include?(item) }).to be(true)
      else
        raise "Unsupported contains expectation for path '#{path}'"
      end
    end

    class << self
      private

      def fetch_metadata_value(metadata, path)
        current = metadata
        path.split('.').each do |segment|
          return nil unless current.is_a?(Hash)

          current = current[segment] || current[segment.to_sym]
        end
        current
      end

      def values_equal?(lhs, rhs)
        return lhs == rhs if lhs.is_a?(String) && rhs.is_a?(String)
        return convert_numeric(lhs) == convert_numeric(rhs) if numeric_like?(lhs) && numeric_like?(rhs)
        return lhs == rhs if lhs == rhs

        lhs == rhs
      end

      def numeric_like?(value)
        value.is_a?(Numeric) || value.respond_to?(:to_f)
      end

      def convert_numeric(value)
        return value if value.is_a?(Numeric)

        value.to_f
      end
    end
  end
end
# rubocop:enable Metrics/AbcSize, Metrics/CyclomaticComplexity, Metrics/MethodLength, Metrics/PerceivedComplexity, Metrics/ParameterLists, Style/Documentation, Style/IfUnlessModifier, Layout/LineLength, Layout/EmptyLineAfterGuardClause
