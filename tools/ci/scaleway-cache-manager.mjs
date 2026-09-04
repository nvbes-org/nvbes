#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import {
	appendFileSync,
	existsSync,
	mkdirSync,
	mkdtempSync,
	readFileSync,
	realpathSync,
	rmSync,
	statSync,
} from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
	restorePrefixes,
	writablePrefix,
} from "./scaleway-cache-manager.core.mjs";

const workspace = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const action = process.argv[2];
const selected = new Set(process.argv.slice(3));
const allKinds = ["pnpm", "cargo", "terraform", "nx"];
const kinds =
	selected.size === 0
		? allKinds
		: allKinds.filter((kind) => selected.has(kind));
const environment = {
	eventName: process.env.GITHUB_EVENT_NAME ?? "",
	ref: process.env.GITHUB_REF ?? "",
	refName: process.env.GITHUB_REF_NAME ?? "",
	headRef: process.env.GITHUB_HEAD_REF ?? "",
};

if (!new Set(["restore", "save"]).has(action)) {
	throw new Error(
		"usage: scaleway-cache-manager.mjs <restore|save> [pnpm cargo terraform nx]",
	);
}

function pnpmStore() {
	return execFileSync("pnpm", ["store", "path", "--silent"], {
		cwd: workspace,
		encoding: "utf8",
	}).trim();
}

function cacheDefinitions() {
	const platform = `${process.env.RUNNER_OS ?? process.platform}-${process.env.RUNNER_ARCH ?? process.arch}`;
	const cargoHome = process.env.CARGO_HOME || join(homedir(), ".cargo");
	const terraformHome =
		process.env.TF_PLUGIN_CACHE_DIR || join(tmpdir(), "terraform-plugin-cache");
	const nxPath = join(workspace, ".nx", "cache");
	const nxVersion = JSON.parse(
		readFileSync(join(workspace, "package.json"), "utf8"),
	).devDependencies.nx;
	return {
		pnpm: {
			path: pnpmStore(),
			key: `v1/pnpm/${platform}/pnpm-11.18.0-node-24`,
		},
		cargo: {
			path: cargoHome,
			members: ["registry", "git"],
			key: `v1/cargo/${platform}/rust-1.91.1`,
		},
		terraform: {
			path: terraformHome,
			key: `v1/terraform/${platform}/terraform-1.15.8`,
		},
		nx: {
			path: nxPath,
			key: `v1/nx/${platform}/nx-${nxVersion}`,
		},
	};
}

function configured() {
	return Boolean(
		process.env.SCW_CI_CACHE_BUCKET &&
			process.env.SCW_CI_CACHE_S3_ENDPOINT &&
			process.env.AWS_ACCESS_KEY_ID &&
			process.env.AWS_SECRET_ACCESS_KEY,
	);
}

function aws(arguments_, options = {}) {
	return spawnSync("aws", arguments_, {
		encoding: "utf8",
		stdio: options.quiet ? "ignore" : "inherit",
		env: {
			...process.env,
			AWS_DEFAULT_REGION: process.env.SCW_CI_CACHE_REGION,
		},
	});
}

function objectUrl(prefix, definition) {
	return `s3://${process.env.SCW_CI_CACHE_BUCKET}/${prefix}/archives/${definition.key}.tar.gz`;
}

function cachePath(definition) {
	mkdirSync(definition.path, { recursive: true });
	return realpathSync(definition.path);
}

function summary(lines) {
	console.log(lines.join("\n"));
	if (process.env.GITHUB_STEP_SUMMARY) {
		appendFileSync(
			process.env.GITHUB_STEP_SUMMARY,
			`\n### Scaleway central cache\n\n${lines.map((line) => `- ${line}`).join("\n")}\n`,
		);
	}
}

function restore(definitions) {
	const lines = [];
	for (const kind of kinds) {
		const definition = definitions[kind];
		const destination = cachePath(definition);
		let hit = null;
		for (const prefix of restorePrefixes(environment)) {
			const temporary = join(
				mkdtempSync(join(tmpdir(), "nvbes-cache-")),
				`${kind}.tar.gz`,
			);
			const download = aws(
				[
					"--endpoint-url",
					process.env.SCW_CI_CACHE_S3_ENDPOINT,
					"s3",
					"cp",
					objectUrl(prefix, definition),
					temporary,
					"--only-show-errors",
				],
				{ quiet: true },
			);
			if (download.status === 0) {
				const extracted = spawnSync(
					"tar",
					["-xzf", temporary, "-C", destination],
					{ stdio: "inherit" },
				);
				if (extracted.status === 0) hit = prefix;
			}
			rmSync(dirname(temporary), { recursive: true, force: true });
			if (hit) break;
		}
		lines.push(`${kind}: ${hit ? `hit (${hit})` : "miss"}`);
	}
	summary(lines);
}

function save(definitions) {
	const prefix = writablePrefix(environment);
	if (!prefix) return summary(["publication ignorée (événement non push)"]);
	const lines = [];
	for (const kind of kinds) {
		const definition = definitions[kind];
		if (!existsSync(definition.path)) {
			lines.push(`${kind}: absent, publication ignorée`);
			continue;
		}
		const source = cachePath(definition);
		const temporaryDirectory = mkdtempSync(join(tmpdir(), "nvbes-cache-"));
		const archive = join(temporaryDirectory, `${kind}.tar.gz`);
		const members = definition.members ?? ["."];
		for (const member of definition.members ?? [])
			mkdirSync(join(source, member), { recursive: true });
		const archived = spawnSync(
			"tar",
			["-czf", archive, "-C", source, ...members],
			{ stdio: "inherit" },
		);
		if (archived.status !== 0) {
			lines.push(`${kind}: échec de création`);
			continue;
		}
		const uploaded = aws([
			"--endpoint-url",
			process.env.SCW_CI_CACHE_S3_ENDPOINT,
			"s3",
			"cp",
			archive,
			objectUrl(prefix, definition),
			"--only-show-errors",
		]);
		const size = statSync(archive).size;
		lines.push(
			`${kind}: ${uploaded.status === 0 ? `publié (${prefix}, ${Math.ceil(size / 1_048_576)} MiB)` : "échec de publication"}`,
		);
		rmSync(temporaryDirectory, { recursive: true, force: true });
	}
	summary(lines);
}

if (!configured()) {
	summary(["désactivé : credentials ou configuration Scaleway indisponibles"]);
} else if (action === "restore") {
	restore(cacheDefinitions());
} else {
	save(cacheDefinitions());
}
