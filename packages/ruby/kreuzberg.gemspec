# frozen_string_literal: true

require_relative 'lib/kreuzberg/version'

repo_root = File.expand_path('../..', __dir__)
crate_prefix = 'packages/ruby/'
git_cmd = %(git -C "#{repo_root}" ls-files -z #{crate_prefix})
git_files =
  `#{git_cmd}`.split("\x0")
              .select { |path| path.start_with?(crate_prefix) }
              .map { |path| path.delete_prefix(crate_prefix) }
fallback_files = Dir.chdir(__dir__) do
  Dir.glob(
    %w[
      README.md
      LICENSE
      ext/**/*.rs
      ext/**/*.rb
      ext/**/*.toml
      ext/**/*.lock
      ext/**/*.md
      ext/**/build.rs
      ext/**/Cargo.*
      exe/*
      lib/**/*.rb
      spec/**/*.rb
      vendor/**/*.rb
    ],
    File::FNM_DOTMATCH
  )
end
files = git_files.empty? ? fallback_files : git_files

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

  spec.files = files
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
  spec.add_development_dependency 'rubocop', '~> 1.66'
  spec.add_development_dependency 'rubocop-performance', '~> 1.21'
  spec.add_development_dependency 'rubocop-rspec', '~> 3.0'
  spec.add_development_dependency 'yard', '~> 0.9'
end
