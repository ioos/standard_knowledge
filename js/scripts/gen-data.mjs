/**
 * Assembles public/ data files from the pre-generated data/ directory
 * (produced by utils/generate_partitions.py) for the Vite dev server and
 * Vitest browser suite.
 *
 * Run automatically as part of `npm run wasm`.
 * Requires data/ to exist — run `uv run --script utils/generate_partitions.py` first,
 * or `nox -s generate_partitions`.
 */

import {
	cpSync,
	mkdirSync,
	readdirSync,
	readFileSync,
	writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const dataDir = join(__dirname, "../../data");
const publicDir = join(__dirname, "../public");
const publicDataDir = join(publicDir, "data");

mkdirSync(join(publicDataDir, "partitions"), { recursive: true });

// Copy raw CF YAML for the Rust YAML-ingestion path
cpSync(
	join(dataDir, "cf_standards.yaml"),
	join(publicDir, "cf_standards.yaml"),
);
console.log("gen-data: copied cf_standards.yaml");

// Extract the flat knowledge array — data/all-knowledge.json has a
// {cf_standards, knowledge} wrapper, but public/knowledge.json must be the
// bare Vec<Knowledge> array that loadKnowledgeFromJson expects.
const allKnowledge = JSON.parse(
	readFileSync(join(dataDir, "all-knowledge.json"), "utf-8"),
);
writeFileSync(
	join(publicDir, "knowledge.json"),
	JSON.stringify(allKnowledge.knowledge),
);
console.log(
	`gen-data: extracted ${allKnowledge.knowledge.length} knowledge entries → public/knowledge.json`,
);

// Mirror all partition JSON files into public/data/ for Vite serving
for (const file of ["all-standards.json", "all-knowledge.json"]) {
	cpSync(join(dataDir, file), join(publicDataDir, file));
}

const partitionFiles = readdirSync(join(dataDir, "partitions")).filter((f) =>
	f.endsWith(".json"),
);
for (const file of partitionFiles) {
	cpSync(
		join(dataDir, "partitions", file),
		join(publicDataDir, "partitions", file),
	);
}

console.log(
	`gen-data: copied ${partitionFiles.length} partitions → public/data/`,
);
