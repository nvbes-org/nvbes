import { dirname, relative, sep } from "node:path";

const globalRustPaths = new Set([
	"Cargo.lock",
	"Cargo.toml",
	"rust-toolchain.toml",
	"rustfmt.toml",
]);

function normalizedRelative(root, path) {
	return relative(root, path).split(sep).join("/");
}

export function selectAffectedCargoPackages(metadata, changedPaths, workspace) {
	const packages = metadata.packages.filter((item) =>
		metadata.workspace_members.includes(item.id),
	);
	const allNames = packages.map(({ name }) => name).sort();
	if (
		changedPaths.some(
			(path) =>
				globalRustPaths.has(path) ||
				path.startsWith(".cargo/") ||
				path.startsWith("vendor/") ||
				path.startsWith("contracts/protobuf/"),
		)
	) {
		return allNames;
	}

	const roots = packages.map((item) => ({
		name: item.name,
		root: normalizedRelative(workspace, dirname(item.manifest_path)),
	}));
	const selected = new Set();
	let unmatchedRustPath = false;
	for (const path of changedPaths) {
		if (!path.endsWith(".rs") && !path.endsWith("Cargo.toml")) continue;
		const owner = roots
			.filter(
				({ root }) =>
					path === `${root}/Cargo.toml` || path.startsWith(`${root}/`),
			)
			.sort((left, right) => right.root.length - left.root.length)[0];
		if (owner) selected.add(owner.name);
		else unmatchedRustPath = true;
	}
	if (unmatchedRustPath || selected.size === 0) return allNames;

	let changed = true;
	while (changed) {
		changed = false;
		for (const item of packages) {
			if (selected.has(item.name)) continue;
			if (
				item.dependencies.some((dependency) => selected.has(dependency.name))
			) {
				selected.add(item.name);
				changed = true;
			}
		}
	}
	return [...selected].sort();
}
