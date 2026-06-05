# nvbes : Conformité Enterprise & Secteurs Sensibles

Ce document synthétise l'état de préparation de nvbes pour l'onboarding de clients "Enterprise" (secteurs régulés, OIV, grandes entreprises), en gardant à l'esprit que le premier incrément ne couvre pas encore une suite enterprise complète.

## 1. Structure Légale et Gouvernance
- **Entité Légale** : nvbes Cloud SAS (Paris, France).
- **DPA (Data Processing Agreement)** : Finalisé et prêt pour signature. Définit nvbes comme sous-traitant (Data Processor).
- **Liste des Sous-traitants** : Nettoyée pour ne refléter que la stack réelle.
    - *Hébergement* : Scaleway (France).
    - *Paiement* : Stripe.
    - *Analytics/Product* : PostHog, Sentry.
    - *Réseau/Sécurité* : Cloudflare.

## 2. Engagements de Service (SLA)
Un document d'engagement de service a été créé pour garantir la transparence sur :
- **Disponibilité** : 99.9% pour le plan Enterprise.
- **Temps de Réponse Support** : < 2h pour les incidents critiques (P1).
- **Crédits de Service** : Mécanisme de compensation en cas de downtime.

## 3. Posture de Sécurité
- **VDP (Vulnerability Disclosure Policy)** : Politique publique pour l'accueil des signalements de chercheurs en sécurité.
- **Audit Logs** : Spécification technique pour une traçabilité complète des actions administratives (immuabilité, rétention 1 an).
- **Chiffrement** :
    - *Transit* : TLS 1.3 forcé via Cloudflare/API.
    - *Repos* : Chiffrement AES-256 (Scaleway Block Storage) et hachage Argon2id pour les mots de passe.

## 4. Conformité Privacy (RGPD)
- **Cookies** : Spécification conforme CNIL (Refus au premier niveau, traceurs essentiels uniquement par défaut).
- **Localisation des données** : Priorité au stockage souverain (Scaleway FR) pour les fichiers et la base de données.
- **Suppression des données** : Support du droit à l'oubli via les scripts de purge automatisés.

## 5. Cibles de conformité futures

Ces éléments sont des objectifs de maturité, pas des certifications ou attestations acquises. Toute communication publique devra distinguer clairement :

- les exigences déjà implémentées;
- la readiness interne;
- l'audit externe en cours;
- la certification, attestation ou autorisation obtenue.

### Socle prioritaire

- **GDPR / RGPD** : conformité privacy de base, DPA, registre des traitements, droits des personnes, DPIA si nécessaire, gestion des violations et gouvernance CNIL.
- **CNIL** : application des recommandations françaises pertinentes, notamment cookies, sécurité, durées de conservation, violations de données et AIPD.
- **SOC 2 Type I puis Type II** : contrôles sécurité, disponibilité, confidentialité et privacy pour clients enterprise.
- **ISO/IEC 27001** : SMSI cible, avec Statement of Applicability (SoA) maintenu comme livrable de gouvernance.
- **ISO 22301** : continuité d'activité, PRA/PCA, exercices de restauration et gestion de crise.
- **PCI DSS** : scope minimal via Stripe; nvbes ne doit jamais stocker de PAN ou données carte complètes.
- **NIS 2** : veille et readiness si nvbes entre dans un périmètre applicable ou sert des clients soumis.
- **DORA** : readiness contractuelle et opérationnelle pour clients financiers soumis.
- **EU Cyber Resilience Act** : exigences produit logiciel, vulnérabilités, mises à jour et documentation.
- **EU AI Act** : applicable uniquement aux fonctionnalités IA; classification de risque obligatoire avant livraison.

### Cibles sectorielles ou marchés spécifiques

- **CSA STAR** : cible cloud security utile pour grands comptes.
- **TISAX** : cible uniquement pour clients automotive ou supply chain industrielle.
- **HDS** : cible uniquement si nvbes héberge ou traite des données de santé françaises.
- **SecNumCloud** : cible souveraineté/sécurité élevée, à traiter comme programme long terme.
- **PDIS** : cible uniquement si nvbes fournit un service qualifié de détection d'incidents, pas pour le drive V1 standard.
- **FedRAMP** : cible uniquement pour le marché public fédéral US; choisir le niveau FedRAMP applicable (Low, Moderate ou High) au lieu d'un libellé générique.
- **TX-RAMP Level 2** : cible uniquement pour clients publics du Texas; le libellé `TX-RAMP` seul est redondant.
- **eIDAS 2.0** : cible uniquement si nvbes fournit des services d'identité, signature, wallet ou confiance numérique régulés.
- **Microsoft SSPA** : cible uniquement si nvbes devient fournisseur Microsoft ou traite des données pour Microsoft.
- **EU-US Data Privacy Framework, UK Extension et Swiss-US DPF** : pertinents uniquement pour les transferts transatlantiques impliquant des sous-traitants ou entités US certifiés. Le libellé `UL Extension to EU-US DPF` est corrigé en `UK Extension to EU-US DPF`.

### Retirés comme objectifs autonomes

- **ISO/IEC 27001 SoA** : conservé comme livrable du programme ISO/IEC 27001, pas comme conformité séparée.
- **ISO/EIC 27001** : libellé invalide, remplacé par `ISO/IEC 27001`.
- **FedRAMP Certified Class D** : libellé non retenu; FedRAMP doit être cadré par niveau reconnu.
- **TX-RAMP** : remplacé par `TX-RAMP Level 2` quand le marché Texas est visé.

## Prochaines étapes suggérées (Maturité +)
1. **SSO / SAML** : Implémentation du support pour les IdP clients (Okta, Azure AD).
2. **Programme conformité** : Prioriser RGPD/CNIL, SOC 2, ISO/IEC 27001 et ISO 22301 avant les cadres sectoriels.
3. **Bring Your Own Key (BYOK)** : Permettre aux clients sensibles de gérer leurs propres clés de chiffrement pour leurs fichiers.

## Périmètre non encore finalisé

- federation enterprise complète;
- SCIM enterprise complet;
- attestation device enterprise complète;
- policy engine final.

---
*Dernière mise à jour : 2026-06-05*
*Documents de référence dans `docs/legal` et `docs/compliance`.*
