use serde_json::Value;

use crate::openapi_schema_helpers::{
    enum_schema, nullable_string, number_schema, object, ref_schema, string_schema, uuid_schema,
};

pub(crate) fn generic_action_result() -> Value {
    object(&[
        ("object_id", string_schema()),
        ("action_kind", string_schema()),
        ("status", string_schema()),
        ("audit_action", string_schema()),
    ])
}

pub(crate) fn principal_action_result() -> Value {
    object(&[
        ("object_id", uuid_schema()),
        ("tenant_id", uuid_schema()),
        ("principal_id", uuid_schema()),
        ("audit_action", string_schema()),
    ])
}

pub(crate) fn entitlement_action_result() -> Value {
    object(&[
        ("object_id", string_schema()),
        ("action_kind", string_schema()),
        ("status", string_schema()),
        ("published_change_count", number_schema()),
        ("audit_action", string_schema()),
    ])
}

pub(crate) fn operator_grant_action_result() -> Value {
    object(&[
        ("object_id", uuid_schema()),
        ("principal_id", uuid_schema()),
        ("role", ref_schema("BackofficeRole")),
        ("previous_status", nullable_string()),
        ("next_status", enum_schema(&["active", "revoked"])),
        ("audit_action", string_schema()),
    ])
}

pub(crate) fn access_action_result() -> Value {
    object(&[
        ("workspace_id", uuid_schema()),
        ("tenant_id", uuid_schema()),
        ("principal_id", uuid_schema()),
        ("previous_status", string_schema()),
        ("next_status", string_schema()),
        ("audit_action", string_schema()),
    ])
}

pub(crate) fn user_lifecycle_result() -> Value {
    object(&[
        ("principal_id", uuid_schema()),
        ("tenant_id", uuid_schema()),
        ("previous_principal_status", string_schema()),
        ("previous_user_status", string_schema()),
        ("next_principal_status", string_schema()),
        ("next_user_status", string_schema()),
        ("audit_action", string_schema()),
    ])
}

pub(crate) fn workspace_lifecycle_result() -> Value {
    object(&[
        ("workspace_id", uuid_schema()),
        ("tenant_id", uuid_schema()),
        ("previous_status", string_schema()),
        ("next_status", string_schema()),
        ("audit_action", string_schema()),
    ])
}

pub(crate) fn tenant_lifecycle_result() -> Value {
    object(&[
        ("tenant_id", uuid_schema()),
        ("previous_status", string_schema()),
        ("next_status", string_schema()),
        ("audit_action", string_schema()),
    ])
}
