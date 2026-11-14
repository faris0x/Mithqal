use std::hint::black_box;
use std::time::Instant;

use cas::Session;
use cas::eval::evaluate;
use cas::expr::Arena;
use cas::syntax::parse_statement;

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
    measure("cold approx pi 30", 5_000, || {
        let parsed = parse_statement(&mut cold_arena, "approx(pi,30)").unwrap();
        black_box(evaluate(&mut cold_arena, parsed).unwrap());
    });

    let mut cold_arena = Arena::new();
    measure("cold interval add", 20_000, || {
        let parsed = parse_statement(&mut cold_arena, "interval(1,2)+3").unwrap();
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

    let mut session = Session::new();
    measure("parse and collect 100 expressions", 100, || {
        for value in 0..100 {
            black_box(session.eval_statement(&format!("{}+1", value)).unwrap());
        }
        session.collect_garbage(&[]);
    });
}
