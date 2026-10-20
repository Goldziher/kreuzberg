//! Tests for the low-level measured-command primitives: timeout handling, resource-stats
//! bookkeeping, RSS measurability, and stderr-tail surfacing.

use crate::Error;
use crate::types::OutputFormat;
use std::process::Stdio;
use std::time::{Duration, Instant};

use super::super::SubprocessAdapter;

#[cfg(unix)]
#[tokio::test]
async fn subprocess_timeout_kills_and_reaps_process_group() {
    let adapter = SubprocessAdapter::new(
        "timeout-test",
        "sh",
        vec!["-c".to_string(), "sleep 30 & wait".to_string()],
        vec![],
        vec!["pdf".to_string()],
    );
    let input = tempfile::NamedTempFile::new().unwrap();
    let start = Instant::now();

    let execution = adapter
        .execute_subprocess(
            input.path(),
            Duration::from_millis(50),
            false,
            None,
            OutputFormat::Markdown,
        )
        .await
        .unwrap();

    assert!(matches!(execution.error, Some(Error::Timeout(_))));
    assert!(execution.resource_stats.baseline_memory_bytes > 0);
    assert!(execution.resource_stats.peak_memory_bytes > 0);
    assert!(execution.resource_stats.sample_count > 0);
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[cfg(unix)]
#[tokio::test]
async fn measured_command_timer_excludes_pre_spawn_staging() {
    let wall_start = Instant::now();
    tokio::time::sleep(Duration::from_millis(60)).await;
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "sleep 0.02; printf ok"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let outcome = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_secs(1),
        "timer test",
        Duration::from_millis(1),
    )
    .await
    .unwrap();

    assert!(outcome.error.is_none());
    assert!(outcome.output.unwrap().status.success());
    assert!(outcome.resource_stats.baseline_memory_bytes > 0);
    assert!(outcome.resource_stats.peak_memory_bytes > 0);
    assert!(outcome.resource_stats.sample_count > 0);
    assert!(wall_start.elapsed().saturating_sub(outcome.duration) >= Duration::from_millis(40));
}

#[cfg(unix)]
#[tokio::test]
async fn measured_ultrashort_command_is_not_measurable_from_blocked_shell_rss() {
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "printf ok"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let outcome = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_secs(1),
        "ultrashort command",
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    assert!(
        matches!(&outcome.error, Some(Error::Benchmark(message)) if message.contains("target sample")),
        "ultrashort command must fail RSS measurability: {:?}",
        outcome.error
    );
    assert!(outcome.output.unwrap().status.success());
    assert!(outcome.resource_stats.baseline_memory_bytes > 0);
    assert_eq!(outcome.resource_stats.peak_memory_bytes, 0);
    assert_eq!(outcome.resource_stats.sample_count, 0);
}

#[cfg(unix)]
#[tokio::test]
async fn timeout_error_surfaces_child_stderr_tail() {
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "echo XBERG_HANG_SENTINEL 1>&2; sleep 5"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let outcome = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_millis(300),
        "hang probe",
        Duration::from_millis(20),
    )
    .await
    .unwrap();

    let error = outcome.error.expect("a timed-out subprocess must produce an error");
    let Error::Timeout(message) = &error else {
        panic!("expected Error::Timeout, got: {error:?}");
    };
    assert!(
        message.contains("XBERG_HANG_SENTINEL"),
        "timeout error must surface the hung child's stderr tail: {message}"
    );
}

#[cfg(unix)]
#[test]
fn measured_nonzero_exit_preserves_resource_stats() {
    use super::super::support::MeasuredCommandOutcome;
    use crate::monitoring::{ResourceMonitor, ResourceSample};
    use std::os::unix::process::ExitStatusExt;

    let samples = [ResourceSample {
        memory_bytes: 2048,
        vm_size_bytes: 8192,
        page_faults: 0,
        cpu_percent: 0.0,
        timestamp_ms: 1,
    }];
    // The blocked shell is baseline metadata; target RSS may be lower after exec. ~keep
    for target_samples in [samples.as_slice(), &[]] {
        let expected = ResourceMonitor::calculate_stats(target_samples, &[], 4096);
        let measured = MeasuredCommandOutcome {
            output: Some(std::process::Output {
                status: std::process::ExitStatus::from_raw(7 << 8),
                stdout: Vec::new(),
                stderr: Vec::new(),
            }),
            duration: Duration::from_millis(20),
            resource_stats: expected.clone(),
            error: None,
        };
        let execution = SubprocessAdapter::finish_measured_command(measured, "Failing command");

        assert!(matches!(execution.error, Some(Error::Benchmark(_))));
        assert_eq!(execution.duration, Duration::from_millis(20));
        assert_eq!(
            execution.resource_stats.baseline_memory_bytes,
            expected.baseline_memory_bytes
        );
        assert_eq!(execution.resource_stats.peak_memory_bytes, expected.peak_memory_bytes);
        assert_eq!(
            execution.resource_stats.peak_memory_delta_bytes,
            expected.peak_memory_delta_bytes
        );
        assert_eq!(execution.resource_stats.sample_count, expected.sample_count);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn measured_nonzero_exit_preserves_existing_timeout_error() {
    let mut cmd = SubprocessAdapter::measured_command("sh");
    cmd.args(["-c", "sleep 0.02; exit 7"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    SubprocessAdapter::configure_measured_stdin(&mut cmd);
    SubprocessAdapter::configure_child_process(&mut cmd);

    let mut measured = SubprocessAdapter::execute_measured_command(
        &mut cmd,
        Duration::from_secs(1),
        "failing command",
        Duration::from_millis(1),
    )
    .await
    .unwrap();
    measured.error = Some(Error::Timeout("original timeout".to_string()));

    let execution = SubprocessAdapter::finish_measured_command(measured, "Failing command");

    assert!(matches!(
        execution.error,
        Some(Error::Timeout(message)) if message == "original timeout"
    ));
}
