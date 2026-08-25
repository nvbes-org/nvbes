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

async function runChecker(terraform, t, fileName = "runtime.tf") {
	const directory = await mkdtemp(path.join(tmpdir(), "nvbes-scale-to-zero-"));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const file = path.join(directory, fileName);
	await mkdir(path.dirname(file), { recursive: true });
	await writeFile(file, terraform);
	return {
		file,
		result: spawnSync(process.execPath, [checker, directory], {
			encoding: "utf8",
		}),
	};
}

async function assertChecker(
	terraform,
	expectedStatus,
	expectedMessages,
	t,
	fileName,
) {
	const { file, result } = await runChecker(terraform, t, fileName);
	const prefix = path.relative(process.cwd(), file);
	const expectedStderr = expectedMessages.length
		? `${expectedMessages.map((message) => `${prefix}: ${message}`).join("\n")}\n`
		: "";
	assert.equal(result.status, expectedStatus, result.stderr || result.stdout);
	assert.equal(result.stderr, expectedStderr);
	assert.equal(
		result.stdout,
		expectedMessages.length
			? ""
			: "FinOps scale bounds passed for all Scaleway runtimes.\n",
	);
}

for (const [name, maximum] of [
	["unbounded", 10],
	["bounded", 1],
]) {
	test(`rejects ${name} Terraform JSON`, async (t) => {
		const terraform = JSON.stringify({
			resource: {
				scaleway_container: { runtime: { min_scale: 0, max_scale: maximum } },
			},
		});
		await assertChecker(
			terraform,
			1,
			["Terraform JSON syntax is not supported by the FinOps gate"],
			t,
			"runtime.tf.json",
		);
	});
}

test("ignores Terraform JSON inside .terraform directories", async (t) => {
	await assertChecker("{}", 0, [], t, ".terraform/runtime.tf.json");
});

test("rejects every symbolic link outside .terraform", async (t) => {
	const directory = await mkdtemp(path.join(tmpdir(), "nvbes-scale-to-zero-"));
	t.after(() => rm(directory, { recursive: true, force: true }));
	const targetFile = path.join(directory, "unbounded.source");
	const targetDirectory = path.join(directory, "target-directory");
	await writeFile(
		targetFile,
		'resource "scaleway_container" "bypass" { min_scale = 0 max_scale = 10 }',
	);
	await mkdir(targetDirectory);
	for (const [name, target] of [
		["linked-directory", targetDirectory],
		["notes.link", targetFile],
		["runtime.tf", targetFile],
		["runtime.tf.json", targetFile],
	]) {
		await symlink(target, path.join(directory, name));
	}

	const result = spawnSync(process.execPath, [checker, directory], {
		encoding: "utf8",
	});
	const message = "symbolic links are not supported by the FinOps gate";
	assert.equal(result.status, 1, result.stderr || result.stdout);
	assert.equal(result.stdout, "");
	assert.equal(
		result.stderr,
		`${["linked-directory", "notes.link", "runtime.tf.json", "runtime.tf"]
			.map(
				(name) =>
					`${path.relative(process.cwd(), path.join(directory, name))}: ${message}`,
			)
			.join("\n")}\n`,
	);
});

test("accepts bounded zero-minimum resources", async (t) => {
	await assertChecker(
		`
resource "scaleway_container" "api" {
  min_scale = 0
  max_scale = 1
}
resource "scaleway_sdb_sql_database" "api" {
  min_cpu = 0
  max_cpu = 1
}
`,
		0,
		[],
		t,
	);
});

const rejectedFixtures = [
	{
		name: "container maximum above one",
		terraform: `
resource "scaleway_container" "too_many" {
  min_scale = 0
  max_scale = 10
}
`,
		messages: ["scaleway_container must declare max_scale as an integer <= 1"],
	},
	{
		name: "database maximum above one",
		terraform: `
resource "scaleway_sdb_sql_database" "too_many" {
  min_cpu = 0
  max_cpu = 2
}
`,
		messages: [
			"scaleway_sdb_sql_database must declare max_cpu as an integer <= 1",
		],
	},
	{
		name: "nonzero and missing bounds",
		terraform: `
resource "scaleway_container" "always_on" {
  min_scale = 1
  max_scale = 1
}
resource "scaleway_sdb_sql_database" "missing" {
  min_cpu = 0
}
`,
		messages: [
			"scaleway_container must declare min_scale = 0",
			"scaleway_sdb_sql_database must declare max_cpu as an integer <= 1",
		],
	},
	{
		name: "expressions are not literals",
		terraform: `
resource "scaleway_container" "expression" {
  min_scale = var.minimum
  max_scale = 1 + var.burst
}
resource "scaleway_sdb_sql_database" "expression" {
  min_cpu = 0 + var.minimum
  max_cpu = var.maximum
}
`,
		messages: [
			"scaleway_container must declare max_scale as an integer <= 1",
			"scaleway_container must declare min_scale = 0",
			"scaleway_sdb_sql_database must declare max_cpu as an integer <= 1",
			"scaleway_sdb_sql_database must declare min_cpu = 0",
		],
	},
	{
		name: "comments do not provide bounds or resources",
		terraform: `
# resource "scaleway_container" "fake_hash" { min_scale = 1 max_scale = 10 }
// resource "scaleway_sdb_sql_database" "fake_slash" { min_cpu = 1 max_cpu = 2 }
/* resource "scaleway_container" "fake_block" {
  min_scale = 1
  max_scale = 10
} */
resource "scaleway_container" "commented_attrs" {
  # min_scale = 0
  // max_scale = 1
}
`,
		messages: [
			"scaleway_container must declare max_scale as an integer <= 1",
			"scaleway_container must declare min_scale = 0",
		],
	},
	{
		name: "strings and nested blocks are opaque and direct only",
		terraform: `
resource "scaleway_container" "opaque" {
  description = "braces { } comments # // /* */ escaped \\"quote\\""
  template = "\${var.fake}"
  nested {
    min_scale = 1
    max_scale = 10
  }
  min_scale = 0
  max_scale = 1
}
resource "scaleway_sdb_sql_database" "cross_block" {
  nested {
    min_cpu = 1
    max_cpu = 2
  }
}
`,
		messages: [
			"scaleway_sdb_sql_database must declare max_cpu as an integer <= 1",
			"scaleway_sdb_sql_database must declare min_cpu = 0",
		],
	},
	{
		name: "heredocs hide fake attributes and resources",
		terraform: `
resource "scaleway_container" "heredocs" {
  ordinary = <<EOF
resource "scaleway_sdb_sql_database" "fake" {
  min_cpu = 1
  max_cpu = 2
}
EOF
  indented = <<-INDENTED
    resource "scaleway_container" "fake" {
      min_scale = 1
      max_scale = 10
    }
    INDENTED
  min_scale = 0
  max_scale = 1
}
`,
		messages: [],
	},
	{
		name: "signed literals are rejected except parser-normalized negative zero",
		terraform: `
resource "scaleway_container" "signed" {
  min_scale = -0
  max_scale = -1
}
resource "scaleway_sdb_sql_database" "signed" {
  min_cpu = -0
  max_cpu = -1
}
`,
		messages: [
			"scaleway_container must declare max_scale as an integer <= 1",
			"scaleway_container must declare min_scale = 0",
			"scaleway_sdb_sql_database must declare max_cpu as an integer <= 1",
			"scaleway_sdb_sql_database must declare min_cpu = 0",
		],
	},
	{
		name: "CRLF source remains valid",
		terraform:
			'resource "scaleway_container" "crlf" {\r\n  min_scale = 0\r\n  max_scale = 1\r\n}\r\n',
		messages: [],
	},
];

for (const fixture of rejectedFixtures) {
	test(`checks ${fixture.name}`, async (t) => {
		await assertChecker(
			fixture.terraform,
			fixture.messages.length === 0 ? 0 : 1,
			fixture.messages,
			t,
		);
	});
}

test("reports parse failures without a stack trace", async (t) => {
	await assertChecker(
		'resource "scaleway_container" "broken" { min_scale = 0\n',
		1,
		["failed to parse Terraform HCL"],
		t,
	);
});
