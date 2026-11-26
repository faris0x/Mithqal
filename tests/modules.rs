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
module strict.
export public.
we define public(x) = helper(x) + 1.
we define helper(x) = x * 2.
end module.
"#;
    assert_eq!(evaluated(source, "public(3)"), "7");
    assert_eq!(evaluated(source, "helper(3)"), "helper(3)");
}

#[test]
fn private_names_from_different_modules_do_not_collide() {
    let mut session = Session::new();
    session
        .eval_source("module a.\nexport fa.\nwe define fa(x) = helper(x).\nwe define helper(x) = x + 1.\nend module.")
        .unwrap();
    session
        .eval_source("module b.\nexport fb.\nwe define fb(x) = helper(x).\nwe define helper(x) = x + 2.\nend module.")
        .unwrap();
    let left = session.eval_statement("fa(1)").unwrap();
    assert_eq!(session.format_infix(left).unwrap(), "2");
    let right = session.eval_statement("fb(1)").unwrap();
    assert_eq!(session.format_infix(right).unwrap(), "3");
}

#[test]
fn modules_without_exports_keep_public_names() {
    let source = "module plain.\nwe define f(x) = x + 1.\nend module.";
    assert_eq!(evaluated(source, "f(1)"), "2");
}

#[test]
fn exporting_an_undefined_name_is_rejected() {
    let mut session = Session::new();
    let error = session
        .eval_source("module broken.\nexport missing.\nend module.")
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn strict_module_definitions_are_protected() {
    let mut session = Session::new();
    session
        .eval_source("module strict.\nexport f.\nwe define f(x) = x.\nend module.")
        .unwrap();
    let error = session.eval_statement("f=1").unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
}

#[test]
fn exported_names_cannot_be_redefined_by_another_module() {
    let mut session = Session::new();
    session
        .eval_source("module first.\nexport f.\nwe define f(x) = x.\nend module.")
        .unwrap();
    let error = session
        .eval_source("module second.\nexport f.\nwe define f(x) = x + 1.\nend module.")
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalProtected);
}

#[test]
fn imports_are_accepted_at_module_level() {
    let source = "module uses.\nexport result.\nuse statistics.\nwe define result() = mean([1, 2]).\nend module.";
    assert_eq!(evaluated(source, "result()"), "3/2");
}

#[test]
fn imports_are_rejected_inside_blocks() {
    let source = "module uses.\nif true then:\n  use statistics.\n  print 1.\nelse:\n  print 0.\nend if.\nend module.";
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn statistics_helpers_are_private() {
    assert_eq!(
        output("module probe.\nprint sum([1, 2]).\nprint numeric([1]).\nend module."),
        vec!["sum([1,2])", "numeric([1])"]
    );
}
