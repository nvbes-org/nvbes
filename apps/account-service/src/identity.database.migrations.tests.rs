use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[test]
fn migration_versions_are_unique() {
    let migrations_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let mut migrations_by_version = BTreeMap::<u64, Vec<String>>::new();

    for entry in fs::read_dir(&migrations_dir).expect("account migrations directory must exist") {
        let path = entry
            .expect("migration directory entry must be readable")
            .path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("sql") {
            continue;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("migration filename must be valid UTF-8");
        let version = file_name
            .split_once('_')
            .expect("migration filename must start with a numeric version")
            .0
            .parse::<u64>()
            .expect("migration version must be numeric");

        migrations_by_version
            .entry(version)
            .or_default()
            .push(file_name.to_string());
    }

    let duplicates = migrations_by_version
        .into_iter()
        .filter(|(_, filenames)| filenames.len() > 1)
        .collect::<Vec<_>>();

    assert!(
        duplicates.is_empty(),
        "SQLx migration versions must be unique; duplicates: {duplicates:?}"
    );
}
