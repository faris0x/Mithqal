use mithqal::Session;

fn next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[test]
fn random_source_strings_do_not_panic() {
    let alphabet: Vec<char> = "abcxyz0123+-*/^()[],;<>|=!&~_\"'.:\\ ".chars().collect();
    let mut state = 0x5eed_u64;
    let mut session = Session::new();
    for case in 0..2000 {
        let length = (next(&mut state) % 48) as usize;
        let mut source = String::with_capacity(length);
        for _ in 0..length {
            let index = (next(&mut state) as usize) % alphabet.len();
            source.push(alphabet[index]);
        }
        let _ = session.eval_source(&source);
        if case % 500 == 0 {
            let _ = session.take_messages();
        }
    }
}

#[test]
fn random_pattern_sources_do_not_panic() {
    let sources = [
        "f(x_Integer)",
        "f(alternatives(1,2))",
        "f(optional(x_,0))",
        "f(repeated(x__))",
        "f(repeatedNull(x___))",
        "f(longest(x__))",
        "f(shortest(x__))",
        "f(except(1))",
        "f(verbatim(1))",
        "f(patternTest(x_Integer,isEven))",
    ];
    let mut state = 0x1234_u64;
    let mut session = Session::new();
    for _ in 0..200 {
        let pattern = sources[(next(&mut state) as usize) % sources.len()];
        let source = format!("f(x_):=x\n{pattern}");
        let _ = session.eval_source(&source);
    }
}
