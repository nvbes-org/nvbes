# Privacy-Preserving KYC Verification

## Objectif Produit

Construire un produit de verification KYC reutilisable qui permet a un utilisateur de prouver son statut d'identite verifiee sur des sites tiers sans exposer ses documents personnels.

Le produit cible doit permettre:

- verification de carte d'identite, passeport ou permis;
- verification visage 3D avec liveness;
- absence de stockage des documents bruts;
- absence de stockage des scans visage bruts;
- emission d'une preuve cryptographique reutilisable;
- consentement explicite avant chaque partage avec un site tiers;
- divulgation minimale des informations personnelles;
- possibilite d'ancrage Web3 sans publier de donnees personnelles.

Nom de travail: `nvbes Verify`.

## Positionnement

`nvbes Verify` n'est pas un simple "KYC NFT".

Le produit est plutot:

- un service de verification d'identite;
- un issuer de credentials verifiables;
- une couche de consentement;
- un registre de statut et de revocation;
- optionnellement un systeme d'ancrage cryptographique Web3.

Le NFT ou token ne doit jamais devenir le conteneur principal des donnees KYC.

## Decisions Produit

- Les documents bruts ne sont pas stockes.
- Les photos, videos, scans visage et modeles 3D bruts ne sont pas stockes.
- Les donnees personnelles ne sont pas ecrites on-chain.
- Les preuves partagees avec les sites tiers sont minimales et consenties.
- Le visage 3D sert uniquement a produire un template biometrique protege.
- Le template biometrique protege reste une donnee sensible, meme s'il est non reversible.
- Les identifiants derives doivent etre specifiques au relying party pour eviter la correlation inter-sites.
- Le credential doit etre revocable et expire.
- Le systeme doit pouvoir fonctionner sans Web3.
- Web3 est optionnel et limite a l'ancrage, la preuve de statut ou la revocation.

## Non-Objectifs

- Stocker une copie longue duree des documents d'identite.
- Stocker une image, video ou mesh 3D du visage.
- Creer un NFT public contenant une identite legale.
- Mettre un hash direct de numero de document, date de naissance ou visage on-chain.
- Remplacer l'authentification nvbes existante par une wallet.
- Rendre le KYC permanent ou impossible a revoquer.
- Fournir une promesse de non-identifiabilite absolue pour la biometrie.

## Donnees Autorisees

Le systeme peut stocker:

- identifiant utilisateur nvbes pseudonyme;
- statut KYC;
- niveau d'assurance;
- pays du document si necessaire;
- type de document si necessaire;
- claims derives, par exemple `age_over_18`;
- date de verification;
- date d'expiration;
- identifiant de credential;
- hash ou commitment du credential;
- statut actif, expire ou revoque;
- consentements;
- journaux d'audit minimaux;
- template biometrique protege si indispensable.

Le systeme ne doit pas stocker:

- image du document;
- OCR complet du document;
- numero du document;
- date de naissance complete, sauf besoin legal explicite;
- adresse postale complete, sauf besoin legal explicite;
- selfie brut;
- video liveness;
- mesh 3D visage brut;
- face embedding brut;
- hash direct d'un attribut personnel a faible entropie.

## Modele de Verification

### Verification Document

Le flux document produit des assertions, pas un stockage documentaire.

Assertions possibles:

```json
{
  "document_verified": true,
  "document_type": "passport",
  "document_country": "FR",
  "document_not_expired": true,
  "age_over_18": true,
  "assurance_level": "high"
}
```

Les champs exacts doivent rester configurables selon le cas d'usage du site tiers.

### Verification Visage 3D

Le scan visage 3D sert a valider:

- presence humaine;
- liveness;
- correspondance document/personne;
- unicite relative si le produit en a besoin.

Le systeme ne stocke pas le scan.

Il peut stocker un template protege:

```text
protected_face_template = cancellable_transform(face_embedding_3d, user_secret, system_pepper)
```

Contraintes:

- le template ne doit pas permettre de reconstruire le visage;
- le template doit etre cancellable;
- une compromission doit permettre rotation ou invalidation;
- le template ne doit pas etre partage avec les sites tiers;
- les preuves derivees doivent etre specifiques par relying party.

Exemple de derivation:

```text
rp_face_commitment = derive(protected_face_template, relying_party_id, credential_nonce)
```

## Credential Reutilisable

Le bon objet principal est un credential verifiable, pas un NFT.

Formats candidats:

- W3C Verifiable Credential;
- SD-JWT VC;
- BBS+ selective disclosure credential;
- preuve zero-knowledge pour claims simples;
- JWT signe en MVP, si selective disclosure non disponible au depart.

Claims possibles:

```json
{
  "sub": "did:nvbes:user:opaque-id",
  "kyc_verified": true,
  "assurance_level": "high",
  "age_over_18": true,
  "document_country": "FR",
  "face_liveness": true,
  "verified_at": "2026-06-15T00:00:00Z",
  "expires_at": "2027-06-15T00:00:00Z",
  "issuer": "did:nvbes:verify"
}
```

Le credential complet ne doit pas etre partage par defaut.

Chaque site tiers demande uniquement les claims necessaires.

## Consentement

Le consentement est central dans le produit.

Flux:

1. Le site tiers demande une preuve.
2. nvbes affiche les claims demandes.
3. L'utilisateur accepte ou refuse.
4. nvbes emet une presentation limitee.
5. Le site tiers recoit uniquement les claims consentis.
6. Le consentement est journalise.
7. L'utilisateur peut consulter et revoquer les consentements.

Exemple de demande:

```json
{
  "relying_party_id": "exchange.example",
  "requested_claims": ["kyc_verified", "age_over_18", "document_country"],
  "purpose": "Regulatory onboarding",
  "ttl_minutes": 10
}
```

Exemple de presentation:

```json
{
  "kyc_verified": true,
  "age_over_18": true,
  "document_country": "FR",
  "proof": "signed-or-zk-proof",
  "expires_at": "2026-06-15T12:10:00Z"
}
```

## Role de Web3

Web3 est optionnel.

Utilisations acceptables:

- ancrage periodique de Merkle root;
- registre de revocation;
- publication des cles publiques d'issuer;
- preuve publique que le credential existait a une date donnee;
- token non transferable indiquant un statut minimal sans PII.

Ne jamais mettre on-chain:

- document;
- image;
- visage;
- embedding;
- template biometrique;
- date de naissance;
- numero de document;
- adresse;
- nom legal;
- hash direct d'une donnee personnelle.

### Token ou NFT

Si un token existe, il doit etre:

- non transferable;
- sans PII;
- revocable ou associe a un registre de statut;
- optionnel;
- inutile sans verification du credential off-chain;
- non suffisant pour prouver une identite legale.

Le token peut prouver:

```text
This wallet controls a currently valid nvbes Verify status.
```

Il ne doit pas prouver:

```text
This wallet belongs to Jean Dupont, born on ...
```

## Architecture Cible

```text
User device
  -> Capture document and face
  -> Local or provider verification
  -> Raw artifacts deleted

nvbes Verify API
  -> Receives verification result
  -> Issues credential
  -> Stores commitment, status, consent, audit
  -> Never stores raw document or raw face scan

Relying party
  -> Requests claims
  -> User consents
  -> Receives limited presentation

Optional Web3 layer
  -> Anchors Merkle root
  -> Publishes issuer key / revocation status
```

## Composants Systeme

### Verification Orchestrator

Responsabilites:

- piloter le parcours document + visage;
- recevoir le resultat du moteur de verification;
- appliquer la politique de retention immediate;
- produire un statut normalise.

### Credential Issuer

Responsabilites:

- signer les credentials;
- gerer les cles d'issuer;
- definir les schemas de claims;
- gerer expiration et rotation.

### Consent Service

Responsabilites:

- afficher les claims demandes;
- enregistrer le consentement;
- gerer les refus;
- exposer l'historique utilisateur;
- permettre la revocation.

### Presentation Service

Responsabilites:

- produire une preuve limitee;
- appliquer selective disclosure;
- limiter la duree de validite;
- verifier la politique du relying party.

### Revocation Registry

Responsabilites:

- marquer credential actif, expire ou revoque;
- exposer une verification de statut;
- publier un ancrage optionnel;
- eviter toute fuite de PII.

### Biometric Template Service

Responsabilites:

- transformer l'empreinte visage en template protege;
- empecher la reconstruction;
- deriver des preuves specifiques par relying party;
- permettre rotation et invalidation.

## Modele de Donnees Conceptuel

### `kyc_subjects`

- `id`
- `principal_id`
- `status`
- `assurance_level`
- `verified_at`
- `expires_at`
- `created_at`

### `kyc_credentials`

- `id`
- `subject_id`
- `credential_hash`
- `credential_schema`
- `issuer_key_id`
- `status`
- `issued_at`
- `expires_at`
- `revoked_at`

### `kyc_claims`

- `credential_id`
- `claim_key`
- `claim_value_commitment`
- `disclosure_policy`

### `kyc_biometric_templates`

- `subject_id`
- `template_version`
- `protected_template_ciphertext`
- `transform_id`
- `status`
- `created_at`
- `revoked_at`

### `kyc_consents`

- `id`
- `subject_id`
- `relying_party_id`
- `requested_claims`
- `granted_claims`
- `purpose`
- `status`
- `granted_at`
- `expires_at`
- `revoked_at`

### `kyc_audit_events`

- `id`
- `subject_id`
- `event_type`
- `relying_party_id`
- `metadata_minimal`
- `created_at`

## API Conceptuelle

### Demarrer une verification

```http
POST /verify/kyc/sessions
```

### Finaliser une verification

```http
POST /verify/kyc/sessions/{session_id}/complete
```

### Demander une preuve

```http
POST /verify/presentations/requests
```

### Consentir a une preuve

```http
POST /verify/presentations/{request_id}/consent
```

### Verifier une presentation

```http
POST /verify/presentations/verify
```

### Revoquer un credential

```http
POST /verify/credentials/{credential_id}/revoke
```

## MVP Recommande

Phase 1:

- verification document + visage via provider ou moteur isole;
- suppression immediate des artefacts bruts;
- stockage de statut KYC et commitment;
- credential signe simple;
- expiration du credential;
- API relying party avec consentement explicite;
- audit minimal.

Phase 2:

- selective disclosure;
- revocation registry;
- presentation a duree courte;
- portail utilisateur pour gerer les consentements;
- derivation relying-party-specific.

Phase 3:

- proofs zero-knowledge pour `age_over_18`, residence ou statut KYC;
- ancrage Merkle root on-chain;
- token non transferable optionnel;
- key transparency log.

## Risques et Garde-Fous

### Risque Biometrique

Un template visage protege reste une donnee biometrique sensible.

Garde-fous:

- chiffrement fort;
- separation des cles;
- HSM ou enclave si possible;
- rotation de template;
- limitation stricte d'acces;
- audit obligatoire;
- retention minimale.

### Risque de Correlation

Un identifiant global stable permettrait de suivre un utilisateur entre sites.

Garde-fous:

- identifiants specifiques par relying party;
- nonces;
- presentations courtes;
- pas de wallet public comme identifiant principal.

### Risque On-Chain

Une donnee on-chain est difficile a supprimer.

Garde-fous:

- pas de PII on-chain;
- pas de hash direct de PII;
- seulement Merkle root, statut ou cle publique;
- design compatible droit a l'effacement off-chain.

### Risque Legal

KYC, biometrie et verification d'identite peuvent declencher des obligations fortes.

Garde-fous:

- DPIA avant implementation;
- base legale explicite;
- consentement separe pour biometrie si necessaire;
- retention documentee;
- sous-traitants audites;
- procedure de suppression;
- registre des traitements.

## Questions Ouvertes

- Le produit doit-il viser particuliers, entreprises regulees, crypto exchanges ou marketplaces?
- Faut-il verifier seulement `age_over_18` ou aussi identite complete?
- Quel niveau d'assurance est requis?
- Le scan visage 3D doit-il servir a l'unicite ou seulement au liveness?
- Quel fournisseur KYC/liveness peut garantir la suppression immediate?
- Faut-il supporter eIDAS, EUDI Wallet ou autre standard europeen?
- Faut-il emettre des credentials W3C des le MVP ou commencer par JWT signe?
- Le Web3 est-il un avantage produit ou seulement un mecanisme de preuve optionnel?

## Decision de Design Actuelle

Le produit doit etre concu comme:

```text
Reusable Private KYC Credential
```

et non comme:

```text
Public KYC NFT
```

La valeur produit vient de la verification reutilisable, de la minimisation des donnees, du consentement et de la preuve selective.
