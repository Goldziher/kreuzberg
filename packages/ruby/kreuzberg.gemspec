# frozen_string_literal: true

require_relative 'lib/kreuzberg/version'

repo_root = File.expand_path('../..', __dir__)

# Include files from packages/ruby
ruby_prefix = 'packages/ruby/'
ruby_cmd = %(git -C "#{repo_root}" ls-files -z #{ruby_prefix})
ruby_files =
  `#{ruby_cmd}`.split("\x0")
              .select { |path| path.start_with?(ruby_prefix) }
              .map { |path| path.delete_prefix(ruby_prefix) }

# Include the kreuzberg core crate (needed for path patch in Cargo.toml)
core_prefix = 'crates/kreuzberg/'
core_cmd = %(git -C "#{repo_root}" ls-files -z #{core_prefix})
core_files =
  `#{core_cmd}`.split("\x0")
              .select { |path| path.start_with?(core_prefix) }
              .map { |path| path.delete_prefix('crates/') }
              .map { |path| "vendor/#{path}" }

fallback_files = Dir.chdir(__dir__) do
  ruby_fallback = Dir.glob(
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
      sig/**/*.rbs
      spec/**/*.rb
    ],
    File::FNM_DOTMATCH
  )

  # Fallback for core crate - copy from repo root
  core_fallback = Dir.chdir(repo_root) do
    Dir.glob('crates/kreuzberg/**/*', File::FNM_DOTMATCH)
      .reject { |f| File.directory?(f) }
      .map { |path| "vendor/#{path.delete_prefix('crates/')}" }
  end

  ruby_fallback + core_fallback
end

# Check for vendored kreuzberg crate (copied during CI/packaging)
vendor_files = Dir.chdir(__dir__) do
  if Dir.exist?('vendor/kreuzberg')
    Dir.glob('vendor/kreuzberg/**/*', File::FNM_DOTMATCH)
      .reject { |f| File.directory?(f) }
      .reject { |f| f.include?('/.fastembed_cache/') }
      .reject { |f| f.include?('/target/') }
      .reject { |f| f =~ /\.(swp|bak|tmp)$/ }
      .reject { |f| f =~ /~$/ }
  else
    []
  end
end

# Use git-tracked files if available, otherwise fallback to glob
# Always include vendored files if they exist on disk (for CI packaging)
files = if (ruby_files + core_files).empty?
          fallback_files
        elsif vendor_files.any?
          ruby_files + vendor_files
        else
          ruby_files + core_files
        end

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
    'rubygems_mfa_required' => 'true',
    'keywords' => 'document-intelligence,document-extraction,ocr,rust,bindings'
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
  unless Gem.win_platform?
    spec.add_development_dependency 'rbs', '~> 3.0'
    spec.add_development_dependency 'rubocop', '~> 1.66'
    spec.add_development_dependency 'rubocop-performance', '~> 1.21'
    spec.add_development_dependency 'rubocop-rspec', '~> 3.0'
    spec.add_development_dependency 'steep', '~> 1.8'
  end
  spec.add_development_dependency 'yard', '~> 0.9'
end
