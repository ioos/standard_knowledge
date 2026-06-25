#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.12"
# dependencies = ["pyyaml"]
# ///
"""
Generate per-IOOS-category, all-knowledge, and all-standards partition
files from core/standards/*.yaml.

Writes compact JSON and human-readable YAML to data/:
  data/all-standards.{json,yaml}          — full CF vocabulary
  data/all-knowledge.{json,yaml}          — CF standards with knowledge + all knowledge
  data/partitions/{category}.{json,yaml}  — per-IOOS-category subset (self-contained)

Each partition file uses the format:
  { "cf_standards": { "standard_names": {...}, "aliases": {...} },
    "knowledge": [...] }

JSON is compact (browser-decompressable via DecompressionStream).
YAML is pretty-printed for human readability / diffing / sanity-checking.

Consumers call library.loadStandards(data.cf_standards) then
library.loadKnowledgeObjects(data.knowledge) to ingest a partition.

Run: uv run --script utils/generate_partitions.py
"""

import json
import re
from pathlib import Path

import yaml

ROOT = Path(__file__).parent.parent
STANDARDS_DIR = ROOT / "core" / "standards"
DATA_DIR = ROOT / "data"


def slugify(name: str) -> str:
    """'Sea Level' → 'sea_level', 'Temperature' → 'temperature'"""
    return re.sub(r"[^a-z0-9]+", "_", name.lower()).strip("_")


def load_cf() -> dict:
    with (STANDARDS_DIR / "_cf_standards.yaml").open() as f:
        data = yaml.safe_load(f)
    # YAML parses bare numbers (e.g. `unit: 1`) as Python ints/floats, but
    # Rust's CfStandard.{unit,description} are String fields. Coerce so the
    # JSON output is accepted by load_cf_standards_from_json.
    for std in data.get("standard_names", {}).values():
        if "unit" in std and not isinstance(std["unit"], str):
            std["unit"] = str(std["unit"])
        if "description" in std and not isinstance(std["description"], str):
            std["description"] = str(std["description"])
    return data


def load_knowledge() -> list[dict]:
    """Parse every per-standard YAML file into Knowledge-struct-compatible dicts."""
    items = []
    for path in sorted(STANDARDS_DIR.glob("*.yaml")):
        if path.name.startswith("_"):
            continue
        name = path.stem
        raw = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
        items.append(
            {
                "name": raw.get("name") or name,
                "long_name": raw.get("long_name"),
                "ioos_category": raw.get("ioos_category"),
                "common_variable_names": raw.get("common_variable_names") or [],
                "related_standards": raw.get("related_standards") or [],
                "sibling_standards": raw.get("sibling_standards") or [],
                "extra_attrs": raw.get("extra_attrs") or {},
                "other_units": raw.get("other_units") or [],
                # Normalise both singular ("comment") and plural ("comments") spellings
                "comments": raw.get("comments") or raw.get("comment"),
                "qc": raw.get("qc"),
            }
        )
    return items


def subset_cf(cf: dict, names: set[str]) -> dict:
    """Return a CfYaml-format dict containing only the requested standard names."""
    return {
        "standard_names": {k: v for k, v in cf["standard_names"].items() if k in names},
        "aliases": {k: v for k, v in cf.get("aliases", {}).items() if v in names},
    }


def emit(stem: Path, data: dict) -> None:
    """Write compact JSON and pretty YAML for the same data."""
    stem.parent.mkdir(parents=True, exist_ok=True)

    json_path = stem.with_suffix(".json")
    json_path.write_text(
        json.dumps(data, separators=(",", ":")) + "\n", encoding="utf-8"
    )

    yaml_path = stem.with_suffix(".yaml")
    yaml_path.write_text(
        yaml.dump(data, allow_unicode=True, sort_keys=True), encoding="utf-8"
    )

    kb_json = json_path.stat().st_size // 1024
    kb_yaml = yaml_path.stat().st_size // 1024
    rel = stem.relative_to(ROOT)
    print(f"  {rel}.json  ({kb_json} KB)  |  {rel}.yaml  ({kb_yaml} KB)")


def main() -> None:
    cf = load_cf()
    knowledge = load_knowledge()

    # 1. all-standards — the full CF vocabulary (no knowledge enrichment)
    emit(
        DATA_DIR / "all-standards",
        {
            "cf_standards": {
                "standard_names": cf["standard_names"],
                "aliases": cf.get("aliases", {}),
            }
        },
    )

    # 2. Per-IOOS-category partitions
    categories: dict[str, list[dict]] = {}
    for item in knowledge:
        if cat := item.get("ioos_category"):
            categories.setdefault(cat, []).append(item)

    for category, items in sorted(categories.items()):
        names = {item["name"] for item in items}
        emit(
            DATA_DIR / "partitions" / slugify(category),
            {"cf_standards": subset_cf(cf, names), "knowledge": items},
        )

    # 3. all-knowledge — every standard that has community knowledge
    all_names = {item["name"] for item in knowledge}
    emit(
        DATA_DIR / "all-knowledge",
        {"cf_standards": subset_cf(cf, all_names), "knowledge": knowledge},
    )

    print(f"\n{len(categories)} category partitions → {DATA_DIR.relative_to(ROOT)}/")


if __name__ == "__main__":
    main()
