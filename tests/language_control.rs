use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_source(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn strings_are_literals_and_indexable() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "\"a\\nb\""), "\"a\\nb\"");
    assert_eq!(eval(&mut session, "\"abc\"[[2]]"), "\"b\"");
    assert_eq!(eval(&mut session, "\"abc\"[[-1]]"), "\"c\"");
}

#[test]
fn byte_arrays_are_literals_and_indexable() {
    let mut session = Session::new();
    assert_eq!(
        eval(&mut session, "byteArray(1,2,255)"),
        "byteArray(1,2,255)"
    );
    assert_eq!(eval(&mut session, "byteArray(1,2,255)[[3]]"), "255");
}

#[test]
fn association_literals_sort_and_lookup() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "<|b->2,a->1|>"), "<|a->1,b->2|>");
    assert_eq!(eval(&mut session, "<|a->1,b->2|>[[b]]"), "2");
    assert_eq!(eval(&mut session, "<|a->1|>[[missing]]"), "undefined");
}

#[test]
fn indexing_works_on_lists_and_calls() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "[10,20,30][[2]]"), "20");
    assert_eq!(eval(&mut session, "[10,20,30][[-1]]"), "30");
    assert_eq!(eval(&mut session, "f(a,b)[[1]]"), "a");
    assert_eq!(eval(&mut session, "f(a,b)[[0]]"), "f");
    assert_eq!(eval(&mut session, "[[1,2],[3,4]][[2]][[1]]"), "3");
}

#[test]
fn spans_select_ranges() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "[10,20,30,40][[span(2,3)]]"), "[20,30]");
    assert_eq!(eval(&mut session, "\"abcd\"[[span(2,3)]]"), "\"bc\"");
    assert_eq!(
        eval(&mut session, "byteArray(1,2,3)[[span(2,3)]]"),
        "byteArray(2,3)"
    );
    assert_eq!(eval(&mut session, "f(a,b,c)[[span(2,3)]]"), "f(b,c)");
}

#[test]
fn part_assignment_updates_lists_and_nested_parts() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "x=[1,2,3]\nx[[2]]=9\nx"), "[1,9,3]");
    assert_eq!(
        eval(&mut session, "x=[[1,2],[3,4]]\nx[[2]][[1]]=9\nx"),
        "[[1,2],[9,4]]"
    );
    assert_eq!(eval(&mut session, "x=[1,2]\nx[[-1]]=7\nx"), "[1,7]");
}

#[test]
fn part_assignment_updates_associations() {
    let mut session = Session::new();
    assert_eq!(
        eval(&mut session, "a=<|x->1|>\na[[y]]=2\na"),
        "<|x->1,y->2|>"
    );
    assert_eq!(eval(&mut session, "a=<|x->1|>\na[[x]]=9\na"), "<|x->9|>");
}

#[test]
fn part_assignment_updates_strings_and_bytes() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "s=\"abc\"\ns[[2]]=\"x\"\ns"), "\"axc\"");
    assert_eq!(
        eval(&mut session, "b=byteArray(1,2)\nb[[2]]=9\nb"),
        "byteArray(1,9)"
    );
}

#[test]
fn mutation_helpers_update_values() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "x=[1,2]\nappendTo(x,3)\nx"), "[1,2,3]");
    assert_eq!(eval(&mut session, "x=[1,2]\nprependTo(x,0)\nx"), "[0,1,2]");
    assert_eq!(eval(&mut session, "x=5\naddTo(x,3)\nx"), "8");
    assert_eq!(eval(&mut session, "x=5\nsubtractFrom(x,3)\nx"), "2");
    assert_eq!(eval(&mut session, "x=5\ntimesBy(x,3)\nx"), "15");
    assert_eq!(eval(&mut session, "x=10\ndivideBy(x,2)\nx"), "5");
}

#[test]
fn hold_evaluate_and_sequence_are_active() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "hold(1+2)"), "hold(1+2)");
    assert_eq!(eval(&mut session, "hold(evaluate(1+2))"), "hold(3)");
    assert_eq!(eval(&mut session, "holdForm(1+2)"), "holdForm(1+2)");
    assert_eq!(eval(&mut session, "[sequence(1,2),3]"), "[1,2,3]");
    assert_eq!(
        eval(&mut session, "hold(unevaluated(1+2))"),
        "hold(unevaluated(1+2))"
    );
}

#[test]
fn which_and_switch_select_branches() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "which(false,1,true,2)"), "2");
    assert_eq!(eval(&mut session, "x=2\nswitch(x,1,10,2,20,99)"), "20");
    assert_eq!(eval(&mut session, "x=3\nswitch(x,1,10,2,20,99)"), "99");
    assert_eq!(eval(&mut session, "switch(5,y_,y+1)"), "6");
}

#[test]
fn loops_and_compound_evaluate_in_order() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "x=0\ndo(x=x+1,3)\nx"), "3");
    assert_eq!(eval(&mut session, "i=0\nwhile(i<3,i=i+1)\ni"), "3");
    assert_eq!(
        eval(&mut session, "x=0\ni=0\nfor(i=0,i<3,i=i+1,x=x+i)\nx"),
        "3"
    );
    assert_eq!(eval(&mut session, "x=0\ncompound(x=1,x=2,x)"), "2");
}

#[test]
fn break_and_continue_control_loops() {
    let mut session = Session::new();
    assert_eq!(
        eval(
            &mut session,
            "i=0\nx=0\ndo(compound(i=i+1,if(i==2,break(),x=x+i)),5)\nx"
        ),
        "1"
    );
    assert_eq!(
        eval(
            &mut session,
            "i=0\nx=0\ndo(compound(i=i+1,if(i==2,continue(),x=x+i)),3)\nx"
        ),
        "4"
    );
}

#[test]
fn return_exits_the_enclosing_function() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "f(x_):=if(x>0,return(x),0)\nf(5)"), "5");
    assert_eq!(eval(&mut session, "f(x_):=if(x>0,return(x),0)\nf(-5)"), "0");
}

#[test]
fn throw_and_catch_transfer_values() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "catch(throw(5))"), "5");
    assert_eq!(eval(&mut session, "catch(throw(5,tag),tag)"), "5");
    assert!(session.eval_statement("throw(5)").is_err());
    assert!(session.eval_statement("catch(throw(5,other),tag)").is_err());
}

#[test]
fn unset_and_clear_remove_definitions() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "x=1\nunset(x)\nx"), "x");
    assert_eq!(eval(&mut session, "x=1\nclear(x)\nx"), "x");
    assert_eq!(eval(&mut session, "f(x_):=x\nclear(f)\nf(2)"), "f(2)");
    assert_eq!(eval(&mut session, "f(x_):=x\nunset(f(2))\nf(2)"), "f(2)");
}

#[test]
fn delayed_part_assignment_is_rejected() {
    let mut session = Session::new();
    assert!(session.eval_statement("x=[1,2]\nx[[1]]:=3").is_err());
}
