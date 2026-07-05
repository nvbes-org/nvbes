export const quickstarts = {
  react: {
    title: 'React quickstart',
    command: 'pnpm add @nvbes/identity-sdk-web',
    code: `import { createIdentityClient } from '@nvbes/identity-sdk-web';

export const identity = createIdentityClient({
  issuer: import.meta.env.VITE_NVBES_ACCOUNT_ISSUER,
  clientId: import.meta.env.VITE_NVBES_CLIENT_ID,
  redirectUri: window.location.origin + '/auth/callback',
});`,
  },
  'rust-axum': {
    title: 'Rust Axum quickstart',
    command: 'cargo add nvbes-identity-sdk-backend',
    code: `use axum::{routing::get, Router};

async fn me() -> &'static str {
    "validated nvbes Identity session"
}

pub fn router() -> Router {
    Router::new().route("/me", get(me))
}`,
  },
  node: {
    title: 'Node quickstart',
    command: 'pnpm add @nvbes/identity-sdk',
    code: `import { createIdentityClient } from '@nvbes/identity-sdk';

const identity = createIdentityClient({
  baseUrl: process.env.NVBES_ACCOUNT_SERVICE_URL,
});`,
  },
  curl: {
    title: 'curl quickstart',
    command: 'curl https://identity.nvbes.fr/.well-known/openid-configuration',
    code: `curl -s https://identity.nvbes.fr/.well-known/openid-configuration | jq .issuer`,
  },
} as const;

export type QuickstartKey = keyof typeof quickstarts;
