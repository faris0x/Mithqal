#!/usr/bin/env python3
"""Generate high-precision numeric fixtures with mpmath.

The generated file is committed so the Rust test suite does not require
mpmath. Regenerate with:

    python3 -m venv /tmp/mpmath-venv
    /tmp/mpmath-venv/bin/pip install mpmath
    /tmp/mpmath-venv/bin/python tools/generate_mpmath_fixtures.py

This script is developer tooling and is not part of the crate build.
"""

from pathlib import Path

import mpmath as mp

ROOT = Path(__file__).resolve().parents[1]
DIGITS = 50

mp.mp.dps = DIGITS + 20


def number_text(value):
    value = mp.mpf(value)
    if value == 0:
        return "0"
    return mp.nstr(value, DIGITS, strip_zeros=False)


def complex_text(value):
    value = mp.mpc(value)
    return number_text(value.real), number_text(value.imag)


def real(name, function, arguments):
    return (name, "real", arguments, *complex_text(function(*arguments)))


def complex_case(name, function, arguments):
    return (name, "complex", arguments, *complex_text(function(*arguments)))


records = []

real_cases = [
    ("sqrt", mp.sqrt, ["2"]),
    ("sqrt", mp.sqrt, ["1/3"]),
    ("exp", mp.exp, ["1"]),
    ("exp", mp.exp, ["-3"]),
    ("exp", mp.exp, ["1/2"]),
    ("ln", mp.log, ["2"]),
    ("ln", mp.log, ["1/3"]),
    ("ln", mp.log, ["10"]),
    ("sin", mp.sin, ["1"]),
    ("sin", mp.sin, ["-2"]),
    ("sin", mp.sin, ["1/3"]),
    ("cos", mp.cos, ["1"]),
    ("cos", mp.cos, ["-2"]),
    ("cos", mp.cos, ["1/3"]),
    ("tan", mp.tan, ["1"]),
    ("tan", mp.tan, ["-1/2"]),
    ("tan", mp.tan, ["2"]),
    ("asin", mp.asin, ["1/2"]),
    ("asin", mp.asin, ["-3/4"]),
    ("asin", mp.asin, ["9/10"]),
    ("acos", mp.acos, ["1/2"]),
    ("acos", mp.acos, ["-3/4"]),
    ("acos", mp.acos, ["9/10"]),
    ("atan", mp.atan, ["1"]),
    ("atan", mp.atan, ["-5"]),
    ("atan", mp.atan, ["1/3"]),
    ("sinh", mp.sinh, ["1"]),
    ("sinh", mp.sinh, ["-2"]),
    ("sinh", mp.sinh, ["1/3"]),
    ("cosh", mp.cosh, ["1"]),
    ("cosh", mp.cosh, ["-2"]),
    ("cosh", mp.cosh, ["1/3"]),
    ("tanh", mp.tanh, ["1"]),
    ("tanh", mp.tanh, ["-2"]),
    ("tanh", mp.tanh, ["1/3"]),
    ("asinh", mp.asinh, ["2"]),
    ("asinh", mp.asinh, ["-3"]),
    ("asinh", mp.asinh, ["1/2"]),
    ("acosh", mp.acosh, ["2"]),
    ("acosh", mp.acosh, ["5"]),
    ("acosh", mp.acosh, ["3/2"]),
    ("atanh", mp.atanh, ["1/2"]),
    ("atanh", mp.atanh, ["-3/4"]),
    ("atanh", mp.atanh, ["9/10"]),
    ("gamma", mp.gamma, ["1/3"]),
    ("gamma", mp.gamma, ["7/3"]),
    ("gamma", mp.gamma, ["-5/2"]),
    ("gamma", mp.gamma, ["-1/3"]),
    ("gamma", mp.gamma, ["15"]),
    ("gamma", mp.gamma, ["-1/10"]),
    ("gamma", mp.gamma, ["-1/5"]),
    ("gamma", mp.gamma, ["-1/4"]),
    ("gamma", mp.gamma, ["-1/2"]),
    ("gamma", mp.gamma, ["-4/5"]),
    ("gamma", mp.gamma, ["-9/10"]),
    ("lgamma", mp.loggamma, ["1/3"]),
    ("lgamma", mp.loggamma, ["7/2"]),
    ("lgamma", mp.loggamma, ["20"]),
    ("lgamma", lambda z: mp.log(abs(mp.gamma(z))), ["-1/3"]),
    ("lgamma", lambda z: mp.log(abs(mp.gamma(z))), ["-5/2"]),
    ("erf", mp.erf, ["1"]),
    ("erf", mp.erf, ["-2"]),
    ("erf", mp.erf, ["1/3"]),
    ("erf", mp.erf, ["3"]),
    ("erfc", mp.erfc, ["1"]),
    ("erfc", mp.erfc, ["-2"]),
    ("erfc", mp.erfc, ["1/3"]),
    ("erfc", mp.erfc, ["3"]),
    ("erfi", mp.erfi, ["1"]),
    ("erfi", mp.erfi, ["-2"]),
    ("erfi", mp.erfi, ["1/3"]),
    ("erfi", mp.erfi, ["3"]),
]

for name, function, arguments in real_cases:
    records.append(real(name, function, [mp.mpf(argument) for argument in arguments]))

complex_cases = [
    ("sqrt", mp.sqrt, [mp.mpc(1, 2)]),
    ("sqrt", mp.sqrt, [mp.mpc(-3, -1)]),
    ("exp", mp.exp, [mp.mpc(1, 2)]),
    ("exp", mp.exp, [mp.mpc(-2, 1)]),
    ("ln", mp.log, [mp.mpc(1, 2)]),
    ("ln", mp.log, [mp.mpc(-3, -1)]),
    ("sin", mp.sin, [mp.mpc(1, 2)]),
    ("sin", mp.sin, [mp.mpc(-2, 1)]),
    ("cos", mp.cos, [mp.mpc(1, 2)]),
    ("cos", mp.cos, [mp.mpc(-2, 1)]),
    ("tan", mp.tan, [mp.mpc(1, 2)]),
    ("tan", mp.tan, [mp.mpc(-1, 1)]),
    ("asin", mp.asin, [mp.mpc(2, 1)]),
    ("asin", mp.asin, [mp.mpc(-2, -1)]),
    ("acos", mp.acos, [mp.mpc(2, 1)]),
    ("acos", mp.acos, [mp.mpc(-2, -1)]),
    ("atan", mp.atan, [mp.mpc(2, 1)]),
    ("atan", mp.atan, [mp.mpc(-2, -1)]),
    ("sinh", mp.sinh, [mp.mpc(1, 2)]),
    ("sinh", mp.sinh, [mp.mpc(-2, 1)]),
    ("cosh", mp.cosh, [mp.mpc(1, 2)]),
    ("cosh", mp.cosh, [mp.mpc(-2, 1)]),
    ("tanh", mp.tanh, [mp.mpc(1, 2)]),
    ("tanh", mp.tanh, [mp.mpc(-1, 1)]),
    ("asinh", mp.asinh, [mp.mpc(2, 1)]),
    ("asinh", mp.asinh, [mp.mpc(-2, -1)]),
    ("acosh", mp.acosh, [mp.mpc(2, 1)]),
    ("acosh", mp.acosh, [mp.mpc(-2, -1)]),
    ("acosh", mp.acosh, [mp.mpc(-2, 1)]),
    ("acosh", mp.acosh, [mp.mpc(-1, 2)]),
    ("acosh", mp.acosh, [mp.mpc(2, -1)]),
    ("acosh", mp.acosh, [mp.mpc(-1, -2)]),
    ("atanh", mp.atanh, [mp.mpc(1, 2)]),
    ("atanh", mp.atanh, [mp.mpc(-2, 1)]),
    ("gamma", mp.gamma, [mp.mpc(2, 3)]),
    ("gamma", mp.gamma, [mp.mpc(-1, 2)]),
    ("lgamma", lambda z: mp.log(mp.gamma(z)), [mp.mpc(2, 3)]),
    ("lgamma", lambda z: mp.log(mp.gamma(z)), [mp.mpc(-1, 2)]),
    ("erf", mp.erf, [mp.mpc(1, 1)]),
    ("erf", mp.erf, [mp.mpc(-2, 1)]),
    ("erfc", mp.erfc, [mp.mpc(1, 1)]),
    ("erfc", mp.erfc, [mp.mpc(-2, 1)]),
    ("erfi", mp.erfi, [mp.mpc(1, 1)]),
    ("erfi", mp.erfi, [mp.mpc(-2, 1)]),
]

for name, function, arguments in complex_cases:
    records.append(complex_case(name, function, arguments))


def argument_text(kind, value):
    if kind == "real":
        return number_text(value)
    return f"complex({number_text(value.real)},{number_text(value.imag)})"


lines = [
    "# Generated by tools/generate_mpmath_fixtures.py with mpmath " + mp.__version__,
    "# Fields are tab separated: name, kind, argument, real part, imaginary part.",
    "# Values are rounded to 50 significant digits. Comments and blank lines are ignored.",
]
for name, kind, arguments, real_part, imaginary_part in records:
    argument = ", ".join(argument_text(kind, value) for value in arguments)
    lines.append(f"{name}\t{kind}\t{argument}\t{real_part}\t{imaginary_part}")

(ROOT / "tests/fixtures/differential/numeric.tsv").write_text("\n".join(lines) + "\n")
print(f"wrote {len(records)} numeric fixtures")
