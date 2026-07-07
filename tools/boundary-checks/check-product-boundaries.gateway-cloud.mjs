import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);
const textExtensions = new Set([".rs", ".toml", ".sql"]);

const forbiddenDependencyNames = [
	"sqlx",
	"diesel",
	"sea-orm",
	"mongodb",
	"redis",
	"deadpool",
	"deadpool-redis",
	"bb8",
	"bb8-redis",
	"nvbes-account-service",
	"nvbes-billing-service",
	"nvbes-cloud-service",
	"nvbes-developer-service",
	"nvbes-enterprise-service",
	"nvbes-backoffice-service",
];

const forbiddenSourcePatterns = [
	/\b(?:sqlx|diesel|sea_orm|mongodb|redis|deadpool|bb8)::/,
	/\b(?:PgPool|PgConnection|Pool<Postgres>|SqlitePool|MySqlPool|RedisConnectionManager)\b/,
	/\b(?:query|query_as|query_scalar)!\s*\(/,
	/\b(?:SELECT|INSERT\s+INTO|UPDATE|DELETE\s+FROM|CREATE\s+TABLE|ALTER\s+TABLE|DROP\s+TABLE)\b/i,
	/\bnvbes_(?:account|billing|cloud|developer|enterprise|backoffice)_service\b/,
];

export function checkGatewayCloudBoundary(errors) {
	checkGatewayCloudHasNoMigrations(errors);
	checkGatewayCloudManifest(errors);
	checkGatewayCloudSource(errors);
}

function walk(dir, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		if (skippedDirs.has(entry)) continue;
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			walk(path, results);
			continue;
		}
		const relativePath = relative(process.cwd(), path).replaceAll("\\", "/");
		const dot = relativePath.lastIndexOf(".");
		if (dot >= 0 && textExtensions.has(relativePath.slice(dot))) results.push(relativePath);
	}
	return results;
}

function checkGatewayCloudHasNoMigrations(errors) {
	const migrationsDir = "apps/gateway-cloud/migrations";
	if (existsSync(migrationsDir)) {
		errors.push(`${migrationsDir}: Gateway Cloud must not own database migrations or business persistence`);
	}
}

function checkGatewayCloudManifest(errors) {
	const manifestPath = "apps/gateway-cloud/Cargo.toml";
	if (!existsSync(manifestPath)) return;
	const manifest = readFileSync(manifestPath, "utf8");
	for (const dependencyName of forbiddenDependencyNames) {
		const escapedName = escapeRegExp(dependencyName);
		const dependencyPattern = new RegExp(`(^|\\n)\\s*(?:${escapedName}\\s*=|\\[dependencies\\.${escapedName}\\])`, "m");
		if (dependencyPattern.test(manifest)) {
			errors.push(`${manifestPath}: Gateway Cloud must not depend on persistence or service runtime crate ${dependencyName}`);
		}
	}
}

function checkGatewayCloudSource(errors) {
	for (const file of walk("apps/gateway-cloud/src")) {
		const content = readFileSync(file, "utf8");
		for (const pattern of forbiddenSourcePatterns) {
			if (pattern.test(content)) {
				errors.push(`${file}: Gateway Cloud must stay a stateless composition layer and must not own persistence ${pattern}`);
			}
		}
	}
}

function escapeRegExp(value) {
	return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
