use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    app::AccountState,
    audit::{self, AuditInput},
    auth::Principal,
    error::{AccountError, AccountResult},
    profile::ensure_profile,
};

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct Team {
    pub(crate) id: Uuid,
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct TeamsEnvelope {
    teams: Vec<Team>,
}

#[derive(Debug, Deserialize)]
struct CreateTeam {
    name: String,
}

#[derive(Debug, Serialize)]
struct CreatedTeam {
    id: Uuid,
    name: String,
    role: &'static str,
    join_code: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct JoinTeam {
    join_code: String,
}

#[derive(Debug, Serialize, FromRow)]
struct TeamMember {
    principal_id: Uuid,
    role: String,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct MembersEnvelope {
    members: Vec<TeamMember>,
}

pub fn router(state: AccountState) -> Router {
    Router::new()
        .route("/api/v1/teams", get(list).post(create))
        .route("/api/v1/teams/join", post(join))
        .route("/api/v1/teams/{team_id}", get(get_team))
        .route("/api/v1/teams/{team_id}/members", get(list_members))
        .route(
            "/api/v1/teams/{team_id}/members/{principal_id}",
            axum::routing::delete(remove_member),
        )
        .route("/api/v1/teams/{team_id}/leave", post(leave))
        .with_state(state)
}

async fn list(
    State(state): State<AccountState>,
    principal: Principal,
) -> AccountResult<Json<TeamsEnvelope>> {
    principal.require("account:read")?;
    let teams = sqlx::query_as::<_, Team>(
        "SELECT t.id, t.name, m.role, t.created_at FROM account_team_memberships m JOIN account_teams t ON t.id=m.team_id WHERE m.principal_id=$1 AND t.status='active' ORDER BY t.created_at DESC LIMIT 100",
    )
    .bind(principal.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(TeamsEnvelope { teams }))
}

async fn get_team(
    State(state): State<AccountState>,
    principal: Principal,
    Path(team_id): Path<Uuid>,
) -> AccountResult<Json<Team>> {
    principal.require("account:read")?;
    let team = sqlx::query_as::<_, Team>(
        "SELECT t.id, t.name, m.role, t.created_at FROM account_team_memberships m JOIN account_teams t ON t.id=m.team_id WHERE m.principal_id=$1 AND t.id=$2 AND t.status='active'",
    )
    .bind(principal.id)
    .bind(team_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AccountError::NotFound)?;
    Ok(Json(team))
}

async fn list_members(
    State(state): State<AccountState>,
    principal: Principal,
    Path(team_id): Path<Uuid>,
) -> AccountResult<Json<MembersEnvelope>> {
    principal.require("account:read")?;
    require_active_membership(&state.db, team_id, principal.id).await?;
    let members = sqlx::query_as::<_, TeamMember>(
        "SELECT principal_id, role, created_at FROM account_team_memberships WHERE team_id=$1 ORDER BY created_at ASC LIMIT 200",
    )
    .bind(team_id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(MembersEnvelope { members }))
}

async fn create(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Json(input): Json<CreateTeam>,
) -> AccountResult<(StatusCode, Json<CreatedTeam>)> {
    principal.require("account:write")?;
    let name = input.name.trim();
    if !(1..=100).contains(&name.len()) {
        return Err(AccountError::Invalid("team name length"));
    }
    ensure_profile(&state.db, principal.id).await?;
    let id = Uuid::new_v4();
    let join_code = format!("team_{}", Uuid::new_v4().simple());
    let join_hash = hash_join_code(&join_code);
    let correlation_id = audit::correlation_id(&headers);
    let mut tx = state.db.begin().await?;
    let created_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO account_teams(id, owner_principal_id, name, join_code_hash) VALUES($1,$2,$3,$4) RETURNING created_at",
    )
    .bind(id)
    .bind(principal.id)
    .bind(name)
    .bind(join_hash.as_slice())
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO account_team_memberships(team_id, principal_id, role) VALUES($1,$2,'owner')",
    )
    .bind(id)
    .bind(principal.id)
    .execute(&mut *tx)
    .await?;
    audit::record(
        &mut tx,
        AuditInput {
            principal_id: principal.id,
            actor_principal_id: principal.id,
            event_type: "account.team.created",
            resource_type: "team",
            resource_id: Some(id),
            correlation_id,
            details: json!({}),
        },
    )
    .await?;
    audit::enqueue(
        &mut tx,
        "account.team.created.v1",
        id,
        json!({"team_id": id, "owner_principal_id": principal.id}),
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedTeam {
            id,
            name: name.into(),
            role: "owner",
            join_code,
            created_at,
        }),
    ))
}

async fn join(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Json(input): Json<JoinTeam>,
) -> AccountResult<Json<Team>> {
    principal.require("account:write")?;
    let team = join_team(
        &state.db,
        principal.id,
        &input.join_code,
        audit::correlation_id(&headers),
    )
    .await?;
    Ok(Json(team))
}

async fn leave(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Path(team_id): Path<Uuid>,
) -> AccountResult<StatusCode> {
    principal.require("account:write")?;
    leave_team(
        &state.db,
        team_id,
        principal.id,
        principal.id,
        audit::correlation_id(&headers),
        true,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_member(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Path((team_id, member_id)): Path<(Uuid, Uuid)>,
) -> AccountResult<StatusCode> {
    principal.require("account:write")?;
    if member_id == principal.id {
        return Err(AccountError::Invalid("use leave to remove yourself"));
    }
    let role = require_active_membership(&state.db, team_id, principal.id).await?;
    if role != "owner" {
        return Err(AccountError::Forbidden);
    }
    leave_team(
        &state.db,
        team_id,
        member_id,
        principal.id,
        audit::correlation_id(&headers),
        false,
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn join_team(
    db: &PgPool,
    principal_id: Uuid,
    join_code: &str,
    correlation_id: Uuid,
) -> AccountResult<Team> {
    if !join_code.starts_with("team_") || join_code.len() != 37 {
        return Err(AccountError::NotFound);
    }
    ensure_profile(db, principal_id).await?;
    let hash = hash_join_code(join_code);
    let mut tx = db.begin().await?;
    let (team_id, name, created_at): (Uuid, String, DateTime<Utc>) = sqlx::query_as(
        "SELECT id, name, created_at FROM account_teams WHERE join_code_hash=$1 AND status='active' FOR UPDATE",
    )
    .bind(hash.as_slice())
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AccountError::NotFound)?;
    let inserted = sqlx::query("INSERT INTO account_team_memberships(team_id, principal_id, role) VALUES($1,$2,'member') ON CONFLICT DO NOTHING")
        .bind(team_id).bind(principal_id).execute(&mut *tx).await?.rows_affected() == 1;
    if inserted {
        audit::record(
            &mut tx,
            AuditInput {
                principal_id,
                actor_principal_id: principal_id,
                event_type: "account.team.joined",
                resource_type: "team",
                resource_id: Some(team_id),
                correlation_id,
                details: json!({}),
            },
        )
        .await?;
        audit::enqueue(
            &mut tx,
            "account.team.member_joined.v1",
            team_id,
            json!({"team_id": team_id, "principal_id": principal_id}),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Team {
        id: team_id,
        name,
        role: if inserted {
            "member".to_owned()
        } else {
            membership_role(db, team_id, principal_id).await?
        },
        created_at,
    })
}

async fn leave_team(
    db: &PgPool,
    team_id: Uuid,
    principal_id: Uuid,
    actor_principal_id: Uuid,
    correlation_id: Uuid,
    allow_owner_close: bool,
) -> AccountResult<()> {
    let mut tx = db.begin().await?;
    let role: String = sqlx::query_scalar(
        "SELECT m.role FROM account_team_memberships m JOIN account_teams t ON t.id=m.team_id WHERE m.team_id=$1 AND m.principal_id=$2 AND t.status='active' FOR UPDATE OF m",
    )
    .bind(team_id)
    .bind(principal_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AccountError::NotFound)?;
    if role == "owner" {
        if !allow_owner_close {
            return Err(AccountError::Conflict);
        }
        let members: i64 =
            sqlx::query_scalar("SELECT count(*) FROM account_team_memberships WHERE team_id=$1")
                .bind(team_id)
                .fetch_one(&mut *tx)
                .await?;
        if members > 1 {
            return Err(AccountError::Conflict);
        }
        sqlx::query(
            "UPDATE account_teams SET status='closed', updated_at=clock_timestamp() WHERE id=$1",
        )
        .bind(team_id)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("DELETE FROM account_team_memberships WHERE team_id=$1 AND principal_id=$2")
        .bind(team_id)
        .bind(principal_id)
        .execute(&mut *tx)
        .await?;
    audit::record(
        &mut tx,
        AuditInput {
            principal_id,
            actor_principal_id,
            event_type: "account.team.left",
            resource_type: "team",
            resource_id: Some(team_id),
            correlation_id,
            details: json!({"role": role, "closed_team": role == "owner"}),
        },
    )
    .await?;
    audit::enqueue(
        &mut tx,
        "account.team.member_left.v1",
        team_id,
        json!({
            "team_id": team_id,
            "principal_id": principal_id,
            "actor_principal_id": actor_principal_id
        }),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn require_active_membership(
    db: &PgPool,
    team_id: Uuid,
    principal_id: Uuid,
) -> AccountResult<String> {
    sqlx::query_scalar(
        "SELECT m.role FROM account_team_memberships m JOIN account_teams t ON t.id=m.team_id WHERE m.team_id=$1 AND m.principal_id=$2 AND t.status='active'",
    )
    .bind(team_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await?
    .ok_or(AccountError::NotFound)
}

async fn membership_role(db: &PgPool, team_id: Uuid, principal_id: Uuid) -> AccountResult<String> {
    sqlx::query_scalar(
        "SELECT role FROM account_team_memberships WHERE team_id=$1 AND principal_id=$2",
    )
    .bind(team_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await?
    .ok_or(AccountError::Conflict)
}

fn hash_join_code(code: &str) -> [u8; 32] {
    Sha256::digest(code.as_bytes()).into()
}

pub async fn create_and_join_for_synthetic(
    db: &PgPool,
    owner: Uuid,
    member: Uuid,
) -> AccountResult<(Uuid, String)> {
    ensure_profile(db, owner).await?;
    let id = Uuid::new_v4();
    let join_code = format!("team_{}", Uuid::new_v4().simple());
    let hash = hash_join_code(&join_code);
    let mut tx = db.begin().await?;
    sqlx::query("INSERT INTO account_teams(id, owner_principal_id, name, join_code_hash) VALUES($1,$2,'Synthetic team',$3)").bind(id).bind(owner).bind(hash.as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO account_team_memberships(team_id,principal_id,role) VALUES($1,$2,'owner')",
    )
    .bind(id)
    .bind(owner)
    .execute(&mut *tx)
    .await?;
    audit::record(
        &mut tx,
        AuditInput {
            principal_id: owner,
            actor_principal_id: owner,
            event_type: "account.team.created",
            resource_type: "team",
            resource_id: Some(id),
            correlation_id: Uuid::new_v4(),
            details: json!({"source": "synthetic"}),
        },
    )
    .await?;
    audit::enqueue(
        &mut tx,
        "account.team.created.v1",
        id,
        json!({"team_id": id, "owner_principal_id": owner}),
    )
    .await?;
    tx.commit().await?;
    let joined = join_team(db, member, &join_code, Uuid::new_v4()).await?;
    Ok((id, joined.role))
}

pub async fn leave_for_synthetic(db: &PgPool, team_id: Uuid, member: Uuid) -> AccountResult<()> {
    leave_team(db, team_id, member, member, Uuid::new_v4(), true).await
}

#[cfg(test)]
#[path = "account.teams.tests.rs"]
mod tests;
