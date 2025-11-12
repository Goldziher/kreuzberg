# frozen_string_literal: true

require 'mkmf'
require 'rb_sys/mkmf'
require 'rbconfig'

if /mswin|mingw/.match?(RbConfig::CONFIG['host_os'])
  devkit = ENV.fetch('RI_DEVKIT', nil)
  prefix = ENV['MSYSTEM_PREFIX'] || '/ucrt64'

  if devkit
    sysroot = "#{devkit}#{prefix}".tr('\\\\', '/')
    extra_args = [
      '--target=x86_64-pc-windows-gnu',
      "--sysroot=#{sysroot}"
    ]

    existing = ENV['BINDGEN_EXTRA_CLANG_ARGS'].to_s.split(/\s+/)
    ENV['BINDGEN_EXTRA_CLANG_ARGS'] = (existing + extra_args).uniq.join(' ')
  end
end

default_profile = ENV.fetch('CARGO_PROFILE', 'release')
native_dir = 'native'

create_rust_makefile('kreuzberg_rb') do |config|
  config.profile = default_profile.to_sym
  config.ext_dir = native_dir
end
