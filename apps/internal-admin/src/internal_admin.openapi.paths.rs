use serde_json::{Map, Value, json};

use crate::openapi_schemas::{
    critical_action, critical_action_with_body_paths, critical_action_with_paths, json_response,
    json_response_array, operation, path_uuid, query_param,
};

pub(crate) struct CriticalEndpoint<'a> {
    pub(crate) path: &'a str,
    pub(crate) operation_id: &'a str,
    pub(crate) tag: &'a str,
    pub(crate) summary: &'a str,
    pub(crate) path_ids: &'a [&'a str],
    pub(crate) request: &'a str,
    pub(crate) response: &'a str,
}

pub(crate) fn paths() -> Value {
    let mut paths = Map::new();
    insert_gets(&mut paths);
    insert_audit(&mut paths);
    insert_core_mutations(&mut paths);
    insert_center_mutations(&mut paths);
    Value::Object(paths)
}

fn insert_gets(paths: &mut Map<String, Value>) {
    paths.insert(
        "/admin/command-center".to_string(),
        json!({"get": operation("getCommandCenter", ["command"], "Load enterprise command center", None, "CommandCenterSnapshot")}),
    );
    paths.insert(
        "/admin/pending-approvals".to_string(),
        json!({"get": operation("getPendingApprovals", ["command"], "List pending dual-control and review work", None, "PendingApprovalsSnapshot")}),
    );
    paths.insert(
        "/admin/audit-evidence-center".to_string(),
        json!({"get": operation("getAuditEvidenceCenter", ["audit"], "Load global audit evidence health", None, "AuditEvidenceSnapshot")}),
    );
}

fn insert_audit(paths: &mut Map<String, Value>) {
    paths.insert(
        "/workspaces/{workspaceId}/admin/audit-events".to_string(),
        json!({
            "get": {
                "tags": ["audit"],
                "operationId": "listAuditEvents",
                "summary": "List enriched audit events for a workspace tenant",
                "parameters": [
                    path_uuid("workspaceId"),
                    query_param("limit", "integer"),
                    query_param("action", "string"),
                    query_param("target_type", "string"),
                    query_param("q", "string")
                ],
                "responses": json_response_array("AuditEvent")
            }
        }),
    );
    paths.insert(
        "/workspaces/{workspaceId}/admin/audit-evidence/export".to_string(),
        json!({
            "get": {
                "tags": ["audit"],
                "operationId": "exportAuditEvidence",
                "summary": "Export enriched audit evidence package for a workspace tenant",
                "parameters": [
                    path_uuid("workspaceId"),
                    query_param("action", "string"),
                    query_param("target_type", "string"),
                    query_param("q", "string")
                ],
                "responses": json_response("AuditEvidenceExport")
            }
        }),
    );
}

fn insert_core_mutations(paths: &mut Map<String, Value>) {
    for endpoint in [
        CriticalEndpoint {
            path: "/admin/security-center/mfa-factors/{factorId}/revoke",
            operation_id: "revokeMfaFactor",
            tag: "security",
            summary: "Revoke an MFA factor",
            path_ids: &["factorId"],
            request: "CriticalActionRequest",
            response: "SecurityActionResult",
        },
        CriticalEndpoint {
            path: "/admin/security-center/oauth-consents/{consentId}/revoke",
            operation_id: "revokeOauthConsent",
            tag: "security",
            summary: "Revoke an OAuth consent",
            path_ids: &["consentId"],
            request: "CriticalActionRequest",
            response: "SecurityActionResult",
        },
        CriticalEndpoint {
            path: "/admin/identity-governance-center/break-glass/{tenantId}/{principalId}/revoke",
            operation_id: "revokeBreakGlassAccount",
            tag: "governance",
            summary: "Revoke a break-glass account",
            path_ids: &["tenantId", "principalId"],
            request: "CriticalActionRequest",
            response: "GovernanceActionResult",
        },
        CriticalEndpoint {
            path: "/admin/identity-governance-center/recovery-requests/{requestId}/cancel",
            operation_id: "cancelRecoveryRequest",
            tag: "governance",
            summary: "Cancel an enterprise recovery request",
            path_ids: &["requestId"],
            request: "CriticalActionRequest",
            response: "GovernanceActionResult",
        },
        CriticalEndpoint {
            path: "/admin/access-center/workspace-memberships/{workspaceId}/{principalId}/suspend",
            operation_id: "suspendWorkspaceMembership",
            tag: "governance",
            summary: "Suspend workspace membership",
            path_ids: &["workspaceId", "principalId"],
            request: "CriticalActionRequest",
            response: "AccessActionResult",
        },
        CriticalEndpoint {
            path: "/admin/users/{principalId}/suspend",
            operation_id: "suspendUser",
            tag: "governance",
            summary: "Suspend a user",
            path_ids: &["principalId"],
            request: "CriticalActionRequest",
            response: "UserLifecycleResult",
        },
        CriticalEndpoint {
            path: "/admin/users/{principalId}/reactivate",
            operation_id: "reactivateUser",
            tag: "governance",
            summary: "Reactivate a user",
            path_ids: &["principalId"],
            request: "CriticalActionRequest",
            response: "UserLifecycleResult",
        },
        CriticalEndpoint {
            path: "/admin/workspaces/{workspaceId}/suspend",
            operation_id: "suspendWorkspace",
            tag: "governance",
            summary: "Suspend a workspace",
            path_ids: &["workspaceId"],
            request: "CriticalActionRequest",
            response: "WorkspaceLifecycleResult",
        },
        CriticalEndpoint {
            path: "/admin/workspaces/{workspaceId}/reactivate",
            operation_id: "reactivateWorkspace",
            tag: "governance",
            summary: "Reactivate a workspace",
            path_ids: &["workspaceId"],
            request: "CriticalActionRequest",
            response: "WorkspaceLifecycleResult",
        },
        CriticalEndpoint {
            path: "/admin/tenants/{tenantId}/suspend",
            operation_id: "suspendTenant",
            tag: "governance",
            summary: "Suspend a tenant",
            path_ids: &["tenantId"],
            request: "CriticalActionRequest",
            response: "TenantLifecycleResult",
        },
        CriticalEndpoint {
            path: "/admin/tenants/{tenantId}/reactivate",
            operation_id: "reactivateTenant",
            tag: "governance",
            summary: "Reactivate a tenant",
            path_ids: &["tenantId"],
            request: "CriticalActionRequest",
            response: "TenantLifecycleResult",
        },
    ] {
        insert_endpoint(paths, endpoint);
    }
    insert_operator_grant_mutation(
        paths,
        "/admin/identity-governance-center/operator-grants/{principalId}/{role}/grant",
        "grantOperatorRole",
        "Grant a back-office operator role",
    );
    insert_operator_grant_mutation(
        paths,
        "/admin/identity-governance-center/operator-grants/{principalId}/{role}/revoke",
        "revokeOperatorRole",
        "Revoke a back-office operator role",
    );
}

fn insert_center_mutations(paths: &mut Map<String, Value>) {
    for endpoint in crate::openapi_paths_platform::endpoints()
        .into_iter()
        .chain(crate::openapi_paths_workflows::endpoints())
    {
        insert_endpoint(paths, endpoint);
    }
}

pub(crate) fn e<'a>(
    path: &'a str,
    operation_id: &'a str,
    tag: &'a str,
    summary: &'a str,
    path_ids: &'a [&'a str],
    request: &'a str,
    response: &'a str,
) -> CriticalEndpoint<'a> {
    CriticalEndpoint {
        path,
        operation_id,
        tag,
        summary,
        path_ids,
        request,
        response,
    }
}

fn insert_endpoint(paths: &mut Map<String, Value>, endpoint: CriticalEndpoint<'_>) {
    let operation = if endpoint.request == "CriticalActionRequest"
        && endpoint.path_ids.len() == 2
        && endpoint.path_ids[0] == "workspaceId"
    {
        critical_action(
            endpoint.operation_id,
            [endpoint.tag],
            endpoint.summary,
            endpoint.path_ids[1],
            endpoint.response,
        )
    } else if endpoint.request == "CriticalActionRequest" {
        critical_action_with_paths(
            endpoint.operation_id,
            [endpoint.tag],
            endpoint.summary,
            endpoint.path_ids,
            endpoint.response,
        )
    } else {
        critical_action_with_body_paths(
            endpoint.operation_id,
            [endpoint.tag],
            endpoint.summary,
            endpoint.path_ids,
            endpoint.request,
            endpoint.response,
        )
    };
    paths.insert(endpoint.path.to_string(), json!({"post": operation}));
}

fn insert_operator_grant_mutation(
    paths: &mut Map<String, Value>,
    path: &str,
    operation_id: &str,
    summary: &str,
) {
    let mut operation = crate::openapi_schemas::critical_action_with_body_paths(
        operation_id,
        ["governance"],
        summary,
        &["principalId"],
        "CriticalActionRequest",
        "OperatorGrantActionResult",
    );
    operation["parameters"] = json!([
        path_uuid("principalId"),
        {
            "name": "role",
            "in": "path",
            "required": true,
            "schema": {"$ref": "#/components/schemas/BackofficeRole"}
        },
        {"$ref": "#/components/parameters/IdempotencyKey"},
        {"$ref": "#/components/parameters/BackofficeRole"},
        {"$ref": "#/components/parameters/SecondApproverPrincipalId"},
        {"$ref": "#/components/parameters/SecondApproverRole"}
    ]);
    paths.insert(path.to_string(), json!({"post": operation}));
}
