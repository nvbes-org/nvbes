pub fn normalize_scopes(mut scopes: Vec<String>) -> Vec<String> {
    scopes = scopes
        .into_iter()
        .map(|scope| scope.trim().to_lowercase())
        .filter(|scope| !scope.is_empty())
        .collect();
    scopes.sort();
    scopes.dedup();
    scopes
}

pub fn normalize_resources(mut values: Vec<String>) -> Vec<String> {
    values = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect();
    values.sort();
    values.dedup();
    values
}
