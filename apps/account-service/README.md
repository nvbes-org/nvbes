# Account service

Runtime V1 propriétaire des profils, préférences, équipes, memberships et du
cycle de vie des données Account. Il accepte exclusivement les access tokens
RS256 émis par Identity pour l'audience Account.

Les exports sont construits par le job `process-privacy-jobs` et expirent après
24 heures. Une fermeture dispose d'un délai d'annulation de sept jours avant
redaction des données Account. Les autres domaines restent responsables de
leurs propres fragments et suppressions.

Le runtime ne fournit aucune inscription, credential, abonnement ou capacité
Enterprise.
