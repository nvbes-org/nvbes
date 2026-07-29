import { check, group, sleep } from "k6";
import http from "k6/http";
import {
	profileOptions,
	rejectLoadTarget,
	thinkTimeSeconds,
} from "./profiles.js";

const profile = __ENV.K6_PROFILE || "smoke";
const baseUrl = normalizedTarget(
	__ENV.ACCOUNT_SERVICE_BASE_URL || "http://host.docker.internal:4000",
);

export const options = profileOptions(profile, __ENV);

export function setup() {
	rejectLoadTarget(
		baseUrl,
		__ENV.NVBES_TARGET_ENV,
		__ENV.ACCOUNT_LOAD_ALLOWED_ORIGINS,
		__ENV.ACCOUNT_PRODUCTION_DENIED_ORIGINS,
	);

	const response = http.get(`${baseUrl}/health`, {
		tags: { endpoint: "health", operation: "setup" },
		timeout: "5s",
	});
	if (response.status !== 200) {
		throw new Error(
			`Account service readiness failed with status ${response.status}`,
		);
	}
}

export default function accountPublicJourney() {
	const journey = __ITER % 4;

	if (journey === 0) {
		health(baseUrl);
	} else if (journey === 1) {
		openidConfiguration(baseUrl);
	} else if (journey === 2) {
		supportedRegions(baseUrl);
	} else {
		passwordChangeMetadata(baseUrl);
	}

	sleep(thinkTimeSeconds(__ENV.ACCOUNT_K6_THINK_TIME_SECONDS, 0.2));
}

function health(target) {
	group("health", () => {
		const response = http.get(`${target}/health`, {
			tags: { endpoint: "health" },
			timeout: "5s",
		});
		check(response, {
			"health returns 200": (result) => result.status === 200,
			"health reports ok": (result) => json(result)?.status === "ok",
			"health exposes request id": (result) =>
				Boolean(result.headers["X-Request-Id"]),
		});
	});
}

function openidConfiguration(target) {
	group("openid configuration", () => {
		const response = http.get(`${target}/.well-known/openid-configuration`, {
			tags: { endpoint: "openid_configuration" },
			timeout: "5s",
		});
		check(response, {
			"openid configuration returns 200": (result) => result.status === 200,
			"openid configuration has issuer": (result) =>
				Boolean(json(result)?.issuer),
			"openid configuration has token endpoint": (result) =>
				Boolean(json(result)?.token_endpoint),
		});
	});
}

function supportedRegions(target) {
	group("supported regions", () => {
		const response = http.get(`${target}/auth/regions`, {
			tags: { endpoint: "supported_regions" },
			timeout: "5s",
		});
		check(response, {
			"supported regions returns 200": (result) => result.status === 200,
			"supported regions is JSON": (result) =>
				result.headers["Content-Type"]?.includes("application/json") === true,
		});
	});
}

function passwordChangeMetadata(target) {
	group("password change metadata", () => {
		const response = http.get(`${target}/.well-known/change-password`, {
			redirects: 0,
			tags: { endpoint: "change_password_metadata" },
			timeout: "5s",
		});
		check(response, {
			"password metadata is available": (result) =>
				result.status === 200 || result.status === 302 || result.status === 307,
		});
	});
}

function json(response) {
	try {
		return response.json();
	} catch {
		return undefined;
	}
}

function normalizedTarget(value) {
	return value.replace(/\/+$/u, "");
}
