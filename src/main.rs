#[cfg(feature = "cli")]
fn main() {
    let arguments = std::env::args().skip(1).collect();
    std::process::exit(mithqal::cli::run(arguments));
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("mithqal was built without a frontend");
    std::process::exit(2);
}
