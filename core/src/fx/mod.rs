#![doc = include_str!("GRAMMAR.md")]

use std::collections::HashMap;
use std::fmt::Display;

mod eval;
mod parser;

/// One entry of the postfix program a parsed expression is compiled to.
///
/// The order and shape mirror the `exprStack` that ioos_qc's pyparsing
/// grammar builds, so that evaluation reproduces its results exactly.
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// A numeric literal, kept as written and converted when evaluated
    Number(String),
    /// `PI`, matched case-insensitively as a bare word
    Pi,
    /// `E`, matched case-insensitively as a bare word
    E,
    /// Any other bare word: a statistic, or an invalid identifier
    Ident(String),
    /// `name(arg, ...)` with the number of arguments
    Call(String, usize),
    /// One leading `-` on an atom
    UnaryMinus,
    /// One of `+ - * / ^`
    BinOp(char),
}

/// Errors from parsing or evaluating an expression.
///
/// Each evaluation error corresponds to the Python exception that
/// ioos_qc's `fx_parser.eval_fx` raises for the same input.
#[derive(Clone, Debug, PartialEq)]
pub enum FxError {
    /// The text does not match the grammar (pyparsing `ParseException`)
    Parse { position: usize, message: String },
    /// A word that is not a statistic, constant or function (`Exception`)
    InvalidIdentifier(String),
    /// A statistic used in the expression is missing from the stats (`KeyError`)
    MissingStat(String),
    /// Wrong argument count or an argument of the wrong kind (`TypeError`)
    Type(String),
    /// Division by zero, or zero raised to a negative power (`ZeroDivisionError`)
    ZeroDivision(String),
    /// A finite calculation overflowed (`OverflowError`)
    Overflow(String),
    /// A math function was called outside its domain (`ValueError`)
    Domain(String),
    /// The result would be a complex number. Python carries complex values
    /// on; this implementation stops here.
    NonReal(String),
}

impl Display for FxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FxError::Parse { position, message } => {
                write!(f, "{message} (at char {position})")
            }
            FxError::InvalidIdentifier(name) => write!(f, "invalid identifier '{name}'"),
            FxError::MissingStat(name) => write!(f, "missing statistic '{name}'"),
            FxError::Type(message)
            | FxError::ZeroDivision(message)
            | FxError::Overflow(message)
            | FxError::Domain(message)
            | FxError::NonReal(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for FxError {}

/// A parsed expression, ready to be evaluated against different statistics.
#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    tokens: Vec<Token>,
}

impl Expression {
    /// The postfix program, in the order ioos_qc pushes it onto `exprStack`
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    /// Evaluate with `min`, `max`, `mean` and `std` taken from `stats`.
    /// Other keys in `stats` are ignored.
    pub fn evaluate(&self, stats: &HashMap<String, f64>) -> Result<f64, FxError> {
        eval::evaluate(&self.tokens, stats)
    }
}

/// Parse an expression without evaluating it
pub fn parse(source: &str) -> Result<Expression, FxError> {
    parser::parse(source).map(|tokens| Expression { tokens })
}

/// Parse and evaluate an expression, like ioos_qc's `fx_parser.eval_fx`
pub fn eval_fx(source: &str, stats: &HashMap<String, f64>) -> Result<f64, FxError> {
    parse(source)?.evaluate(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats() -> HashMap<String, f64> {
        HashMap::from([
            ("min".to_string(), 1.0),
            ("max".to_string(), 9.0),
            ("mean".to_string(), 5.0),
            ("std".to_string(), 2.0),
        ])
    }

    fn ok(source: &str) -> f64 {
        eval_fx(source, &stats()).unwrap_or_else(|e| panic!("{source:?} failed: {e}"))
    }

    fn err(source: &str) -> FxError {
        eval_fx(source, &stats()).expect_err(source)
    }

    // Expected values were produced by ioos_qc's fx_parser with the same stats.

    #[test]
    fn ioos_qc_documented_examples() {
        assert_eq!(ok("3"), 3.0);
        assert_eq!(ok("( 1 + 2 )"), 3.0);
        assert_eq!(ok("3 * 2 - 6"), 0.0);
        assert_eq!(ok("3 * mean + std / ( max * min )"), 15.0 + 2.0 / 9.0);
        assert_eq!(ok("min - std * 2"), -3.0);
        assert_eq!(ok("max + std / 2"), 10.0);
        assert_eq!(ok("mean * std"), 10.0);
        assert_eq!(ok("mean / std"), 2.5);
        assert_eq!(ok("mean + 3 * std"), 11.0);
    }

    #[test]
    fn literals_and_whitespace() {
        assert_eq!(ok(" 1 "), 1.0);
        assert_eq!(ok("1\t+\n2"), 3.0);
        assert_eq!(ok("1."), 1.0);
        assert_eq!(ok("1e5"), 100000.0);
        assert_eq!(ok("1.e-3"), 0.001);
        assert_eq!(ok("007"), 7.0);
        assert_eq!(ok("1e999"), f64::INFINITY);
        assert!(ok("1e999-1e999").is_nan());
        assert!(matches!(err(""), FxError::Parse { .. }));
        assert!(matches!(err("3 -"), FxError::Parse { .. }));
        assert!(matches!(err(".5"), FxError::Parse { .. }));
        assert!(matches!(err("1e"), FxError::Parse { .. }));
        assert!(matches!(err("2pi"), FxError::Parse { .. }));
    }

    #[test]
    fn unary_signs() {
        assert_eq!(ok("--3"), 3.0);
        assert_eq!(ok("- (3)"), -3.0);
        assert_eq!(ok("-(-3)"), 3.0);
        // Only the minus signs before the first plus count
        assert_eq!(ok("+-3"), 3.0);
        assert_eq!(ok("-+-3"), -3.0);
        // Unary minus binds tighter than ^
        assert_eq!(ok("-2^2"), 4.0);
        assert_eq!(ok("2^-2"), 0.25);
    }

    #[test]
    fn exponent_is_right_associative() {
        assert_eq!(ok("2^3^2"), 512.0);
    }

    #[test]
    fn constants_and_identifiers() {
        assert_eq!(ok("Pi"), std::f64::consts::PI);
        assert_eq!(ok("e"), std::f64::consts::E);
        assert_eq!(ok("mean"), 5.0);
        assert_eq!(err("MEAN"), FxError::InvalidIdentifier("MEAN".to_string()));
        assert_eq!(err("E1"), FxError::InvalidIdentifier("E1".to_string()));
        assert_eq!(err("pi(2)"), FxError::InvalidIdentifier("pi".to_string()));
        assert!(matches!(err("sin"), FxError::Type(_)));
        assert_eq!(
            eval_fx("mean", &HashMap::new()),
            Err(FxError::MissingStat("mean".to_string()))
        );
    }

    #[test]
    fn special_names_called_as_functions_leave_their_arguments() {
        // ioos_qc ignores the arguments of PI(...), E(...) and the statistics,
        // without evaluating them. They stay on the stack, where a later
        // operator can pick them up as an operand.
        assert_eq!(ok("PI(1/0)"), std::f64::consts::PI);
        assert_eq!(ok("PI(5) + 2"), std::f64::consts::PI + 2.0);
        assert_eq!(ok("2 + PI(5)"), 5.0 + std::f64::consts::PI);
        assert_eq!(ok("2 + mean(9)"), 14.0);
    }

    #[test]
    fn functions() {
        assert_eq!(ok("abs(-2)"), 2.0);
        assert_eq!(ok("sgn(-3)"), -1.0);
        assert_eq!(ok("sgn(1e-13)"), 0.0);
        assert_eq!(ok("trunc(7.9)/trunc(2)"), 3.5);
        assert_eq!(ok("round(2.5)"), 2.0);
        assert_eq!(ok("round(3.5)"), 4.0);
        assert_eq!(ok("round(2.675, trunc(2))"), 2.67);
        assert_eq!(ok("round(1234.5, trunc(-2))"), 1200.0);
        assert_eq!(ok("round(0.125, trunc(2))"), 0.12);
        assert_eq!(ok("round(trunc(25), trunc(-1))"), 20.0);
        assert_eq!(ok("sin(0)"), 0.0);
        assert_eq!(ok("exp(0)"), 1.0);
        assert!(matches!(err("round(2.5, 1)"), FxError::Type(_)));
        assert!(matches!(err("sin(1,2)"), FxError::Type(_)));
        assert!(matches!(err("round(1,2,3)"), FxError::Type(_)));
        assert_eq!(err("foo(1)"), FxError::InvalidIdentifier("foo".to_string()));
        assert!(matches!(err("sin()"), FxError::Parse { .. }));
    }

    #[test]
    fn arithmetic_errors() {
        assert!(matches!(err("1/0"), FxError::ZeroDivision(_)));
        assert!(matches!(err("0^-1"), FxError::ZeroDivision(_)));
        assert!(matches!(
            err("trunc(0)^trunc(-1)"),
            FxError::ZeroDivision(_)
        ));
        assert!(matches!(err("2^1024"), FxError::Overflow(_)));
        assert!(matches!(err("exp(1000)"), FxError::Overflow(_)));
        assert!(matches!(err("sin(1e999)"), FxError::Domain(_)));
        assert!(matches!(err("trunc(1e999)"), FxError::Overflow(_)));
        assert!(matches!(err("(-8)^(1/3)"), FxError::NonReal(_)));
        // Float multiplication overflows to infinity without an error
        assert_eq!(ok("1e308 * 10"), f64::INFINITY);
    }

    #[test]
    fn parse_once_evaluate_many() {
        let expression = parse("mean + 2 * std").unwrap();
        assert_eq!(expression.evaluate(&stats()).unwrap(), 9.0);
        let other = HashMap::from([("mean".to_string(), 0.0), ("std".to_string(), 1.0)]);
        assert_eq!(expression.evaluate(&other).unwrap(), 2.0);
        assert_eq!(
            expression.tokens(),
            &[
                Token::Ident("mean".to_string()),
                Token::Number("2".to_string()),
                Token::Ident("std".to_string()),
                Token::BinOp('*'),
                Token::BinOp('+'),
            ]
        );
    }
}
