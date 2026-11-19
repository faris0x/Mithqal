//! Canonical arithmetic, comparisons, and Gaussian rational collection.

#[allow(unused_imports)]
use crate::prelude::*;

use super::*;

impl<'a> Evaluator<'a> {
    pub(super) fn plus(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.iter().any(|arg| self.interval_bounds(*arg).is_some()) {
            return self.plus_intervals(args);
        }
        if let Some(value) = self.error_value(args) {
            return Ok(value);
        }
        let mut has_real = false;
        let mut all_numeric = true;
        for arg in args {
            match self.arena.node(*arg) {
                Node::Real(_) => has_real = true,
                Node::Int(_) | Node::Rat(_) => {}
                _ => all_numeric = false,
            }
        }
        if has_real && all_numeric {
            let mut value = Real::zero(30)
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real addition failed"))?;
            for arg in args {
                let Some(argument) = self.real_endpoint(*arg) else {
                    return Ok(self.arena.intern_call(self.heads.plus, args));
                };
                value = value.add(&argument).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "real addition failed")
                })?;
            }
            return Ok(self.arena.intern_real(value));
        }
        let mut constant = Rat::zero();
        let mut terms: HashMap<ExprId, Rat, FnvBuildHasher> = HashMap::default();
        for arg in args {
            let (coefficient, body) = self.decompose_term(*arg)?;
            match body {
                None => constant = constant.add(&coefficient),
                Some(body) => match terms.get_mut(&body) {
                    Some(value) => *value = value.add(&coefficient),
                    None => {
                        terms.insert(body, coefficient);
                    }
                },
            }
        }

        let mut out = Vec::new();
        if !constant.is_zero() {
            out.push(self.arena.intern_rat(constant));
        }
        for (body, coefficient) in terms {
            if coefficient.is_zero() {
                continue;
            }
            out.push(self.term(body, coefficient));
        }
        Ok(match out.len() {
            0 => self.arena.intern_int(Int::zero()),
            1 => out[0],
            _ => self.arena.intern_call(self.heads.plus, &out),
        })
    }

    pub(super) fn times(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.iter().any(|arg| self.interval_bounds(*arg).is_some()) {
            return self.times_intervals(args);
        }
        let mut has_zero = false;
        let mut undefined = false;
        let mut failed = false;
        let mut has_real = false;
        let mut all_numeric = true;
        for arg in args {
            if self.as_rat(*arg).is_some_and(|value| value.is_zero()) {
                has_zero = true;
            }
            if *arg == self.heads.indeterminate {
                return Ok(self.heads.indeterminate);
            }
            if *arg == self.heads.undefined {
                undefined = true;
            } else if *arg == self.heads.failed {
                failed = true;
            }
            match self.arena.node(*arg) {
                Node::Real(_) => has_real = true,
                Node::Int(_) | Node::Rat(_) => {}
                _ => all_numeric = false,
            }
        }
        if undefined {
            return if has_zero {
                Ok(self.heads.indeterminate)
            } else {
                Ok(self.heads.undefined)
            };
        }
        if failed {
            return Ok(self.heads.failed);
        }
        if has_real && all_numeric {
            let mut value = Real::one(30)
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real product failed"))?;
            for arg in args {
                let Some(argument) = self.real_endpoint(*arg) else {
                    return Ok(self.arena.intern_call(self.heads.times, args));
                };
                value = value.mul(&argument).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "real product failed")
                })?;
            }
            return Ok(self.arena.intern_real(value));
        }

        let mut coefficient = Rat::one();
        let mut bases: HashMap<ExprId, Rat, FnvBuildHasher> = HashMap::default();
        for arg in args {
            self.collect_factor(*arg, &mut coefficient, &mut bases)?;
        }
        if coefficient.is_zero() {
            return Ok(self.arena.intern_int(Int::zero()));
        }

        let mut factors = Vec::new();
        for (base, exponent) in bases {
            if exponent.is_zero() {
                continue;
            }
            if exponent.is_one() {
                factors.push(base);
            } else {
                let exponent_node = self.arena.intern_rat(exponent);
                factors.push(
                    self.arena
                        .intern_call(self.heads.power, &[base, exponent_node]),
                );
            }
        }

        let negative = coefficient.sign() < 0;
        let magnitude = coefficient.abs();
        let mut out = Vec::new();
        if !magnitude.is_one() || factors.is_empty() {
            out.push(self.arena.intern_rat(magnitude));
        }
        out.extend(factors);
        let product = match out.len() {
            0 => self.arena.intern_int(Int::one()),
            1 => out[0],
            _ => self.arena.intern_call(self.heads.times, &out),
        };
        if negative {
            Ok(self.arena.intern_call(self.heads.negate, &[product]))
        } else {
            Ok(product)
        }
    }

    pub(super) fn power(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "power expects two arguments",
            ));
        }
        let base = args[0];
        let exponent = args[1];
        if self.is_error(base) {
            return Ok(base);
        }
        if self.is_error(exponent) {
            return Ok(exponent);
        }
        if let Node::Real(real) = self.arena.node(base)
            && let Some(power) = self.integer_exponent(exponent)
        {
            let value = if power == 0 {
                Real::one(30)
            } else {
                real.pow_si(power)
            };
            if let Ok(value) = value {
                return Ok(self.arena.intern_real(value));
            }
        }
        if let (Some((low, high)), Some(power)) =
            (self.interval_bounds(base), self.integer_exponent(exponent))
            && let Some(result) = self.interval_power(&low, &high, power)
        {
            return Ok(result);
        }
        if let Some(exp) = self.as_rat(exponent) {
            return self.power_from_rational(base, exponent, exp);
        }
        if self.as_rat(base).is_some_and(|value| value.is_one()) {
            return Ok(self.arena.intern_int(Int::one()));
        }
        Ok(self.arena.intern_call(self.heads.power, &[base, exponent]))
    }

    pub(super) fn power_from_rational(
        &mut self,
        base: ExprId,
        exponent: ExprId,
        exp: Rat,
    ) -> Result<ExprId, Diagnostic> {
        if exp.is_zero() {
            return if self.as_rat(base).is_some_and(|value| value.is_zero()) {
                Ok(self.heads.indeterminate)
            } else {
                Ok(self.arena.intern_int(Int::one()))
            };
        }
        if exp.is_one() {
            return Ok(base);
        }
        if !exp.den().is_one() {
            if self.as_rat(base).is_some_and(|value| value.is_one()) {
                return Ok(self.arena.intern_int(Int::one()));
            }
            if self.as_rat(base).is_some_and(|value| value.is_zero()) {
                return if exp.sign() < 0 {
                    Ok(self.heads.undefined)
                } else {
                    Ok(self.arena.intern_int(Int::zero()))
                };
            }
            return Ok(self.arena.intern_call(self.heads.power, &[base, exponent]));
        }
        let Some(power) = exp.num().to_i64() else {
            return Ok(self.arena.intern_call(self.heads.power, &[base, exponent]));
        };
        if let Some(base_value) = self.as_rat(base) {
            if base_value.is_zero() {
                return if power < 0 {
                    Ok(self.heads.undefined)
                } else {
                    Ok(self.arena.intern_int(Int::zero()))
                };
            }
            let value = rat_pow(&base_value, power)
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "numeric power failed"))?;
            return Ok(self.arena.intern_rat(value));
        }
        if base == self.heads.imaginary {
            return Ok(self.imaginary_power(power));
        }
        if let Some((real, imaginary)) = self.gaussian(base) {
            if real.is_zero() && imaginary.is_zero() {
                return if power < 0 {
                    Ok(self.heads.undefined)
                } else {
                    Ok(self.arena.intern_int(Int::zero()))
                };
            }
            return match gaussian_pow(&real, &imaginary, power) {
                Some((real, imaginary)) => Ok(self.gaussian_node(&real, &imaginary)),
                None => Ok(self.heads.undefined),
            };
        }
        if let Some((inner_base, inner_exponent)) = self.nested_power(base)
            && let Some(inner) = self.as_rat(inner_exponent)
        {
            let combined = inner.mul(&exp);
            let combined_node = self.arena.intern_rat(combined);
            return Ok(self
                .arena
                .intern_call(self.heads.power, &[inner_base, combined_node]));
        }
        if let Some(inner_base) = self.negated_operand(base) {
            let inner = self
                .arena
                .intern_call(self.heads.power, &[inner_base, exponent]);
            let value = self.eval(inner)?;
            return if power % 2 == 0 {
                Ok(value)
            } else {
                Ok(self.arena.intern_call(self.heads.negate, &[value]))
            };
        }
        Ok(self.arena.intern_call(self.heads.power, &[base, exponent]))
    }

    pub(super) fn integer_exponent(&self, expression: ExprId) -> Option<i64> {
        let value = self.as_rat(expression)?;
        if !value.den().is_one() {
            return None;
        }
        value.num().to_i64()
    }

    pub(super) fn nested_power(&self, expression: ExprId) -> Option<(ExprId, ExprId)> {
        if let Node::Call { head, args } = self.arena.node(expression)
            && *head == self.heads.power
            && args.len() == 2
        {
            return Some((args[0], args[1]));
        }
        None
    }

    pub(super) fn negated_operand(&self, expression: ExprId) -> Option<ExprId> {
        if let Node::Call { head, args } = self.arena.node(expression)
            && *head == self.heads.negate
            && args.len() == 1
        {
            return Some(args[0]);
        }
        None
    }

    pub(super) fn negate(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 1 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "negate expects one argument",
            ));
        }
        let value = args[0];
        if self.is_error(value) {
            return Ok(value);
        }
        if let Node::Real(real) = self.arena.node(value) {
            let negated = real
                .neg()
                .map_err(|_| Diagnostic::eval(ErrorCode::NumericFailure, "real negation failed"))?;
            return Ok(self.arena.intern_real(negated));
        }
        if let Some((low, high)) = self.interval_bounds(value) {
            let negated_low = high.neg().map_err(|_| {
                Diagnostic::eval(ErrorCode::NumericFailure, "interval negation failed")
            })?;
            let negated_high = low.neg().map_err(|_| {
                Diagnostic::eval(ErrorCode::NumericFailure, "interval negation failed")
            })?;
            return Ok(self.interval_node(negated_low, negated_high));
        }
        if let Some(number) = self.as_rat(value) {
            return Ok(self.arena.intern_rat(number.neg()));
        }
        let call = match self.arena.node(value) {
            Node::Call { head, args } => Some((*head, args.to_vec())),
            _ => None,
        };
        if let Some((head, args)) = call {
            if head == self.heads.negate && args.len() == 1 {
                return Ok(args[0]);
            }
            if head == self.heads.plus {
                let mut terms = Vec::with_capacity(args.len());
                for arg in args {
                    terms.push(self.arena.intern_call(self.heads.negate, &[arg]));
                }
                return Ok(self.arena.intern_call(self.heads.plus, &terms));
            }
        }
        Ok(self.arena.intern_call(self.heads.negate, &[value]))
    }

    pub(super) fn same(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "same expects two arguments",
            ));
        }
        Ok(if args[0] == args[1] {
            self.heads.true_value
        } else {
            self.heads.false_value
        })
    }

    pub(super) fn equal(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "equal expects two arguments",
            ));
        }
        if args[0] == args[1] {
            return Ok(self.heads.true_value);
        }
        if let (Some(left), Some(right)) = (self.as_rat(args[0]), self.as_rat(args[1])) {
            return Ok(if left == right {
                self.heads.true_value
            } else {
                self.heads.false_value
            });
        }
        Ok(self.arena.intern_call(self.heads.equal, args))
    }

    pub(super) fn not_equal(&mut self, args: &[ExprId]) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "notEqual expects two arguments",
            ));
        }
        let equal = self.equal(args)?;
        if equal == self.heads.true_value {
            return Ok(self.heads.false_value);
        }
        if equal == self.heads.false_value {
            return Ok(self.heads.true_value);
        }
        Ok(self.arena.intern_call(self.heads.not_equal, args))
    }

    pub(super) fn ordering(
        &mut self,
        args: &[ExprId],
        wanted: Ordering,
        head: ExprId,
    ) -> Result<ExprId, Diagnostic> {
        if args.len() != 2 {
            return Err(Diagnostic::eval(
                ErrorCode::EvalArity,
                "comparison expects two arguments",
            ));
        }
        if self.is_error(args[0]) {
            return Ok(args[0]);
        }
        if self.is_error(args[1]) {
            return Ok(args[1]);
        }
        if let (Some(left), Some(right)) = (self.as_rat(args[0]), self.as_rat(args[1])) {
            let order = left.cmp(&right);
            let result = match wanted {
                Ordering::Less => {
                    order == Ordering::Less
                        || (head == self.heads.less_equal && order == Ordering::Equal)
                }
                Ordering::Greater => {
                    order == Ordering::Greater
                        || (head == self.heads.greater_equal && order == Ordering::Equal)
                }
                Ordering::Equal => order == Ordering::Equal,
            };
            return Ok(if result {
                self.heads.true_value
            } else {
                self.heads.false_value
            });
        }
        Ok(self.arena.intern_call(head, args))
    }

    pub(super) fn decompose_term(
        &mut self,
        id: ExprId,
    ) -> Result<(Rat, Option<ExprId>), Diagnostic> {
        match self.arena.node(id) {
            Node::Int(value) => Ok((
                Rat::from_ints(value.clone(), Int::one()).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "rational conversion failed")
                })?,
                None,
            )),
            Node::Rat(value) => Ok((value.clone(), None)),
            Node::Real(_) => Ok((Rat::one(), Some(id))),
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                if head == self.heads.negate && args.len() == 1 {
                    let (coefficient, body) = self.decompose_term(args[0])?;
                    return Ok((coefficient.neg(), body));
                }
                if head == self.heads.times {
                    let mut coefficient = Rat::one();
                    let mut body = Vec::new();
                    for arg in args {
                        let (part, part_body) = self.decompose_term(arg)?;
                        coefficient = coefficient.mul(&part);
                        if let Some(part_body) = part_body {
                            body.push(part_body);
                        }
                    }
                    let body = match body.len() {
                        0 => None,
                        1 => Some(body[0]),
                        _ => Some(self.arena.intern_call(self.heads.times, &body)),
                    };
                    return Ok((coefficient, body));
                }
                Ok((Rat::one(), Some(id)))
            }
            Node::Symbol(_) => Ok((Rat::one(), Some(id))),
        }
    }

    pub(super) fn collect_factor(
        &mut self,
        id: ExprId,
        coefficient: &mut Rat,
        bases: &mut HashMap<ExprId, Rat, FnvBuildHasher>,
    ) -> Result<(), Diagnostic> {
        match self.arena.node(id) {
            Node::Int(value) => {
                let value = Rat::from_ints(value.clone(), Int::one()).map_err(|_| {
                    Diagnostic::eval(ErrorCode::NumericFailure, "rational conversion failed")
                })?;
                *coefficient = coefficient.mul(&value);
            }
            Node::Rat(value) => {
                *coefficient = coefficient.mul(value);
            }
            Node::Real(_) => self.add_base(bases, id, Rat::one()),
            Node::Symbol(_) => self.add_base(bases, id, Rat::one()),
            Node::Call { head, args } => {
                let head = *head;
                let args = args.to_vec();
                if head == self.heads.negate && args.len() == 1 {
                    *coefficient = coefficient.neg();
                    return self.collect_factor(args[0], coefficient, bases);
                }
                if head == self.heads.times {
                    for arg in args {
                        self.collect_factor(arg, coefficient, bases)?;
                    }
                    return Ok(());
                }
                if head == self.heads.power
                    && args.len() == 2
                    && let Some(exponent) = self.as_rat(args[1])
                {
                    if !exponent.is_zero() {
                        self.add_base(bases, args[0], exponent);
                    }
                    return Ok(());
                }
                self.add_base(bases, id, Rat::one());
            }
        }
        Ok(())
    }

    pub(super) fn add_base(
        &self,
        bases: &mut HashMap<ExprId, Rat, FnvBuildHasher>,
        base: ExprId,
        exponent: Rat,
    ) {
        match bases.get_mut(&base) {
            Some(value) => *value = value.add(&exponent),
            None => {
                bases.insert(base, exponent);
            }
        }
    }

    pub(super) fn term(&mut self, body: ExprId, coefficient: Rat) -> ExprId {
        if coefficient.is_one() {
            return body;
        }
        if coefficient.sign() < 0 {
            let magnitude = coefficient.abs();
            if magnitude.is_one() {
                return self.arena.intern_call(self.heads.negate, &[body]);
            }
            let coefficient_node = self.arena.intern_rat(magnitude);
            let product = self
                .arena
                .intern_call(self.heads.times, &[coefficient_node, body]);
            return self.arena.intern_call(self.heads.negate, &[product]);
        }
        let coefficient_node = self.arena.intern_rat(coefficient);
        self.arena
            .intern_call(self.heads.times, &[coefficient_node, body])
    }

    pub(super) fn imaginary_power(&mut self, power: i64) -> ExprId {
        let remainder = power.rem_euclid(4);
        match remainder {
            0 => self.arena.intern_int(Int::one()),
            1 => self.heads.imaginary,
            2 => self.arena.intern_int(Int::from_i64(-1)),
            _ => self
                .arena
                .intern_call(self.heads.negate, &[self.heads.imaginary]),
        }
    }

    pub(super) fn gaussian(&self, id: ExprId) -> Option<(Rat, Rat)> {
        match self.arena.node(id) {
            Node::Int(value) => {
                let real = Rat::from_ints(value.clone(), Int::one()).ok()?;
                Some((real, Rat::zero()))
            }
            Node::Rat(value) => Some((value.clone(), Rat::zero())),
            Node::Real(_) => None,
            Node::Symbol(_) => {
                if id == self.heads.imaginary {
                    Some((Rat::zero(), Rat::one()))
                } else {
                    None
                }
            }
            Node::Call { head, args } => {
                let head = *head;
                if head == self.heads.plus {
                    let mut real = Rat::zero();
                    let mut imaginary = Rat::zero();
                    for arg in args.iter() {
                        let (arg_real, arg_imaginary) = self.gaussian(*arg)?;
                        real = real.add(&arg_real);
                        imaginary = imaginary.add(&arg_imaginary);
                    }
                    Some((real, imaginary))
                } else if head == self.heads.times {
                    let mut real = Rat::one();
                    let mut imaginary = Rat::zero();
                    for arg in args.iter() {
                        let (arg_real, arg_imaginary) = self.gaussian(*arg)?;
                        let next_real = real.mul(&arg_real).sub(&imaginary.mul(&arg_imaginary));
                        let next_imaginary =
                            real.mul(&arg_imaginary).add(&imaginary.mul(&arg_real));
                        real = next_real;
                        imaginary = next_imaginary;
                    }
                    Some((real, imaginary))
                } else if head == self.heads.negate && args.len() == 1 {
                    let (real, imaginary) = self.gaussian(args[0])?;
                    Some((real.neg(), imaginary.neg()))
                } else if head == self.heads.power && args.len() == 2 {
                    let exponent = match self.arena.node(args[1]) {
                        Node::Int(value) => value.to_i64()?,
                        Node::Rat(value) if value.den().is_one() => value.num().to_i64()?,
                        _ => return None,
                    };
                    let (real, imaginary) = self.gaussian(args[0])?;
                    gaussian_pow(&real, &imaginary, exponent)
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn gaussian_node(&mut self, real: &Rat, imaginary: &Rat) -> ExprId {
        if imaginary.is_zero() {
            return self.arena.intern_rat(real.clone());
        }
        let imaginary_part = self.term(self.heads.imaginary, imaginary.clone());
        if real.is_zero() {
            imaginary_part
        } else {
            let real_part = self.arena.intern_rat(real.clone());
            self.arena
                .intern_call(self.heads.plus, &[real_part, imaginary_part])
        }
    }

    pub(super) fn as_rat(&self, id: ExprId) -> Option<Rat> {
        match self.arena.node(id) {
            Node::Int(value) => Rat::from_ints(value.clone(), Int::one()).ok(),
            Node::Rat(value) => Some(value.clone()),
            Node::Real(_) => None,
            _ => None,
        }
    }

    pub(super) fn is_error(&self, id: ExprId) -> bool {
        id == self.heads.undefined || id == self.heads.indeterminate || id == self.heads.failed
    }

    pub(super) fn error_value(&self, args: &[ExprId]) -> Option<ExprId> {
        let mut undefined = false;
        let mut failed = false;
        for arg in args {
            if *arg == self.heads.indeterminate {
                return Some(self.heads.indeterminate);
            }
            if *arg == self.heads.undefined {
                undefined = true;
            } else if *arg == self.heads.failed {
                failed = true;
            }
        }
        if undefined {
            Some(self.heads.undefined)
        } else if failed {
            Some(self.heads.failed)
        } else {
            None
        }
    }
}
