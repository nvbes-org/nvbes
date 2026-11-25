use super::*;

pub(super) fn parse_context(value: Option<&Value>) -> Result<Option<RequestContext>, KeywordError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let object = value
        .as_object()
        .ok_or_else(|| KeywordError::InvalidParams("context must be an object".to_string()))?;
    let tenant = object
        .get("tenant")
        .map(|tenant_value| {
            let tenant_object = tenant_value.as_object().ok_or_else(|| {
                KeywordError::InvalidParams("context.tenant must be an object".to_string())
            })?;
            Ok(TenantContext {
                tenant_id: string_field(tenant_object, "tenant_id", String::new),
                workspace_id: string_field(tenant_object, "workspace_id", String::new),
                region_id: string_field(tenant_object, "region_id", String::new),
                data_residency: string_field(tenant_object, "data_residency", String::new),
            })
        })
        .transpose()?;
    Ok(Some(RequestContext {
        request_id: string_field(object, "request_id", String::new),
        correlation_id: string_field(object, "correlation_id", String::new),
        actor_principal_id: string_field(object, "actor_principal_id", String::new),
        tenant,
    }))
}

pub(super) fn parse_assessment(
    params: &HashMap<String, Value>,
) -> Result<AssessRiskRequest, KeywordError> {
    let subjects = if let Some(value) = params.get("subjects") {
        parse_subjects(Some(value))?
    } else {
        Vec::new()
    };
    let instantaneous_signals = if let Some(Value::Array(signals)) = params.get("signals") {
        signals
            .iter()
            .map(signal_from_value)
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    Ok(AssessRiskRequest {
        context: parse_context(params.get("context"))?,
        producer: required_string_field_str(params, "producer")?,
        assessment_key: required_string_field_str(params, "assessment_key")?,
        operation_class: required_string_field_str(params, "operation_class")?,
        subjects,
        instantaneous_signals,
    })
}

pub(super) fn parse_labels(
    params: &HashMap<String, Value>,
) -> Result<Vec<RiskLabel>, KeywordError> {
    let array = params
        .get("labels")
        .and_then(Value::as_array)
        .ok_or_else(|| KeywordError::InvalidParams("labels must be an array".to_string()))?;
    array.iter().map(label_from_value).collect()
}

pub(super) fn label_from_value(value: &Value) -> Result<RiskLabel, KeywordError> {
    let object = value
        .as_object()
        .ok_or_else(|| KeywordError::InvalidParams("label must be an object".to_string()))?;
    Ok(RiskLabel {
        label_id: string_field(object, "label_id", || Uuid::new_v4().to_string()),
        schema_version: uint_field(object, "schema_version", 1) as u32,
        producer: required_string_field(object, "producer")?,
        evaluation_id: required_string_field(object, "evaluation_id")?,
        review_case_id: optional_string_field(object, "review_case_id"),
        kind: parse_enum_i32(
            object.get("kind"),
            RiskLabelKind::Unspecified as i32,
            &[
                ("legitimate", RiskLabelKind::Legitimate as i32),
                ("confirmed_fraud", RiskLabelKind::ConfirmedFraud as i32),
                ("bot", RiskLabelKind::Bot as i32),
                ("account_takeover", RiskLabelKind::AccountTakeover as i32),
                ("chargeback", RiskLabelKind::Chargeback as i32),
                ("false_positive", RiskLabelKind::FalsePositive as i32),
            ],
            "label kind",
        )?,
        source_class: parse_enum_i32(
            object.get("source_class"),
            LabelSourceClass::Unspecified as i32,
            &[
                ("human", LabelSourceClass::Human as i32),
                (
                    "authoritative_external",
                    LabelSourceClass::AuthoritativeExternal as i32,
                ),
                ("verified_product", LabelSourceClass::VerifiedProduct as i32),
                ("heuristic", LabelSourceClass::Heuristic as i32),
            ],
            "label source_class",
        )?,
        source_id: required_string_field(object, "source_id")?,
        confidence: required_f64_field(object, "confidence")?,
        actor: optional_string_field(object, "actor"),
        knowledge_at: parse_timestamp(object.get("knowledge_at"))?,
        evidence_reference: optional_string_field(object, "evidence_reference"),
        mapping_version: string_field(object, "mapping_version", || "v1".to_string()),
        corrects_label_id: optional_string_field(object, "corrects_label_id"),
    })
}
