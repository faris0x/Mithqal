use mithqal::ExprId;
use mithqal::{ErrorCode, Session};

fn eval(source: &str) -> (Session, ExprId) {
    let mut session = Session::new();
    let value = session.eval_source(source).unwrap();
    (session, value)
}

fn output(source: &str) -> Vec<String> {
    let (session, _) = eval(source);
    session.output().to_vec()
}

#[test]
fn module_with_bindings_and_printing() {
    let source = r#"
module algebra.
let x be 3.
let y be 4.
print x + y.
print x * y.
end module.
"#;
    let (session, value) = eval(source);
    assert_eq!(session.format_infix(value).unwrap(), "12");
    assert_eq!(session.output(), &["7", "12"]);
}

#[test]
fn definitions_take_named_parameters() {
    let source = r#"
module functions.
we define square(x) = x * x.
we define add(x, y) = x + y.
we define zero() = 0.
print square(5).
print add(2, 3).
print zero().
end module.
"#;
    assert_eq!(output(source), vec!["25", "5", "0"]);
}

#[test]
fn definitions_apply_to_expressions() {
    let source = r#"
module functions.
we define square(x) = x * x.
print square(1 + 2).
print square(square(2)).
end module.
"#;
    assert_eq!(output(source), vec!["9", "16"]);
}

#[test]
fn conditionals_choose_one_branch() {
    let true_source = r#"
module control.
let x be 5.
if x > 3 then:
    print x.
else:
    print 0.
end if.
end module.
"#;
    assert_eq!(output(true_source), vec!["5"]);
    let false_source = r#"
module control.
let x be 1.
if x > 3 then:
    print x.
else:
    print 0.
end if.
end module.
"#;
    assert_eq!(output(false_source), vec!["0"]);
}

#[test]
fn conditionals_without_else() {
    let source = r#"
module control.
if 1 < 2 then:
    print 7.
end if.
print 8.
end module.
"#;
    assert_eq!(output(source), vec!["7", "8"]);
}

#[test]
fn nested_conditionals_share_end_if() {
    let source = r#"
module control.
let x be 2.
if x > 0 then:
    if x > 1 then:
        print 10.
    else:
        print 20.
    end if.
else:
    print 30.
end if.
end module.
"#;
    assert_eq!(output(source), vec!["10"]);
}

#[test]
fn indentation_is_optional() {
    let flat = r#"
module flat.
let x be 2.
if x > 0 then:
print x.
else:
print 0.
end if.
end module.
"#;
    assert_eq!(output(flat), vec!["2"]);
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let source = r#"
// leading comment

module comments.
// inside module
let x be 1 + 2. // trailing comment

// before end
end module.
"#;
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    assert!(session.output().is_empty());
}

#[test]
fn module_name_is_validated() {
    for source in ["module 9bad.\nend module.", "module.\nend module."] {
        let mut session = Session::new();
        assert!(session.eval_source(source).is_err(), "{source}");
    }
}

#[test]
fn missing_end_module_is_an_error() {
    let source = "module missing.\nlet x be 1.";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn statements_after_module_end_are_rejected() {
    let source = "module extra.\nlet x be 1.\nend module.\nprint x.";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn nested_modules_are_rejected() {
    let source = "module outer.\nmodule inner.\nend module.\nend module.";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn stray_else_and_case_are_rejected() {
    for source in [
        "module m.\nelse:\nend module.",
        "module m.\ncase 1:\nend module.",
    ] {
        let mut session = Session::new();
        let error = session.eval_source(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::ParseUnexpectedToken, "{source}");
    }
}

#[test]
fn whitespace_inside_expressions_is_allowed() {
    let source = "module spacing.\nlet x be 1 + 2.\nprint x.\nend module.";
    assert_eq!(output(source), vec!["3"]);
}

#[test]
fn intake_output_can_be_taken() {
    let mut session = Session::new();
    session
        .eval_source("module o.\nprint 1.\nprint 2.\nend module.")
        .unwrap();
    assert_eq!(session.output(), &["1", "2"]);
    let taken = session.take_output();
    assert_eq!(taken, vec!["1", "2"]);
    assert!(session.output().is_empty());
}

#[test]
fn lowercase_module_keyword_is_required() {
    let source = "MODULE upper.\nPRINT 1.\nEND MODULE.";
    let mut session = Session::new();
    assert!(session.eval_source(source).is_err());
}

#[test]
fn zero_parameter_definitions_work() {
    let source =
        "module zero.\nwe define value() = 42.\nprint value().\nprint value() + 1.\nend module.";
    assert_eq!(output(source), vec!["42", "43"]);
}

#[test]
fn literal_parameter_patterns_enable_recursion() {
    let source = r#"
module recursive.
we define fact(0) = 1.
we define fact(n) = n * fact(n - 1).
print fact(5).
print fact(7).
end module.
"#;
    assert_eq!(output(source), vec!["120", "5040"]);
}

#[test]
fn multiple_literal_parameter_patterns() {
    let source = r#"
module recursive.
we define fib(0) = 0.
we define fib(1) = 1.
we define fib(n) = fib(n - 1) + fib(n - 2).
print fib(8).
end module.
"#;
    assert_eq!(output(source), vec!["21"]);
}

#[test]
fn guarded_definitions() {
    let source = r#"
module guards.
we define sign(x) = 1, where x > 0.
we define sign(x) = -1, where x < 0.
we define sign(x) = 0.
print sign(5).
print sign(-5).
print sign(0).
end module.
"#;
    assert_eq!(output(source), vec!["1", "-1", "0"]);
}

#[test]
fn guarded_definitions_use_other_parameters() {
    let source = r#"
module guards.
we define f(x, y) = x, where x > y.
we define f(x, y) = y.
print f(5, 2).
print f(2, 5).
end module.
"#;
    assert_eq!(output(source), vec!["5", "5"]);
}

#[test]
fn bindings_are_block_scoped() {
    let source = r#"
module scopes.
if true then:
    let x be 5.
    print x.
end if.
print x.
end module.
"#;
    assert_eq!(output(source), vec!["5", "x"]);
}

#[test]
fn definitions_capture_outer_bindings() {
    let source = r#"
module capture.
let a be 10.
we define f(x) = x + a.
print f(5).
end module.
"#;
    assert_eq!(output(source), vec!["15"]);
}

#[test]
fn nested_matches_share_end_match() {
    let source = r#"
module matching.
match 1 with:
case 1:
    match 2 with:
    case 2:
        print 12.
    else:
        print 10.
    end match.
else:
    print 0.
end match.
end module.
"#;
    assert_eq!(output(source), vec!["12"]);
}

#[test]
fn case_outside_match_is_rejected() {
    let source = "module m.\ncase 1:\nend module.";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}
