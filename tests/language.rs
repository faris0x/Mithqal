use mithqal::{ErrorCode, Session};

fn eval(source: &str) -> (Session, u32) {
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
mod algebra
let x = 3
let y=4
prt x+y
prt x*y
end
"#;
    let (session, value) = eval(source);
    assert_eq!(session.format_infix(value), "12");
    assert_eq!(session.output(), &["7", "12"]);
}

#[test]
fn definitions_take_named_parameters() {
    let source = r#"
mod functions
def square(x)=x*x
def add(x,y)=x+y
def zero()=0
prt square(5)
prt add(2,3)
prt zero()
end
"#;
    assert_eq!(output(source), vec!["25", "5", "0"]);
}

#[test]
fn definitions_apply_to_expressions() {
    let source = r#"
mod functions
def square(x)=x*x
prt square(1+2)
prt square(square(2))
end
"#;
    assert_eq!(output(source), vec!["9", "16"]);
}

#[test]
fn conditionals_choose_one_branch() {
    let true_source = r#"
mod control
let x=5
if x>3 then
  prt x
else
  prt 0
end
end
"#;
    assert_eq!(output(true_source), vec!["5"]);
    let false_source = r#"
mod control
let x=1
if x>3 then
  prt x
else
  prt 0
end
end
"#;
    assert_eq!(output(false_source), vec!["0"]);
}

#[test]
fn conditionals_without_else() {
    let source = r#"
mod control
if 1<2 then
  prt 7
end
prt 8
end
"#;
    assert_eq!(output(source), vec!["7", "8"]);
}

#[test]
fn nested_conditionals_share_end() {
    let source = r#"
mod control
let x=2
if x>0 then
  if x>1 then
    prt 10
  else
    prt 20
  end
else
  prt 30
end
end
"#;
    assert_eq!(output(source), vec!["10"]);
}

#[test]
fn indentation_is_optional() {
    let flat = r#"
mod flat
let x=2
if x>0 then
prt x
else
prt 0
end
end
"#;
    assert_eq!(output(flat), vec!["2"]);
    let indented = r#"
mod flat
    let x=2
    if x>0 then
        prt x
    else
        prt 0
    end
end
"#;
    assert_eq!(output(indented), vec!["2"]);
}

#[test]
fn comments_and_blank_lines_are_ignored() {
    let source = r#"
// leading comment

mod comments
// inside module
let x = 1+2 // trailing comment
prt x

// before end
end
// after end
"#;
    assert_eq!(output(source), vec!["3"]);
}

#[test]
fn tabs_are_allowed_around_statements() {
    let source = "mod tabs\n\tlet x=1\n\tprt x\nend\n";
    assert_eq!(output(source), vec!["1"]);
}

#[test]
fn module_name_is_required_and_validated() {
    for source in ["mod\nend", "mod 9bad\nend", "mod good_name\nend"] {
        let mut session = Session::new();
        assert!(session.eval_source(source).is_err(), "{source}");
    }
}

#[test]
fn language_must_start_with_module() {
    for source in [
        "let x=1\nend",
        "def f(x)=x\nend",
        "prt 1\nend",
        "if 1<2 then\nend",
    ] {
        let mut session = Session::new();
        let error = session.eval_source(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::ParseUnexpectedToken, "{source}");
    }
}

#[test]
fn missing_end_is_an_error() {
    let source = "mod missing\nlet x=1\n";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
    assert_eq!(error.span.unwrap().line, 2);
}

#[test]
fn statements_after_module_end_are_rejected() {
    let source = "mod extra\nlet x=1\nend\nprt x\n";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
    assert_eq!(error.span.unwrap().line, 4);
}

#[test]
fn nested_modules_are_rejected() {
    let source = "mod outer\nmod inner\nend\nend";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn stray_end_and_else_are_rejected() {
    for source in ["mod m\nend\nend", "mod m\nelse\nend"] {
        let mut session = Session::new();
        let error = session.eval_source(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::ParseUnexpectedToken, "{source}");
    }
}

#[test]
fn missing_then_is_rejected() {
    let source = "mod m\nif 1<2\nprt 1\nend\nend";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
    assert_eq!(error.span.unwrap().line, 2);
}

#[test]
fn whitespace_inside_expressions_is_rejected() {
    let cases = [
        ("mod m\nlet x=1 + 2\nend", 2, 8),
        ("mod m\nlet x=1\t+2\nend", 2, 8),
        ("mod m\nprt 1 + 2\nend", 2, 6),
        ("mod m\nif 1 < 2 then\nend\nend", 2, 5),
        ("mod m\ndef f (x)=x\nend", 2, 6),
        ("mod m\ndef f(x)=x + 1\nend", 2, 11),
        ("mod m\ndef f(x, y)=x\nend", 2, 9),
        ("mod m\nprt f(x, y)\nend", 2, 9),
    ];
    for (source, line, column) in cases {
        let mut session = Session::new();
        let error = session.eval_source(source).unwrap_err();
        let span = error.span.unwrap();
        assert_eq!(span.line, line, "{source}");
        assert_eq!(span.column, column, "{source}");
    }
}

#[test]
fn whitespace_around_equals_and_keywords_is_allowed() {
    let source = "mod spacing\nlet x = 2\ndef f(y) = y+1\nprt f(x)\nend\n";
    assert_eq!(output(source), vec!["3"]);
}

#[test]
fn expression_parse_errors_keep_source_lines() {
    let source = "mod errors\nlet x=1+\nend";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.span.unwrap().line, 2);
}

#[test]
fn invalid_definition_signatures_are_rejected() {
    for source in [
        "mod m\ndef f=x\nend",
        "mod m\ndef f(x=x\nend",
        "mod m\ndef 9f(x)=x\nend",
    ] {
        let mut session = Session::new();
        assert!(session.eval_source(source).is_err(), "{source}");
    }
}

#[test]
fn intake_output_can_be_taken() {
    let mut session = Session::new();
    session.eval_source("mod o\nprt 1\nprt 2\nend").unwrap();
    assert_eq!(session.output(), &["1", "2"]);
    let taken = session.take_output();
    assert_eq!(taken, vec!["1", "2"]);
    assert!(session.output().is_empty());
}

#[test]
fn lowercase_keywords_are_required() {
    let source = "MOD upper\nPRT 1\nEND";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn zero_parameter_definitions_work() {
    let source = "mod zero\ndef value()=42\nprt value()\nprt value()+1\nend";
    assert_eq!(output(source), vec!["42", "43"]);
}
