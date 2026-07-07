import { existsSync, readFileSync, statSync } from "node:fs";

export function readPackageScripts(errors, path = "package.json") {
	try {
		return JSON.parse(readFileSync(path, "utf8")).scripts ?? {};
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return {};
	}
}

export function validateProofCommand(task, scripts) {
	const errors = [];
	for (const segment of task.proof.split("&&").map((part) => part.trim()).filter(Boolean)) {
		const command = segment.split(/\s+/);
		const [bin, firstArg] = command;
		if (bin === "pnpm") {
			validatePnpmSegment(task, command, scripts, errors);
			continue;
		}
		if (bin === "node") {
			validateToolPath(task, firstArg, errors);
			continue;
		}
		if (bin === "cargo") {
			validateCargoSegment(task, command, errors);
			continue;
		}
		if (bin === "go") {
			validateGoSegment(task, command, errors);
			continue;
		}
		if (bin === "tofu") {
			validateTofuSegment(task, command, errors);
			continue;
		}
		if (bin === "bash") {
			validateShellScript(task, firstArg, errors);
			continue;
		}
		if (bin?.startsWith("tools/migration/")) {
			validateToolPath(task, bin, errors);
			validateDirectToolExecutable(task, bin, errors);
			continue;
		}
		errors.push(`${task.id}: proof uses unsupported command segment ${segment}`);
	}
	return errors;
}

function validatePnpmSegment(task, command, scripts, errors) {
	const [, firstArg, secondArg, thirdArg] = command;
	if (firstArg === "--dir") {
		if (!existsSync(secondArg)) errors.push(`${task.id}: pnpm --dir path is missing: ${secondArg}`);
		if (!thirdArg) errors.push(`${task.id}: pnpm --dir requires a script`);
		else validatePackageScript(task, secondArg, thirdArg, errors);
		return;
	}
	if (!scripts[firstArg]) errors.push(`${task.id}: proof references missing package script ${firstArg}`);
}

function validatePackageScript(task, directory, script, errors) {
	const packagePath = `${directory}/package.json`;
	if (!existsSync(packagePath)) {
		errors.push(`${task.id}: pnpm --dir package.json is missing: ${packagePath}`);
		return;
	}
	let packageScripts;
	try {
		packageScripts = JSON.parse(readFileSync(packagePath, "utf8")).scripts ?? {};
	} catch (error) {
		errors.push(`${task.id}: pnpm --dir package.json is invalid: ${packagePath}: ${error.message}`);
		return;
	}
	if (!packageScripts[script]) errors.push(`${task.id}: pnpm --dir references missing package script ${directory}:${script}`);
}

function validateCargoSegment(task, command, errors) {
	const subcommand = command[1];
	if (!["check", "test"].includes(subcommand)) {
		errors.push(`${task.id}: unsupported cargo proof command ${subcommand}`);
	}
	if (!existsSync("Cargo.toml")) errors.push(`${task.id}: Cargo.toml is missing`);
}

function validateGoSegment(task, command, errors) {
	if (command[1] !== "test") errors.push(`${task.id}: unsupported go proof command ${command[1]}`);
	if (!existsSync("go.mod")) errors.push(`${task.id}: go.mod is missing`);
	for (const target of command.slice(2).filter((part) => part.startsWith("./"))) {
		if (!existsSync(target.slice(2))) errors.push(`${task.id}: go test target is missing: ${target}`);
	}
}

function validateTofuSegment(task, command, errors) {
	const chdir = command.find((part) => part.startsWith("-chdir="));
	if (!chdir) errors.push(`${task.id}: tofu proof requires -chdir=<path>`);
	else if (!existsSync(chdir.slice("-chdir=".length))) errors.push(`${task.id}: tofu chdir path is missing: ${chdir}`);
	if (!command.includes("validate")) errors.push(`${task.id}: tofu proof must run validate`);
}

function validateShellScript(task, scriptPath, errors) {
	if (!scriptPath?.startsWith("scripts/") || !scriptPath.endsWith(".sh")) {
		errors.push(`${task.id}: bash proof must use scripts/*.sh, got ${scriptPath}`);
		return;
	}
	if (!existsSync(scriptPath)) errors.push(`${task.id}: bash proof script is missing: ${scriptPath}`);
}

function validateToolPath(task, toolPath, errors) {
	if (!toolPath?.startsWith("tools/migration/")) {
		errors.push(`${task.id}: proof must use tools/migration scripts, got ${toolPath}`);
		return;
	}
	if (!toolPath.endsWith(".mjs")) {
		errors.push(`${task.id}: proof tool must be an .mjs script, got ${toolPath}`);
		return;
	}
	if (!existsSync(toolPath)) errors.push(`${task.id}: proof tool is missing: ${toolPath}`);
}

function validateDirectToolExecutable(task, toolPath, errors) {
	if (!existsSync(toolPath)) return;
	const executable = (statSync(toolPath).mode & 0o111) !== 0;
	if (!executable) {
		errors.push(`${task.id}: proof must use node ${toolPath} because the tool is not executable`);
	}
}
