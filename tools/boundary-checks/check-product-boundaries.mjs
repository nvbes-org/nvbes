#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const errors = [];
const textExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs"]);
const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);

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

function checkRustProductPackages() {
	const metadata = JSON.parse(run("cargo", ["metadata", "--format-version", "1", "--no-deps"]));
	const workspaceIds = new Set(metadata.workspace_members);
	const workspacePackages = metadata.packages.filter((pkg) => workspaceIds.has(pkg.id));
	const packagesByName = new Map(workspacePackages.map((pkg) => [pkg.name, pkg]));
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

function shouldScan(path) {
	if (path.split("/").some((part) => skippedDirs.has(part))) return false;
	const dot = path.lastIndexOf(".");
	return dot >= 0 && textExtensions.has(path.slice(dot));
}

function walk(dir, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			walk(path, results);
		} else {
			const relativePath = normalizePath(relative(process.cwd(), path));
			if (shouldScan(relativePath)) results.push(relativePath);
		}
	}
	return results;
}

function checkProductSourceImports() {
	for (const root of ["libs/rust/products", "libs/ts/products"]) {
		for (const file of walk(root)) {
			const content = readFileSync(file, "utf8");
			const forbiddenPatterns = [
				/libs\/rust\/adapters-(?:oss|cloud)\//,
				/adapters[-_](?:oss|cloud)/,
				/libs\/ts\/cloud-ui/,
				/from\s+["'][^"']*cloud-ui["']/,
				/from\s+["'][^"']*apps\//,
			];

			for (const pattern of forbiddenPatterns) {
				if (pattern.test(content)) {
					errors.push(`${file}: product source imports forbidden boundary ${pattern}`);
				}
			}
		}
	}
}

checkRustProductPackages();
checkProductSourceImports();

if (errors.length > 0) {
	console.error("Product boundary violations:");
	for (const error of errors) {
		console.error(`- ${error}`);
	}
	process.exit(1);
}

console.log("Product boundaries: ok");
