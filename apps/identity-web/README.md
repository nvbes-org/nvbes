# Identity Web

Site de connexion de l'Authorization Server, construit avec React, TanStack
Router et Vite+. Il appelle uniquement le SDK hébergé sur sa propre origine.
Les tokens des API restent la responsabilité des clients OAuth, dont Account.

## Parcours présents

- Connexion par mot de passe ou passkey existante.
- Step-up passkey, ou TOTP lorsque la politique serveur l'autorise.
- Affichage des scopes, consentement explicite et annulation.
- Annulation, timeout et incompatibilité WebAuthn connus : le statut serveur
  est relu et une autre méthode reste possible selon sa politique.
- Réponses refusées, interaction expirée et panne : messages sans données
  sensibles ; aucune relance automatique d'une mutation.
- Une seule initialisation par document, y compris pour PAR. L'état en mémoire
  est invalidé à la sortie de page pour empêcher sa réutilisation via bfcache.

L'ajout du premier facteur dispose d'écrans : création puis assertion passkey,
ou clé TOTP à saisir manuellement puis confirmation par code. Les secrets restent
en mémoire et sont retirés à expiration, annulation, confirmation ou sortie de page.
Les listes de facteurs orientent l'écran ; le serveur revérifie la fraîcheur de
l'authentification et l'autorisation à chaque mutation. Après une preuve forte
fraîche confirmée pour le client, le consentement permet également l'ajout de
facteurs supplémentaires. Un TOTP déjà actif n'est pas proposé à nouveau.
Configurer TOTP comme facteur supplémentaire ne satisfait pas une politique
WebAuthn : le serveur doit encore confirmer une preuve passkey avant consentement.
La gestion accessible après preuve forte affiche les passkeys et le facteur TOTP,
permet de renommer les passkeys et demande confirmation avant révocation. Le
dernier facteur ne peut pas être supprimé. Toute révocation réussie interrompt
ce parcours OAuth pour demander une nouvelle connexion avec un facteur conservé.
Le site Account reste à construire.

Un client à politique `primary` peut également proposer une confirmation MFA
volontaire depuis le consentement. TOTP ou passkey ouvre les actions de sécurité
avec une preuve de gestion distincte, bornée à cinq minutes au maximum. La
politique OAuth enregistrée n'est pas modifiée. Annuler cette confirmation rend
le consentement initial sans accorder d'accès de gestion. Les contrôles serveur
restent requis sur chaque opération sensible.
Aucun lien d'inscription publique n'est exposé.

Lorsqu'une politique OAuth exige une preuve forte fraîche et que le serveur la
confirme, le consentement propose de générer dix codes de secours. Un avertissement
précède la génération, qui remplace les anciens codes. L'affichage reste uniquement
en mémoire jusqu'à confirmation, expiration de la preuve ou sortie de page.
Après connexion par mot de passe, l'écran de step-up propose un code de secours
en cas de perte des facteurs. Sa consommation abandonne l'interaction OAuth et
ouvre `/recovery`, qui reprend uniquement la session de récupération. Cette page
permet de remplacer les facteurs par une passkey ou d'annuler, sans restaurer les
sessions révoquées. Une nouvelle connexion depuis l'application est obligatoire.
Le client doit abandonner sa transaction locale via `discardAuthorizationRequest`
avant de recommencer ; Identity ne peut pas effacer le stockage d'un autre site.

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
de la fixture : reconstruire avant de démarrer celle-ci et utiliser une fixture
neuve à chaque exécution (les facteurs du principal de test sont modifiés).
Il traverse les écrans mot de passe/consentement, mot de passe + TOTP,
mot de passe + WebAuthn et login direct passkey, puis l'échange OIDC/DPoP et
Account pour chaque méthode. Il vérifie l'absence de débordement à 390 px,
la reprise après annulation WebAuthn et le refus d'un code TOTP déjà consommé.

La préparation des facteurs passe par les cérémonies SDK actives, pas par
des écrans d'enrollment. CTAP2 et l'authenticator TOTP sont synthétiques ; le
scénario utilise la tolérance TOTP d'un pas pour ne pas attendre le changement
de période. L'annulation est simulée au niveau de navigator.credentials.get,
sans simuler les réponses HTTP. Les clés physiques et les navigateurs mobiles
restent à valider.

Pour tester l'ajout du premier facteur depuis les écrans, démarrer une fixture
neuve pour chaque méthode et ajouter `IDENTITY_WEB_TEST_SUITE=enrollment-totp`
ou `IDENTITY_WEB_TEST_SUITE=enrollment-passkey` à la commande navigateur.
Ces scénarios n'enregistrent aucun facteur via une préparation SDK ; ils vérifient
la confirmation, l'absence de secret dans le consentement et de stockage navigateur
Identity, puis le callback OIDC/DPoP et Account. TOTP couvre aussi l'abandon suivi
d'une nouvelle configuration avec une nouvelle clé. Les authentificateurs restent
synthétiques et les services HTTP réels.
Ils génèrent également les dix codes de secours depuis l'écran, vérifient leur
retrait après confirmation et l'absence de stockage navigateur sur Identity.
Ils utilisent ensuite un code depuis l'écran de step-up, rechargent `/recovery`,
annulent une première fenêtre de création puis enregistrent une passkey de
remplacement. Une nouvelle connexion avec cette passkey atteint Account ; les
anciennes transactions clientes sont abandonnées explicitement avec le SDK.
Avant la récupération, ces scénarios ajoutent aussi un second facteur : passkey
après TOTP ou TOTP après passkey, avec le step-up WebAuthn requis par le client.

Les suites `IDENTITY_WEB_TEST_SUITE=factors-totp` et `factors-passkey` utilisent
chacune une nouvelle fixture. Après préparation SDK des deux facteurs, elles
renomment une passkey via l'écran, annulent une première confirmation de suppression,
révoquent le facteur choisi puis se reconnectent avec le facteur conservé.
Elles vérifient que la suppression du dernier facteur n'est plus proposée et
terminent le callback OIDC/DPoP jusqu'à Account.
Les suites `security-totp` et `security-passkey` vérifient un client `primary`,
la confirmation MFA volontaire, l'accès aux facteurs et codes de secours, puis
le callback OIDC/DPoP. Elles vérifient que le statut OAuth conserve sa politique
initiale. Chaque suite utilise une fixture neuve.

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
