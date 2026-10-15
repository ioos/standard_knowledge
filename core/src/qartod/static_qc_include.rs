/// QARTOD test suite configuration defined in a knowledge YAML file.
///
/// `tests` has the shape of an ioos_qc stream config (`qartod:` and the
/// tests under it). Any number in it can instead be an expression,
/// written `{expr: "mhhw + 6 * ft"}`, that uses the suite's `arguments`
/// and `let` constants. See `core/src/fx/GRAMMAR.md` for the language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticQc {
    pub name: String,
    pub summary: String,
    pub description: String,

    /// Values the user supplies when scaffolding, by name
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub arguments: BTreeMap<String, SuiteArgument>,

    /// Named constants, evaluated in order. Each is a number or an
    /// expression, which can use the arguments and earlier constants.
    #[serde(default, rename = "let", skip_serializing_if = "IndexMap::is_empty")]
    pub constants: IndexMap<String, Num>,

    /// The ioos_qc stream config, with optional expressions
    pub tests: ConfigStream<Num, Num>,
}

/// An argument to a YAML test suite
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuiteArgument {
    #[serde(rename = "type")]
    pub argument_type: SuiteArgumentType,
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<f64>,
}

/// Arguments are used in expressions, so they are numbers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuiteArgumentType {
    Float,
    Int,
}
