import { lookup } from "node:dns/promises";
import process from "node:process";
import { validateResolvedTargetOrigin } from "../load-tests/account/target-policy.js";

const kind = process.argv[2];
if (!["cloud", "service", "web"].includes(kind)) {
	throw new Error("Usage: node validate-load-target.mjs <cloud|service|web>");
}

const target =
	kind === "service"
		? process.env.ACCOUNT_SERVICE_BASE_URL
		: kind === "cloud"
			? process.env.NVBES_CLOUD_GRPC_ENDPOINT
			: process.env.NVBES_WEB_BASE_URL;
const allowedOrigins =
	kind === "service"
		? process.env.ACCOUNT_LOAD_ALLOWED_ORIGINS
		: process.env.ACCOUNT_WEB_ALLOWED_ORIGINS;

await validateResolvedTargetOrigin({
	allowedOrigins,
	kind: kind === "web" ? "web" : "service",
	productionOrigins: process.env.ACCOUNT_PRODUCTION_DENIED_ORIGINS,
	resolveHostname: (hostname) =>
		lookup(hostname, { all: true, verbatim: true }),
	target,
	targetEnvironment: process.env.NVBES_TARGET_ENV,
});

process.stdout.write(`Validated ${kind} quality target policy.\n`);
