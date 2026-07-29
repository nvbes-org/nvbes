# Programme d'assurance sécurité et conformité

## Objet

Ce programme rend les standards de sécurité vérifiables dans le dépôt. Il ne constitue ni
une certification, ni une attestation, ni un avis juridique. La source de vérité
machine-readable est `standards-control-matrix.json`.

## Profils obligatoires

| Profil | Périmètre | Exigence |
| --- | --- | --- |
| `NVBES_BASELINE` | Toutes les applications exposées et toutes les API | OWASP ASVS 5.0.0 niveau 2 |
| `NVBES_CRITICAL` | Account, Backoffice, Billing et opérations privilégiées | ASVS niveau 3, AAL2 et authentification résistante au phishing |

Une fonctionnalité critique hérite des deux profils. Une exception doit être limitée dans
le temps, avoir un owner, une justification, une mesure compensatoire et une date de
réexamen.

## Référentiels adoptés

- OWASP ASVS 5.0.0 pour les exigences et tests de sécurité applicative.
- NIST SP 800-63-4 pour l'identité, les authentificateurs et la fédération.
- RFC 9700, OIDC, JWT BCP, PAR, JAR, DPoP ou mTLS, avec FAPI 2.0 pour le profil haute assurance.
- NIST CSF 2.0 et CIS Controls 8.1 pour le cycle de gouvernance et d'exploitation.
- ISO/IEC 27001:2022 comme cible SMSI, complétée par ISO 27017 et ISO 27018.
- RGPD et recommandations CNIL comme obligations privacy applicables.
- PCI DSS 4.0.1 avec réduction de périmètre par Checkout et Portal hébergés.
- SOC 2 Type II comme future attestation après une période d'observation stable.
- NIS2 et Cyber Resilience Act comme décisions d'applicabilité à documenter.

## États de contrôle

- `implemented` : preuve présente et vérifiable dans le dépôt.
- `partial` : mesure existante mais preuve ou capacité requise encore incomplète.
- `planned` : contrôle non disponible; interdit pour une release qui en dépend.
- `not-applicable` : justification et validation formelle obligatoires.

Le gate vérifie l'intégrité du registre, les profils, la couverture des neuf référentiels,
les preuves textuelles et l'explicitation des écarts. Il ne remplace pas les tests
techniques, les restaurations, un pentest, un audit légal ou une attestation externe.

## Politique de release

Avant une release de production :

1. exécuter `pnpm check:compliance-standards`;
2. exécuter les gates de sécurité spécialisés référencés par la matrice;
3. interdire tout contrôle `planned` dans le profil concerné;
4. accepter un contrôle `partial` uniquement avec un risque signé et une échéance;
5. joindre les preuves opérationnelles qui ne peuvent pas vivre dans Git;
6. faire approuver les changements critiques par Security et le domain owner.

## Politique de communication

La formulation publique autorisée est : « contrôles internes mappés aux référentiels
indiqués ». Les termes « certifié », « attesté » ou « pleinement conforme » sont interdits
sans artefact externe en cours de validité et référencé dans le registre de preuves.

## Sources normatives

- [OWASP ASVS](https://owasp.org/www-project-application-security-verification-standard/)
- [NIST SP 800-63-4](https://pages.nist.gov/800-63-4/)
- [OAuth Security BCP, RFC 9700](https://datatracker.ietf.org/doc/html/rfc9700)
- [FAPI 2.0 Security Profile](https://openid.net/specs/fapi-security-profile-2_0-final.html)
- [NIST CSF 2.0](https://www.nist.gov/cyberframework)
- [CIS Controls 8.1](https://www.cisecurity.org/controls/v8-1)
- [ISO/IEC 27001:2022](https://www.iso.org/standard/27001)
- [ISO/IEC 27017 cloud controls](https://www.iso.org/standard/82878.html)
- [ISO/IEC 27018:2025 public-cloud PII controls](https://www.iso.org/standard/27018)
- [Guide sécurité de la CNIL](https://www.cnil.fr/fr/guide-de-la-securite-des-donnees-personnelles)
- [NIS2](https://eur-lex.europa.eu/eli/dir/2022/2555/oj)
- [Cyber Resilience Act](https://eur-lex.europa.eu/eli/reg/2024/2847/oj)
