use mithqal::ExprId;
use mithqal::diag::ErrorCode;
use mithqal::eval::{evaluate, evaluate_with_limits};
use mithqal::expr::Arena;
use mithqal::print::infix;
use mithqal::syntax::parse_statement;

fn evaluated(source: &str) -> (Arena, ExprId) {
    let mut arena = Arena::new();
    let parsed = parse_statement(&mut arena, source).unwrap();
    let value = evaluate(&mut arena, parsed).unwrap();
    (arena, value)
}

fn ep(source: &str) -> String {
    let (arena, id) = evaluated(source);
    infix(&arena, id)
}

#[test]
fn basic_arithmetic() {
    assert_eq!(ep("1+2"), "3");
    assert_eq!(ep("2*3"), "6");
    assert_eq!(ep("2-3"), "-1");
    assert_eq!(ep("2/4"), "1/2");
    assert_eq!(ep("1/3+2/3"), "1");
    assert_eq!(ep("(2+3)*4"), "20");
}

#[test]
fn additive_and_multiplicative_identities() {
    for source in ["x+0", "0+x", "x*1", "1*x", "x^1", "1*x*1"] {
        assert_eq!(ep(source), "x", "source {source}");
    }
    for source in ["x*0", "0*x", "x-x"] {
        assert_eq!(ep(source), "0", "source {source}");
    }
    assert_eq!(ep("x^2/x^2"), "1");
    assert_eq!(ep("x^0"), "1");
    assert_eq!(ep("1^x"), "1");
    assert_eq!(ep("x/x"), "1");
}

#[test]
fn products_do_not_distribute_symbolically() {
    assert_eq!(ep("2*(x+y)"), "2*(x+y)");
    assert_eq!(ep("(x+1)^2"), "(1+x)^2");
    assert_eq!(ep("(x+1)*(x-1)"), "(-1+x)*(1+x)");
    assert_eq!(ep("2*(3+4)"), "14");
}

#[test]
fn like_terms_and_constants_sort() {
    assert_eq!(ep("x+x"), "2*x");
    assert_eq!(ep("2*x+3*x"), "5*x");
    assert_eq!(ep("x*x"), "x^2");
    assert_eq!(ep("x^2*x^3"), "x^5");
    assert_eq!(ep("x^2/x"), "x");
    assert_eq!(ep("x*x^-2"), "1/x");
    assert_eq!(ep("2*x*3"), "6*x");
    assert_eq!(ep("x*y+y*x"), "2*x*y");
    assert_eq!(ep("x+y-y"), "x");
    assert_eq!(ep("y+1+x"), "1+x+y");
}

#[test]
fn power_boundaries() {
    assert_eq!(ep("0^1"), "0");
    assert_eq!(ep("0^2"), "0");
    assert_eq!(ep("1^0"), "1");
    assert_eq!(ep("2^-2"), "1/4");
    assert_eq!(ep("(-1)^2"), "1");
    assert_eq!(ep("(-1)^3"), "-1");
    assert_eq!(ep("(-2)^2"), "4");
    assert_eq!(ep("4^(1/2)"), "4^(1/2)");
    assert_eq!(ep("2^100"), "1267650600228229401496703205376");
}

#[test]
fn imaginary_unit_powers() {
    assert_eq!(ep("i^0"), "1");
    assert_eq!(ep("i^1"), "i");
    assert_eq!(ep("i^2"), "-1");
    assert_eq!(ep("i^3"), "-i");
    assert_eq!(ep("i^4"), "1");
    assert_eq!(ep("i^5"), "i");
    assert_eq!(ep("i^8"), "1");
    assert_eq!(ep("i^-2"), "-1");
    assert_eq!(ep("2*i*i"), "-2");
    assert_eq!(ep("i*i*i*i"), "1");
}

#[test]
fn gaussian_arithmetic_edges() {
    assert_eq!(ep("(1+i)+(1-i)"), "2");
    assert_eq!(ep("(1+i)-(1-i)"), "2*i");
    assert_eq!(ep("(2+3*i)*(4-i)"), "11+10*i");
    assert_eq!(ep("(1+i)/(1-i)"), "i");
    let (arena, id) = evaluated("(1+i)^-1");
    assert_eq!(
        mithqal::print::canonical(&arena, id),
        "plus(1/2,negate(times(1/2,i)))"
    );
    assert_eq!(ep("(a+b*i)*(c+d*i)"), "(a+b*i)*(c+d*i)");
}

#[test]
fn listable_boundaries() {
    assert_eq!(ep("[1,2]+[3,4]"), "[4,6]");
    assert_eq!(ep("[1,2]*2"), "[2,4]");
    assert_eq!(ep("[1,2]/2"), "[1/2,1]");
    assert_eq!(ep("2^[1,2]"), "[2,4]");
    assert_eq!(ep("[]+1"), "[]");
    assert_eq!(ep("[1+1,2+2]"), "[2,4]");
    assert_eq!(ep("[1/0]"), "[undefined]");
}

#[test]
fn list_length_mismatch_is_an_error() {
    let mut arena = Arena::new();
    let parsed = parse_statement(&mut arena, "[1,2]+[3]").unwrap();
    let error = evaluate(&mut arena, parsed).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalArity);
}

#[test]
fn structural_equality_is_total() {
    assert_eq!(ep("1~1"), "true");
    assert_eq!(ep("1~1.0"), "true");
    assert_eq!(ep("1~2"), "false");
    assert_eq!(ep("x~x"), "true");
    assert_eq!(ep("x~y"), "false");
    assert_eq!(ep("(x+y)~(y+x)"), "true");
    assert_eq!(ep("[1,2]~[1,2]"), "true");
    assert_eq!(ep("[1,2]~[1,3]"), "false");
}

#[test]
fn mathematical_equality_is_decidable_or_unevaluated() {
    assert_eq!(ep("1===1"), "true");
    assert_eq!(ep("1===1/1"), "true");
    assert_eq!(ep("1===2"), "false");
    assert_eq!(ep("1/2===0.5"), "true");
    assert_eq!(ep("x===x"), "true");
    assert_eq!(ep("x===y"), "x===y");
    assert_eq!(ep("[1,2]===[1,2]"), "true");
    assert_eq!(ep("[1,2]===[1,3]"), "[1,2]===[1,3]");
}

#[test]
fn inequality_and_ordering() {
    assert_eq!(ep("1=/=2"), "true");
    assert_eq!(ep("1=/=1"), "false");
    assert_eq!(ep("x=/=y"), "x=/=y");
    assert_eq!(ep("1<2"), "true");
    assert_eq!(ep("2<1"), "false");
    assert_eq!(ep("1<=1"), "true");
    assert_eq!(ep("2>=2"), "true");
    assert_eq!(ep("-1<0"), "true");
    assert_eq!(ep("1/2<2/3"), "true");
    assert_eq!(ep("x<y"), "x<y");
    assert_eq!(ep("x<undefined"), "undefined");
    assert_eq!(ep("undefined<1"), "undefined");
}

#[test]
fn logic_identities_and_short_circuit() {
    assert_eq!(ep("true&&true"), "true");
    assert_eq!(ep("true&&false"), "false");
    assert_eq!(ep("false&&(1/0)"), "false");
    assert_eq!(ep("true||(1/0)"), "true");
    assert_eq!(ep("true&&x"), "x");
    assert_eq!(ep("x&&true"), "x");
    assert_eq!(ep("false||x"), "x");
    assert_eq!(ep("x||false"), "x");
    assert_eq!(ep("and()"), "true");
    assert_eq!(ep("or()"), "false");
    assert_eq!(ep("!true"), "false");
    assert_eq!(ep("!false"), "true");
    assert_eq!(ep("!x"), "!x");
    assert_eq!(ep("!undefined"), "undefined");
}

#[test]
fn control_flow_boundaries() {
    assert_eq!(ep("if(true,1,2)"), "1");
    assert_eq!(ep("if(false,1,2)"), "2");
    assert_eq!(ep("if(true,1,1/0)"), "1");
    assert_eq!(ep("if(false,1/0,2)"), "2");
    assert_eq!(ep("if(x,1,2)"), "if(x,1,2)");
    assert_eq!(ep("if(undefined,1,2)"), "undefined");
    assert_eq!(ep("if(indeterminate,1,2)"), "indeterminate");
    assert_eq!(ep("if(failed,1,2)"), "failed");
    assert_eq!(ep("if(true,2,3)+1"), "3");
}

#[test]
fn control_flow_arity_errors() {
    let mut arena = Arena::new();
    for source in ["if(true,1)", "not()", "not(true,false)"] {
        let parsed = parse_statement(&mut arena, source).unwrap();
        let error = evaluate(&mut arena, parsed).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "source {source}");
    }
}

#[test]
fn error_value_propagation() {
    assert_eq!(ep("undefined+1"), "undefined");
    assert_eq!(ep("indeterminate+1"), "indeterminate");
    assert_eq!(ep("failed+1"), "failed");
    assert_eq!(ep("undefined*5"), "undefined");
    assert_eq!(ep("0*undefined"), "indeterminate");
    assert_eq!(ep("undefined*0"), "indeterminate");
    assert_eq!(ep("indeterminate*0"), "indeterminate");
    assert_eq!(ep("1*undefined"), "undefined");
    assert_eq!(ep("-(1/0)"), "undefined");
    assert_eq!(ep("(1/0)^2"), "undefined");
    assert_eq!(ep("0^0"), "indeterminate");
    assert_eq!(ep("0^-1"), "undefined");
}

#[test]
fn unknown_heads_keep_their_shape() {
    assert_eq!(ep("f(1+2)"), "f(3)");
    assert_eq!(ep("sin(x+x)"), "sin(2*x)");
    assert_eq!(ep("g()"), "g()");
}

#[test]
fn budget_is_enforced() {
    let mut arena = Arena::new();
    let parsed = parse_statement(&mut arena, "1+2").unwrap();
    let error = evaluate_with_limits(&mut arena, parsed, 1, 8).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalBudget);
}

#[test]
fn idempotence_broad_corpus() {
    for source in [
        "1+2",
        "x+x",
        "x-x",
        "x*x",
        "x^2/x",
        "2/3+1/6",
        "(1+i)*(1-i)",
        "(2+3*i)/(1-i)",
        "f(1+2)",
        "[1,2]+3",
        "if(true,1+2,3)",
        "true&&x",
        "false||x",
        "x===y",
        "x<y",
        "1/0",
        "0/0",
    ] {
        let (mut arena, first) = evaluated(source);
        let second = evaluate(&mut arena, first).unwrap();
        assert_eq!(first, second, "not idempotent for {source}");
    }
}
