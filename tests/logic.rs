use mithqal::{ErrorCode, Session};

fn text(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn xor_truth_table() {
    assert_eq!(text("xor(false,false)"), "false");
    assert_eq!(text("xor(false,true)"), "true");
    assert_eq!(text("xor(true,false)"), "true");
    assert_eq!(text("xor(true,true)"), "false");
}

#[test]
fn implication_truth_table() {
    assert_eq!(text("implies(false,false)"), "true");
    assert_eq!(text("implies(false,true)"), "true");
    assert_eq!(text("implies(true,false)"), "false");
    assert_eq!(text("implies(true,true)"), "true");
}

#[test]
fn all_and_any() {
    assert_eq!(text("all([])"), "true");
    assert_eq!(text("all([true])"), "true");
    assert_eq!(text("all([true,true,true])"), "true");
    assert_eq!(text("all([true,false,true])"), "false");
    assert_eq!(text("any([])"), "false");
    assert_eq!(text("any([false])"), "false");
    assert_eq!(text("any([false,false,true])"), "true");
}

#[test]
fn logic_composes_with_predicates() {
    assert_eq!(text("and(isPrime(7),isOdd(7))"), "true");
    assert_eq!(text("or(isPrime(4),isEven(4))"), "true");
    assert_eq!(text("not(isPrime(4))"), "true");
    assert_eq!(text("xor(isPrime(3),isPrime(4))"), "true");
    assert_eq!(text("implies(isPrime(4),isPrime(6))"), "true");
    assert_eq!(text("all([isPrime(2),isPrime(3),isPrime(5)])"), "true");
    assert_eq!(text("any([isPrime(4),isPrime(6),isPrime(7)])"), "true");
}

#[test]
fn symbolic_logic_stays_unevaluated() {
    assert!(text("xor(x,true)").contains("xor"));
    assert!(text("implies(x,true)").contains("implies"));
    assert!(text("all([true,x])").contains("all"));
    assert!(text("any([false,x])").contains("any"));
}

#[test]
fn logic_arity_errors() {
    for source in [
        "xor(true)",
        "xor(true,false,true)",
        "implies(true)",
        "all()",
        "all([true],[false])",
        "any()",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "{source}");
    }
}

#[test]
fn logic_symbols_are_protected() {
    for name in ["xor", "implies", "all", "any"] {
        let mut session = Session::new();
        let error = session.eval_statement(&format!("{name}=1")).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "{name}");
    }
}

#[test]
fn logic_from_the_language() {
    let source = r#"
mod logic
let n=17
if and(isPrime(n),isOdd(n)) then
  prt 1
else
  prt 0
end
if xor(isEven(n),isPrime(n)) then
  prt 2
end
prt implies(isPrime(4),isPrime(6))
end
"#;
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    assert_eq!(session.output(), &["1", "2", "true"]);
}
