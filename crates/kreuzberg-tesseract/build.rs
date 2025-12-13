#![allow(clippy::uninlined_format_args)]

#[cfg(feature = "build-tesseract")]
mod build_tesseract {
    use cmake::Config;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    const LEPTONICA_VERSION: &str = "1.86.0";
    const TESSERACT_VERSION: &str = "5.5.1";

    fn leptonica_url() -> String {
        format!(
            "https://github.com/DanBloomberg/leptonica/archive/refs/tags/{}.zip",
            LEPTONICA_VERSION
        )
    }

    fn tesseract_url() -> String {
        format!(
            "https://github.com/tesseract-ocr/tesseract/archive/refs/tags/{}.zip",
            TESSERACT_VERSION
        )
    }

    fn workspace_cache_dir_from_out_dir() -> Option<PathBuf> {
        let out_dir = env::var_os("OUT_DIR")?;
        let mut path = PathBuf::from(out_dir);
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

        if cfg!(target_os = "windows") {
            return PathBuf::from("C:\\tess");
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
        } else {
            panic!("Unsupported operating system");
        }
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
        let target = target_triple();
        let windows_target = is_windows_target(&target);
        let msvc_target = is_msvc_target(&target);
        let mingw_target = is_mingw_target(&target);

        println!("cargo:warning=custom_out_dir: {:?}", custom_out_dir);

        let cache_dir = custom_out_dir.join("cache");

        if env::var("CARGO_CLEAN").is_ok() {
            clean_cache(&cache_dir);
        }

        std::fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        let out_dir = custom_out_dir.clone();
        let project_dir = custom_out_dir.clone();
        let third_party_dir = project_dir.join("third_party");

        let leptonica_dir = if third_party_dir.join("leptonica").exists() {
            println!("cargo:warning=Using existing leptonica source");
            third_party_dir.join("leptonica")
        } else {
            fs::create_dir_all(&third_party_dir).expect("Failed to create third_party directory");
            download_and_extract(&third_party_dir, &leptonica_url(), "leptonica")
        };

        let tesseract_dir = if third_party_dir.join("tesseract").exists() {
            println!("cargo:warning=Using existing tesseract source");
            third_party_dir.join("tesseract")
        } else {
            fs::create_dir_all(&third_party_dir).expect("Failed to create third_party directory");
            download_and_extract(&third_party_dir, &tesseract_url(), "tesseract")
        };

        let (cmake_cxx_flags, additional_defines) = get_os_specific_config();

        let leptonica_install_dir = out_dir.join("leptonica");
        let leptonica_cache_dir = cache_dir.join("leptonica");

        build_or_use_cached("leptonica", &leptonica_cache_dir, &leptonica_install_dir, || {
            let mut leptonica_config = Config::new(&leptonica_dir);

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
                    leptonica_config.generator("MinGW Makefiles");
                    leptonica_config.define("MSYS2_ARG_CONV_EXCL", "/MD;/MDd");
                } else if msvc_target && env::var("VSINSTALLDIR").is_ok() {
                    leptonica_config.generator("NMake Makefiles");
                }
                leptonica_config.define("CMAKE_CL_SHOWINCLUDES_PREFIX", "");
            }

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
        let tesseract_cache_dir = cache_dir.join("tesseract");
        let tessdata_prefix = project_dir.clone();
        let tessdata_prefix_cmake = normalize_cmake_path(&tessdata_prefix);

        build_or_use_cached("tesseract", &tesseract_cache_dir, &tesseract_install_dir, || {
            let cmakelists_path = tesseract_dir.join("CMakeLists.txt");
            let cmakelists = std::fs::read_to_string(&cmakelists_path)
                .expect("Failed to read CMakeLists.txt")
                .replace("set(HAVE_TIFFIO_H ON)", "");
            std::fs::write(&cmakelists_path, cmakelists).expect("Failed to write CMakeLists.txt");

            let mut tesseract_config = Config::new(&tesseract_dir);
            if windows_target {
                if mingw_target {
                    tesseract_config.generator("MinGW Makefiles");
                    tesseract_config.define("MSYS2_ARG_CONV_EXCL", "/MD;/MDd");
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

        set_os_specific_link_flags();

        println!("cargo:warning=Leptonica include dir: {:?}", leptonica_include_dir);
        println!("cargo:warning=Leptonica lib dir: {:?}", leptonica_lib_dir);
        println!("cargo:warning=Tesseract install dir: {:?}", tesseract_install_dir);
        println!("cargo:warning=Tessdata dir: {:?}", tessdata_prefix);
    }

    fn get_os_specific_config() -> (String, Vec<(String, String)>) {
        let mut cmake_cxx_flags = String::new();
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
        } else if target_linux {
            cmake_cxx_flags.push_str("-std=c++17 ");
            if target_musl || env::var("CC").map(|cc| cc.contains("clang")).unwrap_or(false) {
                cmake_cxx_flags.push_str("-stdlib=libc++ ");
                let cxx_compiler = env::var("CXX").unwrap_or_else(|_| {
                    if let Ok(target) = env::var("TARGET") {
                        if target != env::var("HOST").unwrap_or_default() {
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
                let cxx_compiler = env::var("CXX").unwrap_or_else(|_| {
                    if let Ok(target) = env::var("TARGET") {
                        if target != env::var("HOST").unwrap_or_default() {
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
        } else if target_windows {
            if target_msvc {
                cmake_cxx_flags.push_str("/EHsc /MP /std:c++17 /DTESSERACT_STATIC ");
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
                cmake_cxx_flags.push_str("-std=c++17 -DTESSERACT_STATIC ");
                additional_defines.push(("CMAKE_C_FLAGS_RELEASE".to_string(), "-O2 -DNDEBUG".to_string()));
                additional_defines.push(("CMAKE_C_FLAGS_DEBUG".to_string(), "-O0 -g".to_string()));
                additional_defines.push(("CMAKE_C_COMPILER".to_string(), "gcc".to_string()));
                additional_defines.push(("CMAKE_CXX_COMPILER".to_string(), "g++".to_string()));
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

        (cmake_cxx_flags, additional_defines)
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
        } else if target_linux {
            if target_musl || env::var("CC").map(|cc| cc.contains("clang")).unwrap_or(false) {
                println!("cargo:rustc-link-lib=c++");
            } else {
                println!("cargo:rustc-link-lib=stdc++");
                println!("cargo:rustc-link-lib=stdc++fs");
            }
            println!("cargo:rustc-link-lib=pthread");
            println!("cargo:rustc-link-lib=m");
            println!("cargo:rustc-link-lib=dl");
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

    fn download_and_extract(target_dir: &Path, url: &str, name: &str) -> PathBuf {
        use reqwest::blocking::Client;
        use zip::ZipArchive;

        fs::create_dir_all(target_dir).expect("Failed to create target directory");

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        println!("cargo:warning=Downloading {} from {}", name, url);
        let max_attempts = 5;
        let mut response = None;

        for attempt in 1..=max_attempts {
            let err_msg = match client.get(url).send() {
                Ok(resp) if resp.status().is_success() => {
                    response = Some(resp);
                    break;
                }
                Ok(resp) => format!("HTTP {}", resp.status()),
                Err(err) => err.to_string(),
            };

            if attempt == max_attempts {
                panic!(
                    "Failed to download {} after {} attempts: {}",
                    name, max_attempts, err_msg
                );
            }

            let backoff = 2u64.pow((attempt - 1).min(4));
            println!(
                "cargo:warning=Download attempt {}/{} for {} failed ({}). Retrying in {}s...",
                attempt, max_attempts, name, err_msg, backoff
            );
            std::thread::sleep(std::time::Duration::from_secs(backoff));
        }

        let mut response = response.expect("unreachable: download loop must either succeed or panic");

        let mut content = Vec::new();
        response.copy_to(&mut content).expect("Failed to read archive content");

        println!("cargo:warning=Downloaded {} bytes for {}", content.len(), name);

        let temp_file = target_dir.join(format!("{}.zip", name));
        fs::write(&temp_file, content).expect("Failed to write archive to file");

        let extract_dir = target_dir.join(name);
        if extract_dir.exists() {
            fs::remove_dir_all(&extract_dir).expect("Failed to remove existing directory");
        }
        fs::create_dir_all(&extract_dir).expect("Failed to create extraction directory");

        let mut archive = ZipArchive::new(fs::File::open(&temp_file).unwrap()).unwrap();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let file_path = file.mangled_name();
            let file_path = file_path.to_str().unwrap();

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

    fn normalize_cmake_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
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
        let is_windows = target_triple.contains("windows");
        let is_windows_gnu = is_windows && target_env == "gnu";

        let lib_name = if is_windows && !is_windows_gnu {
            format!("{}.lib", name)
        } else {
            format!("lib{}.a", name)
        };

        let cached_path = cache_dir.join(&lib_name);
        let marker_path = cache_dir.join(format!("{}.target", name));
        let out_path = install_dir.join("lib").join(&lib_name);

        let possible_lib_names: Vec<String> = if is_windows {
            let mut base = match name {
                "leptonica" => vec![
                    "leptonica.lib".to_string(),
                    "libleptonica.lib".to_string(),
                    "leptonica-static.lib".to_string(),
                    "leptonica-1.84.1.lib".to_string(),
                    "leptonica-1.86.0.lib".to_string(),
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
                        let _ = fs::remove_file(&cached_path);
                        false
                    }
                }
            };

        let link_name_to_use = if cache_valid {
            println!("cargo:warning=Using cached {} library for {}", name, target_triple);
            if let Err(e) = fs::copy(&cached_path, &out_path) {
                println!("cargo:warning=Failed to copy cached library: {}", e);
                build_fn();
            }
            name.to_string()
        } else {
            println!("Building {} library", name);
            build_fn();

            let mut found_lib_name = None;
            'search: for lib_name in &possible_lib_names {
                for dir in &candidate_lib_dirs {
                    let lib_path = dir.join(lib_name);
                    if lib_path.exists() {
                        println!("cargo:warning=Found {} library at: {}", name, lib_path.display());
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
                    println!("cargo:warning=Failed to copy library to standard location: {}", e);
                }
                if let Err(e) = fs::copy(&lib_path, &cached_path) {
                    println!("cargo:warning=Failed to cache library: {}", e);
                } else if let Err(e) = fs::write(&marker_path, &target_triple) {
                    println!("cargo:warning=Failed to write cache marker: {}", e);
                } else {
                    println!("cargo:warning=Cached {} library for {}", name, target_triple);
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
                name.to_string()
            }
        };

        for dir in candidate_lib_dirs.iter().filter(|d| d.exists()) {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }

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
        println!("cargo:warning=Using dynamic linking with system-installed Tesseract libraries");
        println!("cargo:rustc-link-lib=dylib=tesseract");
        println!("cargo:rustc-link-lib=dylib=leptonica");
    }
}
