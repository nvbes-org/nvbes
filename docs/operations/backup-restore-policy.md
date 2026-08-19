# Backup & Disaster Recovery Policy

## Overview

This policy defines the backup architecture, recovery objectives (RPO / RTO), and verification procedures across nvbes services.

## Recovery Objectives

- **RPO (Recovery Point Objective)**:
  - Database (PostgreSQL WAL streaming + continuous archiving): < 5 minutes.
  - Object Storage (Cloud storage replication & versioning): < 15 minutes.
- **RTO (Recovery Time Objective)**:
  - Critical services (Identity, Gateway): < 30 minutes.
  - Asynchronous / worker services: < 2 hours.

## Restoration Verification & Gates

1. Automated point-in-time recovery test suite scheduled regularly against staging/isolated environments.
2. Integrity validation of cryptographic proofs and tenant isolations post-restore.
3. If recovery exceeds acceptable thresholds or data consistency checks fail, the release gate triggers an immediate **no-go**.
