# Identity Web

Site de connexion de l'Authorization Server, construit avec React, TanStack
Router et Vite+. Il appelle uniquement le SDK hébergé sur sa propre origine.
Les tokens des API restent la responsabilité des clients OAuth, dont Account.

## Parcours présents

- Connexion par mot de passe ou passkey existante.
- Step-up passkey, ou TOTP lorsque la politique serveur l'autorise.
- Affichage des scopes, consentement explicite et annulation.
- Réponses refusées, interaction expirée et panne : messages sans données
  sensibles ; aucune relance automatique d'une mutation.
- Une seule initialisation par document, y compris pour PAR. L'état en mémoire
  est invalidé à la sortie de page pour empêcher sa réutilisation via bfcache.

L'enrollment, la récupération, la gestion des facteurs et le site Account
restent à raccorder à des écrans. Aucun lien d'inscription publique n'est exposé.

## Développement

Depuis la racine :

```sh
pnpm nx run identity-web:build
pnpm dev:identity-web
```

Le serveur écoute sur `http://127.0.0.1:4200`. Sans backend configuré, seule
la page d'entrée est exploitable. Pour raccorder un runtime local préparé :

```sh
IDENTITY_WEB_BACKEND_URL=http://127.0.0.1:3060 pnpm dev:identity-web
```

Le backend doit être configuré pour l'origine navigateur du site et ses clients
OAuth enregistrés. Le proxy conserve Origin et les cookies ; il ne transforme
pas une configuration serveur incompatible en configuration autorisée.
Seules les origines HTTP loopback explicites sont admises comme cible de dev.
La navigation interactive GET /oauth/authorize sert le site ; les appels JSON
et les requêtes silencieuses vont au serveur. Aucun proxy public n'est déployé.
Le démarrage commun `pnpm dev` conserve les cinq services ; l'ajout coordonné
des deux sites et de leurs registres clients reste à effectuer.

## Validation reproductible

```sh
pnpm nx run-many -t build,typecheck,test,lint,format:check -p identity-web
# Terminal A : base isolée, runtimes réels, certificats de test, arrêt après 10 min.
NVBES_IDENTITY_TEST_WEB_UI=1 pnpm nx run identity-service:test:https-browser-fixture
# Terminal B : reprendre exclusivement l'origine client imprimée par la fixture.
IDENTITY_WEB_TEST_CLIENT_ORIGIN=https://127.0.0.1:PORT pnpm nx run identity-web:test:browser
```

Playwright doit disposer de Chromium (`pnpm --filter @nvbes/identity-web exec
playwright install chromium`). Le scénario utilise le build présent au démarrage
de la fixture : reconstruire avant de démarrer celle-ci. Il traverse les écrans
mot de passe/consentement, l'échange OIDC/DPoP et Account, avec des identifiants
synthétiques. Il vérifie aussi l'absence de débordement à 390 px.
Les parcours graphiques passkey/TOTP de ce nouveau site restent à prouver.

## Composants et exploitation

Le registry interne ne fournissait pas les contrôles de formulaire requis.
Tailwind a été installé avant le registry officiel shadcn (Radix Nova) :
Button, Input, Field, Alert, Label et Separator. La page compose ces primitives
en Tailwind ; les seules règles CSS propres sont les tokens du thème.
Les polices Geist et Fraunces sont servies localement ; aucun appel Google Fonts,
analytics ou fournisseur d'authentification n'est ajouté.

Les versions Vite+/core/Vitest/coverage sont alignées selon la
[migration officielle Vite+](https://viteplus.dev/guide/migrate).
L'alias core porte une version 0.2.9, d'où les avertissements de peers Vite 8 ;
les contrôles de types, builds et tests utilisent le même moteur.

L'hébergement, le routage HTTP de production, les en-têtes CSP/no-store,
la compatibilité matérielle et les gates de budget/exploitation restent ouverts.
La fixture applique une CSP sans scripts externes et sans inline ; sa configuration
de test et ses identifiants ne sont jamais inclus dans le build du site.
