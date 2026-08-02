# ADR 0007 - Operate Staging as an Ephemeral Environment

## Status

Accepted.

## Context

The staging environment must remain available to developers and automated
validation without requiring source-IP allowlists. It must also avoid the cost
and operational drift of continuously running infrastructure that is used only
during development and release validation windows.

Staging already has stable Cloudflare-managed hostnames and a private Scaleway
network. Publishing a permanent public origin or keeping a traditional VPN
gateway online would preserve an unnecessary public ingress path and would not
allow the compute layer to scale to zero.

OAuth clients also require stable issuer URLs, redirect URIs and cookie origins.
These public contracts must not change when the staging compute layer is
destroyed and recreated.

## Decision

Operate staging as an ephemeral environment with a persistent access and
control plane.

Cloudflare Zero Trust is the stable access boundary. Enrolled devices connect
through the Cloudflare One client, formerly WARP, and access is authorized by
identity, MFA and device policy rather than by the user's source IP address.
Cloudflare Tunnel connects to staging through outbound-only connections, so
staging services require no public ingress address or inbound SSH port.

The following resources remain persistent:

- Cloudflare DNS records, Access policies and Tunnel configuration;
- OAuth issuer hostnames, client redirect URIs and other public identifiers;
- Terraform remote state and locking;
- secrets required to provision the environment;
- object storage used for approved fixtures or recoverable artifacts;
- audit records for provisioning and destruction operations.

The following resources are ephemeral:

- staging API and worker compute;
- Cloudflare Tunnel connector processes;
- private CI runners when required by a validation job;
- the staging database and its private attachment;
- other staging-only compute dependencies.

Terraform is the only supported mechanism for creating and destroying the
ephemeral resources. Provisioning must be deterministic and must not depend on
manual server configuration.

### Lifecycle

The staging lifecycle has the following states:

1. `OFF`: no ephemeral resources are running.
2. `PROVISIONING`: Terraform creates the environment, applies database
   migrations, loads approved test data and starts Tunnel connectors.
3. `READY`: health checks have passed and the environment accepts authorized
   traffic.
4. `DRAINING`: new validation jobs are rejected and active jobs receive a
   bounded period in which to finish.
5. `DESTROYING`: Terraform removes every ephemeral resource.
6. `FAILED`: provisioning or destruction requires automated reconciliation or
   an authorized operator intervention.

Provisioning can be initiated by an authorized manual workflow, a release or
pull-request workflow, or a defined working-hours schedule. The initial V0 does
not provision the environment in response to the first HTTP request. That
mechanism would require a permanently available wake-up controller, a waiting
protocol and additional abuse protections.

The environment enters `DRAINING` after 60 to 90 minutes without qualifying
developer or CI activity. Every provisioning operation also carries an
`expires_at` value with a maximum lifetime of eight hours. A nightly
reconciliation job destroys any remaining ephemeral resources regardless of
their reported activity.

Activity extends the idle deadline only when it originates from an authorized
developer session or an authenticated CI job. Public probes, health checks and
failed access attempts never keep staging alive.

### Data lifecycle

The staging database is recreated from migrations and a versioned,
deterministic test dataset. Staging must not rely on mutable historical data for
correctness and must not contain production personal data.

Artifacts that need to survive destruction must be explicitly classified and
written to persistent object storage. Persistence is opt-in; adding a durable
staging resource requires a separate architectural decision or an amendment to
this decision.

### Automation and safety

Provisioning and destruction automation must:

- use a dedicated staging cloud project and a staging-specific Terraform state;
- authenticate with a machine identity restricted to staging resources;
- acquire an exclusive lock before changing lifecycle state;
- be idempotent and reconcile partially completed operations;
- require the environment identifier to equal `staging` before destruction;
- reject production project, state and resource identifiers;
- record the actor, trigger, state transition, Terraform result and expiry;
- expose readiness and expiry status to authorized users;
- notify active users before entering `DRAINING`;
- verify that all ephemeral resources are absent after destruction.

Private CI and security validation run from an ephemeral runner connected to
the staging private network, or through an identity-bound machine access
policy. GitHub-hosted runners must not require a temporary public ingress
allowlist.

When staging is `OFF`, its stable hostnames return an explicit unavailable or
sleeping response from the persistent edge. They must not resolve directly to a
stale origin address. OAuth issuer names and registered redirect URIs therefore
remain unchanged across environment lifecycles.

## Consequences

Staging compute can scale to zero and is billed only while developers or
validation workflows need it. Access no longer depends on individual public IP
addresses, and application origins no longer expose permanent inbound ports.
Recreating the environment continuously also detects non-deterministic
provisioning, migration and fixture failures earlier.

Starting staging now has a provisioning delay. Developers and CI must wait for
the `READY` state, and workflows must tolerate an environment that is normally
offline. Logs and artifacts disappear unless they are exported before
destruction.

The persistent control plane becomes security-critical. Compromise of its
provisioning identity could create infrastructure or destroy staging, so its
permissions, audit trail and environment guards must be tested. Cloudflare is
also part of the staging access path and an outage can make the environment
unreachable even when its Scaleway resources are healthy.

The V0 deliberately favors a small state machine and scheduled reconciliation
over Kubernetes, a custom autoscaler or an HTTP-triggered wake-up service. A
more reactive provisioning model may be considered only when measured usage
shows that the provisioning delay is materially harmful.

## Validation

This decision is complete only when:

- staging can be provisioned from `OFF` without manual server changes;
- the API, SSH and internal services have no public inbound path;
- an enrolled and authorized device can access the private staging services;
- an unauthorized device or identity cannot access them;
- OAuth issuer URLs and redirect URIs remain stable across destruction and
  recreation;
- database migrations and deterministic fixtures produce a usable environment;
- concurrent lifecycle requests are serialized;
- idle timeout, maximum lifetime and nightly reconciliation each trigger a
  successful destruction;
- failed or interrupted operations converge to `READY` or `OFF`;
- destruction automation cannot target production state or resources;
- CI and security validation require no source-IP allowlist;
- audit records identify who or what initiated every lifecycle transition.
