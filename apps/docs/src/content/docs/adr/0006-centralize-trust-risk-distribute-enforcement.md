---
title: "ADR 0006 - Centraliser Trust/Risk et distribuer l'enforcement"
description: Architecture Decision Record - nvbes platform
---

## Status

Accepted.

## Date

2026-08-02

## Context

Identity, Account, Cloud, Billing, Developer et Enterprise doivent détecter les
bots, les abus et la fraude. Dupliquer un moteur complet dans chaque produit
fragmenterait la réputation, les règles communes, les modèles, l'observabilité
et les capacités de réaction. À l'inverse, centraliser toutes les règles et
toutes les actions créerait un service omniscient, couplé aux détails métier de
chaque produit et placé sur tous les chemins critiques.

Identity ne doit pas devenir le moteur antifraude général de nvbes. Il reste
responsable des risques propres à l'identification et à l'autorisation, tels que
le credential stuffing, la prise de contrôle de compte, les inscriptions
automatisées, le MFA bombing et les anomalies de session.

Les autres produits restent seuls capables d'interpréter correctement leurs
risques métier : fraude au paiement, exfiltration de fichiers, abus de partage,
vol de clés API, invitations malveillantes ou changements administratifs
suspects.

## Decision

Créer un bounded context autonome `Trust/Risk`, indépendant d'Identity et des
domaines produit.

La plateforme Trust/Risk possède :

- la réputation globale des adresses réseau, appareils, principaux,
  organisations et identités machine ;
- la corrélation de vélocité et de comportement entre produits ;
- les règles transversales, modèles partagés, listes de blocage et signaux de
  menace ;
- le calcul d'un score de risque et de raisons explicables ;
- l'historique, l'audit et l'observabilité des évaluations globales ;
- les schémas versionnés des signaux et décisions partagés.

Chaque produit possède :

- la collecte des signaux nécessaires à son contexte ;
- les règles et seuils strictement métier ;
- la décision finale lorsque seule la connaissance du domaine permet de
  l'interpréter ;
- l'enforcement local : autoriser, limiter, bloquer, demander un step-up,
  suspendre ou placer en revue ;
- son comportement explicite lorsque Trust/Risk est indisponible.

L'architecture distingue deux couches :

1. Une couche anti-bot transversale au niveau Edge/Gateway pour le rate
   limiting, les challenges, les preuves de travail, l'intégrité du client et
   la réputation réseau.
2. Une couche antifraude Trust/Risk pour la corrélation globale, la détection
   comportementale et les recommandations de décision contextualisées.

Les décisions synchrones sont réservées aux opérations à risque nécessitant une
réponse immédiate. Le contrat retourne au minimum une décision parmi `allow`,
`deny`, `challenge` et `review`, accompagnée d'un identifiant, d'un score, de
raisons stables et d'une expiration.

Les signaux, résultats et retours d'enforcement non bloquants sont transportés
par des événements versionnés, idempotents et rejouables. Les produits ne
partagent pas directement leurs tables avec Trust/Risk.

La collecte respecte la minimisation des données. Les secrets, credentials,
contenus utilisateur et données métier brutes ne sont pas envoyés à la
plateforme lorsqu'un identifiant pseudonymisé ou un signal dérivé suffit. Les
décisions sensibles restent explicables et auditables.

## Consequences

- Les produits bénéficient d'une réputation et d'une détection coordonnées sans
  dupliquer le moteur transversal.
- Une attaque observée sur un produit peut protéger les autres produits par des
  signaux partagés et bornés.
- Identity reste un serveur AuthN/AuthZ intégrable et ne devient pas une
  dépendance antifraude générale.
- Les équipes produit conservent la maîtrise des faux positifs et des actions
  ayant un impact métier.
- Trust/Risk introduit un nouveau service critique, des contrats de signaux, une
  gouvernance des règles et des exigences de disponibilité et de latence.
- Chaque appel synchrone doit définir une stratégie fail-open, fail-closed ou
  fail-challenge adaptée à son risque ; aucun comportement implicite n'est
  accepté.
- Les modèles et règles globaux nécessitent des métriques de dérive, de précision,
  de faux positifs et de recours utilisateur.

## Validation

La direction est respectée uniquement si :

- aucun produit ne réimplémente un moteur transversal complet ;
- Trust/Risk ne contient pas de logique métier spécifique à un produit ;
- Identity ne décide que des risques AuthN/AuthZ dont il est propriétaire ;
- chaque décision expose un identifiant et des raisons auditables ;
- les contrats synchrones ont des timeouts et un comportement dégradé explicite ;
- les événements sont versionnés, idempotents et rejouables ;
- la rétention, la pseudonymisation et les accès aux signaux sont documentés et
  testés ;
- l'enforcement final reste local au service propriétaire de l'opération.
