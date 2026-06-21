use super::*;

#[test]
fn billing_admin_requires_tenant_context() {
    assert!(require_tenant_id(Some(Uuid::nil())).is_ok());
    assert!(require_tenant_id(None).is_err());
}
