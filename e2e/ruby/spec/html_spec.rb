# frozen_string_literal: true

# Auto-generated tests for html fixtures.

# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'html fixtures' do
  it 'html_complex_layout' do
    E2ERuby.run_fixture(
      'html_complex_layout',
      'web/taylor_swift.html',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['text/html']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 1_000)
    end
  end

  it 'html_simple_table' do
    E2ERuby.run_fixture(
      'html_simple_table',
      'web/simple_table.html',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['text/html']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 100)
      E2ERuby::Assertions.assert_content_contains_all(result, ['Product', 'Category', 'Price', 'Stock', 'Laptop', 'Electronics', 'Sample Data Table'])
      E2ERuby::Assertions.assert_table_count(result, 1, nil)
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
