use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinanceExportType {
    Invoices,
    Payments,
    Tax,
    Ledger,
    Customers,
    Subscriptions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinanceExportRow {
    pub columns: Vec<String>,
}

pub fn export_filename(export_type: FinanceExportType, period: &str) -> String {
    let name = match export_type {
        FinanceExportType::Invoices => "invoices",
        FinanceExportType::Payments => "payments",
        FinanceExportType::Tax => "tax",
        FinanceExportType::Ledger => "ledger",
        FinanceExportType::Customers => "customers",
        FinanceExportType::Subscriptions => "subscriptions",
    };
    format!("billing-{name}-{period}.csv")
}

pub fn csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn stable_csv(headers: &[&str], rows: &[FinanceExportRow]) -> String {
    let mut output = String::new();
    output.push_str(
        &headers
            .iter()
            .map(|value| csv_escape(value))
            .collect::<Vec<_>>()
            .join(","),
    );
    output.push('\n');
    for row in rows {
        output.push_str(
            &row.columns
                .iter()
                .map(|value| csv_escape(value))
                .collect::<Vec<_>>()
                .join(","),
        );
        output.push('\n');
    }
    output
}

pub fn export_projection_excludes_sensitive_payloads(columns: &[&str]) -> bool {
    !columns.iter().any(|column| {
        matches!(
            *column,
            "payload" | "raw_payload" | "provider_secret" | "signature"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_csv_is_stable_and_escaped() {
        let csv = stable_csv(
            &["invoice_id", "description"],
            &[FinanceExportRow {
                columns: vec!["inv_1".to_string(), "Plan, Team".to_string()],
            }],
        );

        assert_eq!(csv, "invoice_id,description\ninv_1,\"Plan, Team\"\n");
    }

    #[test]
    fn export_filename_encodes_type_and_period() {
        assert_eq!(
            export_filename(FinanceExportType::Invoices, "2026-09"),
            "billing-invoices-2026-09.csv"
        );
        assert_eq!(
            export_filename(FinanceExportType::Ledger, "2026-Q3"),
            "billing-ledger-2026-Q3.csv"
        );
    }

    #[test]
    fn dashboard_projection_does_not_read_sensitive_payloads() {
        assert!(export_projection_excludes_sensitive_payloads(&[
            "tenant_id",
            "amount_minor"
        ]));
        assert!(!export_projection_excludes_sensitive_payloads(&[
            "tenant_id",
            "raw_payload"
        ]));
    }
}
