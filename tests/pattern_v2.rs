use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_source(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn typed_blanks_match_by_head() {
    let mut session = Session::new();
    let source = "f(x_Integer):=1\nf(x_Rational):=2\nf(x_String):=3\nf(x_List):=4\nf(x_):=5\nf(3)";
    assert_eq!(eval(&mut session, source), "1");
    assert_eq!(eval(&mut session, "f(1/2)"), "2");
    assert_eq!(eval(&mut session, "f(\"a\")"), "3");
    assert_eq!(eval(&mut session, "f([1,2])"), "4");
    assert_eq!(eval(&mut session, "f(a)"), "5");
    assert_eq!(eval(&mut session, "h(_Integer):=1\nh(3)"), "1");
}

#[test]
fn alternatives_pattern_tests_and_except_work() {
    let mut session = Session::new();
    assert_eq!(
        eval(&mut session, "f(alternatives(1,2)):=1\nf(x_):=0\nf(1)"),
        "1"
    );
    assert_eq!(eval(&mut session, "f(3)"), "0");
    assert_eq!(
        eval(
            &mut session,
            "g(patternTest(x_Integer,isEven)):=1\ng(x_):=0\ng(4)"
        ),
        "1"
    );
    assert_eq!(eval(&mut session, "g(3)"), "0");
    assert_eq!(eval(&mut session, "h(except(1)):=1\nh(x_):=0\nh(2)"), "1");
    assert_eq!(eval(&mut session, "h(1)"), "0");
}

#[test]
fn optional_arguments_use_defaults() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "f(x_,optional(y_,10))=x+y\nf(1)"), "11");
    assert_eq!(eval(&mut session, "f(1,2)"), "3");
}

#[test]
fn repeated_patterns_bind_sequences() {
    let mut session = Session::new();
    assert_eq!(
        eval(&mut session, "g(repeated(x__)):=length([x])\ng(1,2,3)"),
        "3"
    );
    assert_eq!(
        eval(&mut session, "r(repeatedNull(x__)):=length([x])\nr()"),
        "0"
    );
    assert_eq!(eval(&mut session, "r(1,2)"), "2");
}

#[test]
fn verbatim_and_one_identity_patterns_work() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "v(verbatim(2)):=1\nv(x_):=0\nv(2)"), "1");
    assert_eq!(eval(&mut session, "v(3)"), "0");
    assert_eq!(
        eval(
            &mut session,
            "setAttributes(f,oneIdentity)\ng(f(x_)):=x\ng(5)"
        ),
        "5"
    );
}

#[test]
fn orderless_sequences_use_multiset_matching() {
    let mut session = Session::new();
    assert_eq!(
        eval(
            &mut session,
            "setAttributes(f,orderless)\nf(x__,y_):=y\nf(3,1,2)"
        ),
        "3"
    );
}

#[test]
fn flat_sequence_matching_remains_contiguous() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "f(x__,y_):=y\nf(1,2,3)"), "3");
}
