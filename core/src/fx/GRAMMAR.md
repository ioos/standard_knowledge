# fx: threshold expressions

`fx` is the small arithmetic language that ioos_qc's `config_creator` uses to
write QC thresholds in terms of statistics of a dataset, for example
`mean + 3 * std` or `min - std * 2`. This module is a Rust implementation of
it. It follows ioos_qc's parser,
[`ioos_qc/config_creator/fx_parser.py`](https://github.com/ioos/ioos_qc/blob/main/ioos_qc/config_creator/fx_parser.py),
which is itself adapted from the `fourFn.py` example that ships with
pyparsing. Until now that implementation was the only full definition of the
language. This document is meant to be one.

The Python test suite in `py/tests/test_fx.py` checks this implementation
against ioos_qc's parser, including property-based tests with Hypothesis.

## Two levels of the language

ioos_qc accepts two different subsets, and it helps to keep them apart.

**The documented subset.** This is what ioos_qc's usage docs promise and what
`QcVariableConfig` accepts. It is the portable core and the safest thing to
write in configuration:

- the statistics `min`, `max`, `mean` and `std`
- decimal numbers
- the operators `+ - * /`
- parentheses

In ioos_qc, `QcVariableConfig._validate_fx` also requires every token to be
separated by spaces (`mean + 3 * std`, not `mean+3*std`) and rejects
everything else before the parser sees it. This module does not enforce the
spacing rule.

**The full parser.** `fx_parser.eval_fx` accepts more than the validator lets
through: `^`, the constants `PI` and `E`, eight functions, unary signs and
free whitespace. This module implements all of it, so both levels evaluate
the same way here and in Python.

## Lexical rules

| Element | Rule |
| --- | --- |
| Whitespace | Space, tab, newline and carriage return, allowed between any two tokens |
| Number | `[0-9]+ ( "." [0-9]* )? ( [eE] [+-]? [0-9]+ )?` |
| Word | An ASCII letter followed by ASCII letters, digits, `_` or `$` |
| Operators | `+ - * / ^`, plus `(`, `)` and `,` |

Notes on numbers:

- A leading digit is required: `.5` is an error, `0.5` and `5.` are fine.
- An exponent is only taken when it is complete. In `1e` the number is `1`
  and the stray `e` makes the whole expression an error.
- Numbers never carry their own sign. A `-` in front of a number is a unary
  operator (see below).
- Values outside the float range become infinity: `1e999` is `inf`.

## Grammar

In EBNF, starting from `expression`, with whitespace allowed between tokens:

```ebnf
expression = term , { ( "+" | "-" ) , term } ;
term       = factor , { ( "*" | "/" ) , factor } ;
factor     = atom , [ "^" , factor ] ;
atom       = { "+" | "-" } , primary ;
primary    = call | constant | number | word | "(" , expression , ")" ;
call       = word , "(" , expression , { "," , expression } , ")" ;
constant   = "PI" | "E" ;   (* any letter case, as a whole word *)
```

The whole input must match `expression`. Leading and trailing whitespace is
ignored, and anything left over is an error.

This gives the following precedence, from loosest to tightest:

| Level | Operators | Associativity |
| --- | --- | --- |
| 1 | `+` `-` (binary) | left |
| 2 | `*` `/` | left |
| 3 | `^` | right: `2^3^2` is `2^(3^2)` = 512 |
| 4 | `+` `-` (unary) | prefix |

Unary signs bind tighter than `^`, which is unusual: `-2^2` is `(-2)^2` = 4,
and `2^-2` is 0.25.

## Names

A word is looked up as follows. Names are case-sensitive except for the
bare constants.

| Written as | Meaning |
| --- | --- |
| `PI`, `pi`, `Pi` … (bare) | π |
| `E`, `e` (bare) | Euler's number |
| `min`, `max`, `mean`, `std` (bare) | The statistic of that name from the supplied stats |
| `sin(x)`, `cos(x)`, `tan(x)` | Trigonometric functions, in radians |
| `exp(x)` | eˣ |
| `abs(x)` | Absolute value |
| `trunc(x)` | Truncate toward zero, giving an integer |
| `round(x)` | Round half to even, giving an integer |
| `round(x, n)` | Round to `n` decimal places, half to even. `n` must be an integer (see below) |
| `sgn(x)` | −1, 0 or 1. Values within 10⁻¹² of zero give 0 |
| anything else | An error ("invalid identifier") |

Calling a bare function name without parentheses (`sin`) is an error, and so
is calling a function with the wrong number of arguments.

## Evaluation

An expression is compiled to a postfix program and evaluated by popping from
the end, exactly as ioos_qc's `evaluate_stack` does. For well-formed
expressions this gives the usual arithmetic result. Two corners of the
reference implementation behave differently, and this module reproduces both.

**Only minus signs before the first plus count.** A run of unary signs
contributes one negation per `-`, stopping at the first `+`:

| Expression | Result |
| --- | --- |
| `--3` | 3 |
| `+-3` | 3 |
| `-+-3` | −3 |

**Calling a constant or statistic leaves its arguments behind.** `PI(…)`,
`E(…)`, `mean(…)`, `min(…)`, `max(…)` and `std(…)` return the constant or
statistic and never evaluate their arguments (`PI(1/0)` is π, not an error).
The arguments stay on the evaluation stack, where a later operator can take
them as its left operand:

| Expression | Result |
| --- | --- |
| `PI(5) + 2` | π + 2 |
| `2 + PI(5)` | 5 + π, the `2` is never used |
| `2 + mean(9)` | 9 + mean |

The call form only works with the exact upper-case spelling. `pi(2)` is an
invalid identifier even though bare `pi` is π. None of this is useful in a
real configuration. It is recorded here so that the two implementations can
be compared exactly.

## Numbers and types

Results follow Python's numeric rules, because that is what ioos_qc produces.

Values are floats, except that `trunc`, `round(x)` and `sgn` return
integers. The distinction matters in three places:

- `round(x, n)` needs an integer `n`. `round(2.675, trunc(2))` is 2.67, but
  `round(2.5, 1)` is an error, because the literal `1` is a float.
- `round(i, n)` of an integer `i` returns an integer, rounded to a multiple
  of 10⁻ⁿ when `n` is negative.
- `i ^ j` of two integers with `j ≥ 0` is an integer.

Floating point arithmetic is IEEE 754 double precision, with these rules
taken from Python:

| Case | Result |
| --- | --- |
| `x / 0` for any `x` | Error (division by zero) |
| `+`, `-` or `*` overflowing | Infinity, no error |
| `x ^ y` overflowing from finite operands | Error (overflow) |
| `0 ^ y` with `y < 0` | Error (division by zero) |
| `x ^ y` with `x < 0` and `y` not a whole number | Error (complex result, see below) |
| `x ^ 0` | 1, even for `NaN` |
| `exp(x)` overflowing from finite `x` | Error (overflow) |
| `sin`, `cos` or `tan` of ±infinity | Error (domain) |
| `trunc` or `round(x)` of `NaN` or infinity | Error |
| `round(x, n)` of `NaN`, infinity or zero | Returned unchanged |

`round(x, n)` rounds the exact binary value of `x`, so `round(2.675, trunc(2))`
is 2.67, because 2.675 is stored as slightly less than 2.675.

## Errors

| Error | When | Python exception |
| --- | --- | --- |
| `Parse` | The text does not match the grammar | `pyparsing.ParseException` |
| `InvalidIdentifier` | An unknown word | `Exception` |
| `MissingStat` | A statistic not present in the supplied stats | `KeyError` |
| `Type` | Wrong argument count, or a float where an integer is needed | `TypeError` |
| `ZeroDivision` | Division by zero, or zero to a negative power | `ZeroDivisionError` |
| `Overflow` | Overflow in `^`, `exp`, `trunc`, `round`, or an integer too large for a float | `OverflowError` |
| `Domain` | `sin`, `cos`, `tan` of infinity, or `trunc`/`round` of `NaN` | `ValueError` |
| `NonReal` | A complex intermediate result | none, see below |

## Known differences from ioos_qc

These are deliberate, and the comparison tests check that they are the only
ones.

- **Complex numbers.** In Python, `(-8)^(1/3)` is a complex number, and
  evaluation carries on with it. `abs()` can even turn it back into a real
  number: `abs((-8)^(1/3))` is 2.0. A threshold can't be complex, so this
  module stops with a `NonReal` error as soon as a complex value would
  appear.
- **Large integers.** Python integers have unlimited precision. Here they
  are held in a float, so integer results are exact only up to 2⁵³, and an
  integer too large for a float is an `Overflow` error. Python only gets
  that far with expressions like `trunc(10)^trunc(400)`.
- **Statistics are plain floats.** ioos_qc computes its statistics with
  numpy, and numpy floats follow numpy's rules (for example `mean / 0` is
  `inf` with a warning instead of an error). This module and the comparison
  tests use Python's float rules, which is what `eval_fx` does with plain
  Python floats.
- **Digits are ASCII.** Python's `\d` also matches other Unicode decimal
  digits, so ioos_qc accepts `٣` as 3. Here only `0`–`9` are digits.
- **No shared state.** ioos_qc keeps its stack in a module-level list that
  grows on every call and is never cleared. Results are unaffected, but it
  leaks memory and is not safe across threads. Here each evaluation has its
  own stack.
