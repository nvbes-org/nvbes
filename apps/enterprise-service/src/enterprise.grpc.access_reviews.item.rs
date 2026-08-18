use sqlx::{Row, postgres::PgRow};
use uuid::Uuid;

pub(crate) struct AccessReviewItemRow {
    pub(super) id: Uuid,
    pub(super) item_type: String,
    pub(super) subject_id: String,
    pub(super) workspace_id: Option<Uuid>,
    pub(super) decision: String,
}

impl AccessReviewItemRow {
    pub(super) fn from_row(row: PgRow) -> Result<Self, sqlx::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            item_type: row.try_get("item_type")?,
            subject_id: row.try_get("subject_id")?,
            workspace_id: row.try_get("workspace_id")?,
            decision: row.try_get("decision")?,
        })
    }
}
