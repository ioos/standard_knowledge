# /// script
# requires-python = ">=3.13"
# dependencies = [
#     "beautifulsoup4",
#     "lxml",
#     "pyyaml",
#     "requests",
# ]
# [tool.uv]
# exclude-newer = "2025-07-17T00:00:00Z"
# ///
from pathlib import Path
import requests
from bs4 import BeautifulSoup
import yaml

commit = "2e7dbf1f335277bb979997c01f513a22056cdfaf"
version = "94"
standards_url = f"https://raw.githubusercontent.com/cf-convention/vocabularies/{commit}/docs/cf-standard-names/version/{version}/cf-standard-name-table.xml"
cf_yaml = Path(__file__).parent / "../core/standards/_cf_standards.yaml"

standard_names = {}
aliases = {}

print(
    f"Fetching CF standards version {version} from commit {commit}:\n  {standards_url}"
)

response = requests.get(standards_url)

soup = BeautifulSoup(response.text, "xml")

for node in soup.find_all("entry"):
    name = str(node.get("id"))
    standard_names[name] = {
        "unit": str(node.canonical_units.string),
        "description": str(node.description.string) or "",
    }

for node in soup.find_all("alias"):
    name = str(node.get("id"))
    aliases[name] = str(node.entry_id.string)


dump = {"standard_names": standard_names, "aliases": aliases}

with cf_yaml.open() as f:
    existing = yaml.safe_load(f)

with cf_yaml.open("w") as f:
    yaml.dump(dump, f)

print(
    f"Updated {len(standard_names)} CF standard names and {len(aliases)} aliases in {cf_yaml}"
)
if len(existing["standard_names"]) != len(standard_names) or len(
    existing["aliases"]
) != len(aliases):
    print(
        f"Previously had {len(existing['standard_names'])} standard names and {len(existing['aliases'])} aliases."
    )
