"""Tests for the fx threshold expression evaluator.

These run everywhere, including the pyodide wheel. The comparison against
ioos_qc's own parser is in test_fx_ioos_qc.py.
"""

import math

import pytest
from hypothesis import given, settings
from hypothesis import strategies as st
from standard_knowledge import eval_fx

STATS = {"min": 1.0, "max": 9.0, "mean": 5.0, "std": 2.0}


@pytest.mark.parametrize(
    ("fx", "expected"),
    [
        # The examples from ioos_qc's usage docs and test data
        ("3", 3.0),
        ("( 1 + 2 )", 3.0),
        ("3 * 2 - 6", 0.0),
        ("3 * mean + std / ( max * min )", 15.0 + 2.0 / 9.0),
        ("min - std * 2", -3.0),
        ("max + std / 2", 10.0),
        ("mean * std", 10.0),
        ("mean / std", 2.5),
        # The rest of the parser
        ("mean+3*std", 11.0),
        ("2^3^2", 512.0),
        ("-2^2", 4.0),
        ("+-3", 3.0),
        ("-+-3", -3.0),
        ("Pi", math.pi),
        ("e", math.e),
        ("round(2.5)", 2.0),
        ("round(2.675, trunc(2))", 2.67),
        ("sgn(-3)", -1.0),
        ("2 + PI(5)", 5.0 + math.pi),
    ],
)
def test_examples(fx, expected):
    assert eval_fx(fx, STATS) == expected


@pytest.mark.parametrize(
    ("fx", "error"),
    [
        ("3 -", ValueError),
        ("kurtosis * 2", ValueError),
        ("(-8)^(1/3)", ValueError),
        ("sin(1e999)", ValueError),
        ("round(2.5, 1)", TypeError),
        ("sin(1, 2)", TypeError),
        ("1 / 0", ZeroDivisionError),
        ("2^1024", OverflowError),
        ("exp(1000)", OverflowError),
    ],
)
def test_errors(fx, error):
    with pytest.raises(error):
        eval_fx(fx, STATS)


def test_missing_stat_is_key_error():
    with pytest.raises(KeyError, match="std"):
        eval_fx("mean + std", {"mean": 1.0})


def test_ints_and_extra_stats_are_accepted():
    assert eval_fx("mean", {"mean": 3, "median": 2.0}) == 3.0


# The documented subset of the language (statistics, numbers, + - * / and
# parentheses) is ordinary arithmetic, so Python can evaluate it directly.

numbers = st.one_of(
    st.integers(min_value=0, max_value=10**6).map(str),
    st.floats(min_value=0, max_value=1e6, allow_nan=False).map(lambda f: f"{f:.6f}"),
)
names = st.sampled_from(sorted(STATS))
leaves = st.one_of(numbers, names)


def combine(children):
    binary = st.tuples(children, st.sampled_from(["+", "-", "*", "/"]), children)
    return st.one_of(
        binary.map(lambda t: f"{t[0]} {t[1]} {t[2]}"),
        children.map(lambda c: f"( {c} )"),
    )


documented = st.recursive(leaves, combine, max_leaves=12)
stats_values = st.fixed_dictionaries(
    {name: st.floats(min_value=-1e6, max_value=1e6) for name in STATS}
)


def python_outcome(fx, stats):
    try:
        # Only generated strings of numbers, names and arithmetic reach here
        return ("ok", float(eval(fx, {"__builtins__": {}}, dict(stats))))
    except ZeroDivisionError:
        return ("error", ZeroDivisionError)


def fx_outcome(fx, stats):
    try:
        return ("ok", eval_fx(fx, stats))
    except ZeroDivisionError:
        return ("error", ZeroDivisionError)


@settings(max_examples=500, deadline=None)
@given(fx=documented, stats=stats_values)
def test_documented_subset_matches_python_arithmetic(fx, stats):
    assert fx_outcome(fx, stats) == python_outcome(fx, stats)
