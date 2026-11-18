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

fn source_output(source: &str) -> Vec<String> {
    let mut session = Session::new();
    session.eval_source(source).unwrap();
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
    assert_eq!(session.format_infix(value).unwrap(), "12");
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
    assert_eq!(error.span.as_deref().unwrap().line, 2);
}

#[test]
fn statements_after_module_end_are_rejected() {
    let source = "mod extra\nlet x=1\nend\nprt x\n";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
    assert_eq!(error.span.as_deref().unwrap().line, 4);
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
    assert_eq!(error.span.as_deref().unwrap().line, 2);
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
        let span = error.span.as_deref().unwrap();
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
    assert_eq!(error.span.as_deref().unwrap().line, 2);
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

#[test]
fn literal_parameter_patterns_enable_recursion() {
    let source = r#"
mod recursive
def fact(0)=1
def fact(n)=n*fact(n-1)
prt fact(0)
prt fact(1)
prt fact(5)
prt fact(7)
end
"#;
    assert_eq!(source_output(source), vec!["1", "1", "120", "5040"]);
}

#[test]
fn multiple_literal_parameter_patterns() {
    let source = r#"
mod patterns
def f(0,0)=0
def f(0,n)=n
def f(m,0)=m
def f(m,n)=m+n
prt f(0,0)
prt f(0,3)
prt f(4,0)
prt f(4,5)
end
"#;
    assert_eq!(source_output(source), vec!["0", "3", "4", "9"]);
}

#[test]
fn guarded_definitions() {
    let source = r#"
mod guards
def sign(x) when x<0=-1
def sign(x) when x==0=0
def sign(x)=1
prt sign(-5)
prt sign(-1)
prt sign(0)
prt sign(1)
prt sign(5)
end
"#;
    assert_eq!(source_output(source), vec!["-1", "-1", "0", "1", "1"]);
}

#[test]
fn guarded_definitions_use_other_parameters() {
    let source = r#"
mod guards
def clamp(x,low,high) when x<low=low
def clamp(x,low,high) when x>high=high
def clamp(x,low,high)=x
prt clamp(-5,0,10)
prt clamp(15,0,10)
prt clamp(5,0,10)
end
"#;
    assert_eq!(source_output(source), vec!["0", "10", "5"]);
}

#[test]
fn bindings_are_block_scoped() {
    let source = r#"
mod scopes
let x=1
if true then
  let x=2
  let y=3
  prt x
  prt y
end
prt x
if false then
  let x=9
else
  let x=7
  prt x
end
prt x
end
"#;
    assert_eq!(source_output(source), vec!["2", "3", "1", "7", "1"]);
}

#[test]
fn definitions_capture_outer_bindings() {
    let source = r#"
mod capture
let base=10
def addBase(x)=x+base
prt addBase(5)
if true then
  let base=20
  def localAdd(x)=x+base
  prt localAdd(5)
end
prt addBase(5)
end
"#;
    assert_eq!(source_output(source), vec!["15", "25", "15"]);
}

#[test]
fn match_with_literal_and_name_cases() {
    let source = r#"
mod matching
let x=2
match x
case 0
  prt 100
case 1
  prt 200
case n
  prt n+10
else
  prt -1
end
end
"#;
    assert_eq!(source_output(source), vec!["12"]);
}

#[test]
fn match_falls_back_to_else() {
    let source = r#"
mod matching
let x=9
match x
case 0
  prt 0
case 1
  prt 1
else
  prt 99
end
end
"#;
    assert_eq!(source_output(source), vec!["99"]);
}

#[test]
fn match_literal_can_be_an_expression() {
    let source = r#"
mod matching
match 3
case 1+2
  prt 7
else
  prt 8
end
end
"#;
    assert_eq!(source_output(source), vec!["7"]);
}

#[test]
fn match_case_bindings_are_scoped() {
    let source = r#"
mod matching
let n=100
match 5
case n
  prt n
end
prt n
end
"#;
    assert_eq!(source_output(source), vec!["5", "100"]);
}

#[test]
fn nested_matches_share_end() {
    let source = r#"
mod matching
match 1
case 1
  match 2
  case 2
    prt 12
  else
    prt 10
  end
else
  prt 0
end
end
"#;
    assert_eq!(source_output(source), vec!["12"]);
}

#[test]
fn match_requires_a_case() {
    let source = "mod m\nmatch 1\nend";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn case_outside_match_is_rejected() {
    let source = "mod m\ncase 1\nend";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseUnexpectedToken);
}

#[test]
fn guard_requires_when_keyword() {
    let source = "mod m\ndef f(x) x>0=x\nend";
    let mut session = Session::new();
    assert!(session.eval_source(source).is_err());
}

#[test]
fn guard_expression_is_whitespace_free() {
    let source = "mod m\ndef f(x) when x > 0=x\nend";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::ParseWhitespace);
}

#[test]
fn low_gc_threshold_keeps_let_bindings_alive() {
    let config = mithqal::Config {
        gc_threshold_nodes: 1,
        ..mithqal::Config::default()
    };
    let mut session = Session::with_config(config);
    let source = r#"
mod gc
let x=(a+b+c+d)*(e+f+g+h)
let y=1
prt x
end
"#;
    session.eval_language(source).unwrap();
    assert_eq!(session.output(), &["(a+b+c+d)*(e+f+g+h)"]);
}

#[test]
fn low_gc_threshold_keeps_match_scrutinees_alive() {
    let config = mithqal::Config {
        gc_threshold_nodes: 1,
        ..mithqal::Config::default()
    };
    let mut session = Session::with_config(config);
    let source = r#"
mod gc
let x=(a+b+c+d)*(e+f+g+h)
match x
case (a+b+c+d)*(e+f+g+h)
  prt 7
else
  prt 8
end
end
"#;
    session.eval_language(source).unwrap();
    assert_eq!(session.output(), &["7"]);
}

#[test]
fn low_gc_threshold_keeps_nested_scope_values_alive() {
    let config = mithqal::Config {
        gc_threshold_nodes: 1,
        ..mithqal::Config::default()
    };
    let mut session = Session::with_config(config);
    let source = r#"
mod gc
let x=(a+b+c+d)*(e+f+g+h)
if 1<2 then
  let y=1
  prt x
end
end
"#;
    session.eval_language(source).unwrap();
    assert_eq!(session.output(), &["(a+b+c+d)*(e+f+g+h)"]);
}
