use crate::{
    cockpit_auth::{DEFAULT_OPERATOR_ROLE, OperatorSession},
    cockpit_model::ServiceId,
    operations_db::load_case,
    operations_error::OperationsError,
    operations_model::*,
    operations_service::execute,
};
use sqlx::PgPool;
use uuid::Uuid;

fn actor() -> OperatorSession {
    OperatorSession {
        operator_id: "verified-operator".into(),
        role: DEFAULT_OPERATOR_ROLE.into(),
        has_mfa_step_up: true,
    }
}
fn command(action: Action) -> Command {
    Command {
        idempotency_key: Uuid::new_v4(),
        correlation_id: Uuid::new_v4(),
        reason: "Synthetic support verification".into(),
        action,
    }
}
fn open() -> Command {
    command(Action::OpenCase {
        category: CaseCategory::Support,
        owner: ServiceId::Account,
        subject_id: Uuid::new_v4(),
        source: "synthetic ticket".into(),
        summary: "Test support request".into(),
        related_case_id: None,
    })
}

#[sqlx::test(migrations = "./migrations")]
async fn full_case_lifecycle_retries_concurrency_and_appeal(pool: PgPool) {
    let actor = actor();
    let create = open();
    let (first, replay) = tokio::join!(
        execute(&pool, &actor, create.clone()),
        execute(&pool, &actor, create.clone())
    );
    let first = first.unwrap();
    assert_eq!(first.id, replay.unwrap().id);
    let id = first.case_id.unwrap();
    let mut changed = create;
    changed.reason = "Different payload same key".into();
    assert!(matches!(
        execute(&pool, &actor, changed).await,
        Err(OperationsError::Conflict)
    ));
    let mut version = 1;
    for status in [
        CaseStatus::Investigating,
        CaseStatus::AwaitingUser,
        CaseStatus::Investigating,
        CaseStatus::ActionPending,
        CaseStatus::Investigating,
        CaseStatus::Resolved,
        CaseStatus::Closed,
        CaseStatus::Appealed,
        CaseStatus::Investigating,
        CaseStatus::Resolved,
        CaseStatus::Closed,
    ] {
        let receipt = execute(
            &pool,
            &actor,
            command(Action::Transition {
                case_id: id,
                expected_version: version,
                status,
                evidence: "API receipt and user notification verified".into(),
            }),
        )
        .await
        .unwrap();
        version += 1;
        assert_eq!(receipt.version, Some(version));
    }
    let final_case = load_case(&pool, id).await.unwrap();
    assert_eq!(final_case.status, CaseStatus::Closed);
    let stale = command(Action::Transition {
        case_id: id,
        expected_version: 1,
        status: CaseStatus::Investigating,
        evidence: "reference".into(),
    });
    assert!(matches!(
        execute(&pool, &actor, stale).await,
        Err(OperationsError::Conflict)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM operations_audit")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, version);
    assert!(
        sqlx::query("DELETE FROM operations_audit")
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("TRUNCATE operations_audit")
            .execute(&pool)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn validation_and_audit_failure_roll_back_mutations(pool: PgPool) {
    let mut invalid = open();
    invalid.reason = " ".into();
    assert!(execute(&pool, &actor(), invalid).await.is_err());
    sqlx::raw_sql("CREATE FUNCTION reject_test_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected storage failure'; END; $$; CREATE TRIGGER reject_test_audit BEFORE INSERT ON operations_audit FOR EACH STATEMENT EXECUTE FUNCTION reject_test_audit();")
        .execute(&pool).await.unwrap();
    assert!(execute(&pool, &actor(), open()).await.is_err());
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM operations_cases")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn costs_are_corrected_without_erasing_history(pool: PgPool) {
    let record = |replaces| {
        command(Action::RecordCost {
            month: chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            provider: "Scaleway".into(),
            category: "compute".into(),
            actual_cents: 500,
            forecast_cents: 600,
            evidence: "synthetic invoice reference".into(),
            replaces,
        })
    };
    let first = execute(&pool, &actor(), record(None)).await.unwrap();
    execute(&pool, &actor(), record(first.cost_id))
        .await
        .unwrap();
    assert!(matches!(
        execute(&pool, &actor(), record(first.cost_id)).await,
        Err(OperationsError::Conflict)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM operations_costs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn observations_and_appeals_preserve_domain_and_subject_boundaries(pool: PgPool) {
    let id = execute(&pool, &actor(), open())
        .await
        .unwrap()
        .case_id
        .unwrap();
    let observation = command(Action::RecordObservation {
        case_id: id,
        expected_version: 1,
        service: ServiceId::Email,
        observed_at: chrono::Utc::now(),
        api_reference: "Email operations receipt synthetic-1".into(),
        summary: "Delivery investigated; no email content copied".into(),
    });
    execute(&pool, &actor(), observation).await.unwrap();
    let appeal = command(Action::OpenCase {
        category: CaseCategory::Appeal,
        owner: ServiceId::Account,
        subject_id: Uuid::new_v4(),
        source: "appeal ticket".into(),
        summary: "Wrong subject".into(),
        related_case_id: Some(id),
    });
    assert!(execute(&pool, &actor(), appeal).await.is_err());
    let mut unauthorized = actor();
    unauthorized.role = "billing_operator".into();
    assert!(matches!(
        execute(&pool, &unauthorized, open()).await,
        Err(OperationsError::Forbidden)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn appeal_notes_and_cost_corrections_cover_failure_branches(pool: PgPool) {
    let actor = actor();
    assert!(matches!(
        execute(
            &pool,
            &actor,
            command(Action::OpenCase {
                category: CaseCategory::Appeal,
                owner: ServiceId::Account,
                subject_id: Uuid::new_v4(),
                source: "appeal ticket".into(),
                summary: "Appeal without original case".into(),
                related_case_id: None,
            }),
        )
        .await,
        Err(OperationsError::Invalid(_))
    ));
    assert!(matches!(
        execute(
            &pool,
            &actor,
            command(Action::OpenCase {
                category: CaseCategory::Appeal,
                owner: ServiceId::Account,
                subject_id: Uuid::new_v4(),
                source: "appeal ticket".into(),
                summary: "Appeal with missing original".into(),
                related_case_id: Some(Uuid::new_v4()),
            }),
        )
        .await,
        Err(OperationsError::NotFound)
    ));

    let opened = execute(&pool, &actor, open()).await.unwrap();
    let case_id = opened.case_id.unwrap();
    execute(
        &pool,
        &actor,
        command(Action::AddNote {
            case_id,
            expected_version: 1,
            note: "Investigating synthetic support request".into(),
            evidence: "Operator verified ticket timeline".into(),
        }),
    )
    .await
    .unwrap();
    for (version, status) in [
        (2, CaseStatus::Investigating),
        (3, CaseStatus::Resolved),
        (4, CaseStatus::Closed),
    ] {
        execute(
            &pool,
            &actor,
            command(Action::Transition {
                case_id,
                expected_version: version,
                status,
                evidence: "API receipt and user notification verified".into(),
            }),
        )
        .await
        .unwrap();
    }
    assert!(matches!(
        execute(
            &pool,
            &actor,
            command(Action::AddNote {
                case_id,
                expected_version: 5,
                note: "Closed cases cannot accept notes".into(),
                evidence: "Operator attempted late note".into(),
            }),
        )
        .await,
        Err(OperationsError::Invalid(_))
    ));

    assert!(matches!(
        execute(
            &pool,
            &actor,
            command(Action::RecordCost {
                month: chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
                provider: "Scaleway".into(),
                category: "compute".into(),
                actual_cents: 100,
                forecast_cents: 150,
                evidence: "synthetic invoice reference".into(),
                replaces: Some(Uuid::new_v4()),
            }),
        )
        .await,
        Err(OperationsError::NotFound)
    ));

    let original = execute(
        &pool,
        &actor,
        command(Action::RecordCost {
            month: chrono::NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            provider: "Scaleway".into(),
            category: "compute".into(),
            actual_cents: 200,
            forecast_cents: 250,
            evidence: "synthetic invoice reference".into(),
            replaces: None,
        }),
    )
    .await
    .unwrap();
    assert!(matches!(
        execute(
            &pool,
            &actor,
            command(Action::RecordCost {
                month: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                provider: "Scaleway".into(),
                category: "compute".into(),
                actual_cents: 180,
                forecast_cents: 220,
                evidence: "synthetic invoice reference".into(),
                replaces: original.cost_id,
            }),
        )
        .await,
        Err(OperationsError::Invalid(_))
    ));
}
