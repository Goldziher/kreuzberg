# frozen_string_literal: true

require 'mkmf'
require 'rb_sys/mkmf'
require 'rbconfig'

# Set shorter build directory for Windows to avoid MAX_PATH issues
if Gem.win_platform?
  # Use a much shorter path to avoid Windows MAX_PATH (260 char) limit
  short_target_dir = 'D:/kz-build'
  ENV['CARGO_TARGET_DIR'] = short_target_dir
  ENV['OUT_DIR'] = short_target_dir
  puts "Windows detected: Using short build path #{short_target_dir}"
end

if /mswin|mingw/.match?(RbConfig::CONFIG['host_os'])
  devkit = ENV.fetch('RI_DEVKIT', nil)
  prefix = ENV['MSYSTEM_PREFIX'] || '/ucrt64'
  compat_include = File.expand_path('native/include/msvc_compat', __dir__).tr('\\', '/')

  extra_args = []
  extra_args << "-I#{compat_include}"

  if devkit
    sysroot = "#{devkit}#{prefix}".tr('\\\\', '/')
    extra_args.concat([
                        '--target=x86_64-pc-windows-gnu',
                        "--sysroot=#{sysroot}"
                      ])
  end

  unless extra_args.empty?
    existing = ENV['BINDGEN_EXTRA_CLANG_ARGS'].to_s.split(/\s+/).reject(&:empty?)
    ENV['BINDGEN_EXTRA_CLANG_ARGS'] = (existing + extra_args).uniq.join(' ')
  end
end

default_profile = ENV.fetch('CARGO_PROFILE', 'release')
native_dir = 'native'

create_rust_makefile('kreuzberg_rb') do |config|
  config.profile = default_profile.to_sym
  # Build from the actual Rust crate location (native/) so Cargo.toml is resolvable in packaged gems
  config.ext_dir = native_dir
end
