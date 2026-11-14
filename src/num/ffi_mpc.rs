//! Raw MPC declarations.

use core::ffi::{c_int, c_long};

use super::ffi_mpfr::Mpfr;

/// The MPC complex structure. `mpc_t` is an array of one of these.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Mpc {
    pub(crate) real: Mpfr,
    pub(crate) imaginary: Mpfr,
}

unsafe extern "C" {
    pub(crate) fn mpc_init2(value: *mut Mpc, precision: c_long);
    pub(crate) fn mpc_clear(value: *mut Mpc);
    pub(crate) fn mpc_set_fr_fr(
        rop: *mut Mpc,
        real: *const Mpfr,
        imaginary: *const Mpfr,
        rnd: c_int,
    ) -> c_int;
    pub(crate) fn mpc_exp(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_log(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_sin(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_cos(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_tan(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_sqrt(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_asin(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_acos(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_atan(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_sinh(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_cosh(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_tanh(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_asinh(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_acosh(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_atanh(rop: *mut Mpc, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_abs(rop: *mut Mpfr, op: *const Mpc, rnd: c_int) -> c_int;
    pub(crate) fn mpc_pow(
        rop: *mut Mpc,
        base: *const Mpc,
        exponent: *const Mpc,
        rnd: c_int,
    ) -> c_int;
}
