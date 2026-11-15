use mithqal::ExprId;
use mithqal::diag::ErrorCode;
use mithqal::eval::evaluate;
use mithqal::expr::Arena;
use mithqal::print::infix;
use mithqal::syntax::parse_statement;

fn eval_source(source: &str) -> (Arena, ExprId) {
    let mut arena = Arena::new();
    let parsed = parse_statement(&mut arena, source).unwrap();
    let evaluated = evaluate(&mut arena, parsed).unwrap();
    (arena, evaluated)
}

fn ep(source: &str) -> String {
    let (arena, id) = eval_source(source);
    infix(&arena, id)
}

#[test]
fn rational_arithmetic() {
    assert_eq!(ep("2/3+1/6"), "5/6");
    assert_eq!(ep("1/3+1/6+1/2"), "1");
    assert_eq!(ep("1/3*3/4"), "1/4");
    assert_eq!(ep("1.25+0.75"), "2");
    assert_eq!(ep("4/2"), "2");
}

#[test]
fn identities_and_cancellation() {
    assert_eq!(ep("x+0"), "x");
    assert_eq!(ep("x*1"), "x");
    assert_eq!(ep("x*0"), "0");
    assert_eq!(ep("x^1"), "x");
    assert_eq!(ep("x^0"), "1");
    assert_eq!(ep("x-x"), "0");
    assert_eq!(ep("x/x"), "1");
}

#[test]
fn like_terms() {
    assert_eq!(ep("x+x"), "2*x");
    assert_eq!(ep("2*x+3*x"), "5*x");
    assert_eq!(ep("x^2+3*x^2"), "4*x^2");
    assert_eq!(ep("x*y+2*x*y"), "3*x*y");
    assert_eq!(ep("2*x-5*x"), "-(3*x)");
}

#[test]
fn powers() {
    assert_eq!(ep("2^10"), "1024");
    assert_eq!(ep("2^-1"), "1/2");
    assert_eq!(ep("2^3^2"), "512");
    assert_eq!(ep("x*x"), "x^2");
    assert_eq!(ep("x^2*x^3"), "x^5");
    assert_eq!(ep("x*x^-1"), "1");
    assert_eq!(ep("sin(x)*sin(x)"), "sin(x)^2");
}

#[test]
fn imaginary_unit() {
    assert_eq!(ep("i^2"), "-1");
    assert_eq!(ep("i^3"), "-i");
    assert_eq!(ep("i^4"), "1");
    assert_eq!(ep("i^-1"), "-i");
    assert_eq!(ep("i*i"), "-1");
    assert_eq!(ep("2*i+3*i"), "5*i");
}

#[test]
fn gaussian_rationals_normalize() {
    assert_eq!(ep("(1+i)*(1-i)"), "2");
    assert_eq!(ep("(1+i)^2"), "2*i");
    assert_eq!(ep("(2+3*i)/(1-i)"), "-1/2+(5/2)*i");
    assert_eq!(ep("(a+b*i)*(c+d*i)"), "(a+b*i)*(c+d*i)");
}

#[test]
fn listable_arithmetic() {
    assert_eq!(ep("[1,2]+3"), "[4,5]");
    assert_eq!(ep("[1,2]*[3,4]"), "[3,8]");
    assert_eq!(ep("[1,2]^2"), "[1,4]");
    assert_eq!(ep("-[1,2]"), "[-1,-2]");
}

#[test]
fn list_length_mismatch_is_an_error() {
    let mut arena = Arena::new();
    let parsed = parse_statement(&mut arena, "[1,2]+[3,4,5]").unwrap();
    let error = evaluate(&mut arena, parsed).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalArity);
}

#[test]
fn comparisons() {
    assert_eq!(ep("1===1"), "true");
    assert_eq!(ep("1===2"), "false");
    assert_eq!(ep("1~1"), "true");
    assert_eq!(ep("x~y"), "false");
    assert_eq!(ep("2=/=3"), "true");
    assert_eq!(ep("1<2"), "true");
    assert_eq!(ep("2<=2"), "true");
    assert_eq!(ep("3>2"), "true");
    assert_eq!(ep("3>=4"), "false");
    assert_eq!(ep("x<y"), "x<y");
    assert_eq!(ep("x===y"), "x===y");
}

#[test]
fn logic_and_control() {
    assert_eq!(ep("true&&false"), "false");
    assert_eq!(ep("true||false"), "true");
    assert_eq!(ep("!true"), "false");
    assert_eq!(ep("if(true,1,2)"), "1");
    assert_eq!(ep("if(false,1,2)"), "2");
    assert_eq!(ep("if(x,1,2)"), "if(x,1,2)");
    assert_eq!(ep("if(undefined,1,2)"), "undefined");
}

#[test]
fn error_values() {
    assert_eq!(ep("1/0"), "undefined");
    assert_eq!(ep("0/0"), "indeterminate");
    assert_eq!(ep("0*(1/0)"), "indeterminate");
    assert_eq!(ep("1/0+1"), "undefined");
    assert_eq!(ep("0^0"), "indeterminate");
    assert_eq!(ep("0^-1"), "undefined");
}

#[test]
fn unknown_heads_evaluate_arguments() {
    assert_eq!(ep("f(1+2)"), "f(3)");
    assert_eq!(ep("sin(x+x)"), "sin(2*x)");
}

#[test]
fn evaluation_is_idempotent() {
    for source in [
        "x+x",
        "2/3+1/6",
        "x-x",
        "x*x",
        "(1+i)*(1-i)",
        "f(1+2)",
        "[1,2]+3",
        "if(true,1+2,3)",
    ] {
        let (mut arena, first) = eval_source(source);
        let second = evaluate(&mut arena, first).unwrap();
        assert_eq!(first, second, "not idempotent for {source}");
    }
}
