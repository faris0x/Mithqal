use mithqal::{ErrorCode, Session};

fn text(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value)
}

#[test]
fn means_and_medians() {
    assert_eq!(text("mean([1,2,3,4])"), "5/2");
    assert_eq!(text("mean([2,4,6])"), "4");
    assert_eq!(text("mean([1/2,1/3])"), "5/12");
    assert_eq!(text("median([1,2,3])"), "2");
    assert_eq!(text("median([1,2,3,4])"), "5/2");
    assert_eq!(text("median([4,1,3,2])"), "5/2");
    assert_eq!(text("median([7])"), "7");
}

#[test]
fn extrema_and_ranges() {
    assert_eq!(text("minimum([3,1,2])"), "1");
    assert_eq!(text("maximum([3,1,2])"), "3");
    assert_eq!(text("minimum([-5,2,-1])"), "-5");
    assert_eq!(text("maximum([-5,2,-1])"), "2");
    assert_eq!(text("range([3,1,2])"), "2");
    assert_eq!(text("range([-5,2,-1])"), "7");
}

#[test]
fn totals_and_counts() {
    assert_eq!(text("total([1,2,3,4])"), "10");
    assert_eq!(text("total([1/2,1/3])"), "5/6");
    assert_eq!(text("count([1,2,3,4])"), "4");
    assert_eq!(text("count([7])"), "1");
}

#[test]
fn variances_and_deviations() {
    assert_eq!(text("variance([1,2,3,4])"), "5/4");
    assert_eq!(text("sampleVariance([1,2,3,4])"), "5/3");
    assert_eq!(text("variance([2,2,2])"), "0");
    assert_eq!(text("standardDeviation([1,3])"), "1");
    assert_eq!(text("standardDeviation([1,2,3,4])"), "sqrt(5/4)");
    assert_eq!(text("sampleStandardDeviation([1,3])"), "sqrt(2)");
}

#[test]
fn frequencies_and_modes() {
    assert_eq!(text("frequency([1,2,2,3,3,3])"), "[[1,1],[2,2],[3,3]]");
    assert_eq!(text("frequency([4,4,4])"), "[[4,3]]");
    assert_eq!(text("mode([1,2,2,3,3])"), "[2,3]");
    assert_eq!(text("mode([1,1,2,3,3,3])"), "[3]");
    assert_eq!(text("mode([7])"), "[7]");
}

#[test]
fn invalid_statistics_inputs_stay_unevaluated() {
    for source in [
        "mean([])",
        "median([])",
        "sampleVariance([1])",
        "mean([x,1])",
        "median(x)",
        "frequency([1,x])",
    ] {
        assert!(
            text(source).contains(source.split('(').next().unwrap()),
            "{source}"
        );
    }
}

#[test]
fn statistics_arity_errors() {
    for source in [
        "mean()",
        "mean([1],[2])",
        "median()",
        "variance()",
        "mode()",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(source).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalArity, "{source}");
    }
}

#[test]
fn statistics_symbols_are_protected() {
    for name in [
        "mean",
        "median",
        "minimum",
        "maximum",
        "range",
        "total",
        "count",
        "variance",
        "sampleVariance",
        "standardDeviation",
        "sampleStandardDeviation",
        "frequency",
        "mode",
    ] {
        let mut session = Session::new();
        let error = session.eval_statement(&format!("{name}=1")).unwrap_err();
        assert_eq!(error.code, ErrorCode::EvalProtected, "{name}");
    }
}

#[test]
fn statistics_from_the_language() {
    let source = r#"
mod stats
let data=[1,2,2,3,4]
prt mean(data)
prt median(data)
prt mode(data)
prt variance(data)
end
"#;
    let mut session = Session::new();
    session.eval_source(source).unwrap();
    assert_eq!(session.output(), &["12/5", "2", "[2]", "26/25"]);
}
