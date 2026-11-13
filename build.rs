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

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let source = manifest.join("vendor/gmp");

    if !source.join("configure").exists() {
        panic!("missing vendored GMP source at {}", source.display());
    }

    let build = out.join("gmp-build");
    fs::create_dir_all(&build).unwrap();

    let prefix = out.join("gmp");
    let mut configure = Command::new(source.join("configure"));
    configure
        .current_dir(&build)
        .arg("--disable-shared")
        .arg("--enable-static")
        .arg("--enable-fat")
        .arg("--with-pic")
        .arg(format!("--prefix={}", prefix.display()))
        .env("CFLAGS", "-O2 -fPIC -std=gnu11");
    run(&mut configure, "GMP configure");

    let mut make = Command::new("make");
    make.current_dir(&build).arg(format!("-j{}", jobs()));
    run(&mut make, "GMP build");

    let mut install = Command::new("make");
    install.current_dir(&build).arg("install");
    run(&mut install, "GMP install");

    let libdir = prefix.join("lib");
    println!("cargo:rustc-link-search=native={}", libdir.display());
    println!("cargo:rustc-link-lib=static=gmp");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", source.display());

    let _ = Path::new(&libdir);
}
