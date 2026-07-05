import { check, sleep } from "k6";
import http from "k6/http";

export const options = {
	vus: Number(__ENV.K6_VUS || "1"),
	duration: __ENV.K6_DURATION || "30s",
	thresholds: {
		http_req_failed: ["rate<0.01"],
		http_req_duration: ["p(95)<500"],
	},
};

const identityApiBaseUrl =
	__ENV.IDENTITY_API_BASE_URL || "http://host.docker.internal:4000";

export default function identitySmoke() {
	const response = http.get(`${identityApiBaseUrl}/health`);

	check(response, {
		"identity health is 2xx": (result) =>
			result.status >= 200 && result.status < 300,
	});

	sleep(1);
}
