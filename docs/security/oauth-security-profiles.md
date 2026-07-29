# OAuth security profiles

nvbes implements OAuth using final RFCs and the OAuth Security Best Current
Practice (RFC 9700). The product does not claim conformance to the OAuth 2.1
Internet-Draft.

## Standard profile

`security_profile=standard` requires:

- authorization code flow;
- pushed authorization requests (PAR);
- PKCE with `S256` for every authorization-code client;
- exact registered redirect URI matching;
- rotating refresh-token families with reuse detection;
- strict JWT issuer, audience, algorithm, type, lifetime, and key validation.

Signed request objects are not accepted in this profile. DPoP can be configured
as an additional sender constraint.

## High-assurance profile

`security_profile=high_assurance` follows the FAPI 2.0 Security Profile:

- confidential client;
- PAR and PKCE `S256`;
- `private_key_jwt` client authentication using PS256, ES256, or EdDSA;
- a signed JAR request object using PS256, ES256, or EdDSA;
- request-object `kid`, `iss`, `aud`, `exp`, and `nbf`, with optional
  `typ=oauth-authz-req+jwt` and replay-protected `jti` when supplied;
- DPoP with server nonces and replay storage, or mutual TLS;
- sender-constrained access and refresh tokens.

For DPoP, tokens contain `cnf.jkt` and use the `DPoP` token type. For mutual
TLS, tokens contain `cnf["x5t#S256"]`; the thumbprint is computed from the
verified client certificate by the mTLS listener and matched against the
client registration.

High-assurance clients currently support authorization code, refresh token,
and client credentials grants. Device authorization and token exchange are
rejected for this profile.

Authorization codes expire after 60 seconds. DPoP authorization codes are
bound to the key used at PAR and can only be redeemed with that key. In the
high-assurance profile, refresh tokens remain sender-constrained and are not
rotated during ordinary refreshes, as required by the final FAPI 2.0 profile.

## Client registration fields

- `security_profile`: `standard` or `high_assurance`
- `client_assertion_public_key_jwk`: public JWK used by `private_key_jwt`
- `client_assertion_required`: must be `true` for high assurance
- `request_object_signing_jwks`: JWKS containing PS256 RSA-2048+ or ES256
  P-256 signing keys
- `sender_constraint`: `dpop` or `mtls`
- `tls_client_certificate_sha256`: unpadded base64url SHA-256 certificate
  thumbprint, required for mTLS

Client authentication and request-object keys are stored individually with an
explicit purpose and lifecycle state (`active`, `retiring`, or `revoked`).
Rotation may retain an old key for a bounded grace interval. Revocation takes
effect immediately, and a high-assurance client cannot revoke its final usable
key for either purpose.

## Production activation and conformance evidence

`NVBES_FAPI_HIGH_ASSURANCE_ENABLED` defaults to `false`. Enabling it requires
DPoP or mTLS. Production startup additionally requires
`NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256`.

Before production activation:

1. deploy the exact candidate commit to an isolated staging issuer;
2. run the official OpenID Foundation
   `fapi2-security-profile-final-test-plan` for every enabled sender constraint;
3. export an evidence manifest based on
   `fapi-conformance-evidence.example.json`, with every plan marked `passed`;
4. archive that manifest outside the repository, approve its SHA-256 digest,
   and set `NVBES_FAPI_CONFORMANCE_EVIDENCE_FILE` plus
   `NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256`;
5. run `pnpm check:fapi-conformance` or the production release gate.

The example manifest is deliberately `pending`; repository tests cannot claim
an official conformance result for an undeployed issuer.

The authorization server signs access, refresh, and ID tokens with PS256.
Scaleway KMS deployments must therefore use an RSA-PSS SHA-256 asymmetric
signing key; an RSA PKCS#1 v1.5 key is not compatible with the advertised
algorithm. Resource servers temporarily continue to verify already-issued
RS256 tokens until their natural expiry; all newly issued tokens use PS256.

JARM is not currently advertised or claimed. It can be added later as the
FAPI 2.0 Message Signing profile without changing the security meaning of the
two profiles above.

## Standards claims

Conformance communication names RFC 9700 and the applicable final RFCs rather
than OAuth 2.1, which remains an Internet-Draft as of July 2026. Authentication
documentation treats WebAuthn Level 2 as the stable W3C Recommendation;
WebAuthn Level 3 remains a Candidate Recommendation and is not presented as a
final standard.

## API guardrails

OAuth request bodies are limited to 64 KiB and ten seconds, compressed request
bodies are rejected, sensitive request schemas reject unknown fields, and RAR
objects have bounded entry counts and nesting depth. OAuth client-management
mutations require an `Idempotency-Key`.

Remote request-object retrieval is disabled by both profiles. Outbound
security-event delivery requires HTTPS, resolves only to public addresses,
pins each connection to the validated DNS answers, disables redirects, and
uses a five-second timeout. The destination is matched exactly against the
registered endpoint allowlist before resolution.

Webhook controls bind the payload, event identifier, and timestamp into the
signature, enforce a five-minute freshness window, and use stored delivery or
provider event identifiers as replay and idempotency guards. Financial
mutations use the shared idempotency-key mechanism.

## Normative references

- RFC 9700 — Best Current Practice for OAuth 2.0 Security
- RFC 8725 — JSON Web Token Best Current Practices
- RFC 8705 — OAuth 2.0 Mutual-TLS Client Authentication and
  Certificate-Bound Access Tokens
- RFC 9101 — JWT-Secured Authorization Request
- RFC 9126 — Pushed Authorization Requests
- RFC 9449 — Demonstrating Proof of Possession
- FAPI 2.0 Security Profile, Final, February 2025
