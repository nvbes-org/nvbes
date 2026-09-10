use std::collections::HashMap;

use serde_json::Value;

/// Substitutes variable references in a string value.
///
/// Supported patterns:
/// - `${VAR}` — environment variable or context variable
/// - `${LAST.field}` — field from the last step's output
/// - `${STEP[n].field}` — field from step n's output (0-indexed)
/// - `${UNIQUE}` — random unique string (nanoid-like)
/// - `${UUID}` — random UUID v4
pub fn substitute(input: &str, ctx: &VariableContext) -> String {
    let mut result = input.to_string();

    // ${UUID}
    if result.contains("${UUID}") {
        result = result.replace("${UUID}", &uuid::Uuid::new_v4().to_string());
    }

    // ${UNIQUE}
    if result.contains("${UNIQUE}") {
        result = result.replace("${UNIQUE}", &unique_string());
    }

    // ${VAR} — env or context
    for cap in variable_refs(&result) {
        let value = resolve_variable(&cap, ctx);
        result = result.replace(&format!("${{{cap}}}"), &value);
    }

    result
}

/// Substitutes variables in a JSON value recursively.
pub fn substitute_value(value: &Value, ctx: &VariableContext) -> Value {
    match value {
        Value::String(s) => Value::String(substitute(s, ctx)),
        Value::Array(arr) => Value::Array(arr.iter().map(|v| substitute_value(v, ctx)).collect()),
        Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (k, v) in map {
                new_map.insert(substitute(k, ctx), substitute_value(v, ctx));
            }
            Value::Object(new_map)
        }
        other => other.clone(),
    }
}

#[derive(Clone)]
pub struct VariableContext {
    pub env: HashMap<String, String>,
    pub step_outputs: Vec<HashMap<String, Value>>,
}

impl VariableContext {
    pub fn new(env: HashMap<String, String>) -> Self {
        Self {
            env,
            step_outputs: Vec::new(),
        }
    }

    pub fn push_output(&mut self, output: HashMap<String, Value>) {
        self.step_outputs.push(output);
    }

    pub fn last_output(&self) -> Option<&HashMap<String, Value>> {
        self.step_outputs.last()
    }

    pub fn step_output(&self, index: usize) -> Option<&HashMap<String, Value>> {
        self.step_outputs.get(index)
    }
}

fn variable_refs(input: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if c == '$' && chars.peek().map(|&(_, ch)| ch) == Some('{') {
            chars.next(); // skip '{'
            let start = i + 2;
            let mut depth = 1;
            let mut end = start;
            for (j, ch) in chars.by_ref() {
                match ch {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = j;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if depth == 0 {
                refs.push(input[start..end].to_string());
            }
        }
    }
    refs
}

fn resolve_variable(name: &str, ctx: &VariableContext) -> String {
    if let Some(pos) = name.strip_prefix("LAST.")
        && let Some(last) = ctx.last_output()
    {
        return extract_field(last, pos);
    }
    if let Some(rest) = name.strip_prefix("STEP[")
        && let Some(close) = rest.find(']')
        && let Ok(index) = rest[..close].parse::<usize>()
    {
        let field = &rest[close + 1..].strip_prefix('.').unwrap_or("");
        if let Some(output) = ctx.step_output(index) {
            return extract_field(output, field);
        }
    }
    if let Some(val) = ctx.env.get(name) {
        return val.clone();
    }
    std::env::var(name).unwrap_or_else(|_| format!("${{{name}}}"))
}

fn extract_field(map: &HashMap<String, Value>, field: &str) -> String {
    let mut current = Value::Object(map.clone().into_iter().collect());
    for part in field.split('.') {
        current = match &current {
            Value::Object(obj) => match obj.get(part) {
                Some(value) => value.clone(),
                None => return format!("${{{field}}}"),
            },
            _ => return format!("${{{field}}}"),
        };
    }
    match current {
        Value::String(s) => s,
        other => other.to_string(),
    }
}

fn unique_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{ts:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_uuid() {
        let result = substitute("${UUID}", &VariableContext::new(HashMap::new()));
        assert!(uuid::Uuid::parse_str(&result).is_ok());
    }

    #[test]
    fn substitutes_unique() {
        let a = substitute("${UNIQUE}", &VariableContext::new(HashMap::new()));
        let b = substitute("${UNIQUE}", &VariableContext::new(HashMap::new()));
        assert_ne!(a, b);
    }

    #[test]
    fn substitutes_env_var() {
        let mut env = HashMap::new();
        env.insert("MY_VAR".to_string(), "hello".to_string());
        let result = substitute("${MY_VAR}", &VariableContext::new(env));
        assert_eq!(result, "hello");
    }

    #[test]
    fn substitutes_last_field() {
        let mut ctx = VariableContext::new(HashMap::new());
        let mut output = HashMap::new();
        output.insert("key".to_string(), Value::String("test:123".to_string()));
        ctx.push_output(output);
        let result = substitute("${LAST.key}", &ctx);
        assert_eq!(result, "test:123");
    }

    #[test]
    fn substitutes_step_index() {
        let mut ctx = VariableContext::new(HashMap::new());
        let mut out0 = HashMap::new();
        out0.insert("id".to_string(), Value::String("first".to_string()));
        ctx.push_output(out0);
        let mut out1 = HashMap::new();
        out1.insert("ref".to_string(), Value::String("second".to_string()));
        ctx.push_output(out1);
        assert_eq!(substitute("${STEP[0].id}", &ctx), "first");
        assert_eq!(substitute("${STEP[1].ref}", &ctx), "second");
    }

    #[test]
    fn substitutes_nested_object() {
        let mut ctx = VariableContext::new(HashMap::new());
        let mut output = HashMap::new();
        let mut body = serde_json::Map::new();
        body.insert("token".to_string(), Value::String("abc".to_string()));
        output.insert("body".to_string(), Value::Object(body));
        ctx.push_output(output);
        let result = substitute("${LAST.body.token}", &ctx);
        assert_eq!(result, "abc");
    }

    #[test]
    fn substitutes_in_json_value() {
        let mut env = HashMap::new();
        env.insert("HOST".to_string(), "localhost".to_string());
        let ctx = VariableContext::new(env);
        let input = serde_json::json!({"url": "http://${HOST}:3000"});
        let result = substitute_value(&input, &ctx);
        assert_eq!(result["url"], "http://localhost:3000");
    }
}
