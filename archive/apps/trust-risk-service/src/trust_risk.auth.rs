use tonic::{Status, metadata::MetadataMap};

use crate::config::{OperatorPolicy, ProducerPolicy, TrustRiskConfig};

pub fn producer<'a>(
    metadata: &MetadataMap,
    config: &'a TrustRiskConfig,
    name: &str,
) -> Result<&'a ProducerPolicy, Status> {
    let policy = config.producers.get(name).ok_or_else(unauthenticated)?;
    authenticate(metadata, &policy.token)?;
    Ok(policy)
}

pub fn operator<'a>(
    metadata: &MetadataMap,
    config: &'a TrustRiskConfig,
    actor: &str,
    permission: &str,
) -> Result<&'a OperatorPolicy, Status> {
    let policy = config.operators.get(actor).ok_or_else(unauthenticated)?;
    authenticate(metadata, &policy.token)?;
    if !policy.permits(permission) {
        return Err(Status::permission_denied("operation is not permitted"));
    }
    Ok(policy)
}

fn authenticate(metadata: &MetadataMap, expected: &str) -> Result<(), Status> {
    let provided = metadata
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    constant_time_eq(provided.as_bytes(), expected.as_bytes())
        .then_some(())
        .ok_or_else(unauthenticated)
}

fn unauthenticated() -> Status {
    Status::unauthenticated("internal authentication required")
}

pub(crate) fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let length_difference = left.len() ^ right.len();
    let max_len = left.len().max(right.len());
    let difference = (0..max_len).fold(length_difference, |difference, index| {
        let left = left.get(index).copied().unwrap_or_default();
        let right = right.get(index).copied().unwrap_or_default();
        difference | usize::from(left ^ right)
    });
    difference == 0
}

#[cfg(test)]
#[path = "trust_risk.auth.tests.rs"]
mod tests;
