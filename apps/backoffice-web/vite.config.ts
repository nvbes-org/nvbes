import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import devtoolsJson from "vite-plugin-devtools-json";
import { defineConfig, loadEnv, type Plugin } from "vite-plus";
import {
	browserIsolationHeaders,
	buildWebCsp,
	cspMetaFromHeader,
	permissionsPolicy,
	productionTransportHeaders,
	uaClientHintsHeaders,
} from "../../libs/ts/web-runtime/src/csp";
import { observabilitySourceMapPlugins } from "../../tools/web-build/vite-observability-sourcemaps";

function cspFor(mode: string): string {
	return buildWebCsp({
		mode,
		imgSrc: [],
		fontSrc: [],
	});
}

function cspPlugin(mode: string): Plugin {
	return {
		name: "backoffice-service-csp",
		transformIndexHtml(html: string) {
			const csp = cspMetaFromHeader(cspFor(mode));
			return html.replace(
				"<!-- %CSP_META% -->",
				`<meta http-equiv="Content-Security-Policy" content="${csp}" />`,
			);
		},
	};
}

export default defineConfig(({ mode }) => {
	const localEnv = loadEnv(mode, process.cwd(), "");
	const rootEnv = loadEnv(mode, path.resolve(process.cwd(), "../../"), "");
	const envSources = [process.env, localEnv, rootEnv];
	const backofficeServiceBaseUrl =
		process.env.VITE_BACKOFFICE_SERVICE_BASE_URL ||
		localEnv.VITE_BACKOFFICE_SERVICE_BASE_URL ||
		rootEnv.VITE_BACKOFFICE_SERVICE_BASE_URL ||
		"http://localhost:4000";
	const cspHeader = cspFor(mode);

	return {
		plugins: [
			tailwindcss(),
			react(),
			devtoolsJson(),
			cspPlugin(mode),
			...observabilitySourceMapPlugins({
				appName: "backoffice-web",
				envSources,
			}),
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
				{ find: "@", replacement: path.resolve(__dirname, "./src") },
				{
					find: "@nvbes/web-runtime",
					replacement: path.resolve(
						__dirname,
						"../../libs/ts/web-runtime/src/index.ts",
					),
				},
			],
		},
		server: {
			port: 5178,
			strictPort: true,
			host: "127.0.0.1",
			headers: {
				...uaClientHintsHeaders,
				...browserIsolationHeaders,
				...productionTransportHeaders(mode),
				"Content-Security-Policy": cspHeader,
				"X-Frame-Options": "DENY",
				"Referrer-Policy": "strict-origin-when-cross-origin",
				"Permissions-Policy": permissionsPolicy,
			},
			proxy: {
				"/admin": { target: backofficeServiceBaseUrl, changeOrigin: true },
				"/workspaces": { target: backofficeServiceBaseUrl, changeOrigin: true },
			},
		},
		preview: {
			port: 5178,
			strictPort: true,
			host: "127.0.0.1",
			headers: {
				...uaClientHintsHeaders,
				...browserIsolationHeaders,
				...productionTransportHeaders(mode),
				"Content-Security-Policy": cspHeader,
				"X-Frame-Options": "DENY",
				"Referrer-Policy": "strict-origin-when-cross-origin",
				"Permissions-Policy": permissionsPolicy,
			},
		},
	};
});
