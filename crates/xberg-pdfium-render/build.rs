// Copyright 2021, pdfium-sys Developers
// Copyright 2022 - 2024, pdfium-render Developers
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be

#[cfg(feature = "bindings")]
extern crate bindgen;

#[cfg(feature = "bindings")]
use {
    bindgen::BindgenError,
    std::env::var,
    std::ffi::OsStr,
    std::fs::{read_dir, write},
    std::path::PathBuf,
};

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
enum BuildError {
    #[cfg(feature = "bindings")]
    #[allow(dead_code)]
    IoError(std::io::Error),
    #[cfg(feature = "bindings")]
    #[allow(dead_code)]
    BindgenError(BindgenError),
    #[cfg(feature = "bindings")]
    #[allow(dead_code)]
    PathConversionError(PathBuf),
    #[cfg(feature = "bindings")]
    #[allow(dead_code)]
    NoHeaderFilesFound(String),
}

#[cfg(feature = "bindings")]
impl From<std::io::Error> for BuildError {
    fn from(item: std::io::Error) -> Self {
        BuildError::IoError(item)
    }
}

#[cfg(feature = "bindings")]
impl From<BindgenError> for BuildError {
    fn from(item: BindgenError) -> Self {
        BuildError::BindgenError(item)
    }
}

fn main() -> Result<(), BuildError> {
    println!("cargo:rustc-check-cfg=cfg(pdfium_bindings_generated)");
    #[cfg(feature = "bindings")]
    {
        build_bindings()?;
        if std::env::var("DOCS_RS").is_err() {
            println!("cargo:rustc-cfg=pdfium_bindings_generated");
        }
    }

    if std::env::var("PDFIUM_STATIC_LIB_PATH").is_ok() {
        println!("cargo:rustc-cfg=pdfium_use_static");
        statically_link_pdfium();
    } else if let Ok(path) = std::env::var("PDFIUM_DYNAMIC_LIB_PATH") {
        println!("cargo:rustc-link-lib=dylib=pdfium");
        println!("cargo:rustc-link-search=native={}", path);
    } else if std::env::var("TARGET")
        .map(|t| t != "wasm32-unknown-unknown")
        .unwrap_or(true)
    {
        // Neither env var is set, so this build links nothing: the crate falls back to its
        // `dynamic_bindings` module, which resolves `FPDF_*` symbols via `libloading::Library`
        // at first Pdfium use, not at link time. That call dlopen()s the platform library name
        // (`libpdfium.so`/`.dylib`/`pdfium.dll`) from the current working directory and then the
        // system library search path -- neither of which this build places anything into. If no
        // caller supplies a Pdfium binary out of band, `Pdfium::default()`/`bind_to_system_library()`
        // return (or, via `Pdfium::default()`, panic on) a load-library error the first time Pdfium
        // extraction actually runs, not during this build. See issue #702.
        println!(
            "cargo:warning=xberg-pdfium-render: no PDFIUM_STATIC_LIB_PATH or PDFIUM_DYNAMIC_LIB_PATH \
             set; this build will compile but cannot load libpdfium at runtime unless one is present \
             on the system library search path or working directory (see issue #702)"
        );
    }

    println!("cargo:rerun-if-env-changed=PDFIUM_STATIC_LIB_PATH");
    println!("cargo:rerun-if-env-changed=PDFIUM_DYNAMIC_LIB_PATH");

    Ok(())
}

#[cfg(feature = "bindings")]
fn build_bindings() -> Result<(), BuildError> {
    let mut release_dir_count = 0usize;

    for release in read_dir("include/")? {
        let release = release?.path();

        if release.is_dir() {
            release_dir_count += 1;

            build_bindings_for_one_pdfium_release(
                release
                    .file_name()
                    .ok_or(BuildError::PathConversionError(release.clone()))?
                    .to_str()
                    .ok_or(BuildError::PathConversionError(release.clone()))?,
            )?;
        }
    }

    if release_dir_count == 0 {
        // The `bindings` feature was explicitly enabled, but include/ contains no Pdfium
        // release directories at all, so build_bindings_for_one_pdfium_release() above was
        // never even called: the loop above simply finishes and this function would otherwise
        // return Ok(()), silently regenerating nothing. That is the same failure mode the
        // per-release guard in build_bindings_for_one_pdfium_release() exists to prevent, just
        // one scope further out.
        return Err(BuildError::NoHeaderFilesFound(
            "No Pdfium release directories found in include/; refusing to silently succeed \
             while the `bindings` feature is enabled. Add a release directory with header files."
                .to_string(),
        ));
    }

    Ok(())
}

#[cfg(feature = "bindings")]
fn build_bindings_for_one_pdfium_release(release: &str) -> Result<(), BuildError> {
    if var("DOCS_RS").is_err() {
        let header_file_extension = OsStr::new("h");
        let wrapper_file_name = OsStr::new("rust-import-wrapper.h");
        let mut included_header_files = Vec::new();

        for header in read_dir(format!("include/{}/", release))? {
            let header = header?.path();

            if header.is_file()
                && header.file_name().is_some()
                && header.extension() == Some(header_file_extension)
                && header.file_name() != Some(wrapper_file_name)
            {
                let header_file_name = header
                    .file_name()
                    .ok_or(BuildError::PathConversionError(header.clone()))?
                    .to_str()
                    .ok_or(BuildError::PathConversionError(header.clone()))?
                    .to_owned();

                included_header_files.push(header_file_name);
            }
        }

        if included_header_files.is_empty() {
            // The `bindings` feature was explicitly enabled, which means the caller asked for
            // a fresh bindgen run. Silently falling back to whatever pre-generated bindings
            // happen to be checked in would let a build "succeed" while regenerating nothing,
            // shipping stale bindings without any signal that regeneration never happened.
            return Err(BuildError::NoHeaderFilesFound(format!(
                "No header files found in include/{}/; refusing to silently reuse pre-generated bindings \
                 while the `bindings` feature is enabled. Add header files or disable the feature.",
                release
            )));
        }

        let wrapper = included_header_files
            .iter()
            .map(|file_name| format!("#include \"{}\"", file_name))
            .collect::<Vec<_>>()
            .join("\n");

        let out_path = PathBuf::from(var("OUT_DIR").expect("Cargo sets OUT_DIR for build scripts"));
        let wrapper_path = out_path.join(format!("{release}-rust-import-wrapper.h"));
        write(&wrapper_path, wrapper)?;

        let bindings = bindgen::Builder::default()
            .header(wrapper_path.to_string_lossy())
            .clang_arg(format!("-Iinclude/{release}"))
            .clang_arg("-DPDF_USE_SKIA")
            .clang_arg("-D_SKIA_SUPPORT_")
            .clang_arg("-DPDF_ENABLE_XFA")
            .clang_arg("-DPDF_ENABLE_V8")
            .generate_cstr(true)
            .enable_function_attribute_detection()
            .size_t_is_usize(true)
            .layout_tests(false)
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            .clang_args(["-fretain-comments-from-system-headers", "-fparse-all-comments"].iter())
            .generate_comments(true);

        #[cfg(all(feature = "pdfium_use_win32", target_os = "windows"))]
        let bindings = bindings.clang_arg("-D_WIN32");

        let bindings = bindings.generate()?;
        bindings.write_to_file(out_path.join(format!("{release}.rs")))?;
    }

    Ok(())
}

fn statically_link_pdfium() {
    if let Ok(path) = std::env::var("PDFIUM_STATIC_LIB_PATH") {
        println!("cargo:rustc-link-lib=static=pdfium");
        println!("cargo:rustc-link-search=native={}", path);

        let target = std::env::var("TARGET").unwrap_or_default();
        if target.contains("apple") {
            println!("cargo:rustc-link-lib=dylib=c++");
            println!("cargo:rustc-link-lib=framework=CoreGraphics");
        } else if target.contains("linux") || target.contains("gnu") || target.contains("musl") {
            if target.contains("musl") {
                println!("cargo:rustc-link-lib=static=stdc++");
            } else {
                println!("cargo:rustc-link-lib=dylib=stdc++");
            }
        }
    }
}
