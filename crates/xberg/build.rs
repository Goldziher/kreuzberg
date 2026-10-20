fn main() {
    link_ios_compiler_runtime();
    println!("cargo::rustc-check-cfg=cfg(coverage)");

    println!("cargo::rustc-check-cfg=cfg(inference_ort)");
    if std::env::var_os("CARGO_FEATURE_ORT_BUNDLED").is_some()
        || std::env::var_os("CARGO_FEATURE_ORT_DYNAMIC").is_some()
    {
        println!("cargo::rustc-cfg=inference_ort");
    }

    println!("cargo::rustc-check-cfg=cfg(auto_rotate)");
    if std::env::var_os("CARGO_FEATURE_AUTO_ROTATE").is_some()
        || std::env::var_os("CARGO_FEATURE_AUTO_ROTATE_TRACT").is_some()
    {
        println!("cargo::rustc-cfg=auto_rotate");
    }

    println!("cargo::rustc-check-cfg=cfg(sceptre_ocr)");
    if std::env::var_os("CARGO_FEATURE_SCEPTRE_OCR_ORT").is_some()
        || std::env::var_os("CARGO_FEATURE_SCEPTRE_OCR_TRACT").is_some()
        || std::env::var_os("CARGO_FEATURE_SCEPTRE_OCR_CANDLE").is_some()
    {
        println!("cargo::rustc-cfg=sceptre_ocr");
    }

    if std::env::var_os("CARGO_FEATURE_ORT_BUNDLED").is_some()
        && std::env::var_os("CARGO_FEATURE_ORT_DYNAMIC").is_some()
    {
        println!(
            "cargo::warning=features 'ort-bundled' and 'ort-dynamic' are both enabled; bundled ORT remains the default unless dynamic ORT is explicitly selected at runtime"
        );
    }

    println!("cargo::rustc-check-cfg=cfg(layout_detection)");
    if std::env::var_os("CARGO_FEATURE_LAYOUT_DETECTION").is_some()
        || std::env::var_os("CARGO_FEATURE_LAYOUT_TRACT").is_some()
    {
        println!("cargo::rustc-cfg=layout_detection");
    }

    println!("cargo::rustc-check-cfg=cfg(paddle_ocr)");
    if std::env::var_os("CARGO_FEATURE_PADDLE_OCR_ORT").is_some()
        || std::env::var_os("CARGO_FEATURE_PADDLE_OCR_TRACT").is_some()
    {
        println!("cargo::rustc-cfg=paddle_ocr");
    }

    // See `build_id`'s doc comment for what this feeds: `cache::version::cache_version_tag`
    // (crates/xberg/src/cache/version.rs) folds it in alongside the crate version, the cache
    // schema version, and the debug/release profile bit.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        emit_rerun_directives(&manifest_dir);
    }
    println!("cargo::rustc-env=XBERG_BUILD_ID={}", build_id());
}

fn link_ios_compiler_runtime() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("ios")
        || std::env::var_os("CARGO_FEATURE_ORT_BUNDLED").is_none()
    {
        return;
    }
    let (sdk, library) = if std::env::var("CARGO_CFG_TARGET_ABI").as_deref() == Ok("sim") {
        ("iphonesimulator", "clang_rt.iossim")
    } else {
        ("iphoneos", "clang_rt.ios")
    };
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH unset");
    let apple_arch = if arch == "aarch64" { "arm64" } else { &arch };
    let runtime = ios_compiler_runtime(sdk, library);
    let staged = stage_ios_compiler_runtime(sdk, &runtime, apple_arch);
    println!(
        "cargo::rustc-link-search=native={}",
        staged.parent().expect("runtime has no parent").display()
    );
    println!("cargo::rustc-link-lib=static={library}");
    println!("cargo::rerun-if-changed={}", runtime.display());
    for variable in ["DEVELOPER_DIR", "TOOLCHAINS", "SDKROOT"] {
        println!("cargo::rerun-if-env-changed={variable}");
    }
}

fn ios_compiler_runtime(sdk: &str, library: &str) -> std::path::PathBuf {
    let output = std::process::Command::new("xcrun")
        .args(["--sdk", sdk, "clang", "--print-resource-dir"])
        .output()
        .unwrap_or_else(|error| panic!("query {sdk} compiler runtime directory: {error}"));
    assert!(
        output.status.success(),
        "query {sdk} compiler runtime directory ({}): {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let resource = String::from_utf8(output.stdout)
        .unwrap_or_else(|error| panic!("decode {sdk} compiler runtime directory: {error}"));
    assert!(!resource.trim().is_empty(), "empty {sdk} compiler runtime directory");
    std::path::Path::new(resource.trim())
        .join("lib/darwin")
        .join(format!("lib{library}.a"))
}

fn stage_ios_compiler_runtime(sdk: &str, archive: &std::path::Path, arch: &str) -> std::path::PathBuf {
    use std::io::Read;

    let mut header = [0; 8];
    std::fs::File::open(archive)
        .and_then(|mut file| file.read_exact(&mut header))
        .unwrap_or_else(|error| panic!("read {sdk} compiler runtime archive: {error}"));
    let destination = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR unset"))
        .join(archive.file_name().expect("runtime archive has no filename"));
    // Rust bundles native archives into rlibs, but cannot read Xcode's universal archives. ~keep
    if &header == b"!<arch>\n" {
        std::fs::copy(archive, &destination)
            .unwrap_or_else(|error| panic!("copy {sdk} compiler runtime archive: {error}"));
    } else {
        let output = std::process::Command::new("xcrun")
            .arg("lipo")
            .arg(archive)
            .args(["-thin", arch, "-output"])
            .arg(&destination)
            .output()
            .unwrap_or_else(|error| panic!("extract {arch} from {sdk} compiler runtime: {error}"));
        assert!(
            output.status.success(),
            "extract {arch} from {sdk} compiler runtime ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    destination
}

/// Best-effort identifier for *this* build, embedded at compile time via `env!("XBERG_BUILD_ID")`.
///
/// Exists so two separately built binaries at the same crate version and cache schema version
/// get different on-disk cache entries instead of silently serving each other's results (a real
/// defect: see the module doc on `cache::version`). Resolved in this order:
///
/// 1. The `XBERG_BUILD_ID` environment variable, if set to a non-empty value. This is the
///    escape hatch for the two cases the git-based derivation below cannot reach on its own:
///    a Docker build (`.dockerignore` excludes `.git` from every build context in this repo, so
///    `git` has nothing to read there) and a crates.io/docs.rs build from a published source
///    tarball (no `.git` at all). A release pipeline can pass `--build-arg XBERG_BUILD_ID=<sha>`
///    / set the env var to the commit it packaged; a developer running two arms of an A/B
///    measurement from the same dirty worktree can export distinct values by hand to force
///    distinct cache entries instead of remembering to wipe the cache between arms.
/// 2. The current commit SHA (`git rev-parse HEAD`) plus, when the working tree has uncommitted
///    changes, a hash of the tracked diff against `HEAD` and the list of untracked paths. This
///    covers ordinary local development and any CI job that runs the build from a real git
///    checkout: two builds from different commits, or from the same commit with different
///    uncommitted edits, get different ids. Requires `git` on `PATH` and `CARGO_MANIFEST_DIR`
///    to sit inside a git checkout; any failure (missing binary, not a repository, non-UTF-8
///    output) falls through to the next case rather than failing the build.
/// 3. `""` when neither is available. `cache_version_tag` (crates/xberg/src/cache/version.rs)
///    then folds in an empty build id, collapsing this component of the tag back to what it
///    always was before `XBERG_BUILD_ID` existed. For a build from a published crates.io/docs.rs
///    tarball this is not a gap, it is correct: crates.io tarballs are immutable per version, so
///    two such builds of the same version share identical source and there is no "build" left
///    for an id to distinguish.
///
/// Cargo does NOT track environment variables for a build script unless the script says so, and
/// emitting no `rerun-if-*` at all only gets the default "rerun when a file in the package
/// changed" heuristic -- which never notices `XBERG_BUILD_ID`. Measured: two builds with
/// different `XBERG_BUILD_ID` values produced the identical tag `2d0ec67f`, i.e. the override was
/// inert. `emit_rerun_directives` below therefore declares the env var and the git refs
/// explicitly. Note that emitting any `rerun-if-*` instruction REPLACES the default heuristic,
/// so the source tree has to be declared alongside them or an ordinary code edit stops
/// re-running the script.
///
/// Never touches the network and never fails the build; worst case it returns `""`.
/// Declare everything `build_id` reads, because emitting any `rerun-if-*` instruction turns off
/// Cargo's default "any file in the package changed" heuristic.
///
/// `src` and `Cargo.toml` restore that default. `XBERG_BUILD_ID` is the override. `.git/HEAD` and
/// `.git/index` cover committing, switching branch and staging; an unstaged edit is already
/// covered because it changes a file under `src`.
fn emit_rerun_directives(manifest_dir: &str) {
    println!("cargo::rerun-if-env-changed=XBERG_BUILD_ID");
    println!("cargo::rerun-if-changed={manifest_dir}/src");
    println!("cargo::rerun-if-changed={manifest_dir}/Cargo.toml");

    let Some(git_dir) = git_stdout(manifest_dir, &["rev-parse", "--git-dir"]) else {
        return;
    };
    let git_dir = git_dir.trim();
    if git_dir.is_empty() {
        return;
    }
    let base = if std::path::Path::new(git_dir).is_absolute() {
        git_dir.to_string()
    } else {
        format!("{manifest_dir}/{git_dir}")
    };
    println!("cargo::rerun-if-changed={base}/HEAD");
    println!("cargo::rerun-if-changed={base}/index");
}

fn build_id() -> String {
    if let Ok(id) = std::env::var("XBERG_BUILD_ID")
        && !id.is_empty()
    {
        return id;
    }

    let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") else {
        return String::new();
    };

    let Some(sha) = git_stdout(&manifest_dir, &["rev-parse", "HEAD"]) else {
        return String::new();
    };
    let sha = sha.trim();

    // Tracked, uncommitted changes (staged and unstaged) against HEAD.
    let diff = git_stdout(&manifest_dir, &["diff", "HEAD"]).unwrap_or_default();
    // New files git doesn't know about yet: a diff against HEAD can't see these, but an A/B
    // measurement that adds a file without committing it needs a distinct id all the same.
    let untracked = git_stdout(&manifest_dir, &["ls-files", "--others", "--exclude-standard"]).unwrap_or_default();

    if diff.is_empty() && untracked.is_empty() {
        return sha.to_string();
    }

    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    diff.hash(&mut hasher);
    untracked.hash(&mut hasher);
    format!("{sha}-dirty-{:016x}", hasher.finish())
}

/// Run `git <args>` in `dir`, returning its stdout as UTF-8 on success.
///
/// Returns `None` on any failure -- `git` missing, `dir` not inside a repository, a non-zero
/// exit, or non-UTF-8 output -- so callers can fall through to the next id source.
fn git_stdout(dir: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}
