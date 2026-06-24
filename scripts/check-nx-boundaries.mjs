#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";

const dependencyFields = ["dependencies", "devDependencies", "peerDependencies", "optionalDependencies"];

function readJson(path) {
	return JSON.parse(readFileSync(path, "utf8"));
}

function projectFiles() {
	try {
		const output = execFileSync("rg", ["--files", "-g", "project.json", "apps", "libs", "tools"], {
			encoding: "utf8",
			stdio: ["ignore", "pipe", "pipe"],
		});
		return output.split("\n").filter((path) => path.endsWith("project.json"));
	} catch (error) {
		if (error?.code !== "ENOENT") throw error;
		return projectFilesFromNode(["apps", "libs", "tools"]);
	}
}

function projectFilesFromNode(roots) {
	const files = [];
	const visit = (dir) => {
		if (!existsSync(dir)) return;
		for (const entry of readdirSync(dir, { withFileTypes: true })) {
			const path = join(dir, entry.name);
			if (entry.isDirectory()) {
				visit(path);
			} else if (entry.isFile() && entry.name === "project.json") {
				files.push(path);
			}
		}
	};
	for (const root of roots) visit(root);
	return files.sort();
}

function tag(meta, prefix) {
	return [...meta.tags].find((value) => value.startsWith(`${prefix}:`))?.slice(prefix.length + 1);
}

function getDomain(meta) {
	return tag(meta, "domain");
}

function getLayer(meta) {
	return tag(meta, "layer");
}

function getScope(meta) {
	return tag(meta, "scope");
}

function errorsForProject(_name, meta) {
	const errors = [];
	const scope = getScope(meta);
	const type = tag(meta, "type");
	const domain = getDomain(meta);
	const layer = getLayer(meta);

	if (!scope) errors.push("missing scope tag");
	if (!type) errors.push("missing type tag");
	if (!domain) errors.push("missing domain tag");
	if (!layer) errors.push("missing layer tag");
	if (scope && !["oss", "cloud", "internal"].includes(scope)) errors.push(`unsupported scope tag ${scope}`);
	if (type && !["app", "lib", "workspace"].includes(type)) errors.push(`unsupported type tag ${type}`);
	if (type === "app" && layer !== "app") errors.push("app projects must use layer:app");
	if (type === "lib" && layer === "app") errors.push("library projects cannot use layer:app");
	if (layer === "core" && domain !== "shared") errors.push("core layer must use domain:shared");
	if (type === "workspace" && layer !== "tooling") errors.push("workspace projects must use layer:tooling");
	return errors;
}

function localPackageName(root) {
	const path = join(root, "package.json");
	if (!existsSync(path)) return undefined;
	return readJson(path).name;
}

function packageDependencies(root) {
	const path = join(root, "package.json");
	if (!existsSync(path)) return [];
	const pkg = readJson(path);
	return dependencyFields.flatMap((field) => Object.keys(pkg[field] ?? {}));
}

function addDependencyErrors(source, target, projectMeta, errors) {
	const sourceMeta = projectMeta.get(source);
	const targetMeta = projectMeta.get(target);
	if (!sourceMeta || !targetMeta) return;
	const sourceDomain = getDomain(sourceMeta);
	const targetDomain = getDomain(targetMeta);
	const sourceLayer = getLayer(sourceMeta);
	const targetLayer = getLayer(targetMeta);
	const sourceScope = getScope(sourceMeta);
	const targetScope = getScope(targetMeta);
	const sourceType = tag(sourceMeta, "type");

	if (source === target) {
		errors.push(`${source} depends on itself`);
		return;
	}
	if (sourceType === "workspace") return;
	if (sourceScope === "oss" && ["cloud", "internal"].includes(targetScope)) {
		errors.push(`${source} cannot depend on ${targetScope} project ${target}`);
	}
	if (sourceScope === "cloud" && targetScope === "internal") {
		errors.push(`${source} cannot depend on internal project ${target}`);
	}
	if (sourceLayer === "core") {
		errors.push(`${source} is core and must not depend on ${target}`);
		return;
	}
	if (sourceDomain === "drive" && targetDomain === "identity") {
		errors.push(`${source} cannot depend on identity project ${target}`);
	}
	if (sourceDomain === "identity" && targetDomain === "drive") {
		errors.push(`${source} cannot depend on drive project ${target}`);
	}
	if (sourceLayer === "app" && targetLayer === "app") {
		errors.push(`${source} cannot depend on app project ${target}`);
	}
	if (targetLayer === "app" && sourceLayer !== "app") {
		errors.push(`${source} cannot depend on app project ${target}`);
	}
	if (sourceLayer === "sdk" && targetLayer === "app") {
		errors.push(`${source} cannot depend on app project ${target}`);
	}
	if (sourceDomain === "shared" && targetDomain && targetDomain !== "shared") {
		errors.push(`${source} cannot depend on domain-specific project ${target}`);
	}
}

const projectMeta = new Map();
const packageToProject = new Map();
for (const path of projectFiles()) {
	const root = dirname(path).replace(/^\.\//, "");
	const data = readJson(path);
	const name = data.name;
	if (!name) continue;
	projectMeta.set(name, {
		root,
		tags: new Set(Array.isArray(data.tags) ? data.tags : []),
		projectType: data.projectType,
	});
	const packageName = localPackageName(root);
	if (packageName) packageToProject.set(packageName, name);
}

const dependencyErrors = [];
for (const [name, meta] of projectMeta.entries()) {
	for (const error of errorsForProject(name, meta)) dependencyErrors.push(`${name}: ${error}`);
	for (const dependency of packageDependencies(meta.root)) {
		const target = packageToProject.get(dependency);
		if (target) addDependencyErrors(name, target, projectMeta, dependencyErrors);
	}
}

if (dependencyErrors.length > 0) {
	console.error("Nx dependency boundary violations:");
	for (const error of dependencyErrors) console.error(`- ${error}`);
	process.exit(1);
}

console.log("Nx dependency boundaries: ok");
