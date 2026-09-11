# ADR 0013 - Préparer les sites et clients multiplateformes du socle

## Status

Proposed — 2026-09-07. Proposition de préparation, sans changement runtime.

## Context

Le périmètre demandé comprend deux sites distincts : Identity pour les parcours
OAuth/OIDC hébergés et Account pour la gestion du compte. Landing et Home, puis
Desktop, Mobile, TV et Watch, sont des consommateurs futurs, pas des lots V1
implicitement ouverts. La [direction V1](../product/nvbes-product-strategy.md)
et le plafond global de 30 EUR TTC/mois restent applicables.

Les [frontières Identity/Account](0005-separate-identity-from-account.md) et
[Trust/Risk](0006-centralize-trust-risk-distribute-enforcement.md) restent
applicables. La présence d'un SDK ou de routes archivées ne prouve pas qu'un
contrat est exposé par le runtime actif. La
[matrice vérifiée](../architecture/frontend-platform-preparation.md) consigne
les écarts et bloque l'ouverture des parcours concernés.

## Decision

- Créer ultérieurement deux applications autonomes, `identity-web` et
  `account-web`, sans restaurer les applications archivées par défaut.
- Garder des contrats publics HTTP/JSON par domaine, décrits avec OpenAPI et
  vérifiés contre les routeurs actifs avant génération de clients.
- Partager les contrats et transports indépendamment des composants React.
  Ajouter les SDK natifs uniquement avec les plateformes effectivement choisies.
- Utiliser une entrée HTTP logique commune de routage, sans nouveau service
  d'agrégation obligatoire. Les hôtes publics, chemins, réécritures et CORS
  doivent être arrêtés avant le branchement. Conserver l'issuer Identity stable.
- Ne pas adopter GraphQL, Federation ou gRPC-Web dans ce lot. Réexaminer GraphQL
  sur des besoins d'agrégation mesurés ; Federation exige en plus un besoin de
  composition indépendante des domaines. Le gRPC interne Trust/Risk demeure.
- Identity Web porte uniquement l'authentification et l'autorisation hébergées.
  Account Web ne collecte jamais les credentials Identity. La gestion des
  primitives de sécurité passe par une façade Account vers Identity, comme
  prévu par ADR 0005 ; cette façade n'est pas encore attestée dans le runtime.
- Account Web utilise OIDC Authorization Code avec PKCE S256 selon ADR 0005.
  Les scopes et audiences sont distincts par service. Un token Account ne doit
  jamais être transmis à Billing. Le mécanisme d'obtention du token Billing
  constitue un gate préalable, pas une hypothèse de Token Exchange disponible.
- Ne pas créer de BFF de session séparé à ce stade. Sa sélection ultérieure
  exige une décision explicite sur le stockage des tokens, les cookies, CSRF,
  la révocation et son coût. Elle ne doit pas changer implicitement le modèle
  de client public retenu par ADR 0005.
- Chaque service contrôle ses ressources et applique localement Trust/Risk.
  Une dépendance fonctionnelle d'un site n'implique pas un appel navigateur.
  Les décisions et opérations Trust/Risk restent internes ; une éventuelle
  collecte publique exige un contrat séparé, minimal et borné.

## Consequences

Les sites et futurs clients évoluent indépendamment des services et des UI web.
Les autorisations, versions et erreurs doivent être cohérentes ; plusieurs
audiences imposent une gestion explicite des tokens. Le routage partagé ne
remplace ni l'autorisation métier ni l'isolation des domaines.

La préparation documentaire ne crée aucune ressource ni coût récurrent.
L'hébergement des deux sites et le routage devront fournir leur coût fixe,
leur pire coût variable plafonné et la preuve du total inférieur à 30 EUR TTC.
Aucun gain financier d'une gateway ou d'un BFF n'est présumé.

Le premier parcours est Account → Identity → callback Account → profil →
déconnexion. Il reste bloqué jusqu'à livraison complète du protocole Identity
actif, des contrats de session et des tests intersites. Billing suit après ses
gates de sécurité. Aucune inscription publique ni paiement réel n'est ouvert
par cette décision.
