//! Test command - run cargo test with HORUS-specific enhancements
//!
//! Features beyond cargo test:
//! - Build-before-test: Ensures Cargo.toml is generated/updated
//! - Auto session isolation: Sets unique HORUS_SESSION_ID per test run
//! - Simulation mode: Enables simulation drivers for hardware-free testing
//! - Shared memory cleanup: Cleans up /dev/shm/horus/sessions/test_* after tests
//! - Default single-threaded: Prevents shared memory conflicts (--parallel to override)
//! - Integration test mode: Runs tests marked #[ignore]

use anyhow::{Context, Result};
use colored::*;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

use crate::commands::run;

/// Shared memory base directory (platform-specific)
fn shm_base_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        PathBuf::from("/tmp/horus")
    } else {
        PathBuf::from("/dev/shm/horus")
    }
}

/// Clean up test session shared memory
fn cleanup_test_sessions(session_prefix: &str, verbose: bool) -> Result<()> {
    let sessions_dir = shm_base_dir().join("sessions");

    if !sessions_dir.exists() {
        return Ok(());
    }

    let mut cleaned = 0;
    if let Ok(entries) = fs::read_dir(&sessions_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with(session_prefix) {
                if let Err(e) = fs::remove_dir_all(entry.path()) {
                    if verbose {
                        eprintln!("  {} Failed to clean {}: {}", "[!]".yellow(), name_str, e);
                    }
                } else {
                    cleaned += 1;
                }
            }
        }
    }

    if cleaned > 0 && verbose {
        println!("  {} Cleaned up {} test session(s)", "[*]".cyan(), cleaned);
    }

    Ok(())
}

/// Check if Cargo.toml needs regeneration
fn needs_rebuild(horus_dir: &PathBuf) -> bool {
    let cargo_toml = horus_dir.join("Cargo.toml");
    let horus_yaml = PathBuf::from("horus.yaml");

    // If no Cargo.toml, definitely needs build
    if !cargo_toml.exists() {
        return true;
    }

    // If horus.yaml exists and is newer than Cargo.toml, needs rebuild
    if horus_yaml.exists() {
        if let (Ok(yaml_meta), Ok(cargo_meta)) =
            (fs::metadata(&horus_yaml), fs::metadata(&cargo_toml))
        {
            if let (Ok(yaml_time), Ok(cargo_time)) = (yaml_meta.modified(), cargo_meta.modified()) {
                return yaml_time > cargo_time;
            }
        }
    }

    // Check if any .rs files are newer than Cargo.toml
    // (simplified check - just look at main.rs if it exists)
    for main_file in &["main.rs", "lib.rs", "src/main.rs", "src/lib.rs"] {
        let path = PathBuf::from(main_file);
        if path.exists() {
            if let (Ok(src_meta), Ok(cargo_meta)) = (fs::metadata(&path), fs::metadata(&cargo_toml))
            {
                if let (Ok(src_time), Ok(cargo_time)) = (src_meta.modified(), cargo_meta.modified())
                {
                    if src_time > cargo_time {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Run tests for a HORUS project with full robotics-aware features
#[allow(clippy::too_many_arguments)]
pub fn run_tests(
    filter: Option<String>,
    release: bool,
    nocapture: bool,
    test_threads: Option<usize>,
    parallel: bool,
    simulation: bool,
    integration: bool,
    no_build: bool,
    no_cleanup: bool,
    verbose: bool,
) -> Result<()> {
    let horus_dir = PathBuf::from(".horus");

    // Generate unique session ID for test isolation
    let session_id = format!(
        "test_{}",
        Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("0000")
    );

    println!(
        "{} Running HORUS tests (session: {})",
        "[*]".cyan(),
        session_id.yellow()
    );

    // Step 1: Build if needed (unless --no-build)
    if !no_build {
        if needs_rebuild(&horus_dir) || !horus_dir.join("Cargo.toml").exists() {
            println!(
                "  {} Generating/updating build configuration...",
                "[*]".cyan()
            );

            // Use the same build logic as `horus build`
            run::execute_build_only(vec![], release, false)
                .context("Failed to build project before testing")?;
        } else if verbose {
            println!("  {} Build is up to date, skipping...", "[*]".cyan());
        }
    }

    // Verify .horus/Cargo.toml exists after potential build
    let cargo_toml = horus_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        println!("{} No .horus/Cargo.toml found.", "[!]".yellow());
        println!(
            "    Run {} first to set up the build environment.",
            "horus build".cyan()
        );
        return Ok(());
    }

    // Step 2: Set up test environment
    env::set_var("HORUS_SESSION_ID", &session_id);

    if simulation {
        env::set_var("HORUS_SIMULATION_MODE", "1");
        println!(
            "  {} Simulation mode enabled (no hardware required)",
            "[*]".cyan()
        );
    }

    if verbose {
        println!(
            "  {} Environment: HORUS_SESSION_ID={}",
            "[*]".cyan(),
            session_id
        );
    }

    // Step 3: Build cargo test command
    let mut cmd = Command::new("cargo");
    cmd.arg("test");
    cmd.current_dir(&horus_dir);

    // Pass through environment
    cmd.env("HORUS_SESSION_ID", &session_id);
    if simulation {
        cmd.env("HORUS_SIMULATION_MODE", "1");
    }

    // Add release flag if requested
    if release {
        cmd.arg("--release");
        if verbose {
            println!("  {} Building in release mode", "[*]".cyan());
        }
    }

    // Add test filter if provided
    if let Some(ref f) = filter {
        cmd.arg(f);
        if verbose {
            println!("  {} Filter: {}", "[*]".cyan(), f);
        }
    }

    // Separator for test runner args
    cmd.arg("--");

    // Add --nocapture if requested
    if nocapture {
        cmd.arg("--nocapture");
    }

    // Thread control: default to single-threaded unless --parallel or explicit -j
    let effective_threads = if let Some(threads) = test_threads {
        // Explicit thread count overrides everything
        threads
    } else if parallel {
        // --parallel means use all cores
        num_cpus::get()
    } else {
        // Default: single-threaded for shared memory safety
        1
    };

    cmd.arg(format!("--test-threads={}", effective_threads));

    if verbose {
        println!(
            "  {} Test threads: {} {}",
            "[*]".cyan(),
            effective_threads,
            if effective_threads == 1 {
                "(single-threaded for shared memory safety)"
                    .dimmed()
                    .to_string()
            } else {
                "".to_string()
            }
        );
    }

    // Integration tests: run ignored tests
    if integration {
        cmd.arg("--ignored");
        println!(
            "  {} Running integration tests (marked #[ignore])",
            "[*]".cyan()
        );
    }

    // Show test output in verbose mode
    if verbose {
        cmd.arg("--show-output");
    }

    println!("  {} Executing: cargo test in .horus/", "->".blue());

    // Step 4: Run the tests
    let status = cmd.status().context("Failed to execute cargo test")?;

    // Step 5: Cleanup shared memory (unless --no-cleanup)
    if !no_cleanup {
        cleanup_test_sessions("test_", verbose)?;
    }

    // Step 6: Report results
    if status.success() {
        println!("{}", "Tests passed!".green().bold());
    } else {
        println!("{}", "Some tests failed".red().bold());
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shm_base_dir() {
        let base = shm_base_dir();
        // Should be a valid path
        assert!(base.to_string_lossy().contains("horus"));
    }

    #[test]
    fn test_needs_rebuild_no_cargo_toml() {
        let temp_dir = PathBuf::from("/tmp/test_horus_nonexistent");
        assert!(needs_rebuild(&temp_dir));
    }
}
