import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { profileOptions } from "../load-tests/account/profiles.js";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptDirectory, "../..");
const loadRoot = path.join(workspaceRoot, "tools/load-tests/account");

const profileContracts = {
	load: {
		droppedThreshold: "count==0",
		minimumIterations: 73_794,
		scenario: {
			executor: "ramping-arrival-rate",
			maxVUs: 400,
			preAllocatedVUs: 50,
			stages: [
				{ duration: "2m", target: 50 },
				{ duration: "10m", target: 50 },
				{ duration: "2m", target: 100 },
				{ duration: "5m", target: 100 },
				{ duration: "1m", target: 0 },
			],
			startRate: 5,
			timeUnit: "1s",
		},
	},
	scalability: {
		droppedThreshold: "count==0",
		minimumIterations: 88_200,
		scenario: {
			duration: "15m",
			executor: "constant-arrival-rate",
			gracefulStop: "1m",
			maxVUs: 1_000,
			preAllocatedVUs: 100,
			rate: 100,
			timeUnit: "1s",
		},
	},
	smoke: {
		minimumIterations: 1,
		scenario: { duration: "1m", executor: "constant-vus", vus: 1 },
	},
	soak: {
		droppedThreshold: "count==0",
		minimumIterations: 352_800,
		scenario: {
			duration: "4h",
			executor: "constant-arrival-rate",
			gracefulStop: "2m",
			maxVUs: 250,
			preAllocatedVUs: 50,
			rate: 25,
			timeUnit: "1s",
		},
	},
	spike: {
		droppedThreshold: "count==0",
		minimumIterations: 69_825,
		scenario: {
			executor: "ramping-arrival-rate",
			maxVUs: 1_000,
			preAllocatedVUs: 100,
			stages: [
				{ duration: "1m", target: 20 },
				{ duration: "15s", target: 500 },
				{ duration: "2m", target: 500 },
				{ duration: "15s", target: 20 },
				{ duration: "2m", target: 20 },
				{ duration: "30s", target: 0 },
			],
			startRate: 5,
			timeUnit: "1s",
		},
	},
	stress: {
		droppedThreshold: "count<=19110",
		minimumIterations: 171_990,
		scenario: {
			executor: "ramping-arrival-rate",
			gracefulStop: "1m",
			maxVUs: 1_000,
			preAllocatedVUs: 100,
			stages: [
				{ duration: "2m", target: 50 },
				{ duration: "3m", target: 100 },
				{ duration: "3m", target: 250 },
				{ duration: "5m", target: 500 },
				{ duration: "2m", target: 0 },
			],
			startRate: 10,
			timeUnit: "1s",
		},
		stressSlo: true,
	},
	volume: {
		minimumIterations: 100_000,
		scenario: {
			executor: "shared-iterations",
			iterations: 100_000,
			maxDuration: "30m",
			vus: 100,
		},
	},
};

if (path.resolve(process.argv[1] || "") === fileURLToPath(import.meta.url)) {
	const sources = {
		api: await readFile(path.join(loadRoot, "account-api.js"), "utf8"),
		metadata: await readFile(
			path.join(scriptDirectory, "write-load-metadata.mjs"),
			"utf8",
		),
		runner: await readFile(
			path.join(scriptDirectory, "run-account-k6.sh"),
			"utf8",
		),
		session: await readFile(path.join(loadRoot, "account-session.js"), "utf8"),
		verifier: await readFile(
			path.join(scriptDirectory, "verify-load-artifact.mjs"),
			"utf8",
		),
	};
	validateLoadProfiles();
	validateLoadSources(sources);
	process.stdout.write("Account load suite contract valid.\n");
}

export function validateLoadProfiles(resolveOptions = profileOptions) {
	for (const [profile, contract] of Object.entries(profileContracts)) {
		const actual = resolveOptions(profile, {});
		assert(isRecord(actual), `load profile ${profile} options are missing`);
		assertDeepEqual(
			actual,
			expectedOptions(profile, contract),
			`load profile ${profile}`,
		);
	}
}

function expectedOptions(profile, contract) {
	const standardSlo = {
		checks: ["rate>0.995"],
		http_req_duration: ["p(95)<400", "p(99)<800"],
		http_req_failed: ["rate<0.005"],
	};
	const stressSlo = {
		checks: ["rate>0.98"],
		http_req_duration: ["p(95)<1500", "p(99)<3000"],
		http_req_failed: ["rate<0.02"],
	};
	const thresholds = {
		...(contract.stressSlo ? stressSlo : standardSlo),
		iterations: [`count>=${contract.minimumIterations}`],
	};
	if (contract.droppedThreshold) {
		thresholds.dropped_iterations = [contract.droppedThreshold];
	}

	return {
		discardResponseBodies: false,
		maxRedirects: 0,
		noConnectionReuse: false,
		scenarios: {
			account: {
				...contract.scenario,
				tags: {
					profile,
					release: "local",
					replicas: "unknown",
					target_environment: "local",
				},
			},
		},
		setupTimeout: "30s",
		summaryTrendStats: ["avg", "min", "med", "max", "p(90)", "p(95)", "p(99)"],
		teardownTimeout: "30s",
		thresholds,
	};
}

function validateLoadSources(sources) {
	for (const source of [sources.api, sources.session]) {
		assert(
			source.includes("rejectLoadTarget"),
			"each load suite must enforce the target policy",
		);
		assert(
			!setupFunction(source).includes("return"),
			"setup must not return data because K6_SUMMARY_EXPORT serializes setup_data",
		);
	}
	assert(
		!sources.metadata.includes("ACCOUNT_TEST_COOKIE"),
		"load metadata must never read or serialize the session cookie",
	);
	assert(
		sources.verifier.includes("summary.setup_data"),
		"artifact verification must reject serialized setup data",
	);
	assert(
		sources.runner.includes("verify-load-artifact.mjs"),
		"runner must verify evidence redaction before succeeding",
	);
	assert(
		sources.runner.includes("validate-load-target.mjs"),
		"runner must enforce the exact target allowlist before k6",
	);
	assert(
		sources.runner.includes('--user "$(id -u):$(id -g)"'),
		"Docker k6 must write evidence as the host runner user",
	);
	for (const reservedVariable of ["K6_VUS", "K6_DURATION", "K6_ITERATIONS"]) {
		assert(
			sources.runner.includes(reservedVariable),
			`runner must enumerate ambient ${reservedVariable}`,
		);
	}
	assert(
		sources.runner.includes("$reserved_variable is reserved"),
		"runner must fail when a reserved k6 environment override is set",
	);
	assert(
		sources.session.includes("ACCOUNT_TEST_COOKIE"),
		"session suite must require a cookie",
	);
	assert(
		!sources.session.includes("console.log"),
		"session suite must never log credentials",
	);
}

function setupFunction(source) {
	const start = source.indexOf("export function setup()");
	const end = source.indexOf("\n}\n", start);
	assert(start >= 0 && end > start, "suite setup function is missing");
	return source.slice(start, end);
}

function assertDeepEqual(actual, expected, label) {
	assert(
		isDeepStrictEqual(actual, expected),
		`${label} differs from its exact contract`,
	);
}

function assert(condition, message) {
	if (!condition) {
		throw new Error(message);
	}
}

function isRecord(value) {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}
