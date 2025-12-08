use rb_sys_build::{RbConfig, RubyEngine};

use crate::{
    features::is_env_variable_defined,
    version::{MIN_SUPPORTED_STABLE_VERSION, Version},
};
use std::{convert::TryFrom, error::Error, path::Path};

pub fn setup(rb_config: &RbConfig) -> Result<(), Box<dyn Error>> {
    let ruby_version = Version::current(rb_config);
    let ruby_engine = rb_config.ruby_engine();
    let strategy = Strategy::try_from((ruby_engine, ruby_version))?;

    strategy.apply()?;

    Ok(())
}

#[derive(Debug)]
enum Strategy {
    RustOnly(Version),
    CompiledOnly,
    RustThenCompiled(Version),
    Testing(Version),
}

impl TryFrom<(RubyEngine, Version)> for Strategy {
    type Error = Box<dyn Error>;

    fn try_from((engine, current_ruby_version): (RubyEngine, Version)) -> Result<Self, Self::Error> {
        let mut strategy = None;

        match engine {
            RubyEngine::TruffleRuby => {
                return Ok(Strategy::CompiledOnly);
            }
            RubyEngine::JRuby => {
                return Err("JRuby is not supported".into());
            }
            RubyEngine::Mri => {}
        }

        if current_ruby_version.is_stable() {
            strategy = Some(Strategy::RustOnly(current_ruby_version));
        } else {
            maybe_warn_old_ruby_version(current_ruby_version);
        }

        if is_fallback_enabled() {
            strategy = Some(Strategy::RustThenCompiled(current_ruby_version));
        }

        if is_testing() {
            strategy = Some(Strategy::Testing(current_ruby_version));
        }

        if is_force_enabled() {
            strategy = Some(Strategy::CompiledOnly);
        }

        if let Some(strategy) = strategy {
            return Ok(strategy);
        }

        Err("Stable API is needed but could not find a candidate. Try enabling the `stable-api-compiled-fallback` feature in rb-sys.".into())
    }
}

impl Strategy {
    fn apply(self) -> Result<(), Box<dyn Error>> {
        println!("cargo:rustc-check-cfg=cfg(stable_api_include_rust_impl)");
        println!("cargo:rustc-check-cfg=cfg(stable_api_enable_compiled_mod)");
        println!("cargo:rustc-check-cfg=cfg(stable_api_export_compiled_as_api)");
        println!("cargo:rustc-check-cfg=cfg(stable_api_has_rust_impl)");
        match self {
            Strategy::RustOnly(current_ruby_version) => {
                if current_ruby_version.is_stable() {
                    println!("cargo:rustc-cfg=stable_api_include_rust_impl");
                } else {
                    return Err(format!("A stable Ruby API is needed but could not find a candidate. If you are using a stable version of Ruby, try upgrading rb-sys. Otherwise if you are testing against ruby-head or Ruby < {}, enable the `stable-api-compiled-fallback` feature in rb-sys.", MIN_SUPPORTED_STABLE_VERSION).into());
                }
            }
            Strategy::CompiledOnly => {
                compile()?;
                println!("cargo:rustc-cfg=stable_api_enable_compiled_mod");
                println!("cargo:rustc-cfg=stable_api_export_compiled_as_api");
            }
            Strategy::RustThenCompiled(current_ruby_version) => {
                if current_ruby_version.is_stable() {
                    println!("cargo:rustc-cfg=stable_api_has_rust_impl");
                    println!("cargo:rustc-cfg=stable_api_include_rust_impl");
                } else {
                    compile()?;
                    println!("cargo:rustc-cfg=stable_api_enable_compiled_mod");
                    println!("cargo:rustc-cfg=stable_api_export_compiled_as_api");
                }
            }
            Strategy::Testing(current_ruby_version) => {
                compile()?;

                println!("cargo:rustc-cfg=stable_api_enable_compiled_mod");

                if current_ruby_version.is_stable() {
                    println!("cargo:rustc-cfg=stable_api_include_rust_impl");
                } else {
                    println!("cargo:rustc-cfg=stable_api_export_compiled_as_api");
                }
            }
        };

        Ok(())
    }
}

fn is_fallback_enabled() -> bool {
    println!("cargo:rerun-if-env-changed=RB_SYS_STABLE_API_COMPILED_FALLBACK");

    is_env_variable_defined("CARGO_FEATURE_STABLE_API_COMPILED_FALLBACK")
        || cfg!(rb_sys_use_stable_api_compiled_fallback)
        || is_env_variable_defined("RB_SYS_STABLE_API_COMPILED_FALLBACK")
}

fn is_force_enabled() -> bool {
    println!("cargo:rerun-if-env-changed=RB_SYS_STABLE_API_COMPILED_FORCE");

    is_env_variable_defined("CARGO_FEATURE_STABLE_API_COMPILED_FORCE")
        || cfg!(rb_sys_force_stable_api_compiled)
        || is_env_variable_defined("RB_SYS_STABLE_API_COMPILED_FORCE")
}

fn is_testing() -> bool {
    is_env_variable_defined("CARGO_FEATURE_STABLE_API_COMPILED_TESTING")
}

fn maybe_warn_old_ruby_version(current_ruby_version: Version) {
    if current_ruby_version < MIN_SUPPORTED_STABLE_VERSION {
        println!(
            "cargo:warning=Support for Ruby {} will be removed in a future release.",
            current_ruby_version
        );
    }
}

fn compile() -> Result<(), Box<dyn Error>> {
    eprintln!("INFO: Compiling the stable API compiled module");
    let mut build = rb_sys_build::cc::Build::new();
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = crate_dir.join("src").join("stable_api").join("compiled.c");
    eprintln!("cargo:rerun-if-changed={}", path.display());

    build.file(path);
    build.try_compile("compiled")
}
