import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { parse } from "@cdktn/hcl2json";

const ROOT = path.resolve(process.argv[2] ?? "infrastructure");
const RESOURCE_POLICIES = new Map([
	[
		"scaleway_container",
		{ minimumAttribute: "min_scale", maximumAttribute: "max_scale" },
	],
	[
		"scaleway_sdb_sql_database",
		{ minimumAttribute: "min_cpu", maximumAttribute: "max_cpu" },
	],
]);

async function terraformFiles(directory) {
	const entries = await readdir(directory, { withFileTypes: true });
	const files = [];

	for (const entry of entries) {
		if (entry.name === ".terraform") continue;
		const entryPath = path.join(directory, entry.name);
		if (entry.isDirectory()) files.push(...(await terraformFiles(entryPath)));
		else if (
			entry.isFile() &&
			(entry.name.endsWith(".tf") || entry.name.endsWith(".tf.json"))
		) {
			files.push(entryPath);
		}
	}

	return files;
}

function isUnsignedZero(value) {
	return typeof value === "number" && value === 0 && !Object.is(value, -0);
}

function isBoundedMaximum(value) {
	return (
		typeof value === "number" &&
		!Object.is(value, -0) &&
		(value === 0 || value === 1)
	);
}

function violation(file, resourceType, message) {
	return `${path.relative(process.cwd(), file)}: ${resourceType} ${message}`;
}

const violations = [];
for (const file of (await terraformFiles(ROOT)).sort()) {
	if (file.endsWith(".tf.json")) {
		violations.push(
			`${path.relative(process.cwd(), file)}: Terraform JSON syntax is not supported by the FinOps gate`,
		);
		continue;
	}

	let parsed;
	try {
		parsed = await parse(file, await readFile(file, "utf8"));
	} catch {
		violations.push(
			`${path.relative(process.cwd(), file)}: failed to parse Terraform HCL`,
		);
		continue;
	}

	for (const [resourceType, policy] of RESOURCE_POLICIES) {
		const resources = parsed.resource?.[resourceType];
		if (resources === undefined || typeof resources !== "object") continue;
		for (const bodies of Object.values(resources)) {
			for (const body of Array.isArray(bodies) ? bodies : []) {
				if (!isUnsignedZero(body[policy.minimumAttribute])) {
					violations.push(
						violation(
							file,
							resourceType,
							`must declare ${policy.minimumAttribute} = 0`,
						),
					);
				}
				if (!isBoundedMaximum(body[policy.maximumAttribute])) {
					violations.push(
						violation(
							file,
							resourceType,
							`must declare ${policy.maximumAttribute} as an integer <= 1`,
						),
					);
				}
			}
		}
	}
}

if (violations.length > 0) {
	console.error(violations.sort().join("\n"));
	process.exitCode = 1;
} else {
	console.log("FinOps scale bounds passed for all Scaleway runtimes.");
}
