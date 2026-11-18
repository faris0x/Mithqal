use std::hint::black_box;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use mithqal::Session;
use mithqal::eval::evaluate;
use mithqal::expr::Arena;
use mithqal::syntax::parse_statement;

static OVER_BUDGET: AtomicBool = AtomicBool::new(false);

fn measure(name: &str, iterations: u32, mut action: impl FnMut()) -> f64 {
    let start = Instant::now();
    for _ in 0..iterations {
        action();
    }
    let elapsed = start.elapsed();
    let per = elapsed.as_secs_f64() / iterations as f64;
    println!("{name}: {:.2} us/op", per * 1_000_000.0);
    per * 1_000_000.0
}

fn nested_call_source(depth: usize) -> String {
    let mut source = String::from("1");
    for _ in 0..depth {
        source = format!("f({source})");
    }
    source
}

fn check_budget(name: &str, per_microseconds: f64, budget_microseconds: f64) {
    if per_microseconds > budget_microseconds {
        eprintln!(
            "budget exceeded for {name}: {per_microseconds:.2} us/op > {budget_microseconds:.2} us/op"
        );
        OVER_BUDGET.store(true, Ordering::Relaxed);
    }
}

fn main() {
    let mut cold_arena = Arena::new();
    measure("cold parse and evaluate 1+2", 20_000, || {
        let parsed = parse_statement(&mut cold_arena, "1+2").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("parse only 1+2", 50_000, || {
        black_box(parse_statement(&mut cold_arena, "1+2").unwrap());
    });

    let mut cold_arena = Arena::new();
    let parsed = parse_statement(&mut cold_arena, "1+2").unwrap();
    measure("evaluate parsed 1+2", 50_000, || {
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let deep_source = nested_call_source(512);
    let mut cold_arena = Arena::new();
    measure("cold parse and evaluate deep call 512", 200, || {
        let parsed = parse_statement(&mut cold_arena, &deep_source).unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let deep_source = nested_call_source(1024);
    let mut cold_arena = Arena::new();
    measure("cold parse deep call 1024", 200, || {
        black_box(parse_statement(&mut cold_arena, &deep_source).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold approx pi 30", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(pi,30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold approx complex exp", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(exp(i*pi),30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold approx real sinh", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(sinh(2),30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold approx gamma", 3_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(gamma(5),30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold approx erf", 3_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(erf(1),30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold approx complex asin", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(asin(2),30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold approx erfi", 3_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(erfi(1),30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold approx complex erf", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(erf(1+i),30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold interval add", 20_000, || {
        let parsed = parse_statement(&mut cold_arena, "interval(1,2)+3").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold interval multiply", 20_000, || {
        let parsed = parse_statement(&mut cold_arena, "interval(1,2)*interval(3,4)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold expand (x+y)^8", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "expand((x+y)^8)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    let per = measure("cold factor x^6-1", 1_000, || {
        let parsed = parse_statement(&mut cold_arena, "factor(x^6-1)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });
    check_budget("cold factor x^6-1", per, 250.0);

    let mut cold_arena = Arena::new();
    let per = measure("cold factor bivariate hensel", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "factor((x^2+y)*(x^2+y^2))").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });
    check_budget("cold factor bivariate hensel", per, 250.0);

    let mut cold_arena = Arena::new();
    let per = measure("cold factor wide hensel", 1_000, || {
        let parsed = parse_statement(
            &mut cold_arena,
            "factor(x^4+30000000000000000000000000*x^2+200000000000000000000000000000000000000000000000000)",
        )
        .unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });
    check_budget("cold factor wide hensel", per, 1_000.0);

    let mut cold_arena = Arena::new();
    let per = measure("cold factor four variable", 1_000, || {
        let parsed = parse_statement(&mut cold_arena, "factor((x+a+b+c)*(x+a^2+b^2+c^2))").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });
    check_budget("cold factor four variable", per, 250.0);

    let mut cold_arena = Arena::new();
    let per = measure("cold factor five variable", 500, || {
        let parsed =
            parse_statement(&mut cold_arena, "factor((x+a+b+c+d)*(x+a^2+b^2+c^2+d^2))").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });
    check_budget("cold factor five variable", per, 350.0);

    let mut cold_arena = Arena::new();
    let per = measure("cold factor six variable", 200, || {
        let parsed = parse_statement(
            &mut cold_arena,
            "factor((x+a+b+c+d+e)*(x+a^2+b^2+c^2+d^2+e^2))",
        )
        .unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });
    check_budget("cold factor six variable", per, 500.0);

    let mut cold_arena = Arena::new();
    let per = measure("cold factor degree ten hensel", 500, || {
        let parsed = parse_statement(&mut cold_arena, "factor((x^5+y)*(x^5+y*x+1))").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });
    check_budget("cold factor degree ten hensel", per, 2_000.0);

    let mut cold_arena = Arena::new();
    measure("cold solve quadratic", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "solve(x^2-3*x+2==0,x)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold differentiate x^6", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "differentiate(x^6,x)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold integrate polynomial", 1_000, || {
        let parsed = parse_statement(&mut cold_arena, "integrate(x^3-2*x+1,x)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold definite integral", 1_000, || {
        let parsed = parse_statement(&mut cold_arena, "integrate(x^3-2*x+1,x,0,1)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold gcd 84 30", 10_000, || {
        let parsed = parse_statement(&mut cold_arena, "gcd(84,30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold factorial 10", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "factorial(10)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold determinant 3x3", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "det([[1,2,3],[4,5,6],[7,8,10]])").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold inverse 2x2", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "inverse([[1,2],[3,4]])").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold isPrime 1000000007", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "isPrime(1000000007)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold factorInteger 360", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "factorInteger(360)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold mean list", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "mean([1,2,3,4])").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold variance list", 3_000, || {
        let parsed = parse_statement(&mut cold_arena, "variance([1,2,3,4])").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let values = (0..64)
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let mean_source = format!("mean([{values}])");
    let variance_source = format!("variance([{values}])");
    let mut cold_arena = Arena::new();
    measure("cold mean list 64", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, &mean_source).unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold variance list 64", 1_000, || {
        let parsed = parse_statement(&mut cold_arena, &variance_source).unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold primeCount 1000", 2_000, || {
        let parsed = parse_statement(&mut cold_arena, "primeCount(1000)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold fibonacci 20", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "fibonacci(20)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut session = Session::new();
    measure("cached parse and evaluate 1+2", 20_000, || {
        black_box(session.eval_statement("1+2").unwrap());
    });

    let mut session = Session::new();
    measure("cached parse and evaluate x+x", 20_000, || {
        black_box(session.eval_statement("x+x").unwrap());
    });

    let mut session = Session::new();
    measure("cached parse and evaluate 2/3+1/6", 20_000, || {
        black_box(session.eval_statement("2/3+1/6").unwrap());
    });

    let mut session = Session::new();
    session.eval_statement("f(x_):=x^2").unwrap();
    measure("cached pattern call f(3)", 20_000, || {
        black_box(session.eval_statement("f(3)").unwrap());
    });

    let mut session = Session::new();
    let mut source = String::from("x");
    for _ in 0..50 {
        source.push_str("+x");
    }
    measure("cached evaluate 50 like terms", 2_000, || {
        black_box(session.eval_statement(&source).unwrap());
    });

    let mut session = Session::new();
    session.eval_statement("f=fn(x)=>x+1").unwrap();
    measure("cached closure call f(3)", 20_000, || {
        black_box(session.eval_statement("f(3)").unwrap());
    });

    let mut session = Session::new();
    measure("cached approx pi 30", 5_000, || {
        black_box(session.eval_statement("approx(pi,30)").unwrap());
    });

    let mut session = Session::new();
    measure("cached compiled 1+2", 20_000, || {
        black_box(session.eval_compiled_statement("1+2").unwrap());
    });

    let program = "mod bench\ndef square(x)=x*x\nlet a=3\nprt square(a+1)\nend\n";
    let mut session = Session::new();
    measure("language module eval", 2_000, || {
        black_box(session.eval_source(program).unwrap());
    });

    let mut session = Session::new();
    measure("parse and collect 100 expressions", 100, || {
        for value in 0..100 {
            black_box(session.eval_statement(&format!("{}+1", value)).unwrap());
        }
        session.collect_garbage(&[]);
    });

    if OVER_BUDGET.load(Ordering::Relaxed) {
        std::process::exit(1);
    }
}
