use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    ProForma,
    Issued,
    Paid,
    Void,
    Refunded,
    WrittenOff,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub line_type: String,
    pub description: String,
    pub quantity: i64,
    pub unit_amount_minor: i64,
    pub amount_minor: i64,
    pub tax_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceTotals {
    pub subtotal_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvoiceDocument {
    pub invoice_number: String,
    pub issue_date: String,
    pub seller_name: String,
    pub customer_name: String,
    pub customer_vat_id: Option<String>,
    pub lines: Vec<InvoiceLine>,
    pub totals: InvoiceTotals,
    pub currency: String,
}

pub fn calculate_invoice_totals(lines: &[InvoiceLine]) -> InvoiceTotals {
    let subtotal_minor = lines.iter().map(|line| line.amount_minor).sum();
    let tax_minor = lines.iter().map(|line| line.tax_minor).sum();
    InvoiceTotals {
        subtotal_minor,
        tax_minor,
        total_minor: subtotal_minor + tax_minor,
    }
}

pub fn next_invoice_number(legal_entity: &str, year: i32, sequence: i64) -> String {
    format!("{legal_entity}-{year}-{sequence:06}")
}

pub fn invoice_line(
    line_type: &str,
    description: &str,
    quantity: i64,
    unit_amount_minor: i64,
    tax_minor: i64,
) -> InvoiceLine {
    InvoiceLine {
        line_type: line_type.to_string(),
        description: description.to_string(),
        quantity,
        unit_amount_minor,
        amount_minor: quantity * unit_amount_minor,
        tax_minor,
    }
}

pub fn invoice_document_text(document: &InvoiceDocument) -> String {
    let mut output = format!(
        "Invoice {}\nIssue date: {}\nSeller: {}\nCustomer: {}\n",
        document.invoice_number, document.issue_date, document.seller_name, document.customer_name
    );
    if let Some(vat_id) = &document.customer_vat_id {
        output.push_str(&format!("VAT ID: {vat_id}\n"));
    }
    for line in &document.lines {
        output.push_str(&format!(
            "{} | {} | qty={} | unit={} | amount={} | tax={}\n",
            line.line_type,
            line.description,
            line.quantity,
            line.unit_amount_minor,
            line.amount_minor,
            line.tax_minor
        ));
    }
    output.push_str(&format!(
        "Subtotal: {} {}\nTax: {} {}\nTotal: {} {}\n",
        document.totals.subtotal_minor,
        document.currency,
        document.totals.tax_minor,
        document.currency,
        document.totals.total_minor,
        document.currency
    ));
    output
}

pub fn invoice_document_pdf_bytes(document: &InvoiceDocument) -> Vec<u8> {
    let text = invoice_document_text(document);
    simple_pdf_from_text(&text)
}

fn simple_pdf_from_text(text: &str) -> Vec<u8> {
    let lines = text.lines().take(42).collect::<Vec<_>>();
    let mut content = String::from("BT\n/F1 10 Tf\n50 790 Td\n14 TL\n");
    for line in lines {
        content.push('(');
        content.push_str(&escape_pdf_text(line));
        content.push_str(") Tj\nT*\n");
    }
    content.push_str("ET\n");

    let objects = [
        "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string(),
        "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n".to_string(),
        "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n".to_string(),
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n".to_string(),
        format!(
            "5 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
            content.len(),
            content
        ),
    ];

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0_usize];
    for object in &objects {
        offsets.push(pdf.len());
        pdf.extend_from_slice(object.as_bytes());
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(),
            xref_offset
        )
        .as_bytes(),
    );
    pdf
}

fn escape_pdf_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii())
        .flat_map(|ch| match ch {
            '(' => "\\(".chars().collect::<Vec<_>>(),
            ')' => "\\)".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            _ => vec![ch],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoice_total_is_reconstructible_from_lines() {
        let totals = calculate_invoice_totals(&[
            InvoiceLine {
                line_type: "plan".to_string(),
                description: "Team".to_string(),
                quantity: 1,
                unit_amount_minor: 3_900,
                amount_minor: 3_900,
                tax_minor: 780,
            },
            InvoiceLine {
                line_type: "credit".to_string(),
                description: "Launch credit".to_string(),
                quantity: 1,
                unit_amount_minor: 0,
                amount_minor: 0,
                tax_minor: 0,
            },
        ]);
        assert_eq!(totals.total_minor, 4_680);
    }

    #[test]
    fn generated_line_captures_quantity_unit_and_total() {
        let line = invoice_line("addon", "Extra seats", 3, 900, 540);
        assert_eq!(line.amount_minor, 2_700);
        assert_eq!(line.tax_minor, 540);
    }

    #[test]
    fn next_invoice_number_is_zero_padded() {
        assert_eq!(next_invoice_number("NVBES", 2026, 42), "NVBES-2026-000042");
    }

    #[test]
    fn invoice_pdf_escapes_backslash_in_customer_name() {
        let lines = vec![invoice_line("plan", "Team", 1, 3_900, 780)];
        let document = InvoiceDocument {
            invoice_number: "NVBES-2026-000001".to_string(),
            issue_date: "2026-06-21".to_string(),
            seller_name: "nvbes".to_string(),
            customer_name: r"Acme\Corp".to_string(),
            customer_vat_id: None,
            totals: calculate_invoice_totals(&lines),
            lines,
            currency: "EUR".to_string(),
        };
        let pdf = invoice_document_pdf_bytes(&document);
        let text = String::from_utf8_lossy(&pdf);
        assert!(
            text.contains(r"Acme\\Corp"),
            "backslash must be escaped in PDF text"
        );
    }

    #[test]
    fn invoice_document_is_regenerable_from_canonical_data() {
        let lines = vec![invoice_line("plan", "Team", 1, 3_900, 780)];
        let document = InvoiceDocument {
            invoice_number: "NVBES-2026-000001".to_string(),
            issue_date: "2026-06-21".to_string(),
            seller_name: "nvbes".to_string(),
            customer_name: "Acme".to_string(),
            customer_vat_id: Some("FR123".to_string()),
            totals: calculate_invoice_totals(&lines),
            lines,
            currency: "EUR".to_string(),
        };

        let text = invoice_document_text(&document);
        assert!(text.contains("NVBES-2026-000001"));
        assert!(text.contains("VAT ID: FR123"));
        assert!(text.contains("Total: 4680 EUR"));
    }

    #[test]
    fn invoice_pdf_is_valid_pdf_container_from_canonical_data() {
        let lines = vec![invoice_line("plan", "Team (EU)", 1, 3_900, 780)];
        let document = InvoiceDocument {
            invoice_number: "NVBES-2026-000001".to_string(),
            issue_date: "2026-06-21".to_string(),
            seller_name: "nvbes".to_string(),
            customer_name: "Acme".to_string(),
            customer_vat_id: Some("FR123".to_string()),
            totals: calculate_invoice_totals(&lines),
            lines,
            currency: "EUR".to_string(),
        };

        let pdf = invoice_document_pdf_bytes(&document);
        let pdf_text = String::from_utf8_lossy(&pdf);
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf_text.contains("NVBES-2026-000001"));
        assert!(pdf_text.contains("Team \\(EU\\)"));
        assert!(pdf_text.contains("%%EOF"));
    }

    #[test]
    fn invoice_document_text_omits_vat_id_when_absent() {
        let lines = vec![invoice_line("plan", "Team", 1, 3_900, 780)];
        let document = InvoiceDocument {
            invoice_number: "NVBES-2026-000002".to_string(),
            issue_date: "2026-06-21".to_string(),
            seller_name: "nvbes".to_string(),
            customer_name: "Acme".to_string(),
            customer_vat_id: None,
            totals: calculate_invoice_totals(&lines),
            lines,
            currency: "EUR".to_string(),
        };
        let text = invoice_document_text(&document);
        assert!(!text.contains("VAT ID:"));
        assert!(text.contains("NVBES-2026-000002"));
    }
}
