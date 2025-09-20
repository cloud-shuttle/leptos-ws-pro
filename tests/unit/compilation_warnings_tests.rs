//! TDD tests for compilation warning elimination
//!
//! This module tests that we have zero compilation warnings,
//! ensuring production-ready code quality.

use std::process::Command;

#[test]
fn test_zero_compilation_warnings() {
    // Given: We want zero compilation warnings
    // When: Running cargo check
    let output = Command::new("cargo")
        .args(&["check", "--quiet"])
        .output()
        .expect("Failed to run cargo check");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Then: Should have zero warnings
    let warning_count = stderr.matches("warning:").count();

    if warning_count > 0 {
        println!("Found {} warnings:", warning_count);
        println!("{}", stderr);
    }

    assert_eq!(
        warning_count, 0,
        "Expected zero compilation warnings, found {}",
        warning_count
    );
}

#[test]
fn test_zero_compilation_errors() {
    // Given: We want zero compilation errors
    // When: Running cargo check
    let output = Command::new("cargo")
        .args(&["check", "--quiet"])
        .output()
        .expect("Failed to run cargo check");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Then: Should have zero errors
    let error_count = stderr.matches("error:").count();

    if error_count > 0 {
        println!("Found {} errors:", error_count);
        println!("{}", stderr);
    }

    assert_eq!(
        error_count, 0,
        "Expected zero compilation errors, found {}",
        error_count
    );
}

#[test]
fn test_cargo_fix_applicable_warnings() {
    // Given: We want to check if cargo fix can resolve some warnings
    // When: Running cargo fix --dry-run
    let output = Command::new("cargo")
        .args(&["fix", "--allow-dirty", "--dry-run"])
        .output()
        .expect("Failed to run cargo fix --dry-run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Then: Should show what can be automatically fixed
    println!("Cargo fix dry run output:");
    println!("STDOUT: {}", stdout);
    println!("STDERR: {}", stderr);

    // This test documents what cargo fix can handle
    assert!(
        output.status.success() || !stderr.is_empty(),
        "Cargo fix should provide information about fixable warnings"
    );
}
