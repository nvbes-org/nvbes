# Déconnexion initiée par un client OIDC

## Contrat cible et état

Le parcours existant `/logout` confirme une révocation de session Identity,
sans retour RP. La validation cryptographique et la politique de retour sont
maintenant disponibles dans `identity.tokens.logout.rs` et
`identity.oauth.logout_request.rs`. Elles ne sont pas encore montées sur un
endpoint HTTP et ne confèrent aucune capacité de révocation. La découverte
n'annonce donc pas encore `end_session_endpoint`.

Le contrat cible suit [OpenID Connect RP-Initiated Logout 1.0](https://openid.net/specs/openid-connect-rpinitiated-1_0.html).
Le transport devra accepter GET et POST form avec le même décodage conservant
les doublons. Toute demande doit passer par une confirmation explicite sur
Identity. Aucun GET de préparation ne révoque de session. La décision finale
doit être liée à la session présente et à la preuve CSRF du navigateur, vérifiée
dans la transaction qui effectue la révocation.

## Validation livrée

`verify_logout_hint` vérifie signature RS256, type JWT, issuer exact et clé
encore admise. Les en-têtes de chargement distant sont refusés. Un ID Token
expiré peut identifier une session ; il ne peut pas authentifier une requête
API. La vérification conserve les contraintes temporelles de l'émetteur,
les identifiants UUID et les méthodes d'authentification reconnues. Le résultat
contient uniquement client, principal, session et date d'émission.

`LogoutRequest::from_fields` vérifie le client actuellement enregistré et sa
cohérence avec l'audience signée. Le retour exige un ID Token valide et une
égalité exacte avec une adresse `post_logout_redirect_uris` enregistrée. Aucun
retour n'est déduit du client, du Referer ou de l'origine de la requête. Le state
est borné et encodé comme valeur de query ; il ne peut modifier la destination.
Les paramètres dupliqués, inconnus et les entrées trop volumineuses sont refusés.
Les préférences de langue et logout_hint ne modifient aucune autorisation.

Ce profil n'accepte pas de retour fondé uniquement sur un client_id public :
aucune autre preuve de légitimité de la session RP n'est encore implémentée.
Une demande sans indice peut préparer la déconnexion locale après confirmation,
mais ne reçoit pas de destination externe.

## Raccordement restant

- Lier le hint au navigateur et à une session actuelle ou récente en base ;
  rejeter les associations incohérentes et couvrir le changement de compte.
- Monter GET/POST de préparation avec limites de taille, quotas, réponses
  privées et absence de redirection externe en cas d'erreur.
- Transmettre le contexte à l'UI sans conserver l'ID Token dans les journaux,
  le stockage persistant ou l'historique de navigation.
- Revalider le contexte et la configuration client lors de la confirmation,
  puis révoquer atomiquement et retourner uniquement la destination autorisée.
- Raccorder le SDK et Account, vérifier state au retour et tester annulation,
  double soumission, expiration, panne et changement de session en navigateur.
- Ajouter la découverte seulement après ces preuves ; les notifications
  back-channel gardent leur chantier distinct dans le lot C.

## Preuves

Huit tests unitaires supplémentaires couvrent les jetons expirés, les clés
retirées, le type de jeton, la signature altérée, les en-têtes hostiles, les
claims incohérents, les paramètres dupliqués, l'encodage de state et les retours
non enregistrés. Ils exécutent de vraies signatures RSA. Ils ne constituent pas
une preuve de logout HTTP, de mutation en base ou de parcours navigateur.
