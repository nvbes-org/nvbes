use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    Open,
    InReview,
    Resolved,
    Inconclusive,
}

impl ReviewState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InReview => "in_review",
            Self::Resolved => "resolved",
            Self::Inconclusive => "inconclusive",
        }
    }

    fn parse(value: &str) -> Result<Self, ReviewError> {
        match value {
            "open" => Ok(Self::Open),
            "in_review" => Ok(Self::InReview),
            "resolved" => Ok(Self::Resolved),
            "inconclusive" => Ok(Self::Inconclusive),
            _ => Err(ReviewError::CorruptState),
        }
    }

    pub fn permits(self, target: Self, has_authoritative_label: bool) -> bool {
        matches!(
            (self, target),
            (Self::Open, Self::InReview)
                | (Self::InReview, Self::Inconclusive)
                | (Self::InReview, Self::Resolved) if target != Self::Resolved || has_authoritative_label
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReviewCase {
    pub id: Uuid,
    pub evaluation_id: Uuid,
    pub state: ReviewState,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn ensure_case_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    evaluation_id: Uuid,
    retention_days: u32,
) -> Result<Uuid, sqlx::Error> {
    let id = Uuid::new_v4();
    let review_id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO trust_risk_review_cases (id, evaluation_id, state, expires_at)
        VALUES ($1, $2, 'open', clock_timestamp() + ($3 * INTERVAL '1 day'))
        ON CONFLICT (evaluation_id) DO UPDATE SET evaluation_id = EXCLUDED.evaluation_id
        RETURNING id
        "#,
    )
    .bind(id)
    .bind(evaluation_id)
    .bind(i64::from(retention_days))
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO trust_risk_review_events (review_case_id, from_state, to_state, actor, reason)
        SELECT $1, NULL, 'open', 'trust-risk-service', 'risk recommendation requires review'
        WHERE NOT EXISTS (SELECT 1 FROM trust_risk_review_events WHERE review_case_id = $1)
        "#,
    )
    .bind(review_id)
    .execute(&mut **tx)
    .await?;
    Ok(review_id)
}

pub async fn transition(
    pool: &PgPool,
    id: Uuid,
    target: ReviewState,
    assign_to: Option<String>,
    actor: &str,
    reason: &str,
) -> Result<ReviewCase, ReviewError> {
    if actor.len() < 3 || reason.trim().len() < 3 || reason.len() > 300 {
        return Err(ReviewError::InvalidInput);
    }
    let mut tx = pool.begin().await?;
    let row = sqlx::query_as::<_, (Uuid, String, Option<String>, DateTime<Utc>, DateTime<Utc>)>(
        "SELECT evaluation_id, state, assigned_to, created_at, updated_at FROM trust_risk_review_cases WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ReviewError::NotFound)?;
    let current = ReviewState::parse(&row.1)?;
    let has_authoritative_label = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM trust_risk_canonical_labels AS canonical
            JOIN trust_risk_labels AS label ON label.id = canonical.label_id
            WHERE canonical.evaluation_id = $1 AND label.source_class IN (1, 2, 3)
        )
        "#,
    )
    .bind(row.0)
    .fetch_one(&mut *tx)
    .await?;
    if !current.permits(target, has_authoritative_label) {
        return Err(ReviewError::InvalidTransition);
    }
    let updated_at = sqlx::query_scalar::<_, DateTime<Utc>>(
        "UPDATE trust_risk_review_cases SET state = $2, assigned_to = COALESCE($3, assigned_to), updated_at = clock_timestamp() WHERE id = $1 RETURNING updated_at",
    )
    .bind(id).bind(target.as_str()).bind(&assign_to).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO trust_risk_review_events (review_case_id, from_state, to_state, actor, reason) VALUES ($1, $2, $3, $4, $5)")
        .bind(id).bind(current.as_str()).bind(target.as_str()).bind(actor).bind(reason.trim())
        .execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(ReviewCase {
        id,
        evaluation_id: row.0,
        state: target,
        assigned_to: assign_to.or(row.2),
        created_at: row.3,
        updated_at,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum ReviewError {
    #[error("review case was not found")]
    NotFound,
    #[error("review transition is not permitted")]
    InvalidTransition,
    #[error("review request is invalid")]
    InvalidInput,
    #[error("review state is invalid")]
    CorruptState,
    #[error("review persistence failed")]
    Database(#[from] sqlx::Error),
}

#[cfg(test)]
#[path = "trust_risk.review.tests.rs"]
mod tests;
