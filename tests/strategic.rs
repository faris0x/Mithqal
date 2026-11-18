use mithqal::Session;

fn show(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

const COORDINATION: &str =
    "game([a,b],[a,b],setOf([[[a,a],[1,1]],[[a,b],[0,0]],[[b,a],[0,0]],[[b,b],[2,2]]]))";
const PENNIES: &str =
    "game([h,t],[h,t],setOf([[[h,h],[1,-1]],[[h,t],[-1,1]],[[t,h],[-1,1]],[[t,t],[1,-1]]]))";
const DOMINANT: &str =
    "game([a,b],[x,y],setOf([[[a,x],[2,0]],[[a,y],[2,1]],[[b,x],[0,0]],[[b,y],[0,1]]]))";

const UTILITY: &str = "fn(action,state)=>if(same(action,a),state,10-state)";

#[test]
fn decision_theory_uses_expected_utility() {
    assert_eq!(
        show(&format!("expectedUtility(uniform([1,2]),a,{UTILITY})")),
        "3/2"
    );
    assert_eq!(
        show(&format!("bestActions(uniform([1,2]),[a,b],{UTILITY})")),
        "[b]"
    );
    assert_eq!(
        show(&format!("decisionValue(uniform([1,2]),[a,b],{UTILITY})")),
        "17/2"
    );
    assert_eq!(
        show(&format!(
            "minimaxRegretActions(uniform([1,2]),[a,b],{UTILITY})"
        )),
        "[b]"
    );
}

#[test]
fn best_actions_include_ties() {
    assert_eq!(
        show("bestActions(uniform([1,2]),[a,b],fn(action,state)=>state)"),
        "[a,b]"
    );
}

#[test]
fn coordination_games_have_two_pure_equilibria() {
    assert_eq!(show(&format!("pureNash({COORDINATION})")), "[[a,a],[b,b]]");
    assert_eq!(show(&format!("paretoOptimal({COORDINATION})")), "[[b,b]]");
    assert_eq!(show(&format!("bestRows({COORDINATION},a)")), "[a]");
    assert_eq!(show(&format!("bestRows({COORDINATION},b)")), "[b]");
    assert_eq!(show(&format!("dominantRowActions({COORDINATION})")), "[]");
}

#[test]
fn zero_sum_games_are_detected() {
    assert_eq!(show(&format!("isZeroSum({PENNIES})")), "true");
    assert_eq!(show(&format!("pureNash({PENNIES})")), "[]");
    assert_eq!(show(&format!("isZeroSum({COORDINATION})")), "false");
}

#[test]
fn dominant_strategies_and_equilibria() {
    assert_eq!(show(&format!("dominantRowActions({DOMINANT})")), "[a]");
    assert_eq!(show(&format!("dominantColumnActions({DOMINANT})")), "[y]");
    assert_eq!(show(&format!("pureNash({DOMINANT})")), "[[a,y]]");
    assert_eq!(show(&format!("paretoOptimal({DOMINANT})")), "[[a,y]]");
}
