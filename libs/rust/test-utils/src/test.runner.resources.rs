use super::registry::KeywordError;
use tokio::process::{Child, Command};

#[derive(Default)]
pub struct Resources {
    pub(crate) databases: Vec<(String, String)>,
    pub(crate) children: Vec<Child>,
}

pub(crate) async fn psql(url: &str, sql: &str) -> Result<(), KeywordError> {
    crate::environment::validate_loopback_url(url, "PostgreSQL")
        .map_err(KeywordError::InvalidParams)?;
    let output = Command::new("psql")
        .env("PGDATABASE", url)
        .args(["-X", "-v", "ON_ERROR_STOP=1", "-c", sql])
        .kill_on_drop(true)
        .output()
        .await
        .map_err(|_| KeywordError::Execution("psql could not execute".into()))?;
    if !output.status.success() {
        return Err(KeywordError::Execution(
            "PostgreSQL test operation failed".into(),
        ));
    }
    Ok(())
}

impl Resources {
    pub async fn cleanup(&mut self) -> Result<(), KeywordError> {
        let mut failed = false;
        for mut child in self.children.drain(..) {
            // A child that already exited needs no signal, but is still reaped.
            if child.try_wait().ok().flatten().is_none() && child.kill().await.is_err() {
                failed = true;
            }
        }
        for (url, name) in self.databases.drain(..) {
            // Names are generated internally, never accepted from cleanup YAML.
            if psql(
                &url,
                &format!("DROP DATABASE IF EXISTS \"{name}\" WITH (FORCE)"),
            )
            .await
            .is_err()
            {
                failed = true;
            }
        }
        if failed {
            Err(KeywordError::Execution(
                "owned test resource cleanup failed".into(),
            ))
        } else {
            Ok(())
        }
    }
}
