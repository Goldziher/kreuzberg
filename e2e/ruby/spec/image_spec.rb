# frozen_string_literal: true

# Auto-generated tests for image fixtures.

# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
require_relative 'spec_helper'

RSpec.describe 'image fixtures' do
  it 'image_metadata_only' do
    E2ERuby.run_fixture(
      'image_metadata_only',
      'images/example.jpg',
      { ocr: nil },
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['image/jpeg']
      )
      E2ERuby::Assertions.assert_max_content_length(result, 100)
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength, Metrics/BlockLength
