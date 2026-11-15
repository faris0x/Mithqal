use mithqal::Session;

fn text(source: &str) -> String {
    let mut session = Session::new();
    let value = session.eval_statement(source).unwrap();
    session.format_infix(value).unwrap()
}

macro_rules! eval_case {
    ($name:ident, $source:expr, $expected:expr) => {
        #[test]
        fn $name() {
            assert_eq!(text($source), $expected, $source);
        }
    };
}

eval_case!(case_arith_1, "1+2", "3");
eval_case!(case_arith_2, "10-4", "6");
eval_case!(case_arith_3, "6*7", "42");
eval_case!(case_arith_4, "10/4", "5/2");
eval_case!(case_arith_5, "2^10", "1024");
eval_case!(case_arith_6, "-5+8", "3");
eval_case!(case_arith_7, "0*999", "0");
eval_case!(case_arith_8, "1/2+1/3", "5/6");
eval_case!(case_arith_9, "100/25", "4");
eval_case!(case_arith_10, "7-10", "-3");

eval_case!(case_floor_1, "floor(7/2)", "3");
eval_case!(case_floor_2, "floor(-7/2)", "-4");
eval_case!(case_ceil_1, "ceil(7/2)", "4");
eval_case!(case_ceil_2, "ceil(-7/2)", "-3");
eval_case!(case_round_1, "round(7/2)", "4");
eval_case!(case_round_2, "round(-7/2)", "-4");
eval_case!(case_gcd_1, "gcd(84,30)", "6");
eval_case!(case_gcd_2, "gcd(0,0)", "0");
eval_case!(case_lcm_1, "lcm(12,18)", "36");
eval_case!(case_factorial_1, "factorial(6)", "720");
eval_case!(case_binomial_1, "binomial(10,3)", "120");
eval_case!(case_mod_1, "mod(-7,3)", "2");
eval_case!(case_mod_2, "mod(7,-3)", "1");
eval_case!(case_powermod_1, "powerMod(2,10,1000)", "24");

eval_case!(case_prime_1, "isPrime(97)", "true");
eval_case!(case_prime_2, "isPrime(100)", "false");
eval_case!(case_nextprime_1, "nextPrime(100)", "101");
eval_case!(case_prevprime_1, "previousPrime(100)", "97");
eval_case!(
    case_factorint_1,
    "factorInteger(360)",
    "[[2,3],[3,2],[5,1]]"
);
eval_case!(case_divisors_1, "divisors(36)", "[1,2,3,4,6,9,12,18,36]");
eval_case!(case_divcount_1, "divisorCount(36)", "9");
eval_case!(case_totient_1, "totient(36)", "12");
eval_case!(case_sigma_1, "sigma(12)", "28");
eval_case!(case_moebius_1, "moebius(30)", "-1");
eval_case!(case_radical_1, "radical(360)", "30");
eval_case!(case_primecount_1, "primeCount(100)", "25");
eval_case!(case_fibonacci_1, "fibonacci(20)", "6765");
eval_case!(case_modinverse_1, "modInverse(3,11)", "4");
eval_case!(case_jacobi_1, "jacobi(2,15)", "1");

eval_case!(case_expand_1, "expand((x+1)^2)", "1+x^2+2*x");
eval_case!(case_expand_2, "expand((x+y)(x-y))", "-y^2+x^2");
eval_case!(case_factor_1, "factor(x^2-1)", "(-1+x)*(1+x)");
eval_case!(case_factor_2, "factor(x^3+y^3)", "(x+y)*(-x*y+x^2+y^2)");
eval_case!(case_degree_1, "degree(x^3+2*x+1,x)", "3");
eval_case!(case_coeff_1, "coefficient(x^3+2*x+1,x,1)", "2");
eval_case!(
    case_divide_1,
    "polynomialDivide(x^3+1,x-1,x)",
    "[1+x+x^2,2]"
);
eval_case!(case_disc_1, "discriminant(x^2-4,x)", "16");
eval_case!(case_gcdpoly_1, "polynomialGcd(x^2-1,x^2+2*x+1,x)", "1+x");
eval_case!(case_resultant_1, "resultant(x^2-1,x-2,x)", "3");
eval_case!(case_solve_1, "solve(x^2-4,x)", "[-2,2]");
eval_case!(case_solve_2, "solve(x^2+1,x)", "[-i,i]");
eval_case!(case_cancel_1, "cancel((x^2-1)/(x+1))", "-1+x");
eval_case!(case_together_1, "together(1/x+1/y)", "(x+y)*(x*y)^(-1)");

eval_case!(case_diff_1, "differentiate(x^3,x)", "3*x^2");
eval_case!(case_diff_2, "differentiate(sin(x),x)", "cos(x)");
eval_case!(case_diff_3, "differentiate(exp(2*x),x)", "2*exp(2*x)");
eval_case!(case_diff_4, "differentiate(ln(x),x)", "x^(-1)");
eval_case!(case_int_1, "integrate(x^2,x)", "(1/3)*x^3");
eval_case!(case_int_2, "integrate(sin(x),x)", "-cos(x)");
eval_case!(case_int_3, "integrate(1/x,x)", "ln(abs(x))");
eval_case!(case_defint_1, "integrate(x^2,x,0,3)", "9");
eval_case!(case_defint_2, "integrate(2*x+1,x,1,2)", "4");

eval_case!(case_det_1, "det([[1,2],[3,4]])", "-2");
eval_case!(case_det_2, "det([[1,2],[2,4]])", "0");
eval_case!(case_rank_1, "rank([[1,2],[2,4]])", "1");
eval_case!(
    case_transpose_1,
    "transpose([[1,2],[3,4]])",
    "[[1,3],[2,4]]"
);
eval_case!(
    case_inverse_1,
    "inverse([[1,2],[3,4]])",
    "[[-2,1],[3/2,-1/2]]"
);
eval_case!(case_rref_1, "rref([[1,2],[3,4]])", "[[1,0],[0,1]]");
eval_case!(case_trace_1, "trace([[1,2],[3,4]])", "5");
eval_case!(
    case_matmul_1,
    "matmul([[1,2],[3,4]],[[5,6],[7,8]])",
    "[[19,22],[43,50]]"
);
eval_case!(case_identity_1, "identity(2)", "[[1,0],[0,1]]");
eval_case!(
    case_solvelin_1,
    "solveLinear([[2,1],[1,3]],[5,7])",
    "[8/5,9/5]"
);
eval_case!(case_dot_1, "dot([1,2,3],[4,5,6])", "32");
eval_case!(case_cross_1, "cross([1,0,0],[0,1,0])", "[0,0,1]");
eval_case!(case_charpoly_1, "charpoly([[1,2],[3,4]])", "[-2,-5,1]");
eval_case!(
    case_eigen_1,
    "eigenvalues([[1,2],[3,4]])",
    "[(1/2)*(5-sqrt(33)),(1/2)*(5+sqrt(33))]"
);
eval_case!(case_eigen_2, "eigenvalues([[0,-1],[1,0]])", "[-i,i]");
eval_case!(case_null_1, "nullspace([[1,2],[2,4]])", "[[-2,1]]");
eval_case!(case_kernel_1, "kernel([[1,2],[2,4]])", "[[-2,1]]");

eval_case!(case_mean_1, "mean([1,2,3,4])", "5/2");
eval_case!(case_median_1, "median([1,2,3])", "2");
eval_case!(case_min_1, "minimum([3,1,2])", "1");
eval_case!(case_max_1, "maximum([3,1,2])", "3");
eval_case!(case_range_1, "range([3,1,2])", "2");
eval_case!(case_total_1, "total([1,2,3,4])", "10");
eval_case!(case_count_1, "count([1,2,3,4])", "4");
eval_case!(case_variance_1, "variance([1,2,3,4])", "5/4");
eval_case!(case_svar_1, "sampleVariance([1,2,3,4])", "5/3");
eval_case!(
    case_freq_1,
    "frequency([1,2,2,3,3,3])",
    "[[1,1],[2,2],[3,3]]"
);
eval_case!(case_mode_1, "mode([1,2,2,3,3])", "[2,3]");

eval_case!(case_logic_1, "xor(true,false)", "true");
eval_case!(case_logic_2, "implies(true,false)", "false");
eval_case!(case_logic_3, "all([true,true])", "true");
eval_case!(case_logic_4, "any([false,true])", "true");
eval_case!(case_logic_5, "and(isPrime(7),isOdd(7))", "true");
eval_case!(case_logic_6, "not(isPrime(4))", "true");
