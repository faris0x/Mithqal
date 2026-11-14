//! Raw MPFR declarations.

use core::ffi::{c_char, c_int, c_long, c_ulong};

use super::ffi::Mpq;

/// The MPFR floating point structure.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Mpfr {
    pub(crate) precision: c_long,
    pub(crate) sign: c_int,
    pub(crate) exponent: c_long,
    pub(crate) limbs: *mut c_ulong,
}

pub(crate) const MPFR_RNDN: c_int = 0;
pub(crate) const MPFR_RNDU: c_int = 2;
pub(crate) const MPFR_RNDD: c_int = 3;

unsafe extern "C" {
    pub(crate) fn mpfr_init2(value: *mut Mpfr, precision: c_long);
    pub(crate) fn mpfr_clear(value: *mut Mpfr);
    pub(crate) fn mpfr_set(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_set_ui(rop: *mut Mpfr, op: c_ulong, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_set_si(rop: *mut Mpfr, op: c_long, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_set_d(rop: *mut Mpfr, op: f64, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_set_q(rop: *mut Mpfr, op: *const Mpq, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_get_d(op: *const Mpfr, rnd: c_int) -> f64;
    pub(crate) fn mpfr_get_str(
        text: *mut c_char,
        exponent: *mut c_long,
        base: c_int,
        digits: usize,
        op: *const Mpfr,
        rnd: c_int,
    ) -> *mut c_char;
    pub(crate) fn mpfr_add(
        rop: *mut Mpfr,
        left: *const Mpfr,
        right: *const Mpfr,
        rnd: c_int,
    ) -> c_int;
    pub(crate) fn mpfr_sub(
        rop: *mut Mpfr,
        left: *const Mpfr,
        right: *const Mpfr,
        rnd: c_int,
    ) -> c_int;
    pub(crate) fn mpfr_mul(
        rop: *mut Mpfr,
        left: *const Mpfr,
        right: *const Mpfr,
        rnd: c_int,
    ) -> c_int;
    pub(crate) fn mpfr_div(
        rop: *mut Mpfr,
        left: *const Mpfr,
        right: *const Mpfr,
        rnd: c_int,
    ) -> c_int;
    pub(crate) fn mpfr_neg(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_abs(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_sqrt(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_pow_si(
        rop: *mut Mpfr,
        base: *const Mpfr,
        exponent: c_long,
        rnd: c_int,
    ) -> c_int;
    pub(crate) fn mpfr_pow(
        rop: *mut Mpfr,
        base: *const Mpfr,
        exponent: *const Mpfr,
        rnd: c_int,
    ) -> c_int;
    pub(crate) fn mpfr_exp(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_log(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_sin(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_cos(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_tan(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_asin(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_acos(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_atan(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_sinh(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_cosh(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_tanh(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_asinh(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_acosh(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_atanh(rop: *mut Mpfr, op: *const Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_floor(rop: *mut Mpfr, op: *const Mpfr) -> c_int;
    pub(crate) fn mpfr_ceil(rop: *mut Mpfr, op: *const Mpfr) -> c_int;
    pub(crate) fn mpfr_round(rop: *mut Mpfr, op: *const Mpfr) -> c_int;
    pub(crate) fn mpfr_const_pi(rop: *mut Mpfr, rnd: c_int) -> c_int;
    pub(crate) fn mpfr_cmp(left: *const Mpfr, right: *const Mpfr) -> c_int;
    pub(crate) fn mpfr_cmp_ui(left: *const Mpfr, right: c_ulong) -> c_int;
    pub(crate) fn mpfr_cmpabs_ui(left: *const Mpfr, right: c_ulong) -> c_int;
    pub(crate) fn mpfr_nan_p(op: *const Mpfr) -> c_int;
    pub(crate) fn mpfr_inf_p(op: *const Mpfr) -> c_int;
}
