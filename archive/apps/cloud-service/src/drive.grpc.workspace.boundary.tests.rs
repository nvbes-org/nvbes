const WORKSPACES: &str = include_str!("drive.grpc.workspace.rs");
const MEMBERS: &str = include_str!("drive.grpc.workspace.members.rs");
const INVITATIONS: &str = include_str!("drive.grpc.workspace.invitations.rs");
const PERSISTENCE: &str = include_str!("drive.grpc.workspace.persistence.rs");

#[test]
fn grpc_workspace_operations_are_transaction_bound() {
    for (source, expected_operations) in [(WORKSPACES, 5), (MEMBERS, 4), (INVITATIONS, 4)] {
        assert_eq!(
            source.matches("begin_request_transaction(").count(),
            expected_operations,
            "every implemented workspace RPC must establish one scoped transaction"
        );
        for forbidden in [
            ".execute(db)",
            ".fetch_all(db)",
            ".fetch_one(db)",
            ".fetch_optional(db)",
            "db.begin()",
        ] {
            assert!(
                !source.contains(forbidden),
                "workspace RPC bypasses the scoped transaction with {forbidden}"
            );
        }
    }
}

#[test]
fn grpc_workspace_fetch_helpers_require_a_transaction() {
    assert!(!PERSISTENCE.contains("db: &sqlx::PgPool"));
    assert!(!PERSISTENCE.contains(".fetch_optional(db)"));
    assert!(
        PERSISTENCE
            .matches("Transaction<'_, sqlx::Postgres>")
            .count()
            >= 6
    );
}
