mod features;
#[cfg(feature = "stable-api")]
mod stable_api_config;
mod version;

use features::*;
use rb_sys_build::{bindings, RbConfig, RubyEngine};
use std::io::Write;
use std::{
    env,
    fs::{self, File},
    path::PathBuf,
};
use version::Version;

const SUPPORTED_RUBY_VERSIONS: [Version; 10] = [
    Version::new(2, 3),
    Version::new(2, 4),
    Version::new(2, 5),
    Version::new(2, 6),
    Version::new(2, 7),
    Version::new(3, 0),
    Version::new(3, 1),
    Version::new(3, 2),
    Version::new(3, 3),
    Version::new(3, 4),
];

fn main() {
    warn_deprecated_feature_flags();

    let mut rbconfig = RbConfig::current();
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cfg_capture_path = out_dir.join(format!("cfg-capture-{}", rbconfig.ruby_version_slug()));
    let mut cfg_capture_file = File::create(cfg_capture_path).expect("create cfg capture file");

    println!("cargo:rerun-if-env-changed=RUBY_ROOT");
    println!("cargo:rerun-if-env-changed=RUBY_VERSION");
    println!("cargo:rerun-if-env-changed=RUBY");

    for file in fs::read_dir("build").unwrap() {
        println!("cargo:rerun-if-changed={}", file.unwrap().path().display());
    }

    let bindings_path = bindings::generate(
        &rbconfig,
        is_ruby_static_enabled(&rbconfig),
        &mut cfg_capture_file,
    )
    .expect("generate bindings");
    println!("Bindings generated at: {}", bindings_path.display());
    println!(
        "cargo:rustc-env=RB_SYS_BINDINGS_PATH={}",
        bindings_path.display()
    );
    export_cargo_cfg(&mut rbconfig, &mut cfg_capture_file);

    #[cfg(feature = "stable-api")]
    if let Err(e) = stable_api_config::setup(&rbconfig) {
        eprintln!("Failed to setup stable API: {}", e);
        std::process::exit(1);
    }

    if is_link_ruby_enabled() {
        link_libruby(&mut rbconfig);
    } else {
        add_libruby_to_blocklist(&mut rbconfig);
        enable_dynamic_lookup(&mut rbconfig);
    }

    expose_cargo_features(&mut cfg_capture_file);
    add_unsupported_link_args_to_blocklist(&mut rbconfig);
    rbconfig.print_cargo_args();

    if is_debug_build_enabled() {
        debug_and_exit(&mut rbconfig);
    }
}

macro_rules! cfg_capture {
    ($file:expr, $fmt:expr, $($arg:tt)*) => {
        println!($fmt, $($arg)*);
        writeln!($file, $fmt, $($arg)*).unwrap();
    };
}

macro_rules! cfg_capture_opt {
    ($file:expr, $fmt:expr, $opt:expr) => {
        if let Some(val) = $opt {
            cfg_capture!($file, $fmt, val);
        }
    };
}

fn add_unsupported_link_args_to_blocklist(rbconfig: &mut RbConfig) {
    rbconfig.blocklist_link_arg("-Wl,--compress-debug-sections=zlib");
    rbconfig.blocklist_link_arg("-s");
}

fn add_libruby_to_blocklist(rbconfig: &mut RbConfig) {
    rbconfig.blocklist_lib(&rbconfig.libruby_so_name());
    rbconfig.blocklist_lib(&rbconfig.libruby_static_name());
}

fn debug_and_exit(rbconfig: &mut RbConfig) {
    eprintln!("========== RbConfig\n");
    dbg!(rbconfig);

    eprintln!("========== Environment Variables\n");
    let env: std::collections::HashMap<_, _> = std::env::vars().collect();
    dbg!(env);

    eprintln!("==========\n");
    eprintln!("The \"RB_SYS_DEBUG_BUILD\" env var was set, aborting.");
    std::process::exit(1);
}

fn link_libruby(rbconfig: &mut RbConfig) {
    if is_link_ruby_enabled() {
        rbconfig.link_ruby(is_ruby_static_enabled(rbconfig));
    }
}

fn export_cargo_cfg(rbconfig: &mut RbConfig, cap: &mut File) {
    rustc_cfg(rbconfig, "ruby_major", "MAJOR");
    rustc_cfg(rbconfig, "ruby_minor", "MINOR");
    rustc_cfg(rbconfig, "ruby_teeny", "TEENY");
    rustc_cfg(rbconfig, "ruby_patchlevel", "PATCHLEVEL");
    rustc_cfg(rbconfig, "ruby_api_version", "RUBY_API_VERSION");

    println!("cargo:rustc-check-cfg=cfg(use_global_allocator)");
    if is_global_allocator_enabled(rbconfig) {
        println!("cargo:rustc-cfg=use_global_allocator");
    }

    println!("cargo:rustc-check-cfg=cfg(has_ruby_abi_version)");
    if rbconfig.has_ruby_dln_check_abi() {
        println!("cargo:rustc-cfg=has_ruby_abi_version");
    }

    println!("cargo:rustc-check-cfg=cfg(ruby_engine, values(\"mri\", \"truffleruby\"))");
    match rbconfig.ruby_engine() {
        RubyEngine::Mri => {
            println!("cargo:rustc-cfg=ruby_engine=\"mri\"");
        }
        RubyEngine::TruffleRuby => {
            println!("cargo:rustc-cfg=ruby_engine=\"truffleruby\"");
        }
        _ => panic!("unsupported ruby engine"),
    }

    let version = Version::current(rbconfig);

    for v in SUPPORTED_RUBY_VERSIONS.iter() {
        let v = v.to_owned();

        println!(
            "cargo:rustc-check-cfg=cfg(ruby_lt_{}_{})",
            v.major(),
            v.minor()
        );
        if version < v {
            println!(r#"cargo:rustc-cfg=ruby_lt_{}_{}"#, v.major(), v.minor());
            cfg_capture!(cap, r#"cargo:version_lt_{}_{}=true"#, v.major(), v.minor());
        } else {
            cfg_capture!(cap, r#"cargo:version_lt_{}_{}=false"#, v.major(), v.minor());
        }

        println!(
            "cargo:rustc-check-cfg=cfg(ruby_lte_{}_{})",
            v.major(),
            v.minor()
        );
        if version <= v {
            println!(r#"cargo:rustc-cfg=ruby_lte_{}_{}"#, v.major(), v.minor());
            cfg_capture!(cap, r#"cargo:version_lte_{}_{}=true"#, v.major(), v.minor());
        } else {
            cfg_capture!(
                cap,
                r#"cargo:version_lte_{}_{}=false"#,
                v.major(),
                v.minor()
            );
        }

        println!(
            "cargo:rustc-check-cfg=cfg(ruby_eq_{}_{})",
            v.major(),
            v.minor()
        );
        if version == v {
            println!(r#"cargo:rustc-cfg=ruby_eq_{}_{}"#, v.major(), v.minor());
            cfg_capture!(cap, r#"cargo:version_eq_{}_{}=true"#, v.major(), v.minor());
        } else {
            cfg_capture!(cap, r#"cargo:version_eq_{}_{}=false"#, v.major(), v.minor());
        }

        println!(
            "cargo:rustc-check-cfg=cfg(ruby_gte_{}_{})",
            v.major(),
            v.minor()
        );
        if version >= v {
            println!(r#"cargo:rustc-cfg=ruby_gte_{}_{}"#, v.major(), v.minor());
            cfg_capture!(cap, r#"cargo:version_gte_{}_{}=true"#, v.major(), v.minor());
        } else {
            cfg_capture!(
                cap,
                r#"cargo:version_gte_{}_{}=false"#,
                v.major(),
                v.minor()
            );
        }

        println!(
            "cargo:rustc-check-cfg=cfg(ruby_gt_{}_{})",
            v.major(),
            v.minor()
        );
        if version > v {
            println!(r#"cargo:rustc-cfg=ruby_gt_{}_{}"#, v.major(), v.minor());
            cfg_capture!(cap, r#"cargo:version_gt_{}_{}=true"#, v.major(), v.minor());
        } else {
            cfg_capture!(cap, r#"cargo:version_gt_{}_{}=false"#, v.major(), v.minor());
        }
    }

    cfg_capture_opt!(cap, "cargo:root={}", rbconfig.get("prefix"));
    cfg_capture_opt!(cap, "cargo:include={}", rbconfig.get("includedir"));
    cfg_capture_opt!(cap, "cargo:archinclude={}", rbconfig.get("archincludedir"));
    cfg_capture_opt!(cap, "cargo:libdir={}", rbconfig.get("libdir"));
    cfg_capture_opt!(cap, "cargo:version={}", rbconfig.get("ruby_version"));
    cfg_capture_opt!(cap, "cargo:major={}", rbconfig.get("MAJOR"));
    cfg_capture_opt!(cap, "cargo:minor={}", rbconfig.get("MINOR"));
    cfg_capture_opt!(cap, "cargo:teeny={}", rbconfig.get("TEENY"));
    cfg_capture_opt!(cap, "cargo:patchlevel={}", rbconfig.get("PATCHLEVEL"));
    cfg_capture!(cap, "cargo:engine={}", rbconfig.ruby_engine());

    for key in rbconfig.all_keys() {
        cfg_capture!(
            cap,
            "cargo:rbconfig_{}={}",
            key,
            rbconfig.get(key).expect("key")
        );
    }

    if is_ruby_static_enabled(rbconfig) {
        cfg_capture!(cap, "cargo:lib={}", rbconfig.libruby_static_name());
        cfg_capture!(cap, "cargo:ruby_static={}", "true");
    } else {
        cfg_capture!(cap, "cargo:lib={}", rbconfig.libruby_so_name());
    }
}

fn rustc_cfg(rbconfig: &RbConfig, name: &str, key: &str) {
    println!("cargo:rustc-check-cfg=cfg({})", name);
    if let Some(k) = rbconfig.get(key) {
        println!("cargo:rustc-cfg={}=\"{}\"", name, k);
    }
}

fn enable_dynamic_lookup(rbconfig: &mut RbConfig) {
    // See https://github.com/oxidize-rb/rb-sys/issues/88
    if cfg!(target_os = "macos") {
        rbconfig.push_dldflags("-Wl,-undefined,dynamic_lookup");
    } else if matches!(rbconfig.ruby_engine(), RubyEngine::TruffleRuby) {
        rbconfig.push_dldflags("-Wl,-z,lazy");
    }
}

fn expose_cargo_features(cap: &mut File) {
    for (key, val) in std::env::vars() {
        if !key.starts_with("CARGO_FEATURE_") {
            continue;
        }

        cfg_capture!(cap, "cargo:{}={}", key.to_lowercase(), val);
    }
}

fn warn_deprecated_feature_flags() {
    if cfg!(feature = "ruby-macros") {
        println!("cargo:warning=The \"ruby-macros\" feature flag is deprecated and will be removed in a future release. Please use \"stable-api\" instead.");
    }
}
