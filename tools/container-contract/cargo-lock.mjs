import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";

export function parseVersion(version) {
	const parts = version.split(".").map(Number);
	assert.ok(
		parts.length >= 2 && parts.length <= 3 && parts.every(Number.isInteger),
		`invalid Rust version: ${version}`,
	);
	return [parts[0], parts[1], parts[2] ?? 0];
}

export function compareVersions(left, right) {
	const a = parseVersion(left);
	const b = parseVersion(right);
	for (let index = 0; index < a.length; index += 1) {
		if (a[index] !== b[index]) return a[index] - b[index];
	}
	return 0;
}

function parseLockPackages(source) {
	const packages = [];
	for (const block of source.split(/(?=^\[\[package\]\]\s*$)/m)) {
		if (!block.startsWith("[[package]]")) continue;
		const name = block.match(/^name\s*=\s*"([^"]+)"/m)?.[1];
		const version = block.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
		assert.ok(
			name && version,
			"every Cargo.lock package must have a name and version",
		);
		const dependencyBlock =
			block.match(/^dependencies\s*=\s*\[([\s\S]*?)^\]/m)?.[1] ?? "";
		const dependencies = [
			...dependencyBlock.matchAll(/^\s*"([^"]+)",?\s*$/gm),
		].map((match) => match[1]);
		packages.push({ dependencies, key: `${name}@${version}`, name, version });
	}
	return packages;
}

function parseLockDependency(specification) {
	const withoutSource = specification.replace(/\s+\([^)]*\)$/, "");
	const match = withoutSource.match(
		/^(.+?)\s+(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)$/,
	);
	return match
		? { name: match[1], version: match[2] }
		: { name: withoutSource, version: undefined };
}

export function lockedPackageGraphSnapshot(lockfile, rootName) {
	const packages = parseLockPackages(lockfile);
	const byName = new Map();
	for (const pkg of packages) {
		const candidates = byName.get(pkg.name) ?? [];
		candidates.push(pkg);
		byName.set(pkg.name, candidates);
	}

	const roots = byName.get(rootName) ?? [];
	assert.equal(
		roots.length,
		1,
		`Cargo.lock must contain exactly one ${rootName} package`,
	);
	const pending = [roots[0]];
	const visited = new Map();

	while (pending.length > 0) {
		const pkg = pending.pop();
		if (visited.has(pkg.key)) continue;
		visited.set(pkg.key, pkg);

		for (const specification of pkg.dependencies) {
			const dependency = parseLockDependency(specification);
			const candidates = (byName.get(dependency.name) ?? []).filter(
				(candidate) =>
					!dependency.version || candidate.version === dependency.version,
			);
			assert.equal(
				candidates.length,
				1,
				`cannot uniquely resolve locked dependency ${specification} from ${pkg.key}`,
			);
			pending.push(candidates[0]);
		}
	}

	const packageIds = [...visited.keys()].sort();
	return createHash("sha256").update(packageIds.join("\n")).digest("hex");
}

export function offlineMetadataMsrv(workspaceRoot, rootName) {
	const result = spawnSync(
		"cargo",
		["metadata", "--locked", "--offline", "--format-version", "1"],
		{ cwd: workspaceRoot, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
	);
	if (result.status !== 0) {
		return { error: result.error?.message ?? result.stderr.trim() };
	}

	const metadata = JSON.parse(result.stdout);
	const packages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
	const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
	const root = metadata.packages.find((pkg) => pkg.name === rootName);
	assert.ok(root, `cargo metadata must contain ${rootName}`);

	const pending = [root.id];
	const visited = new Set();
	let msrv;
	while (pending.length > 0) {
		const id = pending.pop();
		if (visited.has(id)) continue;
		visited.add(id);
		const pkg = packages.get(id);
		if (
			pkg?.rust_version &&
			(!msrv || compareVersions(pkg.rust_version, msrv) > 0)
		) {
			msrv = pkg.rust_version;
		}
		for (const dependency of nodes.get(id)?.deps ?? []) {
			if (dependency.dep_kinds.some((kind) => kind.kind !== "dev")) {
				pending.push(dependency.pkg);
			}
		}
	}
	return { msrv };
}

