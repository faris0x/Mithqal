use std::hint::black_box;
use std::time::Instant;

use cas::Session;

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
    let mut session = Session::new();
    measure("parse and evaluate 1+2", 20_000, || {
        black_box(session.eval_statement("1+2").unwrap());
    });

    let mut session = Session::new();
    measure("parse and evaluate x+x", 20_000, || {
        black_box(session.eval_statement("x+x").unwrap());
    });

    let mut session = Session::new();
    measure("parse and evaluate 2/3+1/6", 20_000, || {
        black_box(session.eval_statement("2/3+1/6").unwrap());
    });

    let mut session = Session::new();
    session.eval_statement("f(x_):=x^2").unwrap();
    measure("pattern call f(3)", 20_000, || {
        black_box(session.eval_statement("f(3)").unwrap());
    });

    let mut session = Session::new();
    let mut source = String::from("x");
    for _ in 0..50 {
        source.push_str("+x");
    }
    measure("evaluate 50 like terms", 2_000, || {
        black_box(session.eval_statement(&source).unwrap());
    });

    let mut session = Session::new();
    session.eval_statement("f=fn(x)=>x+1").unwrap();
    measure("closure call f(3)", 20_000, || {
        black_box(session.eval_statement("f(3)").unwrap());
    });

    let mut session = Session::new();
    measure("compiled 1+2", 20_000, || {
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
