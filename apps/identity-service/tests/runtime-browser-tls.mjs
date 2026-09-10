import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createServer } from 'node:https';
import { request } from 'node:http';
import { command } from './runtime-processes.mjs';

export async function testCertificates(fixture) {
  const directory = await mkdtemp(join(tmpdir(), 'nvbes-browser-tls-'));
  fixture.cleanups.push(() => rm(directory, { recursive: true, force: true }));
  const path = (name) => join(directory, name);
  await command('openssl', [
    'req',
    '-x509',
    '-newkey',
    'rsa:2048',
    '-nodes',
    '-days',
    '1',
    '-subj',
    '/CN=nvbes isolated browser test CA',
    '-keyout',
    path('ca.key'),
    '-out',
    path('ca.pem'),
    '-addext',
    'basicConstraints=critical,CA:TRUE',
  ]);
  await command('openssl', [
    'req',
    '-newkey',
    'rsa:2048',
    '-nodes',
    '-subj',
    '/CN=localhost',
    '-keyout',
    path('server.key'),
    '-out',
    path('server.csr'),
  ]);
  await writeFile(
    path('extensions'),
    'basicConstraints=critical,CA:FALSE\nkeyUsage=digitalSignature,keyEncipherment\nextendedKeyUsage=serverAuth\nsubjectAltName=DNS:localhost,IP:127.0.0.1,IP:::1\n',
  );
  await command('openssl', [
    'x509',
    '-req',
    '-in',
    path('server.csr'),
    '-CA',
    path('ca.pem'),
    '-CAkey',
    path('ca.key'),
    '-CAcreateserial',
    '-days',
    '1',
    '-extfile',
    path('extensions'),
    '-out',
    path('server.pem'),
  ]);
  return {
    caPath: path('ca.pem'),
    key: await readFile(path('server.key')),
    cert: await readFile(path('server.pem')),
  };
}

export async function tlsEndpoint({ fixture, tls, origin, backend, middleware, config }) {
  const address = new URL(origin);
  const server = createServer(tls, (incoming, outgoing) => {
    if (incoming.headers.host !== address.host) {
      outgoing.writeHead(400).end();
      return;
    }
    // Fixture-only pages and synthetic credentials; never included in a runtime binary.
    if (incoming.url === '/__fixture/config') {
      outgoing.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' });
      outgoing.end(JSON.stringify(config));
      return;
    }
    if (
      incoming.url === '/' ||
      incoming.url.startsWith('/callback') ||
      incoming.url === '/__fixture/hosted'
    ) {
      outgoing.writeHead(200, {
        'content-type': 'text/html',
        'cache-control': 'no-store',
        'referrer-policy': 'no-referrer',
      });
      outgoing.end(
        '<!doctype html><html lang="en"><title>Isolated Identity browser test</title><body>Protocol test fixture</body></html>',
      );
      return;
    }
    if (
      incoming.url.startsWith('/libs/') ||
      incoming.url.startsWith('/node_modules/') ||
      incoming.url.startsWith('/@')
    ) {
      middleware(incoming, outgoing, () => outgoing.writeHead(404).end());
      return;
    }
    if (!backend) {
      outgoing.writeHead(404).end();
      return;
    }
    const target = new URL(backend);
    const upstream = request(
      {
        hostname: '127.0.0.1',
        port: target.port,
        path: incoming.url,
        method: incoming.method,
        headers: incoming.headers,
        timeout: 5000,
      },
      (response) => {
        outgoing.writeHead(response.statusCode, response.headers);
        response.pipe(outgoing);
      },
    );
    upstream.on('timeout', () => upstream.destroy());
    upstream.on('error', () => {
      if (!outgoing.headersSent) outgoing.writeHead(502);
      outgoing.end();
    });
    incoming.pipe(upstream);
  });
  fixture.cleanups.push(async () => {
    server.closeAllConnections();
    await new Promise((resolve, reject) =>
      server.close((error) => (error ? reject(error) : resolve())),
    );
  });
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(Number(address.port), '127.0.0.1', resolve);
  });
}
