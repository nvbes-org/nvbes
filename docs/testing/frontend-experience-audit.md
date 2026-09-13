# Audit Expérience Frontend & Parcours : Suite de Tests Qualité

> **Statut : Document de référence V1.** Ce document établit l'audit exhaustif, les
> décisions d'architecture et les spécifications techniques de la suite de tests
> pour l'expérience frontend et les parcours utilisateurs de nvbes. Il couvre
> l'ensemble des exigences fonctionnelles, d'accessibilité (a11y), de non-régression
> visuelle et de compatibilité multi-navigateurs, sous la contrainte FinOps absolue
> de **30 EUR TTC par mois**.

---

## 1. Contexte & Enjeux Produit

La stratégie produit canonique de nvbes ([Direction Produit V1](../product/nvbes-product-strategy.md))
définit un socle B2C sécurisé articulé autour de 5 domaines : **Identity**, **Account**,
**Billing**, **Email** et **Trust/Risk**, complétés par **Platform Operations**.

L'expérience utilisateur repose sur des interfaces web hautement interactives,
soumises à de fortes contraintes réglementaires (RGPD, CNIL, directives d'accessibilité UE)
et de sécurité (authentification durcie, cookies `__Host-`, protection contre le vol de session,
défense anti-bots).

Pour garantir une expérience irréprochable sans dette technique, la qualité frontend
doit s'articuler autour de 4 piliers complémentaires :

```
                 ┌──────────────────────────────────────────────┐
                 │     Tests E2E Multi-Navigateurs (Playwright)  │
                 │   Parcours critiques : Signup, MFA, Cookies  │
                 ├──────────────────────────────────────────────┤
                 │     Tests Visuels Pixel-Perfect (Playwright) │
                 │      Régression layout, thèmes, responsive   │
                 ├──────────────────────────────────────────────┤
                 │   Tests d'Accessibilité (Axe-core WCAG AA)   │
                 │     Contraste, focus trap, aria, target size │
                 ├──────────────────────────────────────────────┤
                 │    Tests de Composants (RTL + MSW v2)        │
                 │    États UI, isolation réseau, formulaires   │
                 └──────────────────────────────────────────────┘
```

---

## 2. Synthèse de l'Audit de l'Existant

### 2.1 Cartographie du Monorepo

- **Applications Web Historiques** : `identity-web` et `account-web` sont positionnées dans `archive/apps/`.
- **Primitives UI** : Plus de 40 composants basés sur Radix UI / shadcn (`archive/apps/identity-web/src/components/ui/*`), incluant dialogues, formulaires, alertes, selects, sheets, barres de progression et jauges de robustesse de mot de passe.
- **Formulaires & Pages Métier** : `LoginPage` (multi-étapes avec identifiant, mot de passe, WebAuthn/passkey, TOTP MFA, consentement), `RegisterPage` (vérification de disponibilité en temps réel), `VerifyEmailPage`, `VerifyEmailResultPage`, et les écrans de gestion de compte (`AccountProfilePage`, `AccountSecurityPage`, `AccountPrivacyPage`).

### 2.2 Grille de Maturité par Pilier

| Pilier                                 | État Actuel                                            | Diagnostic & Risques Majeurs                                                                                                                                                                                                                                                 |  Maturité   |
| :------------------------------------- | :----------------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | :---------: |
| **1. Tests de Composants (RTL + MSW)** | 5 fichiers tests isolés utilisant Vitest et Happy-DOM. | **MSW inexistant**. Utilisation de `vi.mock()` sur les queries internes (`identity.auth.queries.ts`), couplant les tests aux fonctions internes plutôt qu'aux contrats HTTP. Les codes d'erreur (409, 429, 500), les en-têtes et le cache TanStack Query ne sont pas testés. | **1.5 / 5** |
| **2. Tests Visuels (Pixel-perfect)**   | Aucun test visuel existant.                            | Risque élevé de régressions CSS invisibles lors des montées de version Tailwind/Radix, chevauchement de z-index, rupture de reflow en mode sombre (`dark`) ou sur mobile.                                                                                                    |  **0 / 5**  |
| **3. Accessibilité (a11y - Axe-core)** | `@axe-core/playwright` dans `accessibility.spec.ts`.   | **Audit partiel** : seules 2 pages sont couvertes (`/login`, `/register`). Le runner filtre et ignore arbitrairement les violations `moderate` et `minor`. Zéro test au niveau composant. Aucune règle WCAG 2.2 vérifiée (Target Size 2.5.8, Focus Appearance 2.4.11).       |  **2 / 5**  |
| **4. E2E Multi-Navigateurs**           | Playwright configuré sur 5 profils de devices.         | **Exécution restreinte à Chromium** dans les scripts CI et `package.json` (`--project=chromium`). Arguments Blink propriétaires (`--disable-blink-features=AutomationControlled`). Absence de test des flux MFA TOTP et de gestion du compte.                                | **2.5 / 5** |

---

## 3. Spécifications & Architecture par Pilier

---

### Pilier 1 : Tests de Composants (React Testing Library + MSW v2)

#### 3.1 Pourquoi abandonner `vi.mock()` au profit de MSW ?

L'anti-pattern actuel dans `useRegisterPage.availability.test.tsx` :

```typescript
// ❌ ANTI-PATTERN : Mock de la fonction interne JS
vi.mock('../src/identity.auth.queries', () => ({
  identityAuthQueryKeys: { registrationAvailability: (email) => [...] },
  registrationAvailabilityQueryFn: mocks.checkAvailability,
}));
```

**Conséquences néfastes** :

1. **Faux sentiment de sécurité** : Si le backend modifie la structure du payload JSON ou le chemin de l'endpoint (`/auth/check-email` $\to$ `/auth/registration-availability`), le test reste vert alors que la production est cassée.
2. **Ignorance des couches réelles** : Ne teste ni `fetch`, ni les sérialiseurs JSON, ni les intercepteurs d'en-têtes (CSRF, cookies de session), ni le comportement de reprise sur erreur de TanStack Query.

#### 3.2 Architecture Cible MSW v2

MSW intercepte les requêtes réseau au niveau du protocole HTTP sans modifier le code applicatif :

```
[Composant React]
       │
       ▼ (fetch natif)
[TanStack Query]
       │
       ▼ (requête HTTP)
[MSW SetupServer] ──► Vérifie URL, Méthode, Headers, Query Params
       │
       ├─► Réponse 200 OK (JSON conforme OpenAPI)
       ├─► Réponse 409 Conflict (Email déjà utilisé)
       ├─► Réponse 429 Too Many Requests (Rate limit)
       └─► Réponse 500 / Network Error (Mode dégradé UI)
```

#### 3.3 Implémentation de Référence

```typescript
// src/test/handlers/identity.handlers.ts
import { http, HttpResponse } from 'msw';

export const identityHandlers = [
  http.get('/auth/registration-availability', ({ request }) => {
    const url = new URL(request.url);
    const email = url.searchParams.get('email');

    if (!email || !email.includes('@')) {
      return HttpResponse.json(
        { error: 'invalid_email', message: 'Format invalide.' },
        { status: 400 },
      );
    }
    if (email.toLowerCase().startsWith('pris@')) {
      return HttpResponse.json({ available: false });
    }
    return HttpResponse.json({ available: true });
  }),

  http.post('/auth/register', async ({ request }) => {
    const body = (await request.json()) as { email?: string; password?: string };
    if (body.email?.startsWith('conflit@')) {
      return HttpResponse.json(
        { code: 'EMAIL_ALREADY_EXISTS', message: 'Cet email est déjà utilisé.' },
        { status: 409 },
      );
    }
    return HttpResponse.json({ ok: true, requiresVerification: true }, { status: 201 });
  }),
];
```

```typescript
// src/test/test-utils.tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { render, type RenderOptions } from '@testing-library/react';
import { type ReactElement, type ReactNode } from 'react';

export function createTestQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0 },
      mutations: { retry: false },
    },
  });
}

export function renderWithProviders(ui: ReactElement, options?: Omit<RenderOptions, 'wrapper'>) {
  const queryClient = createTestQueryClient();
  function Wrapper({ children }: { children: ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  }
  return { ...render(ui, { wrapper: Wrapper, ...options }), queryClient };
}
```

---

### Pilier 2 : Tests Visuels (Chromatic vs Playwright & Arbitrage FinOps)

#### 3.4 Arbitrage FinOps : Chromatic vs Playwright Native

| Critère                     | Chromatic (Storybook Cloud)                                     | Playwright Native Screenshot                                         |
| :-------------------------- | :-------------------------------------------------------------- | :------------------------------------------------------------------- |
| **Coût d'exploitation**     | **149 $ / mois** (au-delà de 5 000 snapshots gratuits/mois).    | **0 EUR** de surcoût SaaS.                                           |
| **Conformité FinOps nvbes** | **NON CONFORME** (Plafond total = 30 € TTC/mois).               | **CONFORME** (Zero SaaS cost).                                       |
| **Infrastructure**          | Cloud tiers propriétaire, clés API externes, dépendance réseau. | 100% hébergé en local / conteneur CI Scaleway.                       |
| **Dépendance outillage**    | Nécessite Storybook complet pour chaque composant.              | Teste directement les pages et composants réels via le navigateur.   |
| **Déterminisme**            | Géré côté cloud.                                                | Maîtrisé via Docker Linux pour standardiser le sous-pixel rendering. |

> [!IMPORTANT]
> **Décision d'architecture** : Playwright Native Screenshot Comparison (`toHaveScreenshot()`)
> est la seule solution recevable respectant le budget FinOps et l'indépendance de la plateforme.

#### 3.5 Spécification de la Suite Visuelle Playwright

Pour éliminer les faux positifs (flakiness) liés aux polices de caractères, horodatages et animations :

1. **Désactivation des animations et transitions** :
   ```css
   *,
   *::before,
   *::after {
     animation-delay: 0s !important;
     animation-duration: 0.01ms !important;
     transition-delay: 0s !important;
     transition-duration: 0.01ms !important;
   }
   ```
2. **Masquage des zones variables** :
   ```typescript
   await expect(page).toHaveScreenshot('login-page.png', {
     mask: [page.locator('.dynamic-timestamp'), page.locator('[data-testid="csrf-token"]')],
     maxDiffPixelRatio: 0.002, // Seuil de 0.2% max
     threshold: 0.2, // Sensibilité chromatique par pixel
     animations: 'disabled',
     caret: 'hide',
   });
   ```
3. **Matrice de Test Visuel** :
   - **Thèmes** : Clair (`light`) et Sombre (`dark`).
   - **Viewports** :
     - Desktop : $1280 \times 800$
     - Tablette : $768 \times 1024$
     - Mobile : $375 \times 667$ (iPhone SE / écran standard)
   - **Pages cibles prioritaires** : `/login`, `/register`, `/verify`, `/verify-result`, et les modales (`CookieConsentSettings`, `DialogConfirm`).

---

### Pilier 3 : Tests d'Accessibilité (Axe-core & Conformité WCAG 2.1/2.2 AA)

#### 3.6 Architecture à Double Barrière

L'accessibilité ne doit pas être validée uniquement en fin de chaîne lors des E2E. Une double barrière garantit un feedback immédiat pour le développeur :

```
┌────────────────────────────────────────────────────────┐
│ BARRIÈRE 1 : Niveau Composant (Vitest + axe-core)      │
│ - Vérifie les primitives UI isolées (Boutons, Inputs)  │
│ - Détecte rôles ARIA orphelins, labels manquants       │
├────────────────────────────────────────────────────────┤
│ BARRIÈRE 2 : Niveau Page & E2E (Playwright + AxeBuilder│
│ - Structure sémantique globale (h1 -> h2, landmarks)   │
│ - Ordre de tabulation clavier & piégeage focus modale  │
│ - Ratios de contraste sur thèmes clair & sombre        │
│ - Vérification des règles WCAG 2.2 AA                   │
└────────────────────────────────────────────────────────┘
```

#### 3.7 Intégration des Critères WCAG 2.2 AA

En plus des règles WCAG 2.1 AA (contraste 4.5:1, landmarks, attributs alt), la suite intègre les ajouts de la norme WCAG 2.2 :

1. **Critère 2.5.8 (Target Size - Minimum)** :
   - Toute cible interactive (bouton, lien, case à cocher) doit avoir une surface minimale de **$24 \times 24$ pixels CSS**, ou un espacement suffisant pour éviter les clics accidentels. Sur mobile, la recommandation ergonomique cible est **$44 \times 44$ pixels**.
2. **Critère 2.4.11 (Focus Appearance)** :
   - L'indicateur de focus clavier doit être distinct et visible, avec un outline de minimum 2px et un ratio de contraste d'au moins **$3:1$** par rapport au fond adjacent.
3. **Critère 3.3.8 (Accessible Authentication)** :
   - Interdiction d'imposer un test cognitif (mémorisation de motifs complexes, énigmes). L'interface doit autoriser le copier-coller pour les mots de passe et codes TOTP, et exposer les attributs HTML d'autocomplétion appropriés (`autocomplete="one-time-code"`).

#### 3.8 Règle Zéro Tolérance (Suppression du filtre laxiste)

Dans `archive/apps/identity-web/e2e/accessibility.spec.ts`, le code actuel filtrait les erreurs :

```typescript
// ❌ INSUFFISANT : Ignore les violations modérées requises en AA
const blockingViolations = result.violations.filter(
  ({ impact }) => impact === 'critical' || impact === 'serious',
);
```

**Nouvelle règle de conformité stricte** :

```typescript
// ✅ CONFORME : 0 violation sur les tags WCAG 2.1 & 2.2 AA
const tags = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];
const result = await new AxeBuilder({ page }).withTags(tags).analyze();
expect(result.violations).toEqual([]);
```

---

### Pilier 4 : Tests E2E Navigateur (Playwright Multi-Navigateurs)

#### 3.9 Matrice Multi-Moteurs Réelle

Les tests E2E ne doivent plus être limités à Chromium. La suite Playwright doit exécuter les parcours critiques sur les trois moteurs de rendu majeurs et sur émulateurs mobiles :

```typescript
// playwright.config.ts
export default defineConfig({
  projects: [
    {
      name: 'desktop-chromium',
      use: { ...devices['Desktop Chrome'] },
    },
    {
      name: 'desktop-firefox',
      use: { ...devices['Desktop Firefox'] },
    },
    {
      name: 'desktop-webkit',
      use: { ...devices['Desktop Safari'] },
    },
    {
      name: 'mobile-chromium',
      use: { ...devices['Pixel 7'] },
    },
    {
      name: 'mobile-webkit',
      use: { ...devices['iPhone 15'] },
    },
  ],
});
```

#### 3.10 Neutralisation des Incompatibilités Multi-Navigateurs

1. **Suppression des drapeaux Chromium propriétaires** :
   Ne pas utiliser `--disable-blink-features=AutomationControlled` qui casse ou est ignoré sur WebKit et Firefox.
2. **Spécificités Safari WebKit (ITP & Cookies)** :
   WebKit applique une politique stricte sur les cookies tiers et les attributs `SameSite`. Les tests vérifient que les flux d'authentification fonctionnent avec des cookies propriétaires (`First-party`) préfixés `__Host-` et sécurisés (`Secure`, `HttpOnly`, `SameSite=Lax`).
3. **Gestion du Focus Clavier sur Safari / macOS** :
   Par défaut sur macOS/WebKit, la touche Tab ne navigue pas sur les boutons non textuels sans activer `tabIndex=0` explicite ou utiliser l'API accessible de Playwright.

#### 3.11 Cartographie des Parcours Critiques E2E

```
[PARCOURS 1 : Inscription & Activation]
Inscription (email + mdp fort) ──► Capture Email isolée ──► Clic token vérification ──► Compte Activé
                                                                                           │
[PARCOURS 2 : Authentification & MFA]                                                      ▼
Connexion identifiant ──► Saisie MDP ──► Enrôlement TOTP (QR/Secret) ──► Validation code ──► Session Active
                                                                                           │
[PARCOURS 3 : Sécurité des Sessions]                                                       ▼
Vérification cookies __Host- ──► Accès espace sécurisé ──► Déconnexion ──► Invalidation token côté serveur
                                                                                           │
[PARCOURS 4 : Cycle de Vie & RGPD]                                                         ▼
Modification profil ──► Demande export archive RGPD ──► Suppression de compte ──► Purge données
```

---

## 4. Plan de Déploiement & Intégration CI

### 4.1 Commandes et Scripts de Déclenchement

| Commande                          | Scope                                          | Déclencheur CI             |
| :-------------------------------- | :--------------------------------------------- | :------------------------- |
| `pnpm test:web:components`        | Vitest + React Testing Library + MSW v2        | Pre-commit / PR            |
| `pnpm test:web:a11y`              | Axe-core sur composants et pages complètes     | PR / Nightly               |
| `pnpm test:web:visual`            | Playwright `toHaveScreenshot()` multi-viewport | PR avec changements UI     |
| `pnpm test:web:e2e:cross-browser` | Playwright Chromium + Firefox + WebKit         | Pre-release Staging / Gate |

### 4.2 Critères d'Acceptation de la Gate de Release

Une release frontend est validée pour la production si et seulement si :

1. **Composants** : 100% des tests RTL/MSW verts, zéro mock réseau non typé par MSW.
2. **Accessibilité** : 0 violation Axe-core (`critical`, `serious`, `moderate`) sur les tags WCAG 2.1/2.2 AA.
3. **Visuel** : Tolérance de divergence pixel $\le 0.002$ sur la baseline de référence Docker Linux.
4. **E2E** : 100% des parcours critiques réussis sur Chromium, Firefox, WebKit et Mobile.
5. **FinOps** : 0 ressource externe payante sollicitée.
