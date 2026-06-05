# Audit legal UE/FR - 2026-05-05

## Statut

Ce document est un audit documentaire de conformite, pas un avis juridique.

Perimetre relu:

- README
- docs/compliance/security-privacy-baseline.md
- docs/product/marketing-growth.md
- docs/product/pricing.md
- docs/product/finops-billing.md
- docs/api/public-api-v1.md
- docs/api/v1-contracts.md
- docs/architecture/infrastructure-devops.md

## Synthese

Base documentaire globalement solide sur la securite, la privacy by design, la retention, l'audit et la facturation.

Le principal risque n'est pas l'absence d'intention de conformite, mais l'absence de documents juridiques opposables et de procedures prouvables pour soutenir les promesses publiques "europeen", "securise" et "RGPD by design".

## Points forts

- Les exigences de securite, retention, audit et gestion d'incident sont deja posees.
- Les contraintes analytics RGPD sont formulees de maniere prudente.
- Les sujets TVA, facturation, HT/TTC et reverse charge sont identifies.
- Les limites du scope V1 sont explicites, ce qui reduit une partie du risque de sur-promesse.

## Ecarts prioritaires

### P0 - Absence des documents juridiques externes obligatoires ou quasi obligatoires

Constat:

- Les documents a maintenir sont listes, mais ils ne sont pas presents dans le repo: politique de confidentialite, conditions d'utilisation, DPA, liste des sous-traitants, politique de retention, procedure RGPD ([security-privacy-baseline.md](./security-privacy-baseline.md)).

Impact:

- Impossible de soutenir proprement les obligations d'information RGPD envers les personnes.
- Faible defensabilite contractuelle face aux clients B2B.
- Risque commercial immediat si le site ou le produit ouvrent sans pages juridiques.

Actions minimales:

- Rediger une politique de confidentialite conforme RGPD.
- Rediger des conditions d'utilisation/CGU.
- Rediger un DPA client conforme a l'article 28 RGPD.
- Publier une liste de sous-traitants et une politique de retention.
- Ajouter une procedure de traitement des droits RGPD et une procedure de violation de donnees.

### P0 - Absence de cadre documentaire RGPD "accountability" prouvable

Constat:

- Les docs parlent d'export, suppression, retention, backups, incidents et audit, mais aucun registre des traitements, registre des violations, decision log AIPD, ni matrice base legale/finalite/categorie de donnees n'est present.

Impact:

- Conformite difficile a demontrer en controle.
- Risque fort sur les demandes clients enterprise ou security review.

Actions minimales:

- Creer un registre article 30 pour nvbes en tant que responsable de traitement.
- Creer un registre "sous-traitant" des categories d'activites si nvbes traite les donnees clients pour leur compte.
- Creer un registre des violations de donnees.
- Creer une matrice finalites -> bases legales -> categories -> retention -> destinataires -> transferts.
- Documenter explicitement si une AIPD est requise ou non pour V1, et pourquoi.

### P0 - Promesses marketing "EU-first / heberge en Europe / drive europeen securise" insuffisamment bornees

Constat:

- Les promesses publiques sont fortes dans [README.md](../README.md), [marketing-growth.md](../product/marketing-growth.md) et [pricing.md](../product/pricing.md).
- Les docs ne documentent pas encore la carte exacte des sous-traitants, des acces support/admin, ni les transferts hors UE eventuels.

Impact:

- Risque de promesse trompeuse si un sous-traitant, un outil support, un outil observabilite ou un provider email expose des donnees hors UE/EEE.
- Risque eleve si les claims "europeen" ou "heberge en Europe" ne distinguent pas residence des donnees, administration, support et transferts.

Actions minimales:

- Remplacer les claims absolus par des formulations bornees tant que la cartographie n'est pas faite.
- Documenter pour chaque sous-traitant: role, donnees traitees, lieu de traitement, pays, garanties de transfert.
- Ajouter une note explicite sur les transferts internationaux et les garanties associees si applicables.

### P1 - Cookies/analytics: base RGPD correcte mais procedure CNIL incomplete

Constat:

- Les docs exigent le consentement pour les analytics marketing non essentiels.
- Il manque la preuve operationnelle: CMP, journal de consentement, symetrie accepter/refuser, retrait simple, qualification des cookies exemptes ou non.

Impact:

- Risque CNIL eleve des mise en ligne de la landing/pricing si des traceurs non exemptes partent avant consentement.

Actions minimales:

- Documenter la liste des traceurs par finalite.
- Distinguer mesure d'audience exemptee vs soumise a consentement.
- Documenter la banniere, la preuve de consentement, et le mecanisme de retrait.
- Interdire tout tracking non essentiel avant consentement.

### P1 - Obligations hebergeur/plateforme insuffisamment formalisees

Constat:

- Le produit heberge des fichiers clients et permet des liens de partage publics.
- Les docs parlent de contenus abusifs et d'anti-malware, mais pas de mecanisme formel de notice-and-action, de canal de signalement, ni de workflow LCEN/DSA.

Impact:

- Risque juridique sur la gestion des contenus illicites si le service ouvre a des tiers ou expose des liens publics.

Actions minimales:

- Creer une procedure de signalement de contenu illicite.
- Definir les delais, journaux, escalades et conditions de retrait.
- Ajouter les mentions legales site/hebergeur.
- Evaluer si les obligations DSA de reporting s'appliquent en fonction de la taille et de la qualification du service.

### P1 - Billing/fiscalite: principes presents, mais support juridique incomplet

Constat:

- Les docs identifient HT/TTC, B2B/B2C, TVA intra-UE, reverse charge et conservation des factures.
- Il manque les conditions contractuelles de paiement, renouvellement, suspension, remboursement, credits, annulation et resiliation.

Impact:

- Risque de litige client et de mise en cause des factures/emails billing si la documentation commerciale reste trop succincte.

Actions minimales:

- Ajouter une politique commerciale contractuelle: renouvellement, echeance, incident de paiement, suspension, suppression apres resiliation, credits/refunds.
- Verifier les mentions facture et les regles B2B/B2C par pays avant lancement payant.
- Verifier si le produit restera strictement B2B. Sinon, ajouter l'analyse droit de la consommation.

## Commentaires par document

### README

- Les claims "EU-first", "residence des donnees en Europe" et "drive cloud europeen securise" sont commerciaux et devront etre synchronises avec les sous-traitants reels et la politique de confidentialite.

### security-privacy-baseline.md

- Bon niveau d'exigence interne.
- Le document identifie lui-meme les artefacts manquants; il confirme donc les priorites P0.
- Ajouter explicitement: registre des violations, matrice de bases legales, decision AIPD, procedure article 33/34 RGPD.

### marketing-growth.md

- Les preuves marketing sont ambitieuses.
- Tant que la liste des sous-traitants et les transferts ne sont pas figes, eviter les formulations absolues sur la souverainete.
- La section analytics RGPD doit etre reliee a une spec CMP/cookies distincte.

### pricing.md et finops-billing.md

- Bonne anticipation de la TVA et du reverse charge.
- Il manque le support contractuel externe: CGU/CGV, politique de resiliation, politique de remboursement, mentions facture detaillees.
- Si Solo Pro peut etre achete par un particulier, il faut ouvrir un chantier droit de la consommation FR/UE distinct.

### architecture/infrastructure-devops.md

- Bonne base operationnelle pour la gestion d'incident.
- Le document doit mentionner explicitement l'objectif de notification CNIL sous 72h quand une violation de donnees presente un risque pour les personnes.

## Plan de remediation minimum avant lancement public

1. Creer les pages juridiques externes: Confidentialite, CGU/CGV, DPA, Sous-traitants, Retention.
2. Creer les preuves internes: registre article 30, registre des violations, decision AIPD, matrice de bases legales.
3. Figer la cartographie sous-traitants/transferts et recalibrer les claims marketing.
4. Documenter CMP/cookies/consent proof avant toute landing trackee.
5. Documenter la procedure contenu illicite/abus pour les liens publics.
6. Faire valider la partie TVA/facturation par conseil fiscal/comptable avant lancement payant UE.

## Sources juridiques et administratives verifiees

- RGPD, Reglement (UE) 2016/679: articles 13, 28, 30, 32, 33, 35.
  https://eur-lex.europa.eu/eli/reg/2016/679/oj/eng
- CNIL, "Le registre des activites de traitement".
  https://www.cnil.fr/fr/RGPD-le-registre-des-activites-de-traitement
- CNIL, "Clauses contractuelles types entre responsable de traitement et sous-traitant".
  https://www.cnil.fr/fr/clauses-contractuelles-types-entre-responsable-de-traitement-et-sous-traitant
- CNIL, "L'analyse d'impact relative a la protection des donnees (AIPD)".
  https://www.cnil.fr/fr/RGPD-analyse-impact-protection-des-donnees-aipd
- CNIL, "Securite : gerer les incidents et les violations".
  https://www.cnil.fr/fr/securite-gerer-les-incidents-et-les-violations
- CNIL, "Les regles a suivre pour les cookies".
  https://www.cnil.fr/fr/cookies-et-autres-traceurs/regles/cookies
- CNIL, "Cookies et traceurs : que dit la loi ?".
  https://www.cnil.fr/fr/cookies-et-autres-traceurs/que-dit-la-loi
- Reglement (UE) 2022/2065, Digital Services Act.
  https://eur-lex.europa.eu/eli/reg/2022/2065/oj
- Légifrance, loi n° 2004-575 du 21 juin 2004, article 6.
  https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006421546
- Service-Public, mentions obligatoires sur le site internet d'une societe.
  https://entreprendre.service-public.fr/vosdroits/F37351
- Service-Public, mentions obligatoires sur une facture.
  https://entreprendre.service-public.fr/vosdroits/F31808
- Ministere de l'Economie, CGV entre professionnels.
  https://www.economie.gouv.fr/entreprises/gerer-sa-comptabilite-et-ses-demarches/conditions-generales-de-vente-entre
