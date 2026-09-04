import assert from "node:assert/strict";
import test from "node:test";
import { selectAffectedCargoPackages } from "./cargo-affected.core.mjs";

const workspace = "/workspace";
const metadata = {
	workspace_members: ["core-id", "email-id", "unrelated-id"],
	packages: [
		{
			id: "core-id",
			name: "nvbes-core",
			manifest_path: "/workspace/libs/rust/core/Cargo.toml",
			dependencies: [],
		},
		{
			id: "email-id",
			name: "nvbes-email-worker",
			manifest_path: "/workspace/apps/email-worker/Cargo.toml",
			dependencies: [{ name: "nvbes-core" }],
		},
		{
			id: "unrelated-id",
			name: "nvbes-unrelated",
			manifest_path: "/workspace/libs/rust/unrelated/Cargo.toml",
			dependencies: [],
		},
	],
};

test("selects a changed crate and its workspace dependants", () => {
	assert.deepEqual(
		selectAffectedCargoPackages(
			metadata,
			["libs/rust/core/src/lib.rs"],
			workspace,
		),
		["nvbes-core", "nvbes-email-worker"],
	);
});

test("does not select unrelated crates", () => {
	assert.deepEqual(
		selectAffectedCargoPackages(
			metadata,
			["apps/email-worker/src/main.rs"],
			workspace,
		),
		["nvbes-email-worker"],
	);
});

test("selects the workspace for global and unowned Rust inputs", () => {
	const all = ["nvbes-core", "nvbes-email-worker", "nvbes-unrelated"];
	assert.deepEqual(
		selectAffectedCargoPackages(metadata, ["Cargo.lock"], workspace),
		all,
	);
	assert.deepEqual(
		selectAffectedCargoPackages(metadata, ["build.rs"], workspace),
		all,
	);
});
