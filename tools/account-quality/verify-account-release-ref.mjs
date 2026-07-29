import { pathToFileURL } from "node:url";

const RELEASE_SHA_PATTERN = /^[a-f0-9]{40}$/u;
const REPOSITORY_PATTERN = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u;

export async function verifyAccountReleaseRef(
	config,
	{ fetchImpl = fetch } = {},
) {
	const expected = validateConfiguration(config);
	const response = await fetchImpl(
		`${expected.apiUrl}/repos/${expected.repository}/git/ref/tags/${expected.refName}`,
		{
			headers: {
				Accept: "application/vnd.github+json",
				Authorization: `Bearer ${expected.token}`,
				"X-GitHub-Api-Version": "2022-11-28",
			},
			redirect: "error",
			signal: AbortSignal.timeout(60_000),
		},
	);
	assert(
		response.ok,
		`Account RC tag lookup failed with HTTP ${response.status}`,
	);
	const reference = await response.json();
	assert(
		reference?.ref === expected.ref &&
			reference.object?.type === "commit" &&
			reference.object?.sha === expected.releaseSha,
		"Account RC tag is annotated, moved, or does not resolve to the release SHA",
	);
	return {
		releaseSha: expected.releaseSha,
		tag: expected.refName,
	};
}

function validateConfiguration(config) {
	assert(
		typeof config.apiUrl === "string" &&
			config.apiUrl === new URL(config.apiUrl).origin &&
			config.apiUrl.startsWith("https://"),
		"GitHub API URL must be an exact HTTPS origin",
	);
	assert(
		REPOSITORY_PATTERN.test(config.repository ?? ""),
		"GitHub repository is invalid",
	);
	assert(
		RELEASE_SHA_PATTERN.test(config.releaseSha ?? ""),
		"release SHA must be a full immutable commit SHA",
	);
	const refName = `account-rc-${config.releaseSha}`;
	assert(
		config.refType === "tag",
		"Account release must be dispatched from an RC tag",
	);
	assert(
		config.refName === refName,
		"Account RC tag name does not bind the release SHA",
	);
	assert(
		config.ref === `refs/tags/${refName}`,
		"Account release ref is not the exact RC tag",
	);
	assert(
		typeof config.token === "string" && config.token !== "",
		"GitHub token is missing",
	);
	return { ...config, refName };
}

function assert(condition, message) {
	if (!condition) throw new Error(message);
}

async function main() {
	const env = process.env;
	const result = await verifyAccountReleaseRef({
		apiUrl: env.GH_API_URL,
		ref: env.NVBES_RELEASE_REF,
		refName: env.NVBES_RELEASE_REF_NAME,
		refType: env.NVBES_RELEASE_REF_TYPE,
		releaseSha: env.NVBES_RELEASE_SHA,
		repository: env.GH_REPOSITORY,
		token: env.GH_TOKEN,
	});
	process.stdout.write(
		`Verified immutable Account RC tag ${result.tag} -> ${result.releaseSha}.\n`,
	);
}

if (
	process.argv[1] &&
	import.meta.url === pathToFileURL(process.argv[1]).href
) {
	main().catch((error) => {
		process.stderr.write(
			`${error instanceof Error ? error.message : "Account RC tag verification failed"}\n`,
		);
		process.exitCode = 1;
	});
}
