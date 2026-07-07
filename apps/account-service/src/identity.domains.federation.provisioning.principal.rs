use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{domains::cloud::workspace_port, http::error::AppError};

pub async fn resolve_or_create_principal(
    db: &PgPool,
    tenant_id: Uuid,
    email: &str,
    username: &str,
    email_verified: bool,
) -> Result<(Uuid, bool, bool), AppError> {
    let existing_user = sqlx::query(
        r#"
        SELECT
          p.id AS principal_id,
          p.tenant_id,
          u.email,
          u.username AS current_username
        FROM users u
        INNER JOIN principals p ON p.id = u.principal_id
        WHERE lower(u.email) = $1
        LIMIT 1
        "#,
    )
    .bind(email)
    .fetch_optional(db)
    .await?;

    let mut created_account = false;
    let mut created_membership = false;
    let principal_id = if let Some(row) = existing_user {
        let principal_id: Uuid = row.get("principal_id");
        let existing_tenant_id: Option<Uuid> = row.get("tenant_id");
        let current_username: Option<String> = row.get("current_username");
        if existing_tenant_id.is_some_and(|current| current != tenant_id) {
            return Err(AppError::conflict(
                crate::domains::federation::contract::PRINCIPAL_CONFLICT,
                "The existing account belongs to a different tenant.",
            ));
        }

        if current_username
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            sqlx::query(
                r#"
                UPDATE users
                SET username = $2,
                    email_verified_at = CASE
                      WHEN $3 THEN COALESCE(email_verified_at, NOW())
                      ELSE email_verified_at
                    END,
                    updated_at = NOW()
                WHERE principal_id = $1
                "#,
            )
            .bind(principal_id)
            .bind(username.trim())
            .bind(email_verified)
            .execute(db)
            .await?;
        } else {
            sqlx::query(
                r#"
                UPDATE users
                SET email_verified_at = CASE
                  WHEN $2 THEN COALESCE(email_verified_at, NOW())
                  ELSE email_verified_at
                END,
                updated_at = NOW()
                WHERE principal_id = $1
                "#,
            )
            .bind(principal_id)
            .bind(email_verified)
            .execute(db)
            .await?;
        }

        principal_id
    } else {
        created_account = true;
        let principal_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO principals (
              id,
              tenant_id,
              kind,
              status,
              created_at,
              updated_at
            )
            VALUES ($1, $2, 'human', 'active', NOW(), NOW())
            "#,
        )
        .bind(principal_id)
        .bind(tenant_id)
        .execute(db)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO users (
              principal_id,
              tenant_id,
              email,
              firstname,
              lastname,
              username,
              email_verified_at,
              created_at,
              updated_at
            )
            VALUES ($1, $2, $3, NULL, NULL, $4, CASE WHEN $5 THEN NOW() ELSE NULL END, NOW(), NOW())
            "#,
        )
        .bind(principal_id)
        .bind(tenant_id)
        .bind(email)
        .bind(username.trim())
        .bind(email_verified)
        .execute(db)
        .await?;

        principal_id
    };

    let membership = sqlx::query(
        r#"
        INSERT INTO tenant_memberships (
          tenant_id,
          principal_id,
          principal_kind,
          status,
          source,
          created_at,
          updated_at
        )
        VALUES ($1, $2, 'human', 'active', 'jit', NOW(), NOW())
        ON CONFLICT (tenant_id, principal_id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(db)
    .await?;

    if membership.rows_affected() > 0 {
        created_membership = true;
    }

    Ok((principal_id, created_account, created_membership))
}

pub async fn first_workspace_context(
    _db: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<(Option<Uuid>, Option<Uuid>, Option<String>), AppError> {
    let mut candidates = Vec::new();
    for workspace in workspace_port::list_tenant_workspaces(tenant_id, None, principal_id).await? {
        for member in workspace_port::list_workspace_members(
            Some(tenant_id),
            workspace.workspace_id,
            principal_id,
        )
        .await?
        {
            if member.principal_id == principal_id && member.active {
                candidates.push((
                    role_rank(&member.role),
                    member.joined_at,
                    workspace.workspace_id,
                    workspace.organization_id,
                    workspace.data_region.clone(),
                ));
            }
        }
    }
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    Ok(candidates
        .into_iter()
        .next()
        .map(|(_, _, workspace_id, organization_id, data_region)| {
            (Some(workspace_id), organization_id, data_region)
        })
        .unwrap_or((None, None, None)))
}

fn role_rank(role: &str) -> u8 {
    match role {
        "owner" => 0,
        "admin" => 1,
        "member" => 2,
        _ => 3,
    }
}
