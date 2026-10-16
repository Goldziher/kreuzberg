//! Build script for xberg-tesseract: when `build-tesseract`/`build-tesseract-wasm` is
//! enabled, downloads and builds vendored Leptonica and Tesseract via CMake, resolving
//! a per-platform cache directory for the compiled artifacts.

#![allow(clippy::uninlined_format_args)]

#[cfg(any(
    feature = "build-tesseract-wasm",
    all(feature = "build-tesseract", not(feature = "dynamic-linking"))
))]
#[path = "build_support/source_archive.rs"]
mod source_archive;
#[cfg(any(
    feature = "build-tesseract-wasm",
    all(feature = "build-tesseract", not(feature = "dynamic-linking"))
))]
#[path = "build_support/source_cache.rs"]
mod source_cache;

#[cfg(any(
    feature = "build-tesseract-wasm",
    all(feature = "build-tesseract", not(feature = "dynamic-linking"))
))]
mod build_tesseract {
    use crate::source_archive::{ArchiveLimits, extract_source_archive};
    use crate::source_cache::{
        PreparedSourceTree, SourceArtifact, canonicalize_trusted_build_root, cmake_source_path, copy_verified_artifact,
        ensure_directory, ensure_private_cache_root, prepare_source_tree, prepare_verified_artifact, read_exact_size,
        tool_argument_path,
    };
    use cmake::Config;
    use std::env;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    const LEPTONICA_VERSION: &str = "1.87.0";
    const LEPTONICA_REVISION: &str = "13275a278eb55b5746e33f95fbf5a2c8f604b3ab";
    const LEPTONICA_SHA256: &str = "0febcd4fc5cdc9c52d59509b45483d107f9f40922899e3f134ea615094ecbc77";
    const LEPTONICA_ARCHIVE_SIZE: u64 = 14_348_280;
    const LEPTONICA_ARCHIVE_ROOT: &str = "leptonica-13275a278eb55b5746e33f95fbf5a2c8f604b3ab";
    const LEPTONICA_LICENSE_FILE: &str = "leptonica-license.txt";
    const TESSERACT_VERSION: &str = "5.5.3";
    const TESSERACT_REVISION: &str = "db0ec62f81b0737fbbe184d8fea40af5738f8eef";
    const TESSERACT_SHA256: &str = "d2470cc33ee34deeae6fc47809d0b33a3623a4343d92ff317ac3b9903c507bad";
    const TESSERACT_ARCHIVE_SIZE: u64 = 2_533_335;
    const TESSERACT_ARCHIVE_ROOT: &str = "tesseract-db0ec62f81b0737fbbe184d8fea40af5738f8eef";
    const TESSERACT_LICENSE_FILE: &str = "LICENSE";
    const TESSDATA_FAST_REVISION: &str = "87416418657359cb625c412a48b6e1d6d41c29bd";
    const ENG_TRAINEDDATA_SHA256: &str = "7d4322bd2a7749724879683fc3912cb542f19906c83bcc1a52132556427170b2";
    const ENG_TRAINEDDATA_SIZE: u64 = 4_113_088;
    const MAX_SOURCE_ARCHIVE_ENTRIES: usize = 50_000;
    const MAX_SOURCE_UNCOMPRESSED_BYTES: u64 = 512 * 1024 * 1024;
    const SOURCE_ARCHIVE_LIMITS: ArchiveLimits = ArchiveLimits {
        max_entries: MAX_SOURCE_ARCHIVE_ENTRIES,
        max_uncompressed_bytes: MAX_SOURCE_UNCOMPRESSED_BYTES,
    };

    const LEPTONICA_ARTIFACT: SourceArtifact<'static> = SourceArtifact {
        name: "leptonica.zip",
        cache_key: "leptonica-source",
        sha256: LEPTONICA_SHA256,
        expected_size: LEPTONICA_ARCHIVE_SIZE,
    };
    const TESSERACT_ARTIFACT: SourceArtifact<'static> = SourceArtifact {
        name: "tesseract.zip",
        cache_key: "tesseract-source",
        sha256: TESSERACT_SHA256,
        expected_size: TESSERACT_ARCHIVE_SIZE,
    };
    const ENG_TRAINEDDATA_ARTIFACT: SourceArtifact<'static> = SourceArtifact {
        name: "eng.traineddata",
        cache_key: "tessdata-fast-eng",
        sha256: ENG_TRAINEDDATA_SHA256,
        expected_size: ENG_TRAINEDDATA_SIZE,
    };

    struct SourceArchiveSpec {
        artifact: SourceArtifact<'static>,
        source_name: &'static str,
        archive_root: &'static str,
        license_file: &'static str,
    }

    const LEPTONICA_SOURCE: SourceArchiveSpec = SourceArchiveSpec {
        artifact: LEPTONICA_ARTIFACT,
        source_name: "leptonica",
        archive_root: LEPTONICA_ARCHIVE_ROOT,
        license_file: LEPTONICA_LICENSE_FILE,
    };
    const TESSERACT_SOURCE: SourceArchiveSpec = SourceArchiveSpec {
        artifact: TESSERACT_ARTIFACT,
        source_name: "tesseract",
        archive_root: TESSERACT_ARCHIVE_ROOT,
        license_file: TESSERACT_LICENSE_FILE,
    };

    fn leptonica_url() -> String {
        format!(
            "https://codeload.github.com/DanBloomberg/leptonica/zip/{}",
            LEPTONICA_REVISION
        )
    }

    fn tesseract_url() -> String {
        format!(
            "https://codeload.github.com/tesseract-ocr/tesseract/zip/{}",
            TESSERACT_REVISION
        )
    }

    fn tessdata_fast_urls() -> [String; 2] {
        [
            format!(
                "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/{}/eng.traineddata",
                TESSDATA_FAST_REVISION
            ),
            format!(
                "https://github.com/tesseract-ocr/tessdata_fast/raw/{}/eng.traineddata",
                TESSDATA_FAST_REVISION
            ),
        ]
    }

    fn get_or_download_source(
        artifact_cache_dir: &Path,
        third_party_dir: &Path,
        source: &SourceArchiveSpec,
        url: &str,
    ) -> PreparedSourceTree {
        let verified_archive = prepare_verified_artifact(artifact_cache_dir, &source.artifact, || {
            download_file_with_fallback(&[url], source.source_name, source.artifact.expected_size)
        })
        .unwrap_or_else(|error| panic!("Failed to verify {} source archive: {error}", source.source_name));

        let prepared = prepare_source_tree(
            third_party_dir,
            source.source_name,
            &verified_archive,
            |archive, destination| {
                extract_source_archive(archive, destination, source.archive_root, SOURCE_ARCHIVE_LIMITS)
            },
        )
        .unwrap_or_else(|error| panic!("Failed to prepare {} source: {error}", source.source_name));

        if !prepared.path.join(source.license_file).is_file() {
            panic!(
                "Verified {} source is missing required license file {}",
                source.source_name,
                prepared.path.join(source.license_file).display()
            );
        }

        if !prepared.downloaded {
            eprintln!("Using verified cached {} archive", source.source_name);
        }

        prepared
    }

    fn prepare_eng_traineddata(artifact_cache_dir: &Path, destination: &Path) {
        let urls = tessdata_fast_urls();
        let url_refs = urls.iter().map(String::as_str).collect::<Vec<_>>();
        let verified_model = prepare_verified_artifact(artifact_cache_dir, &ENG_TRAINEDDATA_ARTIFACT, || {
            download_file_with_fallback(&url_refs, "eng.traineddata", ENG_TRAINEDDATA_SIZE)
        })
        .unwrap_or_else(|error| panic!("Failed to verify eng.traineddata: {error}"));

        copy_verified_artifact(&verified_model, destination)
            .unwrap_or_else(|error| panic!("Failed to prepare bundled eng.traineddata: {error}"));
    }

    fn workspace_cache_dir_from_out_dir() -> Option<PathBuf> {
        let mut path = cargo_build_root();
        for _ in 0..4 {
            if !path.pop() {
                return None;
            }
        }
        Some(path.join("xberg-tesseract-cache"))
    }

    fn get_preferred_out_dir() -> (PathBuf, bool) {
        if let Ok(custom) = env::var("TESSERACT_RS_CACHE_DIR") {
            return (PathBuf::from(custom), false);
        }

        (
            workspace_cache_dir_from_out_dir().unwrap_or_else(|| cargo_build_root().join("xberg-tesseract-cache")),
            true,
        )
    }

    fn target_triple() -> String {
        env::var("TARGET").unwrap_or_else(|_| env::var("HOST").unwrap_or_default())
    }

    fn target_matches(target: &str, needle: &str) -> bool {
        target.contains(needle)
    }

    fn is_windows_target(target: &str) -> bool {
        target_matches(target, "windows")
    }

    fn is_macos_target(target: &str) -> bool {
        target_matches(target, "apple-darwin")
    }

    fn is_linux_target(target: &str) -> bool {
        target_matches(target, "linux")
    }

    fn is_msvc_target(target: &str) -> bool {
        is_windows_target(target) && target_matches(target, "msvc")
    }

    fn is_mingw_target(target: &str) -> bool {
        is_windows_target(target) && target_matches(target, "gnu")
    }

    fn is_wasm_target(target: &str) -> bool {
        target_matches(target, "wasm32") || target_matches(target, "wasm64")
    }

    fn is_android_target(target: &str) -> bool {
        target_matches(target, "android")
    }

    /// Map a Rust Android target triple to the NDK ABI name.
    fn android_abi(target: &str) -> &'static str {
        if target.contains("aarch64") {
            "arm64-v8a"
        } else if target.contains("x86_64") {
            "x86_64"
        } else if target.contains("i686") {
            "x86"
        } else {
            "armeabi-v7a"
        }
    }

    /// Derive the versioned NDK clang++ path for a given ABI.
    /// e.g. `{ndk}/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android21-clang++`
    fn ndk_clangxx(ndk_home: &str, abi: &str, api: u32) -> Option<String> {
        let host_tags: &[&str] = if cfg!(target_os = "macos") {
            &["darwin-x86_64", "darwin-aarch64"]
        } else {
            &["linux-x86_64", "linux-aarch64"]
        };
        let clang_arch = match abi {
            "arm64-v8a" => "aarch64-linux-android",
            "x86_64" => "x86_64-linux-android",
            "x86" => "i686-linux-android",
            _ => "armv7a-linux-androideabi",
        };
        for tag in host_tags {
            let bin = format!("{}/toolchains/llvm/prebuilt/{}/bin", ndk_home, tag);
            let path = format!("{}/{}{}-clang++", bin, clang_arch, api);
            if Path::new(&path).exists() {
                return Some(path);
            }
        }
        None
    }

    /// Detect whether the build is driven by cargo-zigbuild, which wraps the
    /// C toolchain in a `zigcc`/`zigcxx` shim. zig's bundled libstdc++ has
    /// `std::filesystem` inline (no standalone `libstdc++fs`) and its clang
    /// splits `avx512f` from `evex512`, so tesseract's AVX512 intrinsics
    /// fail to compile. Both workarounds below gate on this.
    fn is_zigbuild() -> bool {
        env::vars().any(|(k, v)| {
            let k_relevant = k == "CC"
                || k == "CXX"
                || k == "RUSTC_LINKER"
                || k.starts_with("CC_")
                || k.starts_with("CXX_")
                || (k.starts_with("CARGO_TARGET_") && k.ends_with("_LINKER"));
            k_relevant && (v.contains("zigcc") || v.contains("zigcxx") || v.contains("cargo-zigbuild"))
        })
    }

    /// Resolve the C++ compiler for CMake, following the cc-rs/Cargo convention:
    /// 1. Check `CXX` env var (explicit override)
    /// 2. Check target-specific `CXX_{target}` env var (e.g. `CXX_x86_64_unknown_linux_musl`)
    /// 3. Fall back to `{fallback}` (e.g. "clang++" or "g++")
    fn resolve_cxx_compiler(target: &str, fallback: &str) -> String {
        if let Ok(cxx) = env::var("CXX")
            && !cxx.is_empty()
        {
            return cxx;
        }

        let target_env = target.replace('-', "_");
        if let Ok(cxx) = env::var(format!("CXX_{target_env}"))
            && !cxx.is_empty()
        {
            return cxx;
        }

        fallback.to_string()
    }

    /// Resolve a MinGW compiler to an absolute path.
    ///
    /// On Windows CI runners (GitHub Actions), both MSVC and MinGW toolchains
    /// are present. CMake may pick up MSVC's cl.exe even when
    /// `CMAKE_CXX_COMPILER=g++` is set, producing MSVC-ABI objects that
    /// MinGW's linker cannot link. Using the absolute path prevents this.
    ///
    /// Search order:
    /// 1. `CXX`/`CC` env var (if it matches the tool name)
    /// 2. Common MSYS2 paths: ucrt64, mingw64, clang64, usr
    /// 3. Fall back to bare name (rely on PATH)
    fn resolve_mingw_compiler(name: &str) -> String {
        let env_var = if name.contains("++") { "CXX" } else { "CC" };
        if let Ok(val) = env::var(env_var)
            && !val.is_empty()
        {
            let p = PathBuf::from(&val);
            if p.is_absolute() && p.exists() {
                return val;
            }
        }

        let msys2_base = PathBuf::from(r"C:\msys64");
        for subsystem in &["ucrt64", "mingw64", "clang64", "usr"] {
            let candidate = msys2_base.join(subsystem).join("bin").join(format!("{}.exe", name));
            if candidate.exists() {
                let path = candidate.to_string_lossy().replace('\\', "/");
                eprintln!("Resolved MinGW {} to {}", name, path);
                return path;
            }
        }

        println!(
            "cargo:warning=Could not resolve absolute path for MinGW {}, using bare name",
            name
        );
        name.to_string()
    }

    /// Create a g++ wrapper script for musl cross-compilation.
    ///
    /// When cross-compiling from a glibc host to a musl target, plain g++ picks up
    /// glibc C headers, producing objects with glibc-versioned symbols (e.g.
    /// `__isoc23_sscanf@@GLIBC_2.38`) incompatible with musl linking.
    ///
    /// This wrapper prepends musl's C header directory via `-isystem` so that musl's
    /// headers shadow glibc's. Unlike libc++ (which uses wrapper `<stddef.h>` etc.
    /// with `#include_next`), libstdc++ includes C headers directly from `<cstdlib>`
    /// etc., so `-isystem` shadowing works correctly without `-nostdinc`.
    ///
    /// Additionally, some glibc-specific C++ platform headers (e.g. `os_defines.h`,
    /// `libc-header-start.h`, `floatn.h`) still get picked up from gcc's built-in
    /// include paths. These headers use `__GLIBC_PREREQ()` and `__GLIBC_USE()` macros
    /// that musl doesn't define. We define these as no-op macros evaluating to 0 so
    /// glibc-guarded code paths are correctly skipped.
    #[cfg(unix)]
    fn create_musl_cxx_wrapper(target: &str) -> Option<String> {
        use std::os::unix::fs::PermissionsExt;

        let host = env::var("HOST").unwrap_or_default();

        if !target.contains("musl") || host.contains("musl") {
            return None;
        }

        let arch = target.split('-').next().unwrap_or("x86_64");
        let musl_include = format!("/usr/include/{arch}-linux-musl");
        if !Path::new(&musl_include).exists() {
            eprintln!("musl include dir not found at {musl_include}, skipping wrapper");
            return None;
        }

        let out_dir = env::var("OUT_DIR").unwrap();
        let wrapper_path = format!("{out_dir}/musl-g++.sh");
        let wrapper_content = format!(
            "#!/bin/sh\n\
             # Auto-generated musl-g++ wrapper for cross-compilation.\n\
             # Prepends musl C headers so they shadow glibc's.\n\
             # Defines glibc compat macros as 0 for musl -- handles os_defines.h,\n\
             # libc-header-start.h, floatn.h etc. that use __GLIBC_PREREQ().\n\
             # Also defines __GNUC_PREREQ for floatn.h which checks compiler version.\n\
             exec g++ -isystem \"{musl_include}\" \\\n\
               '-D__GLIBC_PREREQ(maj,min)=0' \\\n\
               '-D__GLIBC_USE(F)=0' \\\n\
               '-D__GNUC_PREREQ(maj,min)=0' \\\n\
               \"$@\"\n"
        );

        fs::write(&wrapper_path, &wrapper_content).ok()?;
        fs::set_permissions(&wrapper_path, fs::Permissions::from_mode(0o755)).ok()?;

        eprintln!("Created musl g++ wrapper at {wrapper_path} (musl headers: {musl_include})");
        Some(wrapper_path)
    }

    #[cfg(not(unix))]
    fn create_musl_cxx_wrapper(_target: &str) -> Option<String> {
        None
    }

    fn prepare_out_dir() -> PathBuf {
        let (preferred, automatic) = get_preferred_out_dir();
        let owner_reference = cargo_build_root();
        let prepared = if automatic {
            ensure_private_cache_root(&preferred, &owner_reference)
        } else {
            ensure_directory(&preferred)
        };
        match prepared {
            Ok(_) => preferred,
            Err(err) => {
                println!(
                    "cargo:warning=Failed to create cache dir {:?}: {}. Falling back to the Cargo build directory.",
                    preferred, err
                );
                let fallback = cargo_build_root().join("xberg-tesseract-cache");
                ensure_private_cache_root(&fallback, &owner_reference)
                    .expect("Failed to create fallback cache directory in Cargo build directory");
                fallback
            }
        }
    }

    fn cargo_build_root() -> PathBuf {
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));
        canonicalize_trusted_build_root(&out_dir)
            .unwrap_or_else(|error| panic!("Failed to canonicalize Cargo build root {}: {error}", out_dir.display()))
    }

    /// Find the WASI SDK installation directory.
    /// Checks `WASI_SDK_PATH` env var first, then common install locations.
    fn find_wasi_sdk() -> Result<PathBuf, String> {
        if let Ok(sdk_path) = env::var("WASI_SDK_PATH") {
            let path = PathBuf::from(sdk_path);
            if path.join("share/wasi-sysroot").exists() {
                return Ok(path);
            }
        }

        let home = env::var("HOME").unwrap_or_default();
        let common_paths = vec![
            PathBuf::from(&home).join("wasi-sdk"),
            PathBuf::from("/opt/wasi-sdk"),
            PathBuf::from("/usr/local/opt/wasi-sdk"),
        ];

        for base in &["/opt", &home] {
            if let Ok(entries) = fs::read_dir(base) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("wasi-sdk-") {
                        let path = entry.path();
                        if path.join("share/wasi-sysroot").exists() {
                            return Ok(path);
                        }
                    }
                }
            }
        }

        for path in common_paths {
            if path.join("share/wasi-sysroot").exists() {
                return Ok(path);
            }
        }

        Err(
            "WASI SDK not found. Install from https://github.com/WebAssembly/wasi-sdk/releases and set WASI_SDK_PATH"
                .to_string(),
        )
    }

    /// Find the WASI SDK CMake toolchain file.
    fn find_wasi_toolchain(wasi_sdk_dir: &Path) -> PathBuf {
        let candidate = wasi_sdk_dir.join("share/cmake/wasi-sdk.cmake");
        if candidate.exists() {
            eprintln!("Found WASI SDK toolchain: {}", candidate.display());
            return candidate;
        }
        panic!(
            "Could not find WASI SDK CMake toolchain file at: {}\nEnsure WASI SDK is properly installed.",
            candidate.display()
        );
    }

    /// Find the WASI SDK pthread CMake toolchain file (for C++ code using std::mutex/std::thread).
    #[allow(dead_code)]
    fn find_wasi_pthread_toolchain(wasi_sdk_dir: &Path) -> PathBuf {
        let candidate = wasi_sdk_dir.join("share/cmake/wasi-sdk-pthread.cmake");
        if candidate.exists() {
            println!(
                "cargo:warning=Found WASI SDK pthread toolchain: {}",
                candidate.display()
            );
            return candidate;
        }
        panic!(
            "Could not find WASI SDK pthread CMake toolchain at: {}\nEnsure WASI SDK is properly installed.",
            candidate.display()
        );
    }

    /// Find the compiler-rt builtins library in WASI SDK.
    fn find_wasi_compiler_rt(wasi_sdk_dir: &Path) -> Option<PathBuf> {
        let clang_lib = wasi_sdk_dir.join("lib/clang");
        if let Ok(entries) = fs::read_dir(&clang_lib) {
            for entry in entries.flatten() {
                let rt_dir = entry.path().join("lib/wasi");
                if rt_dir.join("libclang_rt.builtins-wasm32.a").exists() {
                    return Some(rt_dir);
                }
            }
        }
        None
    }

    pub fn build() {
        let target = target_triple();

        if is_wasm_target(&target) {
            println!(
                "cargo:warning=Detected WASM target: {}, routing to build_wasm()",
                target
            );
            return build_wasm();
        }

        let artifact_cache_dir = prepare_out_dir().join("source-artifacts");
        let build_root = cargo_build_root();
        let windows_target = is_windows_target(&target);
        let msvc_target = is_msvc_target(&target);
        let mingw_target = is_mingw_target(&target);
        let android_target = is_android_target(&target);

        eprintln!("build_root: {:?}", build_root);

        let out_dir = build_root.clone();
        let project_dir = build_root.clone();
        let third_party_dir = project_dir.join("third_party");

        let leptonica_dir = get_or_download_source(
            &artifact_cache_dir,
            &third_party_dir,
            &LEPTONICA_SOURCE,
            &leptonica_url(),
        )
        .path;
        let tesseract_dir = get_or_download_source(
            &artifact_cache_dir,
            &third_party_dir,
            &TESSERACT_SOURCE,
            &tesseract_url(),
        )
        .path;

        let (cmake_cxx_flags, cmake_c_flags, additional_defines) = get_os_specific_config();

        let leptonica_install_dir = out_dir.join("leptonica");
        let leptonica_link_name = build_static_library("leptonica", &leptonica_install_dir, || {
            let mut leptonica_config = Config::new(
                cmake_source_path(&leptonica_dir)
                    .expect("CMake source path must preserve Leptonica directory identity"),
            );

            let leptonica_src_dir = leptonica_dir.join("src");
            let environ_h_path = leptonica_src_dir.join("environ.h");

            if environ_h_path.exists() {
                let environ_h = std::fs::read_to_string(&environ_h_path)
                    .expect("Failed to read environ.h")
                    .replace("#define  HAVE_LIBZ          1", "#define  HAVE_LIBZ          0")
                    .replace("#ifdef  NO_CONSOLE_IO", "#define NO_CONSOLE_IO\n#ifdef  NO_CONSOLE_IO");
                std::fs::write(environ_h_path, environ_h).expect("Failed to write environ.h");
            }

            let makefile_static_path = leptonica_dir.join("prog").join("makefile.static");

            let leptonica_src_cmakelists = leptonica_dir.join("src").join("CMakeLists.txt");

            if leptonica_src_cmakelists.exists() {
                let cmakelists = std::fs::read_to_string(&leptonica_src_cmakelists)
                    .expect("Failed to read leptonica src CMakeLists.txt");
                let patched = cmakelists.replace(
                        "if(MINGW)\n  set_target_properties(\n    leptonica PROPERTIES SUFFIX\n                         \"-${PROJECT_VERSION}${CMAKE_SHARED_LIBRARY_SUFFIX}\")\nendif(MINGW)\n",
                        "if(MINGW AND BUILD_SHARED_LIBS)\n  set_target_properties(\n    leptonica PROPERTIES SUFFIX\n                         \"-${PROJECT_VERSION}${CMAKE_SHARED_LIBRARY_SUFFIX}\")\nendif()\n",
                    );
                if patched != cmakelists {
                    std::fs::write(&leptonica_src_cmakelists, patched)
                        .expect("Failed to patch leptonica src CMakeLists.txt");
                }
            }

            if makefile_static_path.exists() {
                let makefile_static = std::fs::read_to_string(&makefile_static_path)
                    .expect("Failed to read makefile.static")
                    .replace(
                        "ALL_LIBS =	$(LEPTLIB) -ltiff -ljpeg -lpng -lz -lm",
                        "ALL_LIBS =	$(LEPTLIB) -lm",
                    );
                std::fs::write(makefile_static_path, makefile_static).expect("Failed to write makefile.static");
            }

            if windows_target {
                if mingw_target {
                    leptonica_config.generator("Unix Makefiles");
                    leptonica_config.define("CMAKE_MAKE_PROGRAM", "mingw32-make");
                    leptonica_config.define("MSYS2_ARG_CONV_EXCL", "/MD;/MDd;/D;-D;-I;-L");
                } else if msvc_target && env::var("VSINSTALLDIR").is_ok() {
                    leptonica_config.generator("NMake Makefiles");
                }
                leptonica_config.define("CMAKE_CL_SHOWINCLUDES_PREFIX", "");
            }

            if env::var("CI").is_err() && env::var("RUSTC_WRAPPER").unwrap_or_default() == "sccache" {
                leptonica_config.env("CC", "sccache cc").env("CXX", "sccache c++");
            }

            let leptonica_install_dir_cmake = normalize_cmake_path(&leptonica_install_dir);

            leptonica_config
                .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
                .define("CMAKE_BUILD_TYPE", "Release")
                .define("BUILD_PROG", "OFF")
                .define("BUILD_SHARED_LIBS", "OFF")
                .define("ENABLE_ZLIB", "OFF")
                .define("ENABLE_PNG", "OFF")
                .define("ENABLE_JPEG", "OFF")
                .define("ENABLE_TIFF", "OFF")
                .define("ENABLE_WEBP", "OFF")
                .define("ENABLE_OPENJPEG", "OFF")
                .define("ENABLE_GIF", "OFF")
                .define("NO_CONSOLE_IO", "ON")
                .define("CMAKE_CXX_FLAGS", &cmake_cxx_flags)
                .define("CMAKE_C_FLAGS", &cmake_c_flags)
                .define("MINIMUM_SEVERITY", "L_SEVERITY_NONE")
                .define("SW_BUILD", "OFF")
                .define("HAVE_LIBZ", "0")
                .define("ENABLE_LTO", "OFF")
                .define("CMAKE_INSTALL_PREFIX", &leptonica_install_dir_cmake);

            if windows_target {
                if msvc_target {
                    leptonica_config
                        .define("CMAKE_C_FLAGS_RELEASE", "/MD /O2")
                        .define("CMAKE_C_FLAGS_DEBUG", "/MDd /Od");
                } else if mingw_target {
                    leptonica_config
                        .define("CMAKE_C_FLAGS_RELEASE", "-O2 -DNDEBUG")
                        .define("CMAKE_C_FLAGS_DEBUG", "-O0 -g");
                } else {
                    leptonica_config
                        .define("CMAKE_C_FLAGS_RELEASE", "-O2")
                        .define("CMAKE_C_FLAGS_DEBUG", "-O0 -g");
                }
            }

            for (key, value) in &additional_defines {
                leptonica_config.define(key, value);
            }

            leptonica_config.build();
        });

        let leptonica_include_dir = leptonica_install_dir.join("include");
        let leptonica_lib_dir = leptonica_install_dir.join("lib");
        let tesseract_install_dir = out_dir.join("tesseract");
        let tessdata_prefix = project_dir.clone();

        let leptonica_install_dir_cmake = normalize_cmake_path(&leptonica_install_dir);
        let leptonica_cmake_dir = leptonica_install_dir.join("lib/cmake/leptonica");
        let leptonica_cmake_dir_cmake = normalize_cmake_path(&leptonica_cmake_dir);
        let leptonica_include_dir_cmake = normalize_cmake_path(&leptonica_include_dir);
        let leptonica_lib_dir_cmake = normalize_cmake_path(&leptonica_lib_dir);
        let tesseract_install_dir_cmake = normalize_cmake_path(&tesseract_install_dir);
        let tessdata_prefix_cmake = normalize_cmake_path(&tessdata_prefix);

        let tesseract_link_name = build_static_library("tesseract", &tesseract_install_dir, || {
            let cmakelists_path = tesseract_dir.join("CMakeLists.txt");
            let cmakelists = std::fs::read_to_string(&cmakelists_path)
                .expect("Failed to read CMakeLists.txt")
                .replace("set(HAVE_TIFFIO_H ON)", "")
                .replace(
                    "add_executable(tesseract src/tesseract.cpp)\n\
                         target_link_libraries(tesseract libtesseract)\n\
                         if(HAVE_TIFFIO_H AND WIN32)\n\
                         \x20 target_link_libraries(tesseract ${TIFF_LIBRARIES})\n\
                         endif()\n\
                         \n\
                         if(OPENMP_BUILD AND UNIX)\n\
                         \x20 target_link_libraries(tesseract pthread)\n\
                         endif()",
                    "",
                )
                .replace("install(TARGETS tesseract DESTINATION bin)", "")
                .replace(
                    "if (MSVC)\n\
                         \x20 install(FILES $<TARGET_PDB_FILE:${PROJECT_NAME}> DESTINATION bin OPTIONAL)\n\
                         endif()",
                    "",
                );

            let cmakelists = if android_target {
                cmakelists.replace(
                    "if(ANDROID)\n\
                         \x20 add_definitions(-DANDROID)\n\
                         \x20 find_package(CpuFeaturesNdkCompat REQUIRED)\n\
                         \x20 target_include_directories(\n\
                         \x20\x20\x20 libtesseract\n\
                         \x20\x20\x20 PRIVATE \"${CpuFeaturesNdkCompat_DIR}/../../../include/ndk_compat\")\n\
                         \x20 target_link_libraries(libtesseract PRIVATE CpuFeatures::ndk_compat)\n\
                         endif()",
                    "if(ANDROID)\n\
                         \x20 add_definitions(-DANDROID)\n\
                         endif()",
                )
            } else {
                cmakelists
            };

            // ~keep `check_leptonica_tiff_support()` compiles AND RUNS a probe against the
            // leptonica just built, purely to decide whether to set `DISABLE_TIFF` -- which this
            // build already passes as `ON` unconditionally, because leptonica is configured
            // `ENABLE_TIFF=OFF` right above. So the probe cannot change the outcome; it can only
            // fail. On a multi-config generator its `try_run` cannot resolve the imported
            // leptonica target ("IMPORTED_LOCATION not set for imported target \"leptonica\"
            // configuration Debug/Release/MinSizeRel/RelWithDebInfo") and hard-fails the whole
            // configure. That is what took out every Windows CI leg: the runner image stopped
            // exposing VSINSTALLDIR, the `msvc_target && VSINSTALLDIR` guard above stopped
            // selecting single-config `NMake Makefiles`, and cmake fell back to the Visual Studio
            // generator. Gate the probe on the answer not already being known.
            let tiff_probe_call = "\ncheck_leptonica_tiff_support()\n";
            assert!(
                cmakelists.contains(tiff_probe_call),
                "tesseract CMakeLists.txt no longer contains the check_leptonica_tiff_support() call this build patches"
            );
            let cmakelists = cmakelists.replace(
                tiff_probe_call,
                "\nif(NOT DISABLE_TIFF)\n  check_leptonica_tiff_support()\nendif()\n",
            );

            std::fs::write(&cmakelists_path, cmakelists).expect("Failed to write CMakeLists.txt");

            let mut tesseract_config = Config::new(
                cmake_source_path(&tesseract_dir)
                    .expect("CMake source path must preserve Tesseract directory identity"),
            );
            if windows_target {
                if mingw_target {
                    tesseract_config.generator("Unix Makefiles");
                    tesseract_config.define("CMAKE_MAKE_PROGRAM", "mingw32-make");
                    tesseract_config.define("MSYS2_ARG_CONV_EXCL", "/MD;/MDd;/D;-D;-I;-L");
                } else if msvc_target && env::var("VSINSTALLDIR").is_ok() {
                    tesseract_config.generator("NMake Makefiles");
                }
                tesseract_config.define("CMAKE_CL_SHOWINCLUDES_PREFIX", "");
            }

            if env::var("CI").is_err() && env::var("RUSTC_WRAPPER").unwrap_or_default() == "sccache" {
                tesseract_config.env("CC", "sccache cc").env("CXX", "sccache c++");
            }
            tesseract_config
                .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
                .define("CMAKE_BUILD_TYPE", "Release")
                .define("BUILD_TRAINING_TOOLS", "OFF")
                .define("BUILD_SHARED_LIBS", "OFF")
                .define("DISABLE_ARCHIVE", "ON")
                .define("DISABLE_CURL", "ON")
                .define("DISABLE_OPENCL", "ON")
                .define("Leptonica_DIR", &leptonica_cmake_dir_cmake)
                .define("LEPTONICA_INCLUDE_DIR", &leptonica_include_dir_cmake)
                .define("LEPTONICA_LIBRARY", &leptonica_lib_dir_cmake)
                .define("CMAKE_PREFIX_PATH", &leptonica_install_dir_cmake)
                .define("CMAKE_INSTALL_PREFIX", &tesseract_install_dir_cmake)
                .define("TESSDATA_PREFIX", &tessdata_prefix_cmake)
                .define("DISABLE_TIFF", "ON")
                .define("DISABLE_PNG", "ON")
                .define("DISABLE_JPEG", "ON")
                .define("DISABLE_WEBP", "ON")
                .define("DISABLE_OPENJPEG", "ON")
                .define("DISABLE_ZLIB", "ON")
                .define("DISABLE_LIBXML2", "ON")
                .define("DISABLE_LIBICU", "ON")
                .define("DISABLE_LZMA", "ON")
                .define("DISABLE_GIF", "ON")
                .define("DISABLE_DEBUG_MESSAGES", "ON")
                .define("debug_file", "/dev/null")
                .define("HAVE_LIBARCHIVE", "OFF")
                .define("HAVE_LIBCURL", "OFF")
                .define("HAVE_TIFFIO_H", "OFF")
                .define("GRAPHICS_DISABLED", "ON")
                .define("DISABLED_LEGACY_ENGINE", "OFF")
                .define("USE_OPENCL", "OFF")
                .define("OPENMP_BUILD", "OFF")
                .define("BUILD_TESTS", "OFF")
                .define("ENABLE_LTO", "OFF")
                .define("BUILD_PROG", "OFF")
                .define("BUILD_TESSERACT_BINARY", "OFF")
                .define("SW_BUILD", "OFF")
                .define("LEPT_TIFF_RESULT", "FALSE")
                .define("INSTALL_CONFIGS", "ON")
                .define("USE_SYSTEM_ICU", "ON")
                .define("CMAKE_CXX_FLAGS", &cmake_cxx_flags)
                .define("CMAKE_C_FLAGS", &cmake_c_flags);

            if is_zigbuild() {
                tesseract_config.define("HAVE_AVX512F", "OFF");
            }

            for (key, value) in &additional_defines {
                tesseract_config.define(key, value);
            }

            tesseract_config.build();
        });

        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
        let eng_traineddata = PathBuf::from(&out_dir).join("eng.traineddata");
        prepare_eng_traineddata(&artifact_cache_dir, &eng_traineddata);
        eprintln!("Bundled eng.traineddata: {:?}", eng_traineddata);

        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed=src/shim.cpp");

        #[cfg(feature = "build-tesseract")]
        cc::Build::new()
            .file("src/shim.cpp")
            .cpp(true)
            .std("c++17")
            .include(tool_argument_path(&tesseract_install_dir.join("include")))
            .compile("xberg_shim");

        println!(
            "cargo:rustc-link-search=native={}",
            tool_argument_path(&leptonica_lib_dir).display()
        );
        println!(
            "cargo:rustc-link-search=native={}",
            tool_argument_path(&tesseract_install_dir.join("lib")).display()
        );

        #[cfg(feature = "dynamic-linking")]
        let link_type = "dylib";
        #[cfg(not(feature = "dynamic-linking"))]
        let link_type = "static";

        println!("cargo:rustc-link-lib={}={}", link_type, tesseract_link_name);
        println!(
            "cargo:warning=Linking with tesseract ({} linking): {}",
            link_type, tesseract_link_name
        );
        println!("cargo:rustc-link-lib={}={}", link_type, leptonica_link_name);
        println!(
            "cargo:warning=Linking with leptonica ({} linking): {}",
            link_type, leptonica_link_name
        );

        set_os_specific_link_flags();

        eprintln!("Leptonica include dir: {:?}", leptonica_include_dir);
        eprintln!("Leptonica lib dir: {:?}", leptonica_lib_dir);
        eprintln!("Tesseract install dir: {:?}", tesseract_install_dir);
        eprintln!("Tessdata dir: {:?}", tessdata_prefix);
    }

    fn get_os_specific_config() -> (String, String, Vec<(String, String)>) {
        let mut cmake_cxx_flags = String::new();
        let mut cmake_c_flags = String::new();
        let mut additional_defines = Vec::new();
        let target = target_triple();
        let target_macos = is_macos_target(&target);
        let target_linux = is_linux_target(&target);
        let target_windows = is_windows_target(&target);
        let target_msvc = is_msvc_target(&target);
        let target_mingw = is_mingw_target(&target);
        let target_musl = target.contains("musl");

        if target_macos {
            cmake_cxx_flags.push_str("-stdlib=libc++ ");
            cmake_cxx_flags.push_str("-std=c++17 ");
            cmake_cxx_flags.push_str("-fno-exceptions ");
        } else if is_android_target(&target) {
            cmake_c_flags.push_str("-std=gnu11 ");
            cmake_cxx_flags.push_str("-std=c++17 ");
            cmake_cxx_flags.push_str("-fno-exceptions ");

            let abi = android_abi(&target);
            let api: u32 = 21;
            additional_defines.push(("ANDROID_ABI".to_string(), abi.to_string()));
            additional_defines.push(("ANDROID_PLATFORM".to_string(), format!("android-{api}")));

            let ndk_home = env::var("ANDROID_NDK_HOME")
                .or_else(|_| env::var("ANDROID_NDK"))
                .or_else(|_| env::var("NDK_HOME"))
                .ok();
            if let Some(ref ndk) = ndk_home {
                additional_defines.push(("CMAKE_ANDROID_NDK".to_string(), ndk.clone()));
                let cxx = ndk_clangxx(ndk, abi, api).unwrap_or_else(|| resolve_cxx_compiler(&target, "clang++"));
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx));
            } else {
                let cxx = resolve_cxx_compiler(&target, "clang++");
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx));
            }

            additional_defines.push(("CMAKE_FIND_ROOT_PATH_MODE_INCLUDE".to_string(), "ONLY".to_string()));
            additional_defines.push(("CMAKE_FIND_ROOT_PATH_MODE_LIBRARY".to_string(), "ONLY".to_string()));
            additional_defines.push(("CMAKE_FIND_ROOT_PATH_MODE_PROGRAM".to_string(), "NEVER".to_string()));
            additional_defines.push((
                "CMAKE_IGNORE_PATH".to_string(),
                "/opt/homebrew/Cellar;/opt/homebrew/include;/opt/homebrew/lib;/usr/local/include;/usr/local/lib"
                    .to_string(),
            ));
        } else if target_linux {
            cmake_c_flags.push_str("-std=gnu11 ");
            cmake_cxx_flags.push_str("-std=gnu++17 ");
            cmake_cxx_flags.push_str("-fno-exceptions ");
            if target_musl {
                let cxx_compiler =
                    create_musl_cxx_wrapper(&target).unwrap_or_else(|| resolve_cxx_compiler(&target, "g++"));
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx_compiler));
            } else if env::var("CC").map(|cc| cc.contains("clang")).unwrap_or(false) {
                cmake_cxx_flags.push_str("-stdlib=libc++ ");
                let cxx_compiler = resolve_cxx_compiler(&target, "clang++");
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx_compiler));
            } else {
                let cxx_compiler = resolve_cxx_compiler(&target, "g++");
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx_compiler));
            }
        } else if target_windows {
            if target_msvc {
                cmake_cxx_flags.push_str("/MP /std:c++17 /DTESSERACT_STATIC ");
                additional_defines.push(("CMAKE_C_FLAGS_RELEASE".to_string(), "/MD /O2".to_string()));
                additional_defines.push(("CMAKE_C_FLAGS_DEBUG".to_string(), "/MDd /Od".to_string()));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_RELEASE".to_string(),
                    "/MD /O2 /DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_DEBUG".to_string(),
                    "/MDd /Od /DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push(("CMAKE_MSVC_RUNTIME_LIBRARY".to_string(), "MultiThreadedDLL".to_string()));
            } else if target_mingw {
                cmake_cxx_flags.push_str("-std=c++17 -DTESSERACT_STATIC -fno-exceptions ");
                additional_defines.push(("CMAKE_C_FLAGS_RELEASE".to_string(), "-O2 -DNDEBUG".to_string()));
                additional_defines.push(("CMAKE_C_FLAGS_DEBUG".to_string(), "-O0 -g".to_string()));
                let gcc_path = resolve_mingw_compiler("gcc");
                let gxx_path = resolve_mingw_compiler("g++");
                additional_defines.push(("CMAKE_C_COMPILER".to_string(), gcc_path));
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), gxx_path));
                additional_defines.push(("CMAKE_SYSTEM_NAME".to_string(), "Windows".to_string()));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_RELEASE".to_string(),
                    "-O2 -DNDEBUG -DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_DEBUG".to_string(),
                    "-O0 -g -DTESSERACT_STATIC".to_string(),
                ));
            } else {
                cmake_cxx_flags.push_str("-std=c++17 -DTESSERACT_STATIC ");
                additional_defines.push(("CMAKE_C_FLAGS_RELEASE".to_string(), "-O2 -DNDEBUG".to_string()));
                additional_defines.push(("CMAKE_C_FLAGS_DEBUG".to_string(), "-O0 -g".to_string()));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_RELEASE".to_string(),
                    "-O2 -DNDEBUG -DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_DEBUG".to_string(),
                    "-O0 -g -DTESSERACT_STATIC".to_string(),
                ));
            }
        }

        cmake_cxx_flags.push_str("-DUSE_STD_NAMESPACE ");
        additional_defines.push(("CMAKE_POSITION_INDEPENDENT_CODE".to_string(), "ON".to_string()));

        if target_windows && target_msvc {
            cmake_cxx_flags.push_str("/permissive- ");
            additional_defines.push(("CMAKE_EXE_LINKER_FLAGS".to_string(), "/INCREMENTAL:NO".to_string()));
            additional_defines.push(("CMAKE_SHARED_LINKER_FLAGS".to_string(), "/INCREMENTAL:NO".to_string()));
            additional_defines.push(("CMAKE_MODULE_LINKER_FLAGS".to_string(), "/INCREMENTAL:NO".to_string()));
        }

        (cmake_cxx_flags, cmake_c_flags, additional_defines)
    }

    fn set_os_specific_link_flags() {
        let target = target_triple();
        let target_macos = is_macos_target(&target);
        let target_linux = is_linux_target(&target);
        let target_windows = is_windows_target(&target);
        let target_mingw = is_mingw_target(&target);
        let target_musl = target.contains("musl");

        if target_macos {
            println!("cargo:rustc-link-lib=c++");
        } else if is_android_target(&target) {
            println!("cargo:rustc-link-lib=c++_static");
            println!("cargo:rustc-link-lib=log");
        } else if target_linux {
            if target_musl {
                if let Ok(output) = std::process::Command::new("gcc")
                    .arg("--print-file-name=libstdc++.a")
                    .output()
                {
                    let path = String::from_utf8_lossy(&output.stdout);
                    if let Some(parent) = std::path::Path::new(path.trim()).parent() {
                        println!("cargo:rustc-link-search=native={}", parent.display());
                    }
                }
                println!("cargo:rustc-link-lib=static=stdc++");
            } else if env::var("CC").map(|cc| cc.contains("clang")).unwrap_or(false) {
                println!("cargo:rustc-link-lib=c++");
            } else {
                println!("cargo:rustc-link-lib=stdc++");
                if !is_zigbuild() {
                    println!("cargo:rustc-link-lib=stdc++fs");
                }
            }
            println!("cargo:rustc-link-lib=pthread");
            println!("cargo:rustc-link-lib=m");
            if !target_musl {
                println!("cargo:rustc-link-lib=dl");
            }
        } else if target_windows {
            if target_mingw {
                println!("cargo:rustc-link-lib=stdc++");
            }
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=ws2_32");
            println!("cargo:rustc-link-lib=advapi32");
            println!("cargo:rustc-link-lib=shell32");
        }

        println!("cargo:rustc-link-search=native={}", env::var("OUT_DIR").unwrap());
    }

    fn download_file_with_fallback(urls: &[&str], label: &str, expected_size: u64) -> io::Result<Vec<u8>> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .http1_only()
            .build()
            .map_err(io::Error::other)?;

        let max_attempts: u32 = 5;
        let mut last_err = String::new();

        for url in urls {
            eprintln!("Downloading {} from {}", label, url);

            for attempt in 1..=max_attempts {
                let err_msg = match client.get(*url).send() {
                    Ok(mut resp) => {
                        if resp.status().is_success() {
                            if let Some(content_length) = resp.content_length()
                                && content_length != expected_size
                            {
                                format!("unexpected Content-Length {content_length}, expected {expected_size}")
                            } else {
                                match read_exact_size(&mut resp, expected_size, label) {
                                    Ok(bytes) => {
                                        eprintln!("Downloaded {label} ({expected_size} bytes)");
                                        return Ok(bytes);
                                    }
                                    Err(error) => error.to_string(),
                                }
                            }
                        } else {
                            format!("HTTP {}", resp.status().as_u16())
                        }
                    }
                    Err(err) => err.to_string(),
                };

                last_err = err_msg.clone();

                if attempt == max_attempts {
                    println!(
                        "cargo:warning=All {} attempts for {} exhausted on URL {}",
                        max_attempts, label, url
                    );
                    break;
                }

                let backoff = 2u64.pow((attempt - 1).min(4));
                println!(
                    "cargo:warning=Download attempt {}/{} for {} failed ({}). Retrying in {}s...",
                    attempt, max_attempts, label, err_msg, backoff
                );
                std::thread::sleep(std::time::Duration::from_secs(backoff));
            }
        }

        Err(io::Error::other(format!(
            "failed to download {label} after trying {} URL(s): {last_err}",
            urls.len()
        )))
    }

    /// Render a path for a CMake `-D<variable>=<value>` argument.
    ///
    /// On Windows the build root is canonicalized, which yields an extended-length `\\?\` path.
    /// CMake cannot consume that form, so the verbatim prefix is stripped before the separators
    /// are converted. See `source_cache::windows_cmake_argument_path`.
    #[cfg(windows)]
    fn normalize_cmake_path(path: &Path) -> String {
        crate::source_cache::windows_cmake_argument_path(&path.to_string_lossy())
    }

    #[cfg(not(windows))]
    fn normalize_cmake_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    /// Apply the WASM patch to Tesseract source. Uses `git apply` if available, falls back to manual application.
    fn apply_tesseract_wasm_patch(tesseract_dir: &Path) {
        let patch_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("patches/tesseract.diff");
        if !patch_file.exists() {
            println!(
                "cargo:warning=Tesseract WASM patch not found at {:?}, skipping",
                patch_file
            );
            return;
        }

        eprintln!("Applying tesseract WASM patch from {:?}", patch_file);

        let dir_str = normalize_cmake_path(tesseract_dir);
        let patch_str = normalize_cmake_path(&patch_file);

        let result = std::process::Command::new("git")
            .args(["apply", "--ignore-whitespace", "--directory"])
            .arg(&dir_str)
            .arg(&patch_str)
            .output();

        let patch_applied = match result {
            Ok(output) if output.status.success() => {
                eprintln!("Successfully applied tesseract WASM patch via git apply");
                true
            }
            _ => {
                eprintln!("git apply failed, trying patch command...");
                let result = std::process::Command::new("patch")
                    .args(["--force", "-p1", "-d"])
                    .arg(&dir_str)
                    .arg("-i")
                    .arg(&patch_str)
                    .output();

                match result {
                    Ok(output) if output.status.success() => {
                        eprintln!("Successfully applied tesseract WASM patch via patch command");
                        true
                    }
                    Ok(output) => {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        println!(
                            "cargo:warning=Patch command failed, will apply programmatic fixups.\
                             \nstderr: {}\nstdout: {}",
                            stderr, stdout
                        );
                        false
                    }
                    Err(e) => {
                        println!(
                            "cargo:warning=patch command not available ({}), will apply programmatic fixups",
                            e
                        );
                        false
                    }
                }
            }
        };

        if !patch_applied {
            apply_wasm_source_fixups(tesseract_dir);
        }

        let source_lists = tesseract_dir.join("cmake/SourceLists.cmake");
        if source_lists.exists() {
            eprintln!("Patching cmake/SourceLists.cmake for WASM compatibility");
            let content = fs::read_to_string(&source_lists).expect("Failed to read cmake/SourceLists.cmake");

            let mut patched = content;

            patched = patched.replace("    ${TESSERACT_SRC_VIEWER}\n", "");

            if let Some(start) = patched.find("set(TESSERACT_SRC_API\n")
                && let Some(end) = patched[start..].find(")\n")
            {
                // `capi.cpp` is the actual C-style API surface (`TessBaseAPICreate`,
                // `TessBaseAPIInit5`, `TessBaseAPISetImage`, `TessMonitorCreate`, etc.)
                // that `xberg-tesseract`'s Rust FFI binds to directly. Dropping it here
                // previously left every one of those symbols unresolved in the linked
                // wasm module; wasm-bindgen's import handling silently resolved them to
                // no-op stubs returning 0/null instead of failing the link, so
                // null pointer at runtime. `renderer.cpp`/`altorenderer.cpp`/
                // `lstmboxrenderer.cpp`/`pdfrenderer.cpp`/`wordstrboxrenderer.cpp` are
                // pulled in too because `capi.cpp`'s renderer-constructor functions
                // (`TessPDFRendererCreate`, etc.) reference those concrete renderer
                // classes even though the Rust binding never calls them — the linker
                // still needs them defined once `capi.cpp` is part of the build.
                let replacement = "set(TESSERACT_SRC_API\n    src/api/baseapi.cpp\n    src/api/capi.cpp\n    src/api/renderer.cpp\n    src/api/altorenderer.cpp\n    src/api/hocrrenderer.cpp\n    src/api/lstmboxrenderer.cpp\n    src/api/pdfrenderer.cpp\n    src/api/wordstrboxrenderer.cpp\n)\n";
                patched = format!("{}{}{}", &patched[..start], replacement, &patched[start + end + 2..]);
            }

            fs::write(&source_lists, patched).expect("Failed to write patched cmake/SourceLists.cmake");
            eprintln!("Successfully patched cmake/SourceLists.cmake");
        }

        let cmakelists = tesseract_dir.join("CMakeLists.txt");
        if cmakelists.exists() {
            let content = fs::read_to_string(&cmakelists).expect("Failed to read CMakeLists.txt");
            let mut patched = content;

            patched = patched.replace(
                "add_executable(tesseract src/tesseract.cpp)",
                "# WASM: disabled tesseract binary\n# add_executable(tesseract src/tesseract.cpp)",
            );
            patched = patched.replace(
                "target_link_libraries(tesseract libtesseract)",
                "# target_link_libraries(tesseract libtesseract)",
            );
            patched = patched.replace(
                "target_link_libraries(tesseract pthread)",
                "# target_link_libraries(tesseract pthread)",
            );
            patched = patched.replace(
                "install(TARGETS tesseract DESTINATION bin)",
                "# install(TARGETS tesseract DESTINATION bin)",
            );
            patched = patched.replace(
                "if (MSVC)\n\
                 \x20 install(FILES $<TARGET_PDB_FILE:${PROJECT_NAME}> DESTINATION bin OPTIONAL)\n\
                 endif()",
                "# WASM: disabled MSVC PDB install\n\
                 # if (MSVC)\n\
                 #   install(FILES $<TARGET_PDB_FILE:${PROJECT_NAME}> DESTINATION bin OPTIONAL)\n\
                 # endif()",
            );

            fs::write(&cmakelists, patched).expect("Failed to write patched CMakeLists.txt");
            eprintln!("Disabled tesseract binary build in CMakeLists.txt");
        }
    }

    /// Apply C++ source fixups programmatically when the diff patch fails.
    /// These are the same changes from patches/tesseract.diff applied via string replacement.
    /// All replacements are idempotent (no-op if already applied).
    fn apply_wasm_source_fixups(tesseract_dir: &Path) {
        eprintln!("Applying programmatic C++ source fixups for WASM");

        let simddetect = tesseract_dir.join("src/arch/simddetect.cpp");
        if simddetect.exists() {
            let content = fs::read_to_string(&simddetect).expect("Failed to read simddetect.cpp");
            if !content.contains("#if !defined(__wasm__)") {
                let patched = content.replace(
                    "#if defined(HAVE_AVX) || defined(HAVE_AVX2) || defined(HAVE_FMA) || defined(HAVE_SSE4_1)\n\
                     // See https://en.wikipedia.org/wiki/CPUID.\n\
                     #  define HAS_CPUID\n\
                     #endif",
                    "#if !defined(__wasm__)\n\
                     #if defined(HAVE_AVX) || defined(HAVE_AVX2) || defined(HAVE_FMA) || defined(HAVE_SSE4_1)\n\
                     // See https://en.wikipedia.org/wiki/CPUID.\n\
                     #  define HAS_CPUID\n\
                     #endif\n\
                     #endif",
                );
                fs::write(&simddetect, patched).expect("Failed to write simddetect.cpp");
                eprintln!("Patched simddetect.cpp: added __wasm__ guard for CPUID");
            }
        }

        let pageiter = tesseract_dir.join("src/ccmain/pageiterator.cpp");
        if pageiter.exists() {
            let content = fs::read_to_string(&pageiter).expect("Failed to read pageiterator.cpp");
            if content.contains("if (up_in_image.y() > 0.0F) {") && !content.contains("if (up_in_image.y() >= 0.0F) {")
            {
                let patched = content.replace("if (up_in_image.y() > 0.0F) {", "if (up_in_image.y() >= 0.0F) {");
                fs::write(&pageiter, patched).expect("Failed to write pageiterator.cpp");
                eprintln!("Patched pageiterator.cpp: fixed orientation null vector check");
            }
        }

        let tessclass_h = tesseract_dir.join("src/ccmain/tesseractclass.h");
        if tessclass_h.exists() {
            let content = fs::read_to_string(&tessclass_h).expect("Failed to read tesseractclass.h");
            if content.contains("DebugPixa pixa_debug_;") {
                let patched = content.replace("DebugPixa pixa_debug_;", "std::unique_ptr<DebugPixa> pixa_debug_;");
                fs::write(&tessclass_h, patched).expect("Failed to write tesseractclass.h");
                eprintln!("Patched tesseractclass.h: pixa_debug_ -> unique_ptr");
            }
        }

        let tessclass_cpp = tesseract_dir.join("src/ccmain/tesseractclass.cpp");
        if tessclass_cpp.exists() {
            let content = fs::read_to_string(&tessclass_cpp).expect("Failed to read tesseractclass.cpp");
            if content.contains("pixa_debug_.WritePDF") {
                let mut patched = content;
                patched = patched.replace(
                    "  std::string debug_name = imagebasename + \"_debug.pdf\";\n  pixa_debug_.WritePDF(debug_name.c_str());",
                    "  if (pixa_debug_) {\n    std::string debug_name = imagebasename + \"_debug.pdf\";\n    pixa_debug_->WritePDF(debug_name.c_str());\n  }",
                );
                patched = patched.replace("&pixa_debug_)", "pixa_debug_.get())");
                fs::write(&tessclass_cpp, patched).expect("Failed to write tesseractclass.cpp");
                eprintln!("Patched tesseractclass.cpp: updated pixa_debug_ for unique_ptr");
            }
        }

        let pageseg = tesseract_dir.join("src/ccmain/pagesegmain.cpp");
        if pageseg.exists() {
            let content = fs::read_to_string(&pageseg).expect("Failed to read pagesegmain.cpp");
            if content.contains("pixa_debug_.AddPix") || content.contains("&pixa_debug_") {
                let mut patched = content;
                patched = patched.replace("pixa_debug_.AddPix(", "pixa_debug_->AddPix(");
                patched = patched.replace(
                    "if (tessedit_dump_pageseg_images) {\n    pixa_debug_->AddPix(",
                    "if (tessedit_dump_pageseg_images && pixa_debug_) {\n    pixa_debug_->AddPix(",
                );
                patched = patched.replace("&pixa_debug_", "pixa_debug_.get()");
                fs::write(&pageseg, patched).expect("Failed to write pagesegmain.cpp");
                eprintln!("Patched pagesegmain.cpp: updated pixa_debug_ for unique_ptr");
            }
        }

        let cmakelists = tesseract_dir.join("CMakeLists.txt");
        if cmakelists.exists() {
            let content = fs::read_to_string(&cmakelists).expect("Failed to read CMakeLists.txt");
            let mut patched = content;
            patched = patched.replace("  src/opencl/*.cpp\n", "");
            patched = patched.replace("  src/viewer/*.cpp\n", "");
            // NOTE: src/api/{capi,renderer,altorenderer,hocrrenderer,lstmboxrenderer,
            // pdfrenderer,wordstrboxrenderer}.cpp are intentionally NOT stripped here.
            // `capi.cpp` is the C-style API surface (`TessBaseAPICreate`, `TessBaseAPIInit5`,
            // dropping it left those symbols unresolved, silently stubbed to 0/null by
            // wasm-bindgen's import handling instead of failing the link, so every
            // WASM Tesseract call returned a null pointer at runtime. The renderer files
            // are kept because capi.cpp's renderer-constructor functions reference those
            // concrete classes even though the Rust binding never calls them.
            fs::write(&cmakelists, &patched).expect("Failed to write CMakeLists.txt");
            eprintln!("Patched CMakeLists.txt: removed unnecessary sources for WASM");
        }

        eprintln!("Programmatic C++ source fixups complete");
    }

    /// Install a no-op mutex header for WASM builds.
    ///
    /// The wasm32-wasi-threads libc++ provides std::mutex that uses memory.atomic.wait32
    /// instructions. These deadlock in single-threaded WASM (no SharedArrayBuffer).
    /// This function writes a header that replaces std::mutex with a no-op stub when
    /// TESSERACT_WASM_NOOP_MUTEX is defined, and patches Tesseract source files to use it.
    /// Patch Tesseract source for single-threaded WASM builds.
    ///
    /// The non-threaded wasm32-wasi sysroot doesn't provide `<mutex>` or `<thread>`.
    /// This function:
    /// 1. Writes a no-op header providing stub mutex, lock_guard, thread, and this_thread types
    /// 2. Patches Tesseract source files to use the stubs instead of std:: types
    fn apply_wasm_noop_mutex_patch(tesseract_dir: &Path) {
        let noop_header = tesseract_dir.join("src/wasm_noop_mutex.h");
        let header_content = r#"// No-op threading primitives for single-threaded WASM builds.
// Replaces std::mutex, std::lock_guard, std::thread, std::this_thread
// to avoid dependency on <mutex>/<thread> which are unavailable in
// the non-threaded wasm32-wasi sysroot.
#ifndef TESSERACT_WASM_NOOP_MUTEX_H_
#define TESSERACT_WASM_NOOP_MUTEX_H_

#ifdef TESSERACT_WASM_NOOP_MUTEX

namespace wasm_noop {

struct mutex {
    void lock() {}
    void unlock() {}
    bool try_lock() { return true; }
};

template <typename M>
struct lock_guard {
    explicit lock_guard(M&) {}
    ~lock_guard() = default;
    lock_guard(const lock_guard&) = delete;
    lock_guard& operator=(const lock_guard&) = delete;
};

// No-op thread: single-threaded WASM never spawns threads.
// The callable is invoked synchronously in the constructor.
struct thread {
    thread() = default;
    template <typename F, typename... Args>
    explicit thread(F&& f, Args&&... args) {
        // Execute synchronously — no real thread in WASM.
        f(static_cast<Args&&>(args)...);
    }
    bool joinable() const { return false; }
    void join() {}
    void detach() {}
};

namespace this_thread {
    inline void yield() {}
}  // namespace this_thread

}  // namespace wasm_noop

#define TESSERACT_MUTEX_TYPE wasm_noop::mutex
#define TESSERACT_LOCK_GUARD wasm_noop::lock_guard
#define TESSERACT_THREAD_TYPE wasm_noop::thread
#define TESSERACT_THIS_THREAD wasm_noop::this_thread

#else

#include <mutex>
#include <thread>
#define TESSERACT_MUTEX_TYPE std::mutex
#define TESSERACT_LOCK_GUARD std::lock_guard
#define TESSERACT_THREAD_TYPE std::thread
#define TESSERACT_THIS_THREAD std::this_thread

#endif  // TESSERACT_WASM_NOOP_MUTEX
#endif  // TESSERACT_WASM_NOOP_MUTEX_H_
"#;
        fs::write(&noop_header, header_content).expect("Failed to write wasm_noop_mutex.h");
        eprintln!("Wrote wasm_noop_mutex.h for WASM no-op threading stubs");

        let files_to_patch = [
            "src/lstm/networkscratch.h",
            "src/ccstruct/imagedata.h",
            "src/ccstruct/imagedata.cpp",
            "src/ccutil/object_cache.h",
            "src/classify/intfx.cpp",
        ];

        for rel_path in &files_to_patch {
            let file_path = tesseract_dir.join(rel_path);
            if !file_path.exists() {
                eprintln!("Skipping {}: file not found", rel_path);
                continue;
            }

            let content = fs::read_to_string(&file_path).unwrap_or_default();
            let patched = content
                .replace("#include <mutex>", "#include \"wasm_noop_mutex.h\"")
                .replace("#include <thread>", "#include \"wasm_noop_mutex.h\"")
                .replace("std::mutex", "TESSERACT_MUTEX_TYPE")
                .replace(
                    "std::lock_guard<TESSERACT_MUTEX_TYPE>",
                    "TESSERACT_LOCK_GUARD<TESSERACT_MUTEX_TYPE>",
                )
                .replace("std::thread", "TESSERACT_THREAD_TYPE")
                .replace("std::this_thread", "TESSERACT_THIS_THREAD")
                .replace("TESSERACT_THIS_THREAD_TYPE", "TESSERACT_THIS_THREAD");

            if patched != content {
                fs::write(&file_path, patched).unwrap_or_else(|_| panic!("Failed to patch {}", rel_path));
                eprintln!("Patched {} for WASM no-op threading", rel_path);
            }
        }
    }

    fn build_leptonica_wasm(leptonica_src: &Path, leptonica_install: &Path, wasi_sdk_dir: &Path) {
        let toolchain_file = find_wasi_toolchain(wasi_sdk_dir);
        let sysroot = wasi_sdk_dir.join("share/wasi-sysroot");
        let clang = wasi_sdk_dir.join("bin/clang");

        let mut config = Config::new(
            cmake_source_path(leptonica_src).expect("CMake source path must preserve Leptonica directory identity"),
        );

        config.target("wasm32-wasi");
        if cfg!(target_os = "windows") {
            config.generator("Ninja");
        }
        config.define("CMAKE_TOOLCHAIN_FILE", normalize_cmake_path(&toolchain_file));
        config.define("CMAKE_SYSROOT", normalize_cmake_path(&sysroot));
        config.define("CMAKE_C_COMPILER", normalize_cmake_path(&clang));

        config
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .define("CMAKE_TRY_COMPILE_TARGET_TYPE", "STATIC_LIBRARY")
            .define("LIBWEBP_SUPPORT", "OFF")
            .define("OPENJPEG_SUPPORT", "OFF")
            .define("ENABLE_ZLIB", "OFF")
            .define("ENABLE_PNG", "OFF")
            .define("ENABLE_JPEG", "OFF")
            .define("ENABLE_TIFF", "OFF")
            .define("ENABLE_WEBP", "OFF")
            .define("ENABLE_OPENJPEG", "OFF")
            .define("ENABLE_GIF", "OFF")
            .define("BUILD_PROG", "OFF")
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("NO_CONSOLE_IO", "ON")
            .define("HAVE_LIBZ", "0")
            .define("ENABLE_LTO", "OFF")
            .define("CMAKE_C_FLAGS", "-fPIC -Os -fno-lto -fno-exceptions -D_WASI_EMULATED_PROCESS_CLOCKS -D_WASI_EMULATED_SIGNAL -Wno-implicit-function-declaration")
            .define("CMAKE_INSTALL_PREFIX", normalize_cmake_path(leptonica_install));

        config.build();
    }

    fn build_wasm_posix_compat(wasi_sdk_dir: &Path) {
        println!("cargo:rerun-if-changed=src/wasm_compat.c");
        cc::Build::new()
            .file("src/wasm_compat.c")
            .compiler(tool_argument_path(&wasi_sdk_dir.join("bin/clang")))
            .archiver(tool_argument_path(&wasi_sdk_dir.join("bin/llvm-ar")))
            .target("wasm32-wasip1")
            .flag(format!(
                "--sysroot={}",
                tool_argument_path(&wasi_sdk_dir.join("share/wasi-sysroot")).display()
            ))
            .compile("xberg_wasm_posix_compat");
    }

    fn build_wasm() {
        eprintln!("Building for WASM target with WASI SDK");

        let artifact_cache_dir = prepare_out_dir().join("source-artifacts");
        let build_root = cargo_build_root();
        let project_dir = build_root.clone();
        let third_party_dir = project_dir.join("third_party");

        eprintln!("Looking for WASI SDK...");
        let wasi_sdk_dir = match find_wasi_sdk() {
            Ok(path) => {
                eprintln!("Found WASI SDK at: {}", path.display());
                path
            }
            Err(err) => {
                panic!(
                    "{}

Installation instructions:
  Download from: https://github.com/WebAssembly/wasi-sdk/releases
  Extract to ~/wasi-sdk or /opt/wasi-sdk
  Set WASI_SDK_PATH environment variable to the extracted directory",
                    err
                );
            }
        };

        build_wasm_posix_compat(&wasi_sdk_dir);

        let leptonica_dir = get_or_download_source(
            &artifact_cache_dir,
            &third_party_dir,
            &LEPTONICA_SOURCE,
            &leptonica_url(),
        )
        .path;

        let tesseract_source = get_or_download_source(
            &artifact_cache_dir,
            &third_party_dir,
            &TESSERACT_SOURCE,
            &tesseract_url(),
        );
        apply_tesseract_wasm_patch(&tesseract_source.path);
        apply_wasm_noop_mutex_patch(&tesseract_source.path);
        let tesseract_dir = tesseract_source.path;

        let leptonica_install_dir = build_root.join("leptonica");
        let _leptonica_link_name = build_static_library("leptonica", &leptonica_install_dir, || {
            eprintln!("Building Leptonica for WASM...");
            build_leptonica_wasm(&leptonica_dir, &leptonica_install_dir, &wasi_sdk_dir);
        });

        let tesseract_install_dir = build_root.join("tesseract");
        let _tesseract_link_name = build_static_library("tesseract", &tesseract_install_dir, || {
            eprintln!("Building Tesseract for WASM (SIMD enabled)...");
            build_tesseract_wasm(
                &tesseract_dir,
                &tesseract_install_dir,
                &leptonica_install_dir,
                &wasi_sdk_dir,
                true,
            );
        });

        let leptonica_lib_dir = leptonica_install_dir.join("lib");
        let tesseract_lib_dir = tesseract_install_dir.join("lib");

        println!(
            "cargo:rustc-link-search=native={}",
            tool_argument_path(&leptonica_lib_dir).display()
        );
        println!(
            "cargo:rustc-link-search=native={}",
            tool_argument_path(&tesseract_lib_dir).display()
        );

        println!("cargo:rustc-link-lib=static=tesseract");
        println!("cargo:rustc-link-lib=static=leptonica");

        let sysroot_lib = wasi_sdk_dir.join("share/wasi-sysroot/lib/wasm32-wasi");
        eprintln!("Linking WASI SDK sysroot from: {}", sysroot_lib.display());

        println!("cargo:rustc-link-search=native={}", sysroot_lib.display());
        let sysroot_lib_noeh = sysroot_lib.join("noeh");
        if sysroot_lib_noeh.exists() {
            println!("cargo:rustc-link-search=native={}", sysroot_lib_noeh.display());
        }
        println!("cargo:rustc-link-lib=static=c++");
        println!("cargo:rustc-link-lib=static=c++abi");
        println!("cargo:rustc-link-lib=static=c");
        println!("cargo:rustc-link-lib=static=wasi-emulated-process-clocks");
        println!("cargo:rustc-link-lib=static=wasi-emulated-signal");

        if let Some(rt_dir) = find_wasi_compiler_rt(&wasi_sdk_dir) {
            eprintln!("Linking compiler-rt from: {}", rt_dir.display());
            println!("cargo:rustc-link-search=native={}", rt_dir.display());
            println!("cargo:rustc-link-lib=static=clang_rt.builtins-wasm32");
        } else {
            eprintln!("compiler-rt builtins not found in WASI SDK, some symbols may be unresolved");
        }

        let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
        let eng_traineddata = PathBuf::from(&out_dir).join("eng.traineddata");
        prepare_eng_traineddata(&artifact_cache_dir, &eng_traineddata);

        eprintln!("WASM build completed successfully!");
        eprintln!("Leptonica install dir: {:?}", leptonica_install_dir);
        eprintln!("Tesseract install dir: {:?}", tesseract_install_dir);
    }

    fn build_tesseract_wasm(
        src_dir: &Path,
        tesseract_install: &Path,
        leptonica_install: &Path,
        wasi_sdk_dir: &Path,
        enable_simd: bool,
    ) {
        let toolchain_file = find_wasi_toolchain(wasi_sdk_dir);
        let sysroot = wasi_sdk_dir.join("share/wasi-sysroot");
        let clang = wasi_sdk_dir.join("bin/clang");
        let clangxx = wasi_sdk_dir.join("bin/clang++");

        let mut config = Config::new(
            cmake_source_path(src_dir).expect("CMake source path must preserve Tesseract directory identity"),
        );

        config.target("wasm32-wasi");
        if cfg!(target_os = "windows") {
            config.generator("Ninja");
        }
        config.define("CMAKE_TOOLCHAIN_FILE", normalize_cmake_path(&toolchain_file));
        config.define("CMAKE_SYSROOT", normalize_cmake_path(&sysroot));
        config.define("CMAKE_C_COMPILER", normalize_cmake_path(&clang));
        config.define("CMAKE_CXX_COMPILER", normalize_cmake_path(&clangxx));
        config.define("WASI_SDK_PREFIX", normalize_cmake_path(wasi_sdk_dir));

        let leptonica_lib_dir = leptonica_install.join("lib");
        let leptonica_include_dir = leptonica_install.join("include");

        let leptonica_cmake_dir = leptonica_install.join("lib/cmake/leptonica");
        config.define("Leptonica_DIR", normalize_cmake_path(&leptonica_cmake_dir));
        config.define("CMAKE_PREFIX_PATH", normalize_cmake_path(leptonica_install));
        config.define(
            "CMAKE_EXE_LINKER_FLAGS",
            format!("-L{}", normalize_cmake_path(&leptonica_lib_dir)),
        );

        let noop_mutex_include = src_dir.join("src");
        let mut cxx_flags = String::from(
            "-DTESSERACT_IMAGEDATA_AS_PIX -DTESSERACT_WASM_NOOP_MUTEX -fno-exceptions -D_WASI_EMULATED_PROCESS_CLOCKS -D_WASI_EMULATED_SIGNAL ",
        );
        if enable_simd {
            cxx_flags.push_str("-msimd128 ");
        }
        cxx_flags.push_str(&format!(
            "-fPIC -Os -fno-lto -I{} -I{}",
            normalize_cmake_path(&leptonica_include_dir),
            normalize_cmake_path(&noop_mutex_include)
        ));

        let c_flags = format!(
            "-fPIC -Os -fno-lto -fno-exceptions -D_WASI_EMULATED_PROCESS_CLOCKS -D_WASI_EMULATED_SIGNAL -I{}",
            normalize_cmake_path(&leptonica_include_dir)
        );

        config
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .define("CMAKE_TRY_COMPILE_TARGET_TYPE", "STATIC_LIBRARY")
            .define("LEPT_TIFF_RESULT", "1")
            .define("LEPT_TIFF_RESULT__TRYRUN_OUTPUT", "")
            .define("BUILD_TESSERACT_BINARY", "OFF")
            .define("BUILD_TRAINING_TOOLS", "OFF")
            .define("INSTALL_CONFIGS", "ON")
            .define("BUILD_TESTS", "OFF")
            .define("BUILD_PROG", "OFF")
            .define("SYNTAX_LOG", "OFF")
            .define("DISABLE_ARCHIVE", "ON")
            .define("DISABLE_CURL", "ON")
            .define("DISABLE_OPENCL", "ON")
            .define("DISABLE_TIFF", "ON")
            .define("DISABLE_PNG", "ON")
            .define("DISABLE_JPEG", "ON")
            .define("DISABLE_WEBP", "ON")
            .define("DISABLE_OPENJPEG", "ON")
            .define("DISABLE_ZLIB", "ON")
            .define("DISABLE_LIBXML2", "ON")
            .define("DISABLE_LIBICU", "ON")
            .define("DISABLE_LZMA", "ON")
            .define("DISABLE_GIF", "ON")
            .define("DISABLE_DEBUG_MESSAGES", "ON")
            .define("GRAPHICS_DISABLED", "ON")
            .define("USE_OPENCL", "OFF")
            .define("OPENMP_BUILD", "OFF")
            .define("ENABLE_LTO", "OFF")
            .define("HAVE_SSE4_1", "OFF")
            .define("HAVE_AVX", "OFF")
            .define("HAVE_AVX2", "OFF")
            .define("HAVE_AVX512F", "OFF")
            .define("HAVE_FMA", "OFF")
            .define("CMAKE_INSTALL_PREFIX", normalize_cmake_path(tesseract_install))
            .define("CMAKE_CXX_FLAGS", &cxx_flags)
            .define("CMAKE_C_FLAGS", &c_flags);

        config.build();
    }

    fn build_static_library<F>(name: &str, install_dir: &Path, build_fn: F) -> String
    where
        F: FnOnce(),
    {
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let target_triple = env::var("TARGET")
            .unwrap_or_else(|_| env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown".to_string()));
        let is_windows = target_triple.contains("windows");
        let is_windows_gnu = is_windows && target_env == "gnu";

        let lib_name = if is_windows && !is_windows_gnu {
            format!("{}.lib", name)
        } else {
            format!("lib{}.a", name)
        };

        let out_path = install_dir.join("lib").join(&lib_name);

        let possible_lib_names: Vec<String> = if is_windows {
            let mut base = match name {
                "leptonica" => vec![
                    "leptonica.lib".to_string(),
                    "libleptonica.lib".to_string(),
                    "leptonica-static.lib".to_string(),
                    format!("leptonica-{}.lib", LEPTONICA_VERSION),
                    "leptonica-1.86.0.lib".to_string(),
                    "leptonica-1.84.1.lib".to_string(),
                    "leptonicad.lib".to_string(),
                    "libleptonica_d.lib".to_string(),
                    format!("leptonica-{}d.lib", LEPTONICA_VERSION),
                    "leptonica-1.86.0d.lib".to_string(),
                    "leptonica-1.84.1d.lib".to_string(),
                ],
                "tesseract" => vec![
                    "tesseract.lib".to_string(),
                    "libtesseract.lib".to_string(),
                    "tesseract-static.lib".to_string(),
                    "tesseract53.lib".to_string(),
                    "tesseract54.lib".to_string(),
                    "tesseract55.lib".to_string(),
                    "tesseractd.lib".to_string(),
                    "libtesseract_d.lib".to_string(),
                    "tesseract53d.lib".to_string(),
                    "tesseract54d.lib".to_string(),
                    "tesseract55d.lib".to_string(),
                ],
                _ => vec![format!("{}.lib", name)],
            };

            if is_windows_gnu {
                match name {
                    "leptonica" => {
                        base.push(format!("libleptonica-{}.a", LEPTONICA_VERSION));
                        base.push("libleptonica.a".to_string());
                    }
                    "tesseract" => {
                        base.push(format!("libtesseract{}.a", TESSERACT_VERSION.replace('.', "")));
                        base.push("libtesseract.a".to_string());
                        base.push("libtesseract55.a".to_string());
                    }
                    _ => {
                        base.push(format!("lib{}.a", name));
                    }
                }
            }

            base
        } else {
            vec![format!("lib{}.a", name)]
        };

        if install_dir.exists() {
            fs::remove_dir_all(install_dir)
                .unwrap_or_else(|error| panic!("Failed to remove stale {name} install directory: {error}"));
        }
        fs::create_dir_all(out_path.parent().unwrap()).expect("Failed to create output directory");

        let candidate_lib_dirs = [
            install_dir.join("lib"),
            install_dir.join("lib64"),
            install_dir.join("lib").join("tesseract"),
        ];

        let link_name_to_use = {
            println!("Building {} library", name);
            build_fn();

            let mut found_lib_name = None;
            'search: for lib_name in &possible_lib_names {
                for dir in &candidate_lib_dirs {
                    let lib_path = dir.join(lib_name);
                    if lib_path.exists() {
                        eprintln!("Found {} library at: {}", name, lib_path.display());
                        let link_name = if lib_name.ends_with(".lib") {
                            lib_name.strip_suffix(".lib").unwrap_or(lib_name).to_string()
                        } else if lib_name.ends_with(".a") {
                            lib_name
                                .strip_prefix("lib")
                                .and_then(|s| s.strip_suffix(".a"))
                                .unwrap_or(lib_name)
                                .to_string()
                        } else {
                            lib_name.to_string()
                        };
                        found_lib_name = Some((lib_path, link_name));
                        break 'search;
                    }
                }
            }

            if let Some((lib_path, link_name)) = found_lib_name {
                if out_path.exists() {
                    println!(
                        "cargo:warning=Library already available at expected location: {}",
                        out_path.display()
                    );
                } else if let Err(e) = fs::copy(&lib_path, &out_path) {
                    eprintln!("Failed to copy library to standard location: {}", e);
                }
                link_name
            } else {
                println!(
                    "cargo:warning=Library {} not found! Searched for: {:?}",
                    name, possible_lib_names
                );
                for dir in &candidate_lib_dirs {
                    eprintln!("Checked directory: {}", dir.display());
                    if let Ok(entries) = fs::read_dir(dir) {
                        eprintln!("Files in {}:", dir.display());
                        for entry in entries.flatten() {
                            eprintln!("  - {}", entry.file_name().to_string_lossy());
                        }
                    } else {
                        eprintln!("Directory not accessible: {}", dir.display());
                    }
                }
                name.to_string()
            }
        };

        for dir in candidate_lib_dirs.iter().filter(|d| d.exists()) {
            println!("cargo:rustc-link-search=native={}", tool_argument_path(dir).display());
        }

        link_name_to_use
    }
}

#[cfg(all(feature = "dynamic-linking", not(feature = "build-tesseract-wasm")))]
fn build_pkg_config_shim() {
    let tesseract = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("tesseract")
        .unwrap_or_else(|error| panic!("failed to discover the system Tesseract headers with pkg-config: {error}"));
    let leptonica = pkg_config::Config::new()
        .cargo_metadata(false)
        .probe("lept")
        .unwrap_or_else(|error| panic!("failed to discover the system Leptonica headers with pkg-config: {error}"));

    let mut shim = cc::Build::new();
    shim.file("src/shim.cpp").cpp(true).std("c++17");
    for include_path in tesseract.include_paths.iter().chain(&leptonica.include_paths) {
        shim.include(include_path);
    }
    shim.compile("xberg_shim");

    pkg_config::Config::new()
        .probe("tesseract")
        .unwrap_or_else(|error| panic!("failed to link system Tesseract with pkg-config: {error}"));
    pkg_config::Config::new()
        .probe("lept")
        .unwrap_or_else(|error| panic!("failed to link system Leptonica with pkg-config: {error}"));
}

#[cfg(all(feature = "dynamic-linking", not(feature = "build-tesseract-wasm")))]
fn build_vcpkg_shim() {
    let tesseract = vcpkg::Config::new()
        .cargo_metadata(false)
        .find_package("tesseract")
        .unwrap_or_else(|error| panic!("failed to discover system Tesseract with vcpkg: {error}"));

    let mut shim = cc::Build::new();
    shim.file("src/shim.cpp").cpp(true).std("c++17");
    for include_path in &tesseract.include_paths {
        shim.include(include_path);
    }
    shim.compile("xberg_shim");

    for metadata in &tesseract.cargo_metadata {
        println!("{metadata}");
    }
}

#[cfg(all(feature = "dynamic-linking", not(feature = "build-tesseract-wasm")))]
fn build_system_shim() {
    let target_environment =
        std::env::var("CARGO_CFG_TARGET_ENV").expect("Cargo must set CARGO_CFG_TARGET_ENV for build scripts");
    if target_environment == "msvc" {
        build_vcpkg_shim();
    } else {
        build_pkg_config_shim();
    }
}

fn main() {
    // docs.rs only needs Rust declarations; native downloads and linking are unavailable. ~keep
    if std::env::var_os("DOCS_RS").is_some() {
        return;
    }

    // A release workflow exports `MACOSX_DEPLOYMENT_TARGET` for its macOS legs, and cmake then
    // configures an iOS target as macOS: the static libraries come out with platform MACOS and
    // the final iOS link fails. iOS takes `IPHONEOS_DEPLOYMENT_TARGET` instead. ~keep
    if std::env::var("TARGET").is_ok_and(|target| target.contains("apple-ios")) {
        // SAFETY: a build script is single-threaded at this point; nothing else reads the env.
        unsafe { std::env::remove_var("MACOSX_DEPLOYMENT_TARGET") };
    }

    // `dynamic-linking` is an explicit opt out of the vendored build, so it outranks the
    // default `build-tesseract` rather than being ignored beside it. Cargo features are
    // additive and a dependent cannot switch off a dependency's defaults, so precedence here
    // is the only way a network-isolated build -- a conda-forge recipe, a distro package --
    // can ask for the system libraries. `build-tesseract-wasm` still wins: that target has no
    // system Tesseract to link against. ~keep
    #[cfg(any(
        feature = "build-tesseract-wasm",
        all(feature = "build-tesseract", not(feature = "dynamic-linking"))
    ))]
    {
        build_tesseract::build();
    }

    #[cfg(all(feature = "dynamic-linking", not(feature = "build-tesseract-wasm")))]
    {
        eprintln!("Using dynamic linking with system-installed Tesseract libraries");
        println!("cargo:rerun-if-changed=src/shim.cpp");
        build_system_shim();
    }
}
