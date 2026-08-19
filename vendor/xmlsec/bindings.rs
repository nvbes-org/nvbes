//!
//! XmlSec Bindings Generation
//!
use bindgen::Builder   as BindgenBuilder;
use bindgen::Formatter as BindgenFormatter;

use pkg_config::Config as PkgConfig;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;


const BINDINGS: &str = "bindings.rs";


fn main()
{
    native_toolchain::enforce_native_dependency_hardening();

    println!("cargo:rustc-check-cfg=cfg(xmlsec_key_load_ex)");
    println!("cargo:rustc-link-lib=xmlsec1-openssl");  // -lxmlsec1-openssl
    println!("cargo:rustc-link-lib=xmlsec1");          // -lxmlsec1
    println!("cargo:rustc-link-lib=xml2");             // -lxml2
    println!("cargo:rustc-link-lib=ssl");              // -lssl
    println!("cargo:rustc-link-lib=crypto");           // -lcrypto

    let path_out      = PathBuf::from(env::var("OUT_DIR").unwrap());
    let path_bindings = path_out.join(BINDINGS);

    if !path_bindings.exists()
    {
        if env::var_os("LIBCLANG_PATH").is_none() {
            for llvm_dir in [
                "/usr/lib/llvm-19/lib",
                "/usr/lib/llvm-18/lib",
                "/usr/lib/llvm-17/lib",
                "/usr/lib/llvm-16/lib",
                "/usr/lib/llvm-15/lib",
                "/usr/lib/llvm-14/lib",
                "/usr/lib/aarch64-linux-gnu",
                "/usr/lib/x86_64-linux-gnu",
                "/usr/local/opt/llvm/lib",
                "/opt/homebrew/opt/llvm/lib",
            ] {
                let path = std::path::Path::new(llvm_dir);
                if path.exists() {
                    if let Ok(entries) = fs::read_dir(path) {
                        if entries.filter_map(|e| e.ok()).any(|e| {
                            let name = e.file_name();
                            let s = name.to_string_lossy();
                            s.starts_with("libclang.") || s.starts_with("libclang-")
                        }) {
                            unsafe {
                                env::set_var("LIBCLANG_PATH", llvm_dir);
                            }
                            break;
                        }
                    }
                }
            }
        }

        PkgConfig::new()
            .probe("xmlsec1")
            .expect("Could not find xmlsec1 using pkg-config");

        let bindbuild = BindgenBuilder::default()
            .header("bindings.h")
            .clang_args(fetch_xmlsec_config_flags())
            .clang_args(fetch_xmlsec_config_libs())
            .layout_tests(true)
            .formatter(BindgenFormatter::default())
            .generate_comments(true);

        let bindings = bindbuild.generate()
            .expect("Unable to generate bindings");

        bindings.write_to_file(&path_bindings)
            .expect("Couldn't write bindings!");
    }

    let bindings = fs::read_to_string(&path_bindings)
        .expect("Couldn't read generated bindings!");

    if bindings.contains("xmlSecOpenSSLAppKeyLoadEx") {
        println!("cargo:rustc-cfg=xmlsec_key_load_ex");
    }
}


fn fetch_xmlsec_config_flags() -> Vec<String>
{
    let out = Command::new("xmlsec1-config")
        .arg("--cflags")
        .output()
        .expect("Failed to get --cflags from xmlsec1-config. Is xmlsec1 installed?")
        .stdout;

    args_from_output(out)
}


fn fetch_xmlsec_config_libs() -> Vec<String>
{
    let out = Command::new("xmlsec1-config")
        .arg("--libs")
        .output()
        .expect("Failed to get --libs from xmlsec1-config. Is xmlsec1 installed?")
        .stdout;

    args_from_output(out)
}


fn args_from_output(args: Vec<u8>) -> Vec<String>
{
    let decoded = String::from_utf8(args)
        .expect("Got invalid UTF8 from xmlsec1-config");

    let args = decoded.split_whitespace()
        .map(|p| p.to_owned())
        .collect::<Vec<String>>();

    args
}


mod native_toolchain {
    use std::env;

    const REQUIRED_C_FLAGS: &[&str] = &[
        "-Wall",
        "-Wextra",
        "-Wformat=2",
        "-Wformat-security",
        "-Werror=format-security",
        "-D_FORTIFY_SOURCE=3",
        "-fstack-protector-strong",
        "-fPIE",
        "-fno-omit-frame-pointer",
    ];

    const REQUIRED_LINUX_LD_FLAGS: &[&str] = &[
        "-pie",
        "-Wl,-z,relro",
        "-Wl,-z,now",
        "-Wl,-z,noexecstack",
    ];

    const REQUIRED_MACOS_LD_FLAGS: &[&str] = &[
        "-Wl,-dead_strip",
    ];

    pub fn enforce_native_dependency_hardening() {
        println!("cargo:rerun-if-env-changed=NVBES_C_TOOLCHAIN_HARDENING");
        println!("cargo:rerun-if-env-changed=CFLAGS");
        println!("cargo:rerun-if-env-changed=CXXFLAGS");
        println!("cargo:rerun-if-env-changed=LDFLAGS");

        if env::var("NVBES_C_TOOLCHAIN_HARDENING").as_deref() != Ok("required") {
            return;
        }

        require_flags("CFLAGS", &env::var("CFLAGS").unwrap_or_default(), REQUIRED_C_FLAGS);
        require_flags(
            "CXXFLAGS",
            &env::var("CXXFLAGS").unwrap_or_default(),
            REQUIRED_C_FLAGS,
        );

        let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
        let required_ld_flags = match target_os.as_str() {
            "linux" => REQUIRED_LINUX_LD_FLAGS,
            "macos" => REQUIRED_MACOS_LD_FLAGS,
            _ => &[][..],
        };
        require_flags(
            "LDFLAGS",
            &env::var("LDFLAGS").unwrap_or_default(),
            required_ld_flags,
        );
    }

    fn require_flags(variable: &str, actual: &str, expected: &[&str]) {
        for flag in expected {
            if !actual.split_whitespace().any(|value| value == *flag) {
                panic!(
                    "{variable} is missing required native toolchain hardening flag `{flag}`. \
                     Source scripts/c-toolchain-hardened-env.sh before building native dependencies."
                );
            }
        }
    }
}
