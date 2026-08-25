# Backup & Disaster Recovery Policy

## Overview

This policy defines the backup architecture, recovery objectives (RPO / RTO), and verification procedures across nvbes services.

## Recovery Objectives

Before invited accounts are opened, the V1 foundation uses measured, manual
recovery objectives rather than an unverified SLA:

- **RPO (Recovery Point Objective)**: at most 24 hours;
- **RTO (Recovery Time Objective)**: at most 8 hours.

These targets apply to the initial single-region, scale-to-zero foundation and
must be demonstrated by an isolated restoration exercise. Tighter objectives
such as five-minute database RPO or two-hour worker RTO are future targets only;
they must not be presented as guarantees until the architecture, tests, measured
results and budget support them.

## Restoration Verification & Gates

1. Restore the latest eligible backup into an isolated, non-production target.
2. Record the backup timestamp, start and completion times, operator and target.
3. Run domain-specific integrity checks without exposing production secrets or
   personal data.
4. Delete or retain the isolated target according to the documented evidence
   policy after verification.
5. If recovery exceeds the applicable target or an integrity check fails, the
   release decision is an immediate **no-go**.

Automation is introduced only after the manual exercise is reliable and its
recurring cost remains inside the global FinOps contract.
