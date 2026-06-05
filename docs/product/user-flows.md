# Parcours Utilisateur

Les details d'interface, d'etats critiques, de taxonomie UI et de copywriting sont definis dans [Design, UX et Design System](design-ux.md).

## Flow d'Activation Critique

```text
Creer un compte
Creer un workspace
Uploader un premier fichier
Creer un lien de partage securise
Inviter un membre
Passer au plan payant
```

## Flow Signup

Champs requis:

- Email.
- Mot de passe.
- Prenom et nom si fournis.
- Nom d'utilisateur.
- Date de naissance si fournie.
- Region supportee detectee automatiquement ou selectionnee.
- Nom du workspace.

Chaque utilisateur demarre dans un contexte workspace.

## Flow Essai

1. L'utilisateur cree un compte.
2. L'utilisateur cree un workspace.
3. Le workspace demarre un essai de 14 jours.
4. Le workspace d'essai est limite a 5 Go.
5. L'utilisateur choisit un plan payant au moment de l'upgrade.
6. L'utilisateur upgrade pour debloquer les limites completes du plan.

## Premiere Session

Apres inscription, l'utilisateur arrive directement dans la vue Drive.

Ordre UX recommande:

1. Creer compte.
2. Verifier email si necessaire.
3. Nommer l'espace.
4. Arriver directement dans Fichiers.
5. Importer un premier fichier.
6. Proposer de creer un lien securise.
7. Proposer d'inviter un membre.
8. Afficher trial et quota sans bloquer l'activation.

Actions d'etat vide:

- Uploader un fichier.
- Creer un dossier.
- Inviter un membre.

## Flow Upload

1. L'utilisateur glisse des fichiers dans la vue Drive.
2. Le frontend affiche la progression.
3. Le fichier apparait dans le dossier courant.
4. Le quota est mis a jour.
5. Un audit event est cree.

## Flow Partage

1. L'utilisateur selectionne un fichier ou dossier.
2. L'utilisateur cree un lien de partage.
3. L'utilisateur choisit l'expiration: 1 jour, 7 jours ou 30 jours.
4. L'utilisateur choisit la permission de telechargement.
5. L'utilisateur copie le lien.
6. L'utilisateur peut revoquer le lien plus tard.

Contraintes securite:

- Aucun lien public sans expiration en V1.
- Expiration par defaut: 7 jours.
- Duree maximale definie par plan.
- Acces publics journalises.
- Rate limiting dedie sur les liens publics.
- Les fichiers partages publiquement passent par la politique anti-malware.
- Pas de preview publique riche en V1.

## Flow Invitation Membre

1. Owner ou admin saisit un email.
2. Owner ou admin choisit un role.
3. L'invitation email est envoyee.
4. L'invite rejoint le workspace.
5. Un audit event est cree.

## Flow Billing

La page billing affiche:

1. Plan actuel.
2. Stockage utilise.
3. Utilisateurs inclus/utilises.
4. Prochaine facture estimee.
5. Usage additionnel.
6. Alertes quota.
7. Moyen de paiement.
8. Factures.
9. Actions upgrade et downgrade.

## Flow Privacy

Parametres compte:

- Exporter mes donnees.
- Supprimer mon compte.

Parametres workspace:

- Exporter les donnees du workspace.
- Supprimer le workspace.
