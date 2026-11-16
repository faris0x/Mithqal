//! Kernel head dispatch for algebra, calculus, logic, and numeric families.

use super::*;

impl<'a> Evaluator<'a> {
    pub(super) fn apply(&mut self, head: ExprId, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if head == self.heads.plus {
            return self.plus(args);
        }
        if head == self.heads.times {
            return self.times(args);
        }
        if head == self.heads.power {
            return self.power(args);
        }
        if head == self.heads.negate {
            return self.negate(args);
        }
        if head == self.heads.list {
            return Ok(self.arena.intern_call(self.heads.list, args));
        }
        if head == self.heads.same {
            return self.same(args);
        }
        if head == self.heads.equal {
            return self.equal(args);
        }
        if head == self.heads.not_equal {
            return self.not_equal(args);
        }
        if head == self.heads.less {
            return self.ordering(args, Ordering::Less, self.heads.less);
        }
        if head == self.heads.less_equal {
            return self.ordering(args, Ordering::Less, self.heads.less_equal);
        }
        if head == self.heads.greater {
            return self.ordering(args, Ordering::Greater, self.heads.greater);
        }
        if head == self.heads.greater_equal {
            return self.ordering(args, Ordering::Greater, self.heads.greater_equal);
        }
        if args.len() == 1 {
            let interval_unary = head == self.heads.abs
                || head == self.heads.sqrt
                || head == self.heads.exp
                || head == self.heads.ln
                || head == self.heads.sin
                || head == self.heads.cos;
            if interval_unary
                && let Some((low, high)) = self.interval_bounds(args[0])
                && let Some(result) = self.interval_unary(head, &low, &high)
            {
                return Ok(result);
            }
            if head == self.heads.abs
                && let Some(result) = self.simplify_abs(args[0])
            {
                return Ok(result);
            }
            if head == self.heads.sqrt
                && let Some(result) = self.simplify_sqrt(args[0])
            {
                return Ok(result);
            }
            if let Some(value) = self.exact_constant_value(args[0]) {
                if head == self.heads.exp && value.is_zero() {
                    return Ok(self.arena.intern_int(Int::one()));
                }
                if head == self.heads.ln && value.is_one() {
                    return Ok(self.arena.intern_int(Int::zero()));
                }
                if (head == self.heads.sin || head == self.heads.tan || head == self.heads.atan)
                    && value.is_zero()
                {
                    return Ok(self.arena.intern_int(Int::zero()));
                }
                if head == self.heads.cos && value.is_zero() {
                    return Ok(self.arena.intern_int(Int::one()));
                }
                if head == self.heads.atan
                    && value.is_one()
                    && let Some(angle) = self.pi_fraction(1, 4)
                {
                    return Ok(angle);
                }
                if head == self.heads.asin {
                    if value.is_zero() {
                        return Ok(self.arena.intern_int(Int::zero()));
                    }
                    if value.is_one()
                        && let Some(angle) = self.pi_fraction(1, 2)
                    {
                        return Ok(angle);
                    }
                    if value.neg().is_one()
                        && let Some(angle) = self.pi_fraction(-1, 2)
                    {
                        return Ok(angle);
                    }
                }
                if head == self.heads.acos {
                    if value.is_one() {
                        return Ok(self.arena.intern_int(Int::zero()));
                    }
                    if value.is_zero()
                        && let Some(angle) = self.pi_fraction(1, 2)
                    {
                        return Ok(angle);
                    }
                    if value.neg().is_one()
                        && let Some(angle) = self.pi_fraction(1, 1)
                    {
                        return Ok(angle);
                    }
                }
            }
        }
        let kernel = self
            .arena
            .symbol_id(head)
            .and_then(|symbol| KernelFunction::from_name(self.arena.symbol_name(symbol)));
        if let Some(kernel) = kernel {
            return self.apply_kernel(kernel, head, args);
        }
        if let Some(symbol) = self.arena.symbol_id(head)
            && let Some(result) = self.apply_user(symbol, head, args)?
        {
            return Ok(result);
        }
        Ok(self.arena.intern_call(head, args))
    }

    pub(super) fn apply_kernel(
        &mut self,
        function: KernelFunction,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        match function {
            KernelFunction::Algebra(function) => self.apply_algebra(function, head, args),
            KernelFunction::Calculus(function) => self.apply_calculus(function, head, args),
            KernelFunction::Numeric(function) => {
                let original = self.arena.intern_call(head, args);
                Ok(self
                    .apply_exact_numeric(function, args)?
                    .unwrap_or(original))
            }
            KernelFunction::Linear(function) => {
                if args.len() != function.arity() {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "linear algebra function received the wrong number of arguments",
                    ));
                }
                let original = self.arena.intern_call(head, args);
                Ok(self.apply_linear(function, args).unwrap_or(original))
            }
            KernelFunction::NumberTheory(function) => {
                if args.len() != function.arity() {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "number theory function received the wrong number of arguments",
                    ));
                }
                let original = self.arena.intern_call(head, args);
                Ok(self.apply_number_theory(function, args).unwrap_or(original))
            }
            KernelFunction::Logic(function) => {
                let arity = match function {
                    LogicHead::Xor | LogicHead::Implies => 2,
                    LogicHead::All | LogicHead::Any => 1,
                };
                if args.len() != arity {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "logic function received the wrong number of arguments",
                    ));
                }
                let original = self.arena.intern_call(head, args);
                Ok(self.apply_logic(function, args).unwrap_or(original))
            }
            KernelFunction::Statistics(function) => {
                if args.len() != 1 {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "statistics function expects one list",
                    ));
                }
                let original = self.arena.intern_call(head, args);
                Ok(self.apply_statistics(function, args[0]).unwrap_or(original))
            }
        }
    }

    pub(super) fn apply_algebra(
        &mut self,
        function: AlgebraHead,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        let context = crate::algebra::Context {
            plus: self.heads.plus,
            times: self.heads.times,
            power: self.heads.power,
            negate: self.heads.negate,
            list: self.heads.list,
            equal: self.heads.equal,
        };
        match function {
            AlgebraHead::Expand => self.algebra_unary(args, head, context, crate::algebra::expand),
            AlgebraHead::Factor => self.algebra_unary(args, head, context, crate::algebra::factor),
            AlgebraHead::Together => {
                self.algebra_unary(args, head, context, crate::algebra::together)
            }
            AlgebraHead::Cancel => self.algebra_unary(args, head, context, crate::algebra::cancel),
            AlgebraHead::PolynomialGcd => {
                require_arity(
                    args,
                    3,
                    "polynomialGcd expects two polynomials and a variable",
                )?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::polynomial_gcd(self.arena, args[0], args[1], args[2], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::Resultant => {
                require_arity(args, 3, "resultant expects two polynomials and a variable")?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::resultant(self.arena, args[0], args[1], args[2], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::Solve => {
                let normalized = if let [expression] = args {
                    vec![*expression, self.infer_variable(*expression)?]
                } else {
                    args.to_vec()
                };
                require_arity(&normalized, 2, "solve expects an equation and a variable")?;
                let original = self.arena.intern_call(head, &normalized);
                Ok(
                    crate::algebra::solve(self.arena, normalized[0], normalized[1], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::Degree => {
                require_arity(args, 2, "degree expects a polynomial and a variable")?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::degree(self.arena, args[0], args[1], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::Coefficient => {
                require_arity(
                    args,
                    3,
                    "coefficient expects a polynomial, a variable, and an index",
                )?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::coefficient(self.arena, args[0], args[1], args[2], &context)
                        .unwrap_or(original),
                )
            }
            AlgebraHead::PolynomialDivide => {
                require_arity(
                    args,
                    3,
                    "polynomialDivide expects a dividend, a divisor, and a variable",
                )?;
                let original = self.arena.intern_call(head, args);
                Ok(crate::algebra::polynomial_divide(
                    self.arena, args[0], args[1], args[2], &context,
                )
                .unwrap_or(original))
            }
            AlgebraHead::Discriminant => {
                require_arity(args, 2, "discriminant expects a polynomial and a variable")?;
                let original = self.arena.intern_call(head, args);
                Ok(
                    crate::algebra::discriminant(self.arena, args[0], args[1], &context)
                        .unwrap_or(original),
                )
            }
        }
    }

    pub(super) fn apply_calculus(
        &mut self,
        function: CalculusHead,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<ExprId, Diagnostic> {
        let normalized = match (function, args) {
            (CalculusHead::Differentiate, [expression]) => {
                vec![*expression, self.infer_variable(*expression)?]
            }
            (CalculusHead::Integrate, [expression]) => {
                vec![*expression, self.infer_variable(*expression)?]
            }
            (CalculusHead::Limit, [expression, point]) => {
                vec![*expression, self.infer_variable(*expression)?, *point]
            }
            (CalculusHead::Series, [expression, point, order]) => {
                vec![
                    *expression,
                    self.infer_variable(*expression)?,
                    *point,
                    *order,
                ]
            }
            (CalculusHead::Converges, [expression, point]) => {
                vec![*expression, self.infer_variable(*expression)?, *point]
            }
            _ => args.to_vec(),
        };
        let args = normalized.as_slice();
        let valid = matches!(
            (function, args.len()),
            (CalculusHead::Differentiate, 2)
                | (CalculusHead::Integrate, 2 | 4)
                | (CalculusHead::Limit, 3 | 4)
                | (CalculusHead::Series, 4)
                | (CalculusHead::Converges, 3)
        );
        if !valid {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "calculus function received the wrong number of arguments",
            ));
        }
        let original = self.arena.intern_call(head, args);
        if self.arena.symbol_id(args[1]).is_none() {
            return Ok(original);
        }
        if matches!(function, CalculusHead::Limit) {
            return self.apply_limit(head, args);
        }
        if matches!(function, CalculusHead::Series) {
            return self.apply_series(head, args);
        }
        if matches!(function, CalculusHead::Converges) {
            return self.apply_converges(head, args);
        }
        if matches!(function, CalculusHead::Integrate) && args.len() == 4 {
            return Ok(self
                .definite_integral(args[0], args[1], args[2], args[3])
                .unwrap_or(original));
        }
        let context = crate::calculus::Context {
            plus: self.heads.plus,
            times: self.heads.times,
            power: self.heads.power,
            negate: self.heads.negate,
            list: self.heads.list,
            equal: self.heads.equal,
        };
        let result = match function {
            CalculusHead::Differentiate => {
                crate::calculus::differentiate(self.arena, args[0], args[1], &context)
            }
            CalculusHead::Integrate if args.len() == 4 => None,
            CalculusHead::Integrate => {
                crate::calculus::integrate(self.arena, args[0], args[1], &context)
            }
            CalculusHead::Limit | CalculusHead::Series | CalculusHead::Converges => None,
        };
        let result = result.map(|value| {
            if matches!(function, CalculusHead::Differentiate) {
                let algebra = crate::algebra::Context {
                    plus: self.heads.plus,
                    times: self.heads.times,
                    power: self.heads.power,
                    negate: self.heads.negate,
                    list: self.heads.list,
                    equal: self.heads.equal,
                };
                crate::algebra::cancel(self.arena, value, &algebra).unwrap_or(value)
            } else {
                value
            }
        });
        Ok(result.unwrap_or(original))
    }

    pub(super) fn definite_integral(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        lower: ExprId,
        upper: ExprId,
    ) -> Option<ExprId> {
        if self.interval_has_interior_pole(expression, variable, lower, upper) {
            return None;
        }
        let context = self.calculus_context();
        let antiderivative =
            crate::calculus::integrate(self.arena, expression, variable, &context)?;
        if self.has_transcendental_singularity(antiderivative, variable, lower, upper) {
            return None;
        }
        let lower_value = self.endpoint_limit(antiderivative, variable, lower, LimitSide::Right)?;
        let upper_value = self.endpoint_limit(antiderivative, variable, upper, LimitSide::Left)?;
        let negative_lower = self.negate_limit_value(lower_value);
        match self.add_limit_values(upper_value, negative_lower)? {
            LimitValue::Finite(value) => Some(value),
            _ => None,
        }
    }

    pub(super) fn has_transcendental_singularity(
        &mut self,
        antiderivative: ExprId,
        variable: ExprId,
        lower: ExprId,
        upper: ExprId,
    ) -> bool {
        let (Some(lower), Some(upper)) = (self.as_rat(lower), self.as_rat(upper)) else {
            return false;
        };
        let (lower, upper) = if lower <= upper {
            (lower, upper)
        } else {
            (upper, lower)
        };
        let mut arguments = Vec::new();
        self.collect_singularity_arguments(antiderivative, variable, &mut arguments);
        arguments
            .into_iter()
            .any(|argument| self.argument_has_interior_zero(argument, variable, &lower, &upper))
    }

    pub(super) fn collect_singularity_arguments(
        &self,
        expression: ExprId,
        variable: ExprId,
        arguments: &mut Vec<ExprId>,
    ) {
        let Node::Call { head, args } = self.arena.node(expression) else {
            return;
        };
        let name = self
            .arena
            .symbol_id(*head)
            .map(|symbol| self.arena.symbol_name(symbol));
        if name == Some("ln") && args.len() == 1 && self.expression_contains(args[0], variable) {
            let argument = self.abs_argument(args[0]).unwrap_or(args[0]);
            arguments.push(argument);
        }
        if name == Some("power")
            && args.len() == 2
            && self.expression_contains(args[0], variable)
            && self
                .exact_constant_value(args[1])
                .is_some_and(|exponent| exponent.sign() < 0)
        {
            arguments.push(args[0]);
        }
        for argument in args.iter() {
            self.collect_singularity_arguments(*argument, variable, arguments);
        }
    }

    pub(super) fn argument_has_interior_zero(
        &mut self,
        argument: ExprId,
        variable: ExprId,
        lower: &Rat,
        upper: &Rat,
    ) -> bool {
        let context = self.algebra_context();
        if let Some(roots) = crate::algebra::univariate_rational_root_multiplicities(
            self.arena, argument, variable, &context,
        ) && roots
            .iter()
            .any(|(root, _)| *root > *lower && *root < *upper)
        {
            return true;
        }
        let samples = 16i64;
        let denominator = Rat::from_i64(samples + 1, 1).ok();
        let span = upper.sub(lower);
        let Some(denominator) = denominator else {
            return false;
        };
        let mut previous_sign = 0i8;
        for index in 1..=samples {
            let fraction = Rat::from_i64(index, 1)
                .ok()
                .and_then(|value| value.div(&denominator).ok());
            let Some(fraction) = fraction else {
                return false;
            };
            let point = lower.add(&span.mul(&fraction));
            let point_expression = self.rat_expression(&point);
            let substituted = self.substitute_bound(argument, variable, point_expression);
            let Ok(value) = self.eval(substituted) else {
                return false;
            };
            let Ok(approximate) = self.approximate(value, 30) else {
                return false;
            };
            let Some(Numeric::Real(real)) = approximate else {
                return matches!(approximate, Some(Numeric::Complex(_, _)));
            };
            let sign = real.sign();
            if sign == 0 {
                return true;
            }
            if previous_sign != 0 && sign != previous_sign {
                return true;
            }
            previous_sign = sign;
        }
        false
    }

    pub(super) fn interval_has_interior_pole(
        &mut self,
        expression: ExprId,
        variable: ExprId,
        lower: ExprId,
        upper: ExprId,
    ) -> bool {
        let (Some(lower), Some(upper)) = (self.as_rat(lower), self.as_rat(upper)) else {
            return false;
        };
        let (lower, upper) = if lower <= upper {
            (lower, upper)
        } else {
            (upper, lower)
        };
        let cancelled = self.cancel_expression(expression);
        let (_, denominator) = self.split_quotient(cancelled);
        if is_one_value(self.arena, denominator) {
            return false;
        }
        let context = self.algebra_context();
        let Some(roots) = crate::algebra::univariate_rational_root_multiplicities(
            self.arena,
            denominator,
            variable,
            &context,
        ) else {
            return false;
        };
        roots.iter().any(|(root, _)| *root > lower && *root < upper)
    }

    pub(super) fn endpoint_limit(
        &mut self,
        antiderivative: ExprId,
        variable: ExprId,
        point: ExprId,
        finite_side: LimitSide,
    ) -> Option<LimitValue> {
        let side = match self.limit_target(point) {
            LimitTarget::Finite(_) => finite_side,
            _ => LimitSide::Both,
        };
        self.limit_value(antiderivative, variable, point, side)
    }

    pub(super) fn apply_statistics(
        &mut self,
        function: StatisticsHead,
        expression: ExprId,
    ) -> Option<ExprId> {
        match function {
            StatisticsHead::Mean => crate::statistics::mean_stat(self.arena, expression),
            StatisticsHead::Median => crate::statistics::median(self.arena, expression),
            StatisticsHead::Minimum => crate::statistics::minimum(self.arena, expression),
            StatisticsHead::Maximum => crate::statistics::maximum(self.arena, expression),
            StatisticsHead::Range => crate::statistics::range(self.arena, expression),
            StatisticsHead::Total => crate::statistics::total(self.arena, expression),
            StatisticsHead::Count => crate::statistics::count(self.arena, expression),
            StatisticsHead::Variance => crate::statistics::variance_stat(self.arena, expression),
            StatisticsHead::SampleVariance => {
                crate::statistics::sample_variance(self.arena, expression)
            }
            StatisticsHead::StandardDeviation => {
                crate::statistics::standard_deviation(self.arena, expression)
            }
            StatisticsHead::SampleStandardDeviation => {
                crate::statistics::sample_standard_deviation(self.arena, expression)
            }
            StatisticsHead::Frequency => crate::statistics::frequency(self.arena, expression),
            StatisticsHead::Mode => crate::statistics::mode(self.arena, expression),
        }
    }

    pub(super) fn apply_logic(&mut self, function: LogicHead, args: &[ExprId]) -> Option<ExprId> {
        let boolean = |value| {
            if value {
                self.heads.true_value
            } else {
                self.heads.false_value
            }
        };
        match function {
            LogicHead::Xor => {
                let left = self.boolean_value(args[0])?;
                let right = self.boolean_value(args[1])?;
                Some(boolean(left ^ right))
            }
            LogicHead::Implies => {
                let left = self.boolean_value(args[0])?;
                let right = self.boolean_value(args[1])?;
                Some(boolean(!left || right))
            }
            LogicHead::All | LogicHead::Any => {
                let Node::Call { head, args: values } = self.arena.node(args[0]) else {
                    return None;
                };
                let head = *head;
                if self
                    .arena
                    .symbol_id(head)
                    .map(|symbol| self.arena.symbol_name(symbol))
                    != Some("list")
                {
                    return None;
                }
                let values = values.to_vec();
                let mut result = matches!(function, LogicHead::All);
                for value in values {
                    let value = self.boolean_value(value)?;
                    if matches!(function, LogicHead::All) {
                        result &= value;
                    } else {
                        result |= value;
                    }
                }
                Some(boolean(result))
            }
        }
    }

    pub(super) fn boolean_value(&self, value: ExprId) -> Option<bool> {
        if let Node::Symbol(symbol) = self.arena.node(value) {
            match self.arena.symbol_name(*symbol) {
                "true" => return Some(true),
                "false" => return Some(false),
                _ => {}
            }
        }
        None
    }

    pub(super) fn apply_number_theory(
        &mut self,
        function: NumberTheoryHead,
        args: &[ExprId],
    ) -> Option<ExprId> {
        match function {
            NumberTheoryHead::IsPrime => crate::numbertheory::is_prime(self.arena, args[0]),
            NumberTheoryHead::NextPrime => crate::numbertheory::next_prime(self.arena, args[0]),
            NumberTheoryHead::FactorInteger => {
                crate::numbertheory::factor_integer(self.arena, args[0])
            }
            NumberTheoryHead::Divisors => crate::numbertheory::divisors(self.arena, args[0]),
            NumberTheoryHead::Totient => crate::numbertheory::totient(self.arena, args[0]),
            NumberTheoryHead::Sigma => crate::numbertheory::sigma(self.arena, args[0]),
            NumberTheoryHead::IsEven => crate::numbertheory::is_even(self.arena, args[0]),
            NumberTheoryHead::IsOdd => crate::numbertheory::is_odd(self.arena, args[0]),
            NumberTheoryHead::Divides => crate::numbertheory::divides(self.arena, args[0], args[1]),
            NumberTheoryHead::Modulo => crate::numbertheory::modulo(self.arena, args[0], args[1]),
            NumberTheoryHead::PowerMod => {
                crate::numbertheory::power_mod(self.arena, args[0], args[1], args[2])
            }
            NumberTheoryHead::PreviousPrime => {
                crate::numbertheory::previous_prime(self.arena, args[0])
            }
            NumberTheoryHead::DivisorCount => {
                crate::numbertheory::divisor_count(self.arena, args[0])
            }
            NumberTheoryHead::Moebius => crate::numbertheory::moebius(self.arena, args[0]),
            NumberTheoryHead::Radical => crate::numbertheory::radical(self.arena, args[0]),
            NumberTheoryHead::IsSquareFree => {
                crate::numbertheory::is_square_free(self.arena, args[0])
            }
            NumberTheoryHead::PrimeCount => crate::numbertheory::prime_count(self.arena, args[0]),
            NumberTheoryHead::Fibonacci => crate::numbertheory::fibonacci(self.arena, args[0]),
            NumberTheoryHead::IsPerfectSquare => {
                crate::numbertheory::is_perfect_square(self.arena, args[0])
            }
            NumberTheoryHead::ModInverse => {
                crate::numbertheory::mod_inverse(self.arena, args[0], args[1])
            }
            NumberTheoryHead::Jacobi => crate::numbertheory::jacobi(self.arena, args[0], args[1]),
        }
    }

    pub(super) fn apply_linear(&mut self, function: LinearHead, args: &[ExprId]) -> Option<ExprId> {
        match function {
            LinearHead::Determinant => crate::linalg::determinant(self.arena, args[0])
                .or_else(|| crate::linalg::determinant_symbolic(self.arena, args[0])),
            LinearHead::Rank => crate::linalg::rank(self.arena, args[0]),
            LinearHead::Transpose => crate::linalg::transpose(self.arena, args[0])
                .or_else(|| crate::linalg::transpose_symbolic(self.arena, args[0])),
            LinearHead::Inverse => crate::linalg::inverse(self.arena, args[0])
                .or_else(|| crate::linalg::inverse_symbolic(self.arena, args[0])),
            LinearHead::Rref => crate::linalg::rref(self.arena, args[0]),
            LinearHead::Trace => crate::linalg::trace(self.arena, args[0])
                .or_else(|| crate::linalg::trace_symbolic(self.arena, args[0])),
            LinearHead::Matmul => crate::linalg::matmul(self.arena, args[0], args[1])
                .or_else(|| crate::linalg::matmul_symbolic(self.arena, args[0], args[1])),
            LinearHead::Identity => crate::linalg::identity(self.arena, args[0]),
            LinearHead::SolveLinear => crate::linalg::solve_linear(self.arena, args[0], args[1]),
            LinearHead::Dot => crate::linalg::dot(self.arena, args[0], args[1]),
            LinearHead::Cross => crate::linalg::cross(self.arena, args[0], args[1]),
            LinearHead::Plu => crate::linalg::plu(self.arena, args[0]),
            LinearHead::Charpoly => crate::linalg::charpoly(self.arena, args[0]),
            LinearHead::Eigenvalues => crate::linalg::eigenvalues(self.arena, args[0]),
            LinearHead::Nullspace => crate::linalg::nullspace(self.arena, args[0]),
            LinearHead::Eigenvectors => crate::linalg::eigenvectors(self.arena, args[0]),
            LinearHead::Qr => crate::linalg::qr(self.arena, args[0]),
        }
    }

    pub(super) fn apply_exact_numeric(
        &mut self,
        function: ExactNumericHead,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let arity = match function {
            ExactNumericHead::Gcd | ExactNumericHead::Lcm => {
                if args.len() < 2 {
                    return Err(Diagnostic::eval(
                        ErrorCode::EvalArity,
                        "gcd and lcm expect at least two integers",
                    ));
                }
                return Ok(self.apply_fold_integer(function, args));
            }
            ExactNumericHead::Factorial => 1,
            ExactNumericHead::Binomial => 2,
            ExactNumericHead::Floor | ExactNumericHead::Ceil | ExactNumericHead::Round => 1,
        };
        if args.len() != arity {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "numeric function received the wrong number of arguments",
            ));
        }
        Ok(self.apply_simple_numeric(function, args))
    }

    pub(super) fn apply_fold_integer(
        &mut self,
        function: ExactNumericHead,
        args: &[ExprId],
    ) -> Option<ExprId> {
        let mut value = exact_integer(self.arena, args[0])?;
        for arg in &args[1..] {
            let next = exact_integer(self.arena, *arg)?;
            value = match function {
                ExactNumericHead::Gcd => value.gcd(&next),
                ExactNumericHead::Lcm => integer_lcm(&value, &next),
                _ => return None,
            };
        }
        Some(self.arena.intern_int(value))
    }

    pub(super) fn apply_simple_numeric(
        &mut self,
        function: ExactNumericHead,
        args: &[ExprId],
    ) -> Option<ExprId> {
        match function {
            ExactNumericHead::Factorial => {
                let value = exact_integer(self.arena, args[0])?.to_i64()?;
                if !(0..=10_000).contains(&value) {
                    return None;
                }
                Some(self.arena.intern_int(integer_factorial(value)?))
            }
            ExactNumericHead::Binomial => {
                let n = exact_integer(self.arena, args[0])?.to_i64()?;
                let k = exact_integer(self.arena, args[1])?.to_i64()?;
                if n < 0 || k < 0 {
                    return None;
                }
                Some(self.arena.intern_int(integer_binomial(n, k)?))
            }
            ExactNumericHead::Floor | ExactNumericHead::Ceil | ExactNumericHead::Round => {
                if let Node::Real(value) = self.arena.node(args[0]) {
                    let result = match function {
                        ExactNumericHead::Floor => value.floor(),
                        ExactNumericHead::Ceil => value.ceil(),
                        _ => value.round(),
                    };
                    return result.ok().map(|value| self.arena.intern_real(value));
                }
                let rational = exact_rational(self.arena, args[0])?;
                let value = match function {
                    ExactNumericHead::Floor => rational.floor(),
                    ExactNumericHead::Ceil => rational.ceil(),
                    _ => rational.round(),
                };
                Some(self.arena.intern_int(value))
            }
            _ => None,
        }
    }

    pub(super) fn apply_user(
        &mut self,
        symbol: SymbolId,
        head: ExprId,
        args: &[ExprId],
    ) -> Result<Option<ExprId>, Diagnostic> {
        let rules: Vec<Rule> = self.definitions.rules(symbol).to_vec();
        if rules.is_empty() {
            return Ok(None);
        }
        let call = self.arena.intern_call(head, args);
        for rule in rules {
            let mut bindings = Bindings::new();
            let mut conditions = Vec::new();
            if !match_pattern(self.arena, rule.lhs, call, &mut bindings, &mut conditions) {
                continue;
            }
            let mut accepted = true;
            for condition in conditions {
                let condition = substitute(self.arena, condition, &bindings);
                let value = self.eval(condition)?;
                if value != self.heads.true_value {
                    accepted = false;
                    break;
                }
            }
            if !accepted {
                continue;
            }
            let rhs = substitute(self.arena, rule.rhs, &bindings);
            return if rule.delayed {
                self.eval(rhs).map(Some)
            } else {
                Ok(Some(rhs))
            };
        }
        Ok(None)
    }
}
