use super::*;

#[test]
fn billing_export_type_parser_accepts_supported_exports() {
    assert_eq!(
        parse_finance_export_type("ledger").expect("ledger"),
        FinanceExportType::Ledger
    );
    assert!(parse_finance_export_type("provider_payloads").is_err());
}

#[test]
fn billing_export_codes_match_finance_export_names() {
    assert_eq!(export_type_code(&FinanceExportType::Invoices), "invoices");
    assert_eq!(
        export_type_code(&FinanceExportType::Subscriptions),
        "subscriptions"
    );
}
