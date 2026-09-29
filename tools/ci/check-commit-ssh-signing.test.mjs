import assert from 'node:assert/strict';
import test from 'node:test';
import {
  classifyCommitSignature,
  evaluateSignedCommits,
  evaluateSigningConfig,
} from './check-commit-ssh-signing.mjs';

test('requires commit.gpgsign and a signing key; ssh or openpgp formats ok', () => {
  assert.deepEqual(
    evaluateSigningConfig({
      gpgsign: 'true',
      gpgFormat: 'ssh',
      signingKey: '/home/dev/.ssh/id_ed25519.pub',
    }),
    { ok: true, reasons: [], format: 'ssh' },
  );
  assert.equal(
    evaluateSigningConfig({
      gpgsign: 'true',
      gpgFormat: null,
      signingKey: 'A844FF7689BBBE14',
    }).ok,
    true,
  );
  const refused = evaluateSigningConfig({
    gpgsign: 'false',
    gpgFormat: 'openpgp',
    signingKey: null,
  });
  assert.equal(refused.ok, false);
  assert.equal(refused.reasons.length, 2);
});

test('classifies SSH, PGP and unsigned commit objects', () => {
  assert.equal(
    classifyCommitSignature(
      'tree abc\nauthor a <a@b> 1 +0000\ncommitter a <a@b> 1 +0000\ngpgsig -----BEGIN SSH SIGNATURE-----\n U1NI\n -----END SSH SIGNATURE-----\n\nmsg\n',
    ),
    'ssh',
  );
  assert.equal(
    classifyCommitSignature(
      'tree abc\ngpgsig -----BEGIN PGP SIGNATURE-----\n version\n -----END PGP SIGNATURE-----\n\nmsg\n',
    ),
    'pgp',
  );
  assert.equal(classifyCommitSignature('tree abc\n\nmsg\n'), 'none');
});

test('range evaluation refuses only unsigned commits', () => {
  const result = evaluateSignedCommits([
    { sha: 'a'.repeat(40), kind: 'ssh' },
    { sha: 'b'.repeat(40), kind: 'pgp' },
    { sha: 'c'.repeat(40), kind: 'none' },
  ]);
  assert.equal(result.ok, false);
  assert.equal(result.failures.length, 1);
  assert.equal(result.failures[0].kind, 'none');
});
