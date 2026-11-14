use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn run(command: &mut Command, what: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("failed to start {what}: {error}"));
    if !status.success() {
        panic!("{what} failed with status {status}");
    }
}

fn jobs() -> String {
    std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(2)
        .to_string()
}

fn configure_build_install(
    source: &Path,
    build: &Path,
    prefix: &Path,
    args: &[String],
    what: &str,
) {
    fs::create_dir_all(build).unwrap();
    let mut configure = Command::new(source.join("configure"));
    configure
        .current_dir(build)
        .arg("--disable-shared")
        .arg("--enable-static")
        .arg("--with-pic")
        .arg(format!("--prefix={}", prefix.display()))
        .args(args)
        .env("CFLAGS", "-O2 -fPIC -std=gnu11");
    run(&mut configure, &format!("{what} configure"));

    let mut make = Command::new("make");
    make.current_dir(build).arg(format!("-j{}", jobs()));
    run(&mut make, &format!("{what} build"));

    let mut install = Command::new("make");
    install.current_dir(build).arg("install");
    run(&mut install, &format!("{what} install"));
}

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let gmp_source = manifest.join("vendor/gmp");
    let mpfr_source = manifest.join("vendor/mpfr");
    let mpc_source = manifest.join("vendor/mpc");
    for source in [&gmp_source, &mpfr_source, &mpc_source] {
        if !source.join("configure").exists() {
            panic!("missing vendored source at {}", source.display());
        }
    }

    let gmp_prefix = out.join("gmp");
    let mpfr_prefix = out.join("mpfr");
    let mpc_prefix = out.join("mpc");

    configure_build_install(
        &gmp_source,
        &out.join("gmp-build"),
        &gmp_prefix,
        &["--enable-fat".to_string()],
        "GMP",
    );
    configure_build_install(
        &mpfr_source,
        &out.join("mpfr-build"),
        &mpfr_prefix,
        &[format!("--with-gmp={}", gmp_prefix.display())],
        "MPFR",
    );
    configure_build_install(
        &mpc_source,
        &out.join("mpc-build"),
        &mpc_prefix,
        &[
            format!("--with-gmp={}", gmp_prefix.display()),
            format!("--with-mpfr={}", mpfr_prefix.display()),
        ],
        "MPC",
    );

    for prefix in [&mpc_prefix, &mpfr_prefix, &gmp_prefix] {
        println!(
            "cargo:rustc-link-search=native={}",
            prefix.join("lib").display()
        );
    }
    println!("cargo:rustc-link-lib=static=mpc");
    println!("cargo:rustc-link-lib=static=mpfr");
    println!("cargo:rustc-link-lib=static=gmp");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rerun-if-changed=build.rs");
    for source in [&gmp_source, &mpfr_source, &mpc_source] {
        println!("cargo:rerun-if-changed={}", source.display());
    }
}
