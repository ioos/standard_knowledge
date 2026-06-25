/// A knowledge is a subset of a Standard
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Knowledge {
    /// Standard name the knowledge applies to
    pub name: String,

    /// Human readable name
    pub long_name: Option<String>,

    /// Usual IOOS category for the standard
    pub ioos_category: Option<String>,

    /// Common variable names in a dataset
    #[serde(default)]
    pub common_variable_names: Vec<String>,

    /// Other standards to consider
    #[serde(default)]
    pub related_standards: Vec<String>,

    /// Standards that are usually used together
    #[serde(default)]
    pub sibling_standards: Vec<String>,

    /// Extra attributes that are usually included in Xarray or NetCDF metadata
    #[serde(default)]
    pub extra_attrs: BTreeMap<String, String>,

    /// Other units that may be seen
    #[serde(default)]
    pub other_units: Vec<String>,

    /// Community comments on standard usage
    pub comments: Option<String>,

    /// QARTOD test suites
    pub qc: Option<BTreeMap<String, StaticQc>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct YamlKnowledge {
    // /// Standard name the knowledge applies to
    pub name: Option<String>,
    /// Human readable name
    pub long_name: Option<String>,

    /// Usual IOOS category for the standard
    pub ioos_category: Option<String>,

    /// Common variable names in a dataset
    pub common_variable_names: Option<Vec<String>>,

    /// Other standards to consider
    pub related_standards: Option<Vec<String>>,

    /// Standards that are usually used together
    pub sibling_standards: Option<Vec<String>>,

    /// Extra attributes that are usually included in Xarray or NetCDF metadata
    pub extra_attrs: Option<BTreeMap<String, String>>,

    /// Other units that may be seen
    pub other_units: Option<Vec<String>>,

    /// Community comments on standard usage
    pub comments: Option<String>,

    /// QARTOD test suites
    pub qc: Option<BTreeMap<String, StaticQc>>,
}
