/**
 * Generates tree-shakable ESM modules from the pre-built data/ directory
 * (produced by utils/generate_partitions.py).
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
 * Run: npm run build  (requires data/ to exist — run generate_partitions.py first)
 */

import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const dataDir = join(__dirname, "../../data");
const outDir = join(__dirname, "..");

function writeModule(path, data) {
	writeFileSync(
		path,
		`// @generated — run \`npm run build\` to regenerate\nexport default ${JSON.stringify(data)};\n`,
	);
}

// ── all-standards ─────────────────────────────────────────────────────────────

const allStandards = JSON.parse(
	readFileSync(join(dataDir, "all-standards.json"), "utf-8"),
);
writeModule(join(outDir, "all-standards.js"), allStandards);
console.log("build: wrote all-standards.js");

// ── per-category partitions ───────────────────────────────────────────────────

mkdirSync(join(outDir, "partitions"), { recursive: true });

const partitionFiles = readdirSync(join(dataDir, "partitions")).filter((f) =>
	f.endsWith(".json"),
);

for (const file of partitionFiles) {
	const slug = file.slice(0, -5); // strip .json
	const data = JSON.parse(
		readFileSync(join(dataDir, "partitions", file), "utf-8"),
	);
	writeModule(join(outDir, "partitions", `${slug}.js`), data);
}

console.log(
	`build: wrote ${partitionFiles.length} category partitions → partitions/`,
);

// ── all-knowledge ─────────────────────────────────────────────────────────────

const allKnowledge = JSON.parse(
	readFileSync(join(dataDir, "all-knowledge.json"), "utf-8"),
);
writeModule(join(outDir, "all-knowledge.js"), allKnowledge);
console.log("build: wrote all-knowledge.js");
