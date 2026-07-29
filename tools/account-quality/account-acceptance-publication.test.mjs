import assert from "node:assert/strict";
import {
	mkdir,
	mkdtemp,
	readdir,
	readFile,
	rm,
	symlink,
	writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import test from "node:test";
import { prepareAccountAcceptancePublication } from "./prepare-account-acceptance-publication.mjs";
import { REQUIRED_HUMAN_CATEGORIES } from "./verify-acceptance-evidence.mjs";

test("publishes only the exact signed acceptance packet", async (t) => {
	const fixture = await packetFixture(t);
	const result = await prepareAccountAcceptancePublication({
		publicationRoot: fixture.publication,
		sourceRoot: fixture.source,
	});
	assert.equal(result.files, 16);
	assert.ok(result.totalBytes > 0);
	assert.deepEqual(await tree(fixture.publication), await tree(fixture.source));
	assert.deepEqual(
		JSON.parse(
			await readFile(
				path.join(fixture.publication, "acceptance-evidence.json"),
				"utf8",
			),
		),
		fixture.manifest,
	);
});

test("rejects unreferenced extras and directories", async (t) => {
	for (const extra of ["untrusted.txt", "extra/untrusted.txt"]) {
		const fixture = await packetFixture(t);
		await writeFixtureFile(fixture.source, extra, "untrusted");
		await assert.rejects(
			prepareAccountAcceptancePublication({
				publicationRoot: fixture.publication,
				sourceRoot: fixture.source,
			}),
			/missing or extra/u,
		);
	}
});

test("rejects symlinks before copying evidence", async (t) => {
	const fixture = await packetFixture(t);
	const report = fixture.manifest.reports[0].path;
	const reportPath = path.join(fixture.source, report);
	await rm(reportPath);
	await symlink(
		path.join(fixture.source, fixture.manifest.reports[1].path),
		reportPath,
	);
	await assert.rejects(
		prepareAccountAcceptancePublication({
			publicationRoot: fixture.publication,
			sourceRoot: fixture.source,
		}),
		/symbolic link/u,
	);
});

test("rejects escaping and duplicate manifest paths", async (t) => {
	for (const mutate of [
		(manifest) => {
			manifest.reports[0].path = "../outside.txt";
		},
		(manifest) => {
			manifest.reports[0].path = manifest.reports[1].path;
		},
	]) {
		const fixture = await packetFixture(t);
		mutate(fixture.manifest);
		await writeFile(
			path.join(fixture.source, "acceptance-evidence.json"),
			JSON.stringify(fixture.manifest),
		);
		await assert.rejects(
			prepareAccountAcceptancePublication({
				publicationRoot: fixture.publication,
				sourceRoot: fixture.source,
			}),
			/escapes|duplicated/u,
		);
	}
});

async function packetFixture(t) {
	const root = await mkdtemp(path.join(tmpdir(), "account-acceptance-packet-"));
	t.after(() => rm(root, { force: true, recursive: true }));
	const source = path.join(root, "source");
	const publication = path.join(root, "publication");
	await mkdir(source);
	const reports = REQUIRED_HUMAN_CATEGORIES.map((category) => ({
		category,
		path: `reports/${category}.txt`,
	}));
	const manifest = {
		deployment: {
			path: "deployment/attestation.json",
			signaturePath: "deployment/attestation.sig",
		},
		reports,
		schemaVersion: 1,
	};
	await writeFixtureFile(
		source,
		"acceptance-evidence.json",
		JSON.stringify(manifest),
	);
	await writeFixtureFile(
		source,
		"acceptance-evidence.sig",
		"manifest-signature",
	);
	await writeFixtureFile(
		source,
		manifest.deployment.path,
		"deployment-attestation",
	);
	await writeFixtureFile(
		source,
		manifest.deployment.signaturePath,
		"deployment-signature",
	);
	for (const report of reports) {
		await writeFixtureFile(source, report.path, report.category);
	}
	return { manifest, publication, source };
}

async function writeFixtureFile(root, relativePath, content) {
	const filePath = path.join(root, relativePath);
	await mkdir(path.dirname(filePath), { recursive: true });
	await writeFile(filePath, content);
}

async function tree(root) {
	const result = [];
	async function visit(directory, relative = "") {
		for (const entry of await readdir(directory, { withFileTypes: true })) {
			const child = path.posix.join(relative, entry.name);
			if (entry.isDirectory()) {
				await visit(path.join(directory, entry.name), child);
			} else {
				result.push(child);
			}
		}
	}
	await visit(root);
	return result.sort();
}
