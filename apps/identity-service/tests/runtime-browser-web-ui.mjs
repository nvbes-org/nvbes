import { readFile, readdir } from 'node:fs/promises';
import { resolve } from 'node:path';

/** Serve the actual production build only inside the isolated HTTPS fixture. */
export async function identityWebBuild() {
  const root = resolve('apps/identity-web/dist');
  const index = await readFile(resolve(root, 'index.html'));
  const assets = new Map();
  for (const name of await readdir(resolve(root, 'assets'))) {
    const extension = name.split('.').at(-1);
    const type = { js: 'text/javascript', css: 'text/css', woff2: 'font/woff2' }[extension];
    if (type)
      assets.set('/assets/' + name, { type, body: await readFile(resolve(root, 'assets', name)) });
  }
  return (request, response) => {
    if (request.method !== 'GET') return false;
    const url = new URL(request.url, 'https://localhost');
    const document =
      ['/', '/oauth/authorize', '/recovery', '/logout'].includes(url.pathname) &&
      request.headers.accept?.includes('text/html') &&
      !url.searchParams.getAll('prompt').some((value) => value.split(' ').includes('none'));
    const asset = assets.get(url.pathname);
    if (!document && !asset) return false;
    response.writeHead(200, {
      'content-type': document ? 'text/html; charset=utf-8' : asset.type,
      'cache-control': 'no-store',
      'referrer-policy': 'no-referrer',
      'x-content-type-options': 'nosniff',
      'content-security-policy':
        "default-src 'none'; script-src 'self'; style-src 'self'; font-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'",
    });
    response.end(document ? index : asset.body);
    return true;
  };
}
