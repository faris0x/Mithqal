//! Native analytic number theory accelerator.
//!
//! Experimental analytic zeta and Dirichlet L-function evaluation.
//!
//! The Hurwitz zeta function uses Euler-Maclaurin summation in the form
//! described by Johansson and used by FLINT. Dirichlet L-functions are
//! decomposed into Hurwitz zeta values. Evaluations and zero searches are
//! numerical experiments, not certified enclosures.

#[allow(unused_imports)]
use crate::prelude::*;

/// A double precision complex number.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Complex {
    /// Real part.
    pub(crate) re: f64,
    /// Imaginary part.
    pub(crate) im: f64,
}

impl Complex {
    /// Build a complex number.
    pub(crate) fn new(re: f64, im: f64) -> Self {
        Complex { re, im }
    }

    /// Zero.
    pub(crate) fn zero() -> Self {
        Complex::new(0.0, 0.0)
    }

    /// One.
    pub(crate) fn one() -> Self {
        Complex::new(1.0, 0.0)
    }

    /// Addition.
    pub(crate) fn add(self, other: Self) -> Self {
        Complex::new(self.re + other.re, self.im + other.im)
    }

    /// Subtraction.
    pub(crate) fn sub(self, other: Self) -> Self {
        Complex::new(self.re - other.re, self.im - other.im)
    }

    /// Multiplication.
    pub(crate) fn mul(self, other: Self) -> Self {
        Complex::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    /// Division.
    pub(crate) fn div(self, other: Self) -> Option<Self> {
        let denominator = other.re * other.re + other.im * other.im;
        if denominator == 0.0 {
            return None;
        }
        Some(Complex::new(
            (self.re * other.re + self.im * other.im) / denominator,
            (self.im * other.re - self.re * other.im) / denominator,
        ))
    }

    /// Negation.
    pub(crate) fn neg(self) -> Self {
        Complex::new(-self.re, -self.im)
    }

    /// Modulus.
    pub(crate) fn abs(self) -> f64 {
        (self.re * self.re + self.im * self.im).sqrt()
    }

    /// Complex exponential.
    pub(crate) fn exp(self) -> Self {
        let magnitude = self.re.exp();
        Complex::new(magnitude * self.im.cos(), magnitude * self.im.sin())
    }

    /// Principal complex logarithm.
    pub(crate) fn ln(self) -> Option<Self> {
        let magnitude = self.abs();
        if magnitude == 0.0 {
            return None;
        }
        Some(Complex::new(magnitude.ln(), argument(self.re, self.im)))
    }

    /// Complex power.
    pub(crate) fn pow(self, exponent: Self) -> Option<Self> {
        Some(exponent.mul(self.ln()?).exp())
    }

    /// Complex sine.
    pub(crate) fn sin(self) -> Self {
        let imaginary_unit = Complex::new(0.0, 1.0);
        let positive = self.mul(imaginary_unit).exp();
        let negative = self.mul(imaginary_unit.neg()).exp();
        positive
            .sub(negative)
            .div(Complex::new(0.0, 2.0))
            .unwrap_or_else(Complex::zero)
    }

    /// Complex cosine.
    pub(crate) fn cos(self) -> Self {
        let imaginary_unit = Complex::new(0.0, 1.0);
        let positive = self.mul(imaginary_unit).exp();
        let negative = self.mul(imaginary_unit.neg()).exp();
        positive
            .add(negative)
            .div(Complex::new(2.0, 0.0))
            .unwrap_or_else(Complex::zero)
    }

    /// Complex gamma by Lanczos approximation with reflection.
    pub(crate) fn gamma(self) -> Option<Self> {
        const COEFFICIENTS: [f64; 9] = [
            0.999_999_999_999_809_9,
            676.520_368_121_885_1,
            -1_259.139_216_722_402_8,
            771.323_428_777_653_1,
            -176.615_029_162_140_6,
            12.507_343_278_686_905,
            -0.138_571_095_265_720_12,
            9.984_369_578_019_57e-6,
            1.505_632_735_149_311_6e-7,
        ];
        if self.re < 0.5 {
            let pi = core::f64::consts::PI;
            let sine = Complex::new(pi * self.re, pi * self.im).sin();
            return Complex::new(pi, 0.0).div(sine.mul(Complex::one().sub(self).gamma()?));
        }
        let z = self.sub(Complex::one());
        let mut series = Complex::new(COEFFICIENTS[0], 0.0);
        for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
            series = series
                .add(Complex::new(*coefficient, 0.0).div(z.add(Complex::new(index as f64, 0.0)))?);
        }
        let shifted = z.add(Complex::new(7.5, 0.0));
        let two_pi = (2.0 * core::f64::consts::PI).sqrt();
        Some(
            Complex::new(two_pi, 0.0)
                .mul(shifted.pow(z.add(Complex::new(0.5, 0.0)))?)
                .mul(shifted.neg().exp())
                .mul(series),
        )
    }
}

const BERNOULLI: [(f64, usize); 10] = [
    (1.0 / 6.0, 2),
    (-1.0 / 30.0, 4),
    (1.0 / 42.0, 6),
    (-1.0 / 30.0, 8),
    (5.0 / 66.0, 10),
    (-691.0 / 2730.0, 12),
    (7.0 / 6.0, 14),
    (-3617.0 / 510.0, 16),
    (43867.0 / 798.0, 18),
    (-174611.0 / 330.0, 20),
];

/// Evaluate the Hurwitz zeta function by Euler-Maclaurin summation.
///
/// Valid for `a > 0` and `s != 1`. The truncation parameters are fixed for
/// moderate double precision accuracy.
pub(crate) fn hurwitz_zeta(s: Complex, a: f64) -> Option<Complex> {
    if a <= 0.0 {
        return None;
    }
    if s.re == 1.0 && s.im == 0.0 {
        return None;
    }
    let terms = 128usize;
    let mut result = Complex::zero();
    for k in 0..terms {
        result = result.add(power_real(a + k as f64, s.neg()));
    }
    let shifted = a + terms as f64;
    let one_minus_s = Complex::one().sub(s);
    let integral = power_real(shifted, one_minus_s).div(s.sub(Complex::one()))?;
    result = result.add(integral);
    result = result.add(power_real(shifted, s.neg()).mul(Complex::new(0.5, 0.0)));
    let mut rising = Complex::one();
    let mut exponent = 0usize;
    for (bernoulli, order) in BERNOULLI {
        while exponent < order - 1 {
            rising = rising.mul(s.add(Complex::new(exponent as f64, 0.0)));
            exponent += 1;
        }
        let power = power_real(shifted, s.add(Complex::new((order - 1) as f64, 0.0)).neg());
        let term = power
            .mul(rising)
            .mul(Complex::new(bernoulli / factorial(order), 0.0));
        result = result.add(term);
        if exponent + 1 < 128 {
            rising = rising.mul(s.add(Complex::new(exponent as f64, 0.0)));
            exponent += 1;
        }
    }
    Some(result)
}

/// Evaluate the Riemann zeta function.
pub(crate) fn zeta(s: Complex) -> Option<Complex> {
    if s.im == 0.0 && s.re <= 0.0 && s.re % 1.0 == 0.0 {
        let index = (-s.re) as i64;
        return Some(match index {
            0 => Complex::new(-0.5, 0.0),
            1 => Complex::new(-1.0 / 12.0, 0.0),
            2 => Complex::zero(),
            3 => Complex::new(1.0 / 120.0, 0.0),
            _ => {
                if index % 2 == 0 {
                    Complex::zero()
                } else {
                    return None;
                }
            }
        });
    }
    if s.re < 0.5 {
        let one_minus = Complex::one().sub(s);
        let pi = core::f64::consts::PI;
        let factor = power_real(2.0, s)
            .mul(power_real(pi, s.sub(Complex::one())))
            .mul(Complex::new(pi * s.re / 2.0, pi * s.im / 2.0).sin())
            .mul(one_minus.gamma()?);
        return Some(factor.mul(zeta(one_minus)?));
    }
    hurwitz_zeta(s, 1.0)
}

/// A Dirichlet character given by its values on residues.
#[derive(Clone, Debug)]
pub(crate) struct DirichletCharacter {
    modulus: u64,
    values: Vec<Complex>,
}

impl DirichletCharacter {
    /// Build a character from values at `0, 1, ..., modulus - 1`.
    pub(crate) fn new(modulus: u64, values: Vec<Complex>) -> Option<Self> {
        if modulus == 0 || values.len() as u64 != modulus {
            return None;
        }
        Some(DirichletCharacter { modulus, values })
    }

    /// The modulus.
    pub(crate) fn modulus(&self) -> u64 {
        self.modulus
    }

    /// The value at a residue.
    pub(crate) fn value(&self, residue: u64) -> Complex {
        self.values[(residue % self.modulus) as usize]
    }

    /// The complex conjugate character.
    pub(crate) fn conjugate(&self) -> Self {
        DirichletCharacter {
            modulus: self.modulus,
            values: self
                .values
                .iter()
                .map(|value| Complex::new(value.re, -value.im))
                .collect(),
        }
    }

    /// The parity, zero for even and one for odd, or `None` for non-real data.
    pub(crate) fn parity(&self) -> Option<u32> {
        let value = self.value(self.modulus - 1);
        if value.im.abs() > 1e-12 {
            return None;
        }
        if (value.re - 1.0).abs() < 1e-12 {
            Some(0)
        } else if (value.re + 1.0).abs() < 1e-12 {
            Some(1)
        } else {
            None
        }
    }

    /// The Gauss sum.
    pub(crate) fn gauss_sum(&self) -> Option<Complex> {
        let modulus = self.modulus as f64;
        let mut total = Complex::zero();
        for residue in 1..self.modulus {
            let angle = 2.0 * core::f64::consts::PI * residue as f64 / modulus;
            total = total.add(
                self.value(residue)
                    .mul(Complex::new(angle.cos(), angle.sin())),
            );
        }
        Some(total)
    }

    /// The conductor by testing induced characters on divisors.
    pub(crate) fn conductor(&self) -> Option<u64> {
        (1..=self.modulus)
            .find(|divisor| self.modulus.is_multiple_of(*divisor) && self.is_induced_by(*divisor))
    }

    /// True when the character is induced from the given modulus.
    pub(crate) fn is_induced_by(&self, modulus: u64) -> bool {
        if !self.modulus.is_multiple_of(modulus) {
            return false;
        }
        for left in 1..self.modulus {
            if gcd_u64(left, self.modulus) != 1 {
                continue;
            }
            for right in left + 1..self.modulus {
                if gcd_u64(right, self.modulus) == 1
                    && left % modulus == right % modulus
                    && self.value(left).sub(self.value(right)).abs() > 1e-12
                {
                    return false;
                }
            }
        }
        true
    }

    /// The primitive character inducing this one.
    pub(crate) fn primitive_reduction(&self) -> Option<(Self, u64)> {
        let conductor = self.conductor()?;
        if conductor == self.modulus {
            return Some((self.clone(), self.modulus));
        }
        let mut values = vec![Complex::zero(); conductor as usize];
        for residue in 1..conductor {
            if gcd_u64(residue, conductor) != 1 {
                continue;
            }
            let mut lift = residue;
            while gcd_u64(lift, self.modulus) != 1 {
                lift += conductor;
                if lift >= self.modulus * 2 {
                    return None;
                }
            }
            values[residue as usize] = self.value(lift);
        }
        Some((DirichletCharacter::new(conductor, values)?, self.modulus))
    }

    /// The root number for a primitive character.
    pub(crate) fn root_number(&self) -> Option<Complex> {
        let parity = self.parity()?;
        let gauss = self.gauss_sum()?;
        let modulus = (self.modulus as f64).sqrt();
        let denominator = if parity == 0 {
            Complex::new(modulus, 0.0)
        } else {
            Complex::new(0.0, modulus)
        };
        gauss.div(denominator)
    }
}

/// Evaluate a Dirichlet L-function through its Hurwitz decomposition.
pub(crate) fn dirichlet_l(s: Complex, character: &DirichletCharacter) -> Option<Complex> {
    let modulus = character.modulus();
    if modulus == 1 {
        return zeta(s);
    }
    if s.re == 1.0 && s.im == 0.0 {
        let epsilon = 1e-4;
        let first = dirichlet_l(Complex::new(1.0 + epsilon, 0.0), character)?;
        let second = dirichlet_l(Complex::new(1.0 + epsilon / 2.0, 0.0), character)?;
        return Some(second.mul(Complex::new(2.0, 0.0)).sub(first));
    }
    if s.re < 0.5 {
        let (primitive, original_modulus) = character.primitive_reduction()?;
        let mut value = primitive_functional(s, &primitive)?;
        if original_modulus != primitive.modulus() {
            let mut remaining = original_modulus;
            let mut divisor = 2u64;
            while divisor * divisor <= remaining {
                if remaining % divisor == 0 {
                    let factor = Complex::one().sub(
                        primitive
                            .value(divisor)
                            .mul(power_real(divisor as f64, s.neg())),
                    );
                    value = value.mul(factor);
                    while remaining % divisor == 0 {
                        remaining /= divisor;
                    }
                }
                divisor += 1;
            }
            if remaining > 1 {
                let factor = Complex::one().sub(
                    primitive
                        .value(remaining)
                        .mul(power_real(remaining as f64, s.neg())),
                );
                value = value.mul(factor);
            }
        }
        return Some(value);
    }
    let mut sum = Complex::zero();
    for residue in 1..modulus {
        let value = character.value(residue);
        if value.re == 0.0 && value.im == 0.0 {
            continue;
        }
        let term = hurwitz_zeta(s, residue as f64 / modulus as f64)?;
        sum = sum.add(value.mul(term));
    }
    let scale = power_real(modulus as f64, s.neg());
    Some(scale.mul(sum))
}

/// Certify `L(1, chi)` for a quadratic character through the class number
/// formula, using exact class groups, torsion, and regulators for small
/// quadratic fields.
#[cfg(feature = "numberfields")]
pub(crate) fn certified_quadratic_l_one(modulus: u64) -> Option<f64> {
    use crate::accelerators::numberfield::{NumberField, class_group, maximal_order, unit_group};
    use crate::algebra::poly_z::ZPoly;
    use crate::num::Int;

    let (discriminant, polynomial) = match modulus {
        3 => (-3i64, vec![3, 0, 1]),
        4 => (-4, vec![1, 0, 1]),
        5 => (5, vec![-5, 0, 1]),
        _ => return None,
    };
    let field = NumberField::new(ZPoly::new(
        polynomial.into_iter().map(Int::from_i64).collect(),
    ))
    .ok()?;
    let order = maximal_order(&field)?;
    let class = class_group(&order)?;
    let units = unit_group(&order)?;
    let class_number = class.class_number().to_i64()? as f64;
    let absolute = (discriminant.abs() as f64).sqrt();
    if discriminant < 0 {
        let torsion = units.torsion_order() as f64;
        Some(2.0 * core::f64::consts::PI * class_number / (torsion * absolute))
    } else {
        let regulator = units.regulator()?;
        Some(2.0 * class_number * regulator / absolute)
    }
}

/// The quadratic character modulo 4, the Dirichlet beta character.
pub(crate) fn beta_character() -> DirichletCharacter {
    DirichletCharacter::new(
        4,
        vec![
            Complex::zero(),
            Complex::one(),
            Complex::zero(),
            Complex::new(-1.0, 0.0),
        ],
    )
    .unwrap()
}

/// The quadratic character modulo 3.
pub(crate) fn character_mod_three() -> DirichletCharacter {
    DirichletCharacter::new(
        3,
        vec![Complex::zero(), Complex::one(), Complex::new(-1.0, 0.0)],
    )
    .unwrap()
}

fn primitive_functional(s: Complex, character: &DirichletCharacter) -> Option<Complex> {
    let parity = character.parity()?;
    let root = character.root_number()?;
    let pi = core::f64::consts::PI;
    let argument = Complex::new(pi * (s.re + parity as f64) / 2.0, pi * s.im / 2.0);
    let reflected = dirichlet_l(Complex::one().sub(s), &character.conjugate())?;
    Some(
        root.mul(power_real(2.0, s))
            .mul(power_real(pi, s.sub(Complex::one())))
            .mul(power_real(
                character.modulus() as f64,
                Complex::new(0.5, 0.0).sub(s),
            ))
            .mul(argument.sin())
            .mul(Complex::one().sub(s).gamma()?)
            .mul(reflected),
    )
}

fn gcd_u64(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn factorial(value: usize) -> f64 {
    let mut result = 1.0f64;
    for factor in 1..=value {
        result *= factor as f64;
    }
    result
}

fn argument(re: f64, im: f64) -> f64 {
    if re > 0.0 {
        (im / re).atan()
    } else if re < 0.0 && im >= 0.0 {
        (im / re).atan() + core::f64::consts::PI
    } else if re < 0.0 {
        (im / re).atan() - core::f64::consts::PI
    } else if im > 0.0 {
        core::f64::consts::PI / 2.0
    } else if im < 0.0 {
        -core::f64::consts::PI / 2.0
    } else {
        0.0
    }
}

fn power_real(base: f64, exponent: Complex) -> Complex {
    if base <= 0.0 {
        return Complex::zero();
    }
    exponent.mul(Complex::new(base.ln(), 0.0)).exp()
}

/// Refine a zero of a complex function by a finite difference Newton step.
pub(crate) fn refine_zero<F>(function: F, start: Complex) -> Option<Complex>
where
    F: Fn(Complex) -> Option<Complex>,
{
    let mut point = start;
    for _ in 0..40 {
        let value = function(point)?;
        if value.abs() < 1e-12 {
            return Some(point);
        }
        let step = 1e-6;
        let dx = function(Complex::new(point.re + step, point.im))?
            .sub(function(Complex::new(point.re - step, point.im))?);
        let dy = function(Complex::new(point.re, point.im + step))?
            .sub(function(Complex::new(point.re, point.im - step))?);
        let two_step = Complex::new(2.0 * step, 0.0);
        let dx = dx.div(two_step)?;
        let dy = dy.div(Complex::new(0.0, 2.0 * step))?;
        let determinant = dx.re * dy.im - dx.im * dy.re;
        if determinant.abs() < 1e-18 {
            return None;
        }
        let delta_re = (-value.re * dy.im + value.im * dy.re) / determinant;
        let delta_im = (-value.im * dx.re + value.re * dx.im) / determinant;
        point = Complex::new(point.re + delta_re, point.im + delta_im);
    }
    Some(point)
}

/// A numerical zero certificate from residual and argument-principle counts.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ZeroCertificate {
    /// The certified point.
    pub(crate) zero: Complex,
    /// The residual modulus.
    pub(crate) residual: f64,
    /// The zero count in the enclosing square.
    pub(crate) count: i64,
}

/// Numerically certify an isolated zero by the argument principle.
///
/// This verifies one zero in a small square at two boundary resolutions. It is
/// not a rigorous interval certificate.
pub(crate) fn certify_zero<F>(function: F, zero: Complex, half: f64) -> Option<ZeroCertificate>
where
    F: Fn(Complex) -> Option<Complex>,
{
    let residual = function(zero)?.abs();
    if residual > 1e-4 {
        return None;
    }
    let coarse = argument_principle_count(&function, zero, half, 256)?;
    let fine = argument_principle_count(&function, zero, half, 512)?;
    if coarse != 1 || fine != 1 {
        return None;
    }
    Some(ZeroCertificate {
        zero,
        residual,
        count: fine,
    })
}

fn argument_principle_count<F>(
    function: &F,
    center: Complex,
    half: f64,
    samples: usize,
) -> Option<i64>
where
    F: Fn(Complex) -> Option<Complex>,
{
    if samples < 4 || half <= 0.0 {
        return None;
    }
    let corners = [
        Complex::new(center.re - half, center.im - half),
        Complex::new(center.re + half, center.im - half),
        Complex::new(center.re + half, center.im + half),
        Complex::new(center.re - half, center.im + half),
    ];
    let mut total = 0.0f64;
    let mut previous = None;
    let evaluate = |point: Complex| -> Option<f64> {
        let value = function(point)?;
        Some(argument(value.re, value.im))
    };
    for edge in 0..4 {
        let start = corners[edge];
        let end = corners[(edge + 1) % 4];
        for step in 0..samples {
            let fraction = step as f64 / samples as f64;
            let point = Complex::new(
                start.re + (end.re - start.re) * fraction,
                start.im + (end.im - start.im) * fraction,
            );
            let angle = evaluate(point)?;
            if let Some(previous_angle) = previous {
                let mut difference = angle - previous_angle;
                while difference > core::f64::consts::PI {
                    difference -= 2.0 * core::f64::consts::PI;
                }
                while difference < -core::f64::consts::PI {
                    difference += 2.0 * core::f64::consts::PI;
                }
                total += difference;
            }
            previous = Some(angle);
        }
    }
    Some((total / (2.0 * core::f64::consts::PI)).round() as i64)
}

/// Scan the critical line for approximate zeros by minimizing the modulus.
///
/// This is a numerical experiment for functions expected to have zeros on
/// `Re(s) = 1/2`. It does not certify that all zeros were found.
pub(crate) fn critical_line_zeros<F>(
    function: F,
    tmin: f64,
    tmax: f64,
    steps: usize,
) -> Vec<Complex>
where
    F: Fn(Complex) -> Option<Complex>,
{
    if steps < 2 || tmax <= tmin {
        return Vec::new();
    }
    let dt = (tmax - tmin) / steps as f64;
    let mut values = Vec::with_capacity(steps + 1);
    for index in 0..=steps {
        let t = tmin + index as f64 * dt;
        values.push(
            function(Complex::new(0.5, t))
                .map(|value| value.abs())
                .unwrap_or(f64::INFINITY),
        );
    }
    let mut zeros = Vec::new();
    for index in 1..steps {
        if values[index] > values[index - 1] || values[index] > values[index + 1] {
            continue;
        }
        let mut left = tmin + (index - 1) as f64 * dt;
        let mut right = tmin + (index + 1) as f64 * dt;
        let gold = 0.618_033_988_749_895;
        for _ in 0..80 {
            let first = right - gold * (right - left);
            let second = left + gold * (right - left);
            let first_value = function(Complex::new(0.5, first))
                .map(|value| value.abs())
                .unwrap_or(f64::INFINITY);
            let second_value = function(Complex::new(0.5, second))
                .map(|value| value.abs())
                .unwrap_or(f64::INFINITY);
            if first_value < second_value {
                right = second;
            } else {
                left = first;
            }
        }
        let t = (left + right) / 2.0;
        let magnitude = function(Complex::new(0.5, t))
            .map(|value| value.abs())
            .unwrap_or(f64::INFINITY);
        if magnitude < 1e-4 {
            zeros.push(Complex::new(0.5, t));
        }
    }
    zeros
}

/// Find approximate zeros in a box by grid minima and Newton refinement.
///
/// This is a numerical experiment and does not certify zero counts.
#[allow(clippy::needless_range_loop)]
pub(crate) fn find_zeros<F>(
    function: F,
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    steps: usize,
) -> Vec<Complex>
where
    F: Fn(Complex) -> Option<Complex>,
{
    if steps < 3 || xmax <= xmin || ymax <= ymin {
        return Vec::new();
    }
    let dx = (xmax - xmin) / steps as f64;
    let dy = (ymax - ymin) / steps as f64;
    let mut magnitudes = vec![vec![f64::INFINITY; steps + 1]; steps + 1];
    for i in 0..=steps {
        for j in 0..=steps {
            let point = Complex::new(xmin + i as f64 * dx, ymin + j as f64 * dy);
            if let Some(value) = function(point) {
                magnitudes[i][j] = value.abs();
            }
        }
    }
    let mut zeros = Vec::new();
    for i in 1..steps {
        for j in 1..steps {
            let value = magnitudes[i][j];
            if value > magnitudes[i - 1][j]
                || value > magnitudes[i + 1][j]
                || value > magnitudes[i][j - 1]
                || value > magnitudes[i][j + 1]
            {
                continue;
            }
            let start = Complex::new(xmin + i as f64 * dx, ymin + j as f64 * dy);
            if let Some(root) = refine_zero(&function, start)
                && root.re >= xmin
                && root.re <= xmax
                && root.im >= ymin
                && root.im <= ymax
                && root.im >= 0.0
                && zeros
                    .iter()
                    .all(|existing: &Complex| (existing.sub(root)).abs() > 1e-5)
            {
                zeros.push(root);
            }
        }
    }
    zeros.sort_by(|left, right| {
        left.im
            .partial_cmp(&right.im)
            .unwrap_or(core::cmp::Ordering::Equal)
    });
    zeros
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational(value: f64) -> Complex {
        Complex::new(value, 0.0)
    }

    #[test]
    fn zeta_values_match_known_constants() {
        let pi = core::f64::consts::PI;
        let zeta_two = zeta(rational(2.0)).unwrap();
        println!("zeta2 {:?} expected {}", zeta_two, pi * pi / 6.0);
        assert!((zeta_two.re - pi * pi / 6.0).abs() < 1e-10);
        let zeta_zero = zeta(rational(0.0)).unwrap();
        assert!((zeta_zero.re + 0.5).abs() < 1e-10);
        let zeta_negative = zeta(rational(-1.0)).unwrap();
        assert!((zeta_negative.re + 1.0 / 12.0).abs() < 1e-10);
        assert!(zeta(rational(-2.0)).unwrap().abs() < 1e-12);
    }

    #[test]
    fn dirichlet_beta_and_mod_three_values() {
        let beta_two = dirichlet_l(rational(2.0), &beta_character()).unwrap();
        assert!((beta_two.re - 0.915_965_594_177_219).abs() < 1e-9);
        let modulo_three = dirichlet_l(rational(2.0), &character_mod_three()).unwrap();
        assert!((modulo_three.re - 0.781_302_412_896_486).abs() < 1e-9);
        let beta_one = dirichlet_l(rational(1.0), &beta_character()).unwrap();
        let pi = core::f64::consts::PI;
        assert!((beta_one.re - pi / 4.0).abs() < 1e-4, "{beta_one:?}");
        let three_one = dirichlet_l(rational(1.0), &character_mod_three()).unwrap();
        assert!(
            (three_one.re - pi / (3.0 * 3.0f64.sqrt())).abs() < 1e-4,
            "{three_one:?}"
        );
        let beta_negative_two = dirichlet_l(rational(-2.0), &beta_character()).unwrap();
        assert!(
            (beta_negative_two.re + 0.5).abs() < 1e-8,
            "{beta_negative_two:?}"
        );
    }

    #[test]
    fn conductor_reduction_removes_induced_euler_factors() {
        let base = character_mod_three();
        let mut values = Vec::with_capacity(12);
        for residue in 0..12u64 {
            values.push(if gcd_u64(residue, 12) == 1 {
                base.value(residue % 3)
            } else {
                Complex::zero()
            });
        }
        let induced = DirichletCharacter::new(12, values).unwrap();
        assert_eq!(induced.conductor(), Some(3));
        let (primitive, original) = induced.primitive_reduction().unwrap();
        assert_eq!(original, 12);
        assert_eq!(primitive.modulus(), 3);
        let direct = dirichlet_l(rational(2.0), &induced).unwrap();
        let reduced = dirichlet_l(rational(2.0), &primitive).unwrap();
        let factor =
            Complex::one().sub(primitive.value(2).mul(power_real(2.0, rational(2.0).neg())));
        let expected = reduced.mul(factor);
        assert!(
            (direct.re - expected.re).abs() < 1e-9,
            "{direct:?} {expected:?}"
        );
    }

    #[cfg(feature = "numberfields")]
    #[test]
    fn quadratic_l_one_certificates_match_numerical_values() {
        for modulus in [3u64, 4, 5] {
            let certified = certified_quadratic_l_one(modulus).unwrap();
            let character = match modulus {
                3 => character_mod_three(),
                4 => beta_character(),
                _ => DirichletCharacter::new(
                    5,
                    vec![
                        Complex::zero(),
                        Complex::one(),
                        Complex::new(-1.0, 0.0),
                        Complex::new(-1.0, 0.0),
                        Complex::one(),
                    ],
                )
                .unwrap(),
            };
            let numerical = dirichlet_l(rational(1.0), &character).unwrap().re;
            assert!(
                (certified - numerical).abs() < 1e-4,
                "{modulus} {certified} {numerical}"
            );
        }
    }

    #[test]
    fn first_nontrivial_zeta_zero_is_found() {
        let zeros = critical_line_zeros(zeta, 10.0, 18.0, 32);
        assert!(!zeros.is_empty());
        let first = zeros
            .iter()
            .min_by(|left, right| left.im.partial_cmp(&right.im).unwrap())
            .unwrap();
        let certificate = certify_zero(zeta, *first, 0.25).unwrap();
        assert_eq!(certificate.count, 1);
        assert!((first.re - 0.5).abs() < 1e-4, "{first:?}");
        assert!((first.im - 14.134_725).abs() < 1e-3, "{first:?}");
    }
}
