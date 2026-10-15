use serde::{Deserialize, Serialize};

// The test configs are generic over their number types so the same shapes
// describe both an ioos_qc config (the `f64` and `isize` defaults) and a YAML
// test suite template, where any number can be an expression (`Num`).

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct GrossRangeTest<N = f64> {
    pub suspect_span: (N, N),
    pub fail_span: (N, N),
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct LocationTest<N = f64> {
    bbox: (N, N, N, N), // (min_lon, min_lat, max_lon, max_lat)
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct RateOfChange<N = f64> {
    pub threshold: N,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Spike<N = f64> {
    pub suspect_threshold: N,
    pub fail_threshold: N,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct FlatLine<N = f64, I = isize> {
    pub tolerance: N,
    pub suspect_threshold: I,
    pub fail_threshold: I,
}

/// Low variability over `test_period` seconds, measured with
/// `check_type` (`std`, the default, or `range`)
#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct AttenuatedSignal<N = f64, I = isize> {
    pub suspect_threshold: N,
    pub fail_threshold: N,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_period: Option<I>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_obs: Option<I>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_period: Option<I>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_type: Option<String>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ConfigStreamQartod<N = f64, I = isize> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gross_range_test: Option<GrossRangeTest<N>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_test: Option<LocationTest<N>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_of_change_test: Option<RateOfChange<N>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spike_test: Option<Spike<N>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flat_line_test: Option<FlatLine<N, I>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attenuated_signal_test: Option<AttenuatedSignal<N, I>>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct ConfigStream<N = f64, I = isize> {
    pub qartod: ConfigStreamQartod<N, I>,
}

impl<N, I> ConfigStream<N, I> {
    /// Convert every number with `num`, and every integer with `int`
    #[allow(dead_code)]
    pub fn try_map<M, J, E>(
        &self,
        num: &mut impl FnMut(&N) -> Result<M, E>,
        int: &mut impl FnMut(&I) -> Result<J, E>,
    ) -> Result<ConfigStream<M, J>, E> {
        let q = &self.qartod;
        let mut pair = |(a, b): &(N, N)| -> Result<(M, M), E> { Ok((num(a)?, num(b)?)) };
        let gross_range_test = match &q.gross_range_test {
            Some(t) => Some(GrossRangeTest {
                suspect_span: pair(&t.suspect_span)?,
                fail_span: pair(&t.fail_span)?,
            }),
            None => None,
        };
        let location_test = match &q.location_test {
            Some(t) => {
                let (a, b, c, d) = &t.bbox;
                Some(LocationTest {
                    bbox: (num(a)?, num(b)?, num(c)?, num(d)?),
                })
            }
            None => None,
        };
        let rate_of_change_test = match &q.rate_of_change_test {
            Some(t) => Some(RateOfChange {
                threshold: num(&t.threshold)?,
            }),
            None => None,
        };
        let spike_test = match &q.spike_test {
            Some(t) => Some(Spike {
                suspect_threshold: num(&t.suspect_threshold)?,
                fail_threshold: num(&t.fail_threshold)?,
            }),
            None => None,
        };
        let flat_line_test = match &q.flat_line_test {
            Some(t) => Some(FlatLine {
                tolerance: num(&t.tolerance)?,
                suspect_threshold: int(&t.suspect_threshold)?,
                fail_threshold: int(&t.fail_threshold)?,
            }),
            None => None,
        };
        let attenuated_signal_test = match &q.attenuated_signal_test {
            Some(t) => Some(AttenuatedSignal {
                suspect_threshold: num(&t.suspect_threshold)?,
                fail_threshold: num(&t.fail_threshold)?,
                test_period: t.test_period.as_ref().map(&mut *int).transpose()?,
                min_obs: t.min_obs.as_ref().map(&mut *int).transpose()?,
                min_period: t.min_period.as_ref().map(&mut *int).transpose()?,
                check_type: t.check_type.clone(),
            }),
            None => None,
        };
        Ok(ConfigStream {
            qartod: ConfigStreamQartod {
                gross_range_test,
                location_test,
                rate_of_change_test,
                spike_test,
                flat_line_test,
                attenuated_signal_test,
            },
        })
    }
}

/// A number in a YAML test suite: a literal, or an fx expression written
/// `{expr: "mhhw + 6 * ft"}`
#[derive(Clone, PartialEq, Debug)]
pub enum Num {
    Value(f64),
    Expr(String),
}

impl Default for Num {
    fn default() -> Self {
        Num::Value(0.0)
    }
}

impl Serialize for Num {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Num::Value(value) => serializer.serialize_f64(*value),
            Num::Expr(source) => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("expr", source)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Num {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct NumVisitor;

        impl<'de> serde::de::Visitor<'de> for NumVisitor {
            type Value = Num;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a number or {expr: \"...\"}")
            }

            fn visit_f64<E>(self, value: f64) -> Result<Num, E> {
                Ok(Num::Value(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Num, E> {
                Ok(Num::Value(value as f64))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Num, E> {
                Ok(Num::Value(value as f64))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Num, A::Error> {
                use serde::de::Error;
                let source = match map.next_key::<String>()? {
                    Some(key) if key == "expr" => map.next_value::<String>()?,
                    _ => return Err(A::Error::custom("expected {expr: \"...\"}")),
                };
                if map.next_key::<String>()?.is_some() {
                    return Err(A::Error::custom("expected only an expr key"));
                }
                Ok(Num::Expr(source))
            }
        }

        deserializer.deserialize_any(NumVisitor)
    }
}
