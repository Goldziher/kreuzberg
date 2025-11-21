# frozen_string_literal: true

# Auto-generated tests for xml fixtures.

# rubocop:disable Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'xml fixtures' do
  it 'xml_plant_catalog' do
    E2ERuby.run_fixture(
      'xml_plant_catalog',
      'xml/plant_catalog.xml',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['application/xml']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
      E2ERuby::Assertions.assert_metadata_expectation(result, 'element_count', { gte: 1 })
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
