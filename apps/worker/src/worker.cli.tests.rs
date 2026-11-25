use super::*;

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[test]
fn serve_and_every_task_parse() {
    assert_eq!(parse(&args(&["serve"])), Ok(Command::Serve));
    for task in WorkerTask::ALL {
        assert_eq!(parse(&args(&["run", task.name()])), Ok(Command::Run(task)));
    }
}

#[test]
fn anything_else_is_a_usage_error() {
    for invalid in [
        vec![],
        vec!["run"],
        vec!["run", "billing-outbox"],
        vec!["serve", "now"],
        vec!["migrate"],
    ] {
        let error = parse(&args(&invalid)).unwrap_err();
        assert!(error.to_string().ends_with(USAGE), "{invalid:?}");
    }
}
