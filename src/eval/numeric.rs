//! Numeric boundary, interval, and complex evaluation.

use super::*;

impl<'a> Evaluator<'a> {
    pub(super) fn eval_interval(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "interval expects two endpoints",
            ));
        }
        let low = self.eval(args[0])?;
        let high = self.eval(args[1])?;
        if let (Some(low_value), Some(high_value)) =
            (self.real_endpoint(low), self.real_endpoint(high))
            && low_value.cmp(&high_value) == Ordering::Greater
        {
            return Ok(self.heads.failed);
        }
        Ok(self.arena.intern_call(self.heads.interval, &[low, high]))
    }

    pub(super) fn eval_complex(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "complex expects a real and an imaginary part",
            ));
        }
        let real = self.eval(args[0])?;
        let imaginary = self.eval(args[1])?;
        if let Some(value) = self.real_endpoint(imaginary)
            && value.is_zero()
        {
            return Ok(real);
        }
        Ok(self
            .arena
            .intern_call(self.heads.complex, &[real, imaginary]))
    }

    pub(super) fn real_endpoint(&self, id: ExprId) -> Option<Real> {
        match self.arena.node(id) {
            Node::Int(value) => {
                if let Some(value) = value.to_i64() {
                    Real::from_i64(value, 30).ok()
                } else {
                    let rational = Rat::from_ints(value.clone(), Int::one()).ok()?;
                    Real::from_rat(&rational, 30).ok()
                }
            }
            Node::Rat(value) => Real::from_rat(value, 30).ok(),
            Node::Real(value) => Some(value.clone()),
            _ => None,
        }
    }

    pub(super) fn interval_parts(&self, id: ExprId) -> Option<(ExprId, ExprId)> {
        match self.arena.node(id) {
            Node::Call { head, args } if *head == self.heads.interval && args.len() == 2 => {
                Some((args[0], args[1]))
            }
            _ => None,
        }
    }

    pub(super) fn interval_bounds(&self, id: ExprId) -> Option<(Real, Real)> {
        let (low, high) = self.interval_parts(id)?;
        let low = self.real_endpoint(low)?.lower_bound().ok()?;
        let high = self.real_endpoint(high)?.upper_bound().ok()?;
        Some((low, high))
    }

    pub(super) fn interval_node(&mut self, low: Real, high: Real) -> ExprId {
        let (low, high) = if low.cmp(&high) == Ordering::Greater {
            (high, low)
        } else {
            (low, high)
        };
        let low = self.arena.intern_real(low);
        let high = self.arena.intern_real(high);
        self.arena.intern_call(self.heads.interval, &[low, high])
    }

    pub(super) fn add_interval(
        &mut self,
        low: &Real,
        high: &Real,
        other_low: &Real,
        other_high: &Real,
    ) -> Result<(Real, Real), Diagnostic> {
        let low = low
            .add_round(other_low, MPFR_RNDD)
            .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "interval addition failed"))?;
        let high = high
            .add_round(other_high, MPFR_RNDU)
            .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "interval addition failed"))?;
        Ok((low, high))
    }

    pub(super) fn mul_interval(
        &mut self,
        low: &Real,
        high: &Real,
        other_low: &Real,
        other_high: &Real,
    ) -> Result<(Real, Real), Diagnostic> {
        let mut lower_bound: Option<Real> = None;
        let mut upper_bound: Option<Real> = None;
        for (left, right) in [
            (low, other_low),
            (low, other_high),
            (high, other_low),
            (high, other_high),
        ] {
            let lower = left.mul_round(right, MPFR_RNDD).map_err(|_| {
                Diagnostic::eval(ErrorCode::NumericFailure, "interval product failed")
            })?;
            let upper = left.mul_round(right, MPFR_RNDU).map_err(|_| {
                Diagnostic::eval(ErrorCode::NumericFailure, "interval product failed")
            })?;
            let replace_low = match &lower_bound {
                Some(current) => lower.cmp(current) == Ordering::Less,
                None => true,
            };
            if replace_low {
                lower_bound = Some(lower);
            }
            let replace_high = match &upper_bound {
                Some(current) => upper.cmp(current) == Ordering::Greater,
                None => true,
            };
            if replace_high {
                upper_bound = Some(upper);
            }
        }
        Ok((
            lower_bound.expect("interval product has a lower bound"),
            upper_bound.expect("interval product has an upper bound"),
        ))
    }

    pub(super) fn plus_intervals(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut scalars = Vec::new();
        let mut intervals = Vec::new();
        for arg in args {
            if let Some(bounds) = self.interval_bounds(*arg) {
                intervals.push(bounds);
            } else {
                scalars.push(*arg);
            }
        }
        let scalar = if scalars.is_empty() {
            Real::zero(30)
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real failed"))?
        } else {
            let sum = self.plus(&scalars)?;
            let Some(value) = self.real_endpoint(sum) else {
                return Ok(self.arena.intern_call(self.heads.plus, args));
            };
            value
        };
        let mut low = scalar.clone();
        let mut high = scalar;
        for (other_low, other_high) in intervals {
            let (next_low, next_high) = self.add_interval(&low, &high, &other_low, &other_high)?;
            low = next_low;
            high = next_high;
        }
        Ok(self.interval_node(low, high))
    }

    pub(super) fn times_intervals(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        let mut scalars = Vec::new();
        let mut intervals = Vec::new();
        for arg in args {
            if let Some(bounds) = self.interval_bounds(*arg) {
                intervals.push(bounds);
            } else {
                scalars.push(*arg);
            }
        }
        let scalar = if scalars.is_empty() {
            Real::one(30).map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real failed"))?
        } else {
            let product = self.times(&scalars)?;
            let Some(value) = self.real_endpoint(product) else {
                return Ok(self.arena.intern_call(self.heads.times, args));
            };
            value
        };
        let mut low = scalar.clone();
        let mut high = scalar;
        for (other_low, other_high) in intervals {
            let (next_low, next_high) = self.mul_interval(&low, &high, &other_low, &other_high)?;
            low = next_low;
            high = next_high;
        }
        Ok(self.interval_node(low, high))
    }

    pub(super) fn interval_power(
        &mut self,
        low: &Real,
        high: &Real,
        exponent: i64,
    ) -> Option<ExprId> {
        if exponent == 0 {
            let one = Real::one(30).ok()?;
            return Some(self.interval_node(one.clone(), one));
        }
        if exponent < 0 {
            if low.sign() <= 0 && high.sign() >= 0 {
                return None;
            }
            let positive = exponent.unsigned_abs() as i64;
            let (low_power, high_power) = self.interval_positive_power(low, high, positive)?;
            let one = Real::one(30).ok()?;
            let low_result = one.div_round(&high_power, MPFR_RNDD).ok()?;
            let high_result = one.div_round(&low_power, MPFR_RNDU).ok()?;
            return Some(self.interval_node(low_result, high_result));
        }
        let (low_power, high_power) = self.interval_positive_power(low, high, exponent)?;
        Some(self.interval_node(low_power, high_power))
    }

    pub(super) fn interval_positive_power(
        &mut self,
        low: &Real,
        high: &Real,
        exponent: i64,
    ) -> Option<(Real, Real)> {
        let low_power = low.pow_si_round(exponent, MPFR_RNDD).ok()?;
        let high_power = high.pow_si_round(exponent, MPFR_RNDU).ok()?;
        if exponent % 2 == 0 && low.sign() < 0 && high.sign() > 0 {
            let zero = Real::zero(30).ok()?;
            let low_abs = low.abs().ok()?;
            let high_abs = high.abs().ok()?;
            let high_magnitude = if low_abs.cmp(&high_abs) == Ordering::Greater {
                low_abs
            } else {
                high_abs
            };
            let upper = high_magnitude.pow_si_round(exponent, MPFR_RNDU).ok()?;
            return Some((zero, upper));
        }
        if exponent % 2 == 0 && high.sign() <= 0 {
            let upper = low.pow_si_round(exponent, MPFR_RNDU).ok()?;
            let lower = high.pow_si_round(exponent, MPFR_RNDD).ok()?;
            return Some((lower, upper));
        }
        Some((low_power, high_power))
    }

    pub(super) fn interval_unary(
        &mut self,
        head: ExprId,
        low: &Real,
        high: &Real,
    ) -> Option<ExprId> {
        let (low_value, high_value) = if head == self.heads.sqrt {
            if high.sign() < 0 {
                return None;
            }
            let zero = Real::zero(30).ok()?;
            let low = if low.sign() < 0 { &zero } else { low };
            (
                low.sqrt_round(MPFR_RNDD).ok()?,
                high.sqrt_round(MPFR_RNDU).ok()?,
            )
        } else if head == self.heads.exp {
            (
                low.exp_round(MPFR_RNDD).ok()?,
                high.exp_round(MPFR_RNDU).ok()?,
            )
        } else if head == self.heads.ln {
            if low.sign() <= 0 {
                return None;
            }
            (
                low.ln_round(MPFR_RNDD).ok()?,
                high.ln_round(MPFR_RNDU).ok()?,
            )
        } else if head == self.heads.abs {
            if low.sign() >= 0 {
                (low.clone(), high.clone())
            } else if high.sign() <= 0 {
                (high.neg().ok()?, low.neg().ok()?)
            } else {
                let low_magnitude = low.neg().ok()?;
                let high_magnitude = if low_magnitude.cmp(high) == Ordering::Greater {
                    low_magnitude
                } else {
                    high.clone()
                };
                (Real::zero(30).ok()?, high_magnitude)
            }
        } else if head == self.heads.sin || head == self.heads.cos {
            let minus_one = Real::from_i64(-1, 30).ok()?;
            let one = Real::one(30).ok()?;
            self.interval_trig(low, high, head == self.heads.sin, &minus_one, &one)?
        } else {
            return None;
        };
        if !low_value.is_finite() || !high_value.is_finite() {
            return None;
        }
        Some(self.interval_node(low_value, high_value))
    }

    pub(super) fn interval_trig(
        &self,
        low: &Real,
        high: &Real,
        sine: bool,
        minus_one: &Real,
        one: &Real,
    ) -> Option<(Real, Real)> {
        let low_value = low.to_f64();
        let high_value = high.to_f64();
        if !low_value.is_finite() || !high_value.is_finite() {
            return None;
        }
        let magnitude = low_value.abs().max(high_value.abs());
        let epsilon = 1e-12 * magnitude.max(1.0);
        let two_pi = 2.0 * core::f64::consts::PI;
        if high_value - low_value >= two_pi - epsilon || magnitude > 1e15 {
            return Some((minus_one.clone(), one.clone()));
        }
        let (low_lower, low_upper) = if sine {
            (
                low.sin_round(MPFR_RNDD).ok()?,
                low.sin_round(MPFR_RNDU).ok()?,
            )
        } else {
            (
                low.cos_round(MPFR_RNDD).ok()?,
                low.cos_round(MPFR_RNDU).ok()?,
            )
        };
        let (high_lower, high_upper) = if sine {
            (
                high.sin_round(MPFR_RNDD).ok()?,
                high.sin_round(MPFR_RNDU).ok()?,
            )
        } else {
            (
                high.cos_round(MPFR_RNDD).ok()?,
                high.cos_round(MPFR_RNDU).ok()?,
            )
        };
        let mut lower = if low_lower.cmp(&high_lower) == Ordering::Less {
            low_lower
        } else {
            high_lower
        };
        let mut upper = if low_upper.cmp(&high_upper) == Ordering::Greater {
            low_upper
        } else {
            high_upper
        };
        let lower_phase = if sine {
            -core::f64::consts::FRAC_PI_2
        } else {
            core::f64::consts::PI
        };
        let upper_phase = if sine {
            core::f64::consts::FRAC_PI_2
        } else {
            0.0
        };
        if interval_contains_phase(
            low_value - epsilon,
            high_value + epsilon,
            lower_phase,
            two_pi,
        ) {
            lower = minus_one.clone();
        }
        if interval_contains_phase(
            low_value - epsilon,
            high_value + epsilon,
            upper_phase,
            two_pi,
        ) {
            upper = one.clone();
        }
        if lower.cmp(minus_one) == Ordering::Less {
            lower = minus_one.clone();
        }
        if upper.cmp(one) == Ordering::Greater {
            upper = one.clone();
        }
        Some((lower, upper))
    }

    pub(super) fn eval_approx(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.is_empty() || args.len() > 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "approx expects an expression and an optional digit count",
            ));
        }
        let digits = if args.len() == 2 {
            let value = self.eval(args[1])?;
            match self.arena.node(value) {
                Node::Int(value) => value.to_i64().unwrap_or(0),
                _ => 0,
            }
        } else {
            16
        };
        if digits <= 0 || digits > 10_000 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalDefinition,
                "approx digit count must be between 1 and 10000",
            ));
        }
        let expression = self.eval(args[0])?;
        if let Some(value) = self.approximate(expression, digits as u32)? {
            return Ok(self.numeric_expr(value));
        }
        Ok(self.arena.intern_call(self.heads.approx, args))
    }

    pub(super) fn approximate(
        &mut self,
        expression: ExprId,
        digits: u32,
    ) -> Result<Option<Numeric>, Diagnostic> {
        match self.arena.node(expression) {
            Node::Int(value) => {
                if let Some(value) = value.to_i64() {
                    let real = Real::from_i64(value, digits).map_err(|_| {
                        Diagnostic::eval(ErrorCode::NumericFailure, "real conversion failed")
                    })?;
                    return Ok(Some(Numeric::Real(real)));
                }
                let rational = Rat::from_ints(value.clone(), Int::one()).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "rational conversion failed")
                })?;
                self.approximate_rat(rational, digits)
            }
            Node::Rat(value) => self.approximate_rat(value.clone(), digits),
            Node::Real(value) => Ok(Some(Numeric::Real(value.clone()))),
            Node::Symbol(_) => {
                if expression == self.heads.pi {
                    return Ok(Some(Numeric::Real(Real::pi(digits).map_err(|_| {
                        Diagnostic::eval(ErrorCode::NumericFailure, "pi failed")
                    })?)));
                }
                if expression == self.heads.e {
                    return Ok(Some(Numeric::Real(Real::e(digits).map_err(|_| {
                        Diagnostic::eval(ErrorCode::NumericFailure, "e failed")
                    })?)));
                }
                if expression == self.heads.imaginary {
                    let zero = Real::zero(digits)
                        .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real failed"))?;
                    let one = Real::one(digits)
                        .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real failed"))?;
                    return Ok(Some(Numeric::Complex(zero, one)));
                }
                if self.is_error(expression) {
                    return Ok(Some(Numeric::Error(expression)));
                }
                Ok(None)
            }
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                let function = self
                    .arena
                    .symbol_id(head)
                    .and_then(|symbol| NumericHead::from_name(self.arena.symbol_name(symbol)));
                match function {
                    Some(NumericHead::Plus) => {
                        let mut value = Numeric::Real(Real::zero(digits).map_err(|_| {
                            Diagnostic::eval(ErrorCode::NumericFailure, "real failed")
                        })?);
                        for arg in args {
                            let Some(argument) = self.approximate(arg, digits)? else {
                                return Ok(None);
                            };
                            let Some(next) = numeric_add(&value, &argument) else {
                                return Ok(None);
                            };
                            value = next;
                        }
                        Ok(Some(value))
                    }
                    Some(NumericHead::Times) => {
                        let mut value = Numeric::Real(Real::one(digits).map_err(|_| {
                            Diagnostic::eval(ErrorCode::NumericFailure, "real failed")
                        })?);
                        for arg in args {
                            let Some(argument) = self.approximate(arg, digits)? else {
                                return Ok(None);
                            };
                            let Some(next) = numeric_mul(&value, &argument) else {
                                return Ok(None);
                            };
                            value = next;
                        }
                        Ok(Some(value))
                    }
                    Some(NumericHead::Negate) if args.len() == 1 => {
                        let Some(argument) = self.approximate(args[0], digits)? else {
                            return Ok(None);
                        };
                        Ok(numeric_neg(&argument))
                    }
                    Some(NumericHead::Power) if args.len() == 2 => {
                        let exponent = match self.arena.node(args[1]) {
                            Node::Int(value) => value.to_i64(),
                            Node::Rat(value) if value.den().is_one() => value.num().to_i64(),
                            _ => None,
                        };
                        let Some(base) = self.approximate(args[0], digits)? else {
                            return Ok(None);
                        };
                        if let Some(exponent) = exponent {
                            return Ok(numeric_pow(&base, exponent, digits));
                        }
                        let Some(exponent) = self.approximate(args[1], digits)? else {
                            return Ok(None);
                        };
                        match (base, exponent) {
                            (Numeric::Real(base), Numeric::Real(exponent)) => {
                                match base.pow(&exponent) {
                                    Ok(value) if value.is_finite() => {
                                        Ok(Some(Numeric::Real(value)))
                                    }
                                    _ => {
                                        let Some(zero) = Real::zero(digits).ok() else {
                                            return Ok(None);
                                        };
                                        Ok(numeric_complex_pow(
                                            &(base, zero.clone()),
                                            &(exponent, zero),
                                        ))
                                    }
                                }
                            }
                            (
                                Numeric::Complex(real, imaginary),
                                Numeric::Complex(exponent_real, exponent_imaginary),
                            ) => Ok(numeric_complex_pow(
                                &(real, imaginary),
                                &(exponent_real, exponent_imaginary),
                            )),
                            (
                                Numeric::Real(real),
                                Numeric::Complex(exponent_real, exponent_imaginary),
                            ) => {
                                let Some(zero) = Real::zero(digits).ok() else {
                                    return Ok(None);
                                };
                                Ok(numeric_complex_pow(
                                    &(real, zero),
                                    &(exponent_real, exponent_imaginary),
                                ))
                            }
                            (Numeric::Complex(real, imaginary), Numeric::Real(exponent)) => {
                                let Some(zero) = Real::zero(digits).ok() else {
                                    return Ok(None);
                                };
                                Ok(numeric_complex_pow(&(real, imaginary), &(exponent, zero)))
                            }
                            (Numeric::Error(value), _) | (_, Numeric::Error(value)) => {
                                Ok(Some(Numeric::Error(value)))
                            }
                        }
                    }
                    Some(NumericHead::Complex) if args.len() == 2 => {
                        let Some(real) = self.approximate(args[0], digits)? else {
                            return Ok(None);
                        };
                        let Some(imaginary) = self.approximate(args[1], digits)? else {
                            return Ok(None);
                        };
                        match (real, imaginary) {
                            (Numeric::Real(real), Numeric::Real(imaginary)) => {
                                Ok(Some(Numeric::Complex(real, imaginary)))
                            }
                            (Numeric::Error(value), _) | (_, Numeric::Error(value)) => {
                                Ok(Some(Numeric::Error(value)))
                            }
                            _ => Ok(None),
                        }
                    }
                    Some(function) if args.len() == 1 && function.is_unary() => {
                        self.approximate_named_unary(args[0], digits, function)
                    }
                    Some(NumericHead::Abs) if args.len() == 1 => {
                        let Some(argument) = self.approximate(args[0], digits)? else {
                            return Ok(None);
                        };
                        Ok(numeric_abs(&argument))
                    }
                    _ => Ok(None),
                }
            }
        }
    }

    pub(super) fn complex_unary(
        real: &Real,
        imaginary: &Real,
        function: NumericHead,
    ) -> Option<(Real, Real)> {
        if matches!(function, NumericHead::Atanh) {
            return Self::complex_atanh(real, imaginary);
        }
        let complex = ComplexMpc::from_parts(real, imaginary)?;
        let digits =
            ((real.bits.max(imaginary.bits) as f64 * core::f64::consts::LOG10_2).ceil() as u32 + 4)
                .max(16);
        match function {
            NumericHead::Sqrt => complex.sqrt(),
            NumericHead::Exp => complex.exp(),
            NumericHead::Ln => complex.log(),
            NumericHead::Sin => complex.sin(),
            NumericHead::Cos => complex.cos(),
            NumericHead::Tan => complex.tan(),
            NumericHead::Asin => complex.asin(),
            NumericHead::Acos => complex.acos(),
            NumericHead::Atan => complex.atan(),
            NumericHead::Sinh => complex.sinh(),
            NumericHead::Cosh => complex.cosh(),
            NumericHead::Tanh => complex.tanh(),
            NumericHead::Asinh => complex.asinh(),
            NumericHead::Acosh => complex.acosh(),
            NumericHead::Atanh => None,
            NumericHead::Gamma => complex.gamma(digits).and_then(|value| value.parts()),
            NumericHead::Lgamma => complex.lgamma(digits).and_then(|value| value.parts()),
            _ => None,
        }
    }

    pub(super) fn complex_atanh(real: &Real, imaginary: &Real) -> Option<(Real, Real)> {
        let bits = real.bits.max(imaginary.bits);
        let one = Real::one_bits(bits);
        let plus_real = one.add(real).ok()?;
        let plus = ComplexMpc::from_parts(&plus_real, imaginary)?.log()?;
        let minus_real = one.sub(real).ok()?;
        let minus_imaginary = if imaginary.is_zero() {
            Real::from_i64_bits(0, bits).ok()?
        } else {
            imaginary.neg().ok()?
        };
        let minus = ComplexMpc::from_parts(&minus_real, &minus_imaginary)?.log()?;
        let real_difference = plus.0.sub(&minus.0).ok()?;
        let imaginary_difference = plus.1.sub(&minus.1).ok()?;
        let two = Real::from_i64_bits(2, bits).ok()?;
        Some((
            real_difference.div(&two).ok()?,
            imaginary_difference.div(&two).ok()?,
        ))
    }

    pub(super) fn approximate_named_unary(
        &mut self,
        argument: ExprId,
        digits: u32,
        function: NumericHead,
    ) -> Result<Option<Numeric>, Diagnostic> {
        let Some(argument) = self.approximate(argument, digits)? else {
            return Ok(None);
        };
        match argument {
            Numeric::Real(value) => {
                let result = match function {
                    NumericHead::Sqrt => value.sqrt(),
                    NumericHead::Exp => value.exp(),
                    NumericHead::Ln => value.ln(),
                    NumericHead::Sin => value.sin(),
                    NumericHead::Cos => value.cos(),
                    NumericHead::Tan => value.tan(),
                    NumericHead::Asin => value.asin(),
                    NumericHead::Acos => value.acos(),
                    NumericHead::Atan => value.atan(),
                    NumericHead::Sinh => value.sinh(),
                    NumericHead::Cosh => value.cosh(),
                    NumericHead::Tanh => value.tanh(),
                    NumericHead::Asinh => value.asinh(),
                    NumericHead::Acosh => value.acosh(),
                    NumericHead::Atanh => value.atanh(),
                    NumericHead::Floor => value.floor(),
                    NumericHead::Ceil => value.ceil(),
                    NumericHead::Round => value.round(),
                    NumericHead::Gamma => value.gamma(),
                    NumericHead::Lgamma => value.lgamma(),
                    NumericHead::Erf => value.erf(),
                    NumericHead::Erfc => value.erfc(),
                    _ => return Ok(None),
                };
                if let Ok(real) = result
                    && real.is_finite()
                {
                    return Ok(Some(Numeric::Real(real)));
                }
                let zero = if matches!(function, NumericHead::Asin | NumericHead::Acos) {
                    let Some(one) = Real::one(digits).ok() else {
                        return Ok(None);
                    };
                    if value.cmp(&one) == Ordering::Greater {
                        Real::zero(digits).ok().and_then(|zero| zero.neg().ok())
                    } else {
                        Real::zero(digits).ok()
                    }
                } else {
                    Real::zero(digits).ok()
                };
                let Some(zero) = zero else {
                    return Ok(None);
                };
                Ok(Self::complex_unary(&value, &zero, function)
                    .map(|(real, imaginary)| Numeric::Complex(real, imaginary)))
            }
            Numeric::Error(value) => Ok(Some(Numeric::Error(value))),
            Numeric::Complex(real, imaginary) => {
                Ok(Self::complex_unary(&real, &imaginary, function)
                    .map(|(real, imaginary)| Numeric::Complex(real, imaginary)))
            }
        }
    }

    pub(super) fn approximate_rat(
        &mut self,
        value: Rat,
        digits: u32,
    ) -> Result<Option<Numeric>, Diagnostic> {
        let real = Real::from_rat(&value, digits)
            .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real conversion failed"))?;
        Ok(Some(Numeric::Real(real)))
    }

    pub(super) fn numeric_expr(&mut self, value: Numeric) -> ExprId {
        match value {
            Numeric::Real(value) => {
                if value.is_finite() {
                    self.arena.intern_real(value)
                } else {
                    self.heads.undefined
                }
            }
            Numeric::Error(value) => value,
            Numeric::Complex(real, imaginary) => {
                if !real.is_finite() || !imaginary.is_finite() {
                    return self.heads.undefined;
                }
                if imaginary.is_zero() {
                    self.arena.intern_real(real)
                } else {
                    let real = self.arena.intern_real(real);
                    let imaginary = self.arena.intern_real(imaginary);
                    self.arena
                        .intern_call(self.heads.complex, &[real, imaginary])
                }
            }
        }
    }
}
