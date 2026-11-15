//! Raw GMP declarations.
//!
//! This module is the only place that names GMP symbols directly. The layout
//! follows `gmp.h` on Linux for the x86 64 ABI.

use core::ffi::{c_char, c_int, c_long, c_ulong};

/// The GMP integer structure. `mpz_t` is an array of one of these.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Mpz {
    pub(crate) alloc: c_int,
    pub(crate) size: c_int,
    pub(crate) limbs: *mut c_ulong,
}

/// The GMP rational structure. `mpq_t` is an array of one of these.
#[repr(C)]
#[derive(Debug)]
pub(crate) struct Mpq {
    pub(crate) num: Mpz,
    pub(crate) den: Mpz,
}

unsafe extern "C" {
    pub(crate) fn __gmpz_init(value: *mut Mpz);
    pub(crate) fn __gmpz_clear(value: *mut Mpz);
    pub(crate) fn __gmpz_set(rop: *mut Mpz, op: *const Mpz);
    pub(crate) fn __gmpz_set_si(rop: *mut Mpz, op: c_long);
    pub(crate) fn __gmpz_set_ui(rop: *mut Mpz, op: c_ulong);
    pub(crate) fn __gmpz_set_str(rop: *mut Mpz, text: *const c_char, base: c_int) -> c_int;
    pub(crate) fn __gmpz_get_str(text: *mut c_char, base: c_int, op: *const Mpz) -> *mut c_char;
    pub(crate) fn __gmpz_add(rop: *mut Mpz, left: *const Mpz, right: *const Mpz);
    pub(crate) fn __gmpz_sub(rop: *mut Mpz, left: *const Mpz, right: *const Mpz);
    pub(crate) fn __gmpz_mul(rop: *mut Mpz, left: *const Mpz, right: *const Mpz);
    pub(crate) fn __gmpz_neg(rop: *mut Mpz, op: *const Mpz);
    pub(crate) fn __gmpz_abs(rop: *mut Mpz, op: *const Mpz);
    pub(crate) fn __gmpz_cmp(left: *const Mpz, right: *const Mpz) -> c_int;
    pub(crate) fn __gmpz_cmp_si(left: *const Mpz, right: c_long) -> c_int;
    pub(crate) fn __gmpz_cmp_ui(left: *const Mpz, right: c_ulong) -> c_int;
    pub(crate) fn __gmpz_tdiv_qr(
        quotient: *mut Mpz,
        remainder: *mut Mpz,
        dividend: *const Mpz,
        divisor: *const Mpz,
    );
    pub(crate) fn __gmpz_gcd(rop: *mut Mpz, left: *const Mpz, right: *const Mpz);
    pub(crate) fn __gmpz_pow_ui(rop: *mut Mpz, base: *const Mpz, exponent: c_ulong);
    pub(crate) fn __gmpz_sizeinbase(op: *const Mpz, base: c_int) -> usize;
    pub(crate) fn __gmpz_fits_slong_p(op: *const Mpz) -> c_int;
    pub(crate) fn __gmpz_get_si(op: *const Mpz) -> c_long;

    pub(crate) fn __gmpq_init(value: *mut Mpq);
    pub(crate) fn __gmpq_clear(value: *mut Mpq);
    pub(crate) fn __gmpq_set(rop: *mut Mpq, op: *const Mpq);
    pub(crate) fn __gmpq_set_str(rop: *mut Mpq, text: *const c_char, base: c_int) -> c_int;
    pub(crate) fn __gmpq_get_str(text: *mut c_char, base: c_int, op: *const Mpq) -> *mut c_char;
    pub(crate) fn __gmpq_canonicalize(value: *mut Mpq);
    pub(crate) fn __gmpq_add(rop: *mut Mpq, left: *const Mpq, right: *const Mpq);
    pub(crate) fn __gmpq_sub(rop: *mut Mpq, left: *const Mpq, right: *const Mpq);
    pub(crate) fn __gmpq_mul(rop: *mut Mpq, left: *const Mpq, right: *const Mpq);
    pub(crate) fn __gmpq_div(rop: *mut Mpq, left: *const Mpq, right: *const Mpq);
    pub(crate) fn __gmpq_neg(rop: *mut Mpq, op: *const Mpq);
    pub(crate) fn __gmpq_cmp(left: *const Mpq, right: *const Mpq) -> c_int;
    pub(crate) fn __gmpq_equal(left: *const Mpq, right: *const Mpq) -> c_int;
    pub(crate) fn __gmpq_get_d(op: *const Mpq) -> f64;
}
