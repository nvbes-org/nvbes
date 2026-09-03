export function successfulContinuousIntegrationRun(runs, { repository, sha }) {
	return runs.find(
		(run) =>
			run.repository?.full_name === repository &&
			run.path === ".github/workflows/ci.yml" &&
			run.event === "push" &&
			run.head_branch === "main" &&
			run.head_sha === sha &&
			run.status === "completed" &&
			run.conclusion === "success",
	);
}

export function validateDeploymentRevision({ ref, sha }) {
	if (ref !== "refs/heads/main") {
		throw new Error(
			`Production deployment requires refs/heads/main, received ${ref}`,
		);
	}
	if (!/^[0-9a-f]{40}$/u.test(sha)) {
		throw new Error("GITHUB_SHA must be a full Git commit SHA");
	}
}
