# Roadmap

## V1

Objectif: livrer Identity puis nvbes Cloud comme fondation B2B pour tous: utilisable en solo via workspace personnel, pret pour workspaces de groupe, sans activer Business en production initiale.

Fondations produit:

- Auth.
- Fondations identity enterprise: `user global`, `tenant`, `organization` optionnelle, memberships multi-scope.
- Creation automatique d'un workspace personnel dedie pour chaque nouveau user.
- Separation stricte entre workspace personnel et workspaces de groupe.
- Sessions hybrides Redis: JWT courts + introspection/decision centrale.
- MFA, step-up et scaffold WebAuthn.
- Workspace.
- Upload et download.
- Dossiers.
- Recherche simple.
- Renommer, deplacer, corbeille, restaurer, supprimer.
- Liens de partage.
- Expiration et revocation des liens.
- Membres et roles.
- Quotas.
- Billing personnel puis groupe, avec plans groupe prepares mais lancement progressif.
- Audit basique.
- Export et suppression RGPD.
- API publique V1 limitee aux integrations fichiers.
- API keys legacy, scopes, rate limits et OpenAPI.
- Integrations machine via Identity `service accounts` et OAuth clients.
- Scaffold federation/SCIM reserve des la V1 meme si non expose.

Fondations design/UX:

- Direction artistique B2B sobre et dense.
- nvbes Design System base sur `shadcn/ui`, `Radix UI`, `Tailwind CSS` et `lucide-react`.
- Taxonomie UI francaise.
- UX map V1.
- Wireframes V1.
- Inventaire composants.
- Copy deck V1.
- Checklist accessibilite WCAG AA.
- Prototype clickable.
- Test utilisabilite avec 3 a 5 utilisateurs cibles.
- Strategie UI testing.
- Protocole UX testing.
- Ecran API pour gerer les integrations machine, scopes et documentation.

Fondations marketing/growth:

- Positionnement V1: B2B pour tous, avec usage solo professionnel comme entree de gamme et groupes prepares.
- ICP initial: independants professionnels, agences, studios et petites structures manipulant des fichiers clients.
- Messaging V1 et landing page.
- Pricing page orientee conversion.
- Funnel de conversion documente.
- Event taxonomy product analytics.
- Dashboards activation, conversion, retention, revenue et FinOps.
- Consentement analytics RGPD.
- Experiment log.
- Cadence de revue KPI hebdomadaire pendant beta.
- Go-to-market checklist avant lancement payant.

Fondations business/FinOps:

- Plateforme billing interne canonique pour catalogue, subscriptions, invoices, payments, ledger, usage, entitlements et reconciliation.
- Stripe comme PSP principal V1; Mollie comme PSP secondaire active par routing provider-neutral.
- Mappings provider-neutral pour Product/Price/Customer/Subscription/Invoice/Tax, avec compatibilite Stripe pendant la transition.
- Ledger interne append-only pour usages, invoices, paiements, refunds, credits, write-offs et adjustments.
- Meters V1: `storage_gb_month`, `team_seat_month`, `egress_gb` en suivi.
- TVA EU, B2B/B2C et factures conformes.
- Dashboard marge/couts par workspace et par plan.
- Budgets cloud par environnement.
- Protections anti-abus sur trials.
- Validation marge brute avant lancement payant.
- Validation externe obligatoire avant activation multi-pays payante: TVA, e-invoicing, revenue recognition audit et retention des pieces comptables.

Sequence production initiale:

- Identity seul sur Scaleway, optimise France et cout minimal.
- Cloud/Drive full feature sur la meme architecture minimum cost.
- Developer Hub avec APIs publiques.
- DevOps production elargie pour extension EU.
- Ingestion data, transformation, vectorisation et entrainement LLM.
- Docs, Sheets, Slides puis Forms.
- Plans Team et Workspace.
- Business prepare mais non active en production.

Fondations infra/DevOps:

- Infrastructure as Code pour les composants critiques.
- Environnements development, staging et production separes.
- CI/CD avec tests, scans, staging, approval production et rollback documente.
- Strategie de test V1 et matrice de regression.
- Migrations PostgreSQL versionnees.
- Backups chiffres avec RPO/RTO et test de restauration avant lancement.
- Observabilite V1: logs structures, metriques, dashboards et alertes critiques.
- Workers monitorables avec retries, backoff et dead-letter.
- Object Storage prive avec policies, CORS limite et lifecycle rules.
- Gestion des secrets par environnement.
- Runbooks incidents minimum pour API, PostgreSQL, Object Storage, billing, jobs RGPD et bucket public.

## V1.5

Objectif: ameliorer l'usage quotidien, les workspaces personnels et le controle equipe.

- Preview de fichiers.
- Recherche amelioree.
- Ameliorations de la vue des liens partages.
- Notifications email.
- Reporting d'usage plus precis.
- Estimations billing plus claires.
- Raffinements de permissions.
- Caps de depense configurables.
- Alertes cout client avancees.
- Optimisation des marges par plan.
- Webhooks publics signes.
- SDK ou exemples d'integration avances.
- Federation entrante et provisioning enterprise en increment cible si le scaffold V1 est stable.

## V2

Objectif: servir des workspaces plus sensibles a la securite sans lancer Business trop tot.

- Versioning.
- Politiques de retention avancees.
- SSO/SAML sur le scaffold federation deja pose.
- SCIM sur le scaffold provisioning deja pose.
- Audit logs avances.
- Revue des sessions admin.
- Permissions plus granulaires.
- Preparation Business: contrats, policies, audit avance, SSO/SCIM et SLA, sans activation commerciale tant que le support et la marge ne sont pas valides.

## V3

Objectif: augmenter la valeur plateforme.

- Sync desktop.
- Apps mobiles natives.
- OCR.
- Recherche IA.
- Integrations.
- Archivage long terme.
- Reutilisation du module storage dans les autres produits nvbes.
- Docs, Sheets, Slides et Forms si Cloud est stable.
- Photo Editor et Video Editor comme produits separes apres validation cout/media.

## Explorations Produits

Ces idees ne sont pas dans le scope Drive V1/V2. Elles documentent des pistes futures a revisiter avec une decision produit separee.

- Strategie produit nvbes: positionnement B2B pour tous, workspaces personnels, offres personnelles, add-ons, sequence production et politique donnees. Voir [Strategie Produit nvbes](product/nvbes-product-strategy.md).
- Privacy-preserving KYC verification: verifier document et visage sans stocker les artefacts bruts, puis emettre un credential reutilisable avec consentement. Voir [Privacy-Preserving KYC Verification](product/privacy-preserving-kyc.md).
- Plateforme globale zero-stack: architecture cible multi-cloud, cellulaire et hyperscale pour les futurs produits nvbes. Voir [Plan Plateforme Globale - Stack Zero](blueprint/nvbes-global-platform-zero-stack.plan.md).
- Trajectoire low budget vers plateforme globale: plan starter economique avec chemin d'upgrade progressif vers l'architecture zero-stack. Voir [Plan Starter Low Budget vers Plateforme Globale](blueprint/nvbes-low-budget-to-global-platform.plan.md).
- Hierarchie monorepo OSS/Cloud: separation entre source privee, export public OSS, docs publiques et operations Cloud. Voir [Plan Monorepo nvbes OSS et nvbes Cloud](blueprint/nvbes-oss-cloud-monorepo.plan.md).
- Structuration complete Big Bang zero dette: migration complete vers une plateforme nvbes reconstruite from scratch. Voir [Plan Structuration Complete Big Bang Zero Dette](blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md).
- Plateforme billing interne multi-provider: internaliser catalogue, subscriptions, invoices, payments, ledger, tax evidence, reconciliation, routing Stripe/Mollie et reporting finance sans devenir PSP. Voir [Plan Plateforme Billing Interne Multi-Provider](blueprint/nvbes-internal-billing-platform.plan.md).
