//! User-facing built-in function catalog.

/// Strongest support guarantee for a catalog function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Support {
    /// Exact evaluation for the documented domain.
    Exact,
    /// Symbolic transformation for the documented domain.
    Symbolic,
    /// Numeric evaluation through `approx`.
    Approximate,
    /// Language or control form.
    Language,
}

impl Support {
    /// Short label used by the CLI and documentation.
    pub const fn label(self) -> &'static str {
        match self {
            Support::Exact => "exact",
            Support::Symbolic => "symbolic",
            Support::Approximate => "approx",
            Support::Language => "language",
        }
    }
}

/// Preferred presentation for a function result.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum DisplayForm {
    /// Standard infix expression.
    Infix,
    /// Prime factorization product.
    PrimeProduct,
    /// Ordered series expansion.
    OrderedSeries,
    /// List result.
    List,
    /// Boolean result.
    Boolean,
    /// Matrix result.
    Matrix,
}

impl DisplayForm {
    /// Short label used by the CLI and documentation.
    pub const fn label(self) -> &'static str {
        match self {
            DisplayForm::Infix => "infix",
            DisplayForm::PrimeProduct => "prime product",
            DisplayForm::OrderedSeries => "ordered series",
            DisplayForm::List => "list",
            DisplayForm::Boolean => "boolean",
            DisplayForm::Matrix => "matrix",
        }
    }
}

/// Capability metadata for one advertised function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    /// Strongest support guarantee.
    pub support: Support,
    /// Exact or symbolic domain covered by the implementation.
    pub domain: &'static str,
    /// Symbolic behavior.
    pub symbolic: &'static str,
    /// Approximate behavior.
    pub approximate: &'static str,
    /// Error and unevaluated-input behavior.
    pub errors: &'static str,
    /// Preferred result presentation.
    pub display: DisplayForm,
}

impl Capability {
    /// Exact support for the given domain.
    pub const fn exact(domain: &'static str) -> Self {
        Capability {
            support: Support::Exact,
            domain,
            symbolic: "Symbolic arguments remain unevaluated.",
            approximate: "Available through approx where documented.",
            errors: "Invalid input returns a diagnostic.",
            display: DisplayForm::Infix,
        }
    }

    /// Symbolic support for the given domain.
    pub const fn symbolic(domain: &'static str) -> Self {
        Capability {
            support: Support::Symbolic,
            domain,
            symbolic: "Transforms supported symbolic expressions.",
            approximate: "Not applicable.",
            errors: "Unsupported forms remain unevaluated.",
            display: DisplayForm::Infix,
        }
    }

    /// Approximate support for the given domain.
    pub const fn approximate(domain: &'static str) -> Self {
        Capability {
            support: Support::Approximate,
            domain,
            symbolic: "Symbolic arguments remain unevaluated.",
            approximate: "Evaluated under approx.",
            errors: "Invalid input returns a diagnostic.",
            display: DisplayForm::Infix,
        }
    }

    /// Language or control form.
    pub const fn language() -> Self {
        Capability {
            support: Support::Language,
            domain: "Surface language and evaluator forms.",
            symbolic: "Language evaluation.",
            approximate: "Not applicable.",
            errors: "Diagnostics follow the language contract.",
            display: DisplayForm::Infix,
        }
    }

    /// Replace the display form.
    pub const fn display(self, display: DisplayForm) -> Self {
        Capability { display, ..self }
    }

    /// Replace the symbolic behavior note.
    pub const fn symbolic_text(self, symbolic: &'static str) -> Self {
        Capability { symbolic, ..self }
    }

    /// Replace the approximate behavior note.
    pub const fn approximate_text(self, approximate: &'static str) -> Self {
        Capability {
            approximate,
            ..self
        }
    }

    /// Replace the error behavior note.
    pub const fn errors(self, errors: &'static str) -> Self {
        Capability { errors, ..self }
    }
}

/// Expected behavior of the documented example.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ExampleKind {
    /// Evaluation should change the expression.
    Evaluates,
    /// Symbolic output may remain unchanged.
    Symbolic,
    /// Approximate evaluation should change the expression.
    Approximate,
    /// Language or definition form.
    Language,
}

impl ExampleKind {
    /// Short label used by documentation and tests.
    pub const fn label(self) -> &'static str {
        match self {
            ExampleKind::Evaluates => "evaluates",
            ExampleKind::Symbolic => "symbolic",
            ExampleKind::Approximate => "approx",
            ExampleKind::Language => "language",
        }
    }
}

/// Metadata for one built-in function.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct FunctionInfo {
    /// Function name.
    pub name: &'static str,
    /// Category name.
    pub category: &'static str,
    /// Source signature.
    pub signature: &'static str,
    /// One line summary.
    pub summary: &'static str,
    /// Documented example.
    pub example: &'static str,
    /// Expected example behavior.
    pub example_kind: ExampleKind,
    /// Capability metadata.
    pub capability: Capability,
}

/// Catalog category order.
pub const CATEGORIES: &[&str] = &[
    "core",
    "algebra",
    "calculus",
    "linear algebra",
    "number theory",
    "statistics",
    "logic",
    "numeric",
    "language",
];

/// Advertised built-in functions with capability metadata.
pub const FUNCTIONS: &[FunctionInfo] = &[
    FunctionInfo {
        name: "gcd",
        category: "core",
        signature: "gcd(a,b,...)",
        summary: "Greatest common divisor.",
        example: "gcd(84,30)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("documented integer and rational inputs"),
    },
    FunctionInfo {
        name: "lcm",
        category: "core",
        signature: "lcm(a,b,...)",
        summary: "Least common multiple.",
        example: "lcm(12,18)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("documented integer and rational inputs"),
    },
    FunctionInfo {
        name: "factorial",
        category: "core",
        signature: "factorial(n)",
        summary: "Exact factorial.",
        example: "factorial(10)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("non-negative integers up to 10000")
            .errors("Out of range input remains unevaluated."),
    },
    FunctionInfo {
        name: "binomial",
        category: "core",
        signature: "binomial(n,k)",
        summary: "Exact binomial coefficient.",
        example: "binomial(20,10)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("documented integer and rational inputs"),
    },
    FunctionInfo {
        name: "floor",
        category: "core",
        signature: "floor(value)",
        summary: "Floor of an exact or approximate value.",
        example: "floor(7/2)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("documented integer and rational inputs"),
    },
    FunctionInfo {
        name: "ceil",
        category: "core",
        signature: "ceil(value)",
        summary: "Ceiling of an exact or approximate value.",
        example: "ceil(7/2)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("documented integer and rational inputs"),
    },
    FunctionInfo {
        name: "round",
        category: "core",
        signature: "round(value)",
        summary: "Nearest integer.",
        example: "round(7/2)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("documented integer and rational inputs"),
    },
    FunctionInfo {
        name: "length",
        category: "core",
        signature: "length(list)",
        summary: "Number of items in a list.",
        example: "length([1,2,3])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("a list argument"),
    },
    FunctionInfo {
        name: "sort",
        category: "core",
        signature: "sort(list)",
        summary: "Order items by numeric then canonical order.",
        example: "sort([3,1,2])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("a list argument"),
    },
    FunctionInfo {
        name: "first",
        category: "core",
        signature: "first(list)",
        summary: "First item of a nonempty list.",
        example: "first([1,2,3])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("a nonempty list argument"),
    },
    FunctionInfo {
        name: "rest",
        category: "core",
        signature: "rest(list)",
        summary: "List without its first item.",
        example: "rest([1,2,3])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("a list argument"),
    },
    FunctionInfo {
        name: "cons",
        category: "core",
        signature: "cons(item,list)",
        summary: "Prepend an item to a list.",
        example: "cons(0,[1,2])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("an item and a list"),
    },
    FunctionInfo {
        name: "nth",
        category: "core",
        signature: "nth(list,index)",
        summary: "Zero based list index.",
        example: "nth([4,5,6],1)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("a list and an in range integer index"),
    },
    FunctionInfo {
        name: "reverse",
        category: "core",
        signature: "reverse(list)",
        summary: "Reverse a list.",
        example: "reverse([1,2,3])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("a list argument"),
    },
    FunctionInfo {
        name: "isNumber",
        category: "core",
        signature: "isNumber(value)",
        summary: "Test whether a value is a number.",
        example: "isNumber(3)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("any value")
            .display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "expand",
        category: "algebra",
        signature: "expand(expr)",
        summary: "Expand products and powers of polynomials.",
        example: "expand((x+y)^3)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "factor",
        category: "algebra",
        signature: "factor(expr)",
        summary: "Factor a polynomial over the rationals.",
        example: "factor(x^6-1)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "together",
        category: "algebra",
        signature: "together(expr)",
        summary: "Combine a rational expression over one denominator.",
        example: "together(1/x+1/(x+1))",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "cancel",
        category: "algebra",
        signature: "cancel(expr)",
        summary: "Cancel common polynomial factors.",
        example: "cancel((x^2-1)/(x-1))",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "polynomialGcd",
        category: "algebra",
        signature: "polynomialGcd(p,q,var)",
        summary: "Polynomial gcd in one variable.",
        example: "polynomialGcd(x^2-1,x^2-2*x+1,x)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "resultant",
        category: "algebra",
        signature: "resultant(p,q,var)",
        summary: "Resultant of two polynomials.",
        example: "resultant(x^2-1,x-1,x)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "solve",
        category: "algebra",
        signature: "solve(expr[,var])",
        summary: "Solve an equation with one inferred variable.",
        example: "solve(x^2-4)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain")
            .display(DisplayForm::List),
    },
    FunctionInfo {
        name: "degree",
        category: "algebra",
        signature: "degree(p,var)",
        summary: "Polynomial degree in one variable.",
        example: "degree(x^3-2*x+1,x)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "coefficient",
        category: "algebra",
        signature: "coefficient(p,var,n)",
        summary: "Coefficient of one power.",
        example: "coefficient(x^3-2*x+1,x,1)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "polynomialDivide",
        category: "algebra",
        signature: "polynomialDivide(p,q,var)",
        summary: "Polynomial quotient and remainder.",
        example: "polynomialDivide(x^2-1,x-1,x)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "discriminant",
        category: "algebra",
        signature: "discriminant(p,var)",
        summary: "Polynomial discriminant.",
        example: "discriminant(x^2-4,x)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "differentiate",
        category: "calculus",
        signature: "differentiate(expr[,var])",
        summary: "Exact derivative with single-variable inference.",
        example: "differentiate(sin(x))",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "integrate",
        category: "calculus",
        signature: "integrate(expr[,var]) | integrate(expr,a,b) | integrate(expr,var,a,b)",
        summary: "Antiderivative or definite integral.",
        example: "integrate(x^2*exp(x))",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "limit",
        category: "calculus",
        signature: "limit(expr[,var],point[,side])",
        summary: "Limit at a finite point or infinity.",
        example: "limit(sin(x)/x,0)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "series",
        category: "calculus",
        signature: "series(expr[,var],point,order[,evaluateFactorials])",
        summary: "Truncated Taylor series.",
        example: "series(exp(x),0,4)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("analytic expressions with supported derivatives")
            .display(DisplayForm::OrderedSeries)
            .errors("Unsupported forms remain unevaluated; an invalid factorial flag returns a diagnostic."),
    },
    FunctionInfo {
        name: "converges",
        category: "calculus",
        signature: "converges(expr[,var],point)",
        summary: "Whether a limit exists.",
        example: "converges(1/x,inf)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::symbolic("supported expressions in the documented domain"),
    },
    FunctionInfo {
        name: "assume",
        category: "calculus",
        signature: "assume(fact,expr)",
        summary: "Evaluate under local sign assumptions.",
        example: "assume(x>0,abs(x))",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "assuming",
        category: "calculus",
        signature: "assuming(fact,expr)",
        summary: "Alias for assume.",
        example: "assuming(x<0,sqrt(x^2))",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "det",
        category: "linear algebra",
        signature: "det(matrix)",
        summary: "Determinant.",
        example: "det([[1,2],[3,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "rank",
        category: "linear algebra",
        signature: "rank(matrix)",
        summary: "Matrix rank.",
        example: "rank([[1,2],[2,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "transpose",
        category: "linear algebra",
        signature: "transpose(matrix)",
        summary: "Matrix transpose.",
        example: "transpose([[1,2],[3,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "inverse",
        category: "linear algebra",
        signature: "inverse(matrix)",
        summary: "Matrix inverse when it exists.",
        example: "inverse([[1,2],[3,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "rref",
        category: "linear algebra",
        signature: "rref(matrix)",
        summary: "Reduced row echelon form.",
        example: "rref([[1,2],[3,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "trace",
        category: "linear algebra",
        signature: "trace(matrix)",
        summary: "Sum of diagonal entries.",
        example: "trace([[1,2],[3,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "matmul",
        category: "linear algebra",
        signature: "matmul(a,b)",
        summary: "Matrix product.",
        example: "matmul([[1,2],[3,4]],[[5,6],[7,8]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "identity",
        category: "linear algebra",
        signature: "identity(n)",
        summary: "Identity matrix.",
        example: "identity(3)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "solveLinear",
        category: "linear algebra",
        signature: "solveLinear(a,b)",
        summary: "Solve a linear system.",
        example: "solveLinear([[2,1],[1,3]],[5,7])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "dot",
        category: "linear algebra",
        signature: "dot(a,b)",
        summary: "Vector dot product.",
        example: "dot([1,2,3],[4,5,6])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "cross",
        category: "linear algebra",
        signature: "cross(a,b)",
        summary: "Three-dimensional cross product.",
        example: "cross([1,0,0],[0,1,0])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices"),
    },
    FunctionInfo {
        name: "plu",
        category: "linear algebra",
        signature: "plu(matrix)",
        summary: "Permutation, lower, and upper factors.",
        example: "plu([[2,1],[4,3]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices")
            .display(DisplayForm::List),
    },
    FunctionInfo {
        name: "charpoly",
        category: "linear algebra",
        signature: "charpoly(matrix)",
        summary: "Characteristic polynomial coefficients.",
        example: "charpoly([[1,2],[3,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices")
            .display(DisplayForm::List),
    },
    FunctionInfo {
        name: "eigenvalues",
        category: "linear algebra",
        signature: "eigenvalues(matrix)",
        summary: "Exact eigenvalues where supported.",
        example: "eigenvalues([[1,2],[3,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices")
            .display(DisplayForm::List),
    },
    FunctionInfo {
        name: "eigenvectors",
        category: "linear algebra",
        signature: "eigenvectors(matrix)",
        summary: "Exact eigenvectors where supported.",
        example: "eigenvectors([[1,1],[0,2]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices")
            .display(DisplayForm::List),
    },
    FunctionInfo {
        name: "nullspace",
        category: "linear algebra",
        signature: "nullspace(matrix)",
        summary: "Basis of the null space.",
        example: "nullspace([[1,2],[2,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices")
            .display(DisplayForm::List),
    },
    FunctionInfo {
        name: "kernel",
        category: "linear algebra",
        signature: "kernel(matrix)",
        summary: "Alias for nullspace.",
        example: "kernel([[1,2],[2,4]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices")
            .display(DisplayForm::List),
    },
    FunctionInfo {
        name: "qr",
        category: "linear algebra",
        signature: "qr(matrix)",
        summary: "Thin QR factors.",
        example: "qr([[1,0],[0,1]])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("exact rational and symbolic matrices")
            .display(DisplayForm::List),
    },
    FunctionInfo {
        name: "isPrime",
        category: "number theory",
        signature: "isPrime(n)",
        summary: "Primality test.",
        example: "isPrime(1000000007)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "nextPrime",
        category: "number theory",
        signature: "nextPrime(n)",
        summary: "Next prime above n.",
        example: "nextPrime(100)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "previousPrime",
        category: "number theory",
        signature: "previousPrime(n)",
        summary: "Previous prime below n.",
        example: "previousPrime(100)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "factorInteger",
        category: "number theory",
        signature: "factorInteger(n)",
        summary: "Prime factorization list.",
        example: "factorInteger(360)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("nonzero integers")
            .display(DisplayForm::PrimeProduct)
            .symbolic_text("Noninteger arguments remain unevaluated.")
            .errors("Zero remains unevaluated."),
    },
    FunctionInfo {
        name: "divisors",
        category: "number theory",
        signature: "divisors(n)",
        summary: "Positive divisors.",
        example: "divisors(36)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers").display(DisplayForm::List),
    },
    FunctionInfo {
        name: "divisorCount",
        category: "number theory",
        signature: "divisorCount(n)",
        summary: "Number of positive divisors.",
        example: "divisorCount(360)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "totient",
        category: "number theory",
        signature: "totient(n)",
        summary: "Euler totient.",
        example: "totient(360)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "sigma",
        category: "number theory",
        signature: "sigma(n)",
        summary: "Sum of positive divisors.",
        example: "sigma(36)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "isEven",
        category: "number theory",
        signature: "isEven(n)",
        summary: "Parity test.",
        example: "isEven(42)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "isOdd",
        category: "number theory",
        signature: "isOdd(n)",
        summary: "Parity test.",
        example: "isOdd(43)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "divides",
        category: "number theory",
        signature: "divides(a,b)",
        summary: "Whether a divides b.",
        example: "divides(3,12)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "mod",
        category: "number theory",
        signature: "mod(a,m)",
        summary: "Nonnegative remainder.",
        example: "mod(-7,3)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "powerMod",
        category: "number theory",
        signature: "powerMod(a,e,m)",
        summary: "Modular exponentiation.",
        example: "powerMod(2,10,1000)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "moebius",
        category: "number theory",
        signature: "moebius(n)",
        summary: "Moebius function.",
        example: "moebius(30)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "radical",
        category: "number theory",
        signature: "radical(n)",
        summary: "Product of distinct prime factors.",
        example: "radical(360)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "isSquareFree",
        category: "number theory",
        signature: "isSquareFree(n)",
        summary: "Square-free predicate.",
        example: "isSquareFree(30)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "primeCount",
        category: "number theory",
        signature: "primeCount(n)",
        summary: "Number of primes at most n.",
        example: "primeCount(1000)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "fibonacci",
        category: "number theory",
        signature: "fibonacci(n)",
        summary: "Fibonacci number.",
        example: "fibonacci(20)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "isPerfectSquare",
        category: "number theory",
        signature: "isPerfectSquare(n)",
        summary: "Perfect square test.",
        example: "isPerfectSquare(144)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "modInverse",
        category: "number theory",
        signature: "modInverse(a,m)",
        summary: "Modular inverse.",
        example: "modInverse(3,7)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "jacobi",
        category: "number theory",
        signature: "jacobi(a,n)",
        summary: "Jacobi symbol.",
        example: "jacobi(5,21)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("integers"),
    },
    FunctionInfo {
        name: "mean",
        category: "statistics",
        signature: "mean(list)",
        summary: "Exact arithmetic mean.",
        example: "mean([1,2,3,4])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "median",
        category: "statistics",
        signature: "median(list)",
        summary: "Exact median.",
        example: "median([1,2,3,4])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "minimum",
        category: "statistics",
        signature: "minimum(list)",
        summary: "Smallest value.",
        example: "minimum([3,1,2])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "maximum",
        category: "statistics",
        signature: "maximum(list)",
        summary: "Largest value.",
        example: "maximum([3,1,2])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "range",
        category: "statistics",
        signature: "range(list)",
        summary: "Maximum minus minimum.",
        example: "range([3,1,2])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "total",
        category: "statistics",
        signature: "total(list)",
        summary: "Sum of values.",
        example: "total([1,2,3,4])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "count",
        category: "statistics",
        signature: "count(list)",
        summary: "Number of values.",
        example: "count([1,2,3,4])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "variance",
        category: "statistics",
        signature: "variance(list)",
        summary: "Population variance.",
        example: "variance([1,2,3,4])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "sampleVariance",
        category: "statistics",
        signature: "sampleVariance(list)",
        summary: "Sample variance.",
        example: "sampleVariance([1,2,3,4])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "standardDeviation",
        category: "statistics",
        signature: "standardDeviation(list)",
        summary: "Population standard deviation.",
        example: "standardDeviation([4,4,4,4])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "sampleStandardDeviation",
        category: "statistics",
        signature: "sampleStandardDeviation(list)",
        summary: "Sample standard deviation.",
        example: "sampleStandardDeviation([4,4,4,4])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists"),
    },
    FunctionInfo {
        name: "frequency",
        category: "statistics",
        signature: "frequency(list)",
        summary: "Sorted value counts.",
        example: "frequency([1,1,2])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists").display(DisplayForm::List),
    },
    FunctionInfo {
        name: "mode",
        category: "statistics",
        signature: "mode(list)",
        summary: "Most frequent values.",
        example: "mode([1,1,2])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("numeric lists").display(DisplayForm::List),
    },
    FunctionInfo {
        name: "and",
        category: "logic",
        signature: "and(a,b,...)",
        summary: "Logical conjunction.",
        example: "and(true,false)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("boolean expressions").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "or",
        category: "logic",
        signature: "or(a,b,...)",
        summary: "Logical disjunction.",
        example: "or(true,false)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("boolean expressions").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "not",
        category: "logic",
        signature: "not(a)",
        summary: "Logical negation.",
        example: "not(true)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("boolean expressions").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "xor",
        category: "logic",
        signature: "xor(a,b)",
        summary: "Exclusive or.",
        example: "xor(true,false)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("boolean expressions").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "implies",
        category: "logic",
        signature: "implies(a,b)",
        summary: "Logical implication.",
        example: "implies(true,false)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("boolean expressions").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "all",
        category: "logic",
        signature: "all(list)",
        summary: "True when every item is true.",
        example: "all([true,true])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("boolean expressions").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "any",
        category: "logic",
        signature: "any(list)",
        summary: "True when at least one item is true.",
        example: "any([false,true])",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("boolean expressions").display(DisplayForm::Boolean),
    },
    FunctionInfo {
        name: "approx",
        category: "numeric",
        signature: "approx(expr[,digits])",
        summary: "Numeric boundary with significance tracking.",
        example: "approx(pi,30)",
        example_kind: ExampleKind::Approximate,
        capability: Capability::approximate("supported numeric values under approx"),
    },
    FunctionInfo {
        name: "interval",
        category: "numeric",
        signature: "interval(low,high)",
        summary: "Approximate interval value.",
        example: "interval(1,2)+3",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "complex",
        category: "numeric",
        signature: "complex(real,imaginary)",
        summary: "Approximate complex value.",
        example: "approx(2+3*i,30)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "abs",
        category: "numeric",
        signature: "abs(expr)",
        summary: "Absolute value with assumption support.",
        example: "abs(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "sqrt",
        category: "numeric",
        signature: "sqrt(expr)",
        summary: "Square root.",
        example: "sqrt(x^2)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "exp",
        category: "numeric",
        signature: "exp(expr)",
        summary: "Exponential function.",
        example: "exp(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "ln",
        category: "numeric",
        signature: "ln(expr)",
        summary: "Natural logarithm.",
        example: "ln(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "sin",
        category: "numeric",
        signature: "sin(expr)",
        summary: "Sine.",
        example: "sin(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "cos",
        category: "numeric",
        signature: "cos(expr)",
        summary: "Cosine.",
        example: "cos(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "tan",
        category: "numeric",
        signature: "tan(expr)",
        summary: "Tangent.",
        example: "tan(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "asin",
        category: "numeric",
        signature: "asin(expr)",
        summary: "Inverse sine.",
        example: "asin(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "acos",
        category: "numeric",
        signature: "acos(expr)",
        summary: "Inverse cosine.",
        example: "acos(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "atan",
        category: "numeric",
        signature: "atan(expr)",
        summary: "Inverse tangent.",
        example: "atan(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "sinh",
        category: "numeric",
        signature: "sinh(expr)",
        summary: "Hyperbolic sine.",
        example: "sinh(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "cosh",
        category: "numeric",
        signature: "cosh(expr)",
        summary: "Hyperbolic cosine.",
        example: "cosh(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "tanh",
        category: "numeric",
        signature: "tanh(expr)",
        summary: "Hyperbolic tangent.",
        example: "tanh(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "asinh",
        category: "numeric",
        signature: "asinh(expr)",
        summary: "Inverse hyperbolic sine.",
        example: "asinh(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "acosh",
        category: "numeric",
        signature: "acosh(expr)",
        summary: "Inverse hyperbolic cosine.",
        example: "acosh(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "atanh",
        category: "numeric",
        signature: "atanh(expr)",
        summary: "Inverse hyperbolic tangent.",
        example: "atanh(x)",
        example_kind: ExampleKind::Symbolic,
        capability: Capability::exact("supported exact numeric inputs"),
    },
    FunctionInfo {
        name: "gamma",
        category: "numeric",
        signature: "gamma(expr)",
        summary: "Gamma function.",
        example: "gamma(5)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact(
            "all numeric arguments, with exact integer and half-integer values",
        )
        .symbolic_text("General symbolic arguments remain symbolic.")
        .approximate_text("Numeric arguments evaluate at the current precision.")
        .errors("Non-positive integers return undefined."),
    },
    FunctionInfo {
        name: "lgamma",
        category: "numeric",
        signature: "lgamma(expr)",
        summary: "Log gamma function.",
        example: "lgamma(5)",
        example_kind: ExampleKind::Evaluates,
        capability: Capability::exact(
            "all numeric arguments, with exact integer and half-integer values",
        )
        .symbolic_text("General symbolic arguments remain symbolic.")
        .approximate_text("Numeric arguments evaluate at the current precision.")
        .errors("Non-positive integers return undefined."),
    },
    FunctionInfo {
        name: "erf",
        category: "numeric",
        signature: "erf(expr)",
        summary: "Error function.",
        example: "approx(erf(1),30)",
        example_kind: ExampleKind::Approximate,
        capability: Capability::approximate("supported numeric values under approx"),
    },
    FunctionInfo {
        name: "erfc",
        category: "numeric",
        signature: "erfc(expr)",
        summary: "Complementary error function.",
        example: "approx(erfc(1),30)",
        example_kind: ExampleKind::Approximate,
        capability: Capability::approximate("supported numeric values under approx"),
    },
    FunctionInfo {
        name: "erfi",
        category: "numeric",
        signature: "erfi(expr)",
        summary: "Imaginary error function.",
        example: "approx(erfi(1),30)",
        example_kind: ExampleKind::Approximate,
        capability: Capability::approximate("supported numeric values under approx"),
    },
    FunctionInfo {
        name: "set",
        category: "language",
        signature: "set(symbol,value)",
        summary: "Immediate definition.",
        example: "set(f,3)",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "setDelayed",
        category: "language",
        signature: "setDelayed(lhs,rhs)",
        summary: "Delayed definition.",
        example: "setDelayed(f(x_),x^2)",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "fn",
        category: "language",
        signature: "fn(params)=>body",
        summary: "Anonymous function.",
        example: "fn(x)=>x^2",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "let",
        category: "language",
        signature: "let(binding,expr)",
        summary: "Lexical binding.",
        example: "let([x=2],x)",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "if",
        category: "language",
        signature: "if(condition,then,else)",
        summary: "Conditional expression.",
        example: "if(true,1,2)",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "map",
        category: "language",
        signature: "map(list,function)",
        summary: "Apply a function to every item.",
        example: "map([1,2],fn(x)=>x^2)",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "apply",
        category: "language",
        signature: "apply(function,list)",
        summary: "Apply a function to an argument list.",
        example: "apply(fn(x,y)=>x+y,[3,4])",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "fold",
        category: "language",
        signature: "fold(function,initial,list)",
        summary: "Fold a function over a list.",
        example: "fold(fn(x,y)=>x+y,0,[1,2,3])",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
    FunctionInfo {
        name: "nest",
        category: "language",
        signature: "nest(function,value,count)",
        summary: "Repeated function application.",
        example: "nest(fn(x)=>x+1,0,5)",
        example_kind: ExampleKind::Language,
        capability: Capability::language(),
    },
];

/// Find one catalog entry by exact name.
pub fn find(name: &str) -> Option<&'static FunctionInfo> {
    FUNCTIONS.iter().find(|function| function.name == name)
}

/// Test whether a catalog entry matches a free text query.
pub fn matches(function: &FunctionInfo, query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    function.category.to_ascii_lowercase().contains(&query)
        || function.name.to_ascii_lowercase().contains(&query)
        || function.signature.to_ascii_lowercase().contains(&query)
        || function.summary.to_ascii_lowercase().contains(&query)
        || function
            .capability
            .domain
            .to_ascii_lowercase()
            .contains(&query)
        || function
            .capability
            .support
            .label()
            .to_ascii_lowercase()
            .contains(&query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_entries_are_unique_and_complete() {
        let mut names = HashSet::new();
        for function in FUNCTIONS {
            assert!(names.insert(function.name), "duplicate {}", function.name);
            assert!(!function.signature.is_empty());
            assert!(!function.summary.is_empty());
            assert!(!function.example.is_empty());
            assert!(CATEGORIES.contains(&function.category));
            assert!(!function.capability.domain.is_empty());
            assert!(!function.capability.symbolic.is_empty());
            assert!(!function.capability.approximate.is_empty());
            assert!(!function.capability.errors.is_empty());
            assert!(!function.capability.support.label().is_empty());
            assert!(!function.capability.display.label().is_empty());
            assert!(!function.example_kind.label().is_empty());
        }
    }

    #[test]
    fn capability_matrix_is_classified() {
        let gamma = find("gamma").unwrap().capability;
        assert_eq!(gamma.support, Support::Exact);
        assert!(gamma.domain.contains("half-integer"));
        let det = find("det").unwrap().capability;
        assert_eq!(det.support, Support::Exact);
        let series = find("series").unwrap().capability;
        assert_eq!(series.support, Support::Symbolic);
        assert_eq!(series.display, DisplayForm::OrderedSeries);
        let factors = find("factorInteger").unwrap().capability;
        assert_eq!(factors.support, Support::Exact);
        assert_eq!(factors.display, DisplayForm::PrimeProduct);
        assert_eq!(
            find("isPrime").unwrap().capability.display,
            DisplayForm::Boolean
        );
    }

    #[test]
    fn advertised_names_have_an_implementation_path() {
        let special = [
            "and",
            "or",
            "not",
            "assume",
            "assuming",
            "approx",
            "interval",
            "complex",
            "set",
            "setDelayed",
            "fn",
            "function",
            "let",
            "if",
            "map",
            "apply",
            "fold",
            "nest",
            "abs",
            "sqrt",
            "exp",
            "ln",
            "sin",
            "cos",
            "tan",
            "asin",
            "acos",
            "atan",
            "sinh",
            "cosh",
            "tanh",
            "asinh",
            "acosh",
            "atanh",
        ];
        for function in FUNCTIONS {
            assert!(
                crate::eval::has_kernel_function(function.name)
                    || crate::stdlib::is_library_function(function.name)
                    || function.capability.support == Support::Approximate
                    || special.contains(&function.name),
                "catalog entry {} has no evaluator path",
                function.name
            );
        }
    }

    #[test]
    fn library_exports_are_documented() {
        let documentation = include_str!("../docs/modules.md");
        for (module, name) in crate::stdlib::exports() {
            assert!(
                find(&name).is_some()
                    || documentation.contains(&format!("`{name}("))
                    || documentation.contains(&format!("`{name}`")),
                "library export {module}.{name} is missing from the documentation"
            );
        }
    }

    #[test]
    fn capability_documentation_matches_the_catalog() {
        let documentation = include_str!("../docs/functions.md");
        for function in FUNCTIONS {
            assert!(
                documentation.contains(&format!("`{}`", function.name)),
                "missing documentation for {}",
                function.name
            );
            assert!(
                documentation.contains(function.capability.support.label()),
                "missing support label for {}",
                function.name
            );
        }
    }
}
