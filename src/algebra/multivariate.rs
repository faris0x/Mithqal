//! Multivariate factorisation scaffold.
//!
//! The current pipeline extracts content and monomial factors before it reaches
//! this module. This module owns the multivariate stages that follow.
//!
//! Implemented stages:
//!
//! - Homogeneous binary quadratic forms with rational roots.
//! - Recursive coefficient factorisation for bivariate quadratics.
//! - Even-term grouping.
//!
//! Reserved stages for the full algorithm:
//!
//! - Multivariate square-free decomposition.
//! - Evaluation and interpolation over several variables.
//! - Hensel lifting for factors that are irreducible over the rationals.

use super::*;

/// Resource limits for a multivariate attempt.
#[derive(Copy, Clone, Debug)]
pub(super) struct FactorLimits {
    pub(super) max_variables: usize,
    pub(super) max_terms: usize,
    pub(super) max_total_degree: u32,
}

impl Default for FactorLimits {
    fn default() -> Self {
        FactorLimits {
            max_variables: 8,
            max_terms: MAX_TERMS,
            max_total_degree: 32,
        }
    }
}

/// The stages that are currently available to the multivariate driver.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(super) enum FactorStage {
    BinaryQuadratic,
    HomogeneousBivariate,
    RecursiveQuadratic,
    Grouping,
    SquareFree,
    EvaluationInterpolation,
    HenselLifting,
}

/// A staged multivariate factoriser.
pub(super) struct MultivariateFactorizer<'a> {
    arena: &'a mut Arena,
    context: Context,
    limits: FactorLimits,
}

impl<'a> MultivariateFactorizer<'a> {
    pub(super) fn new(arena: &'a mut Arena, context: &Context) -> Self {
        MultivariateFactorizer {
            arena,
            context: *context,
            limits: FactorLimits::default(),
        }
    }

    /// Run the implemented stages in order and return expression factors.
    pub(super) fn factor(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        if polynomial.terms.len() > self.limits.max_terms {
            return None;
        }
        if polynomial.variables().len() > self.limits.max_variables {
            return None;
        }
        if total_degree(polynomial) > self.limits.max_total_degree {
            return None;
        }
        for stage in [
            FactorStage::BinaryQuadratic,
            FactorStage::HomogeneousBivariate,
            FactorStage::RecursiveQuadratic,
            FactorStage::Grouping,
            FactorStage::SquareFree,
            FactorStage::EvaluationInterpolation,
            FactorStage::HenselLifting,
        ] {
            let factors = match stage {
                FactorStage::BinaryQuadratic => self.factor_binary_quadratic(polynomial),
                FactorStage::HomogeneousBivariate => self.factor_homogeneous_bivariate(polynomial),
                FactorStage::RecursiveQuadratic => self.factor_recursive_quadratic(polynomial),
                FactorStage::Grouping => self.factor_by_grouping(polynomial),
                FactorStage::SquareFree => self.factor_square_free_stage(polynomial),
                FactorStage::EvaluationInterpolation => {
                    self.factor_evaluation_interpolation_stage(polynomial)
                }
                FactorStage::HenselLifting => self.factor_hensel_stage(polynomial),
            };
            if let Some(factors) = factors {
                return Some(factors);
            }
        }
        None
    }

    fn factor_by_grouping(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let (left, right) = super::factor_by_grouping(polynomial)?;
        Some(vec![
            expression_from_poly(self.arena, &left, &self.context),
            expression_from_poly(self.arena, &right, &self.context),
        ])
    }

    fn factor_binary_quadratic(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let quadratic = BivariateQuadratic::from_poly(polynomial)?;
        if !quadratic.is_homogeneous() {
            return None;
        }
        let (scalar, homogeneous) = quadratic.homogeneous_factors()?;
        let mut factors = Vec::new();
        if !scalar.is_one() {
            factors.push(expression_from_rat(self.arena, &scalar));
        }
        for (factor, multiplicity) in homogeneous {
            push_factor(
                self.arena,
                &self.context,
                &mut factors,
                factor,
                multiplicity,
            );
        }
        Some(factors)
    }

    fn factor_homogeneous_bivariate(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let (horizontal, vertical, degree, coefficients) = homogeneous_bivariate_data(polynomial)?;
        if degree < 3 {
            return None;
        }
        let (roots, remaining) = rational_roots(&coefficients);
        if roots.is_empty() {
            return None;
        }
        let mut factors = Vec::new();
        for (root, multiplicity) in roots {
            let denominator = Rat::from_ints(root.den(), Int::one()).ok()?;
            let linear = Poly::variable(horizontal).scale(&denominator).add(
                &Poly::variable(vertical)
                    .scale(&Rat::from_ints(root.num().neg(), Int::one()).ok()?),
            );
            push_factor(
                self.arena,
                &self.context,
                &mut factors,
                linear,
                multiplicity,
            );
        }
        if remaining.len() == 1 {
            if !remaining[0].is_one() {
                factors.push(expression_from_rat(self.arena, &remaining[0]));
            }
        } else if remaining.len() > 1 {
            let remaining_degree = remaining.len() as u32 - 1;
            let mut residual = Poly::zero();
            for (power, coefficient) in remaining.iter().enumerate() {
                if coefficient.is_zero() {
                    continue;
                }
                let mut powers = Vec::new();
                if power > 0 {
                    powers.push((horizontal, power as u32));
                }
                let vertical_power = remaining_degree.saturating_sub(power as u32);
                if vertical_power > 0 {
                    powers.push((vertical, vertical_power));
                }
                powers.sort_by_key(|(symbol, _)| *symbol);
                residual
                    .terms
                    .insert(Monomial { powers }, coefficient.clone());
            }
            push_factor(self.arena, &self.context, &mut factors, residual, 1);
        }
        Some(factors)
    }

    fn factor_recursive_quadratic(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let quadratic = BivariateQuadratic::from_poly(polynomial)?;
        if quadratic.is_homogeneous() {
            return None;
        }
        let (scalar, homogeneous) = quadratic.homogeneous_factors()?;
        let (first, second) = two_linear_factors(homogeneous)?;
        let (first_horizontal, first_vertical) =
            linear_coefficients(&first, quadratic.horizontal, quadratic.vertical)?;
        let (second_horizontal, second_vertical) =
            linear_coefficients(&second, quadratic.horizontal, quadratic.vertical)?;
        let determinant = first_horizontal
            .mul(&second_vertical)
            .sub(&second_horizontal.mul(&first_vertical));
        if !determinant.is_zero() {
            let first_constant = quadratic
                .linear_horizontal
                .mul(&second_vertical)
                .sub(&quadratic.linear_vertical.mul(&second_horizontal))
                .div(&determinant)
                .ok()?;
            let second_constant = first_horizontal
                .mul(&quadratic.linear_vertical)
                .sub(&first_vertical.mul(&quadratic.linear_horizontal))
                .div(&determinant)
                .ok()?;
            if first_constant.mul(&second_constant) != quadratic.constant.mul(&scalar) {
                return None;
            }
            let mut factors = Vec::new();
            let first_factor = first.add(&Poly::constant(first_constant));
            let second_factor = second.scale(&scalar).add(&Poly::constant(second_constant));
            push_factor(self.arena, &self.context, &mut factors, first_factor, 1);
            push_factor(self.arena, &self.context, &mut factors, second_factor, 1);
            return Some(factors);
        }

        if first != second {
            return None;
        }
        let (horizontal, vertical) =
            linear_coefficients(&first, quadratic.horizontal, quadratic.vertical)?;
        let slope = if !horizontal.is_zero() {
            let slope = quadratic.linear_horizontal.div(&horizontal).ok()?;
            if quadratic.linear_vertical != slope.mul(&vertical) {
                return None;
            }
            slope
        } else if !vertical.is_zero() {
            let slope = quadratic.linear_vertical.div(&vertical).ok()?;
            if quadratic.linear_horizontal != slope.mul(&horizontal) {
                return None;
            }
            slope
        } else {
            return None;
        };
        let roots = rational_roots(&[quadratic.constant.clone(), slope, scalar.clone()]);
        if roots.1.len() > 1 || roots.0.is_empty() {
            return None;
        }
        let mut factors = Vec::new();
        if !scalar.is_one() {
            factors.push(expression_from_rat(self.arena, &scalar));
        }
        for (root, multiplicity) in roots.0 {
            let factor = first.add(&Poly::constant(root.neg()));
            push_factor(
                self.arena,
                &self.context,
                &mut factors,
                factor,
                multiplicity,
            );
        }
        Some(factors)
    }

    fn factor_square_free_stage(&mut self, _polynomial: &Poly) -> Option<Vec<ExprId>> {
        // Reserved for multivariate square-free decomposition.
        None
    }

    fn factor_evaluation_interpolation_stage(&mut self, _polynomial: &Poly) -> Option<Vec<ExprId>> {
        // Reserved for evaluation and interpolation over several variables.
        None
    }

    fn factor_hensel_stage(&mut self, _polynomial: &Poly) -> Option<Vec<ExprId>> {
        // Reserved for Hensel lifting over the rationals.
        None
    }
}

fn homogeneous_bivariate_data(polynomial: &Poly) -> Option<(SymbolId, SymbolId, u32, Vec<Rat>)> {
    let variables = polynomial.variables();
    if variables.len() != 2 {
        return None;
    }
    let mut symbols = variables.into_iter();
    let horizontal = symbols.next()?;
    let vertical = symbols.next()?;
    let mut degree = None;
    for monomial in polynomial.terms.keys() {
        let horizontal_power = monomial.degree_in(horizontal);
        let vertical_power = monomial.degree_in(vertical);
        let total = horizontal_power + vertical_power;
        match degree {
            None => degree = Some(total),
            Some(existing) if existing == total => {}
            Some(_) => return None,
        }
    }
    let degree = degree?;
    if degree == 0 {
        return None;
    }
    let mut coefficients = vec![Rat::zero(); degree as usize + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let power = monomial.degree_in(horizontal) as usize;
        coefficients[power] = coefficient.clone();
    }
    Some((horizontal, vertical, degree, trim(coefficients)))
}

struct BivariateQuadratic {
    horizontal: SymbolId,
    vertical: SymbolId,
    square_horizontal: Rat,
    cross: Rat,
    square_vertical: Rat,
    linear_horizontal: Rat,
    linear_vertical: Rat,
    constant: Rat,
}

impl BivariateQuadratic {
    fn from_poly(polynomial: &Poly) -> Option<Self> {
        let variables = polynomial.variables();
        if variables.len() != 2 {
            return None;
        }
        let mut symbols = variables.into_iter();
        let horizontal = symbols.next()?;
        let vertical = symbols.next()?;
        let mut quadratic = BivariateQuadratic {
            horizontal,
            vertical,
            square_horizontal: Rat::zero(),
            cross: Rat::zero(),
            square_vertical: Rat::zero(),
            linear_horizontal: Rat::zero(),
            linear_vertical: Rat::zero(),
            constant: Rat::zero(),
        };
        for (monomial, coefficient) in &polynomial.terms {
            match (monomial.degree_in(horizontal), monomial.degree_in(vertical)) {
                (2, 0) => quadratic.square_horizontal = coefficient.clone(),
                (1, 1) => quadratic.cross = coefficient.clone(),
                (0, 2) => quadratic.square_vertical = coefficient.clone(),
                (1, 0) => quadratic.linear_horizontal = coefficient.clone(),
                (0, 1) => quadratic.linear_vertical = coefficient.clone(),
                (0, 0) => quadratic.constant = coefficient.clone(),
                _ => return None,
            }
        }
        Some(quadratic)
    }

    fn is_homogeneous(&self) -> bool {
        self.linear_horizontal.is_zero()
            && self.linear_vertical.is_zero()
            && self.constant.is_zero()
    }

    fn homogeneous_factors(&self) -> Option<(Rat, Vec<(Poly, u32)>)> {
        if self.square_horizontal.is_zero() {
            return self.degenerate_homogeneous_factors();
        }
        let roots = rational_roots(&[
            self.square_vertical.clone(),
            self.cross.clone(),
            self.square_horizontal.clone(),
        ]);
        if roots.1.len() > 1 || roots.0.is_empty() {
            return None;
        }
        let mut scalar = self.square_horizontal.clone();
        let mut factors = Vec::new();
        for (root, multiplicity) in roots.0 {
            let denominator = Rat::from_ints(root.den(), Int::one()).ok()?;
            for _ in 0..multiplicity {
                scalar = scalar.div(&denominator).ok()?;
            }
            let linear = Poly::variable(self.horizontal).scale(&denominator).add(
                &Poly::variable(self.vertical)
                    .scale(&Rat::from_ints(root.num().neg(), Int::one()).ok()?),
            );
            factors.push((linear, multiplicity));
        }
        Some((scalar, factors))
    }

    fn degenerate_homogeneous_factors(&self) -> Option<(Rat, Vec<(Poly, u32)>)> {
        if self.cross.is_zero() {
            return None;
        }
        let mut factors = Vec::new();
        if !self.square_vertical.is_zero() {
            factors.push((Poly::variable(self.vertical), 1));
        }
        let mut linear = Poly::variable(self.horizontal).scale(&self.cross);
        if !self.square_vertical.is_zero() {
            linear = linear.add(&Poly::variable(self.vertical).scale(&self.square_vertical));
        }
        factors.push((linear, 1));
        Some((Rat::one(), factors))
    }
}

fn two_linear_factors(factors: Vec<(Poly, u32)>) -> Option<(Poly, Poly)> {
    match factors.as_slice() {
        [(first, 1), (second, 1)] => Some((first.clone(), second.clone())),
        [(factor, 2)] => Some((factor.clone(), factor.clone())),
        _ => None,
    }
}

fn linear_coefficients(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<(Rat, Rat)> {
    let mut horizontal_coefficient = Rat::zero();
    let mut vertical_coefficient = Rat::zero();
    for (monomial, coefficient) in &polynomial.terms {
        match (monomial.degree_in(horizontal), monomial.degree_in(vertical)) {
            (1, 0) => horizontal_coefficient = coefficient.clone(),
            (0, 1) => vertical_coefficient = coefficient.clone(),
            (0, 0) => {}
            _ => return None,
        }
    }
    Some((horizontal_coefficient, vertical_coefficient))
}

fn push_factor(
    arena: &mut Arena,
    context: &Context,
    factors: &mut Vec<ExprId>,
    polynomial: Poly,
    multiplicity: u32,
) {
    let expression = expression_from_poly(arena, &polynomial, context);
    if multiplicity <= 1 {
        factors.push(expression);
        return;
    }
    let exponent = arena.intern_int(Int::from_u64(multiplicity as u64));
    factors.push(arena.intern_call(context.power, &[expression, exponent]));
}

fn total_degree(polynomial: &Poly) -> u32 {
    polynomial
        .terms
        .keys()
        .map(|monomial| {
            monomial
                .powers
                .iter()
                .map(|(_, exponent)| exponent)
                .sum::<u32>()
        })
        .max()
        .unwrap_or(0)
}

/// Entry point used by the main factorization function.
pub(super) fn factor_stage(
    arena: &mut Arena,
    polynomial: &Poly,
    context: &Context,
) -> Option<Vec<ExprId>> {
    MultivariateFactorizer::new(arena, context).factor(polynomial)
}
