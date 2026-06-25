use serde_json::{Value, json};

pub(crate) fn object(properties: &[(&str, Value)]) -> Value {
    let properties = properties
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    let required = properties.keys().cloned().collect::<Vec<_>>();
    json!({"type": "object", "additionalProperties": false, "properties": properties, "required": required})
}

pub(crate) fn array_schema(items: Value) -> Value {
    json!({"type": "array", "items": items})
}

pub(crate) fn ref_schema(name: &str) -> Value {
    json!({"$ref": format!("#/components/schemas/{name}")})
}

pub(crate) fn nullable_ref(name: &str) -> Value {
    json!({"anyOf": [ref_schema(name), {"type": "null"}]})
}

pub(crate) fn string_schema() -> Value {
    json!({"type": "string"})
}

pub(crate) fn number_schema() -> Value {
    json!({"type": "number"})
}

pub(crate) fn uuid_schema() -> Value {
    json!({"type": "string", "format": "uuid"})
}

pub(crate) fn nullable_uuid() -> Value {
    json!({"type": ["string", "null"], "format": "uuid"})
}

pub(crate) fn nullable_string() -> Value {
    json!({"type": ["string", "null"]})
}

pub(crate) fn date_time_schema() -> Value {
    json!({"type": "string", "format": "date-time"})
}

pub(crate) fn enum_schema(values: &[&str]) -> Value {
    json!({"type": "string", "enum": values})
}

pub(crate) fn free_object() -> Value {
    json!({"type": "object", "additionalProperties": true})
}

pub(crate) fn free_value() -> Value {
    json!({})
}
