use std::collections::{HashMap, HashSet};

use crate::qartod::QcTestSuite;
use crate::standards_filter::StandardsFilter;
use crate::{standard::Standard, Knowledge};

#[derive(Debug, Default, Clone)]
pub struct StandardsLibrary {
    pub standards: HashMap<String, Standard>,
}

impl StandardsLibrary {
    /// Load CF standards from library
    pub fn load_cf_standards(&mut self) {
        use crate::cf::cf_standards;

        self.standards.extend(cf_standards());
    }

    pub fn filter(&self) -> StandardsFilter {
        StandardsFilter {
            standards: self.standards.values().cloned().collect(),
        }
    }

    /// Return a standard by name or alias
    pub fn get(&self, standard_name_or_alias: &str) -> Result<Standard, &'static str> {
        let filter = self.filter();
        let standard = filter.get(standard_name_or_alias)?;
        Ok(standard.clone())
    }

    /// Update the loaded standards with knowledge
    pub fn apply_knowledge(&mut self, knowledge: Vec<Knowledge>) {
        for know in knowledge {
            if let Some(standard) = self.standards.get(&know.name) {
                let mut common_variable_names = standard.common_variable_names.clone();
                common_variable_names.append(&mut know.common_variable_names.clone());

                let mut related_standards = standard.related_standards.clone();
                related_standards.append(&mut know.related_standards.clone());

                let mut sibling_standards = standard.sibling_standards.clone();
                sibling_standards.append(&mut know.sibling_standards.clone());

                let mut extra_attrs = standard.extra_attrs.clone();
                for (key, value) in know.extra_attrs {
                    extra_attrs.insert(key, value);
                }

                let mut other_units = standard.other_units.clone();
                other_units.append(&mut know.other_units.clone());

                let mut qartod = standard.qartod.clone();

                if let Some(qc) = know.qc {
                    for (slug, qc) in qc {
                        qartod.push(QcTestSuite { slug, qc });
                    }
                }

                let new_standard = Standard {
                    long_name: know.long_name,
                    ioos_category: know.ioos_category,
                    common_variable_names,
                    related_standards,
                    sibling_standards,
                    extra_attrs,
                    other_units: know.other_units,
                    comments: know.comments,
                    qartod,
                    ..standard.clone()
                };

                self.standards.insert(know.name, new_standard);
            }
        }
    }

    /// Load community knowledge
    pub fn load_knowledge(&mut self) {
        let knowledge = crate::library_knowledge::load_knowledge();
        self.apply_knowledge(knowledge);
    }

    /// Return a set of all known IOOS categories
    pub fn known_ioos_categories(&self) -> HashSet<String> {
        self.standards
            .values()
            .flat_map(|s| s.ioos_category.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_load_standards() {
        let mut library = StandardsLibrary::default();
        library.load_cf_standards();
    }

    #[test]
    fn can_get_standard() {
        let mut library = StandardsLibrary::default();
        library.load_cf_standards();
        let pressure = library.get("air_pressure_at_mean_sea_level").unwrap();
        assert_eq!(pressure.name, "air_pressure_at_mean_sea_level");
    }

    #[test]
    fn can_get_standard_by_alias() {
        let mut library = StandardsLibrary::default();
        library.load_cf_standards();
        let pressure = library.get("air_pressure_at_sea_level").unwrap();
        assert_eq!(pressure.name, "air_pressure_at_mean_sea_level");
    }

    #[test]
    fn can_apply_knowledge() {
        let mut library = StandardsLibrary::default();
        library.load_cf_standards();
        let pressure = library.get("air_pressure_at_mean_sea_level").unwrap();
        assert_eq!(pressure.name, "air_pressure_at_mean_sea_level");
        assert_eq!(pressure.long_name, None);

        let know = Knowledge {
            name: "air_pressure_at_mean_sea_level".to_string(),
            long_name: Some("Air Pressure at Sea Level".to_string()),
            ..Default::default()
        };

        library.apply_knowledge(vec![know]);

        let updated_pressure = library.get("air_pressure_at_mean_sea_level").unwrap();
        assert_eq!(updated_pressure.name, "air_pressure_at_mean_sea_level");
        assert_eq!(
            updated_pressure.long_name.as_ref().unwrap(),
            "Air Pressure at Sea Level"
        );

        assert_ne!(pressure, updated_pressure);
    }

    #[test]
    fn can_find_by_variable_name() {
        let mut library = StandardsLibrary::default();
        library.load_cf_standards();
        let know = Knowledge {
            name: "air_pressure_at_mean_sea_level".to_string(),
            long_name: Some("Air Pressure at Sea Level".to_string()),
            common_variable_names: vec!["pressure".to_string()],
            ..Default::default()
        };

        library.apply_knowledge(vec![know]);

        let filtered = library.filter().by_variable_name("pressure");
        let pressure = &filtered.standards[0];
        assert_eq!(pressure.name, "air_pressure_at_mean_sea_level");
    }

    #[test]
    fn can_find_by_variable_name_case_insensitive() {
        let mut library = StandardsLibrary::default();
        library.load_cf_standards();
        let know = Knowledge {
            name: "air_pressure_at_mean_sea_level".to_string(),
            long_name: Some("Air Pressure at Sea Level".to_string()),
            common_variable_names: vec!["airPressure".to_string()],
            ..Default::default()
        };

        library.apply_knowledge(vec![know]);

        // Should match camelCase with snake_case
        let filtered = library.filter().by_variable_name("air_pressure");
        assert_eq!(filtered.standards.len(), 1);
        assert_eq!(filtered.standards[0].name, "air_pressure_at_mean_sea_level");

        // Should match snake_case with camelCase
        let know2 = Knowledge {
            name: "sea_surface_wave_mean_period".to_string(),
            long_name: Some("Mean Wave Period".to_string()),
            common_variable_names: vec!["mean_period".to_string()],
            ..Default::default()
        };
        library.apply_knowledge(vec![know2]);

        let filtered = library.filter().by_variable_name("meanPeriod");
        assert_eq!(filtered.standards.len(), 1);
        assert_eq!(filtered.standards[0].name, "sea_surface_wave_mean_period");

        // Should match uppercase
        let filtered = library.filter().by_variable_name("AIR_PRESSURE");
        assert_eq!(filtered.standards.len(), 1);
        assert_eq!(filtered.standards[0].name, "air_pressure_at_mean_sea_level");
    }
}
