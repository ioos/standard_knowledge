"""Compare the Rust fx evaluator with ioos_qc's fx_parser, its reference.

Every input is evaluated by both. They must agree on the value (bit for
bit, including the sign of zero) or on the kind of error. The only accepted
differences are the ones listed under "Known differences" in
core/src/fx/GRAMMAR.md, and each mismatch is checked against that list by
re-running ioos_qc with those cases turned into errors.
"""

import math
import warnings
from contextlib import contextmanager

import pytest
from hypothesis import example, given, settings
from hypothesis import strategies as st
from standard_knowledge import eval_fx

fx_parser = pytest.importorskip("ioos_qc.config_creator.fx_parser")
pyparsing = pytest.importorskip("pyparsing")

STAT_NAMES = ["min", "max", "mean", "std"]
FIXED_STATS = {"min": 1.0, "max": 9.0, "mean": 5.0, "std": 2.0}


def _bounded(power):
    """`power`, raising early for integer results far past the float range

    Python computes `int ** int` exactly, which can take minutes for
    generated inputs like `trunc(9)^trunc(9)^trunc(9)`. Such a result can't
    become a float, so the comparison would see an OverflowError anyway.
    """

    def wrapper(a, b):
        if (
            isinstance(a, int)
            and isinstance(b, int)
            and b > 0
            and abs(a) > 1
            and b * math.log2(abs(a)) > 1100
        ):
            raise OverflowError("integer result too large for a float")
        return power(a, b)

    return wrapper


def reference(fx, stats):
    saved_pow = fx_parser.opn["^"]
    fx_parser.opn["^"] = _bounded(saved_pow)
    try:
        with warnings.catch_warnings():
            # pyparsing may warn about the pre-PEP 8 names ioos_qc uses
            warnings.simplefilter("ignore", DeprecationWarning)
            return fx_parser.eval_fx(fx, stats)
    finally:
        fx_parser.opn["^"] = saved_pow


def error_kind(error):
    """The Python exception the Rust binding raises for the same failure"""
    if isinstance(error, pyparsing.ParseBaseException):
        return "ValueError"
    for kind in (ZeroDivisionError, OverflowError, KeyError, TypeError, ValueError):
        if isinstance(error, kind):
            return kind.__name__
    if type(error) is Exception:
        # ioos_qc raises a bare Exception for an invalid identifier
        return "ValueError"
    raise error


def outcome(func, fx, stats):
    try:
        value = func(fx, stats)
    except RecursionError:
        raise
    except Exception as error:  # noqa: BLE001 - every error is part of the comparison
        return ("error", error_kind(error))
    if isinstance(value, complex):
        return ("complex", value)
    try:
        return ("ok", float(value))
    except OverflowError:
        # A Python integer too large for a float can't be a threshold
        return ("error", "OverflowError")


def same_value(a, b):
    if math.isnan(a) or math.isnan(b):
        return math.isnan(a) and math.isnan(b)
    return a == b and math.copysign(1.0, a) == math.copysign(1.0, b)


def agree(expected, actual):
    if expected[0] != actual[0]:
        return False
    if expected[0] == "ok":
        return same_value(expected[1], actual[1])
    return expected == actual


class KnownDifference(Exception):
    pass


def _strict(func):
    def wrapper(*args):
        result = func(*args)
        if isinstance(result, complex):
            raise KnownDifference("complex")
        if isinstance(result, int) and abs(result) > 2**53:
            raise KnownDifference("integer")
        return result

    return wrapper


@contextmanager
def strict_reference():
    """ioos_qc with complex results and inexact integers raising instead"""
    saved_opn, saved_fn = dict(fx_parser.opn), dict(fx_parser.fn)
    fx_parser.opn.update({k: _strict(v) for k, v in saved_opn.items()})
    fx_parser.fn.update({k: _strict(v) for k, v in saved_fn.items()})
    try:
        yield
    finally:
        fx_parser.opn.update(saved_opn)
        fx_parser.fn.update(saved_fn)


def check(fx, stats):
    expected = outcome(reference, fx, stats)
    actual = outcome(eval_fx, fx, stats)
    if agree(expected, actual):
        return

    with strict_reference():
        try:
            reference(fx, stats)
            difference = None
        except KnownDifference as known:
            difference = str(known)
        except Exception:  # noqa: BLE001 - only KnownDifference matters here
            difference = None

    if difference == "complex":
        # The Rust side stops where Python would create a complex number
        assert actual == ("error", "ValueError"), (fx, stats, expected, actual)
    elif difference == "integer":
        # Python integers are exact beyond 2^53; floats are not
        pass
    else:
        pytest.fail(
            f"{fx!r} with {stats!r}: ioos_qc gave {expected}, Rust gave {actual}"
        )


# Inputs worth checking by name: ioos_qc's examples and every corner the
# grammar documentation describes.
CASES = [
    "3",
    "( 1 + 2 )",
    "3 * 2 - 6",
    "3 * mean + std / ( max * min )",
    "min - std * 2",
    "max + std / 2",
    "mean * std",
    "mean / std",
    "mean+3*std",
    " 1 ",
    "1\t+\n2",
    "1.",
    "1e5",
    "1.e-3",
    "007",
    "1e999",
    "1e999-1e999",
    "",
    "3 -",
    ".5",
    "1e",
    "1e+",
    "2pi",
    "--3",
    "+-3",
    "-+-3",
    "- (3)",
    "-(-3)",
    "-2^2",
    "2^-2",
    "2^3^2",
    "2^-3^2",
    "Pi",
    "pi",
    "e",
    "E1",
    "MEAN",
    "kurtosis * 2",
    "sin",
    "pi(2)",
    "PI(1/0)",
    "PI(5) + 2",
    "2 + PI(5)",
    "2 + mean(9)",
    "E(1, 2) * 3",
    "abs(-2)",
    "abs(trunc(-2))",
    "sgn(-3)",
    "sgn(1e-13)",
    "trunc(7.9)/trunc(2)",
    "trunc(-0.5)",
    "-trunc(0)",
    "round(2.5)",
    "round(3.5)",
    "round(-2.5)",
    "round(2.675, trunc(2))",
    "round(0.125, trunc(2))",
    "round(1234.5, trunc(-2))",
    "round(-0.4, trunc(0))",
    "round(trunc(25), trunc(-1))",
    "round(trunc(-25), trunc(-1))",
    "round(trunc(35), trunc(-1))",
    "round(1e308, trunc(-308))",
    "round(1.7976931348623157e308, trunc(-308))",
    "round(5e-324, trunc(400))",
    "round(2.5, 1)",
    "round(1, 2, 3)",
    "sin(0)",
    "sin(1, 2)",
    "sin()",
    "foo(1)",
    "exp(0)",
    "exp(1000)",
    "exp(-1000)",
    "exp(1e999)",
    "sin(1e999)",
    "trunc(1e999)",
    "trunc(1e999 - 1e999)",
    "round(1e999)",
    "1/0",
    "0^-1",
    "trunc(0)^trunc(-1)",
    "0^0",
    "(1e999-1e999)^0",
    "1^(1e999-1e999)",
    "(-1e999)^3",
    "(-1e999)^-3",
    "(-1e999)^0.5",
    "(-2)^1e999",
    "(-0.5)^1e999",
    "2^1024",
    "trunc(2)^trunc(10)",
    "trunc(10)^trunc(400)",
    "trunc(9)^trunc(9)^trunc(9)",
    "(-8)^(1/3)",
    "abs((-8)^(1/3))",
    "1e308 * 10",
]


@pytest.mark.parametrize("fx", CASES)
def test_named_cases(fx):
    check(fx, FIXED_STATS)


@pytest.mark.parametrize(
    "stats",
    [
        {"mean": 0.0, "std": 0.0, "min": 0.0, "max": 0.0},
        {"mean": -0.0, "std": float("inf"), "min": float("-inf"), "max": float("nan")},
        {"mean": 1.0},
        {},
    ],
)
@pytest.mark.parametrize(
    "fx", ["mean / std", "min - std * 2", "-mean", "std^0", "max", "sgn(max)"]
)
def test_unusual_stats(fx, stats):
    check(fx, stats)


# Hypothesis strategies. Expressions are built from the grammar so that most
# of them parse, with words, functions and spacing chosen to reach every
# branch of the evaluator, including the error paths.

space = st.sampled_from(["", "", " ", "  ", "\t", "\n"])

number = st.one_of(
    st.integers(min_value=0, max_value=10**20).map(str),
    st.floats(min_value=0, allow_nan=False, allow_infinity=False).map(repr),
    st.from_regex(r"\A[0-9]{1,3}(\.[0-9]{0,3})?([eE][+-]?[0-9]{1,3})?\Z"),
)

word = st.sampled_from(
    [
        *STAT_NAMES,
        "PI",
        "pi",
        "Pi",
        "E",
        "e",
        "MEAN",
        "foo",
        "sin",
        "exp",
        "trunc",
        "x$",
    ]
)

function = st.sampled_from(
    [
        "sin",
        "cos",
        "tan",
        "exp",
        "abs",
        "trunc",
        "round",
        "sgn",
        "PI",
        "E",
        "pi",
        "foo",
        *STAT_NAMES,
    ]
)

signs = st.text(alphabet="+-", max_size=3)


def extend(children):
    spaced = st.tuples(space, children, space).map("".join)
    return st.one_of(
        st.tuples(spaced, st.sampled_from("+-*/^"), spaced).map("".join),
        st.tuples(signs, space, children).map("".join),
        spaced.map(lambda c: f"({c})"),
        st.tuples(function, space, st.lists(spaced, min_size=1, max_size=3)).map(
            lambda t: f"{t[0]}{t[1]}({','.join(t[2])})"
        ),
    )


expressions = st.recursive(st.one_of(number, word), extend, max_leaves=10)

stat_value = st.one_of(
    st.floats(),
    st.floats(min_value=-1e3, max_value=1e3),
    st.integers(min_value=-5, max_value=5).map(float),
)
stats = st.one_of(
    st.fixed_dictionaries(dict.fromkeys(STAT_NAMES, stat_value)),
    st.dictionaries(st.sampled_from(STAT_NAMES), stat_value),
)


@settings(max_examples=2000, deadline=None)
@given(fx=expressions, stats=stats)
@example(fx="2 + PI(5)", stats=FIXED_STATS)
@example(fx="round(2.675, trunc(2))", stats=FIXED_STATS)
def test_generated_expressions(fx, stats):
    check(fx, stats)


# Raw text over the language's alphabet, mostly invalid, to check that both
# parsers accept and reject the same strings.
alphabet = " \t\n0123456789.eE+-*/^(),abcdeimnorstuxIPS_$"


@settings(max_examples=2000, deadline=None)
@given(fx=st.text(alphabet=alphabet, max_size=24))
def test_raw_text(fx):
    check(fx, FIXED_STATS)
