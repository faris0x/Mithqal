use mithqal::{ErrorCode, Session};

fn output(source: &str) -> Vec<String> {
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    session.output().to_vec()
}

fn evaluated(source: &str, expression: &str) -> String {
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    let value = session.eval_statement(expression).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn strict_modules_hide_private_definitions() {
    let source = r#"
mod strict
export public
def public(x)=helper(x)+1
def helper(x)=x*2
end
"#;
    assert_eq!(evaluated(source, "public(3)"), "7");
    assert_eq!(evaluated(source, "helper(3)"), "helper(3)");
}

#[test]
fn private_names_from_different_modules_do_not_collide() {
    let mut session = Session::new();
    session
        .eval_source("mod a\nexport fa\ndef fa(x)=helper(x)\ndef helper(x)=x+1\nend\n")
        .unwrap();
    session
        .eval_source("mod b\nexport fb\ndef fb(x)=helper(x)\ndef helper(x)=x+2\nend\n")
        .unwrap();
    let left = session.eval_statement("fa(1)").unwrap();
    assert_eq!(session.format_infix(left).unwrap(), "2");
    let right = session.eval_statement("fb(1)").unwrap();
    assert_eq!(session.format_infix(right).unwrap(), "3");
}

#[test]
fn modules_without_exports_keep_public_names() {
    let source = "mod plain\ndef f(x)=x+1\nend\n";
    assert_eq!(evaluated(source, "f(1)"), "2");
}

#[test]
fn exporting_an_undefined_name_is_rejected() {
    let mut session = Session::new();
    let error = session
        .eval_source("mod broken\nexport missing\nend\n")
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn strict_module_definitions_are_protected() {
    let mut session = Session::new();
    session
        .eval_source("mod strict\nexport f\ndef f(x)=x\nend\n")
        .unwrap();
    let error = session.eval_statement("f=1").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
}

#[test]
fn exported_names_cannot_be_redefined_by_another_module() {
    let mut session = Session::new();
    session
        .eval_source("mod first\nexport f\ndef f(x)=x\nend\n")
        .unwrap();
    let error = session
        .eval_source("mod second\nexport f\ndef f(x)=x+1\nend\n")
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
}

#[test]
fn imports_are_accepted_at_module_level() {
    let source = "mod uses\nexport result\nimport statistics\ndef result()=mean([1,2])\nend\n";
    assert_eq!(evaluated(source, "result()"), "3/2");
}

#[test]
fn imports_are_rejected_inside_blocks() {
    let source = "mod uses\nif true then\nimport statistics\nprt 1\nelse\nprt 0\nend\nend\n";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn statistics_helpers_are_private() {
    assert_eq!(
        output("mod probe\nprt sum([1,2])\nprt numeric([1])\nend\n"),
        vec!["sum([1,2])", "numeric([1])"]
    );
}
