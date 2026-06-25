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
