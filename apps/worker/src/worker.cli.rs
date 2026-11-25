use crate::tasks::WorkerTask;

pub const USAGE: &str = "usage: nvbes-worker serve | nvbes-worker run <migrate|identity-emails|privacy-retention|rotate-mfa-key>";

/// `serve` keeps a Scaleway Container alive and runs recurring tasks;
/// `run <task>` executes one batch for a Scaleway Job and exits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Serve,
    Run(WorkerTask),
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("{reason}\n{USAGE}")]
pub struct UsageError {
    reason: String,
}

pub fn parse(args: &[String]) -> Result<Command, UsageError> {
    let usage = |reason: String| UsageError { reason };
    match args {
        [command] if command == "serve" => Ok(Command::Serve),
        [command, task] if command == "run" => task
            .parse()
            .map(Command::Run)
            .map_err(|error| usage(error.to_string())),
        [] => Err(usage("missing command".into())),
        _ => Err(usage(format!("unexpected arguments: {}", args.join(" ")))),
    }
}

#[cfg(test)]
#[path = "worker.cli.tests.rs"]
mod tests;
