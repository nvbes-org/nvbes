use super::parse_flags;

#[test]
fn rejects_invalid_flags_without_echoing_values() {
    for args in [
        vec!["--unknown"],
        vec!["--var"],
        vec!["--var=secret"],
        vec!["--var=1INVALID=secret"],
        vec!["--var=A=1", "--var=A=2"],
        vec!["--manifest=a", "--manifest=b"],
    ] {
        assert!(parse_flags(&args.into_iter().map(String::from).collect::<Vec<_>>()).is_err());
    }
}

#[test]
fn accepts_both_argument_forms() {
    let args = ["--var", "A=1", "--var=B=2", "--manifest", "scope.json"].map(String::from);
    let (vars, manifest) = parse_flags(&args).unwrap();
    assert_eq!(vars.len(), 2);
    assert_eq!(manifest.as_deref(), Some("scope.json"));
}
