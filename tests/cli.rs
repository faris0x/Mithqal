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
fn license_notice_lists_numeric_and_rust_dependencies() {
    let output = run(&["--license"]);
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("GNU GPL version 3.0"));
    assert!(text.contains("Faris Alfarhan"));
    assert!(text.contains("Malachite"));
    assert!(text.contains("rustyline  MIT"));
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

#[test]
fn capability_listing_exposes_support_metadata() {
    let output = run_stdin("list capabilities gamma\nquit\n");
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("gamma [exact]"), "{text}");
    assert!(
        text.contains("domain: all numeric arguments, with exact integer and half-integer values"),
        "{text}"
    );
    assert!(text.contains("display: infix"), "{text}");
}

#[test]
fn factor_integer_uses_prime_product_display() {
    assert_eq!(stdout(&run(&["--eval", "factorInteger(67)"])), "67\n");
    assert_eq!(
        stdout(&run(&["--eval", "factorInteger(360)"])),
        "2^3*3^2*5\n"
    );
    assert_eq!(stdout(&run(&["--eval", "factorInteger(-12)"])), "-2^2*3\n");
}

#[test]
fn settings_control_timing_and_pretty_output() {
    let output = run_stdin("set timing on\n5+3\nset pretty off\nfactorInteger(67)\nset\nquit\n");
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("8 ("), "{text}");
    assert!(text.contains(" us)"), "{text}");
    assert!(text.contains("[[67,1]]"), "{text}");
    assert!(text.contains("timing on"), "{text}");
    assert!(text.contains("pretty off"), "{text}");
}

#[test]
fn definitions_do_not_print_null_results() {
    let output = run_stdin("setDelayed(f(x_),x^2)\nf(3)\nquit\n");
    assert!(output.status.success());
    let text = stdout(&output);
    assert!(text.contains("9"), "{text}");
    assert!(!text.contains("null"), "{text}");
}

#[test]
fn module_output_is_not_duplicated() {
    let source = "mod example\ndef polynomial(x)=x^3+2*x\nprt polynomial(2)\nprt derive(polynomial(x),x)\nend\n";
    let output = run(&["--eval", source]);
    assert!(output.status.success());
    assert_eq!(stdout(&output), "12\n2+3*x^2\n");
}
