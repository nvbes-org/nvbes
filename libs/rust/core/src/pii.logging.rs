use std::fmt;

pub fn mask_for_logging(value: &str) -> String {
    if value.len() < 4 {
        return "[REDACTED]".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    let first = chars[0];
    let last = chars[chars.len() - 1];
    let mid_len = chars.len() - 2;
    if mid_len > 10 {
        format!("{}**********{}", first, last)
    } else {
        format!("{}{}{}", first, "*".repeat(mid_len), last)
    }
}

pub fn mask_email(email: &str) -> String {
    if let Some((local, domain)) = email.split_once('@') {
        let masked_local = if local.len() > 2 {
            format!("{}***", &local[..2])
        } else {
            "***".to_string()
        };
        format!("{}@{}", masked_local, domain)
    } else {
        "[INVALID_EMAIL]".to_string()
    }
}

pub fn mask_uuid(id: &str) -> String {
    if id.len() >= 8 {
        format!("{}-***", &id[..8])
    } else {
        "[REDACTED]".to_string()
    }
}

pub struct SafeLog<T>(pub T);

impl fmt::Debug for SafeLog<String> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", mask_for_logging(&self.0))
    }
}

impl fmt::Debug for SafeLog<Option<String>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(v) => write!(f, "{}", mask_for_logging(v)),
            None => write!(f, "None"),
        }
    }
}
