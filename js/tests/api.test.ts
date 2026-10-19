import { beforeAll, describe, expect, test } from "vitest";
import init, { StandardsLibrary } from "../pkg/standard_knowledge_js.js";

// Exercises the exact ESM + WASM artifact published to npm (the `pkg/`
// directory). Values asserted here are backed by core Rust/Python tests, so
// they are stable: see core/src/standards_library.rs and
// py/tests/test_standards_library.py.

let library: StandardsLibrary;

beforeAll(async () => {
	// `--target web` build: must instantiate the WASM module before use. In a
	// browser context Vite serves the sibling .wasm, so the default init works.
	await init();
	library = new StandardsLibrary();
	library.loadCfStandards();
	library.loadKnowledge();
});

describe("get", () => {
	test("resolves a standard by its CF name", () => {
		const standard = library.get("air_pressure_at_mean_sea_level");
		expect(standard.name).toBe("air_pressure_at_mean_sea_level");
		expect(standard.unit).toBe("Pa");
	});

	test("resolves a standard by alias", () => {
		const standard = library.get("air_pressure_at_sea_level");
		expect(standard.name).toBe("air_pressure_at_mean_sea_level");
	});

	test("exposes community knowledge once loaded", () => {
		const standard = library.get("air_pressure_at_mean_sea_level");
		expect(standard.ioosCategory).toBe("Meteorology");
		expect(standard.commonVariableNames).toContain("pressure");
	});

	test("throws for an unknown name (the x-get-standard error path)", () => {
		expect(() => library.get("not_a_real_standard")).toThrow();
	});
});

describe("filter", () => {
	test("byVariableName matches on a common variable name", () => {
		const standards = library
			.filter()
			.byVariableName("atmospheric_pressure").standards;
		expect(standards.map((s) => s.name)).toContain(
			"air_pressure_at_mean_sea_level",
		);
	});

	test("byIoosCategory narrows to a single category", () => {
		const standards = library.filter().byIoosCategory("Meteorology").standards;
		expect(standards.length).toBeGreaterThan(0);
		expect(standards.every((s) => s.ioosCategory === "Meteorology")).toBe(true);
	});

	test("byUnit narrows to a single unit", () => {
		const standards = library.filter().byUnit("Pa").standards;
		expect(standards.length).toBeGreaterThan(0);
		expect(standards.every((s) => s.unit === "Pa")).toBe(true);
	});

	test("hasQartodTests only returns standards with QARTOD tests", () => {
		const standards = library.filter().hasQartodTests().standards;
		expect(standards.length).toBeGreaterThan(0);
		expect(standards.every((s) => s.qartod.length > 0)).toBe(true);
	});

	test("chained filters compose", () => {
		const all = library.filter().standards.length;
		const filtered = library
			.filter()
			.byUnit("Pa")
			.byIoosCategory("Meteorology").standards;
		expect(filtered.length).toBeGreaterThan(0);
		expect(filtered.length).toBeLessThan(all);
	});

	test("search returns matching standards", () => {
		const standards = library.filter().search("pressure").standards;
		expect(standards.length).toBeGreaterThan(0);
	});
});

describe("standard metadata", () => {
	test("attrs() returns suggested xarray attributes", () => {
		const attrs = library.get("air_pressure_at_mean_sea_level").attrs();
		const obj = Object.fromEntries(attrs);
		expect(obj.standard_name).toBe("air_pressure_at_mean_sea_level");
	});

	test("knownIoosCategories includes the expected categories", () => {
		const categories = library.knownIoosCategories();
		expect(categories).toContain("Meteorology");
		expect(categories).toContain("Temperature");
		expect(categories).toContain("Wind");
	});
});

describe("qartod test suite scaffolding", () => {
	const manual = () => {
		const suite = library
			.get("sea_surface_temperature")
			.qartod.find((q) => q.slug === "qartod_manual");
		if (!suite) throw new Error("no qartod_manual suite");
		return suite;
	};
	const args = { mean: 5, std: 3, sensor_min: 1, sensor_max: 15 };

	test("tests lists the suite's tests", () => {
		const tests = manual().tests;
		expect(tests).toHaveLength(4);
		expect(tests).toContain("gross_range_test");
		expect(tests).toContain("rate_of_change_test");
	});

	test("arguments describes each argument as a plain object", () => {
		const suiteArgs = manual().arguments;
		expect(suiteArgs.mean.required).toBe(true);
		expect(suiteArgs.mean.type).toBe("float");
		expect(suiteArgs.k.default).toBe(3);
		expect(suiteArgs.sample_interval.type).toBe("int");
	});

	test("name, slug and description still work", () => {
		const suite = manual();
		expect(suite.name).toBeTruthy();
		expect(suite.description).toBeTruthy();
	});

	test("scaffold builds a config for the requested tests", () => {
		const config = manual().scaffold(args, ["gross_range_test"]);
		expect(config).toEqual({
			qartod: {
				gross_range_test: { fail_span: [1, 15], suspect_span: [-4, 14] },
			},
		});
	});

	test("scaffold reports every missing argument", () => {
		expect(() => manual().scaffold(args)).toThrow(Error);
		expect(() => manual().scaffold(args)).toThrow(/sample_interval/);
		expect(() => manual().scaffold(args)).toThrow(/rate_of_change_test/);
	});

	test("scaffold rejects a fractional int argument", () => {
		expect(() => manual().scaffold({ ...args, sample_interval: 1.5 })).toThrow(
			/sample_interval/,
		);
	});
});
