//! Multivariate factorisation scaffold.
//!
//! The current pipeline extracts content and monomial factors before it reaches
//! this module. This module owns the multivariate stages that follow.
//!
//! Implemented stages:
//!
//! - Homogeneous binary quadratic forms with rational roots.
//! - Even-term grouping.
//!
//! Reserved stages for the full algorithm:
//!
//! - Recursive factorisation of polynomial coefficients in one variable.
//! - Square-free decomposition for repeated multivariate factors.
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
    Grouping,
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
        for stage in [FactorStage::BinaryQuadratic, FactorStage::Grouping] {
            let factors = match stage {
                FactorStage::BinaryQuadratic => self.factor_binary_quadratic(polynomial),
                FactorStage::Grouping => self.factor_by_grouping(polynomial),
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
        let variables = polynomial.variables();
        if variables.len() != 2 {
            return None;
        }
        let mut symbols = variables.into_iter();
        let horizontal = symbols.next()?;
        let vertical = symbols.next()?;

        let mut square_horizontal = Rat::zero();
        let mut cross = Rat::zero();
        let mut square_vertical = Rat::zero();
        let mut linear_horizontal = Rat::zero();
        let mut linear_vertical = Rat::zero();
        let mut constant = Rat::zero();
        for (monomial, coefficient) in &polynomial.terms {
            let horizontal_power = monomial.degree_in(horizontal);
            let vertical_power = monomial.degree_in(vertical);
            match (horizontal_power, vertical_power) {
                (2, 0) => square_horizontal = coefficient.clone(),
                (1, 1) => cross = coefficient.clone(),
                (0, 2) => square_vertical = coefficient.clone(),
                (1, 0) => linear_horizontal = coefficient.clone(),
                (0, 1) => linear_vertical = coefficient.clone(),
                (0, 0) => constant = coefficient.clone(),
                _ => return None,
            }
        }
        if !linear_horizontal.is_zero() || !linear_vertical.is_zero() || !constant.is_zero() {
            return None;
        }
        if square_horizontal.is_zero() {
            return self.factor_degenerate_quadratic(horizontal, vertical, cross, square_vertical);
        }

        let roots = rational_roots(&[
            square_vertical.clone(),
            cross.clone(),
            square_horizontal.clone(),
        ]);
        if roots.1.len() > 1 || roots.0.is_empty() {
            return None;
        }

        let mut scalar = square_horizontal;
        let mut factors = Vec::new();
        for (root, multiplicity) in roots.0 {
            let denominator = Rat::from_ints(root.den(), Int::one()).ok()?;
            for _ in 0..multiplicity {
                scalar = scalar.div(&denominator).ok()?;
            }
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
        if !scalar.is_one() {
            factors.insert(0, expression_from_rat(self.arena, &scalar));
        }
        Some(factors)
    }

    fn factor_degenerate_quadratic(
        &mut self,
        horizontal: SymbolId,
        vertical: SymbolId,
        cross: Rat,
        square_vertical: Rat,
    ) -> Option<Vec<ExprId>> {
        let mut factors = Vec::new();
        if !square_vertical.is_zero() {
            push_factor(
                self.arena,
                &self.context,
                &mut factors,
                Poly::variable(vertical),
                1,
            );
        }
        let mut linear = Poly::zero();
        if !cross.is_zero() {
            linear = linear.add(&Poly::variable(horizontal).scale(&cross));
        }
        if !square_vertical.is_zero() {
            linear = linear.add(&Poly::variable(vertical).scale(&square_vertical));
        }
        if !linear.is_constant() {
            push_factor(self.arena, &self.context, &mut factors, linear, 1);
        }
        if factors.is_empty() {
            None
        } else {
            Some(factors)
        }
    }
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
