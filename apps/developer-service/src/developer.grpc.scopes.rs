use sqlx::{Row, postgres::PgRow};
use tonic::Status;

use crate::grpc::{
    pb::nvbes::developer::v1 as developer,
    service_status::{non_empty, sql_status},
};

pub async fn list_scopes(db: &sqlx::PgPool) -> Result<developer::ListScopesResponse, Status> {
    let scopes = sqlx::query(
        r#"
        SELECT
          scope_key,
          display_name,
          description,
          risk::text AS risk,
          owner_team,
          lifecycle::text AS lifecycle,
          allowed_audiences
        FROM developer_scope_registry
        ORDER BY scope_key ASC
        "#,
    )
    .fetch_all(db)
    .await
    .map_err(sql_status)?
    .into_iter()
    .map(scope_from_row)
    .collect();

    Ok(developer::ListScopesResponse { scopes })
}

pub async fn create_scope(
    db: &sqlx::PgPool,
    request: developer::CreateScopeRequest,
) -> Result<developer::DeveloperScope, Status> {
    let scope_key = non_empty(request.scope_key, "scope_key")?;
    let risk = validate_risk(&request.risk)?;
    let lifecycle = validate_lifecycle(default_lifecycle(&request.lifecycle))?;

    sqlx::query(
        r#"
        INSERT INTO developer_scope_registry (
          scope_key,
          display_name,
          description,
          risk,
          owner_team,
          lifecycle,
          allowed_audiences
        )
        VALUES ($1, $2, $3, $4::developer_scope_risk, $5, $6::developer_scope_lifecycle, $7)
        "#,
    )
    .bind(&scope_key)
    .bind(request.display_name.trim())
    .bind(request.description.trim())
    .bind(&risk)
    .bind(request.owner_team.trim())
    .bind(&lifecycle)
    .bind(&request.allowed_audiences)
    .execute(db)
    .await
    .map_err(sql_status)?;

    Ok(developer::DeveloperScope {
        scope_key,
        display_name: request.display_name,
        description: request.description,
        risk,
        owner_team: request.owner_team,
        lifecycle,
        allowed_audiences: request.allowed_audiences,
    })
}

pub async fn update_scope(
    db: &sqlx::PgPool,
    request: developer::UpdateScopeRequest,
) -> Result<developer::DeveloperScope, Status> {
    let scope_key = non_empty(request.scope_key, "scope_key")?;
    let risk = validate_risk(&request.risk)?;
    let lifecycle = validate_lifecycle(&request.lifecycle)?;

    let updated = sqlx::query(
        r#"
        UPDATE developer_scope_registry
        SET display_name = $1,
            description = $2,
            risk = $3::developer_scope_risk,
            owner_team = $4,
            lifecycle = $5::developer_scope_lifecycle,
            allowed_audiences = $6,
            updated_at = now()
        WHERE scope_key = $7
        "#,
    )
    .bind(request.display_name.trim())
    .bind(request.description.trim())
    .bind(&risk)
    .bind(request.owner_team.trim())
    .bind(&lifecycle)
    .bind(&request.allowed_audiences)
    .bind(&scope_key)
    .execute(db)
    .await
    .map_err(sql_status)?
    .rows_affected();

    if updated == 0 {
        return Err(Status::not_found("developer scope was not found"));
    }

    Ok(developer::DeveloperScope {
        scope_key,
        display_name: request.display_name,
        description: request.description,
        risk,
        owner_team: request.owner_team,
        lifecycle,
        allowed_audiences: request.allowed_audiences,
    })
}

pub async fn delete_scope(
    db: &sqlx::PgPool,
    scope_key: String,
) -> Result<developer::ScopeDeletion, Status> {
    let scope_key = non_empty(scope_key, "scope_key")?;
    let deleted = sqlx::query("DELETE FROM developer_scope_registry WHERE scope_key = $1")
        .bind(&scope_key)
        .execute(db)
        .await
        .map_err(sql_status)?
        .rows_affected();

    if deleted == 0 {
        return Err(Status::not_found("developer scope was not found"));
    }

    Ok(developer::ScopeDeletion { scope_key })
}

fn scope_from_row(row: PgRow) -> developer::DeveloperScope {
    developer::DeveloperScope {
        scope_key: row.get("scope_key"),
        display_name: row.get("display_name"),
        description: row.get("description"),
        risk: row.get("risk"),
        owner_team: row.get("owner_team"),
        lifecycle: row.get("lifecycle"),
        allowed_audiences: row.get("allowed_audiences"),
    }
}

fn validate_risk(value: &str) -> Result<String, Status> {
    let risk = value.trim().to_lowercase();
    if !matches!(risk.as_str(), "low" | "medium" | "high" | "restricted") {
        return Err(Status::invalid_argument("Invalid risk level"));
    }
    Ok(risk)
}

fn validate_lifecycle(value: &str) -> Result<String, Status> {
    let lifecycle = value.trim().to_lowercase();
    if !matches!(
        lifecycle.as_str(),
        "proposed" | "active" | "deprecated" | "retired"
    ) {
        return Err(Status::invalid_argument("Invalid lifecycle status"));
    }
    Ok(lifecycle)
}

fn default_lifecycle(value: &str) -> &str {
    if value.trim().is_empty() {
        "proposed"
    } else {
        value
    }
}

#[cfg(test)]
#[path = "developer.grpc.scopes.contract_tests.rs"]
mod contract_tests;

#[cfg(test)]
mod tests {
    use super::{default_lifecycle, validate_lifecycle, validate_risk};

    #[test]
    fn scope_risk_validation_normalizes_known_values() {
        assert_eq!(validate_risk(" HIGH ").unwrap(), "high");
        assert_eq!(validate_risk("restricted").unwrap(), "restricted");
        assert!(validate_risk("unknown").is_err());
    }

    #[test]
    fn scope_lifecycle_validation_normalizes_known_values() {
        assert_eq!(validate_lifecycle(" Active ").unwrap(), "active");
        assert_eq!(validate_lifecycle("deprecated").unwrap(), "deprecated");
        assert!(validate_lifecycle("shipping").is_err());
    }

    #[test]
    fn empty_create_lifecycle_defaults_to_proposed() {
        assert_eq!(default_lifecycle(""), "proposed");
        assert_eq!(default_lifecycle("active"), "active");
    }
}
