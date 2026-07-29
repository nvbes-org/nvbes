# Supply-chain admission policies

Install Kyverno, then apply these policies before deploying production
workloads:

```bash
kubectl apply -f deploy/security/kyverno/verify-nvbes-images.yaml
kubectl apply -f deploy/security/kyverno/require-restricted-containers.yaml
```

The signature policy trusts only keyless signatures produced by the tagged
release workflow in `nvbes-org/nvbes`. Images are mutated to their verified
digest. The runtime policy enforces the restricted container baseline.

Test policies in an isolated cluster before enabling them on an existing
namespace. Existing unsigned or mutable-tag workloads will be rejected.
