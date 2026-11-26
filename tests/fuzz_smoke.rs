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
fn random_prose_sources_do_not_panic() {
    let alphabet: Vec<char> = "abcxyz0123+-*/^()[],;<>|=!&~_\"'.:\\ ×÷−≤≥≠∧∨¬π∞é"
        .chars()
        .collect();
    let fragments = [
        "module m.",
        "end module.",
        "we define f(x) = x.",
        "where x > 0",
        "print x.",
        "let x be 1.",
        "if x > 0 then:",
        "else:",
        "end if.",
        "match x with:",
        "case 1:",
        "end match.",
        "assume x is positive.",
        "theorem t.",
        "proof:",
        "end proof.",
    ];
    let mut state = 0x9eed_u64;
    let mut session = Session::with_modules(&[]);
    for case in 0..2000 {
        let mut source = String::new();
        let fragment_count = (next(&mut state) % 6) as usize;
        for _ in 0..fragment_count {
            if next(&mut state).is_multiple_of(2) {
                source.push_str(fragments[(next(&mut state) as usize) % fragments.len()]);
                source.push('\n');
            } else {
                let length = (next(&mut state) % 24) as usize;
                for _ in 0..length {
                    source.push(alphabet[(next(&mut state) as usize) % alphabet.len()]);
                }
                source.push('\n');
            }
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
        let source = format!("module m.\nwe define f(x) = x.\n{pattern}.\nend module.");
        let _ = session.eval_source(&source);
    }
}
