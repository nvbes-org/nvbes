# Data Classification Policy

**Document ID:** GAL-COMP-DCP-001  
**Version:** 1.0  
**Effective Date:** 2026-05-12  
**Owner:** Security & Compliance Team  
**Review Cycle:** Annual or upon significant system change

## 1. Purpose

This document defines the data classification framework for nvbes. It establishes categories for data sensitivity, handling requirements, and controls to ensure appropriate protection of data throughout its lifecycle.

All nvbes systems, services, and personnel handling nvbes data must comply with this policy.

## 2. Classification Levels

nvbes uses four classification levels, ordered from least to most sensitive:

| Level | Label | Color Code |
|-------|-------|------------|
| 0 | PUBLIC | Green |
| 1 | INTERNAL | Yellow |
| 2 | CONFIDENTIAL | Orange |
| 3 | RESTRICTED | Red |

### 2.1 PUBLIC

Data intended for public consumption. Disclosure causes no harm.

**Examples:**
- Marketing website content
- Public API documentation
- Open-source code repositories
- Press releases
- Public blog posts

**Handling Requirements:**

| Requirement | Rule |
|-------------|------|
| Encryption at rest | Not required |
| Encryption in transit | Standard TLS |
| Access control | None required |
| Logging | Unrestricted |
| Retention | As business needs dictate |
| Sharing with subprocessors | Unrestricted |

### 2.2 INTERNAL

Internal business data not harmful if disclosed, but not intended for public distribution.

**Examples:**
- Internal metrics and dashboards
- Non-sensitive configuration files
- Team documentation and runbooks
- IP addresses and user agents
- File metadata (name, size, MIME type)
- SAML public certificates

**Handling Requirements:**

| Requirement | Rule |
|-------------|------|
| Encryption at rest | Standard disk encryption (infrastructure-level) |
| Encryption in transit | TLS 1.2+ |
| Access control | Authenticated users only |
| Logging | Permitted without masking |
| Retention | 2 years maximum unless otherwise justified |
| Sharing with subprocessors | Permitted under NDA |

### 2.3 CONFIDENTIAL

Business-sensitive data that could cause harm to users or the business if leaked.

**Examples:**
- User email addresses
- File contents (user-defined sensitivity)
- Billing data (Stripe customer ID, subscription status)
- API keys (hashed)
- OAuth client secrets (hashed)
- Audit logs
- Business metrics and financial data

**Handling Requirements:**

| Requirement | Rule |
|-------------|------|
| Encryption at rest | Database-level encryption required (TDE or column-level) |
| Encryption in transit | TLS 1.2+ |
| Access control | Workspace-scoped; least privilege |
| Logging | Permitted only with masking (first/last characters visible) |
| Retention | 1 year after account deletion unless legal hold |
| Sharing with subprocessors | Permitted only under DPA with explicit data processing purpose |

### 2.4 RESTRICTED

Highly sensitive data requiring strict controls. Unauthorized disclosure could cause severe harm to individuals or the business.

**Examples:**
- Password hashes
- MFA secrets (TOTP seeds, WebAuthn credentials)
- Payment details (full card numbers — never stored; Stripe handles)
- JWT signing private keys
- Session tokens
- SAML private keys
- PII (national ID numbers, health data, biometric data)

**Handling Requirements:**

| Requirement | Rule |
|-------------|------|
| Encryption at rest | Application-level encryption required |
| Encryption in transit | TLS 1.3 preferred, 1.2+ minimum |
| Access control | Explicit authorization required; no implicit access |
| Logging | Never permitted in logs |
| Retention | Minimum necessary; automatic rotation where applicable |
| Sharing with subprocessors | Prohibited unless legally required and encrypted end-to-end |

## 3. Data Category Classification Map

The following table maps specific data categories to their classification level:

| Data Category | Classification | Justification |
|---------------|----------------|---------------|
| User emails | CONFIDENTIAL | Personal identifier; GDPR-protected |
| Password hashes | RESTRICTED | Authentication credential |
| MFA secrets (TOTP, WebAuthn) | RESTRICTED | Authentication credential |
| File contents | CONFIDENTIAL | User-defined; may contain sensitive data |
| File metadata (name, size, MIME type) | INTERNAL | Low sensitivity; operational necessity |
| Billing data (Stripe customer ID, subscription status) | CONFIDENTIAL | Business-sensitive; links to user identity |
| Payment details (full card numbers) | RESTRICTED | PCI-DSS scope; never stored directly |
| API keys (hashed) | CONFIDENTIAL | Access credential |
| JWT signing private keys | RESTRICTED | System authentication credential |
| Audit logs | CONFIDENTIAL | May contain user activity patterns |
| IP addresses | INTERNAL | Network identifier; low sensitivity alone |
| User agents | INTERNAL | Browser metadata; low sensitivity |
| Session tokens | RESTRICTED | Active authentication credential |
| OAuth client secrets (hashed) | CONFIDENTIAL | Access credential |
| SAML certificates (public) | INTERNAL | Public key material |
| SAML private keys | RESTRICTED | Authentication credential |

## 4. Handling Rules

### 4.1 Logging

| Classification | Log Behavior |
|----------------|-------------|
| PUBLIC | Full value permitted |
| INTERNAL | Full value permitted |
| CONFIDENTIAL | Masked: show first 2 and last 2 characters, redact middle |
| RESTRICTED | Never logged under any circumstances |

**Implementation:**
- PII-safe logging middleware must be applied to all application log pipelines
- Sentry SDK must be configured with PII filtering rules
- Log aggregation systems must enforce classification-aware filtering

### 4.2 Encryption

| Classification | At Rest | In Transit |
|----------------|---------|------------|
| PUBLIC | Infrastructure standard | TLS |
| INTERNAL | Infrastructure standard | TLS 1.2+ |
| CONFIDENTIAL | Database-level encryption | TLS 1.2+ |
| RESTRICTED | Application-level encryption | TLS 1.3 preferred |

### 4.3 Access Control

| Classification | Access Model |
|----------------|-------------|
| PUBLIC | Open |
| INTERNAL | Authenticated users |
| CONFIDENTIAL | Workspace-scoped; role-based |
| RESTRICTED | Explicit authorization; audit trail required |

### 4.4 Retention

| Classification | Default Retention | Deletion |
|----------------|-------------------|----------|
| PUBLIC | Business needs | Standard |
| INTERNAL | 2 years | Standard |
| CONFIDENTIAL | 1 year post-account deletion | Secure deletion |
| RESTRICTED | Minimum necessary; automatic rotation | Cryptographic erasure |

### 4.5 Subprocessor Sharing

| Classification | Sharing Permitted | Conditions |
|----------------|-------------------|------------|
| PUBLIC | Yes | None |
| INTERNAL | Yes | Under NDA |
| CONFIDENTIAL | Yes | Under DPA with explicit purpose |
| RESTRICTED | No | Unless legally required + end-to-end encryption |

## 5. Implementation Requirements

### 5.1 Database Schema

All tables storing user data must include classification columns:

- `classification` — enum (`public`, `internal`, `confidential`, `restricted`)
- `contains_pii` — boolean flag for PII-containing records

Classification is enforced at the application layer through the data classification service.

### 5.2 PII-Safe Logging

All logging must pass through the PII-safe logging middleware:

- Strings classified as CONFIDENTIAL are masked before output
- RESTRICTED data is stripped entirely from log context
- Email addresses are masked to show domain only
- UUIDs are truncated to first 8 characters

### 5.3 Sentry SDK Configuration

Sentry must be configured with:

- `before_send` callback to strip RESTRICTED data
- PII filtering for CONFIDENTIAL fields
- IP address anonymization
- User-agent truncation

## 6. Classification Changes

### 6.1 Increasing Classification

Any user with write access to a resource may increase its classification level (e.g., from CONFIDENTIAL to RESTRICTED).

### 6.2 Decreasing Classification

Only administrators may decrease classification levels. All decreases are logged in the audit trail with justification.

### 6.3 Automatic Classification

Files are automatically classified based on:
- MIME type analysis
- Filename pattern matching
- User-defined overrides (upward only)

## 7. Enforcement

Violations of this policy must be reported through the security incident response procedure. Non-compliance may result in:

- Access revocation
- Disciplinary action
- Legal consequences where applicable

## 8. Review and Updates

This policy is reviewed:
- Annually by the Security & Compliance Team
- Upon significant changes to nvbes's data processing activities
- Following any data breach incident

All changes require approval from the Security & Compliance Team lead.
