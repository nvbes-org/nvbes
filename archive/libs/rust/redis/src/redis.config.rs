#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: String,
    pub password: Option<String>,
    pub max_connections: u32,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            password: None,
            max_connections: 10,
        }
    }
}

impl RedisConfig {
    pub fn from_env() -> Self {
        let url = std::env::var("NVBES_REDIS_URL")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or_else(|| "redis://localhost:6379".to_string());

        let password = std::env::var("NVBES_REDIS_PASSWORD")
            .ok()
            .filter(|v| !v.trim().is_empty());

        let max_connections = std::env::var("NVBES_REDIS_MAX_CONNECTIONS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(10);
        Self {
            url,
            password,
            max_connections,
        }
    }
}
