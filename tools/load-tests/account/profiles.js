import { validateTargetOrigin } from "./target-policy.js";

export function profileOptions(profile, environment) {
	profileDuration(profile, environment.ACCOUNT_K6_DURATION);
	return {
		discardResponseBodies: false,
		maxRedirects: 0,
		noConnectionReuse: false,
		scenarios: {
			account: scenario(profile, environment),
		},
		setupTimeout: "30s",
		summaryTrendStats: ["avg", "min", "med", "max", "p(90)", "p(95)", "p(99)"],
		teardownTimeout: "30s",
		thresholds: thresholds(profile, environment),
	};
}

export function rejectLoadTarget(
	target,
	targetEnvironment,
	allowedOrigins = "",
	productionOrigins = "",
) {
	return validateTargetOrigin({
		allowedOrigins,
		kind: "service",
		productionOrigins,
		target,
		targetEnvironment,
	});
}

export function thinkTimeSeconds(value, fallback) {
	const raw = value === undefined || value === "" ? String(fallback) : value;
	if (!/^(?:0|[1-9]\d*)(?:\.\d+)?$/u.test(raw)) {
		throw new Error(
			"ACCOUNT_K6_THINK_TIME_SECONDS must be a non-negative decimal.",
		);
	}
	const parsed = Number(raw);
	if (!Number.isFinite(parsed) || parsed > 60) {
		throw new Error(
			"ACCOUNT_K6_THINK_TIME_SECONDS must be at most 60 seconds.",
		);
	}
	return parsed;
}

function scenario(profile, environment) {
	switch (profile) {
		case "smoke":
			return {
				executor: "constant-vus",
				vus: integer(environment.ACCOUNT_K6_VUS, 1),
				duration: profileDuration(profile, environment.ACCOUNT_K6_DURATION),
				tags: tags(profile, environment),
			};
		case "load":
			return {
				executor: "ramping-arrival-rate",
				startRate: integer(environment.ACCOUNT_K6_START_RATE, 5),
				timeUnit: "1s",
				preAllocatedVUs: integer(environment.ACCOUNT_K6_PREALLOCATED_VUS, 50),
				maxVUs: integer(environment.ACCOUNT_K6_MAX_VUS, 400),
				stages: [
					{
						target: integer(environment.ACCOUNT_K6_LOAD_RATE, 50),
						duration: "2m",
					},
					{
						target: integer(environment.ACCOUNT_K6_LOAD_RATE, 50),
						duration: "10m",
					},
					{
						target: integer(environment.ACCOUNT_K6_PEAK_RATE, 100),
						duration: "2m",
					},
					{
						target: integer(environment.ACCOUNT_K6_PEAK_RATE, 100),
						duration: "5m",
					},
					{ target: 0, duration: "1m" },
				],
				tags: tags(profile, environment),
			};
		case "volume":
			return {
				executor: "shared-iterations",
				vus: integer(environment.ACCOUNT_K6_VUS, 100),
				iterations: integer(environment.ACCOUNT_K6_ITERATIONS, 100_000),
				maxDuration: profileDuration(profile, environment.ACCOUNT_K6_DURATION),
				tags: tags(profile, environment),
			};
		case "spike":
			return {
				executor: "ramping-arrival-rate",
				startRate: integer(environment.ACCOUNT_K6_START_RATE, 5),
				timeUnit: "1s",
				preAllocatedVUs: integer(environment.ACCOUNT_K6_PREALLOCATED_VUS, 100),
				maxVUs: integer(environment.ACCOUNT_K6_MAX_VUS, 1_000),
				stages: [
					{ target: 20, duration: "1m" },
					{
						target: integer(environment.ACCOUNT_K6_SPIKE_RATE, 500),
						duration: "15s",
					},
					{
						target: integer(environment.ACCOUNT_K6_SPIKE_RATE, 500),
						duration: "2m",
					},
					{ target: 20, duration: "15s" },
					{ target: 20, duration: "2m" },
					{ target: 0, duration: "30s" },
				],
				tags: tags(profile, environment),
			};
		case "stress":
			return {
				executor: "ramping-arrival-rate",
				startRate: integer(environment.ACCOUNT_K6_START_RATE, 10),
				timeUnit: "1s",
				preAllocatedVUs: integer(environment.ACCOUNT_K6_PREALLOCATED_VUS, 100),
				maxVUs: integer(environment.ACCOUNT_K6_MAX_VUS, 1_000),
				stages: [
					{ target: 50, duration: "2m" },
					{ target: 100, duration: "3m" },
					{ target: 250, duration: "3m" },
					{
						target: integer(environment.ACCOUNT_K6_STRESS_RATE, 500),
						duration: "5m",
					},
					{ target: 0, duration: "2m" },
				],
				gracefulStop: "1m",
				tags: tags(profile, environment),
			};
		case "soak":
			return {
				executor: "constant-arrival-rate",
				rate: integer(environment.ACCOUNT_K6_LOAD_RATE, 25),
				timeUnit: "1s",
				duration: profileDuration(profile, environment.ACCOUNT_K6_DURATION),
				preAllocatedVUs: integer(environment.ACCOUNT_K6_PREALLOCATED_VUS, 50),
				maxVUs: integer(environment.ACCOUNT_K6_MAX_VUS, 250),
				gracefulStop: "2m",
				tags: tags(profile, environment),
			};
		case "scalability":
			return {
				executor: "constant-arrival-rate",
				rate: integer(environment.ACCOUNT_K6_LOAD_RATE, 100),
				timeUnit: "1s",
				duration: profileDuration(profile, environment.ACCOUNT_K6_DURATION),
				preAllocatedVUs: integer(environment.ACCOUNT_K6_PREALLOCATED_VUS, 100),
				maxVUs: integer(environment.ACCOUNT_K6_MAX_VUS, 1_000),
				gracefulStop: "1m",
				tags: tags(profile, environment),
			};
		default:
			throw new Error(`Unsupported K6_PROFILE=${profile}`);
	}
}

function thresholds(profile, environment) {
	const common =
		profile === "stress"
			? {
					checks: ["rate>0.98"],
					http_req_duration: ["p(95)<1500", "p(99)<3000"],
					http_req_failed: ["rate<0.02"],
				}
			: {
					checks: ["rate>0.995"],
					http_req_duration: ["p(95)<400", "p(99)<800"],
					http_req_failed: ["rate<0.005"],
				};

	const expectedIterations = minimumIterations(profile, environment);
	const result = {
		...common,
		iterations: [`count>=${expectedIterations}`],
	};

	if (["load", "scalability", "soak", "spike"].includes(profile)) {
		result.dropped_iterations = ["count==0"];
	} else if (profile === "stress") {
		result.dropped_iterations = [
			`count<=${Math.ceil(stressScheduledIterations(environment) * 0.1)}`,
		];
	}
	return result;
}

function minimumIterations(profile, environment) {
	switch (profile) {
		case "smoke":
			return integer(environment.ACCOUNT_K6_VUS, 1);
		case "load": {
			const start = integer(environment.ACCOUNT_K6_START_RATE, 5);
			const load = integer(environment.ACCOUNT_K6_LOAD_RATE, 50);
			const peak = integer(environment.ACCOUNT_K6_PEAK_RATE, 100);
			const scheduled =
				trapezoid(start, load, 120) +
				load * 600 +
				trapezoid(load, peak, 120) +
				peak * 300 +
				trapezoid(peak, 0, 60);
			return Math.floor(scheduled * 0.98);
		}
		case "volume":
			return integer(environment.ACCOUNT_K6_ITERATIONS, 100_000);
		case "spike": {
			const start = integer(environment.ACCOUNT_K6_START_RATE, 5);
			const spike = integer(environment.ACCOUNT_K6_SPIKE_RATE, 500);
			const scheduled =
				trapezoid(start, 20, 60) +
				trapezoid(20, spike, 15) +
				spike * 120 +
				trapezoid(spike, 20, 15) +
				20 * 120 +
				trapezoid(20, 0, 30);
			return Math.floor(scheduled * 0.98);
		}
		case "stress":
			return Math.floor(stressScheduledIterations(environment) * 0.9);
		case "soak": {
			const rate = integer(environment.ACCOUNT_K6_LOAD_RATE, 25);
			return Math.floor(
				rate *
					durationSeconds(
						profileDuration(profile, environment.ACCOUNT_K6_DURATION),
					) *
					0.98,
			);
		}
		case "scalability": {
			const rate = integer(environment.ACCOUNT_K6_LOAD_RATE, 100);
			return Math.floor(
				rate *
					durationSeconds(
						profileDuration(profile, environment.ACCOUNT_K6_DURATION),
					) *
					0.98,
			);
		}
		default:
			throw new Error(`Unsupported K6_PROFILE=${profile}`);
	}
}

function stressScheduledIterations(environment) {
	const start = integer(environment.ACCOUNT_K6_START_RATE, 10);
	const peak = integer(environment.ACCOUNT_K6_STRESS_RATE, 500);
	return (
		trapezoid(start, 50, 120) +
		trapezoid(50, 100, 180) +
		trapezoid(100, 250, 180) +
		trapezoid(250, peak, 300) +
		trapezoid(peak, 0, 120)
	);
}

function integer(value, fallback) {
	if (value === undefined || value === "") {
		return fallback;
	}

	const parsed = Number(value);
	if (!Number.isSafeInteger(parsed) || parsed <= 0) {
		throw new Error(`Expected a positive integer, received ${value}`);
	}
	return parsed;
}

export function profileDuration(profile, value) {
	const expected = {
		load: "20m",
		scalability: "15m",
		smoke: "1m",
		soak: "4h",
		spike: "6m",
		stress: "15m",
		volume: "30m",
	}[profile];

	if (!expected) {
		throw new Error(`Unsupported K6_PROFILE=${profile}`);
	}
	if (value === undefined || value === "" || value === "profile-default") {
		return expected;
	}
	if (value !== expected) {
		throw new Error(
			`ACCOUNT_K6_DURATION for ${profile} must be exactly ${expected}`,
		);
	}
	return value;
}

function durationSeconds(duration) {
	const match = /^(\d+)(s|m|h)$/u.exec(duration);
	if (!match) {
		throw new Error(`Unsupported exact duration: ${duration}`);
	}
	const multiplier = { h: 3_600, m: 60, s: 1 }[match[2]];
	return Number(match[1]) * multiplier;
}

function trapezoid(start, end, seconds) {
	return ((start + end) / 2) * seconds;
}

function tags(profile, environment) {
	return {
		profile,
		replicas: environment.ACCOUNT_K6_REPLICAS || "unknown",
		release: environment.NVBES_RELEASE || "local",
		target_environment: environment.NVBES_TARGET_ENV || "local",
	};
}
