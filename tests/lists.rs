use mithqal::Session;

fn text(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn list_primitives() {
    assert_eq!(text("length([1,2,3])"), "3");
    assert_eq!(text("length([])"), "0");
    assert_eq!(text("first([1,2,3])"), "1");
    assert_eq!(text("rest([1,2,3])"), "[2,3]");
    assert_eq!(text("rest([1])"), "[]");
    assert_eq!(text("cons(0,[1,2])"), "[0,1,2]");
    assert_eq!(text("cons(0,[])"), "[0]");
    assert_eq!(text("nth([4,5,6],1)"), "5");
    assert_eq!(text("reverse([1,2,3])"), "[3,2,1]");
}

#[test]
fn sort_uses_numeric_order() {
    assert_eq!(text("sort([3,1,2])"), "[1,2,3]");
    assert_eq!(text("sort([3/2,1,2])"), "[1,3/2,2]");
    assert_eq!(text("sort([-1,1/2,0])"), "[-1,0,1/2]");
}

#[test]
fn number_predicate() {
    assert_eq!(text("isNumber(3)"), "true");
    assert_eq!(text("isNumber(1/2)"), "true");
    assert_eq!(text("isNumber(x)"), "false");
    assert_eq!(text("isNumber([1])"), "false");
}

#[test]
fn invalid_list_arguments_stay_unevaluated() {
    for source in [
        "first([])",
        "length(x)",
        "cons(0,x)",
        "nth([1],5)",
        "nth([1],-1)",
    ] {
        assert!(
            text(source).contains(source.split('(').next().unwrap()),
            "{source}"
        );
    }
}

#[test]
fn higher_order_forms_evaluate_computed_lists() {
    assert_eq!(text("fold(fn(a,b)=>a+b,0,map([1,2],fn(x)=>x+1))"), "5");
    assert_eq!(text("apply(fn(x,y)=>x+y,rest([9,3,4]))"), "7");
    assert_eq!(text("nest(fn(x)=>x+1,0,3)"), "3");
}
