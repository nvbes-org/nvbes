# Registre d'applicabilité réglementaire

## Règle de décision

Ce registre documente une analyse technique initiale. Toute conclusion d'applicabilité,
d'exemption ou de conformité nécessite un legal sign-off daté. Le responsable Compliance
réévalue les décisions lors d'un changement de pays, de marché, de taille d'entreprise,
de modèle de distribution ou de catégorie de service.

## RGPD et CNIL

- Statut : applicable.
- Rôles possibles : responsable de traitement pour Account, sécurité et facturation;
  sous-traitant pour les contenus hébergés au nom des workspaces.
- Preuves : registre Article 30, DPA, sous-traitants, procédures de droits, rétention,
  violations et DPIA.
- Owner : DPO / Compliance.
- Revue : trimestrielle et avant tout nouveau traitement.

## PCI DSS 4.0.1

- Statut : périmètre à minimiser et éligibilité SAQ A à confirmer annuellement.
- Architecture : Checkout et Customer Portal hébergés par Stripe; aucun PAN, CVV ou
  formulaire carte traité par nvbes.
- Preuves : AOC/attestation Stripe, diagramme de flux, inventaire des pages paiement,
  scans requis et questionnaire annuel applicable.
- Owner : Security et Billing.

## NIS2

- Statut : applicability review.
- Décision requise : qualifier les services fournis, la taille de l'entité, les marchés
  desservis et la transposition nationale applicable.
- Mesures conservatoires : gouvernance du risque, gestion d'incident, sécurité de la
  chaîne d'approvisionnement, continuité, contrôle d'accès et notification.
- Owner : Direction, Legal et Security.
- Échéance : avant tout contrat affirmant une readiness NIS2.

## Cyber Resilience Act

- Statut : applicability review.
- Décision requise : déterminer si un SDK, agent, logiciel distribué ou service de
  traitement distant constitue un produit avec éléments numériques dans le périmètre.
- Mesures conservatoires : secure development lifecycle, SBOM, traitement coordonné des
  vulnérabilités, mises à jour de sécurité, documentation et obligations de signalement.
- Owner : Product Security et Legal.
- Échéance : avant distribution commerciale d'un produit potentiellement couvert.

## ISO/IEC 27001 et SOC 2

- Statut : cibles d'assurance, non acquises.
- ISO requiert un SMSI opéré, une analyse de risques, une Statement of Applicability,
  des audits internes, une revue de direction et un audit de certification indépendant.
- SOC 2 Type II requiert une description du système, des contrôles définis et des preuves
  d'efficacité sur une période d'observation, évaluées par un auditeur indépendant.
- Toute revendication publique reste interdite avant réception de l'artefact externe.
