# frozen_string_literal: true

# rubocop:disable RSpec/DescribeClass, RSpec/ExampleLength
require_relative 'spec_helper'

RSpec.describe 'email fixtures' do
  it 'email_sample_eml' do
    E2ERuby.run_fixture(
      'email_sample_eml',
      'email/sample.eml',
      nil,
      requirements: [],
      notes: nil,
      skip_if_missing: true
    ) do |result|
      E2ERuby::Assertions.assert_expected_mime(
        result,
        ['message/rfc822']
      )
      E2ERuby::Assertions.assert_min_content_length(result, 20)
    end
  end
end
# rubocop:enable RSpec/DescribeClass, RSpec/ExampleLength
