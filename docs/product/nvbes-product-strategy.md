# Direction produit nvbes V1

## Statut et autorité

Ce document est la source de vérité produit pour la V1 actuelle. En cas de
contradiction, il prévaut sur les anciens PRD, blueprints, plans d'exécution et
documents marketing. Les ADR conservent leur valeur historique, mais ne peuvent
pas élargir le périmètre produit actif.

La conception détaillée du socle et de son exploitation est décrite dans
[Socle nvbes V1 piloté par FinOps et Platform Operations](../superpowers/specs/2026-08-25-nvbes-v1-finops-platform-operations-design.md).

## Décision

La V1 finale livre uniquement le socle nvbes réutilisable. Elle ne livre pas
Cloud, Drive ni un autre produit utilisateur final. Le premier produit nvbes
sera sélectionné après validation du socle ; Cloud n'est pas supposé être ce
premier produit.

Le socle mutualise actuellement cinq services métier :

1. Identity ;
2. Account ;
3. Billing ;
4. Email ;
5. Trust/Risk.

Platform Operations complète ces services avec les capacités minimales
d'administration, de support, de sécurité, de traitement des abus et de suivi
FinOps. D'autres capacités ne sont mutualisées que lorsqu'un besoin transversal
réel est démontré avant la connexion du premier produit.

## Public cible

La cible V1 est B2C, tout public. Le modèle permet également à une personne de
créer ou rejoindre une équipe, de la petite collaboration jusqu'à une grande
équipe.

La V1 n'est pas une offre B2B ou Enterprise : elle ne promet ni contrat
entreprise, ni SLA client, ni SSO/SCIM, ni certification, ni programme de
conformité avancé. Les frontières d'identité, de compte, d'équipe,
d'autorisation et d'audit doivent permettre ces évolutions futures sans les
implémenter prématurément.

Les obligations légales applicables à un service B2C restent obligatoires. Les
documents RGPD, sécurité et légaux constituent un socle de protection, pas une
autorisation d'ouvrir un chantier de conformité Enterprise hors budget.

## FinOps comme contrainte de conception

Le coût récurrent total du projet est limité à **30 EUR TTC par mois**, domaines,
infrastructure et fournisseurs compris. La cible opérationnelle est 20 EUR TTC,
avec 10 EUR de réserve.

Toute évolution doit :

- déclarer son coût fixe et son pire coût variable autorisé ;
- rester compatible avec la descente à zéro lorsque le composant est inactif ;
- attribuer ses coûts à un domaine et à une unité de consommation ;
- prévoir quotas, rétention et mode économique dégradé ;
- passer le gate FinOps avant déploiement ;
- être refusée ou différée si le plafond global ne peut pas être démontré.

Les factures et estimations TTC vérifiées manuellement restent l'autorité tant
qu'aucune collecte automatique sûre, exacte et gratuite ne répond au besoin.

## Administration et modération

Un seul opérateur administre actuellement nvbes. Par défaut, les demandes de
support, de sécurité, d'abus, de facturation, de recours et de modération sont
traitées manuellement.

La procédure applicable est le
[runbook Platform Operations](../operations/platform-operations-manual-runbook.md).

Une automatisation est autorisée uniquement si elle remplit toutes les
conditions suivantes :

- elle répond aux exigences fonctionnelles et de sécurité ;
- elle maintient le coût récurrent total sous 30 EUR TTC ;
- elle réduit un volume manuel mesuré, protège une propriété critique ou
  empêche un dépassement budgétaire ;
- elle est observable, réversible et remplaçable ;
- elle ne prend pas seule une décision irréversible ou ambiguë.

Une solution maison optimisée peut être retenue lorsque son coût total, sa
sécurité, sa maintenance et sa remplaçabilité sont meilleurs que les solutions
disponibles. Le temps de développement et d'exploitation reste comptabilisé.

## Qualité et dette technique

Aucune dette technique ou structurelle connue n'est acceptée dans le périmètre
V1 livré. Les rustines, doubles sources de vérité et garanties fictives sont des
NO-GO.

Après la V1, toute dette acceptée doit avoir un propriétaire, une cause, un
impact mesuré, une date de revue et une condition de résolution. La dette est
gérée méthodiquement et proactivement ; elle ne devient pas un backlog sans fin.

## Mise en production progressive

La production est ouverte par étapes réversibles : primitives et FinOps, Email,
Identity et Account en accès interne, Billing en test, Trust/Risk en shadow
mode, Platform Operations, parcours intégré, comptes invités, puis premier
produit.

Les inscriptions publiques et paiements réels restent fermés tant que les
coûts, restaurations, incidents, abus et parcours critiques ne sont pas mesurés
et validés.

## Périmètre V1

Inclus :

- services Identity, Account, Billing, Email et Trust/Risk ;
- équipes comme primitive Account, sans gouvernance Enterprise ;
- contrats, audits, idempotence, outbox, observabilité et contrôles FinOps
  réutilisables ;
- Platform Operations minimal pour un opérateur solo ;
- procédures manuelles sûres et auditables ;
- déploiement progressif et restauration démontrée.

Exclus :

- Cloud, Drive ou tout autre produit final ;
- marketing, pricing ou beta d'un produit non sélectionné ;
- B2B, Enterprise, SLA, SSO/SCIM et conformité avancée ;
- support ou modération entièrement automatisés ;
- haute disponibilité permanente, multi-région ou hyperscale ;
- nouvelle ressource payante sans preuve de compatibilité avec le budget.

## Gate avant le premier produit

Le choix et la connexion du premier produit font l'objet d'une décision séparée.
Le socle doit auparavant fonctionner de bout en bout sans dépendre d'un produit
final, rester sous les gates de 20/30 EUR TTC, démontrer sa restauration et
permettre à l'opérateur solo de traiter les demandes réelles en sécurité.
