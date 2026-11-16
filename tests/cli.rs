use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mithqal"))
        .args(arguments)
        .output()
        .unwrap()
}

fn run_stdin(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mithqal"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn one_shot_evaluation_prints_infix() {
    let output = run(&["--eval", "integrate(x^2,0,5)"]);
    assert!(output.status.success());
    assert_eq!(stdout(&output), "125/3\n");
}

#[test]
fn license_notice_lists_native_and_rust_dependencies() {
    let output = run(&["--license"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("GNU GPL version 3.0"));
    assert!(text.contains("Faris Alfarhan"));
    assert!(text.contains("MPFR"));
    assert!(text.contains("rustyline MIT"));
}

#[test]
fn help_lists_the_public_options() {
    let output = run(&["--help"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("--eval"));
    assert!(text.contains("--license"));
}

#[test]
fn repl_keeps_definitions_and_recovers_from_errors() {
    let output = run_stdin("x = 5\nx^2\nintegrate(x^2,0,5)\n1/0\n2+3\nquit\n");
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("25"), "{text}");
    assert!(text.contains("5"), "{text}");
    let errors = String::from_utf8(output.stderr.clone()).unwrap();
    assert!(errors.contains("error"), "{errors}");
}
