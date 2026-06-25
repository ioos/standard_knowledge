use core::fmt;
use std::collections::BTreeMap;

use indicium::simple::Indexable;

use crate::qartod::TestSuite;

#[derive(Default, Clone)]
pub struct Standard {
    pub name: String,

    /// Human readable name
    pub long_name: Option<String>,
    pub unit: String,
    pub description: String,
    pub aliases: Vec<String>,

    /// Usual IOOS category for the standard
    pub ioos_category: Option<String>,

    /// Common variable names in a dataset
    pub common_variable_names: Vec<String>,

    /// Other standards to consider
    pub related_standards: Vec<String>,

    /// Standards that are usually used together
    pub sibling_standards: Vec<String>,

    /// Extra attributes that are usually included in Xarray or NetCDF metadata
    pub extra_attrs: BTreeMap<String, String>,

    /// Other units that may be seen
    pub other_units: Vec<String>,

    /// Community comments on standard usage
    pub comments: Option<String>,

    /// QARTOD test suites
    pub qartod: Vec<Box<dyn TestSuite>>,
}

impl Standard {
    /// Do any of the fields in the standard match a search pattern
    pub fn matches_pattern(&self, search_str: &str) -> bool {
        let search_str = search_str.to_lowercase();
        let search_str = search_str.as_str();
        self.name.to_lowercase().contains(search_str)
            || self
                .long_name
                .as_ref()
                .is_some_and(|name| name.to_lowercase().contains(search_str))
            || self.unit.to_lowercase().contains(search_str)
            || self.description.to_lowercase().contains(search_str)
            || self
                .aliases
                .iter()
                .any(|alias| alias.to_lowercase().contains(search_str))
            || self
                .ioos_category
                .as_ref()
                .is_some_and(|category| category.to_lowercase().contains(search_str))
            || self
                .common_variable_names
                .iter()
                .any(|name| name.to_lowercase().contains(search_str))
            || self
                .related_standards
                .iter()
                .any(|name| name.to_lowercase().contains(search_str))
            || self
                .other_units
                .iter()
                .any(|unit| unit.to_lowercase().contains(search_str))
            || self
                .comments
                .as_ref()
                .is_some_and(|comment| comment.to_lowercase().contains(search_str))
    }

    /// Display all the fields for a standard
    pub fn display_all(&self) -> String {
        let mut output = self.display_short();

        if !self.aliases.is_empty() {
            output = format!("{output}\n  Aliases: {}", self.aliases.join(", "))
        }
        if let Some(ioos_category) = &self.ioos_category {
            output = format!("{output}\n  IOOS Category: {ioos_category}")
        }
        if !self.common_variable_names.is_empty() {
            output = format!(
                "{output}\n  Common variables:\n  - {}",
                self.common_variable_names.join("\n  - ")
            )
        }
        if !self.related_standards.is_empty() {
            output = format!(
                "{output}\n  Related standards: {}",
                self.related_standards.join(", ")
            )
        }
        if !self.sibling_standards.is_empty() {
            output = format!(
                "{output}\n  Sibling standards: {}",
                self.sibling_standards.join(", ")
            )
        }
        if !self.extra_attrs.is_empty() {
            output = format!(
                "{output}\n  Extra attributes:\n{}",
                self.display_xarray_attrs()
                    .lines()
                    .map(|l| format!("    {l}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }

        if !self.other_units.is_empty() {
            output = format!("{output}\n  Other units: {}", self.other_units.join(", "))
        }

        if !self.qartod.is_empty() {
            output = format!(
                "{output}\n\n  QARTOD Test Suites:\n  - {}",
                self.qartod
                    .iter()
                    .map(|suite| suite.info().to_string())
                    .collect::<Vec<_>>()
                    .join("\n  - ")
            );
        }

        output = format!("{output}\n\n{}", self.description);
        if let Some(comments) = &self.comments {
            output = format!("{output}\n\nComments: {comments}")
        }

        output
    }

    /// Attributes displayed with Xarray
    pub fn xarray_attrs(&self) -> BTreeMap<&str, &str> {
        let mut map = BTreeMap::from([("standard_name", self.name.as_str())]);

        if !self.unit.is_empty() {
            map.insert("units", self.unit.as_str());
        }

        if let Some(long_name) = &self.long_name {
            if !long_name.is_empty() {
                map.insert("long_name", long_name.as_str());
            }
        }

        if let Some(ioos_category) = &self.ioos_category {
            if !ioos_category.is_empty() {
                map.insert("ioos_category", ioos_category.as_str());
            }
        }

        for (key, value) in &self.extra_attrs {
            map.insert(key, value);
        }

        map
    }

    /// Formatted Xarray attributes
    pub fn display_xarray_attrs(&self) -> String {
        let mut output = "{".to_string();
        for (key, value) in self.xarray_attrs() {
            output = format!("{output}\n  \"{key}\": \"{value}\",");
        }
        output = format!("{output}\n}}");
        output
    }

    /// Short format
    pub fn display_short(&self) -> String {
        if let Some(long_name) = &self.long_name {
            return format!("{} - {} - {}", self.name, long_name, self.unit);
        }
        format!("{} - {}", self.name, self.unit)
    }
}

impl fmt::Debug for Standard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Standard")
            .field("name", &self.name)
            .field("long_name", &self.long_name)
            .field("unit", &self.unit)
            .field("description", &self.description)
            .field("aliases", &self.aliases)
            .field("ioos_category", &self.ioos_category)
            .field("common_variable_names", &self.common_variable_names)
            .field("related_standards", &self.related_standards)
            .field("sibling_standards", &self.sibling_standards)
            .field("extra_attrs", &self.extra_attrs)
            .field("other_units", &self.other_units)
            .field("comments", &self.comments)
            .field("qartod", &format!("[{} test suites]", self.qartod.len()))
            .finish()
    }
}

impl PartialEq for Standard {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.long_name == other.long_name
            && self.unit == other.unit
            && self.description == other.description
            && self.aliases == other.aliases
            && self.ioos_category == other.ioos_category
            && self.common_variable_names == other.common_variable_names
            && self.related_standards == other.related_standards
            && self.sibling_standards == other.sibling_standards
            && self.extra_attrs == other.extra_attrs
            && self.other_units == other.other_units
            && self.comments == other.comments
            // Note: We compare only the length of qartod test suites since trait objects cannot be compared
            && self.qartod.len() == other.qartod.len()
    }
}

impl Indexable for Standard {
    fn strings(&self) -> Vec<String> {
        let mut strings = vec![
            self.name.clone(),
            self.long_name.clone().unwrap_or_default(),
            self.unit.clone(),
            self.description.clone(),
            self.ioos_category.clone().unwrap_or_default(),
            self.comments.clone().unwrap_or_default(),
        ];
        strings.retain(|s| !s.is_empty());
        strings.extend(self.aliases.clone());
        strings.extend(self.common_variable_names.clone());
        strings.extend(self.related_standards.clone());
        strings.extend(self.sibling_standards.clone());
        strings.extend(self.extra_attrs.keys().cloned().collect::<Vec<_>>());
        strings.extend(self.other_units.clone());
        strings.extend(self.qartod.iter().map(|q| q.info().name.clone()));
        strings.extend(self.qartod.iter().map(|q| q.info().description.clone()));
        strings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_match_standard_by_long_name() {
        let standard = Standard {
            name: "air_pressure_at_mean_sea_level".to_string(),
            long_name: None,
            unit: "Pa".to_string(),
            description: "A quick note".to_string(),
            aliases: Vec::new(),
            ioos_category: Some("Meteorology".to_string()),
            common_variable_names: Vec::new(),
            related_standards: Vec::new(),
            sibling_standards: Vec::new(),
            extra_attrs: BTreeMap::new(),
            other_units: Vec::new(),
            comments: None,
            qartod: Vec::new(),
        };

        assert!(
            standard.matches_pattern("Met"),
            "Should be able to find met within the standard",
        );

        assert!(
            !standard.matches_pattern("Nothing"),
            "Shouldn't match something random"
        );
    }
}
