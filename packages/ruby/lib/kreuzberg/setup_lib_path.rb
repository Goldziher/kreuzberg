# frozen_string_literal: true

require 'rbconfig'
require 'open3'

module Kreuzberg
  module SetupLibPath
    module_function

    def configure
      lib_dir = File.expand_path('..', __dir__)
      host_os = RbConfig::CONFIG['host_os']

      case host_os
      when /darwin/
        prepend_env('DYLD_LIBRARY_PATH', lib_dir)
        prepend_env('DYLD_FALLBACK_LIBRARY_PATH', "#{lib_dir}:/usr/local/lib:/usr/lib")
        fix_macos_install_name(lib_dir)
      when /linux/
        prepend_env('LD_LIBRARY_PATH', lib_dir)
      when /mswin|mingw|cygwin/
        prepend_env('PATH', lib_dir, separator: ';')
      end
    end

    def prepend_env(key, value, separator: ':')
      current = ENV.fetch(key, nil)
      return if current&.split(separator)&.include?(value)

      ENV[key] = current.nil? || current.empty? ? value : "#{value}#{separator}#{current}"
    end
    private_class_method :prepend_env

    def fix_macos_install_name(lib_dir)
      bundle = macos_bundle(lib_dir)
      return unless bundle

      ensure_install_name(bundle)
      ensure_loader_rpath(bundle)
    rescue Errno::ENOENT, IOError
      # Tool not available (e.g., on CI). The dynamic loader can still use the updated env vars.
    end
    private_class_method :fix_macos_install_name

    def macos_bundle(lib_dir)
      bundle = File.join(lib_dir, 'kreuzberg_rb.bundle')
      pdfium = File.join(lib_dir, 'libpdfium.dylib')
      return unless File.exist?(bundle) && File.exist?(pdfium)

      bundle
    end
    private_class_method :macos_bundle

    def ensure_install_name(bundle)
      output, status = Open3.capture2('otool', '-L', bundle)
      return unless status.success?

      replacements = {
        './libpdfium.dylib' => '@loader_path/libpdfium.dylib',
        '@rpath/libpdfium.dylib' => '@loader_path/libpdfium.dylib'
      }

      replacements.each do |current, desired|
        next unless output.include?(current)

        Open3.capture2('install_name_tool', '-change', current, desired, bundle)
      end
    end
    private_class_method :ensure_install_name

    def ensure_loader_rpath(bundle)
      rpath_output, rpath_status = Open3.capture2('otool', '-l', bundle)
      return unless rpath_status.success? && !rpath_output.include?('@loader_path')

      Open3.capture2('install_name_tool', '-add_rpath', '@loader_path', bundle)
    end
    private_class_method :ensure_loader_rpath
  end
end
