use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use tonic::Status;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthorizationConstraints {
    pub minimum_device_trust: DeviceTrust,
    pub allowed_network_zones: BTreeSet<String>,
    pub maximum_risk_score: Option<u8>,
    pub allowed_data_regions: BTreeSet<String>,
    pub allowed_subject_types: BTreeSet<String>,
    pub denied_actions: BTreeSet<String>,
    pub require_step_up: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceTrust {
    #[default]
    Unknown,
    Registered,
    Managed,
    HardwareAttested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationEnvironment {
    pub device_trust: DeviceTrust,
    pub network_zone: String,
    pub risk_score: u8,
    pub data_region: String,
    pub subject_type: String,
    pub step_up_active: bool,
}

impl AuthorizationConstraints {
    pub fn from_json(value: &serde_json::Value) -> Result<Self, Status> {
        let constraints: Self = serde_json::from_value(value.clone()).map_err(|error| {
            Status::invalid_argument(format!("invalid authorization policy: {error}"))
        })?;
        for (field, values) in [
            ("allowed_network_zones", &constraints.allowed_network_zones),
            ("allowed_data_regions", &constraints.allowed_data_regions),
            ("allowed_subject_types", &constraints.allowed_subject_types),
            ("denied_actions", &constraints.denied_actions),
        ] {
            if values.iter().any(|value| value.trim().is_empty()) {
                return Err(Status::invalid_argument(format!(
                    "{field} cannot contain empty values"
                )));
            }
        }
        Ok(constraints)
    }

    pub fn combine_with_child(&self, child: &Self) -> Self {
        Self {
            minimum_device_trust: self.minimum_device_trust.max(child.minimum_device_trust),
            allowed_network_zones: narrow_set(
                &self.allowed_network_zones,
                &child.allowed_network_zones,
            ),
            maximum_risk_score: narrow_maximum(self.maximum_risk_score, child.maximum_risk_score),
            allowed_data_regions: narrow_set(
                &self.allowed_data_regions,
                &child.allowed_data_regions,
            ),
            allowed_subject_types: narrow_set(
                &self.allowed_subject_types,
                &child.allowed_subject_types,
            ),
            denied_actions: self
                .denied_actions
                .union(&child.denied_actions)
                .cloned()
                .collect(),
            require_step_up: self.require_step_up || child.require_step_up,
        }
    }

    pub fn denial_reason(
        &self,
        action: &str,
        environment: &AuthorizationEnvironment,
    ) -> Option<&'static str> {
        if self.denied_actions.contains(action) {
            return Some("action_denied_by_policy");
        }
        if environment.device_trust < self.minimum_device_trust {
            return Some("device_trust_insufficient");
        }
        if !allows(&self.allowed_network_zones, &environment.network_zone) {
            return Some("network_zone_denied");
        }
        if self
            .maximum_risk_score
            .is_some_and(|maximum| environment.risk_score > maximum)
        {
            return Some("risk_score_too_high");
        }
        if !allows(&self.allowed_data_regions, &environment.data_region) {
            return Some("data_region_denied");
        }
        if !allows(&self.allowed_subject_types, &environment.subject_type) {
            return Some("subject_type_denied");
        }
        if self.require_step_up && !environment.step_up_active {
            return Some("step_up_required_by_policy");
        }
        None
    }
}

fn narrow_set(parent: &BTreeSet<String>, child: &BTreeSet<String>) -> BTreeSet<String> {
    match (parent.is_empty(), child.is_empty()) {
        (true, true) => BTreeSet::new(),
        (true, false) => child.clone(),
        (false, true) => parent.clone(),
        (false, false) => {
            let intersection: BTreeSet<String> = parent.intersection(child).cloned().collect();
            if intersection.is_empty() {
                // Stored policy values are validated as non-empty context labels. An empty
                // string therefore represents a fail-closed intersection, while an actually
                // empty set continues to mean that the dimension is unrestricted.
                [String::new()].into_iter().collect()
            } else {
                intersection
            }
        }
    }
}

fn narrow_maximum(parent: Option<u8>, child: Option<u8>) -> Option<u8> {
    match (parent, child) {
        (Some(parent), Some(child)) => Some(parent.min(child)),
        (Some(parent), None) => Some(parent),
        (None, Some(child)) => Some(child),
        (None, None) => None,
    }
}

fn allows(allowed: &BTreeSet<String>, actual: &str) -> bool {
    allowed.is_empty() || allowed.contains(actual)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn child_policy_cannot_weaken_parent_constraints() {
        let parent = AuthorizationConstraints {
            minimum_device_trust: DeviceTrust::Managed,
            allowed_network_zones: ["corporate".to_string()].into_iter().collect(),
            maximum_risk_score: Some(30),
            allowed_data_regions: ["eu".to_string()].into_iter().collect(),
            denied_actions: ["workspace.delete".to_string()].into_iter().collect(),
            require_step_up: true,
            ..AuthorizationConstraints::default()
        };
        let child = AuthorizationConstraints {
            minimum_device_trust: DeviceTrust::Registered,
            allowed_network_zones: ["corporate".to_string(), "public".to_string()]
                .into_iter()
                .collect(),
            maximum_risk_score: Some(80),
            allowed_data_regions: ["eu".to_string(), "us".to_string()].into_iter().collect(),
            ..AuthorizationConstraints::default()
        };

        let effective = parent.combine_with_child(&child);

        assert_eq!(effective.minimum_device_trust, DeviceTrust::Managed);
        assert_eq!(effective.maximum_risk_score, Some(30));
        assert_eq!(
            effective.allowed_network_zones,
            ["corporate".to_string()].into_iter().collect()
        );
        assert_eq!(
            effective.allowed_data_regions,
            ["eu".to_string()].into_iter().collect()
        );
        assert!(effective.denied_actions.contains("workspace.delete"));
        assert!(effective.require_step_up);
    }

    #[test]
    fn contextual_policy_denies_on_each_restricted_dimension() {
        let constraints = AuthorizationConstraints {
            minimum_device_trust: DeviceTrust::Managed,
            allowed_network_zones: ["corporate".to_string()].into_iter().collect(),
            maximum_risk_score: Some(40),
            allowed_data_regions: ["eu".to_string()].into_iter().collect(),
            allowed_subject_types: ["user".to_string()].into_iter().collect(),
            require_step_up: true,
            ..AuthorizationConstraints::default()
        };
        let environment = AuthorizationEnvironment {
            device_trust: DeviceTrust::Registered,
            network_zone: "corporate".to_string(),
            risk_score: 10,
            data_region: "eu".to_string(),
            subject_type: "user".to_string(),
            step_up_active: true,
        };

        assert_eq!(
            constraints.denial_reason("files.view", &environment),
            Some("device_trust_insufficient")
        );
    }

    #[test]
    fn disjoint_parent_and_child_sets_fail_closed() {
        let parent = AuthorizationConstraints {
            allowed_data_regions: ["eu".to_string()].into_iter().collect(),
            ..AuthorizationConstraints::default()
        };
        let child = AuthorizationConstraints {
            allowed_data_regions: ["us".to_string()].into_iter().collect(),
            ..AuthorizationConstraints::default()
        };
        let effective = parent.combine_with_child(&child);
        let environment = AuthorizationEnvironment {
            device_trust: DeviceTrust::Unknown,
            network_zone: "unknown".to_string(),
            risk_score: 0,
            data_region: "eu".to_string(),
            subject_type: "user".to_string(),
            step_up_active: false,
        };

        assert_eq!(
            effective.denial_reason("files.view", &environment),
            Some("data_region_denied")
        );
    }
}
