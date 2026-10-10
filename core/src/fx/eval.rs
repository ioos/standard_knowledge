//! Stack evaluation matching ioos_qc's `fx_parser.evaluate_stack`, with
//! Python's numeric rules for the operators and functions it uses.

use std::collections::HashMap;
use std::f64::consts::{E, PI};

use super::{FxError, Token};

/// Python distinguishes `int` and `float` results. Integers come from
/// `trunc`, `round` and `sgn` and change how `round(x, n)` and `^` behave.
/// Integers are held as `f64`, so they are exact up to 2^53.
#[derive(Clone, Copy, Debug)]
struct Value {
    value: f64,
    is_int: bool,
}

impl Value {
    fn float(value: f64) -> Self {
        Value {
            value,
            is_int: false,
        }
    }

    fn int(value: f64) -> Result<Self, FxError> {
        if value.is_finite() {
            // Python integers have no negative zero
            Ok(Value {
                value: value + 0.0,
                is_int: true,
            })
        } else {
            Err(FxError::Overflow(
                "integer result is too large for a float".to_string(),
            ))
        }
    }
}

/// Threshold `sgn` uses to treat a value as zero
const EPSILON: f64 = 1e-12;

const STATS: [&str; 4] = ["mean", "min", "max", "std"];

pub(super) fn evaluate(tokens: &[Token], stats: &HashMap<String, f64>) -> Result<f64, FxError> {
    let mut stack = tokens.to_vec();
    pop_value(&mut stack, stats).map(|v| v.value)
}

fn pop_value(stack: &mut Vec<Token>, stats: &HashMap<String, f64>) -> Result<Value, FxError> {
    let token = stack
        .pop()
        .ok_or_else(|| FxError::Type("pop from empty list".to_string()))?;

    match token {
        Token::UnaryMinus => {
            let v = pop_value(stack, stats)?;
            if v.is_int {
                Value::int(-v.value)
            } else {
                Ok(Value::float(-v.value))
            }
        }
        Token::BinOp(op) => {
            // The right operand is on top of the stack
            let right = pop_value(stack, stats)?;
            let left = pop_value(stack, stats)?;
            binary(op, left, right)
        }
        Token::Pi => Ok(Value::float(PI)),
        Token::E => Ok(Value::float(E)),
        Token::Number(text) => text
            .parse::<f64>()
            .map(Value::float)
            .map_err(|e| FxError::Type(e.to_string())),
        Token::Ident(name) => named(&name, 0, stack, stats),
        Token::Call(name, arg_count) => named(&name, arg_count, stack, stats),
    }
}

/// Resolve a word, either bare (`arg_count == 0`) or called
fn named(
    name: &str,
    arg_count: usize,
    stack: &mut Vec<Token>,
    stats: &HashMap<String, f64>,
) -> Result<Value, FxError> {
    // Constants and statistics ignore any call arguments without evaluating
    // them, leaving them on the stack as ioos_qc does.
    match name {
        "PI" => return Ok(Value::float(PI)),
        "E" => return Ok(Value::float(E)),
        _ => {}
    }
    if STATS.contains(&name) {
        return stats
            .get(name)
            .map(|v| Value::float(*v))
            .ok_or_else(|| FxError::MissingStat(name.to_string()));
    }
    if !matches!(
        name,
        "sin" | "cos" | "tan" | "exp" | "abs" | "trunc" | "round" | "sgn"
    ) {
        return Err(FxError::InvalidIdentifier(name.to_string()));
    }

    // Arguments are popped last-first, then put back in order
    let mut args = Vec::with_capacity(arg_count);
    for _ in 0..arg_count {
        args.push(pop_value(stack, stats)?);
    }
    args.reverse();

    match (name, args.as_slice()) {
        ("sin", [x]) => trig(x.value, f64::sin),
        ("cos", [x]) => trig(x.value, f64::cos),
        ("tan", [x]) => trig(x.value, f64::tan),
        ("exp", [x]) => {
            let result = x.value.exp();
            if x.value.is_finite() && result.is_infinite() {
                Err(FxError::Overflow("math range error".to_string()))
            } else {
                Ok(Value::float(result))
            }
        }
        ("abs", [x]) => Ok(Value {
            value: x.value.abs(),
            ..*x
        }),
        ("trunc", [x]) => to_int(x.value.trunc()),
        ("sgn", [x]) => {
            let sign = if x.value < -EPSILON {
                -1.0
            } else if x.value > EPSILON {
                1.0
            } else {
                0.0
            };
            Value::int(sign)
        }
        ("round", [x]) => {
            if x.is_int {
                Ok(*x)
            } else {
                to_int(x.value.round_ties_even())
            }
        }
        ("round", [x, digits]) => {
            if !digits.is_int {
                return Err(FxError::Type(
                    "'float' object cannot be interpreted as an integer".to_string(),
                ));
            }
            let digits = digits.value.clamp(i64::MIN as f64, i64::MAX as f64) as i64;
            if x.is_int {
                round_int(x.value, digits)
            } else {
                round_float(x.value, digits).map(Value::float)
            }
        }
        _ => Err(FxError::Type(format!(
            "{name}() was given {arg_count} arguments"
        ))),
    }
}

fn trig(x: f64, f: fn(f64) -> f64) -> Result<Value, FxError> {
    if x.is_infinite() {
        Err(FxError::Domain("math domain error".to_string()))
    } else {
        Ok(Value::float(f(x)))
    }
}

/// Python's `int(x)` of a float
fn to_int(x: f64) -> Result<Value, FxError> {
    if x.is_nan() {
        Err(FxError::Domain(
            "cannot convert float NaN to integer".to_string(),
        ))
    } else if x.is_infinite() {
        Err(FxError::Overflow(
            "cannot convert float infinity to integer".to_string(),
        ))
    } else {
        Value::int(x)
    }
}

fn binary(op: char, left: Value, right: Value) -> Result<Value, FxError> {
    let both_int = left.is_int && right.is_int;
    let (a, b) = (left.value, right.value);
    match op {
        '+' | '-' | '*' => {
            let result = match op {
                '+' => a + b,
                '-' => a - b,
                _ => a * b,
            };
            if both_int {
                Value::int(result)
            } else {
                Ok(Value::float(result))
            }
        }
        '/' => {
            if b == 0.0 {
                Err(FxError::ZeroDivision("float division by zero".to_string()))
            } else {
                Ok(Value::float(a / b))
            }
        }
        '^' => {
            if both_int && b >= 0.0 {
                // Python integers have arbitrary precision; here they only
                // stay exact while they fit in a float's 53-bit mantissa.
                Value::int(a.powf(b))
            } else {
                float_pow(a, b).map(Value::float)
            }
        }
        _ => unreachable!("the parser only produces + - * / ^"),
    }
}

fn is_odd_integer(x: f64) -> bool {
    (x.abs() % 2.0) == 1.0
}

/// CPython's `float_pow`, including its special cases and errors
fn float_pow(iv: f64, iw: f64) -> Result<f64, FxError> {
    if iw == 0.0 {
        return Ok(1.0);
    }
    if iv.is_nan() {
        return Ok(iv);
    }
    if iw.is_nan() {
        return Ok(if iv == 1.0 { 1.0 } else { iw });
    }
    if iw.is_infinite() {
        let iv = iv.abs();
        return Ok(if iv == 1.0 {
            1.0
        } else if (iw > 0.0) == (iv > 1.0) {
            iw.abs()
        } else {
            0.0
        });
    }
    if iv.is_infinite() {
        let odd = is_odd_integer(iw);
        return Ok(if iw > 0.0 {
            if odd {
                iv
            } else {
                iv.abs()
            }
        } else if odd {
            0.0_f64.copysign(iv)
        } else {
            0.0
        });
    }
    if iv == 0.0 {
        if iw < 0.0 {
            return Err(FxError::ZeroDivision(
                "0.0 cannot be raised to a negative power".to_string(),
            ));
        }
        return Ok(if is_odd_integer(iw) { iv } else { 0.0 });
    }

    let mut base = iv;
    let mut negate = false;
    if base < 0.0 {
        if iw != iw.floor() {
            return Err(FxError::NonReal(
                "a negative number raised to a fractional power is complex".to_string(),
            ));
        }
        base = -base;
        negate = is_odd_integer(iw);
    }
    if base == 1.0 {
        return Ok(if negate { -1.0 } else { 1.0 });
    }

    let result = base.powf(iw);
    if result.is_infinite() {
        return Err(FxError::Overflow(
            "Numerical result out of range".to_string(),
        ));
    }
    Ok(if negate { -result } else { result })
}

/// `round(int, ndigits)`: unchanged for ndigits >= 0, otherwise rounded to a
/// multiple of 10^-ndigits with ties to even
fn round_int(x: f64, digits: i64) -> Result<Value, FxError> {
    if digits >= 0 {
        return Value::int(x);
    }
    let power = 10f64.powi((-digits).min(400) as i32);
    if !power.is_finite() || x.abs() < power / 2.0 {
        return Value::int(0.0);
    }
    let remainder = x % power;
    let base = x - remainder;
    let twice = 2.0 * remainder.abs();
    let rounded = if twice > power || (twice == power && (base / power) % 2.0 != 0.0) {
        base + power.copysign(x)
    } else {
        base
    };
    Value::int(rounded)
}

/// Largest and smallest `ndigits` CPython rounds rather than passing through
const NDIGITS_MAX: i64 = 323;
const NDIGITS_MIN: i64 = -308;

/// `round(float, ndigits)`: correctly rounded to `ndigits` decimal places
/// with ties to even, as CPython does using its exact decimal conversion
fn round_float(x: f64, digits: i64) -> Result<f64, FxError> {
    if !x.is_finite() || x == 0.0 || digits > NDIGITS_MAX {
        return Ok(x);
    }
    if digits < NDIGITS_MIN {
        return Ok(0.0 * x);
    }

    // The exact decimal expansion of |x| as digits and a decimal exponent.
    // 800 significant digits covers every finite double exactly.
    let scientific = format!("{:.800e}", x.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific format");
    let exponent: i64 = exponent.parse().expect("integer exponent");
    let all_digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();

    // Position of the first digit to drop. Digit i has place value
    // 10^(exponent - i), and we keep places down to 10^-digits.
    let keep = exponent + 1 + digits;
    let kept: Vec<u8> = if keep < 0 {
        Vec::new()
    } else {
        let keep = keep as usize;
        let mut kept: Vec<u8> = all_digits.iter().take(keep).copied().collect();
        if keep < all_digits.len() {
            let first_dropped = all_digits[keep];
            let rest_nonzero = all_digits[keep + 1..].iter().any(|d| *d != b'0');
            let last_kept_odd = kept.last().is_some_and(|d| (d - b'0') % 2 == 1);
            let round_up =
                first_dropped > b'5' || (first_dropped == b'5' && (rest_nonzero || last_kept_odd));
            if round_up {
                increment(&mut kept);
            }
        }
        kept
    };
    // Build "<sign>0<digits>e<exp>" like CPython does and convert back
    let sign = if x.is_sign_negative() { "-" } else { "" };
    let text = if kept.is_empty() {
        format!("{sign}0")
    } else {
        let kept = String::from_utf8(kept).expect("ASCII digits");
        // The last kept digit has place value 10^-digits
        format!("{sign}0{kept}e{}", -digits)
    };
    let rounded: f64 = text.parse().expect("valid float text");
    if rounded.is_infinite() {
        return Err(FxError::Overflow(
            "rounded value too large to represent".to_string(),
        ));
    }
    Ok(rounded)
}

/// Add one to a decimal digit string, growing it on carry out
fn increment(digits: &mut Vec<u8>) {
    for d in digits.iter_mut().rev() {
        if *d == b'9' {
            *d = b'0';
        } else {
            *d += 1;
            return;
        }
    }
    digits.insert(0, b'1');
}
