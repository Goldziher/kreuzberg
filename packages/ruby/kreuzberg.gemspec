# frozen_string_literal: true

require_relative 'lib/kreuzberg/version'

Gem::Specification.new do |spec|
  spec.name = 'kreuzberg'
  spec.version = Kreuzberg::VERSION
  spec.authors = ['Na\'aman Hirschfeld']
  spec.email = ['nhirschfeld@gmail.com']

  spec.summary = 'High-performance document intelligence framework'
  spec.description = <<~DESC
    Kreuzberg is a multi-language document intelligence framework with a high-performance
    Rust core. Supports extraction, OCR, chunking, and language detection for 30+ file formats
    including PDF, DOCX, PPTX, XLSX, images, and more.
  DESC
  spec.homepage = 'https://github.com/Goldziher/kreuzberg'
  spec.license = 'MIT'
  spec.required_ruby_version = '>= 3.2.0'

  spec.metadata = {
    'homepage_uri' => spec.homepage,
    'source_code_uri' => 'https://github.com/Goldziher/kreuzberg',
    'changelog_uri' => 'https://github.com/Goldziher/kreuzberg/blob/main/CHANGELOG.md',
    'documentation_uri' => 'https://docs.kreuzberg.dev',
    'bug_tracker_uri' => 'https://github.com/Goldziher/kreuzberg/issues',
    'rubygems_mfa_required' => 'true'
  }

  spec.files = Dir[
    'lib/**/*.rb',
    'ext/**/*.{rb,toml,lock}',
    'LICENSE',
    'README.md'
  ]
  spec.bindir = 'exe'
  spec.executables = spec.files.grep(%r{^exe/}) { |f| File.basename(f) }
  spec.require_paths = ['lib']
  spec.extensions = ['ext/kreuzberg_rb/extconf.rb']

  # Runtime dependencies
  # None - the gem is self-contained with the Rust extension

  # Development dependencies
  spec.add_development_dependency 'bundler', '~> 2.0'
  spec.add_development_dependency 'rake', '~> 13.0'
  spec.add_development_dependency 'rake-compiler', '~> 1.2'
  spec.add_development_dependency 'rb_sys', '~> 0.9'
  spec.add_development_dependency 'rspec', '~> 3.12'
  spec.add_development_dependency 'rubocop', '~> 1.50'
  spec.add_development_dependency 'rubocop-performance', '~> 1.18'
  spec.add_development_dependency 'rubocop-rspec', '~> 2.22'
  spec.add_development_dependency 'yard', '~> 0.9'
end
