use mithqal::Session;

fn show(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn finite_quantifiers_decide() {
    assert_eq!(show("forall(x,[1,2,3],x>0)"), "true");
    assert_eq!(show("forall(x,[1,2,3],x>1)"), "false");
    assert_eq!(show("exists(x,[1,2,3],x>2)"), "true");
    assert_eq!(show("exists(x,[1,2],x>2)"), "false");
    assert_eq!(show("forall(x,[],x>0)"), "true");
    assert_eq!(show("exists(x,[],x>0)"), "false");
}

#[test]
fn set_builder_returns_a_canonical_set() {
    assert_eq!(show("setBuilder(x,[1,2,3,4],isEven(x))"), "[2,4]");
    assert_eq!(show("setBuilder(x,[3,1,2],x>0)"), "[1,2,3]");
    assert_eq!(show("setBuilder(x,[],x>0)"), "[]");
}

#[test]
fn binders_are_canonical_and_alpha_equivalent() {
    assert_eq!(show("forall(x,y,x>0)"), "forall(v0,y,v0>0)");
    assert_eq!(show("same(forall(x,y,x>0),forall(z,y,z>0))"), "true");
    assert_eq!(show("setBuilder(x,y,x>0)"), "setBuilder(v0,y,v0>0)");
}

#[test]
fn nested_binders_evaluate_over_finite_domains() {
    assert_eq!(show("forall(x,[1,2],exists(y,[1,2],y>x))"), "false");
    assert_eq!(show("forall(x,[1],exists(y,[x],y>x))"), "false");
    assert_eq!(show("exists(x,[1,2],forall(y,[1,2],y>=x))"), "true");
}

#[test]
fn undecidable_bodies_remain_symbolic() {
    assert_eq!(show("forall(x,[1,2],x>y)"), "forall(v0,[1,2],v0>y)");
    assert_eq!(show("setBuilder(x,[1,2],x>y)"), "setBuilder(v0,[1,2],v0>y)");
}
