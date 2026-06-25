/**
 * Generates tree-shakable ESM modules from core/standards/*.yaml.
 *
 * Output (all gitignored — regenerate with `npm run build`):
 *   all-standards.js         — full CF vocabulary
 *   all-knowledge.js         — CF standards subset + all community knowledge
 *   partitions/{slug}.js     — per-IOOS-category self-contained subsets
 *
 * Each file exports a single default object:
 *   { cf_standards: { standard_names, aliases }, knowledge: [...] }
 *
 * Consumers load a partition by importing the specific subpath:
 *   import met from "standard_knowledge_data/partitions/meteorology"
 *
 * Run: npm run build  (requires `npm ci` first)
 */

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parse as parseYaml } from "yaml";

const __dirname = dirname(fileURLToPath(import.meta.url));
const standardsDir = join(__dirname, "../../core/standards");
const outDir = join(__dirname, "..");

function slugify(name) {
	return name
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, "_")
		.replace(/^_|_$/g, "");
}

function writeModule(path, data) {
	writeFileSync(
		path,
		`// @generated — run \`npm run build\` to regenerate\nexport default ${JSON.stringify(data)};\n`,
	);
}

// ── Load CF vocabulary ────────────────────────────────────────────────────────

const cfYamlText = readFileSync(
	join(standardsDir, "_cf_standards.yaml"),
	"utf-8",
);
const cfRaw = parseYaml(cfYamlText);

// YAML parses bare numbers (e.g. `unit: 1`) as JS numbers, but Rust's
// CfStandard.{unit,description} are String fields. Coerce before JSON encoding.
for (const std of Object.values(cfRaw.standard_names ?? {})) {
	if (typeof std.unit === "number") std.unit = String(std.unit);
	if (typeof std.description === "number")
		std.description = String(std.description);
}

// ── Load community knowledge ──────────────────────────────────────────────────

const knowledgeFiles = readdirSync(standardsDir)
	.filter((f) => f.endsWith(".yaml") && f !== "_cf_standards.yaml")
	.sort();

const knowledge = knowledgeFiles.map((file) => {
	const stem = basename(file, ".yaml");
	const raw = parseYaml(readFileSync(join(standardsDir, file), "utf-8")) ?? {};
	return {
		name: raw.name ?? stem,
		long_name: raw.long_name ?? null,
		ioos_category: raw.ioos_category ?? null,
		common_variable_names: raw.common_variable_names ?? [],
		related_standards: raw.related_standards ?? [],
		sibling_standards: raw.sibling_standards ?? [],
		extra_attrs: raw.extra_attrs ?? {},
		other_units: raw.other_units ?? [],
		comments: raw.comments ?? raw.comment ?? null,
		qc: raw.qc ?? null,
	};
});

function subsetCf(nameSet) {
	return {
		standard_names: Object.fromEntries(
			Object.entries(cfRaw.standard_names ?? {}).filter(([k]) =>
				nameSet.has(k),
			),
		),
		aliases: Object.fromEntries(
			Object.entries(cfRaw.aliases ?? {}).filter(([, v]) => nameSet.has(v)),
		),
	};
}

// ── Emit all-standards ────────────────────────────────────────────────────────

writeModule(join(outDir, "all-standards.js"), {
	cf_standards: {
		standard_names: cfRaw.standard_names ?? {},
		aliases: cfRaw.aliases ?? {},
	},
});
console.log("build: wrote all-standards.js");

// ── Emit per-category partitions ──────────────────────────────────────────────

mkdirSync(join(outDir, "partitions"), { recursive: true });

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
	writeModule(join(outDir, "partitions", `${slug}.js`), {
		cf_standards: subsetCf(names),
		knowledge: items,
	});
}

console.log(
	`build: wrote ${Object.keys(categories).length} category partitions → partitions/`,
);

// ── Emit all-knowledge ────────────────────────────────────────────────────────

const allNames = new Set(knowledge.map((i) => i.name));
writeModule(join(outDir, "all-knowledge.js"), {
	cf_standards: subsetCf(allNames),
	knowledge,
});
console.log("build: wrote all-knowledge.js");
