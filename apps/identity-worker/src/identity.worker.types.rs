use serde_json::Value;

#[derive(sqlx::FromRow)]
pub(crate) struct WorkerJob {
    pub id: uuid::Uuid,
    pub job_type: String,
    pub payload: Value,
}
