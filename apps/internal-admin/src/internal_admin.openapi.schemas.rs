use serde_json::{Value, json};

use crate::openapi_action_schemas::{
    access_action_result, entitlement_action_result, generic_action_result,
    operator_grant_action_result, principal_action_result, tenant_lifecycle_result,
    user_lifecycle_result, workspace_lifecycle_result,
};
use crate::openapi_schema_helpers::{
    array_schema, date_time_schema, enum_schema, free_object, free_value, nullable_ref,
    nullable_string, nullable_uuid, number_schema, object, ref_schema, string_schema, uuid_schema,
};

pub(crate) fn components() -> Value {
    json!({
        "securitySchemes": {
            "internalToken": {"type": "http", "scheme": "bearer"},
            "operatorContext": {
                "type": "apiKey",
                "in": "header",
                "name": "x-nvbes-actor-principal-id",
                "description": "Back-office actor principal UUID. Mutations also use x-nvbes-backoffice-role and dual-control headers."
            }
        },
        "schemas": {
            "ErrorEnvelope": object(&[("error", ref_schema("ErrorBody"))]),
            "ErrorBody": object(&[
                ("code", string_schema()),
                ("message", string_schema()),
                ("request_id", nullable_string())
            ]),
            "CriticalActionRequest": object(&[
                ("confirm_code", string_schema()),
                ("reason", string_schema())
            ]),
            "BackofficeRole": enum_schema(&[
                "compliance_admin",
                "developer_admin",
                "finance_admin",
                "operations_admin",
                "platform_admin",
                "product_admin",
                "security_admin",
                "support_agent",
                "viewer"
            ]),
            "SuppressionReviewRequest": object(&[
                ("confirm_code", string_schema()),
                ("email", string_schema()),
                ("reason", string_schema())
            ]),
            "RegionFlagRequest": object(&[
                ("confirm_code", string_schema()),
                ("data_region", string_schema()),
                ("jurisdiction", string_schema()),
                ("reason", string_schema())
            ]),
            "RegionExceptionRequest": object(&[
                ("confirm_code", string_schema()),
                ("exception_kind", string_schema()),
                ("reason", string_schema())
            ]),
            "EntitlementFeatureActionRequest": object(&[
                ("confirm_code", string_schema()),
                ("feature_code", string_schema()),
                ("value", free_object()),
                ("reason", string_schema())
            ]),
            "EntitlementQuotaOverrideRequest": object(&[
                ("confirm_code", string_schema()),
                ("quota_code", string_schema()),
                ("included_quantity", number_schema()),
                ("reason", string_schema())
            ]),
            "UsageCorrectionRequest": object(&[
                ("confirm_code", string_schema()),
                ("usage_event_id", nullable_string()),
                ("meter_code", string_schema()),
                ("quantity_delta", number_schema()),
                ("reason", string_schema())
            ]),
            "FreezeMeterRequest": object(&[
                ("confirm_code", string_schema()),
                ("meter_code", string_schema()),
                ("reason", string_schema())
            ]),
            "IncidentStateRequest": object(&[
                ("confirm_code", string_schema()),
                ("reason", string_schema()),
                ("status", enum_schema(&["open", "mitigating", "resolved"]))
            ]),
            "MaintenanceWindowRequest": object(&[
                ("confirm_code", string_schema()),
                ("reason", string_schema()),
                ("title", string_schema()),
                ("scheduled_start_at", date_time_schema()),
                ("scheduled_end_at", date_time_schema())
            ]),
            "CommandCenterSnapshot": free_object(),
            "PendingApprovalsSnapshot": free_object(),
            "AuditEvidenceSnapshot": free_object(),
            "AuditEvidenceExport": free_object(),
            "AuditEvent": object(&[
                ("id", uuid_schema()),
                ("tenant_id", uuid_schema()),
                ("workspace_id", nullable_uuid()),
                ("action", string_schema()),
                ("actor_principal_id", nullable_uuid()),
                ("actor_email", nullable_string()),
                ("target_type", string_schema()),
                ("target_id", nullable_uuid()),
                ("target_link", nullable_ref("AuditTargetLink")),
                ("changes", array_schema(ref_schema("AuditChange"))),
                ("metadata", free_object()),
                ("event_hash", string_schema()),
                ("previous_event_hash", nullable_string()),
                ("hash_chain_status", enum_schema(&["linked", "chain_head", "hash_anomaly"])),
                ("created_at", date_time_schema())
            ]),
            "AuditTargetLink": object(&[
                ("kind", enum_schema(&["tenant", "workspace", "user"])),
                ("id", uuid_schema()),
                ("href", enum_schema(&["#tenant-detail", "#workspace-detail", "#user-detail"])),
                ("label", string_schema())
            ]),
            "AuditChange": object(&[
                ("field", string_schema()),
                ("before", free_value()),
                ("after", free_value())
            ]),
            "RevenueActionResult": generic_action_result(),
            "BillingPlatformActionResult": generic_action_result(),
            "RiskActionResult": generic_action_result(),
            "ComplianceActionResult": generic_action_result(),
            "CommunicationsActionResult": generic_action_result(),
            "RegionActionResult": generic_action_result(),
            "DeveloperActionResult": generic_action_result(),
            "UsageActionResult": generic_action_result(),
            "OperationsActionResult": generic_action_result(),
            "GovernanceActionResult": principal_action_result(),
            "SecurityActionResult": principal_action_result(),
            "OperatorGrantActionResult": operator_grant_action_result(),
            "EntitlementActionResult": entitlement_action_result(),
            "AccessActionResult": access_action_result(),
            "UserLifecycleResult": user_lifecycle_result(),
            "WorkspaceLifecycleResult": workspace_lifecycle_result(),
            "TenantLifecycleResult": tenant_lifecycle_result()
        },
        "responses": {
            "BadRequest": error_response("Invalid request"),
            "Unauthorized": error_response("Authentication required"),
            "Forbidden": error_response("Operator role is not allowed"),
            "RateLimited": error_response("Too many requests")
        },
        "parameters": {
            "IdempotencyKey": {
                "name": "Idempotency-Key",
                "in": "header",
                "required": true,
                "schema": {"type": "string", "minLength": 8},
                "description": "Required on all back-office mutations."
            },
            "BackofficeRole": {
                "name": "x-nvbes-backoffice-role",
                "in": "header",
                "required": true,
                "schema": {"$ref": "#/components/schemas/BackofficeRole"},
                "description": "Operator role used for center/action RBAC."
            },
            "SecondApproverPrincipalId": {
                "name": "x-nvbes-second-approver-principal-id",
                "in": "header",
                "required": true,
                "schema": {"type": "string", "format": "uuid"}
            },
            "SecondApproverRole": {
                "name": "x-nvbes-second-approver-role",
                "in": "header",
                "required": true,
                "schema": {"type": "string", "enum": ["platform_admin"]}
            }
        }
    })
}

pub(crate) fn operation(
    operation_id: &str,
    tags: [&str; 1],
    summary: &str,
    request: Option<&str>,
    response: &str,
) -> Value {
    let mut op = json!({
        "tags": tags,
        "operationId": operation_id,
        "summary": summary,
        "responses": json_response(response)
    });
    if let Some(schema) = request {
        op["requestBody"] = request_body(schema);
    }
    op
}

pub(crate) fn critical_action(
    operation_id: &str,
    tags: [&str; 1],
    summary: &str,
    target_id: &str,
    response: &str,
) -> Value {
    critical_action_with_paths(
        operation_id,
        tags,
        summary,
        &["workspaceId", target_id],
        response,
    )
}

pub(crate) fn critical_action_with_paths(
    operation_id: &str,
    tags: [&str; 1],
    summary: &str,
    path_ids: &[&str],
    response: &str,
) -> Value {
    critical_action_with_body_paths(
        operation_id,
        tags,
        summary,
        path_ids,
        "CriticalActionRequest",
        response,
    )
}

pub(crate) fn critical_action_with_body_paths(
    operation_id: &str,
    tags: [&str; 1],
    summary: &str,
    path_ids: &[&str],
    request: &str,
    response: &str,
) -> Value {
    let mut op = operation(operation_id, tags, summary, Some(request), response);
    op["parameters"] = json!(
        path_ids
            .iter()
            .map(|name| path_uuid(name))
            .chain([
                json!({"$ref": "#/components/parameters/IdempotencyKey"}),
                json!({"$ref": "#/components/parameters/BackofficeRole"}),
                json!({"$ref": "#/components/parameters/SecondApproverPrincipalId"}),
                json!({"$ref": "#/components/parameters/SecondApproverRole"}),
            ])
            .collect::<Vec<_>>()
    );
    op
}

pub(crate) fn json_response_array(schema: &str) -> Value {
    json!({"200": {"description": "OK", "content": {"application/json": {"schema": array_schema(ref_schema(schema))}}}, "401": {"$ref": "#/components/responses/Unauthorized"}, "403": {"$ref": "#/components/responses/Forbidden"}, "429": {"$ref": "#/components/responses/RateLimited"}})
}

pub(crate) fn path_uuid(name: &str) -> Value {
    json!({"name": name, "in": "path", "required": true, "schema": {"type": "string", "format": "uuid"}})
}

pub(crate) fn query_param(name: &str, schema_type: &str) -> Value {
    json!({"name": name, "in": "query", "required": false, "schema": {"type": schema_type}})
}

pub(crate) fn json_response(schema: &str) -> Value {
    json!({"200": {"description": "OK", "content": {"application/json": {"schema": ref_schema(schema)}}}, "400": {"$ref": "#/components/responses/BadRequest"}, "401": {"$ref": "#/components/responses/Unauthorized"}, "403": {"$ref": "#/components/responses/Forbidden"}, "429": {"$ref": "#/components/responses/RateLimited"}})
}

fn request_body(schema: &str) -> Value {
    json!({"required": true, "content": {"application/json": {"schema": ref_schema(schema)}}})
}

fn error_response(description: &str) -> Value {
    json!({"description": description, "content": {"application/json": {"schema": ref_schema("ErrorEnvelope")}}})
}
