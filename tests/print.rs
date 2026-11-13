use cas::expr::Arena;
use cas::print::{canonical, infix};
use cas::syntax::parse_statement;

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
fn canonical_output_uses_full_heads() {
    let mut arena = Arena::new();
    let id = parse_statement(&mut arena, "1+2x").unwrap();
    assert_eq!(canonical(&arena, id), "plus(1,times(2,x))");
}

#[test]
fn round_trip_corpus() {
    for source in [
        "1",
        "-1/2",
        "1.25",
        "x+y",
        "2x",
        "2(x+1)",
        "(x+1)(x-1)",
        "f(x)(y)",
        "x^2y",
        "[1,2,3]",
        "f(x_)",
        "f(x_):=x^2",
        "a=3^2",
        "x_->x^2",
        "x_:>x^2",
        "x===y",
        "x=/=y",
        "x~y",
        "x<y",
        "a&&b||c",
        "fn(x)=>x+1",
        "let([x=1],x+1)",
        "when(x_,x>0)",
        "f(when(x_,x>0))",
        "x_",
        "x__",
        "x___",
        "_",
    ] {
        round_trip(source);
    }
}

#[test]
fn printed_output_has_no_whitespace() {
    let mut arena = Arena::new();
    for source in ["x+y", "f(x_):=x^2", "fn(x)=>x+1", "let([x=1],x+1)"] {
        let id = parse_statement(&mut arena, source).unwrap();
        assert!(!canonical(&arena, id).contains(char::is_whitespace));
        assert!(!infix(&arena, id).contains(char::is_whitespace));
    }
}
