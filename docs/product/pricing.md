# Pricing

## Strategie

nvbes Drive ne doit pas etre positionne comme du stockage brut moins cher.

Les exigences FinOps, Stripe, TVA et unit economics sont definies dans [FinOps et Billing](finops-billing.md).

La revue critique des couts, des quotas et de la pertinence marketing est documentee dans [Audit FinOps: trajectoire production à coût fixe quasi nul](finops-zero-cost-production-audit.md).

Le produit vend:

- Hebergement europeen.
- Securite.
- Controle d'equipe.
- Partage externe clair.
- Administration simple.
- Facturation previsible.

## Plans V1

Role de conversion:

- Solo Pro: rassurer les independants et permettre une adoption simple.
- Team: plan recommande pour l'ICP agences/studios de 2 a 10 personnes.
- Team Plus: repondre aux equipes avec plus de stockage, plus de membres et besoin de controle accru.

API publique V1:

- Solo Pro: API basique, 1 cle, limites faibles.
- Team: API V1 complete, 5 cles, audit API.
- Team Plus: limites plus hautes, 20 cles, expiration des cles.

### Solo Pro

```text
Prix: 15 EUR/mois
Utilisateurs: 1
Stockage: 1 To
Retention: 90 jours
```

Inclus:

- Stockage securise.
- Dossiers.
- Liens de partage securises.
- Expiration des liens.
- Corbeille.
- Chiffrement au repos.
- API publique basique.

Message de valeur:

```text
Pour travailler seul avec des fichiers clients dans un espace europeen controle.
```

### Team

```text
Prix: 39 EUR/mois
Utilisateurs: 3 inclus
Stockage: 2 To
Retention: 180 jours
```

Inclus:

- Tout Solo Pro.
- Workspace equipe.
- Roles.
- Invitations membres.
- Audit basique.
- Vue des liens actifs.
- Policies de partage equipe.
- API publique V1.
- Audit des actions API.

Message de valeur:

```text
Pour une petite equipe qui partage regulierement des fichiers avec ses clients.
```

### Team Plus

```text
Prix: 79 EUR/mois
Utilisateurs: 8 inclus
Stockage: 5 To
Retention: 180 jours
```

Inclus:

- Tout Team.
- Plus de stockage inclus.
- Plus d'utilisateurs inclus.
- Support prioritaire plus tard.
- Retention avancee plus tard.
- Limites API plus hautes.
- Integrations machine via service accounts OAuth; API keys legacy uniquement pendant migration.

Message de valeur:

```text
Pour les equipes qui ont besoin de plus d'espace, plus de membres et plus de controle.
```

## Extension Pay-As-You-Use

- Stockage additionnel.
- Utilisateurs equipe additionnels.
- Retention etendue plus tard.
- Bande passante avancee plus tard.
- Credits OCR et IA plus tard.
- Archivage long terme plus tard.

Meters V1:

- `storage_gb_month`: stockage additionnel facture au Go-mois.
- `team_seat_month`: utilisateurs additionnels factures par mois avec prorata.
- `egress_gb`: bande passante sortante suivie en V1, facturee plus tard apres validation du modele.

Regles V1:

- Usage mesure en interne avant envoi a Stripe.
- Snapshot quotidien pour usage facturable.
- Alertes client a 80% et 100% des quotas.
- Blocage des nouveaux uploads si quota critique depasse apres grace period.
- Estimation de facture visible avant facturation.
- Aucun usage additionnel facture sans affichage produit prealable.

## Essai

Essai recommande au lancement:

```text
14 jours
Sans carte bancaire
5 Go pendant l'essai
Pas de choix de plan obligatoire avant la premiere activation
Upgrade obligatoire pour debloquer les limites completes du plan
```

Protections anti-abus:

- Verification email obligatoire.
- Limite de workspaces par email, domaine et IP.
- Friction ou blocage sur domaines email jetables.
- Limite d'uploads et d'egress pendant trial.
- Monitoring des trials couteux.
- Suppression ou archivage des trials expires selon politique de retention.

## Principe Billing

L'usage facture doit etre visible avant d'etre facture.

Stripe est le provider billing cible pour la V1.

La page facturation doit afficher:

- Plan actuel.
- Stockage inclus.
- Stockage utilise.
- Utilisateurs inclus.
- Utilisateurs actifs.
- Prochaine facture estimee.
- Usage additionnel.

## Exigences Billing Securise

- Les webhooks billing doivent verifier la signature du provider.
- Les evenements billing doivent etre idempotents.
- Les changements de plan, quota ou statut subscription doivent etre audites.
- Les erreurs de synchronisation billing ne doivent pas supprimer ou exposer les donnees client.

## TVA et Facturation EU

- Les prix publics doivent preciser HT ou TTC.
- Le billing doit distinguer B2B et B2C.
- Le numero de TVA intracommunautaire doit etre collecte pour les clients B2B EU.
- Le reverse charge doit etre supporte quand applicable.
- Stripe Tax ou mecanisme equivalent doit etre configure avant lancement payant.
- Les factures doivent etre conservees selon les obligations comptables applicables.
