use mithqal::Session;

fn eval(modules: &[&str], source: &str) -> String {
    let mut session = Session::with_modules(modules);
    let value = session.eval_source(source).unwrap();
    session.format_infix(value).unwrap()
}

#[test]
fn number_theory_policy_is_library_code() {
    assert_eq!(eval(&["numbertheory"], "totient(12)"), "4");
    assert_eq!(eval(&["numbertheory"], "divisors(12)"), "[1,2,3,4,6,12]");
    assert_eq!(eval(&["numbertheory"], "fibonacci(12)"), "144");
    assert_eq!(eval(&["numbertheory"], "nextPrime(12)"), "13");
    assert_eq!(eval(&["numbertheory"], "isInteger(3)"), "true");
    assert_eq!(eval(&["numbertheory"], "freeOf(x+1,y)"), "true");
}

#[test]
fn algebra_and_calculus_policy_is_library_code() {
    assert_eq!(eval(&["algebra"], "factor(x^2-1)"), "(-1+x)*(1+x)");
    assert_eq!(eval(&["calculus"], "differentiate(x^2,x)"), "2*x");
    assert_eq!(eval(&["calculus"], "integrate(x^2,0,5)"), "125/3");
    assert_eq!(eval(&["calculus"], "limit(sin(x)/x,x,0)"), "1");
    assert_eq!(eval(&["calculus"], "derive(x^3,x)"), "3*x^2");
}

#[test]
fn linear_algebra_policy_is_library_code() {
    assert_eq!(
        eval(&["linearalgebra"], "matmul([[1,2],[3,4]],[[5,6],[7,8]])"),
        "[[19,22],[43,50]]"
    );
    assert_eq!(eval(&["linearalgebra"], "det([[1,2],[3,4]])"), "-2");
    assert_eq!(eval(&["linearalgebra"], "kernelDet([[1,2],[3,4]])"), "-2");
    assert_eq!(eval(&["linearalgebra"], "trace(identity(3))"), "3");
    assert_eq!(
        eval(&["linearalgebra"], "inverse([[1,2],[3,4]])"),
        "[[-2,1],[3/2,-1/2]]"
    );
}

#[test]
fn algebraic_and_analytic_number_theory_policy_is_library_code() {
    assert_eq!(
        eval(&["algebraicnumbertheory"], "quadraticClassNumber(-23)"),
        "3"
    );
    assert_eq!(
        eval(&["algebraicnumbertheory"], "quadraticFundamentalUnit(3)"),
        "[2,1]"
    );
    assert_eq!(eval(&["analyticnumbertheory"], "summatoryMobius(10)"), "-1");
}

#[test]
fn prose_modules_lower_and_evaluate() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source("module prose.\nwe define f(x) = x^2 + 1.\nend module.")
        .unwrap();
    let value = session.eval_source("f(2)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "5");

    session
        .eval_source(
            "module guarded.\nwe define square(n) = n * n, where n is an integer.\nend module.",
        )
        .unwrap();
    let value = session.eval_source("square(4)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "16");
    let value = session.eval_source("square(1/2)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "square(1/2)");

    session
        .eval_source("module patterns.\nwe define patterned(sin(u)) = cos(u).\nend module.")
        .unwrap();
    let value = session.eval_source("patterned(sin(y))").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "cos(y)");

    session
        .eval_source(
            "module minicalculus.\nwe define d(x)/dx = 1.\nwe define d(sin(u))/dx = cos(u) * d(u)/dx.\nend module.",
        )
        .unwrap();
    let value = session.eval_source("derivative(sin(x),x)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "cos(x)");
}

#[test]
fn prose_theorem_skeletons_load() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source(
            "module theorem_module.\ntheorem identity.\nfor every expression u in x, d(u)/dx = d(u)/dx.\nproof:\nby the definition of d.\nend proof.\nend module.",
        )
        .unwrap();
    let value = session.eval_source("1+1").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "2");
}

#[test]
fn prose_ground_theorems_are_checked() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source(
            "module checked.\ntheorem two.\n1 + 1 = 2.\nproof:\nby arithmetic.\nend proof.\nend module.",
        )
        .unwrap();
    let mut failing = Session::with_modules(&[]);
    assert!(
        failing
            .eval_source(
                "module unchecked.\ntheorem wrong.\n1 + 1 = 3.\nproof:\nby arithmetic.\nend proof.\nend module.",
            )
            .is_err()
    );

    let mut universal = Session::with_modules(&[]);
    universal
        .eval_source(
            "module universal.\ntheorem identity.\nfor every expression u in x, d(u)/dx = d(u)/dx.\nproof:\nby the definition of d.\nend proof.\nend module.",
        )
        .unwrap();

    let mut universal_failing = Session::with_modules(&[]);
    assert!(
        universal_failing
            .eval_source(
                "module universal_wrong.\ntheorem wrong_identity.\nfor every expression u in x, d(u)/dx = 2 * d(u)/dx.\nproof:\nby definition.\nend proof.\nend module.",
            )
            .is_err()
    );

    let mut steps = Session::with_modules(&[]);
    steps
        .eval_source(
            "module proof_steps.\ntheorem four.\n2 + 2 = 4.\nproof:\nhave sum: 2 + 2 = 4.\nshow 4 = 4.\nby arithmetic.\nend proof.\nend module.",
        )
        .unwrap();

    let mut existential = Session::with_modules(&[]);
    existential
        .eval_source(
            "module existential.\ntheorem has_two.\nthere exists n in [1, 2, 3] such that n == 2.\nproof:\nby arithmetic.\nend proof.\nend module.",
        )
        .unwrap();

    let mut existential_failing = Session::with_modules(&[]);
    assert!(
        existential_failing
            .eval_source(
                "module existential_wrong.\ntheorem has_four.\nthere exists n in [1, 2, 3] such that n == 4.\nproof:\nby arithmetic.\nend proof.\nend module.",
            )
            .is_err()
    );

    let mut proof_assumption = Session::with_modules(&[]);
    proof_assumption
        .eval_source(
            "module proof_assumption.\ntheorem positive_shift.\nabs(x) = x.\nproof:\nassume x > 0.\nby arithmetic.\nend proof.\nend module.",
        )
        .unwrap();

    let mut have_fact = Session::with_modules(&[]);
    have_fact
        .eval_source(
            "module have_fact.\ntheorem substitution.\ny - x = 1.\nproof:\nassume y = x + 1.\nshow y - x = 1.\nby arithmetic.\nend proof.\nend module.",
        )
        .unwrap();

    let mut universal_finite = Session::with_modules(&[]);
    universal_finite
        .eval_source(
            "module universal_finite.\ntheorem positive_list.\nfor every n in [1, 2, 3], n > 0.\nproof:\nby arithmetic.\nend proof.\nend module.",
        )
        .unwrap();

    let mut universal_finite_failing = Session::with_modules(&[]);
    assert!(
        universal_finite_failing
            .eval_source(
                "module universal_finite_wrong.\ntheorem nonnegative_list.\nfor every n in [1, 2, 3], n > 2.\nproof:\nby arithmetic.\nend proof.\nend module.",
            )
            .is_err()
    );

    let mut quantified_step = Session::with_modules(&[]);
    quantified_step
        .eval_source(
            "module quantified_step.\ntheorem positive_list_step.\nfor every n in [1, 2, 3], n > 0.\nproof:\nshow for every n in [1, 2, 3], n > 0.\nby arithmetic.\nend proof.\nend module.",
        )
        .unwrap();

    let mut cases_proof = Session::with_modules(&[]);
    cases_proof
        .eval_source(
            "module cases_proof.\ntheorem square_nonnegative.\nn * n >= 0.\nproof:\nby cases on n in [-1, 0, 1].\nby arithmetic.\nend proof.\nend module.",
        )
        .unwrap();

    let mut contradiction_proof = Session::with_modules(&[]);
    contradiction_proof
        .eval_source(
            "module contradiction_proof.\ntheorem ex_falso.\n2 + 2 = 5.\nproof:\nassume x > 0.\nassume x < 0.\ncontradiction.\nend proof.\nend module.",
        )
        .unwrap();

    let mut named_fact = Session::with_modules(&[]);
    named_fact
        .eval_source(
            "module named_fact.\ntheorem named_sum.\n4 = 4.\nproof:\nhave sum: 2 + 2 = 4.\nshow 4 = 4.\nby sum.\nend proof.\nend module.",
        )
        .unwrap();

    let mut unsupported = Session::with_modules(&[]);
    assert!(
        unsupported
            .eval_source(
                "module unsupported.\ntheorem two.\n1 + 1 = 2.\nproof:\nby magic.\nend proof.\nend module.",
            )
            .is_err()
    );
}

#[test]
fn prose_assumptions_reach_the_evaluator() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source("module assumptions.\nassume x is positive.\nend module.")
        .unwrap();
    let value = session.eval_source("abs(x)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "x");
}

#[test]
fn prose_structures_and_instances_load() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source(
            "module structures_prose.\nstructure testMonoid.\nfield carrier.\nfield op.\nfield identity.\nlaw associative: forall(a, carrier, forall(b, carrier, forall(c, carrier, same(op(op(a, b), c), op(a, op(b, c)))))).\nend structure.\ninstance cyclic : testMonoid.\ncarrier = [0, 1, 2].\nop = fn(a, b) => mod(a + b, 3).\nidentity = 0.\nend instance.\nend module.",
        )
        .unwrap();
    let value = session.eval_source("testMonoidHolds(cyclic)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "true");
}

#[test]
fn prose_local_definitions_evaluate() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source(
            "module localdefs.\nwe define fact(n) = let loop(0, acc) = acc, loop(k, acc) = loop(k - 1, acc * k) in loop(n, 1).\nend module.",
        )
        .unwrap();
    let value = session.eval_source("fact(5)").unwrap();
    assert_eq!(session.format_infix(value).unwrap(), "120");
}

#[test]
fn prose_print_and_module_bindings() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source("module output.\nlet a be 2.\nprint a + 1.\nend module.")
        .unwrap();
    assert_eq!(session.take_output(), vec!["3".to_string()]);
}

#[test]
fn prose_if_blocks_evaluate() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source(
            "module conditionals.\nlet x be 5.\nif x > 3 then:\nprint x.\nelse:\nprint 0.\nend if.\nend module.",
        )
        .unwrap();
    assert_eq!(session.take_output(), vec!["5".to_string()]);

    let mut session = Session::with_modules(&[]);
    session
        .eval_source(
            "module conditionals.\nlet x be 1.\nif x > 3 then:\nprint x.\nelse:\nprint 0.\nend if.\nend module.",
        )
        .unwrap();
    assert_eq!(session.take_output(), vec!["0".to_string()]);
}

#[test]
fn prose_match_blocks_evaluate() {
    let mut session = Session::with_modules(&[]);
    session
        .eval_source(
            "module matching.\nlet x be 2.\nmatch x with:\ncase 0:\nprint 100.\ncase 1:\nprint 200.\ncase n:\nprint n + 10.\nelse:\nprint -1.\nend match.\nend module.",
        )
        .unwrap();
    assert_eq!(session.take_output(), vec!["12".to_string()]);

    let mut session = Session::with_modules(&[]);
    session
        .eval_source(
            "module matching.\nlet x be 9.\nmatch x with:\ncase 0:\nprint 0.\nelse:\nprint 99.\nend match.\nend module.",
        )
        .unwrap();
    assert_eq!(session.take_output(), vec!["99".to_string()]);
}

#[test]
fn kernel_accelerators_remain_available() {
    assert_eq!(
        eval(&["numbertheory"], "factorInteger(12)"),
        "[[2,2],[3,1]]"
    );
    assert_eq!(eval(&["linearalgebra"], "det([[1,2],[3,4]])"), "-2");
    assert_eq!(eval(&["linearalgebra"], "kernelDet([[1,2],[3,4]])"), "-2");
    assert_eq!(eval(&["algebra"], "kernelExpand((x+1)^2)"), "1+x^2+2*x");
}
