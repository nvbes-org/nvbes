#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { buildServiceDocPrompt } from './llm-contracts.mjs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const REPO_ROOT = resolve(__dirname, '../..');

const OUT_DIR = join(REPO_ROOT, 'apps/docs/src/content/docs/architecture');

function ensureDir(dir) {
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
}

/**
 * Service Knowledge Base for grounded domain doc synthesis
 */
const SERVICE_DOMAIN_KNOWLEDGE = {
  'identity-service': {
    domain: 'identity',
    title: 'Service Identity & Authentification',
    description: 'Architecture du service d identité, gestion des sessions, flux OAuth2/OIDC, PKCE et WebAuthn.',
    role: 'Fournisseur central d identité, émetteur de jetons DPoP JWT et gestionnaire de credentials pour l ensemble de l écosystème nvbes.',
    invariants: [
      'Les mots de passe ne sont jamais stockés en clair (hachage Argon2id obligatoire).',
      'Toute session WebAuthn / FIDO2 doit valider le challenge cryptographique côté serveur.',
      'Les jetons d accès utilisent le protocole DPoP (RFC 9449) pour lier le token à la clé privée du client.',
      'Les flux OAuth2 supportent obligatoirement PKCE (S256).',
    ],
    security: {
      authMethod: 'DPoP JWT + WebAuthn + Sessions HTTP-only cookies',
      requiredScopes: ['openid', 'profile', 'email', 'offline_access'],
    },
    sequenceMermaid: `sequenceDiagram
    autonumber
    actor User as "Navigateur / Client Web"
    participant GW as "gateway-cloud"
    participant ID as "identity-service"
    participant DB as "PostgreSQL (Identity DB)"
    participant Redis as "Redis Session Cache"

    User->>GW: POST /api/v1/auth/login (email, password)
    GW->>ID: Route la requete d authentification
    ID->>DB: Recherche de l utilisateur par hash d email
    DB-->>ID: Enregistrement utilisateur & sel Argon2id
    ID->>ID: Verification du mot de passe
    ID->>Redis: Creation de session & DPoP proof
    ID-->>GW: Set-Cookie (Session HTTP-only) + DPoP Token
    GW-->>User: 200 OK (Authentifie)`,
    errors: [
      { errorType: 'InvalidCredentials', httpCode: 401, recoveryStrategy: 'Incrémenter le compteur d échecs, inviter à réinitialiser le mot de passe.' },
      { errorType: 'SessionExpired', httpCode: 401, recoveryStrategy: 'Rediriger vers le flux de ré-authentification ou rafraîchir via Refresh Token.' },
      { errorType: 'MfaRequired', httpCode: 403, recoveryStrategy: 'Déclencher le challenge WebAuthn ou TOTP.' },
    ],
  },
  'cloud-service': {
    domain: 'cloud',
    title: 'Service Cloud & Gestion des Fichiers',
    description: 'Architecture du service Cloud/Drive, quotas de stockage, chiffrement et upload/download multipart.',
    role: 'API de stockage objet, gestion des espaces partagés, métadonnées de fichiers et contrôle d accès fin.',
    invariants: [
      'Tout upload direct vers le stockage objet nécessite une URL pré-signée validée par cloud-service.',
      'Les quotas d espace disque sont vérifiés de manière atomique avant l émission des tokens d upload.',
      'Le partage de documents applique un modèle de permissions immuable avec révocation instantanée.',
    ],
    security: {
      authMethod: 'Bearer DPoP JWT (émis par Identity)',
      requiredScopes: ['drive:read', 'drive:write', 'drive:share', 'drive:admin'],
    },
    sequenceMermaid: `sequenceDiagram
    autonumber
    actor User as "Client Drive Web"
    participant GW as "gateway-cloud"
    participant Cloud as "cloud-service"
    participant Storage as "Object Storage S3"
    participant DB as "PostgreSQL (Cloud DB)"

    User->>GW: POST /api/v1/files/upload-ticket
    GW->>Cloud: Verification du quota et des droits
    Cloud->>DB: Verifie l espace disque disponible
    Cloud->>Storage: Genere l URL pre-signee S3
    Cloud-->>User: 200 OK (URL pre-signee S3 + Token)
    User->>Storage: PUT Direct vers S3 avec le binaire
    Storage-->>User: 200 OK (Upload termine)
    User->>Cloud: POST /api/v1/files/confirm-upload
    Cloud->>DB: Enregistre les metadonnees du fichier`,
    errors: [
      { errorType: 'QuotaExceeded', httpCode: 402, recoveryStrategy: 'Inviter l utilisateur à mettre à niveau son abonnement de stockage.' },
      { errorType: 'UnauthorizedAccess', httpCode: 403, recoveryStrategy: 'Refuser l accès, journaliser l événement dans l audit append-only.' },
      { errorType: 'FileNotFound', httpCode: 404, recoveryStrategy: 'Vérifier la validité de l identifiant ou si le fichier a été purgé.' },
    ],
  },
  'billing-service': {
    domain: 'billing',
    title: 'Service Billing & Abonnements',
    description: 'Architecture du moteur de facturation, gestion multi-PSP (Stripe), webhooks idempotents et gestion des droits.',
    role: 'Gestionnaire des souscriptions, du calcul d usage et de la synchronisation des états de facturation.',
    invariants: [
      'Tous les webhooks PSP (Stripe) doivent être traités avec une idempotence stricte.',
      'Le routage des paiements est neutre vis-à-vis des fournisseurs (ADR 0004).',
      'Aucun changement de statut de souscription n est appliqué sans audit append-only.',
    ],
    security: {
      authMethod: 'gRPC mTLS interne + Signature HMAC des webhooks Stripe',
      requiredScopes: ['billing:read', 'billing:admin'],
    },
    sequenceMermaid: `sequenceDiagram
    autonumber
    actor Stripe as "Stripe PSP Webhook"
    participant GW as "gateway-cloud"
    participant Billing as "billing-service"
    participant DB as "PostgreSQL (Billing DB)"
    participant Worker as "billing-worker"

    Stripe->>GW: POST /webhooks/stripe (Event payload + Sig)
    GW->>Billing: Transmet le webhook avec verification HMAC
    Billing->>Billing: Verifie la cle d idempotence
    Billing->>DB: Enregistre l evenement brut dans l audit ledger
    Billing->>Worker: Publie la tache asynchrone d ajustement des droits
    Billing-->>Stripe: 200 OK (Recu)
    Worker->>DB: Met a jour les entitlements du compte client`,
    errors: [
      { errorType: 'InvalidWebhookSignature', httpCode: 400, recoveryStrategy: 'Rejeter immédiatement la requête et alerter la sécurité.' },
      { errorType: 'DuplicateEventIgnored', httpCode: 200, recoveryStrategy: 'Renvoyer 200 OK sans retraiter l événement (Idempotence).' },
      { errorType: 'PaymentFailed', httpCode: 402, recoveryStrategy: 'Notifier le client par email et entrer en période de grâce.' },
    ],
  },
};

export function generateDomainDoc(serviceKey) {
  const knowledge = SERVICE_DOMAIN_KNOWLEDGE[serviceKey];
  if (!knowledge) {
    console.warn(`[SKIP] No domain knowledge found for "${serviceKey}".`);
    return;
  }

  ensureDir(OUT_DIR);

  let doc = `---
title: ${knowledge.title}
description: ${knowledge.description}
---

## 1. Rôle Architectural

${knowledge.role}

- **Domaine métier** : \`${knowledge.domain}\`
- **Mécanisme d'authentification** : ${knowledge.security.authMethod}
- **Scopes requis** : ${knowledge.security.requiredScopes.map((s) => `\`${s}\``).join(', ')}

---

## 2. Invariants & Règles Métier

Ces règles constituent les garanties fondamentales du service :

${knowledge.invariants.map((inv) => `- **Règle** : ${inv}`).join('\n')}

---

## 3. Flux Principal (Diagramme de Séquence)

\`\`\`mermaid
${knowledge.sequenceMermaid}
\`\`\`

---

## 4. Matrice des Erreurs & Stratégies de Résilience

| Type d'Erreur | Code HTTP | Stratégie de Récupération |
| :--- | :--- | :--- |
`;

  for (const err of knowledge.errors) {
    doc += `| \`${err.errorType}\` | \`${err.httpCode}\` | ${err.recoveryStrategy} |\n`;
  }

  const outFilePath = join(OUT_DIR, `${serviceKey}.md`);
  writeFileSync(outFilePath, doc, 'utf8');
  console.log(`✅ Generated rich domain architecture doc: apps/docs/src/content/docs/architecture/${serviceKey}.md`);
}

export function generateAllDomainDocs() {
  console.log('🤖 Synthesizing grounded domain architecture docs...');
  for (const key of Object.keys(SERVICE_DOMAIN_KNOWLEDGE)) {
    generateDomainDoc(key);
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const target = process.argv[2];
  if (target) {
    generateDomainDoc(target);
  } else {
    generateAllDomainDocs();
  }
}
