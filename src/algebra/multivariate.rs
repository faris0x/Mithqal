//! Multivariate factorisation scaffold.
//!
//! The current pipeline extracts content and monomial factors before it reaches
//! this module. This module owns the multivariate stages that follow.
//!
//! Implemented stages:
//!
//! - Homogeneous binary quadratic forms with rational roots.
//! - Homogeneous bivariate forms of degree three and above.
//! - Recursive coefficient factorisation for bivariate quadratics.
//! - Generic multivariate interpolation across two through five variables for
//!   polynomials that split into linear factors with polynomial coefficients,
//!   including a constant rational leading coefficient.
//! - Power extraction from two through twelve for bivariate polynomials,
//!   including constant rational leading coefficients.
//! - Polynomial-leading quadratics with a general polynomial leading
//!   coefficient.
//! - Bivariate coefficient content extraction.
//! - Bivariate specialization root lifting.
//! - Bivariate Hensel lifting for coprime non-linear factor pairs.
//! - Even-term grouping.
//!
//! Reserved stages for the full algorithm:
//!
//! - Evaluation and interpolation over more than five variables.
//! - General multivariate factorisation beyond the staged bounds.

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
    PolynomialLeadingQuadratic,
    BivariateContent,
    BivariateRootLift,
    Grouping,
    SquareFree,
    MultivariateInterpolation,
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
            FactorStage::PolynomialLeadingQuadratic,
            FactorStage::BivariateContent,
            FactorStage::MultivariateInterpolation,
            FactorStage::Grouping,
            FactorStage::SquareFree,
            FactorStage::HenselLifting,
            FactorStage::BivariateRootLift,
        ] {
            let factors = match stage {
                FactorStage::BinaryQuadratic => self.factor_binary_quadratic(polynomial),
                FactorStage::HomogeneousBivariate => self.factor_homogeneous_bivariate(polynomial),
                FactorStage::RecursiveQuadratic => self.factor_recursive_quadratic(polynomial),
                FactorStage::PolynomialLeadingQuadratic => {
                    self.factor_polynomial_leading_quadratic(polynomial)
                }
                FactorStage::BivariateContent => self.factor_bivariate_content(polynomial),
                FactorStage::BivariateRootLift => self.factor_bivariate_root_lift(polynomial),
                FactorStage::Grouping => self.factor_by_grouping(polynomial),
                FactorStage::SquareFree => self.factor_square_free_stage(polynomial),
                FactorStage::MultivariateInterpolation => {
                    self.factor_multivariate_interpolation(polynomial)
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

    fn factor_bivariate_root_lift(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let (horizontal, vertical) = bivariate_variables(polynomial)?;
        if let Some(factors) =
            self.factor_bivariate_root_lift_orientation(polynomial, horizontal, vertical)
        {
            return Some(factors);
        }
        self.factor_bivariate_root_lift_orientation(polynomial, vertical, horizontal)
    }

    fn factor_bivariate_root_lift_orientation(
        &mut self,
        polynomial: &Poly,
        horizontal: SymbolId,
        vertical: SymbolId,
    ) -> Option<Vec<ExprId>> {
        let degree = x_degree(polynomial, horizontal);
        if !(2..=6).contains(&degree) {
            return None;
        }
        let bound = total_degree(polynomial);
        if bound > 10 {
            return None;
        }
        let root_bound = bound.saturating_sub(1);
        let needed = root_bound as usize + 1;
        for base in [0i64, 10, -10] {
            for reverse in [false, true] {
                let mut samples: Vec<(Rat, Vec<Rat>)> = Vec::new();
                for offset in 0..(4 * needed as i64 + 16) {
                    let Some(value) = Rat::from_i64(base + offset, 1).ok() else {
                        continue;
                    };
                    let (sample_degree, coefficients) =
                        substitute_vertical(polynomial, horizontal, vertical, &value);
                    if sample_degree != degree {
                        continue;
                    }
                    let (roots, remaining) = rational_roots(&coefficients);
                    if remaining.len() > 1 || roots.is_empty() {
                        continue;
                    }
                    let mut values = Vec::new();
                    for (root, multiplicity) in roots {
                        for _ in 0..multiplicity {
                            values.push(root.clone());
                        }
                    }
                    values.sort();
                    if reverse {
                        values.reverse();
                    }
                    samples.push((value, values));
                    if samples.len() == needed {
                        break;
                    }
                }
                if samples.len() < needed {
                    continue;
                }
                let maximum_roots = samples.iter().map(|(_, roots)| roots.len()).min()?;
                for root_index in 0..maximum_roots {
                    let points: Vec<(Rat, Rat)> = samples
                        .iter()
                        .map(|(value, roots)| (value.clone(), roots[root_index].clone()))
                        .collect();
                    let coefficients = interpolate(&points, root_bound)?;
                    let root = Poly::from_dense(vertical, &coefficients);
                    if evaluate_at_horizontal(polynomial, horizontal, vertical, &root)
                        != Poly::zero()
                    {
                        continue;
                    }
                    let Some(quotient) =
                        divide_by_linear_root(polynomial, horizontal, vertical, &root)
                    else {
                        continue;
                    };
                    if quotient.mul(
                        &Poly::variable(horizontal).add(&root.scale(&Rat::from_i64(-1, 1).ok()?)),
                    ) != *polynomial
                    {
                        continue;
                    }
                    let factor =
                        Poly::variable(horizontal).add(&root.scale(&Rat::from_i64(-1, 1).ok()?));
                    let mut factors =
                        vec![expression_from_poly(self.arena, &factor, &self.context)];
                    if let Some(mut nested) = self.factor_bivariate_root_lift(&quotient) {
                        factors.append(&mut nested);
                    } else if let Some(mut nested) = self.factor(&quotient) {
                        factors.append(&mut nested);
                    } else {
                        factors.push(expression_from_poly(self.arena, &quotient, &self.context));
                    }
                    return Some(factors);
                }
            }
        }
        None
    }

    fn factor_bivariate_content(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let (horizontal, vertical) = bivariate_variables(polynomial)?;
        let mut coefficients: Vec<Poly> = Vec::new();
        for (monomial, coefficient) in &polynomial.terms {
            let horizontal_power = monomial.degree_in(horizontal) as usize;
            let vertical_power = monomial.degree_in(vertical);
            if coefficients.len() <= horizontal_power {
                coefficients.resize(horizontal_power + 1, Poly::zero());
            }
            let mut powers = Vec::new();
            if vertical_power > 0 {
                powers.push((vertical, vertical_power));
            }
            coefficients[horizontal_power]
                .terms
                .insert(Monomial { powers }, coefficient.clone());
        }
        if coefficients.len() < 2 {
            return None;
        }
        let mut gcd = Vec::new();
        for coefficient in &coefficients {
            let dense = coefficient.dense_in(vertical)?;
            gcd = if gcd.is_empty() {
                dense
            } else {
                dense_gcd(&gcd, &dense)
            };
            if gcd.len() <= 1 {
                return None;
            }
        }
        if gcd.len() <= 1 {
            return None;
        }
        let mut quotient = Poly::zero();
        for (horizontal_power, coefficient) in coefficients.iter().enumerate() {
            let dense = coefficient.dense_in(vertical)?;
            let (quotient_coefficients, remainder) = dense_div_rem(&dense, &gcd)?;
            if !remainder.is_empty() {
                return None;
            }
            let quotient_coefficient = Poly::from_dense(vertical, &quotient_coefficients);
            for (monomial, value) in &quotient_coefficient.terms {
                let mut powers = Vec::new();
                if horizontal_power > 0 {
                    powers.push((horizontal, horizontal_power as u32));
                }
                for (symbol, exponent) in &monomial.powers {
                    powers.push((*symbol, *exponent));
                }
                powers.sort_by_key(|(symbol, _)| *symbol);
                quotient.terms.insert(Monomial { powers }, value.clone());
            }
        }
        let common = Poly::from_dense(vertical, &gcd);
        if common.mul(&quotient) != *polynomial {
            return None;
        }
        Some(vec![
            expression_from_poly(self.arena, &common, &self.context),
            expression_from_poly(self.arena, &quotient, &self.context),
        ])
    }

    fn factor_polynomial_leading_quadratic(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let (horizontal, vertical) = bivariate_variables(polynomial)?;
        if let Some(factors) =
            self.factor_polynomial_leading_quadratic_orientation(polynomial, horizontal, vertical)
        {
            return Some(factors);
        }
        self.factor_polynomial_leading_quadratic_orientation(polynomial, vertical, horizontal)
    }

    fn factor_polynomial_leading_quadratic_orientation(
        &mut self,
        polynomial: &Poly,
        horizontal: SymbolId,
        vertical: SymbolId,
    ) -> Option<Vec<ExprId>> {
        let (leading, linear, constant) = quadratic_coefficients(polynomial, horizontal, vertical)?;
        if leading.is_constant() {
            return None;
        }
        let four = Poly::constant(Rat::from_i64(4, 1).ok()?);
        let discriminant = linear.mul(&linear).sub(&four.mul(&leading).mul(&constant));
        let discriminant = discriminant.dense_in(vertical)?;
        let root = Poly::from_dense(vertical, &dense_nth_root(&discriminant, 2)?);
        let two_leading = leading.scale(&Rat::from_i64(2, 1).ok()?);
        let linear_term = Poly::variable(horizontal).mul(&two_leading);
        let first = linear_term.add(&linear).sub(&root);
        let second = linear_term.add(&linear).add(&root);
        let leading_coefficients = leading.dense_in(vertical)?;
        let mut attempts: Vec<(Poly, Poly)> = Vec::new();
        if let Some(quotient) =
            divide_linear_by_dense(&first, &leading_coefficients, horizontal, vertical)
        {
            attempts.push((quotient, second.clone()));
        }
        if let Some(quotient) =
            divide_linear_by_dense(&second, &leading_coefficients, horizontal, vertical)
        {
            attempts.push((quotient, first.clone()));
        }
        if let Some(components) = super::factor_dense_components(&leading_coefficients) {
            let mut left = first.clone();
            let mut right = second.clone();
            let mut distributed = true;
            for (component, multiplicity) in components {
                for _ in 0..multiplicity {
                    if let Some(quotient) =
                        divide_linear_by_dense(&left, &component, horizontal, vertical)
                    {
                        left = quotient;
                    } else if let Some(quotient) =
                        divide_linear_by_dense(&right, &component, horizontal, vertical)
                    {
                        right = quotient;
                    } else {
                        distributed = false;
                        break;
                    }
                }
                if !distributed {
                    break;
                }
            }
            if distributed {
                attempts.push((left, right));
            }
        }
        let quarter = Rat::from_i64(1, 4).ok()?;
        for (left, right) in attempts {
            let left = left.scale(&quarter);
            let (_, left) = extract_common_factor(&left)?;
            let (_, right) = extract_common_factor(&right)?;
            if left.mul(&right) == *polynomial {
                return Some(vec![
                    expression_from_poly(self.arena, &left, &self.context),
                    expression_from_poly(self.arena, &right, &self.context),
                ]);
            }
        }
        None
    }

    fn factor_square_free_stage(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let (horizontal, vertical) = bivariate_variables(polynomial)?;
        if let Some(factors) = self.factor_power_orientation(polynomial, horizontal, vertical) {
            return Some(factors);
        }
        self.factor_power_orientation(polynomial, vertical, horizontal)
    }

    fn factor_power_orientation(
        &mut self,
        polynomial: &Poly,
        horizontal: SymbolId,
        vertical: SymbolId,
    ) -> Option<Vec<ExprId>> {
        let leading = constant_leading_coefficient(polynomial, horizontal)?;
        let scaling = if leading.is_one() {
            polynomial.clone()
        } else {
            polynomial.scale(&Rat::one().div(&leading).ok()?)
        };
        let degree = x_degree(&scaling, horizontal);
        if degree < 2 || !is_monic_in(&scaling, horizontal) {
            return None;
        }
        let bound = total_degree(&scaling);
        if bound > 24 {
            return None;
        }
        let maximum_power = (degree.min(bound as usize).min(12)) as u32;
        for power in (2..=maximum_power).rev() {
            if !degree.is_multiple_of(power as usize) || !bound.is_multiple_of(power) {
                continue;
            }
            let root_bound = bound / power;
            let needed = root_bound as usize + 1;
            for base in [0i64, 10, -10] {
                let mut samples: Vec<(Rat, Vec<Rat>)> = Vec::new();
                for offset in 0..(4 * needed as i64 + 16) {
                    let Some(value) = Rat::from_i64(base + offset, 1).ok() else {
                        continue;
                    };
                    let (sample_degree, coefficients) =
                        substitute_vertical(&scaling, horizontal, vertical, &value);
                    if sample_degree != degree {
                        continue;
                    }
                    let Some(mut root) = dense_nth_root(&coefficients, power) else {
                        continue;
                    };
                    root.resize(degree / power as usize + 1, Rat::zero());
                    samples.push((value, root));
                    if samples.len() == needed {
                        break;
                    }
                }
                if samples.len() < needed {
                    continue;
                }
                let mut root_coefficients = Vec::with_capacity(degree / power as usize + 1);
                for coefficient in 0..=degree / power as usize {
                    let points: Vec<(Rat, Rat)> = samples
                        .iter()
                        .map(|(value, roots)| {
                            (
                                value.clone(),
                                roots.get(coefficient).cloned().unwrap_or_else(Rat::zero),
                            )
                        })
                        .collect();
                    root_coefficients.push(interpolate(&points, root_bound)?);
                }
                let mut root = Poly::zero();
                for (horizontal_power, coefficients) in root_coefficients.iter().enumerate() {
                    for (vertical_power, coefficient) in coefficients.iter().enumerate() {
                        if coefficient.is_zero() {
                            continue;
                        }
                        let mut powers = Vec::new();
                        if horizontal_power > 0 {
                            powers.push((horizontal, horizontal_power as u32));
                        }
                        if vertical_power > 0 {
                            powers.push((vertical, vertical_power as u32));
                        }
                        powers.sort_by_key(|(symbol, _)| *symbol);
                        root.terms.insert(Monomial { powers }, coefficient.clone());
                    }
                }
                let mut expansion = Poly::constant(Rat::one());
                for _ in 0..power {
                    expansion = expansion.mul(&root);
                }
                if expansion != scaling {
                    continue;
                }
                let expression = expression_from_poly(self.arena, &root, &self.context);
                let exponent = self.arena.intern_int(Int::from_u64(power as u64));
                let mut factors = Vec::new();
                if !leading.is_one() {
                    factors.push(expression_from_rat(self.arena, &leading));
                }
                factors.push(
                    self.arena
                        .intern_call(self.context.power, &[expression, exponent]),
                );
                return Some(factors);
            }
        }
        None
    }

    fn factor_multivariate_interpolation(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let variables = polynomial.variables();
        if !(2..=5).contains(&variables.len()) {
            return None;
        }
        let variables: Vec<SymbolId> = variables.into_iter().collect();
        for horizontal in &variables {
            let others: Vec<SymbolId> = variables
                .iter()
                .filter(|symbol| *symbol != horizontal)
                .copied()
                .collect();
            if let Some(factors) =
                self.factor_multivariate_orientation(polynomial, *horizontal, &others)
            {
                return Some(factors);
            }
        }
        None
    }

    fn factor_multivariate_orientation(
        &mut self,
        polynomial: &Poly,
        horizontal: SymbolId,
        others: &[SymbolId],
    ) -> Option<Vec<ExprId>> {
        let degree = x_degree(polynomial, horizontal);
        if !(2..=4).contains(&degree) {
            return None;
        }
        let leading = constant_leading_coefficient(polynomial, horizontal)?;
        let scaling = if leading.is_one() {
            polynomial.clone()
        } else {
            polynomial.scale(&Rat::one().div(&leading).ok()?)
        };
        if !is_monic_in(&scaling, horizontal) {
            return None;
        }
        let bound = total_degree(&scaling);
        let root_bound = bound.saturating_sub(degree as u32) + 1;
        if root_bound > 5 {
            return None;
        }
        let grid = root_bound as usize + 1;
        {
            let mut point_count = 1usize;
            for _ in 0..others.len() {
                point_count = point_count.checked_mul(grid)?;
            }
            if point_count > 256 {
                return None;
            }
            for base in [0i64, 10, -10] {
                for reverse in [false, true] {
                    let axes: Vec<Vec<Rat>> = (0..others.len())
                        .map(|_axis| {
                            (0..grid)
                                .filter_map(|offset| Rat::from_i64(base + offset as i64, 1).ok())
                                .collect()
                        })
                        .collect();
                    if axes.iter().any(|axis| axis.len() != grid) {
                        continue;
                    }
                    let mut points: Vec<(Vec<usize>, Vec<Rat>)> = Vec::with_capacity(point_count);
                    let mut complete = true;
                    for code in 0..point_count {
                        let mut indices = vec![0usize; others.len()];
                        let mut remainder = code;
                        for dimension in 0..others.len() {
                            indices[dimension] = remainder % grid;
                            remainder /= grid;
                        }
                        let values: Vec<Rat> = indices
                            .iter()
                            .enumerate()
                            .map(|(dimension, index)| axes[dimension][*index].clone())
                            .collect();
                        let (sample_degree, coefficients) =
                            substitute_multivariate(&scaling, horizontal, others, &values);
                        if sample_degree != degree {
                            complete = false;
                            break;
                        }
                        let (roots, remaining) = rational_roots(&coefficients);
                        if remaining.len() > 1 {
                            complete = false;
                            break;
                        }
                        let mut root_values = Vec::new();
                        for (root, multiplicity) in roots {
                            for _ in 0..multiplicity {
                                root_values.push(root.clone());
                            }
                        }
                        if root_values.len() != degree {
                            complete = false;
                            break;
                        }
                        root_values.sort();
                        if reverse {
                            root_values.reverse();
                        }
                        points.push((indices, root_values));
                    }
                    if !complete {
                        continue;
                    }
                    let degree_bound = (grid - 1) as u32;
                    let mut root_polynomials = Vec::with_capacity(degree);
                    for root_index in 0..degree {
                        let values: Vec<Rat> = points
                            .iter()
                            .map(|(_, roots)| roots[root_index].clone())
                            .collect();
                        let coefficients = tensor_interpolate(&axes, &values, grid, degree_bound)?;
                        let mut root = Poly::zero();
                        for (code, coefficient) in coefficients.iter().enumerate() {
                            if coefficient.is_zero() {
                                continue;
                            }
                            let mut remainder = code;
                            let mut powers = Vec::new();
                            for variable in others.iter() {
                                let power = remainder % grid;
                                remainder /= grid;
                                if power > 0 {
                                    powers.push((*variable, power as u32));
                                }
                            }
                            powers.sort_by_key(|(symbol, _)| *symbol);
                            root.terms.insert(Monomial { powers }, coefficient.clone());
                        }
                        root_polynomials.push(root);
                    }
                    let mut collected: Vec<(Poly, u32)> = Vec::new();
                    let mut product = Poly::constant(Rat::one());
                    for root in root_polynomials {
                        let factor = Poly::variable(horizontal)
                            .add(&root.scale(&Rat::from_i64(-1, 1).ok()?));
                        product = product.mul(&factor);
                        merge_poly_factor(&mut collected, factor);
                    }
                    if product != scaling {
                        continue;
                    }
                    let mut factors = Vec::new();
                    if !leading.is_one() {
                        let mut chosen = None;
                        let mut best = Int::one();
                        for (index, (factor, multiplicity)) in collected.iter().enumerate() {
                            if *multiplicity != 1 {
                                continue;
                            }
                            let denominator = poly_denominator_lcm(factor);
                            if chosen.is_none()
                                || denominator.cmp(&best) == core::cmp::Ordering::Greater
                            {
                                best = denominator;
                                chosen = Some(index);
                            }
                        }
                        if let Some(index) = chosen {
                            collected[index].0 = collected[index].0.scale(&leading);
                        } else {
                            factors.push(expression_from_rat(self.arena, &leading));
                        }
                    }
                    for (factor, multiplicity) in collected {
                        push_factor(
                            self.arena,
                            &self.context,
                            &mut factors,
                            factor,
                            multiplicity,
                        );
                    }
                    return Some(factors);
                }
            }
        }
        None
    }

    fn factor_hensel_stage(&mut self, polynomial: &Poly) -> Option<Vec<ExprId>> {
        let (horizontal, vertical) = bivariate_variables(polynomial)?;
        if let Some(factors) = self.factor_hensel_orientation(polynomial, horizontal, vertical) {
            return Some(factors);
        }
        self.factor_hensel_orientation(polynomial, vertical, horizontal)
    }

    fn factor_hensel_orientation(
        &mut self,
        polynomial: &Poly,
        horizontal: SymbolId,
        vertical: SymbolId,
    ) -> Option<Vec<ExprId>> {
        let degree = x_degree(polynomial, horizontal);
        if !(2..=6).contains(&degree) {
            return None;
        }
        let bound = total_degree(polynomial);
        if bound > 12 {
            return None;
        }
        let vertical_degree = y_degree(polynomial, vertical);
        if vertical_degree == 0 || vertical_degree > 6 {
            return None;
        }
        for base in [0i64, 1, -1, 2, 10] {
            let Some(value) = Rat::from_i64(base, 1).ok() else {
                continue;
            };
            let (slice_degree, slice) =
                substitute_vertical(polynomial, horizontal, vertical, &value);
            if slice_degree != degree {
                continue;
            }
            let Some(components) = super::factor_dense_components(&slice) else {
                continue;
            };
            let mut factors: Vec<Vec<Rat>> = Vec::new();
            for (component, multiplicity) in components {
                if component.len() <= 1 {
                    continue;
                }
                for _ in 0..multiplicity {
                    factors.push(component.clone());
                }
            }
            if !(2..=6).contains(&factors.len()) {
                continue;
            }
            let count = factors.len();
            for mask in 1usize..(1usize << count) - 1 {
                let mut left = vec![Rat::one()];
                let mut right = vec![Rat::one()];
                for (index, factor) in factors.iter().enumerate() {
                    if mask & (1 << index) != 0 {
                        left = dense_multiply(&left, factor);
                    } else {
                        right = dense_multiply(&right, factor);
                    }
                }
                if left.len() <= 1 || right.len() <= 1 {
                    continue;
                }
                let product = dense_multiply(&left, &right);
                let (scalar, remainder) = dense_div_rem(&slice, &product)?;
                if !remainder.is_empty() || scalar.len() > 1 {
                    continue;
                }
                let scalar = scalar.first().cloned().unwrap_or_else(Rat::one);
                let left = dense_scale(&left, &scalar);
                let Some((left_poly, right_poly)) = lift_bivariate_pair(
                    polynomial,
                    horizontal,
                    vertical,
                    &value,
                    &left,
                    &right,
                    vertical_degree,
                ) else {
                    continue;
                };
                if left_poly.mul(&right_poly) != *polynomial {
                    continue;
                }
                let mut result = Vec::new();
                if let Some(mut nested) = self.factor(&left_poly) {
                    result.append(&mut nested);
                } else {
                    result.push(expression_from_poly(self.arena, &left_poly, &self.context));
                }
                if let Some(mut nested) = self.factor(&right_poly) {
                    result.append(&mut nested);
                } else {
                    result.push(expression_from_poly(self.arena, &right_poly, &self.context));
                }
                return Some(result);
            }
        }
        None
    }
}

fn substitute_multivariate(
    polynomial: &Poly,
    horizontal: SymbolId,
    variables: &[SymbolId],
    values: &[Rat],
) -> (usize, Vec<Rat>) {
    let degree = x_degree(polynomial, horizontal);
    let mut coefficients = vec![Rat::zero(); degree + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal) as usize;
        let mut factor = Rat::one();
        for (variable, value) in variables.iter().zip(values.iter()) {
            factor = factor.mul(&rat_power(value, monomial.degree_in(*variable)));
        }
        coefficients[horizontal_power] =
            coefficients[horizontal_power].add(&coefficient.mul(&factor));
    }
    (degree, trim(coefficients))
}

fn split_linear_polynomial(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<(Poly, Poly)> {
    let mut coefficient = Poly::zero();
    let mut constant = Poly::zero();
    for (monomial, value) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal);
        let vertical_power = monomial.degree_in(vertical);
        let target = match horizontal_power {
            1 => &mut coefficient,
            0 => &mut constant,
            _ => return None,
        };
        let mut powers = Vec::new();
        if vertical_power > 0 {
            powers.push((vertical, vertical_power));
        }
        let monomial = Monomial { powers };
        match target.terms.get_mut(&monomial) {
            Some(existing) => *existing = existing.add(value),
            None => {
                target.terms.insert(monomial, value.clone());
            }
        }
    }
    Some((coefficient, constant))
}

fn divide_linear_by_dense(
    factor: &Poly,
    divisor: &[Rat],
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<Poly> {
    let (coefficient, constant) = split_linear_polynomial(factor, horizontal, vertical)?;
    let coefficient = coefficient.dense_in(vertical)?;
    let constant = constant.dense_in(vertical)?;
    let (coefficient, coefficient_remainder) = dense_div_rem(&coefficient, divisor)?;
    if !coefficient_remainder.is_empty() {
        return None;
    }
    let (constant, constant_remainder) = dense_div_rem(&constant, divisor)?;
    if !constant_remainder.is_empty() {
        return None;
    }
    Some(
        Poly::variable(horizontal)
            .mul(&Poly::from_dense(vertical, &coefficient))
            .add(&Poly::from_dense(vertical, &constant)),
    )
}

fn quadratic_coefficients(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<(Poly, Poly, Poly)> {
    let mut leading = Poly::zero();
    let mut linear = Poly::zero();
    let mut constant = Poly::zero();
    let mut degree = 0u32;
    for (monomial, coefficient) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal);
        let vertical_power = monomial.degree_in(vertical);
        degree = degree.max(horizontal_power);
        let mut powers = Vec::new();
        if vertical_power > 0 {
            powers.push((vertical, vertical_power));
        }
        let monomial = Monomial { powers };
        let target = match horizontal_power {
            2 => &mut leading,
            1 => &mut linear,
            0 => &mut constant,
            _ => return None,
        };
        match target.terms.get_mut(&monomial) {
            Some(existing) => *existing = existing.add(coefficient),
            None => {
                target.terms.insert(monomial, coefficient.clone());
            }
        }
    }
    if degree != 2 {
        return None;
    }
    Some((leading, linear, constant))
}

fn y_degree(polynomial: &Poly, vertical: SymbolId) -> u32 {
    polynomial
        .terms
        .keys()
        .map(|monomial| monomial.degree_in(vertical))
        .max()
        .unwrap_or(0)
}

fn dense_scale(coefficients: &[Rat], scale: &Rat) -> Vec<Rat> {
    trim(coefficients.iter().map(|value| value.mul(scale)).collect())
}

fn dense_subtract(left: &[Rat], right: &[Rat]) -> Vec<Rat> {
    let length = left.len().max(right.len());
    let mut difference = vec![Rat::zero(); length];
    for index in 0..length {
        difference[index] = left
            .get(index)
            .cloned()
            .unwrap_or_else(Rat::zero)
            .sub(&right.get(index).cloned().unwrap_or_else(Rat::zero));
    }
    trim(difference)
}

fn dense_extended_gcd(left: &[Rat], right: &[Rat]) -> Option<(Vec<Rat>, Vec<Rat>, Vec<Rat>)> {
    let mut old_remainder = trim(left.to_vec());
    let mut remainder = trim(right.to_vec());
    let mut old_left = vec![Rat::one()];
    let mut left_coefficient = Vec::new();
    let mut old_right = Vec::new();
    let mut right_coefficient = vec![Rat::one()];
    while !remainder.is_empty() {
        let (quotient, next_remainder) = dense_div_rem(&old_remainder, &remainder)?;
        old_remainder = remainder;
        remainder = next_remainder;
        let next_left = dense_subtract(&old_left, &dense_multiply(&quotient, &left_coefficient));
        old_left = left_coefficient;
        left_coefficient = next_left;
        let next_right = dense_subtract(&old_right, &dense_multiply(&quotient, &right_coefficient));
        old_right = right_coefficient;
        right_coefficient = next_right;
    }
    let constant = old_remainder.first()?.clone();
    if constant.is_zero() {
        return None;
    }
    let inverse = Rat::one().div(&constant).ok()?;
    Some((
        old_remainder,
        dense_scale(&old_left, &inverse),
        dense_scale(&old_right, &inverse),
    ))
}

fn binomial(n: u32, k: u32) -> Rat {
    let mut result = 1u128;
    for index in 1..=k {
        result = result * (n - k + index) as u128 / index as u128;
    }
    Rat::from_i64(result as i64, 1).unwrap_or_else(|_| Rat::one())
}

fn taylor_slices(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Rat,
    maximum: u32,
) -> Vec<Vec<Rat>> {
    let mut slices = vec![Vec::new(); maximum as usize + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal) as usize;
        let vertical_power = monomial.degree_in(vertical);
        for power in 0..=vertical_power {
            let factor =
                binomial(vertical_power, power).mul(&rat_power(value, vertical_power - power));
            let contribution = coefficient.mul(&factor);
            let slice = &mut slices[power as usize];
            if slice.len() <= horizontal_power {
                slice.resize(horizontal_power + 1, Rat::zero());
            }
            slice[horizontal_power] = slice[horizontal_power].add(&contribution);
        }
    }
    for slice in &mut slices {
        *slice = trim(std::mem::take(slice));
    }
    slices
}

fn polynomial_from_t_slices(
    slices: &[Vec<Rat>],
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Rat,
) -> Poly {
    let t = Poly::variable(vertical).sub(&Poly::constant(value.clone()));
    let mut result = Poly::zero();
    let mut power = Poly::constant(Rat::one());
    for slice in slices {
        let coefficient = Poly::from_dense(horizontal, slice);
        result = result.add(&coefficient.mul(&power));
        power = power.mul(&t);
    }
    result
}

fn lift_bivariate_pair(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Rat,
    left: &[Rat],
    right: &[Rat],
    maximum: u32,
) -> Option<(Poly, Poly)> {
    let (gcd, _, bezout_right) = dense_extended_gcd(left, right)?;
    if gcd.len() != 1 {
        return None;
    }
    let slices = taylor_slices(polynomial, horizontal, vertical, value, maximum);
    let mut left_slices = vec![trim(left.to_vec())];
    let mut right_slices = vec![trim(right.to_vec())];
    for power in 1..=maximum as usize {
        let mut residual = slices.get(power).cloned().unwrap_or_default();
        for index in 1..power {
            residual = dense_subtract(
                &residual,
                &dense_multiply(&left_slices[index], &right_slices[power - index]),
            );
        }
        let projected = dense_multiply(&bezout_right, &residual);
        let (_, correction_left) = dense_div_rem(&projected, left)?;
        let correction_left = trim(correction_left);
        let numerator = dense_subtract(&residual, &dense_multiply(&correction_left, right));
        let (correction_right, remainder) = dense_div_rem(&numerator, left)?;
        if !remainder.is_empty() {
            return None;
        }
        left_slices.push(correction_left);
        right_slices.push(correction_right);
    }
    Some((
        polynomial_from_t_slices(&left_slices, horizontal, vertical, value),
        polynomial_from_t_slices(&right_slices, horizontal, vertical, value),
    ))
}

fn coefficient_polynomials(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
) -> Option<Vec<Poly>> {
    let degree = x_degree(polynomial, horizontal);
    let mut coefficients = vec![Poly::zero(); degree + 1];
    for (monomial, value) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal) as usize;
        let vertical_power = monomial.degree_in(vertical);
        let mut powers = Vec::new();
        if vertical_power > 0 {
            powers.push((vertical, vertical_power));
        }
        coefficients[horizontal_power]
            .terms
            .insert(Monomial { powers }, value.clone());
    }
    Some(coefficients)
}

fn evaluate_at_horizontal(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Poly,
) -> Poly {
    let Some(coefficients) = coefficient_polynomials(polynomial, horizontal, vertical) else {
        return Poly::zero();
    };
    let mut result = Poly::zero();
    for coefficient in coefficients.iter().rev() {
        result = result.mul(value).add(coefficient);
    }
    result
}

fn divide_by_linear_root(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    root: &Poly,
) -> Option<Poly> {
    let coefficients = coefficient_polynomials(polynomial, horizontal, vertical)?;
    let degree = coefficients.len().checked_sub(1)?;
    let mut quotient = vec![Poly::zero(); degree];
    if degree == 0 {
        return None;
    }
    quotient[degree - 1] = coefficients[degree].clone();
    for power in (1..degree).rev() {
        let next = coefficients[power].add(&root.mul(&quotient[power]));
        quotient[power - 1] = next;
    }
    let remainder = coefficients[0].add(&root.mul(&quotient[0]));
    if remainder != Poly::zero() {
        return None;
    }
    let mut result = Poly::zero();
    for (power, coefficient) in quotient.iter().enumerate() {
        for (monomial, value) in &coefficient.terms {
            let mut powers = Vec::new();
            if power > 0 {
                powers.push((horizontal, power as u32));
            }
            for (symbol, exponent) in &monomial.powers {
                powers.push((*symbol, *exponent));
            }
            powers.sort_by_key(|(symbol, _)| *symbol);
            result.terms.insert(Monomial { powers }, value.clone());
        }
    }
    Some(result)
}

fn bivariate_variables(polynomial: &Poly) -> Option<(SymbolId, SymbolId)> {
    let variables = polynomial.variables();
    if variables.len() != 2 {
        return None;
    }
    let mut symbols = variables.into_iter();
    Some((symbols.next()?, symbols.next()?))
}

fn x_degree(polynomial: &Poly, horizontal: SymbolId) -> usize {
    polynomial
        .terms
        .keys()
        .map(|monomial| monomial.degree_in(horizontal) as usize)
        .max()
        .unwrap_or(0)
}

fn poly_denominator_lcm(polynomial: &Poly) -> Int {
    let mut denominator = Int::one();
    for coefficient in polynomial.terms.values() {
        denominator = lcm(&denominator, &coefficient.den());
    }
    denominator
}

fn constant_leading_coefficient(polynomial: &Poly, horizontal: SymbolId) -> Option<Rat> {
    let degree = x_degree(polynomial, horizontal) as u32;
    let mut leading = None;
    for (monomial, coefficient) in &polynomial.terms {
        if monomial.degree_in(horizontal) != degree {
            continue;
        }
        if monomial
            .powers
            .iter()
            .any(|(symbol, exponent)| *symbol != horizontal && *exponent != 0)
        {
            return None;
        }
        if leading.is_some() {
            return None;
        }
        leading = Some(coefficient.clone());
    }
    let leading = leading?;
    if leading.is_zero() {
        None
    } else {
        Some(leading)
    }
}

fn is_monic_in(polynomial: &Poly, horizontal: SymbolId) -> bool {
    let degree = x_degree(polynomial, horizontal) as u32;
    let mut found = false;
    for (monomial, coefficient) in &polynomial.terms {
        if monomial.degree_in(horizontal) != degree {
            continue;
        }
        if monomial
            .powers
            .iter()
            .any(|(symbol, exponent)| *symbol != horizontal && *exponent != 0)
            || !coefficient.is_one()
            || found
        {
            return false;
        }
        found = true;
    }
    found
}

fn substitute_vertical(
    polynomial: &Poly,
    horizontal: SymbolId,
    vertical: SymbolId,
    value: &Rat,
) -> (usize, Vec<Rat>) {
    let degree = x_degree(polynomial, horizontal);
    let mut coefficients = vec![Rat::zero(); degree + 1];
    for (monomial, coefficient) in &polynomial.terms {
        let horizontal_power = monomial.degree_in(horizontal) as usize;
        let vertical_power = monomial.degree_in(vertical);
        let factor = rat_power(value, vertical_power);
        coefficients[horizontal_power] =
            coefficients[horizontal_power].add(&coefficient.mul(&factor));
    }
    (degree, trim(coefficients))
}

fn rat_power(value: &Rat, exponent: u32) -> Rat {
    let mut result = Rat::one();
    for _ in 0..exponent {
        result = result.mul(value);
    }
    result
}

fn interpolate(points: &[(Rat, Rat)], degree: u32) -> Option<Vec<Rat>> {
    if points.is_empty() {
        return None;
    }
    let mut result = vec![Rat::zero()];
    for (index, (value, image)) in points.iter().enumerate() {
        let mut basis = vec![Rat::one()];
        let mut denominator = Rat::one();
        for (other, (other_value, _)) in points.iter().enumerate() {
            if other == index {
                continue;
            }
            basis = dense_multiply(&basis, &[other_value.neg(), Rat::one()]);
            denominator = denominator.mul(&value.sub(other_value));
        }
        let scale = image.div(&denominator).ok()?;
        if result.len() < basis.len() {
            result.resize(basis.len(), Rat::zero());
        }
        for (power, coefficient) in basis.iter().enumerate() {
            result[power] = result[power].add(&coefficient.mul(&scale));
        }
    }
    let result = trim(result);
    if result.len() as u32 > degree + 1 {
        return None;
    }
    Some(result)
}

fn tensor_interpolate(
    axes: &[Vec<Rat>],
    values: &[Rat],
    grid: usize,
    degree: u32,
) -> Option<Vec<Rat>> {
    let mut data = values.to_vec();
    let mut stride = 1usize;
    for axis in axes {
        let block = stride.checked_mul(grid)?;
        let outer = data.len().checked_div(block)?;
        let mut next = vec![Rat::zero(); data.len()];
        for outer_index in 0..outer {
            for inner in 0..stride {
                let points: Vec<(Rat, Rat)> = (0..grid)
                    .map(|index| {
                        (
                            axis[index].clone(),
                            data[outer_index * block + index * stride + inner].clone(),
                        )
                    })
                    .collect();
                let coefficients = interpolate(&points, degree)?;
                for (index, coefficient) in coefficients.iter().enumerate() {
                    if index < grid {
                        next[outer_index * block + index * stride + inner] = coefficient.clone();
                    }
                }
            }
        }
        data = next;
        stride = block;
    }
    Some(data)
}

fn merge_poly_factor(factors: &mut Vec<(Poly, u32)>, polynomial: Poly) {
    for (existing, multiplicity) in factors.iter_mut() {
        if *existing == polynomial {
            *multiplicity += 1;
            return;
        }
    }
    factors.push((polynomial, 1));
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
