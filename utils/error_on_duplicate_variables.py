#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.13"
# dependencies = ["pyyaml"]
# [tool.uv]
# exclude-newer = "2025-07-17T00:00:00Z"
# ///
"""
Throw an error when standard knowledge are duplicated in the repo
"""

import sys
from pathlib import Path

import yaml

ROOT_DIR = Path(__file__).parent.parent
STANDARDS_PATH = ROOT_DIR / "core" / "standards"


def find_duplicate_variables(knowledge_path: Path) -> list[str]:
    """
    Find duplicate variable names in the standard knowledge YAML files.

    Args:
        knowledge_path (Path): a knowledge YAML file
    """
    variable_names = set()
    duplicate_variables = []

    with knowledge_path.open() as f:
        data = yaml.safe_load(f)

    for variable in data.get("common_variable_names", []):
        if variable in variable_names:
            duplicate_variables.append(variable)
        else:
            variable_names.add(variable)

    return duplicate_variables


def main() -> None:
    """
    Main function to check for duplicate variable names in the standard knowledge YAML files.
    """
    duplicates = []

    for yaml_file in STANDARDS_PATH.glob("*.yaml"):
        if yaml_file.name == "_cf_standards.yaml":
            continue  # Skip the CF standards file
        duplicate_vars = find_duplicate_variables(yaml_file)
        if duplicate_vars:
            duplicates.append((yaml_file, duplicate_vars))

    if duplicates:
        print("Duplicate variable names found in the following files:")
        for file_path, vars in duplicates:
            print(f"- {file_path.relative_to(ROOT_DIR)}: {', '.join(vars)}")
        sys.exit(1)
    else:
        print("No duplicate variable names found.")
        sys.exit(0)


if __name__ == "__main__":
    main()
