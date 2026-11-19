use mithqal::Session;

#[test]
fn message_records_and_drains_messages() {
    let mut session = Session::new();
    let value = session.eval_statement("message(foo,1)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "null");
    let messages = session.take_messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].code, "TGN-MSG-0001");
    assert_eq!(messages[0].name, "foo");
    assert!(messages[0].text.contains("foo"));
    assert!(session.take_messages().is_empty());
}

#[test]
fn message_name_uses_the_tag_form() {
    let mut session = Session::new();
    let _ = session
        .eval_statement("message(messageName(f,tag),1)")
        .unwrap();
    let messages = session.take_messages();
    assert_eq!(messages[0].name, "f::tag");
}

#[test]
fn check_returns_fallback_after_a_message() {
    let mut session = Session::new();
    let value = session.eval_statement("check(message(foo,1),2)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "2");
    assert_eq!(session.take_messages().len(), 1);
    let value = session.eval_statement("check(1+1,0)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "2");
    assert!(session.take_messages().is_empty());
}

#[test]
fn quiet_suppresses_messages() {
    let mut session = Session::new();
    let _ = session.eval_statement("quiet(message(foo,1))").unwrap();
    assert!(session.take_messages().is_empty());
}

#[test]
fn off_and_on_control_named_messages() {
    let mut session = Session::new();
    let _ = session.eval_statement("off(foo)").unwrap();
    let _ = session.eval_statement("message(foo,1)").unwrap();
    assert!(session.take_messages().is_empty());
    let _ = session.eval_statement("on(foo)").unwrap();
    let _ = session.eval_statement("message(foo,1)").unwrap();
    assert_eq!(session.take_messages().len(), 1);
}
