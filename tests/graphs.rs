use mithqal::Session;

fn show(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

const PATH: &str = "graph([1,2,3,4],[[1,2],[2,3],[3,4]])";
const SPLIT: &str = "graph([1,2,3],[[1,2]])";
const TRIANGLE: &str = "graph([1,2,3],[[1,2],[1,3],[2,3]])";
const DAG: &str = "directedGraph([1,2,3],[[1,2],[2,3]])";
const CYCLE: &str = "directedGraph([1,2],[[1,2],[2,1]])";

#[test]
fn undirected_graphs() {
    assert_eq!(show(&format!("neighbors({PATH},2)")), "[1,3]");
    assert_eq!(show(&format!("degreeOf({PATH},2)")), "2");
    assert_eq!(show(&format!("hasEdge({PATH},1,2)")), "true");
    assert_eq!(show(&format!("hasEdge({PATH},2,1)")), "true");
    assert_eq!(show(&format!("reachable({PATH},1)")), "[1,2,3,4]");
    assert_eq!(show(&format!("pathExists({PATH},1,4)")), "true");
    assert_eq!(show(&format!("pathExists({PATH},1,9)")), "false");
    assert_eq!(show(&format!("isConnected({PATH})")), "true");
    assert_eq!(show(&format!("components({PATH})")), "[[1,2,3,4]]");
    assert_eq!(show(&format!("isTree({PATH})")), "true");
    assert_eq!(show(&format!("isComplete({PATH})")), "false");
}

#[test]
fn disconnected_graphs() {
    assert_eq!(show(&format!("isConnected({SPLIT})")), "false");
    assert_eq!(show(&format!("components({SPLIT})")), "[[3],[1,2]]");
    assert_eq!(show(&format!("isTree({SPLIT})")), "false");
    assert_eq!(show(&format!("reachable({SPLIT},3)")), "[3]");
}

#[test]
fn complete_graphs_are_not_trees() {
    assert_eq!(show(&format!("isComplete({TRIANGLE})")), "true");
    assert_eq!(show(&format!("isTree({TRIANGLE})")), "false");
    assert_eq!(show(&format!("isConnected({TRIANGLE})")), "true");
    assert_eq!(show(&format!("degreeOf({TRIANGLE},1)")), "2");
}

#[test]
fn directed_graphs() {
    assert_eq!(show(&format!("isDirected({DAG})")), "true");
    assert_eq!(show(&format!("isUndirected({DAG})")), "false");
    assert_eq!(show(&format!("neighbors({DAG},1)")), "[2]");
    assert_eq!(show(&format!("neighbors({DAG},3)")), "[]");
    assert_eq!(show(&format!("inDegree({DAG},2)")), "1");
    assert_eq!(show(&format!("inDegree({DAG},1)")), "0");
    assert_eq!(show(&format!("outDegree({DAG},1)")), "1");
    assert_eq!(show(&format!("outDegree({DAG},3)")), "0");
    assert_eq!(show(&format!("reachable({DAG},1)")), "[1,2,3]");
    assert_eq!(show(&format!("isAcyclic({DAG})")), "true");
    assert_eq!(show(&format!("isAcyclic({CYCLE})")), "false");
    assert_eq!(
        show(&format!("reachabilityRelation({DAG})")),
        "[[1,1],[1,2],[1,3],[2,2],[2,3],[3,3]]"
    );
}

#[test]
fn undirected_edges_are_normalized() {
    assert_eq!(show("edgesOf(graph([1,2],[[2,1]]))"), "[[1,2]]");
    assert_eq!(show("edgesOf(graph([1,2],[[2,1],[1,2]]))"), "[[1,2]]");
}
