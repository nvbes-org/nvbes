import { execFileSync } from "node:child_process";
import { relative } from "node:path";

function normalizePath(value) {
	return value.replaceAll("\\", "/");
}

function run(command, args) {
	return execFileSync(command, args, {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});
}

function isProductPath(path) {
	const normalized = normalizePath(path);
	return normalized.startsWith("libs/rust/products/") || normalized.startsWith("libs/ts/products/");
}

function scopeForPath(path) {
	const normalized = normalizePath(path);
	if (
		normalized.startsWith("apps/backoffice-") ||
		normalized.startsWith("apps/internal-") ||
		normalized.startsWith("libs/rust/internal/") ||
		normalized.startsWith("libs/ts/internal-") ||
		normalized.includes("/internal/")
	) {
		return "internal";
	}
	if (
		normalized.startsWith("apps/cloud-") ||
		normalized.startsWith("libs/rust/cloud/") ||
		normalized.startsWith("libs/rust/adapters-cloud/") ||
		normalized.startsWith("libs/ts/cloud-ui/")
	) {
		return "cloud";
	}
	if (normalized.startsWith("libs/rust/adapters-oss/")) {
		return "adapter";
	}
	if (normalized.startsWith("apps/")) {
		return "app";
	}
	return "oss";
}

function isRustServiceAppPackage(pkg) {
	const manifestPath = normalizePath(relative(process.cwd(), pkg.manifest_path));
	return manifestPath.startsWith("apps/") && /-service$/.test(pkg.name);
}

function checkRustProductPackages(errors, workspacePackages, packagesByName) {
	const directDeps = new Map();

	for (const pkg of workspacePackages) {
		directDeps.set(
			pkg.name,
			pkg.dependencies
				.map((dep) => packagesByName.get(dep.name))
				.filter(Boolean)
				.map((depPkg) => depPkg.name),
		);
	}

	function transitiveDeps(pkgName, seen = new Set()) {
		for (const depName of directDeps.get(pkgName) ?? []) {
			if (seen.has(depName)) continue;
			seen.add(depName);
			transitiveDeps(depName, seen);
		}
		return seen;
	}

	for (const pkg of workspacePackages) {
		const manifestPath = normalizePath(relative(process.cwd(), pkg.manifest_path));
		if (!isProductPath(manifestPath)) continue;

		for (const depName of transitiveDeps(pkg.name)) {
			const depPkg = packagesByName.get(depName);
			if (!depPkg) continue;
			const depPath = normalizePath(relative(process.cwd(), depPkg.manifest_path));
			const depScope = scopeForPath(depPath);
			if (["adapter", "cloud", "internal", "app"].includes(depScope)) {
				errors.push(`${pkg.name} cannot depend on ${depScope} package ${depName}`);
			}
		}
	}
}

function checkRustServiceAppPackageEdges(errors, workspacePackages, packagesByName) {
	const servicePackageNames = new Set(
		workspacePackages.filter(isRustServiceAppPackage).map((pkg) => pkg.name),
	);

	for (const pkg of workspacePackages) {
		if (!isRustServiceAppPackage(pkg)) continue;
		for (const dep of pkg.dependencies) {
			const depPkg = packagesByName.get(dep.name);
			if (!depPkg || !servicePackageNames.has(depPkg.name)) continue;
			errors.push(`${pkg.name} cannot depend on service app crate ${depPkg.name}`);
		}
	}
}

export function checkRustPackageBoundaries(errors) {
	const metadata = JSON.parse(run("cargo", ["metadata", "--format-version", "1", "--no-deps"]));
	const workspaceIds = new Set(metadata.workspace_members);
	const workspacePackages = metadata.packages.filter((pkg) => workspaceIds.has(pkg.id));
	const packagesByName = new Map(workspacePackages.map((pkg) => [pkg.name, pkg]));

	checkRustProductPackages(errors, workspacePackages, packagesByName);
	checkRustServiceAppPackageEdges(errors, workspacePackages, packagesByName);
}
