use mithqal::diag::ErrorCode;
use mithqal::expr::{Arena, ExprId, Node};
use mithqal::syntax::parse_source;

fn parse_one(source: &str) -> (Arena, ExprId) {
    let mut arena = Arena::new();
    let statements = parse_source(&mut arena, source).unwrap();
    assert_eq!(statements.len(), 1);
    (arena, statements[0])
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

#[test]
fn statements_are_newline_separated() {
    let mut arena = Arena::new();
    let statements = parse_source(&mut arena, "1+2\n3+4\n").unwrap();
    assert_eq!(statements.len(), 2);
    let empty = parse_source(&mut arena, "\n\n").unwrap();
    assert!(empty.is_empty());
}

#[test]
fn comments_occupy_whole_lines() {
    let mut arena = Arena::new();
    let source = "(* a comment *)\n1+2\n(* first\nsecond *)\n3+4\n";
    let statements = parse_source(&mut arena, source).unwrap();
    assert_eq!(statements.len(), 2);
}

#[test]
fn whitespace_inside_a_statement_is_an_error() {
    let mut arena = Arena::new();
    for source in ["1 + 2", " 1+2", "1+2 ", "1\t+2"] {
        let error = parse_source(&mut arena, source).unwrap_err();
        assert_eq!(error.code, ErrorCode::ParseWhitespace);
    }
}

#[test]
fn inline_comments_are_an_error() {
    let mut arena = Arena::new();
    let error = parse_source(&mut arena, "1+2(* comment *)").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseInlineComment);
}

#[test]
fn spaces_are_allowed_on_comment_lines_and_blank_lines() {
    let mut arena = Arena::new();
    let source = "(* comment with spaces *)\n   \n1+2\n";
    let statements = parse_source(&mut arena, source).unwrap();
    assert_eq!(statements.len(), 1);
}

#[test]
fn implicit_multiplication() {
    let (arena, id) = parse_one("2x");
    assert_eq!(head_name(&arena, id), "times");
    let (arena, id) = parse_one("2(x+1)");
    assert_eq!(head_name(&arena, id), "times");
    let (arena, id) = parse_one("(x+1)(x-1)");
    assert_eq!(head_name(&arena, id), "times");
    let (arena, id) = parse_one("2sin(x)");
    assert_eq!(head_name(&arena, id), "times");
    let (arena, id) = parse_one("f(x)(y)");
    assert_eq!(head_name(&arena, id), "times");
    let (arena, id) = parse_one("x^2y");
    assert_eq!(head_name(&arena, id), "times");
    let (arena, id) = parse_one("[1,2]x");
    assert_eq!(head_name(&arena, id), "times");
}

#[test]
fn call_binds_to_a_name() {
    let (arena, id) = parse_one("x(y)");
    assert_eq!(head_name(&arena, id), "x");
}

#[test]
fn indexing_is_reserved() {
    let mut arena = Arena::new();
    for source in ["x[1]", "2[1,2]", "[1,2][3]"] {
        let error = parse_source(&mut arena, source).unwrap_err();
        assert_eq!(error.code, ErrorCode::ParseIndexingUnsupported);
    }
}

#[test]
fn decimal_literals_are_exact() {
    let (arena, id) = parse_one("1.05");
    match arena.try_node(id).unwrap() {
        Node::Rat(value) => assert_eq!(value.to_decimal(), "21/20"),
        _ => panic!("expected a rational"),
    }
    let (arena, id) = parse_one("2.000");
    match arena.try_node(id).unwrap() {
        Node::Int(value) => assert_eq!(value.to_decimal(), "2"),
        _ => panic!("expected an integer"),
    }
}

#[test]
fn definitions_and_patterns_parse() {
    let (arena, id) = parse_one("f(x_):=x^2");
    assert_eq!(head_name(&arena, id), "setDelayed");
    match arena.try_node(id).unwrap() {
        Node::Call { args, .. } => {
            let lhs = args[0];
            assert_eq!(head_name(&arena, lhs), "f");
            match arena.try_node(lhs).unwrap() {
                Node::Call { args, .. } => {
                    assert_eq!(head_name(&arena, args[0]), "pattern");
                }
                _ => panic!("expected a call"),
            }
        }
        _ => panic!("expected a call"),
    }
}

#[test]
fn functions_and_let_parse() {
    let (arena, id) = parse_one("fn(x)=>x+1");
    assert_eq!(head_name(&arena, id), "function");
    let (arena, id) = parse_one("let([x=1],x+1)");
    assert_eq!(head_name(&arena, id), "let");
}

#[test]
fn comparisons_and_logic_parse() {
    for source in ["x===y", "x=/=y", "x~y", "x<y", "x<=y", "x>y", "x>=y"] {
        let (arena, id) = parse_one(source);
        assert!(!head_name(&arena, id).is_empty());
    }
    let (arena, id) = parse_one("a&&b||c");
    assert_eq!(head_name(&arena, id), "or");
}

#[test]
fn unclosed_comment_is_an_error() {
    let mut arena = Arena::new();
    let error = parse_source(&mut arena, "(* never closed\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnclosedComment);
}
