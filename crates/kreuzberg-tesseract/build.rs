#![allow(clippy::uninlined_format_args)]

#[cfg(feature = "build-tesseract")]
mod build_tesseract {
    use cmake::Config;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::thread;
    use std::time::Duration;

    // Use specific release versions for stability
    const LEPTONICA_VERSION: &str = "1.86.0";
    const TESSERACT_VERSION: &str = "5.5.1";
    const MAX_DOWNLOAD_ATTEMPTS: u32 = 4;
    const RETRY_DELAY_SECS: u64 = 5;

    fn leptonica_urls() -> Vec<String> {
        if let Ok(url) = env::var("TESSERACT_LEPTONICA_URL") {
            return vec![url];
        }

        vec![
            format!(
                "https://codeload.github.com/DanBloomberg/leptonica/zip/refs/tags/{}",
                LEPTONICA_VERSION
            ),
            format!(
                "https://github.com/DanBloomberg/leptonica/archive/refs/tags/{}.zip",
                LEPTONICA_VERSION
            ),
        ]
    }

    fn tesseract_urls() -> Vec<String> {
        if let Ok(url) = env::var("TESSERACT_ARCHIVE_URL") {
            return vec![url];
        }

        vec![
            format!(
                "https://codeload.github.com/tesseract-ocr/tesseract/zip/refs/tags/{}",
                TESSERACT_VERSION
            ),
            format!(
                "https://github.com/tesseract-ocr/tesseract/archive/refs/tags/{}.zip",
                TESSERACT_VERSION
            ),
        ]
    }

    fn workspace_cache_dir_from_out_dir() -> Option<PathBuf> {
        let out_dir = env::var_os("OUT_DIR")?;
        let mut path = PathBuf::from(out_dir);
        // OUT_DIR looks like target/<triple>/<profile>/build/<crate-hash>/out
        // Pop until we get back to target/<triple>
        for _ in 0..4 {
            if !path.pop() {
                return None;
            }
        }
        Some(path.join("tesseract-rs-cache"))
    }

    fn get_preferred_out_dir() -> PathBuf {
        if let Ok(custom) = env::var("TESSERACT_RS_CACHE_DIR") {
            return PathBuf::from(custom);
        }

        if let Some(workspace_cache) = workspace_cache_dir_from_out_dir() {
            return workspace_cache;
        }

        if cfg!(target_os = "macos") {
            let home_dir = env::var("HOME").unwrap_or_else(|_| {
                env::var("USER")
                    .map(|user| format!("/Users/{}", user))
                    .expect("Neither HOME nor USER environment variable set")
            });
            PathBuf::from(home_dir)
                .join("Library")
                .join("Application Support")
                .join("tesseract-rs")
        } else if cfg!(target_os = "linux") {
            let home_dir = env::var("HOME").unwrap_or_else(|_| {
                env::var("USER")
                    .map(|user| format!("/home/{}", user))
                    .expect("Neither HOME nor USER environment variable set")
            });
            PathBuf::from(home_dir).join(".tesseract-rs")
        } else if cfg!(target_os = "windows") {
            env::var("APPDATA")
                .or_else(|_| env::var("USERPROFILE").map(|p| format!("{}\\AppData\\Roaming", p)))
                .map(PathBuf::from)
                .expect("Neither APPDATA nor USERPROFILE environment variable set")
                .join("tesseract-rs")
        } else {
            panic!("Unsupported operating system");
        }
    }

    fn prepare_out_dir() -> PathBuf {
        let preferred = get_preferred_out_dir();
        match fs::create_dir_all(&preferred) {
            Ok(_) => preferred,
            Err(err) => {
                println!(
                    "cargo:warning=Failed to create cache dir {:?}: {}. Falling back to temp dir.",
                    preferred, err
                );
                let fallback = env::temp_dir().join("tesseract-rs-cache");
                fs::create_dir_all(&fallback).expect("Failed to create fallback cache directory in temp dir");
                fallback
            }
        }
    }

    pub fn build() {
        let custom_out_dir = prepare_out_dir();

        println!("cargo:warning=custom_out_dir: {:?}", custom_out_dir);

        let cache_dir = custom_out_dir.join("cache");

        if env::var("CARGO_CLEAN").is_ok() {
            clean_cache(&cache_dir);
        }

        std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        let out_dir = custom_out_dir.clone();
        let project_dir = custom_out_dir.clone();
        let third_party_dir = project_dir.join("third_party");

        let leptonica_sources = leptonica_urls();
        let leptonica_dir = if third_party_dir.join("leptonica").exists() {
            println!("cargo:warning=Using existing leptonica source");
            third_party_dir.join("leptonica")
        } else {
            fs::create_dir_all(&third_party_dir).expect("Failed to create third_party directory");
            download_and_extract(&third_party_dir, &leptonica_sources, "leptonica")
        };

        let tesseract_sources = tesseract_urls();
        let tesseract_dir = if third_party_dir.join("tesseract").exists() {
            println!("cargo:warning=Using existing tesseract source");
            third_party_dir.join("tesseract")
        } else {
            fs::create_dir_all(&third_party_dir).expect("Failed to create third_party directory");
            download_and_extract(&third_party_dir, &tesseract_sources, "tesseract")
        };

        let (cmake_cxx_flags, additional_defines) = get_os_specific_config();

        let leptonica_install_dir = out_dir.join("leptonica");
        let leptonica_cache_dir = cache_dir.join("leptonica");

        build_or_use_cached("leptonica", &leptonica_cache_dir, &leptonica_install_dir, || {
            let mut leptonica_config = Config::new(&leptonica_dir);

            let leptonica_src_dir = leptonica_dir.join("src");
            let environ_h_path = leptonica_src_dir.join("environ.h");

            // Only modify environ.h if it exists
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

            // Only modify makefile.static if it exists
            if makefile_static_path.exists() {
                let makefile_static = std::fs::read_to_string(&makefile_static_path)
                    .expect("Failed to read makefile.static")
                    .replace(
                        "ALL_LIBS =	$(LEPTLIB) -ltiff -ljpeg -lpng -lz -lm",
                        "ALL_LIBS =	$(LEPTLIB) -lm",
                    );
                std::fs::write(makefile_static_path, makefile_static).expect("Failed to write makefile.static");
            }

            // Configure build tools
            if cfg!(target_os = "windows") {
                // Use NMake on Windows for better compatibility
                if let Ok(_vs_install_dir) = env::var("VSINSTALLDIR") {
                    leptonica_config.generator("NMake Makefiles");
                }
            }

            // Only use sccache if not in CI
            if env::var("CI").is_err() && env::var("RUSTC_WRAPPER").unwrap_or_default() == "sccache" {
                leptonica_config.env("CC", "sccache cc").env("CXX", "sccache c++");
            }
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
                .define("MINIMUM_SEVERITY", "L_SEVERITY_NONE")
                .define("SW_BUILD", "OFF")
                .define("HAVE_LIBZ", "0")
                .define("ENABLE_LTO", "OFF")
                .define("CMAKE_INSTALL_PREFIX", &leptonica_install_dir);

            // Windows-specific defines
            if cfg!(target_os = "windows") {
                if cfg!(target_env = "msvc") {
                    leptonica_config
                        .define("CMAKE_C_FLAGS_RELEASE", "/MD /O2")
                        .define("CMAKE_C_FLAGS_DEBUG", "/MDd /Od");
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
        let tesseract_cache_dir = cache_dir.join("tesseract");
        // TESSDATA_PREFIX should point to parent directory of tessdata, not tessdata itself
        // Tesseract will append the platform-specific path separator and 'tessdata' directory internally
        let tessdata_prefix = project_dir.clone();
        let tessdata_prefix_cmake = normalize_cmake_path(&tessdata_prefix);

        build_or_use_cached("tesseract", &tesseract_cache_dir, &tesseract_install_dir, || {
            let cmakelists_path = tesseract_dir.join("CMakeLists.txt");
            let cmakelists = std::fs::read_to_string(&cmakelists_path)
                .expect("Failed to read CMakeLists.txt")
                .replace("set(HAVE_TIFFIO_H ON)", "");
            std::fs::write(&cmakelists_path, cmakelists).expect("Failed to write CMakeLists.txt");

            let mut tesseract_config = Config::new(&tesseract_dir);
            // Configure build tools
            if cfg!(target_os = "windows") {
                // Use NMake on Windows for better compatibility
                if let Ok(_vs_install_dir) = env::var("VSINSTALLDIR") {
                    tesseract_config.generator("NMake Makefiles");
                }
            }

            // Only use sccache if not in CI
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
                .define("Leptonica_DIR", &leptonica_install_dir)
                .define("LEPTONICA_INCLUDE_DIR", &leptonica_include_dir)
                .define("LEPTONICA_LIBRARY", &leptonica_lib_dir)
                .define("CMAKE_PREFIX_PATH", &leptonica_install_dir)
                .define("CMAKE_INSTALL_PREFIX", &tesseract_install_dir)
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
                .define("SW_BUILD", "OFF")
                .define("LEPT_TIFF_RESULT", "FALSE")
                .define("INSTALL_CONFIGS", "ON")
                .define("USE_SYSTEM_ICU", "ON")
                .define("CMAKE_CXX_FLAGS", &cmake_cxx_flags);

            for (key, value) in &additional_defines {
                tesseract_config.define(key, value);
            }

            tesseract_config.build();
        });

        println!("cargo:rerun-if-changed=build.rs");
        println!("cargo:rerun-if-changed={}", third_party_dir.display());
        println!("cargo:rerun-if-changed={}", leptonica_dir.display());
        println!("cargo:rerun-if-changed={}", tesseract_dir.display());

        println!("cargo:rustc-link-search=native={}", leptonica_lib_dir.display());
        println!(
            "cargo:rustc-link-search=native={}",
            tesseract_install_dir.join("lib").display()
        );
        // Don't emit link directives here - let build_or_use_cached handle it

        set_os_specific_link_flags();

        println!("cargo:warning=Leptonica include dir: {:?}", leptonica_include_dir);
        println!("cargo:warning=Leptonica lib dir: {:?}", leptonica_lib_dir);
        println!("cargo:warning=Tesseract install dir: {:?}", tesseract_install_dir);
        println!("cargo:warning=Tessdata dir: {:?}", tessdata_prefix);

        download_tessdata(&project_dir);
    }

    fn get_os_specific_config() -> (String, Vec<(String, String)>) {
        let mut cmake_cxx_flags = String::new();
        let mut additional_defines = Vec::new();

        if cfg!(target_os = "macos") {
            cmake_cxx_flags.push_str("-stdlib=libc++ ");
            cmake_cxx_flags.push_str("-std=c++17 ");
        } else if cfg!(target_os = "linux") {
            cmake_cxx_flags.push_str("-std=c++17 ");
            // Check if we're on a system using clang
            if cfg!(target_env = "musl") || env::var("CC").map(|cc| cc.contains("clang")).unwrap_or(false) {
                cmake_cxx_flags.push_str("-stdlib=libc++ ");
                // Use CXX env var if set, otherwise derive from target for cross-compilation
                let cxx_compiler = env::var("CXX").unwrap_or_else(|_| {
                    if let Ok(target) = env::var("TARGET") {
                        if target != env::var("HOST").unwrap_or_default() {
                            // Cross-compiling - use target-prefixed compiler
                            format!("{}-clang++", target)
                        } else {
                            "clang++".to_string()
                        }
                    } else {
                        "clang++".to_string()
                    }
                });
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx_compiler));
            } else {
                // Assume GCC - use CXX env var if set, otherwise derive from target for cross-compilation
                let cxx_compiler = env::var("CXX").unwrap_or_else(|_| {
                    if let Ok(target) = env::var("TARGET") {
                        if target != env::var("HOST").unwrap_or_default() {
                            // Cross-compiling - use target-prefixed compiler
                            format!("{}-g++", target)
                        } else {
                            "g++".to_string()
                        }
                    } else {
                        "g++".to_string()
                    }
                });
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), cxx_compiler));
            }
        } else if cfg!(target_os = "windows") {
            if cfg!(target_env = "msvc") {
                // Windows-specific MSVC flags
                // Add TESSERACT_STATIC to prevent __declspec(dllimport) on API functions
                cmake_cxx_flags.push_str("/EHsc /MP /std:c++17 /DTESSERACT_STATIC ");
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_RELEASE".to_string(),
                    "/MD /O2 /DTESSERACT_STATIC".to_string(),
                ));
                additional_defines.push((
                    "CMAKE_CXX_FLAGS_DEBUG".to_string(),
                    "/MDd /Od /DTESSERACT_STATIC".to_string(),
                ));
                // Do NOT set CMAKE_WINDOWS_EXPORT_ALL_SYMBOLS for static libraries
                // This flag causes CMake to export symbols for DLL linkage which creates
                // __imp_ prefixed symbols that the linker can't find in static libraries
                additional_defines.push(("CMAKE_MSVC_RUNTIME_LIBRARY".to_string(), "MultiThreadedDLL".to_string()));
            } else {
                // MinGW/GNU toolchains reject MSVC-style flags, so stick to GCC syntax
                cmake_cxx_flags.push_str("-std=c++17 -DTESSERACT_STATIC ");
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

        // Common flags and defines for all platforms
        cmake_cxx_flags.push_str("-DUSE_STD_NAMESPACE ");
        additional_defines.push(("CMAKE_POSITION_INDEPENDENT_CODE".to_string(), "ON".to_string()));

        (cmake_cxx_flags, additional_defines)
    }

    fn set_os_specific_link_flags() {
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-lib=c++");
        } else if cfg!(target_os = "linux") {
            if cfg!(target_env = "musl") || env::var("CC").map(|cc| cc.contains("clang")).unwrap_or(false) {
                println!("cargo:rustc-link-lib=c++");
            } else {
                println!("cargo:rustc-link-lib=stdc++");
                // GCC < 9 requires explicit linking of stdc++fs for filesystem support
                // It's safe to link even if not needed (GCC 9+ has it built-in)
                println!("cargo:rustc-link-lib=stdc++fs");
            }
            println!("cargo:rustc-link-lib=pthread");
            println!("cargo:rustc-link-lib=m");
            println!("cargo:rustc-link-lib=dl");
        } else if cfg!(target_os = "windows") {
            if cfg!(target_env = "gnu") {
                println!("cargo:rustc-link-lib=stdc++");
            }
            // Windows requires explicit linking of system libraries for static tesseract
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
            println!("cargo:rustc-link-lib=ws2_32");
            println!("cargo:rustc-link-lib=advapi32");
            println!("cargo:rustc-link-lib=shell32");
        }

        println!("cargo:rustc-link-search=native={}", env::var("OUT_DIR").unwrap());
    }

    fn download_and_extract(target_dir: &Path, urls: &[String], name: &str) -> PathBuf {
        use zip::ZipArchive;

        fs::create_dir_all(target_dir).expect("Failed to create target directory");

        let content = download_bytes_from_urls(urls, name);

        let temp_file = target_dir.join(format!("{}.zip", name));
        fs::write(&temp_file, content).expect("Failed to write archive to file");

        let extract_dir = target_dir.join(name);
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir).expect("Failed to remove existing directory");
        }
        fs::create_dir_all(&extract_dir).expect("Failed to create extraction directory");

        let mut archive = ZipArchive::new(fs::File::open(&temp_file).unwrap()).unwrap();

        // Extract files, ignoring the top-level directory
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let file_path = file.mangled_name();
            let file_path = file_path.to_str().unwrap();

            // Skip the top-level directory
            let path = Path::new(file_path);
            let path = path.strip_prefix(path.components().next().unwrap()).unwrap();

            if path.as_os_str().is_empty() {
                continue;
            }

            let target_path = extract_dir.join(path);

            if file.is_dir() {
                fs::create_dir_all(target_path).unwrap();
            } else {
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                let mut outfile = fs::File::create(target_path).unwrap();
                std::io::copy(&mut file, &mut outfile).unwrap();
            }
        }

        fs::remove_file(temp_file).expect("Failed to remove temporary zip file");

        extract_dir
    }

    fn download_bytes_from_urls(urls: &[String], name: &str) -> Vec<u8> {
        use reqwest::blocking::Client;

        let user_agent = format!(
            "kreuzberg-tesseract/{} ({}/{})",
            env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "dev".to_string()),
            env::var("CARGO_CFG_TARGET_OS").unwrap_or_else(|_| "unknown-os".to_string()),
            env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown-arch".to_string()),
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .user_agent(user_agent)
            .build()
            .expect("Failed to create HTTP client");

        let mut errors = Vec::new();

        for url in urls {
            for attempt in 1..=MAX_DOWNLOAD_ATTEMPTS {
                println!(
                    "cargo:warning=Downloading {} (attempt {}/{}) from {}",
                    name, attempt, MAX_DOWNLOAD_ATTEMPTS, url
                );

                match download_from_url(&client, url) {
                    Ok(content) => {
                        println!(
                            "cargo:warning=Downloaded {} bytes for {} from {}",
                            content.len(),
                            name,
                            url
                        );
                        return content;
                    }
                    Err(err) => {
                        println!(
                            "cargo:warning=Download failed for {} from {} (attempt {}): {}",
                            name, url, attempt, err.message
                        );
                        errors.push(format!("{} attempt {}: {}", url, attempt, err.message));

                        if attempt == MAX_DOWNLOAD_ATTEMPTS || !err.retryable {
                            println!("cargo:warning=Giving up on {} after {} attempt(s)", url, attempt);
                            break;
                        }

                        let delay = compute_retry_delay(attempt);
                        println!("cargo:warning=Retrying {} from {} in {}s", name, url, delay.as_secs());
                        thread::sleep(delay);
                    }
                }
            }
        }

        panic!(
            "Failed to download {} after trying {} mirror(s).\nErrors:\n{}",
            name,
            urls.len(),
            errors.join("\n")
        );
    }

    fn compute_retry_delay(attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_secs(RETRY_DELAY_SECS);
        }
        let exponent = attempt.saturating_sub(1).min(10);
        let backoff = 1u64 << exponent;
        Duration::from_secs(RETRY_DELAY_SECS.saturating_mul(backoff))
    }

    struct DownloadError {
        message: String,
        retryable: bool,
    }

    impl DownloadError {
        fn new(message: impl Into<String>, retryable: bool) -> Self {
            Self {
                message: message.into(),
                retryable,
            }
        }

        fn transient(message: impl Into<String>) -> Self {
            Self::new(message, true)
        }
    }

    fn download_from_url(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, DownloadError> {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            return Err(DownloadError::new(format!("Unsupported URL scheme for {}", url), false));
        }

        let mut response = client
            .get(url)
            .send()
            .map_err(|err| DownloadError::transient(format!("Request error: {}", err)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(DownloadError::new(
                format!("HTTP {} from {}", status, url),
                should_retry_status(status),
            ));
        }

        let mut content = Vec::new();
        response
            .copy_to(&mut content)
            .map_err(|err| DownloadError::transient(format!("Failed to read response: {}", err)))?;

        if content.is_empty() {
            return Err(DownloadError::transient(format!("Received empty payload from {}", url)));
        }

        Ok(content)
    }

    fn should_retry_status(status: reqwest::StatusCode) -> bool {
        status.is_server_error()
            || matches!(
                status,
                reqwest::StatusCode::TOO_MANY_REQUESTS
                    | reqwest::StatusCode::REQUEST_TIMEOUT
                    | reqwest::StatusCode::BAD_GATEWAY
                    | reqwest::StatusCode::SERVICE_UNAVAILABLE
                    | reqwest::StatusCode::GATEWAY_TIMEOUT
            )
    }

    fn normalize_cmake_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }

    fn download_tessdata(project_dir: &Path) {
        let tessdata_dir = project_dir.join("tessdata");
        fs::create_dir_all(&tessdata_dir).expect("Failed to create Tessdata directory");

        let languages = ["eng", "tur"];

        for lang in &languages {
            let filename = format!("{}.traineddata", lang);
            let file_path = tessdata_dir.join(&filename);

            if file_path.exists() {
                println!("cargo:warning={} already exists, skipping download", filename);
                continue;
            }

            let urls = tessdata_urls(lang);
            let bytes = download_bytes_from_urls(&urls, &format!("tessdata {}", lang));
            std::fs::write(&file_path, &bytes).expect("Failed to write Tessdata file");
            println!("cargo:warning={} downloaded", filename);
        }
    }

    fn tessdata_urls(lang: &str) -> Vec<String> {
        if let Ok(base) = env::var("TESSERACT_TESSDATA_BASE_URL") {
            return vec![format!("{}/{}.traineddata", base.trim_end_matches('/'), lang)];
        }

        vec![
            format!(
                "https://github.com/tesseract-ocr/tessdata_best/raw/main/{}.traineddata",
                lang
            ),
            format!(
                "https://raw.githubusercontent.com/tesseract-ocr/tessdata_best/main/{}.traineddata",
                lang
            ),
        ]
    }

    fn clean_cache(cache_dir: &Path) {
        println!("Cleaning cache directory: {:?}", cache_dir);
        if cache_dir.exists() {
            fs::remove_dir_all(cache_dir).expect("Failed to remove cache directory");
        }
    }

    fn build_or_use_cached<F>(name: &str, cache_dir: &Path, install_dir: &Path, build_fn: F)
    where
        F: FnOnce(),
    {
        let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        let target_triple = env::var("TARGET")
            .unwrap_or_else(|_| env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "unknown".to_string()));
        let is_windows = cfg!(target_os = "windows");
        let is_windows_gnu = is_windows && target_env == "gnu";

        // Expected library name for caching
        let lib_name = if is_windows && !is_windows_gnu {
            format!("{}.lib", name)
        } else {
            format!("lib{}.a", name)
        };

        let cached_path = cache_dir.join(&lib_name);
        let marker_path = cache_dir.join(format!("{}.target", name));
        let out_path = install_dir.join("lib").join(&lib_name);

        // For Windows, check multiple possible library names
        // Include both release and debug variants (debug has 'd' suffix)
        let possible_lib_names: Vec<String> = if is_windows {
            let mut base = match name {
                "leptonica" => vec![
                    "leptonica.lib".to_string(),
                    "libleptonica.lib".to_string(),
                    "leptonica-static.lib".to_string(),
                    "leptonica-1.84.1.lib".to_string(),
                    "leptonica-1.86.0.lib".to_string(),
                    // Debug variants
                    "leptonicad.lib".to_string(),
                    "libleptonica_d.lib".to_string(),
                    "leptonica-1.84.1d.lib".to_string(),
                    "leptonica-1.86.0d.lib".to_string(),
                ],
                "tesseract" => vec![
                    "tesseract.lib".to_string(),
                    "libtesseract.lib".to_string(),
                    "tesseract-static.lib".to_string(),
                    "tesseract53.lib".to_string(),
                    "tesseract54.lib".to_string(),
                    "tesseract55.lib".to_string(),
                    // Debug variants
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

        fs::create_dir_all(cache_dir).expect("Failed to create cache directory");
        fs::create_dir_all(out_path.parent().unwrap()).expect("Failed to create output directory");

        let candidate_lib_dirs = [
            install_dir.join("lib"),
            install_dir.join("lib64"),
            install_dir.join("lib").join("tesseract"),
        ];

        // Determine which library name to use for linking
        let cache_valid = cached_path.exists()
            && {
                match fs::read_to_string(&marker_path) {
                    Ok(cached_target) => {
                        let valid = cached_target.trim() == target_triple;
                        if !valid {
                            println!(
                                "cargo:warning=Cached {} library is for wrong architecture (cached: {}, current: {}), rebuilding",
                                name,
                                cached_target.trim(),
                                target_triple
                            );
                            // Delete invalid cached library to force rebuild
                            let _ = fs::remove_file(&cached_path);
                            let _ = fs::remove_file(&marker_path);
                        }
                        valid
                    }
                    Err(_) => {
                        println!(
                            "cargo:warning=Cached {} library missing target marker, rebuilding",
                            name
                        );
                        // Delete cached library without marker to force rebuild
                        let _ = fs::remove_file(&cached_path);
                        false
                    }
                }
            };

        let link_name_to_use = if cache_valid {
            println!("cargo:warning=Using cached {} library for {}", name, target_triple);
            if let Err(e) = fs::copy(&cached_path, &out_path) {
                println!("cargo:warning=Failed to copy cached library: {}", e);
                // If cache copy fails, rebuild
                build_fn();
            }
            // Use generic name for cached libraries
            name.to_string()
        } else {
            println!("Building {} library", name);
            build_fn();

            // Look for the library with various possible names
            let mut found_lib_name = None;
            'search: for lib_name in &possible_lib_names {
                for dir in &candidate_lib_dirs {
                    let lib_path = dir.join(lib_name);
                    if lib_path.exists() {
                        println!("cargo:warning=Found {} library at: {}", name, lib_path.display());
                        // Extract the library name without extension for linking
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
                // Copy to expected location for caching
                if out_path.exists() {
                    println!(
                        "cargo:warning=Library already available at expected location: {}",
                        out_path.display()
                    );
                } else if let Err(e) = fs::copy(&lib_path, &out_path) {
                    println!("cargo:warning=Failed to copy library to standard location: {}", e);
                }
                // Cache the library
                if let Err(e) = fs::copy(&lib_path, &cached_path) {
                    println!("cargo:warning=Failed to cache library: {}", e);
                } else {
                    // Write target marker file
                    if let Err(e) = fs::write(&marker_path, &target_triple) {
                        println!("cargo:warning=Failed to write cache marker: {}", e);
                    } else {
                        println!("cargo:warning=Cached {} library for {}", name, target_triple);
                    }
                }
                link_name
            } else {
                println!(
                    "cargo:warning=Library {} not found! Searched for: {:?}",
                    name, possible_lib_names
                );
                for dir in &candidate_lib_dirs {
                    println!("cargo:warning=Checked directory: {}", dir.display());
                    if let Ok(entries) = fs::read_dir(dir) {
                        println!("cargo:warning=Files in {}:", dir.display());
                        for entry in entries.flatten() {
                            println!("cargo:warning=  - {}", entry.file_name().to_string_lossy());
                        }
                    } else {
                        println!("cargo:warning=Directory not accessible: {}", dir.display());
                    }
                }
                // Fallback to generic name
                name.to_string()
            }
        };

        // Set up linking using the determined library name
        for dir in candidate_lib_dirs.iter().filter(|d| d.exists()) {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }

        // Determine link type based on features
        #[cfg(feature = "dynamic-linking")]
        let link_type = "dylib";
        #[cfg(not(feature = "dynamic-linking"))]
        let link_type = "static";

        println!("cargo:rustc-link-lib={}={}", link_type, link_name_to_use);
        println!(
            "cargo:warning=Linking with library ({} linking): {}",
            link_type, link_name_to_use
        );
    }
}

fn main() {
    #[cfg(feature = "build-tesseract")]
    {
        build_tesseract::build();
    }

    #[cfg(all(feature = "dynamic-linking", not(feature = "build-tesseract")))]
    {
        // Dynamic linking with system libraries - no build needed
        println!("cargo:warning=Using dynamic linking with system-installed Tesseract libraries");
        println!("cargo:rustc-link-lib=dylib=tesseract");
        println!("cargo:rustc-link-lib=dylib=leptonica");
    }
}
