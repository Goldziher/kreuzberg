#![allow(clippy::uninlined_format_args)]

#[cfg(feature = "build-tesseract")]
mod build_tesseract {
    use cmake::Config;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};

    const LEPTONICA_VERSION: &str = "1.86.0";
    const TESSERACT_VERSION: &str = "5.5.1";
    #[allow(dead_code)]
    const EMSDK_COMMIT: &str = "974d5c096bd56e42045d052a47b28198ce75e2a8";
    #[allow(dead_code)]
    const EMSDK_VERSION: &str = "3.1.31";
    #[allow(dead_code)]
    const EMSDK_REPOSITORY: &str = "https://github.com/emscripten-core/emsdk.git";

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

    fn is_wasm_target(target: &str) -> bool {
        target_matches(target, "wasm32") || target_matches(target, "wasm64")
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

    // Detect pre-installed EMSDK (installed via brew, apt, or manually)
    #[allow(dead_code)]
    fn find_emsdk() -> Result<PathBuf, String> {
        // Check EMSDK environment variable first
        if let Ok(emsdk_path) = env::var("EMSDK") {
            let path = PathBuf::from(emsdk_path);
            if path.exists() {
                return Ok(path);
            }
        }

        // Check common installation locations
        let common_paths = vec![
            PathBuf::from("/opt/homebrew/opt/emscripten"), // Homebrew on Apple Silicon
            PathBuf::from("/usr/local/opt/emscripten"),    // Homebrew on Intel Mac
            PathBuf::from(env::var("HOME").unwrap_or_default()).join(".emsdk"),
            PathBuf::from("/usr/lib/emscripten"), // Linux package manager
            PathBuf::from("/opt/emsdk"),          // Manual installation
        ];

        for path in common_paths {
            if path.exists() {
                return Ok(path);
            }
        }

        Err("EMSDK not found. Please install via: brew install emscripten (macOS) or apt-get install emscripten (Linux)".to_string())
    }

    #[allow(dead_code)]
    fn download_emsdk_legacy(emsdk_dir: &Path) -> PathBuf {
        use std::process::Command;

        println!("cargo:warning=Setting up EMSDK (downloading and installing toolchain)...");

        // Create parent directory if it doesn't exist
        if let Some(parent) = emsdk_dir.parent() {
            fs::create_dir_all(parent).expect("Failed to create EMSDK parent directory");
        }

        // Phase 1: Clone the EMSDK repository
        if !emsdk_dir.exists() {
            println!("cargo:warning=Cloning EMSDK repository from {}", EMSDK_REPOSITORY);

            let clone_output = Command::new("git")
                .arg("clone")
                .arg("--depth")
                .arg("1")
                .arg(EMSDK_REPOSITORY)
                .arg(emsdk_dir)
                .output()
                .expect("Failed to run git clone for EMSDK");

            if !clone_output.status.success() {
                panic!(
                    "EMSDK repository clone failed: {}",
                    String::from_utf8_lossy(&clone_output.stderr)
                );
            }

            println!("cargo:warning=EMSDK repository cloned successfully");
        } else {
            println!("cargo:warning=EMSDK directory already exists, skipping clone");
        }

        // Phase 2: Checkout specific commit
        println!("cargo:warning=Checking out EMSDK commit: {}", EMSDK_COMMIT);

        let fetch_output = Command::new("git")
            .current_dir(emsdk_dir)
            .arg("fetch")
            .arg("origin")
            .arg(EMSDK_COMMIT)
            .output()
            .expect("Failed to run git fetch for EMSDK");

        if !fetch_output.status.success() {
            eprintln!(
                "Warning: Failed to fetch specific commit, attempting checkout anyway: {}",
                String::from_utf8_lossy(&fetch_output.stderr)
            );
        }

        let checkout_output = Command::new("git")
            .current_dir(emsdk_dir)
            .arg("checkout")
            .arg(EMSDK_COMMIT)
            .output()
            .expect("Failed to run git checkout for EMSDK");

        if !checkout_output.status.success() {
            eprintln!(
                "Warning: Failed to checkout specific commit: {}",
                String::from_utf8_lossy(&checkout_output.stderr)
            );
        }

        println!("cargo:warning=EMSDK checked out to commit: {}", EMSDK_COMMIT);

        // Phase 3: Make emsdk script executable and patch SSL verification (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let emsdk_script = emsdk_dir.join("emsdk");
            if emsdk_script.exists() {
                let perms = fs::Permissions::from_mode(0o755);
                fs::set_permissions(&emsdk_script, perms)
                    .expect("Failed to set executable permissions on emsdk script");
            }

            // Patch the emsdk.py Python script to disable SSL verification
            let emsdk_py = emsdk_dir.join("emsdk.py");
            if emsdk_py.exists() {
                let script_content = fs::read_to_string(&emsdk_py).expect("Failed to read emsdk.py");

                if !script_content.contains("ssl._create_unverified_context") {
                    let patched_content = script_content.replace(
                        "#!/usr/bin/env python",
                        "#!/usr/bin/env python\nimport ssl\nssl._create_default_https_context = ssl._create_unverified_context"
                    );
                    fs::write(&emsdk_py, patched_content).expect("Failed to write patched emsdk.py");
                    println!("cargo:warning=Patched emsdk.py to disable SSL verification");
                }
            }
        }

        // Phase 4: Install the EMSDK toolchain (use system Node to avoid SSL download issues)
        println!("cargo:warning=Installing EMSDK toolchain (this may take several minutes)...");

        // Find system Node.js
        let node_path = Command::new("which")
            .arg("node")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "node".to_string());

        println!("cargo:warning=Using system Node.js: {}", node_path);

        // Try to install with system Node - allow partial failure if only Node download fails
        #[cfg(unix)]
        let install_output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                "./emsdk install {} 2>&1 | tee /tmp/emsdk-install.log || true",
                EMSDK_VERSION
            ))
            .current_dir(emsdk_dir)
            .env("NODE", &node_path)
            .env("EM_NODE_JS", &node_path)
            .output()
            .expect("Failed to run emsdk install");

        #[cfg(windows)]
        let install_output = Command::new("cmd")
            .args(&["/C", "emsdk.bat", "install", EMSDK_VERSION])
            .current_dir(emsdk_dir)
            .env("NODE", &node_path)
            .env("EM_NODE_JS", &node_path)
            .output()
            .expect("Failed to run emsdk install");

        // Check if install succeeded or at least got the compiler (Node download may have failed)
        let install_stderr = String::from_utf8_lossy(&install_output.stderr);
        let critical_failure =
            !install_output.status.success() && !install_stderr.contains("node") && !install_stderr.contains("Node");

        if critical_failure {
            panic!("EMSDK installation failed: {}", install_stderr);
        }

        if !install_output.status.success() {
            println!(
                "cargo:warning=EMSDK install had warnings (possibly Node download failed, will use system Node): {}",
                install_stderr
            );
        } else {
            println!("cargo:warning=EMSDK toolchain installed successfully");
        }

        // Phase 5: Activate the installed toolchain
        println!("cargo:warning=Activating EMSDK toolchain...");

        #[cfg(unix)]
        let activate_output = Command::new("sh")
            .arg("-c")
            .arg(format!("./emsdk activate {}", EMSDK_VERSION))
            .current_dir(emsdk_dir)
            .output()
            .expect("Failed to run emsdk activate");

        #[cfg(windows)]
        let activate_output = Command::new("cmd")
            .args(&["/C", "emsdk.bat", "activate", EMSDK_VERSION])
            .current_dir(emsdk_dir)
            .output()
            .expect("Failed to run emsdk activate");

        if !activate_output.status.success() {
            panic!(
                "EMSDK activation failed: {}",
                String::from_utf8_lossy(&activate_output.stderr)
            );
        }

        println!("cargo:warning=EMSDK activated successfully");

        // Phase 6: Verify installation (ISSUE #4: Version-aware path checking for EMSDK v3.1.31)
        println!("cargo:warning=Verifying EMSDK installation...");

        // Check both old and new EMSDK directory structures
        let possible_toolchain_paths = vec![
            "upstream/emscripten/cmake/Modules/Platform/Emscripten.cmake",
            "emscripten/cmake/Modules/Platform/Emscripten.cmake", // Older structure
        ];

        let possible_emcc_paths = vec![
            "upstream/emscripten/emcc",
            "emscripten/emcc", // Older structure
        ];

        let toolchain_file = possible_toolchain_paths
            .iter()
            .find(|p| emsdk_dir.join(p).exists())
            .map(|p| emsdk_dir.join(p))
            .unwrap_or_else(|| emsdk_dir.join("upstream/emscripten/cmake/Modules/Platform/Emscripten.cmake"));

        let emcc_path = possible_emcc_paths
            .iter()
            .find(|p| emsdk_dir.join(p).exists())
            .map(|p| emsdk_dir.join(p))
            .unwrap_or_else(|| emsdk_dir.join("upstream/emscripten/emcc"));

        if !toolchain_file.exists() {
            panic!(
                "EMSDK verification failed: toolchain file not found. Checked paths: {:?}",
                possible_toolchain_paths
            );
        }

        if !emcc_path.exists() {
            panic!(
                "EMSDK verification failed: emcc not found. Checked paths: {:?}",
                possible_emcc_paths
            );
        }

        println!("cargo:warning=EMSDK verification completed successfully");
        println!("cargo:warning=EMSDK root directory: {}", emsdk_dir.display());
        println!("cargo:warning=EMSDK toolchain: {}", toolchain_file.display());
        println!("cargo:warning=EMSDK emcc: {}", emcc_path.display());

        emsdk_dir.to_path_buf()
    }

    pub fn build() {
        let target = target_triple();

        // Route WASM targets to WASM build
        if is_wasm_target(&target) {
            println!(
                "cargo:warning=Detected WASM target: {}, routing to build_wasm()",
                target
            );
            return build_wasm();
        }

        let custom_out_dir = prepare_out_dir();
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

    #[allow(dead_code)]
    fn apply_patches(src_dir: &Path, patch_dir: &Path) -> std::io::Result<()> {
        use std::process::Command;

        let patch_file = patch_dir.join("tesseract.diff");
        if !patch_file.exists() {
            println!("cargo:warning=Patch file not found: {}", patch_file.display());
            return Ok(());
        }

        println!("cargo:warning=Applying patches from: {}", patch_file.display());

        let output = Command::new("patch")
            .arg("-p1")
            .current_dir(src_dir)
            .stdin(fs::File::open(&patch_file)?)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("cargo:warning=Patch output: {}", stderr);
            // Don't fail on patch errors, they might be already applied
        }

        Ok(())
    }

    // ISSUE #5: Setup EMSDK environment variables before CMake builds
    #[allow(dead_code)]
    fn setup_emsdk_environment(emsdk_root: &Path) {
        let emsdk_root_str = emsdk_root.to_string_lossy().to_string();

        // Set EMSDK root directory
        unsafe {
            env::set_var("EMSDK", &emsdk_root_str);
        }
        println!("cargo:warning=Set EMSDK={}", emsdk_root_str);

        // Set EMSCRIPTEN path for Emscripten compiler directory
        let emscripten_path = format!("{}/upstream/emscripten", emsdk_root_str);
        unsafe {
            env::set_var("EMSCRIPTEN", &emscripten_path);
        }
        println!("cargo:warning=Set EMSCRIPTEN={}", emscripten_path);

        // Set EM_CONFIG for .emscripten config file
        let em_config = format!("{}/.emscripten", emsdk_root_str);
        unsafe {
            env::set_var("EM_CONFIG", &em_config);
        }
        println!("cargo:warning=Set EM_CONFIG={}", em_config);

        // Update PATH with EMSDK tools directory
        let emsdk_bin_paths = [
            format!("{}/upstream/emscripten", emsdk_root_str),
            format!("{}/upstream/bin", emsdk_root_str),
        ];

        let current_path = env::var("PATH").unwrap_or_default();
        let new_path = format!(
            "{}{}{}",
            emsdk_bin_paths.join(":"),
            if current_path.is_empty() { "" } else { ":" },
            current_path
        );
        unsafe {
            env::set_var("PATH", &new_path);
        }
        println!("cargo:warning=Updated PATH with EMSDK tools");
    }

    // ISSUE #7: Build Leptonica for WASM
    #[allow(dead_code)]
    fn build_leptonica_wasm(leptonica_src: &Path, leptonica_install: &Path, emsdk_dir: &Path) {
        let toolchain_file = emsdk_dir.join("upstream/emscripten/cmake/Modules/Platform/Emscripten.cmake");

        let mut config = Config::new(leptonica_src);

        // Set the EMSDK toolchain file
        config.define("CMAKE_TOOLCHAIN_FILE", &toolchain_file);

        // Image format support disabled (handled by browser)
        config
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            .define("LIBWEBP_SUPPORT", "OFF")
            .define("OPENJPEG_SUPPORT", "OFF")
            .define("ENABLE_ZLIB", "OFF")
            .define("ENABLE_PNG", "OFF")
            .define("ENABLE_JPEG", "OFF")
            .define("ENABLE_TIFF", "OFF")
            .define("ENABLE_WEBP", "OFF")
            .define("ENABLE_OPENJPEG", "OFF")
            .define("ENABLE_GIF", "OFF")
            // Build configuration
            .define("BUILD_PROG", "OFF")
            .define("BUILD_SHARED_LIBS", "OFF")
            .define("NO_CONSOLE_IO", "ON")
            .define("HAVE_LIBZ", "0")
            .define("ENABLE_LTO", "OFF")
            .define("CMAKE_INSTALL_PREFIX", leptonica_install);

        config.build();
    }

    // ISSUE #7: Implement build_wasm() with proper Leptonica and Tesseract integration
    #[allow(dead_code)]
    fn build_wasm() {
        println!("cargo:warning=Building for WASM target with Emscripten SDK");

        let custom_out_dir = prepare_out_dir();
        let cache_dir = custom_out_dir.join("cache");
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");

        let project_dir = custom_out_dir.clone();
        let third_party_dir = project_dir.join("third_party");

        // Step 1: Find pre-installed EMSDK
        println!("cargo:warning=Looking for pre-installed Emscripten SDK...");
        let emsdk_dir = match find_emsdk() {
            Ok(path) => {
                println!("cargo:warning=Found EMSDK at: {}", path.display());
                path
            }
            Err(err) => {
                panic!("{}

Installation instructions:
  macOS:    brew install emscripten
  Ubuntu:   sudo apt-get install emscripten
  Arch:     sudo pacman -S emscripten
  Manual:   git clone https://github.com/emscripten-core/emsdk.git ~/.emsdk && cd ~/.emsdk && ./emsdk install latest && ./emsdk activate latest

After installation, set EMSDK environment variable or ensure it's in a standard location.", err);
            }
        };

        // Setup EMSDK environment variables
        setup_emsdk_environment(&emsdk_dir);

        // Step 2: Download Leptonica and Tesseract sources if not already present
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

        // Step 3: Build Leptonica for WASM
        let leptonica_install_dir = custom_out_dir.join("leptonica");
        let leptonica_cache_dir = cache_dir.join("leptonica");

        build_or_use_cached("leptonica", &leptonica_cache_dir, &leptonica_install_dir, || {
            println!("cargo:warning=Building Leptonica for WASM...");
            build_leptonica_wasm(&leptonica_dir, &leptonica_install_dir, &emsdk_dir);
        });

        // Step 4: Build Tesseract for WASM (SIMD version for modern browsers)
        let tesseract_install_dir = custom_out_dir.join("tesseract");
        let tesseract_cache_dir = cache_dir.join("tesseract");

        build_or_use_cached("tesseract", &tesseract_cache_dir, &tesseract_install_dir, || {
            println!("cargo:warning=Building Tesseract for WASM (SIMD enabled)...");
            build_tesseract_wasm(
                &tesseract_dir,
                &tesseract_install_dir,
                &leptonica_install_dir,
                &emsdk_dir,
                true, // enable_simd
            );
        });

        // Step 5: Setup linker paths for WASM libraries (static linking)
        let leptonica_lib_dir = leptonica_install_dir.join("lib");
        let tesseract_lib_dir = tesseract_install_dir.join("lib");

        println!("cargo:rustc-link-search=native={}", leptonica_lib_dir.display());
        println!("cargo:rustc-link-search=native={}", tesseract_lib_dir.display());

        // For WASM, always use static linking
        println!("cargo:rustc-link-lib=static=tesseract");
        println!("cargo:rustc-link-lib=static=leptonica");

        println!("cargo:warning=WASM build completed successfully!");
        println!("cargo:warning=Leptonica install dir: {:?}", leptonica_install_dir);
        println!("cargo:warning=Tesseract install dir: {:?}", tesseract_install_dir);
    }

    #[allow(dead_code)]
    fn build_tesseract_wasm(
        src_dir: &Path,
        tesseract_install: &Path,
        leptonica_install: &Path,
        emsdk_dir: &Path,
        enable_simd: bool,
    ) {
        let toolchain_file = emsdk_dir.join("upstream/emscripten/cmake/Modules/Platform/Emscripten.cmake");

        let mut config = Config::new(src_dir);

        // Set CMake toolchain for Emscripten
        config.define("CMAKE_TOOLCHAIN_FILE", &toolchain_file);

        // Link against WASM Leptonica
        config.define("Leptonica_DIR", leptonica_install);
        config.define("CMAKE_PREFIX_PATH", leptonica_install);

        // ISSUE #6: Complete CMAKE_CXX_FLAGS for WASM with all WASM-specific flags
        let mut cxx_flags = String::from("-DTESSERACT_IMAGEDATA_AS_PIX ");
        if enable_simd {
            cxx_flags.push_str("-msimd128 ");
        }
        cxx_flags.push_str("-fno-exceptions -fPIC -Os");

        // ISSUE #6: Also add CMAKE_C_FLAGS (was completely missing before)
        let c_flags = "-fno-exceptions -fPIC -Os";

        // Tesseract WASM configuration (from EMSDK_TECHNICAL_REFERENCE.md)
        config
            .define("CMAKE_BUILD_TYPE", "Release")
            .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
            // Core Tesseract settings
            .define("BUILD_TESSERACT_BINARY", "OFF")
            .define("BUILD_TRAINING_TOOLS", "OFF")
            .define("INSTALL_CONFIGS", "ON")
            .define("BUILD_TESTS", "OFF")
            .define("BUILD_PROG", "OFF")
            .define("SYNTAX_LOG", "OFF")
            // Disable unnecessary features
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
            // WASM-specific settings
            .define("GRAPHICS_DISABLED", "ON")
            .define("DISABLED_LEGACY_ENGINE", "ON")
            .define("USE_OPENCL", "OFF")
            .define("OPENMP_BUILD", "OFF")
            .define("ENABLE_LTO", "ON")
            // SIMD configuration
            .define("HAVE_SSE4_1", if enable_simd { "ON" } else { "OFF" })
            .define("HAVE_AVX", "OFF")
            .define("HAVE_AVX2", "OFF")
            .define("HAVE_AVX512F", "OFF")
            .define("HAVE_FMA", "OFF")
            // Installation directory
            .define("CMAKE_INSTALL_PREFIX", tesseract_install)
            // Compiler flags with WASM-specific settings
            .define("CMAKE_CXX_FLAGS", &cxx_flags)
            .define("CMAKE_C_FLAGS", c_flags);

        // Build the library
        config.build();
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
