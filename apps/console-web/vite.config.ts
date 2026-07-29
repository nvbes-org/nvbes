import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import devtoolsJson from "vite-plugin-devtools-json";
import { defineConfig, loadEnv } from "vite-plus";
import {
	browserIsolationHeaders,
	buildWebCsp,
	permissionsPolicy,
	productionTransportHeaders,
	uaClientHintsHeaders,
} from "../../libs/ts/web-runtime/src/csp";
import { observabilitySourceMapPlugins } from "../../tools/web-build/vite-observability-sourcemaps";

function cspFor(mode: string): string {
	return buildWebCsp({
		mode,
		styleSrc: ["https://fonts.googleapis.com"],
		fontSrc: ["https://fonts.gstatic.com"],
	});
}

export default defineConfig(({ mode }) => {
	const localEnv = loadEnv(mode, process.cwd(), "");
	const rootEnv = loadEnv(mode, path.resolve(process.cwd(), "../../"), "");
	const envSources = [process.env, localEnv, rootEnv];
	const accountServiceProxyTarget =
		process.env.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
		localEnv.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
		rootEnv.VITE_ACCOUNT_SERVICE_PROXY_TARGET ||
		process.env.VITE_ACCOUNT_SERVICE_BASE_URL ||
		localEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
		rootEnv.VITE_ACCOUNT_SERVICE_BASE_URL ||
		"http://localhost:4000";
	const developerServiceProxyTarget =
		process.env.VITE_DEVELOPER_SERVICE_PROXY_TARGET ||
		localEnv.VITE_DEVELOPER_SERVICE_PROXY_TARGET ||
		rootEnv.VITE_DEVELOPER_SERVICE_PROXY_TARGET ||
		process.env.VITE_DEVELOPER_SERVICE_BASE_URL ||
		localEnv.VITE_DEVELOPER_SERVICE_BASE_URL ||
		rootEnv.VITE_DEVELOPER_SERVICE_BASE_URL ||
		"http://localhost:4040";

	return {
		plugins: [
			react(),
			tailwindcss(),
			devtoolsJson(),
			...observabilitySourceMapPlugins({ appName: "console-web", envSources }),
		],
		build: {
			target: "esnext",
			sourcemap: "hidden",
			minify: true,
			cssMinify: "esbuild",
			manifest: true,
			modulePreload: { polyfill: false },
		},
		resolve: {
			alias: [
				{
					find: "@",
					replacement: path.resolve(__dirname, "./src"),
				},
				{
					find: "@nvbes/http-client",
					replacement: path.resolve(
						__dirname,
						"../../libs/ts/http-client/src/index.ts",
					),
				},
				{
					find: "@nvbes/identity-client",
					replacement: path.resolve(
						__dirname,
						"../../libs/ts/identity-client/src/index.ts",
					),
				},
				{
					find: "@nvbes/identity-sdk",
					replacement: path.resolve(
						__dirname,
						"../../libs/ts/identity-sdk/src/index.ts",
					),
				},
				{
					find: "@nvbes/identity-sdk-web",
					replacement: path.resolve(
						__dirname,
						"../../libs/ts/identity-sdk-web/src/index.ts",
					),
				},
				{
					find: "@nvbes/web-runtime",
					replacement: path.resolve(
						__dirname,
						"../../libs/ts/web-runtime/src/index.ts",
					),
				},
				{
					find: "@nvbes/web-ui",
					replacement: path.resolve(
						__dirname,
						"../../libs/ts/web-ui/src/index.ts",
					),
				},
			],
		},
		server: {
			port: 5175,
			strictPort: true,
			headers: {
				...uaClientHintsHeaders,
				...browserIsolationHeaders,
				...productionTransportHeaders(mode),
				"Content-Security-Policy": cspFor(mode),
				"X-Frame-Options": "DENY",
				"Referrer-Policy": "strict-origin-when-cross-origin",
				"Permissions-Policy": permissionsPolicy,
			},
			proxy: {
				"/auth": accountServiceProxyTarget,
				"/api": developerServiceProxyTarget,
				"/developer": developerServiceProxyTarget,
				"/oauth": accountServiceProxyTarget,
				"/.well-known": accountServiceProxyTarget,
			},
		},
		preview: {
			port: 5175,
			strictPort: true,
			headers: {
				...uaClientHintsHeaders,
				...browserIsolationHeaders,
				...productionTransportHeaders(mode),
				"Content-Security-Policy": cspFor(mode),
				"X-Frame-Options": "DENY",
				"Referrer-Policy": "strict-origin-when-cross-origin",
				"Permissions-Policy": permissionsPolicy,
			},
		},
	};
});
