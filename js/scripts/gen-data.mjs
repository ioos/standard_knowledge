/**
 * Generates public/cf_standards.yaml and public/knowledge.json from the
 * core/standards directory so the Vite dev server and Vitest browser suite
 * can serve them without embedding data in the WASM binary.
 *
 * Run automatically as part of `npm run wasm`.
 */

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";

const __dirname = dirname(fileURLToPath(import.meta.url));
const standardsDir = join(__dirname, "../../core/standards");
const publicDir = join(__dirname, "../public");

mkdirSync(publicDir, { recursive: true });

// Parse the CF vocabulary — both copy it for the Rust YAML path and parse it
// for building JSON partition files.
const cfYamlText = readFileSync(
	join(standardsDir, "_cf_standards.yaml"),
	"utf-8",
);
writeFileSync(join(publicDir, "cf_standards.yaml"), cfYamlText);
console.log("gen-data: copied cf_standards.yaml");
const cfRaw = parseYaml(cfYamlText);
// YAML parses bare numbers (e.g. `unit: 1`) as JS numbers, but Rust's
// CfStandard.{unit,description} are String fields. Coerce before JSON encoding.
for (const std of Object.values(cfRaw.standard_names ?? {})) {
	if (typeof std.unit === "number") std.unit = String(std.unit);
	if (typeof std.description === "number")
		std.description = String(std.description);
}
const cf = cfRaw;

// Combine every per-standard YAML file into a single knowledge.json that
// matches the Vec<Knowledge> JSON format expected by load_knowledge_from_json.
const knowledgeFiles = readdirSync(standardsDir)
	.filter((f) => f.endsWith(".yaml") && f !== "_cf_standards.yaml")
	.sort();

const knowledge = knowledgeFiles.map((file) => {
	const stem = basename(file, ".yaml");
	const raw = parseYaml(readFileSync(join(standardsDir, file), "utf-8")) ?? {};
	return {
		// name comes from the filename stem, matching the Rust build.rs logic
		name: raw.name ?? stem,
		long_name: raw.long_name ?? null,
		ioos_category: raw.ioos_category ?? null,
		common_variable_names: raw.common_variable_names ?? [],
		related_standards: raw.related_standards ?? [],
		sibling_standards: raw.sibling_standards ?? [],
		extra_attrs: raw.extra_attrs ?? {},
		other_units: raw.other_units ?? [],
		// Normalise both singular and plural spellings found in the YAML files
		comments: raw.comments ?? raw.comment ?? null,
		qc: raw.qc ?? null,
	};
});

writeFileSync(join(publicDir, "knowledge.json"), JSON.stringify(knowledge));
console.log(
	`gen-data: combined ${knowledge.length} knowledge files → public/knowledge.json`,
);

// ── Partition generation ──────────────────────────────────────────────────────
// Mirrors utils/generate_partitions.py but writes to public/data/ for Vite.

function slugify(name) {
	return name
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "_")
		.replace(/^_|_$/g, "");
}

function subsetCf(cfData, nameSet) {
	const standardNames = Object.fromEntries(
		Object.entries(cfData.standard_names ?? {}).filter(([k]) => nameSet.has(k)),
	);
	const aliases = Object.fromEntries(
		Object.entries(cfData.aliases ?? {}).filter(([, v]) => nameSet.has(v)),
	);
	return { standard_names: standardNames, aliases };
}

function writeJson(path, data) {
	writeFileSync(path, JSON.stringify(data));
}

const dataDir = join(publicDir, "data");
mkdirSync(join(dataDir, "partitions"), { recursive: true });

// all-standards — full CF vocabulary as JSON
writeJson(join(dataDir, "all-standards.json"), {
	cf_standards: {
		standard_names: cf.standard_names ?? {},
		aliases: cf.aliases ?? {},
	},
});

// Per-IOOS-category partitions
const categories = {};
for (const item of knowledge) {
	if (item.ioos_category) {
		if (!categories[item.ioos_category]) categories[item.ioos_category] = [];
		categories[item.ioos_category].push(item);
	}
}

for (const [category, items] of Object.entries(categories)) {
	const slug = slugify(category);
	const names = new Set(items.map((i) => i.name));
	writeJson(join(dataDir, "partitions", `${slug}.json`), {
		cf_standards: subsetCf(cf, names),
		knowledge: items,
	});
}

// all-knowledge — every standard with community knowledge
const allNames = new Set(knowledge.map((i) => i.name));
writeJson(join(dataDir, "all-knowledge.json"), {
	cf_standards: subsetCf(cf, allNames),
	knowledge,
});

console.log(
	`gen-data: generated ${Object.keys(categories).length} category partitions → public/data/`,
);
