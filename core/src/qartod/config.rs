use serde::{Deserialize, Deserializer, Serialize};

// ioos_qc stream config. A test that a suite adds is a struct here, a field
// on `ConfigStreamQartod`, and a row in `types::QARTOD_TESTS`.

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrossRangeTest {
    pub suspect_span: (f64, f64),
    pub fail_span: (f64, f64),
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationTest {
    pub bbox: (f64, f64, f64, f64), // (min_lon, min_lat, max_lon, max_lat)
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RateOfChange {
    pub threshold: f64,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Spike {
    pub suspect_threshold: f64,
    pub fail_threshold: f64,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatLine {
    pub tolerance: f64,
    #[serde(deserialize_with = "whole_number")]
    pub suspect_threshold: isize,
    #[serde(deserialize_with = "whole_number")]
    pub fail_threshold: isize,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigStreamQartod {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gross_range_test: Option<GrossRangeTest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location_test: Option<LocationTest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_of_change_test: Option<RateOfChange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spike_test: Option<Spike>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flat_line_test: Option<FlatLine>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigStream {
    pub qartod: ConfigStreamQartod,
}

/// An integer, which may be written as a whole float like `7200.0`
fn whole_number<'de, D: Deserializer<'de>>(deserializer: D) -> Result<isize, D::Error> {
    let value = f64::deserialize(deserializer)?;
    if value.fract() == 0.0 && value.abs() <= isize::MAX as f64 {
        Ok(value as isize)
    } else {
        Err(serde::de::Error::custom(format!(
            "expected a whole number, found {value}"
        )))
    }
}
