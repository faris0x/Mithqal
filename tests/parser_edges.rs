use mithqal::diag::ErrorCode;
use mithqal::expr::{Arena, ExprId, Node};
use mithqal::num::Int;
use mithqal::syntax::{parse_source, parse_statement};

fn parse_one(source: &str) -> (Arena, ExprId) {
    let mut arena = Arena::new();
    let id = parse_statement(&mut arena, source).unwrap();
    (arena, id)
}

fn head_name(arena: &Arena, id: ExprId) -> String {
    match arena.try_node(id).unwrap() {
        Node::Call { head, .. } => {
            let symbol = arena.symbol_id(*head).unwrap();
            arena.symbol_name(symbol).to_string()
        }
        _ => String::new(),
    }
}

fn arg(arena: &Arena, id: ExprId, index: usize) -> ExprId {
    match arena.try_node(id).unwrap() {
        Node::Call { args, .. } => args[index],
        _ => panic!("expected a call"),
    }
}

#[test]
fn arithmetic_precedence() {
    let (arena, id) = parse_one("1+2*3");
    assert_eq!(head_name(&arena, id), "plus");
    assert_eq!(head_name(&arena, arg(&arena, id, 1)), "times");

    let (arena, id) = parse_one("2^3^4");
    assert_eq!(head_name(&arena, id), "power");
    assert_eq!(head_name(&arena, arg(&arena, id, 1)), "power");

    let (arena, id) = parse_one("a/b*c");
    assert_eq!(head_name(&arena, id), "times");
}

#[test]
fn unary_minus_precedence() {
    let (arena, id) = parse_one("-x^2");
    assert_eq!(head_name(&arena, id), "negate");
    assert_eq!(head_name(&arena, arg(&arena, id, 0)), "power");

    let (arena, id) = parse_one("(-2)^2");
    assert_eq!(head_name(&arena, id), "power");
    assert!(matches!(
        arena.try_node(arg(&arena, id, 0)).unwrap(),
        Node::Int(_)
    ));

    let (arena, id) = parse_one("2^-1");
    assert_eq!(head_name(&arena, id), "power");
    assert!(matches!(
        arena.try_node(arg(&arena, id, 1)).unwrap(),
        Node::Int(_)
    ));
}

#[test]
fn implicit_multiplication_variants() {
    for source in [
        "2x", "2(x)", "(a)(b)", "2sin(x)", "f(x)g(x)", "f(x)(y)", "x^2y", "[1,2]x", "1e4",
    ] {
        let (arena, id) = parse_one(source);
        assert_eq!(head_name(&arena, id), "times", "source {source}");
    }
}

#[test]
fn empty_calls_and_lists() {
    let (arena, id) = parse_one("f()");
    assert_eq!(head_name(&arena, id), "f");
    let (arena, id) = parse_one("[]");
    assert_eq!(head_name(&arena, id), "list");
    match arena.try_node(id).unwrap() {
        Node::Call { args, .. } => assert!(args.is_empty()),
        _ => panic!("expected a list"),
    }
}

#[test]
fn nested_lists() {
    let (arena, id) = parse_one("[[1,2],[3,4]]");
    assert_eq!(head_name(&arena, id), "list");
    assert_eq!(head_name(&arena, arg(&arena, id, 0)), "list");
    assert_eq!(head_name(&arena, arg(&arena, id, 1)), "list");
}

#[test]
fn exact_decimal_forms() {
    let (arena, id) = parse_one("1.05");
    match arena.try_node(id).unwrap() {
        Node::Rat(value) => assert_eq!(value.to_decimal(), "21/20"),
        _ => panic!("expected a rational"),
    }
    let (arena, id) = parse_one("0001.20");
    match arena.try_node(id).unwrap() {
        Node::Rat(value) => assert_eq!(value.to_decimal(), "6/5"),
        _ => panic!("expected a rational"),
    }
    let (arena, id) = parse_one("0.0");
    match arena.try_node(id).unwrap() {
        Node::Int(value) => assert_eq!(value.to_decimal(), "0"),
        _ => panic!("expected an integer"),
    }
}

#[test]
fn negative_numbers_fold_during_parsing() {
    let (arena, id) = parse_one("-1");
    match arena.try_node(id).unwrap() {
        Node::Int(value) => assert_eq!(value.to_decimal(), "-1"),
        _ => panic!("expected an integer"),
    }
    let (arena, id) = parse_one("-1.5");
    match arena.try_node(id).unwrap() {
        Node::Rat(value) => assert_eq!(value.to_decimal(), "-3/2"),
        _ => panic!("expected a rational"),
    }
    let (arena, id) = parse_one("--1");
    match arena.try_node(id).unwrap() {
        Node::Int(value) => assert_eq!(value.to_decimal(), "1"),
        _ => panic!("expected an integer"),
    }
}

#[test]
fn numeric_division_folds() {
    let (arena, id) = parse_one("1/2");
    match arena.try_node(id).unwrap() {
        Node::Rat(value) => assert_eq!(value.to_decimal(), "1/2"),
        _ => panic!("expected a rational"),
    }
    let (arena, id) = parse_one("2/4");
    match arena.try_node(id).unwrap() {
        Node::Rat(value) => assert_eq!(value.to_decimal(), "1/2"),
        _ => panic!("expected a rational"),
    }
    let (arena, id) = parse_one("0/0");
    assert_eq!(head_name(&arena, id), "times");
    let (arena, id) = parse_one("1/0");
    assert_eq!(head_name(&arena, id), "power");
    let (arena, id) = parse_one("1/x");
    assert_eq!(head_name(&arena, id), "power");
}

#[test]
fn patterns_parse() {
    let (arena, id) = parse_one("_");
    assert_eq!(head_name(&arena, id), "blank");
    let (arena, id) = parse_one("x_");
    assert_eq!(head_name(&arena, id), "pattern");
    assert_eq!(head_name(&arena, arg(&arena, id, 1)), "blank");
    let (arena, id) = parse_one("x__");
    assert_eq!(head_name(&arena, arg(&arena, id, 1)), "blankSequence");
    let (arena, id) = parse_one("x___");
    assert_eq!(head_name(&arena, arg(&arena, id, 1)), "blankNullSequence");
}

#[test]
fn conditions_map_to_condition() {
    let (arena, id) = parse_one("when(x_,x>0)");
    assert_eq!(head_name(&arena, id), "condition");
    let (arena, id) = parse_one("f(when(x_,x>0))");
    assert_eq!(head_name(&arena, id), "f");
    assert_eq!(head_name(&arena, arg(&arena, id, 0)), "condition");
}

#[test]
fn definitions_and_rules_parse() {
    let (arena, id) = parse_one("f(x_):=x^2");
    assert_eq!(head_name(&arena, id), "setDelayed");
    let (arena, id) = parse_one("a=3^2");
    assert_eq!(head_name(&arena, id), "set");
    let (arena, id) = parse_one("x_->x^2");
    assert_eq!(head_name(&arena, id), "rule");
    let (arena, id) = parse_one("x_:>x^2");
    assert_eq!(head_name(&arena, id), "ruleDelayed");
}

#[test]
fn assignment_and_rules_associate_to_the_right() {
    let (arena, id) = parse_one("a=b=1");
    assert_eq!(head_name(&arena, id), "set");
    assert_eq!(head_name(&arena, arg(&arena, id, 1)), "set");
    let (arena, id) = parse_one("a->b->c");
    assert_eq!(head_name(&arena, id), "rule");
    assert_eq!(head_name(&arena, arg(&arena, id, 1)), "rule");
}

#[test]
fn logic_precedence() {
    let (arena, id) = parse_one("a&&b||c");
    assert_eq!(head_name(&arena, id), "or");
    assert_eq!(head_name(&arena, arg(&arena, id, 0)), "and");
    let (arena, id) = parse_one("!a&&b");
    assert_eq!(head_name(&arena, id), "and");
    assert_eq!(head_name(&arena, arg(&arena, id, 0)), "not");
}

#[test]
fn functions_parse_with_zero_or_many_parameters() {
    let (arena, id) = parse_one("fn()=>1");
    assert_eq!(head_name(&arena, id), "function");
    let (arena, id) = parse_one("fn(x,y)=>x+y");
    assert_eq!(head_name(&arena, id), "function");
    assert_eq!(head_name(&arena, arg(&arena, id, 0)), "list");
}

#[test]
fn let_parses_multiple_bindings() {
    let (arena, id) = parse_one("let([x=1,y=2],x+y)");
    assert_eq!(head_name(&arena, id), "let");
    assert_eq!(head_name(&arena, arg(&arena, id, 0)), "list");
}

#[test]
fn unclosed_delimiters_are_errors() {
    let mut arena = Arena::new();
    for source in ["(x", "[1,2", "f(x", "let([x=1],x", "fn(x=>x"] {
        let error = parse_source(&mut arena, source).unwrap_err();
        assert_eq!(
            error.code,
            ErrorCode::ParseUnexpectedToken,
            "source {source}"
        );
    }
}

#[test]
fn trailing_and_missing_commas_are_errors() {
    let mut arena = Arena::new();
    for source in ["f(x,)", "[1,]", "f(,x)"] {
        assert!(parse_source(&mut arena, source).is_err(), "source {source}");
    }
}

#[test]
fn comparison_chaining_is_an_error() {
    let mut arena = Arena::new();
    let error = parse_source(&mut arena, "x===y===z").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn multiple_statements_are_available_from_source() {
    let mut arena = Arena::new();
    let statements = parse_source(&mut arena, "1\n2\n3\n").unwrap();
    assert_eq!(statements.len(), 3);
}

#[test]
fn parse_statement_rejects_multiple_statements() {
    let mut arena = Arena::new();
    let error = parse_statement(&mut arena, "1\n2\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn empty_source_has_no_statements() {
    let mut arena = Arena::new();
    assert!(parse_source(&mut arena, "").unwrap().is_empty());
    assert!(parse_source(&mut arena, "\n\n").unwrap().is_empty());
}

#[test]
fn patterns_require_an_identifier_prefix() {
    let mut arena = Arena::new();
    for source in ["2_", "(x)_"] {
        assert!(parse_source(&mut arena, source).is_err(), "source {source}");
    }
}

#[test]
fn invalid_numbers_are_reported_by_the_parser() {
    let mut arena = Arena::new();
    for source in ["1.", "1..2"] {
        let error = parse_source(&mut arena, source).unwrap_err();
        assert_eq!(error.code, ErrorCode::ParseInvalidNumber);
    }
}

#[test]
fn integer_literals_use_arbitrary_precision() {
    let (arena, id) = parse_one("123456789012345678901234567890");
    match arena.try_node(id).unwrap() {
        Node::Int(value) => assert_eq!(value.to_decimal(), "123456789012345678901234567890"),
        _ => panic!("expected an integer"),
    }
}

#[test]
fn zero_is_an_integer_node() {
    let (arena, id) = parse_one("0");
    match arena.try_node(id).unwrap() {
        Node::Int(value) => assert_eq!(value, &Int::zero()),
        _ => panic!("expected an integer"),
    }
}

#[test]
fn double_equals_parses_as_equality() {
    let (arena, id) = parse_one("x+1==2");
    assert_eq!(head_name(&arena, id), "equal");
    let (arena, id) = parse_one("a=b");
    assert_eq!(head_name(&arena, id), "set");
    let (arena, id) = parse_one("a===b");
    assert_eq!(head_name(&arena, id), "equal");
}
