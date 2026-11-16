use mithqal::ExprId;
use mithqal::expr::Arena;
use mithqal::print::{canonical, infix};
use mithqal::syntax::parse_statement;

fn parsed(source: &str) -> (Arena, ExprId) {
    let mut arena = Arena::new();
    let id = parse_statement(&mut arena, source).unwrap();
    (arena, id)
}

fn round_trip(source: &str) {
    let mut arena = Arena::new();
    let first = parse_statement(&mut arena, source).unwrap();
    let canonical_text = canonical(&arena, first);
    let second = parse_statement(&mut arena, &canonical_text).unwrap();
    assert_eq!(
        first, second,
        "canonical round trip failed for {source}: {canonical_text}"
    );
    let infix_text = infix(&arena, first);
    let third = parse_statement(&mut arena, &infix_text).unwrap();
    assert_eq!(
        first, third,
        "infix round trip failed for {source}: {infix_text}"
    );
}

#[test]
fn infix_parenthesizes_sums_in_products() {
    let (arena, id) = parsed("(a+b)*c");
    assert_eq!(infix(&arena, id), "c*(a+b)");
    let (arena, id) = parsed("a*(b+c)");
    assert_eq!(infix(&arena, id), "a*(b+c)");
}

#[test]
fn infix_parenthesizes_negative_bases() {
    let (arena, id) = parsed("(-2)^2");
    assert_eq!(infix(&arena, id), "(-2)^2");
    let (arena, id) = parsed("x^-1");
    assert_eq!(infix(&arena, id), "1/x");
}

#[test]
fn infix_parenthesizes_rational_exponents() {
    let (arena, id) = parsed("4^(1/2)");
    assert_eq!(infix(&arena, id), "4^(1/2)");
    let (arena, id) = parsed("(1/2)^2");
    assert_eq!(infix(&arena, id), "(1/2)^2");
}

#[test]
fn infix_parenthesizes_negation_and_not() {
    let (arena, id) = parsed("-(a+b)");
    assert_eq!(infix(&arena, id), "-(a+b)");
    let (arena, id) = parsed("!(a&&b)");
    assert_eq!(infix(&arena, id), "!(a&&b)");
}

#[test]
fn infix_shows_control_and_definition_forms() {
    let (arena, id) = parsed("f(x_):=x^2");
    assert_eq!(infix(&arena, id), "f(x_):=x^2");
    let (arena, id) = parsed("fn(x,y)=>x+y");
    assert_eq!(infix(&arena, id), "fn(x,y)=>x+y");
    let (arena, id) = parsed("let([x=1],x+1)");
    assert_eq!(infix(&arena, id), "let([x=1],1+x)");
}

#[test]
fn canonical_output_is_head_based() {
    let (arena, id) = parsed("f(x_)");
    assert_eq!(canonical(&arena, id), "f(pattern(x,blank()))");
    let (arena, id) = parsed("[1,2]");
    assert_eq!(canonical(&arena, id), "list(1,2)");
    let (arena, id) = parsed("fn(x)=>x");
    assert_eq!(canonical(&arena, id), "function(list(x),x)");
    let (arena, id) = parsed("when(x_,x>0)");
    assert_eq!(
        canonical(&arena, id),
        "condition(pattern(x,blank()),greater(x,0))"
    );
}

#[test]
fn infix_prints_comparisons_and_rules() {
    let (arena, id) = parsed("x===y");
    assert_eq!(infix(&arena, id), "x===y");
    let (arena, id) = parsed("x=/=y");
    assert_eq!(infix(&arena, id), "x=/=y");
    let (arena, id) = parsed("x~y");
    assert_eq!(infix(&arena, id), "x~y");
    let (arena, id) = parsed("x->y");
    assert_eq!(infix(&arena, id), "x->y");
    let (arena, id) = parsed("x:>y");
    assert_eq!(infix(&arena, id), "x:>y");
}

#[test]
fn infix_prints_empty_containers() {
    let (arena, id) = parsed("[]");
    assert_eq!(infix(&arena, id), "[]");
    let (arena, id) = parsed("f()");
    assert_eq!(infix(&arena, id), "f()");
}

#[test]
fn infix_prints_nested_lists() {
    let (arena, id) = parsed("[[1,2],[3,4]]");
    assert_eq!(infix(&arena, id), "[[1,2],[3,4]]");
}

#[test]
fn infix_prints_if_calls() {
    let (arena, id) = parsed("if(x,1,2)");
    assert_eq!(infix(&arena, id), "if(x,1,2)");
}

#[test]
fn infix_prints_fractions_in_context() {
    let (arena, id) = parsed("(1/2)*x");
    assert_eq!(infix(&arena, id), "(1/2)*x");
    let (arena, id) = parsed("1/2+x");
    assert_eq!(infix(&arena, id), "1/2+x");
}

#[test]
fn printed_output_contains_no_whitespace() {
    for source in [
        "x+y",
        "f(x_):=x^2",
        "fn(x)=>x+1",
        "let([x=1],x+1)",
        "[[1,2],[3,4]]",
        "x===y",
        "a&&b||c",
        "(-2)^2",
        "4^(1/2)",
    ] {
        let (arena, id) = parsed(source);
        assert!(!canonical(&arena, id).contains(char::is_whitespace));
        assert!(!infix(&arena, id).contains(char::is_whitespace));
    }
}

#[test]
fn broad_round_trip_corpus() {
    for source in [
        "0",
        "1",
        "-1",
        "1/2",
        "-1/2",
        "1.05",
        "0001.20",
        "x",
        "fooBar",
        "x+y",
        "x-y",
        "x*y",
        "x/y",
        "2x",
        "2(x+1)",
        "(x+1)(x-1)",
        "f(x)(y)",
        "x^2y",
        "2^3^4",
        "(-2)^2",
        "x^-1",
        "4^(1/2)",
        "[1,2,3]",
        "[]",
        "[[1,2],[3,4]]",
        "f()",
        "f(x)",
        "f(x_)",
        "x__",
        "x___",
        "_",
        "f(when(x_,x>0))",
        "f(x_):=x^2",
        "a=b=1",
        "x_->x^2",
        "x_:>x^2",
        "x===y",
        "x=/=y",
        "x~y",
        "x<y",
        "x<=y",
        "x>y",
        "x>=y",
        "a&&b||c",
        "!a",
        "if(x,1,2)",
        "fn(x)=>x+1",
        "fn(x,y)=>x+y",
        "let([x=1,y=2],x+y)",
    ] {
        round_trip(source);
    }
}
