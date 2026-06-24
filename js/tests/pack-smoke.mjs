#!/usr/bin/env node
// Smoke test of the *packaged* artifact, not the source tree. Runs `npm pack`
// on pkg/, installs the resulting tarball into a throwaway project, and imports
// it the way a consumer would. Catches packaging mistakes (missing files,
// wrong `main`/`types`, bad `files` field) that source-level tests cannot.

import { execSync } from "node:child_process";
import { mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const js = resolve(import.meta.dirname, "..");
const pkg = join(js, "pkg");

const run = (cmd, cwd) => execSync(cmd, { cwd, stdio: "inherit" });

const work = mkdtempSync(join(tmpdir(), "sk-pack-"));
try {
	// 1. Pack pkg/ into a tarball inside the throwaway dir.
	run(`npm pack ${pkg} --pack-destination ${work}`, js);
	const tgz = readdirSync(work).find((f) => f.endsWith(".tgz"));
	if (!tgz) throw new Error("npm pack produced no tarball");

	// 2. Install the tarball into a clean consumer project.
	run("npm init -y", work);
	run(`npm install ${join(work, tgz)}`, work);

	// 3. Import and exercise the installed package as a consumer would. The
	//    `--target web` build fetches its .wasm by URL, which Node's fetch won't
	//    do for file:// — so pass the wasm bytes to init() explicitly.
	//    Data is loaded from the generated public/ directory (produced by gen-data).
	const publicDir = join(js, "public");
	const smoke = join(work, "smoke.mjs");
	writeFileSync(
		smoke,
		`import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import init, { StandardsLibrary } from 'standard_knowledge_js';

const require = createRequire(import.meta.url);
const wasmPath = require.resolve('standard_knowledge_js/standard_knowledge_js_bg.wasm');
await init({ module_or_path: readFileSync(wasmPath) });

const cfYaml = readFileSync(${JSON.stringify(join(publicDir, "cf_standards.yaml"))}, 'utf-8');
const knowledgeJson = readFileSync(${JSON.stringify(join(publicDir, "knowledge.json"))}, 'utf-8');

const library = new StandardsLibrary();
library.loadCfStandardsFromYaml(cfYaml);
library.loadKnowledgeFromJson(knowledgeJson);

const standard = library.get('air_pressure_at_mean_sea_level');
if (standard.name !== 'air_pressure_at_mean_sea_level') {
  throw new Error('unexpected name: ' + standard.name);
}
if (standard.unit !== 'Pa') {
  throw new Error('unexpected unit: ' + standard.unit);
}
console.log('pack smoke test OK:', standard.name, standard.unit);
`,
	);
	run("node smoke.mjs", work);
	console.log("✓ Packaged tarball imports and runs correctly.");
} finally {
	rmSync(work, { recursive: true, force: true });
}
