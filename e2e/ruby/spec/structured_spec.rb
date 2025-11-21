# frozen_string_literal: true

# Auto-generated tests for structured fixtures.

# rubocop:disable Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'structured fixtures' do
  it 'structured_json_basic' do
    E2ERuby.run_fixture(
      'structured_json_basic',
      'json/sample_document.json',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/json']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
      E2ERuby::Assertions.assert_content_contains_any(result, ['Sample Document', 'Test Author'])
    end
  end

  it 'structured_json_simple' do
    E2ERuby.run_fixture(
      'structured_json_simple',
      'data_formats/simple.json',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/json']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
      E2ERuby::Assertions.assert_content_contains_any(result, ['{', 'name'])
    end
  end

  it 'structured_yaml_simple' do
    E2ERuby.run_fixture(
      'structured_yaml_simple',
      'data_formats/simple.yaml',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/x-yaml']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 10)
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
