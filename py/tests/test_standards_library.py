import pytest
import standard_knowledge

KNOWLEDGE = {
    "name": "air_pressure_at_mean_sea_level",
    "long_name": "Air Pressure at Sea Level",
    "ioos_category": "Meteorology",
    "common_variable_names": ["air_pressure", "pressure"],
    "related_standards": ["air_pressure"],
    "other_units": ["bar"],
    "comments": "Some adjustment for altitude may be needed",
}


@pytest.fixture
def library():
    lib = standard_knowledge.StandardsLibrary()
    lib.load_cf_standards()
    return lib


def test_library_load_standards(library):
    assert len(library.standards) > 0


def test_get_standard_by_name(library):
    standard = library.get("air_pressure_at_mean_sea_level")
    assert str(standard) == "<Standard: air_pressure_at_mean_sea_level>"
    assert standard.name == "air_pressure_at_mean_sea_level"


def test_get_standard_by_alias(library):
    standard = library.get("air_pressure_at_mean_sea_level")
    assert standard.name == "air_pressure_at_mean_sea_level"


def test_unknown_standard():
    library = standard_knowledge.StandardsLibrary()
    with pytest.raises(KeyError):
        library.get("air_pressure_at_sea_level")


def test_knowledge_must_have_name(library):
    knowledge = {"long_name": "Air Pressure"}

    with pytest.raises(KeyError) as e:
        library.apply_knowledge([knowledge])

    assert "name of the standard" in str(e.value)


def test_can_add_knowledge(library):
    standard = library.get("air_pressure_at_sea_level")
    assert standard.name == "air_pressure_at_mean_sea_level"
    assert standard.long_name is None

    library.apply_knowledge([KNOWLEDGE])

    updated_standard = library.get("air_pressure_at_sea_level")
    assert updated_standard.name == KNOWLEDGE["name"]
    assert updated_standard.long_name == KNOWLEDGE["long_name"]
    assert updated_standard.ioos_category == KNOWLEDGE["ioos_category"]
    assert "pressure" in updated_standard.common_variable_names
    assert "air_pressure" in updated_standard.related_standards
    assert "bar" in updated_standard.other_units
    assert updated_standard.comments == KNOWLEDGE["comments"]

    assert standard != updated_standard


def test_find_standards_by_variable_names(library):
    library.apply_knowledge([KNOWLEDGE])

    standards = library.filter().by_variable_name("pressure")

    standard = standards[0]
    assert standard.name == KNOWLEDGE["name"]


def test_find_standards_by_variable_names_knowledge(library):
    library.load_knowledge()

    standards = library.filter().by_variable_name("atmospheric_pressure")

    names = [s.name for s in standards]
    assert "air_pressure_at_mean_sea_level" in names
    standard = next(s for s in standards if s.name == "air_pressure_at_mean_sea_level")
    assert standard.ioos_category == "Meteorology"


def test_search_standard(library):
    library.apply_knowledge([KNOWLEDGE])

    standards = library.filter().search("pressure")

    assert len(standards) > 0
    # search is a bit fuzzier, so harder to tell where the expected pressure will end up
    # pressure = standards[0]
    # assert pressure.name == KNOWLEDGE["name"], (
    #     "since there isn't a direct name or alias match, the suggested column should make it first"
    # )


def test_can_apply_and_get_qc(library):
    standard = library.get("air_temperature")
    assert standard.qc is None

    qc = {
        "qc": {
            "glos": {
                "name": "GLOS Seagull",
                "summary": "QARTOD tests that GLOS uses for Seagull data",
                "description": "QARTOD tests that GLOS uses for air temperature data on Seagull.",
                "tests": {
                    "qartod": {
                        "flat_line_test": {
                            "fail_threshold": 6,
                            "suspect_threshold": 2,
                            "tolerance": 0.1,
                        },
                        "gross_range_test": {
                            "fail_span": [227.55, 338.75],
                            "suspect_span": [255.45, 310.95],
                        },
                        "rate_of_change_test": {"threshold": 3.3},
                        "spike_test": {"fail_threshold": 3.3, "suspect_threshold": 2.2},
                    }
                },
            }
        }
    }

    library.apply_knowledge([{**KNOWLEDGE, **qc}])

    updated_standard = library.get("air_pressure_at_mean_sea_level")
    qc_test = updated_standard.qc[0]
    qc_info = qc_test.info()
    assert qc_info["name"] == "GLOS Seagull"
    assert qc_info["slug"] == "glos"
    assert qc_info["summary"] == "QARTOD tests that GLOS uses for Seagull data"

    config = qc_test.scaffold({})
    assert config["qartod"]["flat_line_test"]["fail_threshold"] == 6
    assert config["qartod"]["spike_test"]["fail_threshold"] == 3.3


def test_water_level_suites_from_knowledge(library):
    library.load_knowledge()
    standard = library.get("sea_surface_height_above_geopotential_datum")
    suites = {suite.info()["slug"]: suite for suite in standard.qc}
    assert set(suites) == {"gulf_of_maine", "long_island_sound"}

    config = suites["gulf_of_maine"].scaffold({"mllw": 0.0, "mhhw": 1.0})["qartod"]
    assert config["gross_range_test"]["suspect_span"] == [-1.3716000000000002, 2.8288]
    assert config["rate_of_change_test"]["threshold"] == 0.22860000000000003
    assert config["flat_line_test"]["suspect_threshold"] == 7200

    with pytest.raises(RuntimeError, match="Missing required argument: mllw"):
        suites["gulf_of_maine"].scaffold({"mhhw": 1.0})


def test_apply_knowledge_with_expressions(library):
    qc = {
        "qc": {
            "derived": {
                "name": "Derived",
                "summary": "Thresholds from statistics",
                "description": "Spike thresholds as multiples of a standard deviation",
                "arguments": {
                    "std": {"type": "float", "description": "Standard deviation"},
                    "k": {"type": "float", "description": "Multiplier", "default": 3},
                },
                "let": {"suspect": {"expr": "k * std"}},
                "tests": {
                    "qartod": {
                        "spike_test": {
                            "suspect_threshold": {"expr": "suspect"},
                            "fail_threshold": {"expr": "2 * suspect"},
                        }
                    }
                },
            }
        }
    }
    library.apply_knowledge([{**KNOWLEDGE, **qc}])
    suite = library.get("air_pressure_at_mean_sea_level").qc[0]

    config = suite.scaffold({"std": 0.5})["qartod"]["spike_test"]
    assert config == {"suspect_threshold": 1.5, "fail_threshold": 3.0}


def test_apply_knowledge_rejects_bad_expressions(library):
    qc = {
        "qc": {
            "broken": {
                "name": "Broken",
                "summary": "s",
                "description": "d",
                "tests": {
                    "qartod": {"rate_of_change_test": {"threshold": {"expr": "1 +"}}}
                },
            }
        }
    }
    with pytest.raises(ValueError, match="1 \\+"):
        library.apply_knowledge([{**KNOWLEDGE, **qc}])
