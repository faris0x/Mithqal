use mithqal::{ErrorCode, Session};

fn evaluate(source: &str) -> (Session, u32) {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    (session, value)
}

fn text(source: &str) -> String {
    let (session, value) = evaluate(source);
    session.format_infix(value)
}

#[test]
fn determinants() {
    assert_eq!(text("det([[7]])"), "7");
    assert_eq!(text("det([[1,2],[3,4]])"), "-2");
    assert_eq!(text("det([[1,2,3],[4,5,6],[7,8,10]])"), "-3");
    assert_eq!(text("det([[2,0],[0,3]])"), "6");
    assert_eq!(text("det([[1,2],[2,4]])"), "0");
    assert_eq!(text("det([[1/2,0],[0,2]])"), "1");
    assert_eq!(text("det(identity(3))"), "1");
    assert_eq!(text("det([[1,2,3],[4,5,6]])"), "det([[1,2,3],[4,5,6]])");
}

#[test]
fn ranks() {
    assert_eq!(text("rank([[1,2],[2,4]])"), "1");
    assert_eq!(text("rank([[1,2],[3,4]])"), "2");
    assert_eq!(text("rank([[0,0],[0,0]])"), "0");
    assert_eq!(text("rank(identity(4))"), "4");
    assert_eq!(text("rank([[1,2,3],[2,4,6]])"), "1");
}

#[test]
fn transposes() {
    assert_eq!(text("transpose([[1,2,3],[4,5,6]])"), "[[1,4],[2,5],[3,6]]");
    assert_eq!(text("transpose([[1,2],[3,4]])"), "[[1,3],[2,4]]");
    assert_eq!(text("transpose([[1]])"), "[[1]]");
}

#[test]
fn inverses() {
    assert_eq!(text("inverse([[1,2],[3,4]])"), "[[-2,1],[3/2,-1/2]]");
    assert_eq!(text("inverse([[2,0],[0,4]])"), "[[1/2,0],[0,1/4]]");
    assert_eq!(text("inverse(identity(3))"), "[[1,0,0],[0,1,0],[0,0,1]]");
    assert_eq!(
        text("matmul([[1,2],[3,4]],inverse([[1,2],[3,4]]))"),
        "[[1,0],[0,1]]"
    );
    assert!(text("inverse([[1,2],[2,4]])").contains("inverse"));
}

#[test]
fn reduced_row_echelon_forms() {
    assert_eq!(text("rref([[1,2],[3,4]])"), "[[1,0],[0,1]]");
    assert_eq!(text("rref([[1,2,3],[2,4,6]])"), "[[1,2,3],[0,0,0]]");
    assert_eq!(text("rref([[0,1],[1,0]])"), "[[1,0],[0,1]]");
    assert_eq!(text("rref([[1,2,3],[4,5,6]])"), "[[1,0,-1],[0,1,2]]");
}

#[test]
fn traces() {
    assert_eq!(text("trace([[1,2],[3,4]])"), "5");
    assert_eq!(text("trace(identity(5))"), "5");
    assert_eq!(text("trace([[1,2,3],[4,5,6],[7,8,9]])"), "15");
}

#[test]
fn matrix_multiplication() {
    assert_eq!(
        text("matmul([[1,2],[3,4]],[[5,6],[7,8]])"),
        "[[19,22],[43,50]]"
    );
    assert_eq!(text("matmul(identity(2),[[1,2],[3,4]])"), "[[1,2],[3,4]]");
    assert_eq!(text("matmul([[1,2,3]],[[1],[2],[3]])"), "[[14]]");
    assert!(text("matmul([[1,2]],[[1,2]])").contains("matmul"));
}

#[test]
fn identity_matrices() {
    assert_eq!(text("identity(0)"), "[]");
    assert_eq!(text("identity(1)"), "[[1]]");
    assert_eq!(text("identity(2)"), "[[1,0],[0,1]]");
    assert_eq!(text("identity(3)"), "[[1,0,0],[0,1,0],[0,0,1]]");
}

#[test]
fn linear_systems() {
    assert_eq!(text("solveLinear([[2,1],[1,3]],[5,7])"), "[8/5,9/5]");
    assert_eq!(text("solveLinear([[1,0],[0,1]],[3,4])"), "[3,4]");
    assert_eq!(text("solveLinear([[1,1],[1,-1]],[4,0])"), "[2,2]");
    assert!(text("solveLinear([[1,2],[2,4]],[3,6])").contains("solveLinear"));
    assert!(text("solveLinear([[1,2],[2,4]],[3,7])").contains("solveLinear"));
    assert!(text("solveLinear([[1,2,3]],[1])").contains("solveLinear"));
}

#[test]
fn vector_operations() {
    assert_eq!(text("dot([1,2,3],[4,5,6])"), "32");
    assert_eq!(text("dot([1,0],[0,1])"), "0");
    assert_eq!(text("cross([1,0,0],[0,1,0])"), "[0,0,1]");
    assert_eq!(text("cross([1,2,3],[4,5,6])"), "[-3,6,-3]");
    assert!(text("dot([1,2],[1])").contains("dot"));
    assert!(text("cross([1,2],[3,4])").contains("cross"));
}

#[test]
fn non_matrix_arguments_stay_unevaluated() {
    for source in [
        "det(x)",
        "rank(1)",
        "transpose([1,2,3])",
        "trace([[1,2,3],[4,5,6]])",
        "dot([1,2],[1,2,3])",
    ] {
        let result = text(source);
        assert!(
            result.contains("det")
                || result.contains("rank")
                || result.contains("transpose")
                || result.contains("inverse")
                || result.contains("trace")
                || result.contains("dot"),
            "{source} => {result}"
        );
    }
}

#[test]
fn linalg_arity_errors() {
    for source in [
        "det()",
        "det([[1]],[[2]])",
        "rank()",
        "transpose()",
        "inverse()",
        "rref()",
        "trace()",
        "matmul([[1]])",
        "identity()",
        "solveLinear([[1]])",
        "dot([1])",
        "cross([1])",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "{source}");
    }
}

#[test]
fn linalg_symbols_are_protected() {
    for name in [
        "det",
        "rank",
        "transpose",
        "inverse",
        "rref",
        "trace",
        "matmul",
        "identity",
        "solveLinear",
        "dot",
        "cross",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(&format!("{name}=1")).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "{name}");
    }
}

#[test]
fn linalg_works_from_the_language() {
    let source = r#"
mod matrix
let a=[[1,2],[3,4]]
prt det(a)
prt transpose(a)
prt matmul(a,identity(2))
end
"#;
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    assert_eq!(session.output(), &["-2", "[[1,3],[2,4]]", "[[1,2],[3,4]]"]);
}

#[test]
fn symbolic_determinants() {
    assert_eq!(text("det([[a,b],[c,d]])"), "-b*c+a*d");
    assert_eq!(
        text("det([[a,b,c],[d,e,f],[g,h,i]])"),
        "-b*(-f*g+d*i)+a*(-f*h+e*i)+c*(-e*g+d*h)"
    );
    assert_eq!(text("det([[a,0],[0,d]])"), "a*d");
}

#[test]
fn symbolic_transposes_traces_and_products() {
    assert_eq!(text("transpose([[a,b],[c,d]])"), "[[a,c],[b,d]]");
    assert_eq!(text("trace([[a,b],[c,d]])"), "a+d");
    assert_eq!(
        text("matmul([[a,b],[c,d]],[[e,f],[g,h]])"),
        "[[a*e+b*g,a*f+b*h],[c*e+d*g,c*f+d*h]]"
    );
}

#[test]
fn symbolic_inverses() {
    assert_eq!(text("inverse([[a]])"), "[[a^(-1)]]");
    assert_eq!(
        text("inverse([[a,b],[c,d]])"),
        "[[d*(-b*c+a*d)^(-1),-(b*(-b*c+a*d)^(-1))],[-(c*(-b*c+a*d)^(-1)),a*(-b*c+a*d)^(-1)]]"
    );
    assert!(text("inverse([[1,2],[3,x]])").contains("(-6+x)^(-1)"));
    assert!(text("inverse([[1,2,3],[4,5,6],[7,8,x]])").contains("^(-1)"));
}

#[test]
fn plu_decompositions() {
    assert_eq!(
        text("plu([[2,1],[4,3]])"),
        "[[[1,0],[0,1]],[[1,0],[2,1]],[[2,1],[0,1]]]"
    );
    assert_eq!(
        text("plu([[0,1],[1,0]])"),
        "[[[0,1],[1,0]],[[1,0],[0,1]],[[1,0],[0,1]]]"
    );
    assert!(text("plu([[1,2],[2,4]])").contains("plu"));
}

#[test]
fn characteristic_polynomials() {
    assert_eq!(text("charpoly([[1,2],[3,4]])"), "[-2,-5,1]");
    assert_eq!(text("charpoly([[1,0],[0,2]])"), "[2,-3,1]");
    assert_eq!(text("charpoly([[1,0,0],[0,2,0],[0,0,3]])"), "[-6,11,-6,1]");
}

#[test]
fn eigenvalues_and_nullspaces() {
    assert_eq!(text("eigenvalues([[2,0],[0,3]])"), "[2,3]");
    assert_eq!(
        text("eigenvalues([[1,2],[3,4]])"),
        "[(1/2)*(5-sqrt(33)),(1/2)*(5+sqrt(33))]"
    );
    assert_eq!(text("eigenvalues([[0,-1],[1,0]])"), "[-i,i]");
    assert_eq!(text("eigenvalues([[1,2],[2,4]])"), "[0,5]");
    assert_eq!(text("eigenvalues([[1,0,0],[0,2,0],[0,0,3]])"), "[1,2,3]");
    assert!(text("eigenvalues([[a,b],[c,d]])").contains("sqrt"));
    assert_eq!(text("nullspace([[1,2],[2,4]])"), "[[-2,1]]");
    assert_eq!(text("nullspace([[1,0],[0,1]])"), "[]");
    assert_eq!(text("nullspace([[1,2,3],[2,4,6]])"), "[[-2,1,0],[-3,0,1]]");
}

#[test]
fn eigenvectors_and_kernel_alias() {
    assert_eq!(text("eigenvectors([[2,0],[0,3]])"), "[[1,0],[0,1]]");
    assert_eq!(text("eigenvectors([[1,1],[0,2]])"), "[[1,0],[1,1]]");
    assert_eq!(text("eigenvectors([[1,2],[2,4]])"), "[[2,-1],[2,4]]");
    assert_eq!(
        text("eigenvectors([[1,0,0],[0,2,0],[0,0,3]])"),
        "[[1,0,0],[0,1,0],[0,0,1]]"
    );
    assert_eq!(text("kernel([[1,2],[2,4]])"), "[[-2,1]]");
    assert_eq!(text("kernel([[1,0],[0,1]])"), "[]");
}

#[test]
fn qr_decompositions() {
    assert_eq!(text("qr([[1,0],[0,1]])"), "[[[1,0],[0,1]],[[1,0],[0,1]]]");
    assert_eq!(text("qr([[1,1],[0,1]])"), "[[[1,0],[0,1]],[[1,1],[0,1]]]");
    let generic = text("qr([[1,2],[3,4]])");
    assert!(generic.starts_with("[[["), "{generic}");
    assert!(generic.contains("sqrt(10)"), "{generic}");
    assert!(text("qr([[1,2],[3,4],[5,6]])").starts_with("[[["));
    assert!(text("qr([[1,2],[2,4]])").contains("qr"));
}
