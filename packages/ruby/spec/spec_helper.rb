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

    def create_test_file(content, filename: 'test.txt')
      path = File.join(Dir.tmpdir, filename)
      File.write(path, content)
      path
    end

    def create_test_pdf(content = 'Test PDF content')
      # This is a minimal PDF structure for testing
      pdf_content = <<~PDF
        %PDF-1.4
        1 0 obj
        << /Type /Catalog /Pages 2 0 R >>
        endobj
        2 0 obj
        << /Type /Pages /Kids [3 0 R] /Count 1 >>
        endobj
        3 0 obj
        << /Type /Page /Parent 2 0 R /Resources 4 0 R /MediaBox [0 0 612 792] /Contents 5 0 R >>
        endobj
        4 0 obj
        << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>
        endobj
        5 0 obj
        << /Length 44 >>
        stream
        BT
        /F1 12 Tf
        100 700 Td
        (#{content}) Tj
        ET
        endstream
        endobj
        xref
        0 6
        0000000000 65535 f
        0000000009 00000 n
        0000000058 00000 n
        0000000115 00000 n
        0000000227 00000 n
        0000000313 00000 n
        trailer
        << /Size 6 /Root 1 0 R >>
        startxref
        407
        %%EOF
      PDF
      create_test_file(pdf_content, filename: 'test.pdf')
    end
  end)
end
