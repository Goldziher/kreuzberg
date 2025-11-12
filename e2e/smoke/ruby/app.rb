# frozen_string_literal: true

fixture = File.expand_path('fixtures/report.txt', __dir__)
unless File.exist?(fixture)
  warn "Fixture not found: #{fixture}"
  exit 1
end

require 'kreuzberg'

result = Kreuzberg.extract_file_sync(fixture)
content = result&.content.to_s
unless content.include?('smoke')
  warn 'Smoke test failed: snippet missing'
  exit 1
end

puts '[ruby smoke] extraction succeeded'
