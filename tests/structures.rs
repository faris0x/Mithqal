use mithqal::{ErrorCode, Session};

const MONOID: &str = r#"
mod s
structure testMonoid
field carrier
field op
field identity
law associative: forall(a,carrier,forall(b,carrier,forall(c,carrier,same(op(op(a,b),c),op(a,op(b,c))))))
law neutral: forall(a,carrier,same(op(a,identity),a))
end
instance cyclic : testMonoid
carrier = [0,1,2]
op = fn(a,b)=>mod(a+b,3)
identity = 0
end
prt testMonoidHolds(cyclic)
prt testMonoidCarrier(cyclic)
prt testMonoidIdentity(cyclic)
prt apply(testMonoidOp(cyclic),[1,2])
end
"#;

#[test]
fn structure_instances_check_laws() {
    let mut session = Session::new();
    session.eval_source(MONOID).unwrap();
    assert_eq!(session.output(), &["true", "[0,1,2]", "0", "0"]);
}

#[test]
fn accessors_and_law_checks_work_after_declaration() {
    let mut session = Session::new();
    session.eval_source(MONOID).unwrap();
    let value = session.eval_statement("testMonoidHolds(cyclic)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "true");
    let value = session.eval_statement("testMonoidCarrier(cyclic)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "[0,1,2]");
    let value = session
        .eval_statement("apply(testMonoidOp(cyclic),[1,2])")
        .unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "0");
}

#[test]
fn failing_laws_are_rejected() {
    let source = r#"
mod s
structure testMonoid
field carrier
field op
field identity
law associative: forall(a,carrier,forall(b,carrier,forall(c,carrier,same(op(op(a,b),c),op(a,op(b,c))))))
end
instance bad : testMonoid
carrier = [0,1,2]
op = fn(a,b)=>mod(a-b,3)
identity = 0
end
end
"#;
    let mut session = Session::new();
    let error = session.eval_source(source).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn assumed_instances_skip_law_checking() {
    let source = r#"
mod s
structure testMonoid
field carrier
field op
field identity
law associative: forall(a,carrier,forall(b,carrier,forall(c,carrier,same(op(op(a,b),c),op(a,op(b,c))))))
end
instance symbolic : testMonoid assumed
carrier = y
op = fn(a,b)=>a+b
identity = 0
end
end
"#;
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    let value = session
        .eval_statement("testMonoidCarrier(symbolic)")
        .unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "y");
}

#[test]
fn missing_and_unknown_fields_are_rejected() {
    let missing = r#"
mod s
structure testPair
field left
field right
end
instance broken : testPair
left = 1
end
end
"#;
    let mut session = Session::new();
    let error = session.eval_source(missing).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);

    let unknown = r#"
mod s
structure testPair
field left
field right
end
instance broken : testPair
left = 1
right = 2
extra = 3
end
end
"#;
    let mut session = Session::new();
    let error = session.eval_source(unknown).unwrap_err();
    assert_eq!(error.code, ErrorCode::EvalDefinition);
}

#[test]
fn strict_modules_keep_structure_names_private() {
    let source = r#"
mod s
export cyclic
structure testMonoid
field carrier
field op
field identity
law associative: forall(a,carrier,forall(b,carrier,forall(c,carrier,same(op(op(a,b),c),op(a,op(b,c))))))
end
instance cyclic : testMonoid
carrier = [0,1,2]
op = fn(a,b)=>mod(a+b,3)
identity = 0
end
end
"#;
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    let value = session.eval_statement("cyclic").unwrap();
    assert!(session.format_infix(value).unwrap().starts_with('['));
    let value = session.eval_statement("testMonoidHolds(cyclic)").unwrap();
    let text = session.format_infix(value).unwrap();
    assert!(text.starts_with("testMonoidHolds("), "{text}");
}
