use std::fs;
use std::path::{Path, PathBuf};

use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_source(source).unwrap();
    session.format_infix(value).unwrap()
}

fn write_package(directory: &Path, name: &str, source: &str) -> PathBuf {
    let path = directory.join(name);
    fs::write(&path, source).unwrap();
    path
}

#[test]
fn packages_load_imports_and_keep_private_names_private() {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("mithqal-pkg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let dependency = write_package(
        &directory,
        "pkg_b.mith",
        "mod pkgB\nexport external\ndef external(x)=x*2\nend\n",
    );
    let package = write_package(
        &directory,
        "pkg_a.mith",
        "mod pkgA\nexport public\nimport \"pkg_b.mith\"\ndef public(x)=helper(x)\ndef helper(x)=external(x)+1\nend\n",
    );
    let mut session = Session::new();
    let _ = dependency;
    assert_eq!(
        session.needs(&package.to_string_lossy()).unwrap(),
        session.eval_statement("null").unwrap()
    );
    assert_eq!(eval(&mut session, "public(2)"), "5");
    assert_eq!(eval(&mut session, "helper(2)"), "helper(2)");
    assert_eq!(eval(&mut session, "pkgA;helper(2)"), "5");
    assert_eq!(eval(&mut session, "context(pkgA;helper)"), "\"pkgA\"");
    assert_eq!(eval(&mut session, "length(downValues(public))"), "1");
    let path = eval(&mut session, "$MithqalPath");
    assert!(path.starts_with('['));
    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn package_import_cycles_are_rejected() {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("mithqal-cycle-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let a = write_package(&directory, "a.mith", "mod a\nimport \"b.mith\"\nend\n");
    let _ = write_package(&directory, "b.mith", "mod b\nimport \"a.mith\"\nend\n");
    let mut session = Session::new();
    assert!(session.needs(&a.to_string_lossy()).is_err());
    let _ = fs::remove_dir_all(&directory);
}

#[test]
fn missing_packages_are_rejected() {
    let mut session = Session::new();
    assert!(session.needs("definitely_missing_package").is_err());
}

#[test]
fn loaded_modules_evaluate_assumptions_without_restarting() {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("mithqal-assumption-pkg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let positive = write_package(
        &directory,
        "positive.mith",
        "mod assumptionPositive\nexport positiveX\ndef positiveX()=assume(x>0,abs(x))\nend\n",
    );
    let negative = write_package(
        &directory,
        "negative.mith",
        "mod assumptionNegative\nexport negativeX\ndef negativeX()=assume(x<0,abs(x))\nend\n",
    );
    let mut session = Session::new();
    session.needs(&positive.to_string_lossy()).unwrap();
    assert_eq!(eval(&mut session, "positiveX()"), "x");
    session.needs(&negative.to_string_lossy()).unwrap();
    assert_eq!(eval(&mut session, "negativeX()"), "-x");
    let _ = fs::remove_dir_all(&directory);
}
