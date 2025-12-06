# frozen_string_literal: true

require 'kreuzberg'
require 'tmpdir'
require 'fileutils'

RSpec.configure do |config|
  config.expect_with :rspec do |expectations|
    expectations.include_chain_clauses_in_custom_matcher_descriptions = true
  end

  config.mock_with :rspec do |mocks|
    mocks.verify_partial_doubles = true
  end

  config.shared_context_metadata_behavior = :apply_to_host_groups
  config.filter_run_when_matching :focus
  config.example_status_persistence_file_path = 'spec/examples.txt'
  config.disable_monkey_patching!
  config.warnings = true
  config.default_formatter = 'doc' if config.files_to_run.one?
  config.order = :random
  Kernel.srand config.seed

  # Helpers
  config.include(Module.new do
    def fixture_path(filename)
      File.join(__dir__, 'fixtures', filename)
    end

    def test_document_path(relative_path)
      # Go up from packages/ruby/spec to project root, then into test_documents
      File.expand_path(File.join(__dir__, '..', '..', '..', 'test_documents', relative_path))
    end

    def create_test_file(content, filename: 'test.txt')
      path = File.join(Dir.tmpdir, filename)
      File.write(path, content)
      path
    end
  end)
end
