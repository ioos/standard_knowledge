#!/usr/bin/env -S uv run --script
# /// script
# dependencies = ["nanodjango", "erddapy", "pandas", "pyyaml"]
# ///
import urllib.error
from pathlib import Path

import erddapy
import pandas as pd
import yaml
from django.db import models
from django.http import HttpResponse
from nanodjango import Django

KNOWLEDGE_YAML_PATH = Path(__file__).parent / "../core/standards/"

app = Django()


class Dataset(models.Model):
    dataset_id = models.CharField(max_length=255)
    server_id = models.CharField(max_length=255)
    loaded = models.BooleanField(default=False)

    class Meta:
        unique_together = ("dataset_id", "server_id")


class Variable(models.Model):
    dataset = models.ForeignKey(Dataset, on_delete=models.CASCADE)
    variable_name = models.CharField(max_length=255)
    ioos_category = models.CharField(max_length=255, null=True, blank=True)
    standard_name = models.CharField(max_length=255, null=True, blank=True)

    class Meta:
        unique_together = ("dataset", "variable_name")


def datasets_on_server(server_url_or_id: str) -> list[str]:
    e = erddapy.ERDDAP(server_url_or_id)
    url = e.get_search_url(search_for="ioos_category", response="csv")
    df = pd.read_csv(url)
    return df["Dataset ID"].tolist()


@app.api.get("/servers")
def list_servers(request):
    return erddapy.servers


@app.api.get("/server/{server_id}/load")
def load_server_datasets(request, server_id: str):
    dataset_ids = datasets_on_server(server_id)
    for dataset_id in dataset_ids:
        Dataset.objects.get_or_create(
            dataset_id=dataset_id, server_id=server_id, loaded=False
        )

    return {"message": f"Loaded {len(dataset_ids)} datasets from server {server_id}"}


def find_variables_and_categories_for_standards(server: str, dataset_id: str):
    """Given a server and dataset ID, return the variables and their IOOS categories when they have a standard_name."""
    e = erddapy.ERDDAP(server)
    url = e.get_info_url(dataset_id=dataset_id, response="csv")
    df = pd.read_csv(url)

    for var_name, group in df.groupby("Variable Name"):
        if var_name == "NC_GLOBAL":
            continue
        attrs = dict(group[["Attribute Name", "Value"]].values)
        standard_name: str | None = None
        ioos_category: str | None = None
        if "standard_name" in attrs:
            standard_name = attrs["standard_name"]
        if "ioos_category" in attrs:
            ioos_category = attrs["ioos_category"]
        if standard_name:
            yield {
                "variable_name": var_name,
                "standard_name": standard_name,
                "ioos_category": ioos_category,
            }


@app.api.get("/variables/load/random")
def load_random_variable(request, num_datasets: int = 100):
    """Load variables from random unloaded datasets"""
    datasets = Dataset.objects.filter(loaded=False).order_by("?")[:num_datasets]
    for dataset in datasets:
        try:
            for var_info in find_variables_and_categories_for_standards(
                dataset.server_id, dataset.dataset_id
            ):
                Variable.objects.get_or_create(
                    dataset=dataset,
                    variable_name=var_info["variable_name"],
                    standard_name=var_info["standard_name"],
                    ioos_category=var_info["ioos_category"],
                )
            dataset.loaded = True
            dataset.save()
        except (urllib.error.HTTPError, urllib.error.URLError) as e:
            print(
                f"Failed to load dataset {dataset.dataset_id} from server {dataset.server_id}: {e}"
            )
            continue


@app.api.get("/standard/")
def list_standards(request):
    """Return a list of all unique standard names in the database."""
    standards = (
        Variable.objects.filter(standard_name__isnull=False)
        .values_list("standard_name", flat=True)
        .distinct()
        .order_by("standard_name")
    )
    return {"standards": list(standards)}


@app.api.get("/standard/{standard_name}")
def variables_by_standard(request, standard_name: str):
    """Return the count of each variable name and ioos category for a given standard name."""
    variables = Variable.objects.filter(standard_name=standard_name)
    result = {}
    for var in variables:
        key = (var.variable_name, var.ioos_category)
        if key not in result:
            result[key] = 0
        result[key] += 1

    response = {
        "standard_name": standard_name,
        "variables": [
            {"variable_name": var_name, "ioos_category": ioos_category, "count": count}
            for (var_name, ioos_category), count in result.items()
        ],
    }
    # sort results by count descending
    response["variables"].sort(key=lambda x: x["count"], reverse=True)

    return response


@app.api.get("/standard/{standard_name}/as_knowledge")
def standard_as_knowledge(
    request, standard_name: str, merge: bool = False, write: bool = False
):
    """Return the standard as a knowledge yaml.

    - merge: if True, merge with existing knowledge yaml if it exists.
    - write: if True, write the knowledge yaml to disk.
    """
    variables = Variable.objects.filter(standard_name=standard_name)
    most_common_ioos_category = (
        variables.filter(ioos_category__isnull=False)
        .values("ioos_category")
        .annotate(count=models.Count("ioos_category"))
        .order_by("-count")
        .first()
    )
    variable_names_by_use_count = (
        variables.annotate(lower_name=models.functions.Lower("variable_name"))
        .values("lower_name")
        .annotate(count=models.Count("lower_name"))
        .order_by("-count")
    )
    variable_names = [v["lower_name"] for v in variable_names_by_use_count]
    knowledge = {
        "standard_name": standard_name,
        "ioos_category": most_common_ioos_category["ioos_category"]
        if most_common_ioos_category
        else None,
        "common_variable_names": variable_names,
    }

    path = KNOWLEDGE_YAML_PATH / f"{standard_name}.yaml"

    if merge:
        if path.exists():
            with path.open() as f:
                existing_knowledge = yaml.safe_load(f)
            knowledge = {**existing_knowledge, **knowledge}
        else:
            print(f"No existing knowledge file found for {standard_name}")

    if write:
        with path.open("w") as f:
            yaml.dump(knowledge, f)

    print(path)

    return HttpResponse(yaml.dump(knowledge), content_type="text/yaml")


@app.api.get("/standard/{standard_name}/{var_name}")
def dataset_urls_for_a_variable_and_standard(
    request, standard_name: str, var_name: str
):
    """Return the dataset URLs for a given variable name and standard name."""
    variables = Variable.objects.filter(
        standard_name=standard_name, variable_name=var_name
    )
    result = []
    for var in variables:
        dataset = var.dataset
        server = erddapy.servers.get(dataset.server_id)[-1]
        result.append(
            {
                "dataset_id": dataset.dataset_id,
                "server_id": dataset.server_id,
                "url": f"{server}tabledap/{dataset.dataset_id}.html",
            }
        )

    return {
        "standard_name": standard_name,
        "variable_name": var_name,
        "datasets": result,
    }


@app.api.get("/stats")
def stats(request):
    """Return some basic stats about the datasets and variables we've loaded."""
    total_datasets = Dataset.objects.count()
    loaded_datasets = Dataset.objects.filter(loaded=True).count()
    total_variables = Variable.objects.count()
    variables_with_standards = Variable.objects.filter(
        standard_name__isnull=False
    ).count()
    variables_with_categories = Variable.objects.filter(
        ioos_category__isnull=False
    ).count()

    return {
        "total_datasets": total_datasets,
        "loaded_datasets": loaded_datasets,
        "unloaded_datasets": total_datasets - loaded_datasets,
        "total_variables": total_variables,
        "variables_with_standards": variables_with_standards,
        "variables_with_ioos_categories": variables_with_categories,
    }


if __name__ == "__main__":
    app.run()
