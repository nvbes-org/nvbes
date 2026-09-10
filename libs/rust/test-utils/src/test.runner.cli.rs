use std::path::Path;

use nvbes_test_utils::runner::{self, TestResult, manifest, registry::register_all};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: nvbes-runner <test-case.yaml> [--var KEY=VAL ...] [--manifest <path>]");
        std::process::exit(1);
    }

    let file_path = &args[1];
    let (extra_vars, manifest_path) = parse_flags(&args[2..]).unwrap_or_else(|error| {
        eprintln!("Invalid runner arguments: {error}");
        std::process::exit(1);
    });

    let yaml = match std::fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading {file_path}: {e}");
            std::process::exit(1);
        }
    };

    let mut test_case = match runner::parse_test_case(&yaml) {
        Ok(tc) => tc,
        Err(e) => {
            eprintln!("Error parsing {file_path}: {e}");
            std::process::exit(1);
        }
    };

    for (k, v) in extra_vars {
        test_case.env.insert(k, v);
    }

    let manifest = if let Some(ref path) = manifest_path {
        let mut kw_registry = nvbes_test_utils::runner::registry::KeywordRegistry::new();
        register_all(&mut kw_registry);

        let m = match manifest::load_manifest(Path::new(path)) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("Error loading manifest: {e}");
                std::process::exit(1);
            }
        };

        let validation = manifest::validate_test_case(&test_case, Some(&m), &kw_registry);

        for w in &validation.warnings {
            eprintln!("WARNING: {w}");
        }

        if !validation.valid {
            eprintln!("Manifest validation FAILED:");
            for err in &validation.errors {
                eprintln!("  ERROR: {err}");
            }
            std::process::exit(1);
        }

        Some(m)
    } else {
        None
    };

    let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    let result = runtime.block_on(runner::run_test_case_with_manifest(
        &test_case,
        manifest.as_ref(),
    ));

    print_result(&result);

    if result.passed {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

type Flags = (Vec<(String, String)>, Option<String>);

fn parse_flags(args: &[String]) -> Result<Flags, &'static str> {
    let mut vars = Vec::new();
    let mut manifest_path = None;
    let mut i = 0;
    while i < args.len() {
        let (flag, value) = if args[i] == "--var" || args[i] == "--manifest" {
            let value = args.get(i + 1).ok_or("missing flag value")?;
            let flag = args[i].as_str();
            i += 2;
            (flag, value.as_str())
        } else if let Some((flag, value)) = args[i].split_once('=') {
            i += 1;
            (flag, value)
        } else {
            return Err("unknown flag");
        };
        match flag {
            "--var" => {
                let (key, value) = value.split_once('=').ok_or("variable must be KEY=VALUE")?;
                if key.is_empty()
                    || !key.bytes().enumerate().all(|(index, byte)| {
                        byte == b'_'
                            || byte.is_ascii_alphabetic()
                            || (index > 0 && byte.is_ascii_digit())
                    })
                {
                    return Err("invalid variable name");
                }
                if vars.iter().any(|(existing, _)| existing == key) {
                    return Err("duplicate variable");
                }
                vars.push((key.to_string(), value.to_string()));
            }
            "--manifest" if !value.is_empty() && manifest_path.is_none() => {
                manifest_path = Some(value.to_string());
            }
            _ => return Err("unknown, empty or duplicate flag"),
        }
    }
    Ok((vars, manifest_path))
}

fn print_result(result: &TestResult) {
    let status = if result.passed { "PASS" } else { "FAIL" };
    println!("═══════════════════════════════════════════════");
    println!(
        " {} {} ({})",
        result.test_case.id,
        status,
        result.test_case.description.as_deref().unwrap_or("")
    );
    println!("═══════════════════════════════════════════════");

    for (i, step) in result.steps.iter().enumerate() {
        let icon = if step.passed { "✓" } else { "✗" };
        let label = step.label.as_deref().unwrap_or(&step.keyword);
        println!(
            "  {icon} Step {}: {} ({}ms)",
            i + 1,
            label,
            step.duration_ms
        );
        for err in &step.errors {
            println!("    error: {err}");
        }
    }

    println!("───────────────────────────────────────────────");
    println!(
        " Total: {}ms | Steps: {} | {}",
        result.total_duration_ms,
        result.steps.len(),
        status
    );
}

#[cfg(test)]
#[path = "test.runner.cli.tests.rs"]
mod tests;
