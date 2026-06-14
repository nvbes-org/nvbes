#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InheritedPolicyLayer {
    pub scope: PolicyScope,
    pub member_can_create_share_links: Option<bool>,
    pub require_admin_approval_for_member_share: Option<bool>,
    pub default_share_link_ttl_days: Option<i32>,
    pub max_share_link_ttl_days: Option<i32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PolicyScope {
    #[default]
    System,
    Tenant,
    Organization,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveWorkspacePolicy {
    pub member_can_create_share_links: bool,
    pub require_admin_approval_for_member_share: bool,
    pub default_share_link_ttl_days: i32,
    pub max_share_link_ttl_days: i32,
}

impl InheritedPolicyLayer {
    pub fn system() -> Self {
        Self {
            scope: PolicyScope::System,
            ..Self::default()
        }
    }

    pub fn tenant() -> Self {
        Self {
            scope: PolicyScope::Tenant,
            ..Self::default()
        }
    }

    pub fn organization() -> Self {
        Self {
            scope: PolicyScope::Organization,
            ..Self::default()
        }
    }

    pub fn workspace() -> Self {
        Self {
            scope: PolicyScope::Workspace,
            ..Self::default()
        }
    }

    pub fn with_member_share_links(mut self, value: bool) -> Self {
        self.member_can_create_share_links = Some(value);
        self
    }

    pub fn with_require_admin_approval_for_member_share(mut self, value: bool) -> Self {
        self.require_admin_approval_for_member_share = Some(value);
        self
    }

    pub fn with_default_share_link_ttl_days(mut self, value: i32) -> Self {
        self.default_share_link_ttl_days = Some(value);
        self
    }

    pub fn with_max_share_link_ttl_days(mut self, value: i32) -> Self {
        self.max_share_link_ttl_days = Some(value);
        self
    }
}

impl Default for EffectiveWorkspacePolicy {
    fn default() -> Self {
        Self {
            member_can_create_share_links: false,
            require_admin_approval_for_member_share: true,
            default_share_link_ttl_days: 7,
            max_share_link_ttl_days: 30,
        }
    }
}

pub fn resolve_inherited_policy(
    layers: impl IntoIterator<Item = InheritedPolicyLayer>,
) -> EffectiveWorkspacePolicy {
    let mut policy = EffectiveWorkspacePolicy::default();

    for layer in layers {
        if let Some(value) = layer.member_can_create_share_links {
            policy.member_can_create_share_links = value;
        }
        if let Some(value) = layer.require_admin_approval_for_member_share {
            policy.require_admin_approval_for_member_share = value;
        }
        if let Some(value) = layer.default_share_link_ttl_days {
            policy.default_share_link_ttl_days = value;
        }
        if let Some(value) = layer.max_share_link_ttl_days {
            policy.max_share_link_ttl_days = value;
        }
    }

    policy
}

#[cfg(test)]
mod tests {
    use super::{InheritedPolicyLayer, resolve_inherited_policy};

    #[test]
    fn workspace_policy_overrides_organization_tenant_and_system_layers() {
        let policy = resolve_inherited_policy([
            InheritedPolicyLayer::system()
                .with_member_share_links(false)
                .with_max_share_link_ttl_days(30),
            InheritedPolicyLayer::tenant()
                .with_member_share_links(true)
                .with_max_share_link_ttl_days(60),
            InheritedPolicyLayer::organization().with_member_share_links(false),
            InheritedPolicyLayer::workspace().with_member_share_links(true),
        ]);

        assert!(policy.member_can_create_share_links);
        assert_eq!(policy.max_share_link_ttl_days, 60);
    }

    #[test]
    fn missing_lower_layers_fall_back_to_system_defaults() {
        let policy = resolve_inherited_policy([InheritedPolicyLayer::system()
            .with_member_share_links(false)
            .with_default_share_link_ttl_days(7)
            .with_max_share_link_ttl_days(30)]);

        assert!(!policy.member_can_create_share_links);
        assert_eq!(policy.default_share_link_ttl_days, 7);
        assert_eq!(policy.max_share_link_ttl_days, 30);
    }
}
