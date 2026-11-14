use std::hint::black_box;
use std::time::Instant;

use mithqal::Session;
use mithqal::eval::evaluate;
use mithqal::expr::Arena;
use mithqal::syntax::parse_statement;

fn measure(name: &str, iterations: u32, mut action: impl FnMut()) {
    let start = Instant::now();
    for _ in 0..iterations {
        action();
    }
    let elapsed = start.elapsed();
    let per = elapsed.as_secs_f64() / iterations as f64;
    println!("{name}: {:.2} us/op", per * 1_000_000.0);
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
    measure("cold factor x^6-1", 1_000, || {
        let parsed = parse_statement(&mut cold_arena, "factor(x^6-1)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

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
}
