# Roadmap

## V1

Objectif: livrer un drive europeen securise vendable pour petites equipes.

Fondations produit:

- Auth.
- Fondations identity enterprise: `user global`, `tenant`, `organization` optionnelle, memberships multi-scope.
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
- Billing.
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

- ICP prioritaire V1: agences et studios de 2 a 10 personnes.
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

- Stripe comme provider billing V1.
- Mapping Stripe Product/Price/Customer/Subscription/Invoice/Tax.
- Ledger interne des usages facturables.
- Meters V1: `storage_gb_month`, `team_seat_month`, `egress_gb` en suivi.
- TVA EU, B2B/B2C et factures conformes.
- Dashboard marge/couts par workspace et par plan.
- Budgets cloud par environnement.
- Protections anti-abus sur trials.
- Validation marge brute avant lancement payant.

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

Objectif: ameliorer l'usage quotidien et le controle equipe.

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

Objectif: servir des equipes plus sensibles a la securite.

- Versioning.
- Politiques de retention avancees.
- SSO/SAML sur le scaffold federation deja pose.
- SCIM sur le scaffold provisioning deja pose.
- Audit logs avances.
- Revue des sessions admin.
- Permissions plus granulaires.
- Plan Business.

## V3

Objectif: augmenter la valeur plateforme.

- Sync desktop.
- Apps mobiles natives.
- OCR.
- Recherche IA.
- Integrations.
- Archivage long terme.
- Reutilisation du module storage dans les autres produits nvbes.

## Explorations Produits

Ces idees ne sont pas dans le scope Drive V1/V2. Elles documentent des pistes futures a revisiter avec une decision produit separee.

- Privacy-preserving KYC verification: verifier document et visage sans stocker les artefacts bruts, puis emettre un credential reutilisable avec consentement. Voir [Privacy-Preserving KYC Verification](product/privacy-preserving-kyc.md).
