/**
 * Generates public/cf_standards.yaml and public/knowledge.json from the
 * core/standards directory so the Vite dev server and Vitest browser suite
 * can serve them without embedding data in the WASM binary.
 *
 * Run automatically as part of `npm run wasm`.
 */

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "fs";
import { basename, dirname, join } from "path";
import { fileURLToPath } from "url";
import { parse as parseYaml } from "yaml";

const __dirname = dirname(fileURLToPath(import.meta.url));
const standardsDir = join(__dirname, "../../core/standards");
const publicDir = join(__dirname, "../public");

mkdirSync(publicDir, { recursive: true });

// Copy the CF vocabulary YAML as-is — the Rust YAML parser handles it.
const cfYaml = readFileSync(join(standardsDir, "_cf_standards.yaml"));
writeFileSync(join(publicDir, "cf_standards.yaml"), cfYaml);
console.log("gen-data: copied cf_standards.yaml");

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
