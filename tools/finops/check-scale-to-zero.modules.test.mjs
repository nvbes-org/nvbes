import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdir, mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const checker = fileURLToPath(
	new URL("./check-scale-to-zero.mjs", import.meta.url),
);

async function createFixture(t, files) {
	const base = await mkdtemp(path.join(tmpdir(), "nvbes-scale-modules-"));
	t.after(() => rm(base, { recursive: true, force: true }));
	for (const [relativePath, contents] of Object.entries(files)) {
		const file = path.join(base, relativePath);
		await mkdir(path.dirname(file), { recursive: true });
		await writeFile(file, contents);
	}
	return base;
}

function runChecker(root) {
	return spawnSync(process.execPath, [checker, root], { encoding: "utf8" });
}

test("rejects a local module source that escapes the FinOps root", async (t) => {
	const base = await createFixture(t, {
		"infrastructure/main.tf": 'module "outside" { source = "../outside" }',
		"outside/runtime.tf":
			'resource "scaleway_container" "unbounded" { min_scale = 0 max_scale = 10 }',
	});
	const root = path.join(base, "infrastructure");
	const result = runChecker(root);
	const main = path.relative(process.cwd(), path.join(root, "main.tf"));

	assert.equal(result.status, 1, result.stderr || result.stdout);
	assert.equal(result.stdout, "");
	assert.equal(
		result.stderr,
		`${main}: module outside source resolves outside the FinOps root\n`,
	);
});

for (const [name, source] of [
	["registry source", '"hashicorp/consul/aws"'],
	["remote source", '"git::https://example.com/module.git"'],
	["absolute source", '"/tmp/nvbes-module"'],
	["source expression", "var.module_source"],
	[
		"source template expression",
		['"./modules/$', '{var.module_name}"'].join(""),
	],
	["missing source", undefined],
]) {
	test(`rejects a module ${name}`, async (t) => {
		const moduleBody = source === undefined ? "" : `source = ${source}`;
		const base = await createFixture(t, {
			"infrastructure/main.tf": `module "invalid" { ${moduleBody} }`,
		});
		const root = path.join(base, "infrastructure");
		const result = runChecker(root);
		const main = path.relative(process.cwd(), path.join(root, "main.tf"));

		assert.equal(result.status, 1, result.stderr || result.stdout);
		assert.equal(result.stdout, "");
		assert.equal(
			result.stderr,
			`${main}: module invalid source must be a literal local path starting with ./ or ../\n`,
		);
	});
}

test("rejects an invalid local module source path without a stack trace", async (t) => {
	const base = await createFixture(t, {
		"infrastructure/main.tf": 'module "invalid" { source = "./\\u0000" }',
	});
	const root = path.join(base, "infrastructure");
	const result = runChecker(root);
	const main = path.relative(process.cwd(), path.join(root, "main.tf"));

	assert.equal(result.status, 1, result.stderr || result.stdout);
	assert.equal(result.stdout, "");
	assert.equal(
		result.stderr,
		`${main}: module invalid source must resolve to an existing directory\n`,
	);
});

for (const [name, source, extraFiles] of [
	["missing directory", "./modules/missing", {}],
	[
		"non-directory path",
		"./modules/runtime.tf",
		{ "infrastructure/modules/runtime.tf": "" },
	],
]) {
	test(`rejects a module source targeting a ${name}`, async (t) => {
		const base = await createFixture(t, {
			"infrastructure/main.tf": `module "invalid" { source = "${source}" }`,
			...extraFiles,
		});
		const root = path.join(base, "infrastructure");
		const result = runChecker(root);
		const main = path.relative(process.cwd(), path.join(root, "main.tf"));

		assert.equal(result.status, 1, result.stderr || result.stdout);
		assert.equal(result.stdout, "");
		assert.equal(
			result.stderr,
			`${main}: module invalid source must resolve to an existing directory\n`,
		);
	});
}

test("accepts a bounded local module inside the FinOps root", async (t) => {
	const base = await createFixture(t, {
		"infrastructure/main.tf": 'module "inside" { source = "./modules/inside" }',
		"infrastructure/modules/inside/runtime.tf":
			'resource "scaleway_container" "bounded" {\n  min_scale = 0\n  max_scale = 1\n}',
	});
	const result = runChecker(path.join(base, "infrastructure"));

	assert.equal(result.status, 0, result.stderr || result.stdout);
	assert.equal(result.stderr, "");
	assert.equal(
		result.stdout,
		"FinOps scale bounds passed for all Scaleway runtimes.\n",
	);
});

test("scans bounded resources inside a referenced local module", async (t) => {
	const base = await createFixture(t, {
		"infrastructure/main.tf": 'module "inside" { source = "./modules/inside" }',
		"infrastructure/modules/inside/runtime.tf":
			'resource "scaleway_container" "unbounded" {\n  min_scale = 0\n  max_scale = 10\n}',
	});
	const runtime = path.join(base, "infrastructure/modules/inside/runtime.tf");
	const result = runChecker(path.join(base, "infrastructure"));

	assert.equal(result.status, 1, result.stderr || result.stdout);
	assert.equal(result.stdout, "");
	assert.equal(
		result.stderr,
		`${path.relative(process.cwd(), runtime)}: scaleway_container must declare max_scale as an integer <= 1\n`,
	);
});

test("rejects a local module source excluded from recursive discovery", async (t) => {
	const base = await createFixture(t, {
		"infrastructure/main.tf":
			'module "hidden" { source = "./.terraform/hidden" }',
		"infrastructure/.terraform/hidden/runtime.tf":
			'resource "scaleway_container" "unbounded" {\n  min_scale = 0\n  max_scale = 10\n}',
	});
	const root = path.join(base, "infrastructure");
	const result = runChecker(root);
	const main = path.relative(process.cwd(), path.join(root, "main.tf"));

	assert.equal(result.status, 1, result.stderr || result.stdout);
	assert.equal(result.stdout, "");
	assert.equal(
		result.stderr,
		`${main}: module hidden source directory is excluded from the FinOps scan\n`,
	);
});

test("rejects a symlinked local module through the separate symlink gate", async (t) => {
	const base = await createFixture(t, {
		"infrastructure/main.tf": 'module "linked" { source = "./modules/linked" }',
		"outside/runtime.tf":
			'resource "scaleway_container" "unbounded" {\n  min_scale = 0\n  max_scale = 10\n}',
	});
	const root = path.join(base, "infrastructure");
	const link = path.join(root, "modules/linked");
	await mkdir(path.dirname(link), { recursive: true });
	await symlink(path.join(base, "outside"), link);
	const result = runChecker(root);

	assert.equal(result.status, 1, result.stderr || result.stdout);
	assert.equal(result.stdout, "");
	assert.equal(
		result.stderr,
		`${path.relative(process.cwd(), link)}: symbolic links are not supported by the FinOps gate\n`,
	);
});

test("does not accept a source declared only in a nested module block", async (t) => {
	const base = await createFixture(t, {
		"infrastructure/main.tf": `
module "nested" {
  config {
    source = "./modules/inside"
  }
}
`,
	});
	const root = path.join(base, "infrastructure");
	const result = runChecker(root);
	const main = path.relative(process.cwd(), path.join(root, "main.tf"));

	assert.equal(result.status, 1, result.stderr || result.stdout);
	assert.equal(result.stdout, "");
	assert.equal(
		result.stderr,
		`${main}: module nested source must be a literal local path starting with ./ or ../\n`,
	);
});

test("checks every body returned for a duplicate module label", async (t) => {
	const base = await createFixture(t, {
		"infrastructure/main.tf": `
module "duplicate" {
  source = "./modules/inside"
}
module "duplicate" {
  source = "git::https://example.com/module.git"
}
`,
		"infrastructure/modules/inside/runtime.tf": "",
	});
	const root = path.join(base, "infrastructure");
	const result = runChecker(root);
	const main = path.relative(process.cwd(), path.join(root, "main.tf"));

	assert.equal(result.status, 1, result.stderr || result.stdout);
	assert.equal(result.stdout, "");
	assert.equal(
		result.stderr,
		`${main}: module duplicate source must be a literal local path starting with ./ or ../\n`,
	);
});
