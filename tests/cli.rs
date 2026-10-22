use std::process::Command;

fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_aam"))
        .args(arguments)
        .output()
        .unwrap()
}

#[test]
fn default_run_contains_states_and_the_result() {
    let output = run(&[]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("State 0"));
    assert!(text.contains("State 4"));
    assert!(text.contains("Result code: (λz. z)"));
    assert!(text.contains("Transitions: 4"));
}

#[test]
fn custom_source_and_no_trace_work() {
    let output = run(&["--expr", "((λx.λy.x) (λz.z)) (λw.w)", "--no-trace"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Result code: (λz. z)"));
    assert!(!text.contains("State 0"));
}

#[test]
fn all_terminating_examples_run_successfully() {
    let output = run(&["--all", "--no-trace"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout)
            .unwrap()
            .matches("Result code:")
            .count(),
        6
    );
}

#[test]
fn failures_are_nonzero_and_explain_the_problem() {
    for arguments in [
        vec!["--example", "unknown"],
        vec!["--expr", "λ."],
        vec!["--expr", "missing"],
        vec!["--limit", "-1"],
        vec!["--limit"],
        vec!["--expr"],
        vec!["--example", "identity", "--all"],
        vec!["--unexpected"],
        vec!["--example", "omega", "--limit", "12", "--no-trace"],
    ] {
        let output = run(&arguments);
        assert!(!output.status.success(), "{arguments:?}");
        assert!(String::from_utf8(output.stderr).unwrap().contains("Error:"));
    }
}

#[test]
fn example_list_is_available() {
    let output = run(&["--list"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("capture:")
    );
}

#[test]
fn basic_analyzer_exports_complete_site_allocated_graphs() {
    let output = Command::new(env!("CARGO_BIN_EXE_analyze"))
        .args(["--example", "omega", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\"schema\":2"));
    assert!(text.contains("\"allocation\":\"site\""));
    assert!(text.contains("\"complete\":true"));
    assert!(text.contains("\"finals\":[]"));
}

#[test]
fn basic_analyzer_rejects_removed_modes_and_incomplete_runs() {
    for arguments in [
        vec!["--widen"],
        vec!["--k", "1"],
        vec!["--max-states", "1", "--json"],
        vec!["--max-states", "0", "--json"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_analyze"))
            .args(&arguments)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{arguments:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr).unwrap().contains("Error:"));
    }
}
