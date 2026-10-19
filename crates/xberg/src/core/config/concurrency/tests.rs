use std::sync::{Arc, Mutex};

use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::{EnvFilter, Layer};

use super::*;

#[test]
fn half_of_physical_memory_halves_and_rejects_zero() {
    assert_eq!(
        half_of_physical_memory(16 * 1024 * 1024 * 1024),
        Some(8 * 1024 * 1024 * 1024)
    );
    assert_eq!(half_of_physical_memory(1), Some(0));
    assert_eq!(half_of_physical_memory(0), None);
}

#[cfg(target_os = "macos")]
#[test]
fn sysctl_hw_memsize_reads_plausible_physical_memory_in_process() {
    let total = sysctl_hw_memsize().expect("hw.memsize must be readable through sysctlbyname");
    assert!(total >= 256 * 1024 * 1024, "implausibly small hw.memsize: {total}");
    let available = read_available_memory_bytes().expect("macOS must report available memory");
    assert_eq!(available, total / 2);
    assert!(available > 0);
}

#[cfg(feature = "captioning")]
#[test]
fn llm_concurrency_overrides_general_thread_budget() {
    let llm = crate::core::config::LlmConfig {
        max_concurrency: Some(3),
        ..Default::default()
    };
    let general = ConcurrencyConfig {
        max_threads: Some(12),
        max_concurrent_ocr: None,
    };

    assert_eq!(resolve_llm_concurrency(&llm, Some(&general)), 3);
}

#[cfg(feature = "captioning")]
#[test]
fn llm_concurrency_falls_back_to_general_thread_budget() {
    let llm = crate::core::config::LlmConfig::default();
    let general = ConcurrencyConfig {
        max_threads: Some(5),
        max_concurrent_ocr: None,
    };

    assert_eq!(resolve_llm_concurrency(&llm, Some(&general)), 5);
}

/// A tracing `Layer` that records the level of every emitted event.
#[derive(Clone, Default)]
struct EventCapture {
    levels: Arc<Mutex<Vec<tracing::Level>>>,
}

impl<S> Layer<S> for EventCapture
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        self.levels.lock().unwrap().push(*event.metadata().level());
    }
}

fn warn_event_count(capture: &EventCapture) -> usize {
    capture
        .levels
        .lock()
        .unwrap()
        .iter()
        .filter(|level| **level == tracing::Level::WARN)
        .count()
}

/// The budget scales with the machine, so the cap is asserted against an
/// injected core count. Reading the host made this fail on any machine with
/// more than eight cores, and pass on a CI runner.
#[test]
fn test_resolve_thread_budget_none() {
    assert_eq!(resolve_thread_budget_inner(None, 16, None), 8);
    let budget = resolve_thread_budget(None);
    assert!(budget >= 1, "the host always gets at least one thread");
}

// -- Pure-function pinning: resolve_thread_budget_inner --------------------
//
// These exercise resolve_thread_budget_inner directly with injected
// host_cpus/quota_cores so the assertions are deterministic regardless of
// the machine actually running the test suite (unlike resolve_thread_budget,
// which reads real num_cpus/cgroup state).

#[test]
fn test_inner_pins_default_cap_when_no_quota_and_no_max_threads() {
    assert_eq!(resolve_thread_budget_inner(None, 1, None), 1);
    assert_eq!(resolve_thread_budget_inner(None, 4, None), 4);
    assert_eq!(resolve_thread_budget_inner(None, 8, None), 8);
    assert_eq!(resolve_thread_budget_inner(None, 16, None), 8);
    assert_eq!(resolve_thread_budget_inner(None, 64, None), 8);
}

#[test]
fn test_inner_explicit_max_threads_wins_over_host_cpus_and_quota() {
    let config = ConcurrencyConfig {
        max_threads: Some(20),
        max_concurrent_ocr: None,
    };
    assert_eq!(resolve_thread_budget_inner(Some(&config), 4, Some(2)), 20);
    assert_eq!(resolve_thread_budget_inner(Some(&config), 64, None), 20);
}

#[test]
fn test_inner_explicit_max_threads_of_zero_clamps_to_one() {
    let config = ConcurrencyConfig {
        max_threads: Some(0),
        max_concurrent_ocr: None,
    };
    assert_eq!(resolve_thread_budget_inner(Some(&config), 16, None), 1);
}

#[test]
fn test_inner_cgroup_quota_above_default_cap_is_not_clamped_to_eight() {
    // This is the #1392 fix: a real cgroup quota larger than the
    // hardcoded serverless default must be honoured, not silently
    // clamped to 8 the way the unset/no-quota path is.
    assert_eq!(resolve_thread_budget_inner(None, 64, Some(24)), 24);
    assert_eq!(resolve_thread_budget_inner(None, 64, Some(9)), 9);
}

#[test]
fn test_inner_cgroup_quota_below_default_cap_is_used_as_is() {
    assert_eq!(resolve_thread_budget_inner(None, 64, Some(3)), 3);
}

#[test]
fn test_inner_cgroup_quota_never_exceeds_host_cpus() {
    assert_eq!(resolve_thread_budget_inner(None, 4, Some(16)), 4);
}

// -- One-time warning ---------------------------------------------------
//
// `#[serial]`: DEFAULT_CAP_WARNED is a process-global static with no
// injectable variant, so concurrent tests resetting/reading it would race
// each other — same reasoning as the other process-global-static tests in
// this crate (see core::pipeline::mod::tests for the established pattern). ~keep

#[test]
#[serial_test::serial]
fn test_default_cap_warning_fires_exactly_once_when_cores_exceed_cap_and_unset() {
    let already_warned = AtomicBool::new(false);
    let capture = EventCapture::default();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("warn"))
        .with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        for _ in 0..5 {
            assert_eq!(resolve_thread_budget_with_guard(None, 16, None, &already_warned), 8);
        }
    });

    assert_eq!(
        warn_event_count(&capture),
        1,
        "expected exactly one WARN event across repeated calls, got {:?}",
        capture.levels.lock().unwrap()
    );
}

#[test]
#[serial_test::serial]
fn test_default_cap_warning_does_not_fire_when_max_threads_is_set() {
    let already_warned = AtomicBool::new(false);
    let capture = EventCapture::default();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("warn"))
        .with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        let config = ConcurrencyConfig {
            max_threads: Some(4),
            max_concurrent_ocr: None,
        };
        resolve_thread_budget_with_guard(Some(&config), 16, None, &already_warned)
    });

    assert_eq!(warn_event_count(&capture), 0);
}

#[test]
#[serial_test::serial]
fn test_default_cap_warning_does_not_fire_when_cores_at_or_below_default_cap() {
    let already_warned = AtomicBool::new(false);
    let capture = EventCapture::default();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("warn"))
        .with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        resolve_thread_budget_with_guard(None, 8, None, &already_warned);
        resolve_thread_budget_with_guard(None, 1, None, &already_warned);
    });

    assert_eq!(warn_event_count(&capture), 0);
}

#[test]
#[serial_test::serial]
fn test_default_cap_warning_does_not_fire_when_cgroup_quota_present() {
    let already_warned = AtomicBool::new(false);
    let capture = EventCapture::default();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("warn"))
        .with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        resolve_thread_budget_with_guard(None, 16, Some(12), &already_warned);
    });

    assert_eq!(warn_event_count(&capture), 0);
}

#[test]
fn test_resolve_thread_budget_with_config() {
    let config = ConcurrencyConfig {
        max_threads: Some(4),
        max_concurrent_ocr: None,
    };
    assert_eq!(resolve_thread_budget(Some(&config)), 4);
}

#[test]
fn test_resolve_thread_budget_clamps_to_one() {
    let config = ConcurrencyConfig {
        max_threads: Some(0),
        max_concurrent_ocr: None,
    };
    assert_eq!(resolve_thread_budget(Some(&config)), 1);
}

/// A config that sets no maximum takes the same default cap, asserted
/// against an injected core count for the same reason.
#[test]
fn test_resolve_thread_budget_no_max() {
    let config = ConcurrencyConfig {
        max_threads: None,
        max_concurrent_ocr: None,
    };
    assert_eq!(resolve_thread_budget_inner(Some(&config), 16, None), 8);
    let budget = resolve_thread_budget(Some(&config));
    assert!(budget >= 1, "the host always gets at least one thread");
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_batch_plan_without_layout_uses_available_budget() {
    let budget = resolve_thread_budget(None);
    assert_eq!(
        resolve_batch_execution_plan(None, LayoutBatchWorkload::None, budget, None),
        BatchExecutionPlan {
            workers: budget,
            thread_budget: 1,
        }
    );
}

#[test]
#[cfg(all(not(target_arch = "wasm32"), layout_detection))]
fn test_layout_batch_plan_table() {
    for budget in [1, 2, 4, 8] {
        let config = ConcurrencyConfig {
            max_threads: Some(budget),
            max_concurrent_ocr: None,
        };
        assert_eq!(
            resolve_batch_execution_plan(Some(&config), LayoutBatchWorkload::All, 16, None),
            BatchExecutionPlan {
                workers: 1,
                thread_budget: budget,
            }
        );
    }
}

#[test]
#[cfg(all(not(target_arch = "wasm32"), layout_detection))]
fn test_mixed_layout_batch_preserves_two_worker_cap() {
    for (budget, workers, thread_budget) in [(1, 1, 1), (2, 2, 1), (4, 2, 2), (8, 2, 4)] {
        let config = ConcurrencyConfig {
            max_threads: Some(budget),
            max_concurrent_ocr: None,
        };
        assert_eq!(
            resolve_batch_execution_plan(Some(&config), LayoutBatchWorkload::Mixed, 16, None),
            BatchExecutionPlan { workers, thread_budget }
        );
    }
}

#[test]
#[cfg(all(not(target_arch = "wasm32"), layout_detection))]
fn test_layout_batch_plan_respects_input_and_explicit_limits() {
    let config = ConcurrencyConfig {
        max_threads: Some(8),
        max_concurrent_ocr: None,
    };
    assert_eq!(
        resolve_batch_execution_plan(Some(&config), LayoutBatchWorkload::All, 1, Some(8)),
        BatchExecutionPlan {
            workers: 1,
            thread_budget: 8,
        }
    );
    assert_eq!(
        resolve_batch_execution_plan(Some(&config), LayoutBatchWorkload::All, 8, Some(1)),
        BatchExecutionPlan {
            workers: 1,
            thread_budget: 8,
        }
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_non_layout_batch_plan_divides_budget_at_explicit_worker_limit() {
    let config = ConcurrencyConfig {
        max_threads: Some(8),
        max_concurrent_ocr: None,
    };
    let plan = resolve_batch_execution_plan(Some(&config), LayoutBatchWorkload::None, 16, Some(2));
    assert_eq!(plan.workers, 2);
    assert_eq!(plan.thread_budget, 4);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_non_layout_batch_plan_clamps_explicit_limit_to_total_budget() {
    let config = ConcurrencyConfig {
        max_threads: Some(2),
        max_concurrent_ocr: None,
    };
    let plan = resolve_batch_execution_plan(Some(&config), LayoutBatchWorkload::None, 8, Some(6));
    assert_eq!(plan.workers, 2);
    assert_eq!(plan.thread_budget, 1);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_non_layout_batch_plan_gives_single_input_full_inner_budget() {
    let config = ConcurrencyConfig {
        max_threads: Some(8),
        max_concurrent_ocr: None,
    };
    let plan = resolve_batch_execution_plan(Some(&config), LayoutBatchWorkload::None, 1, None);
    assert_eq!(plan.workers, 1);
    assert_eq!(plan.thread_budget, 8);
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn test_batch_plan_never_exceeds_total_budget() {
    for total_budget in 1..=8 {
        let config = ConcurrencyConfig {
            max_threads: Some(total_budget),
            max_concurrent_ocr: None,
        };
        let cases = (0..=12).flat_map(|input_count| {
            [None, Some(0), Some(1), Some(3), Some(16)].map(move |max_concurrent| (input_count, max_concurrent))
        });
        for (input_count, max_concurrent) in cases {
            #[cfg(layout_detection)]
            let layout_workloads = [
                LayoutBatchWorkload::None,
                LayoutBatchWorkload::Mixed,
                LayoutBatchWorkload::All,
            ];
            #[cfg(not(layout_detection))]
            let layout_workloads = [LayoutBatchWorkload::None];
            for layout_workload in layout_workloads {
                let plan = resolve_batch_execution_plan(Some(&config), layout_workload, input_count, max_concurrent);
                assert!(plan.workers * plan.thread_budget <= total_budget);
                assert!(plan.workers <= total_budget);
                assert!(plan.workers <= input_count.max(1));
                if let Some(explicit) = max_concurrent {
                    assert!(plan.workers <= explicit.max(1));
                }
            }
        }
    }
}

// -- Recognition concurrency -------------------------------------------
//
// Injected budget and memory for the same reason as the thread-budget
// pinning above: the real values move with the machine running the tests.

const GIB: u64 = 1024 * 1024 * 1024;

/// Memory for sixty-four sessions, so only the budget under test binds.
const AMPLE_MEMORY: u64 = 64 * TESSERACT_SESSION_MEMORY_BYTES;

/// Resolve against a warning guard this call owns, for the reason given on
/// [`warn_default_thread_cap_once`]: a process-global guard is tripped by
/// whichever test in the binary ran first.
fn recognition_sessions(
    config: Option<&ConcurrencyConfig>,
    thread_budget: usize,
    available_memory: Option<u64>,
) -> usize {
    resolve_recognition_concurrency_with_guard(config, thread_budget, available_memory, &AtomicBool::new(false))
}

/// The clamp is reported, not silent. A budget of 32 that runs 6 sessions
/// otherwise looks exactly like the fixed-four defect this change removes.
#[test]
#[serial_test::serial]
fn test_recognition_memory_clamp_warning_fires_exactly_once() {
    let already_warned = AtomicBool::new(false);
    let capture = EventCapture::default();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("warn"))
        .with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        let six_sessions = Some(6 * TESSERACT_SESSION_MEMORY_BYTES);
        for _ in 0..5 {
            assert_eq!(
                resolve_recognition_concurrency_with_guard(None, 32, six_sessions, &already_warned),
                6
            );
        }
    });

    assert_eq!(
        warn_event_count(&capture),
        1,
        "expected exactly one WARN event across repeated calls, got {:?}",
        capture.levels.lock().unwrap()
    );
}

/// A budget the memory can feed is not a clamp, and neither is an absent
/// reading, so both stay silent.
#[test]
#[serial_test::serial]
fn test_recognition_memory_clamp_warning_does_not_fire_when_memory_does_not_bind() {
    let already_warned = AtomicBool::new(false);
    let capture = EventCapture::default();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("warn"))
        .with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        resolve_recognition_concurrency_with_guard(None, 8, Some(AMPLE_MEMORY), &already_warned);
        resolve_recognition_concurrency_with_guard(None, 8, None, &already_warned);
        let config = ConcurrencyConfig {
            max_threads: Some(32),
            max_concurrent_ocr: Some(2),
        };
        resolve_recognition_concurrency_with_guard(Some(&config), 32, Some(GIB), &already_warned);
    });

    assert_eq!(warn_event_count(&capture), 0);
}

/// The historical four is gone: recognition follows the thread budget.
#[test]
fn test_ocr_concurrency_follows_the_thread_budget() {
    assert_eq!(recognition_sessions(None, 8, Some(AMPLE_MEMORY)), 8);
    assert_eq!(recognition_sessions(None, 32, Some(AMPLE_MEMORY)), 32);
    assert_eq!(recognition_sessions(None, 2, Some(AMPLE_MEMORY)), 2);
}

#[test]
fn test_ocr_concurrency_takes_the_configured_limit_over_the_budget() {
    let config = ConcurrencyConfig {
        max_threads: Some(32),
        max_concurrent_ocr: Some(4),
    };
    assert_eq!(recognition_sessions(Some(&config), 32, Some(AMPLE_MEMORY)), 4);
    assert_eq!(recognition_sessions(Some(&config), 2, None), 4);
}

#[test]
fn test_ocr_concurrency_clamps_a_configured_zero_to_one() {
    let config = ConcurrencyConfig {
        max_threads: None,
        max_concurrent_ocr: Some(0),
    };
    assert_eq!(recognition_sessions(Some(&config), 32, Some(AMPLE_MEMORY)), 1);
}

/// Cores the memory cannot feed are not sessions. Without this a container
/// with a wide CPU quota and a narrow memory limit is killed rather than
/// slowed.
#[test]
fn test_ocr_concurrency_never_exceeds_what_memory_holds() {
    let session = TESSERACT_SESSION_MEMORY_BYTES;
    assert_eq!(recognition_sessions(None, 32, Some(6 * session)), 6);
    assert_eq!(recognition_sessions(None, 32, Some(session / 8)), 1);
    assert_eq!(recognition_sessions(None, 4, Some(64 * session)), 4);
}

/// No reading is not a reading of zero: platforms that report no memory
/// limit keep the budget, the way a missing cgroup CPU quota does.
#[test]
fn test_ocr_concurrency_without_a_memory_reading_follows_the_budget() {
    assert_eq!(recognition_sessions(None, 8, None), 8);
}

#[test]
fn test_ocr_concurrency_reads_the_real_host() {
    assert!(
        resolve_recognition_concurrency(None) >= 1,
        "the host always gets one session"
    );
}

/// The defect: recognition stayed four wide however many threads the host
/// was budgeted. Both the budget and the memory reading are injected, so the
/// assertion is about the resolver rather than about the runner. Reading the
/// real host instead failed on any machine with more than four cores and
/// under about 2.5 GiB free, and passed for the wrong reason on a runner
/// budgeted four or fewer.
#[test]
fn test_ocr_concurrency_is_not_pinned_to_four_on_a_many_core_host() {
    for budget in [5, 32] {
        let sessions = recognition_sessions(None, budget, Some(AMPLE_MEMORY));
        assert!(
            sessions > 4,
            "recognition admits {sessions} sessions on a host budgeted {budget} threads"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn test_cgroup_headroom_reads_both_unlimited_spellings_as_no_limit() {
    assert_eq!(parse_cgroup_headroom("max\n", "1024\n"), None);
    assert_eq!(parse_cgroup_headroom("9223372036854771712\n", "1024\n"), None);
    assert_eq!(parse_cgroup_headroom("0\n", "1024\n"), None);
}

/// The limit is not the headroom. A cgroup already holding most of its
/// allowance grants what is left, not what it was given -- sizing sessions
/// against the limit is how the process gets killed rather than slowed.
#[cfg(target_os = "linux")]
#[test]
fn test_cgroup_headroom_subtracts_current_usage_from_the_limit() {
    assert_eq!(parse_cgroup_headroom("2147483648\n", "0\n"), Some(2 * GIB));
    assert_eq!(parse_cgroup_headroom("8589934592\n", "6442450944\n"), Some(2 * GIB));
    assert_eq!(parse_cgroup_headroom("2147483648\n", "4294967296\n"), Some(0));
    assert_eq!(parse_cgroup_headroom("2147483648\n", "not-a-number\n"), None);
}

#[cfg(target_os = "linux")]
#[test]
fn test_mem_available_is_read_in_kilobytes() {
    let meminfo = "MemTotal:       65787528 kB\nMemFree:          262144 kB\nMemAvailable:    1048576 kB\n";
    assert_eq!(parse_mem_available_bytes(meminfo), Some(GIB));
    assert_eq!(parse_mem_available_bytes("MemTotal: 65787528 kB\n"), None);
}

/// The documented contract for a second extraction, and the line that
/// keeps it from being silent. The first extraction fixes the recognition
/// limit, because the limiters that enforce it are built once per process;
/// a later extraction that names a different value keeps the first one and
/// says so exactly once. The latch cells belong to this test, so the
/// assertion is about the installer rather than about whichever test in
/// the binary ran first.
///
/// One test rather than two: `tracing` caches a callsite's interest the
/// first time it is reached, so a sibling test that reached this warning
/// outside a capturing subscriber left the event unrecorded here for the
/// rest of the process. Measured — it passed alone and under
/// `--test-threads=1`, and failed in parallel. ~keep
#[test]
#[serial_test::serial]
fn a_second_extraction_keeps_the_first_recognition_limit_and_says_so() {
    let once = Once::new();
    let thread_budget = AtomicUsize::new(0);
    let recognition = AtomicUsize::new(0);
    let latch_warned = AtomicBool::new(false);
    let limits = ActiveLimits {
        once: &once,
        thread_budget: &thread_budget,
        recognition: &recognition,
        latch_warned: &latch_warned,
    };
    let first = ConcurrencyConfig {
        max_threads: Some(8),
        max_concurrent_ocr: Some(3),
    };
    let second = ConcurrencyConfig {
        max_threads: Some(8),
        max_concurrent_ocr: Some(16),
    };
    let capture = EventCapture::default();
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::new("warn"))
        .with(capture.clone());

    tracing::subscriber::with_default(subscriber, || {
        assert!(
            limits.install(Some(&first), 8, |_| {}),
            "the first call installs the limits"
        );
        assert_eq!(recognition.load(Ordering::Relaxed), 3);
        assert_eq!(warn_event_count(&capture), 0, "the first extraction loses nothing");

        for _ in 0..3 {
            assert!(
                !limits.install(Some(&second), 8, |_| {}),
                "a later call installs nothing"
            );
        }
        assert_eq!(
            recognition.load(Ordering::Relaxed),
            3,
            "the second extraction's max_concurrent_ocr must not move the fixed limit"
        );

        // A later call that names the SAME limit the process already fixed must not
        // warn: nothing was discarded. Uses its own latch cell so the guard already
        // tripped above cannot mask this assertion (GH#1751: deleting `requested !=
        // active` left every existing test green because none of them exercised this
        // branch). ~keep
        let matching_latch_warned = AtomicBool::new(false);
        let matching_limits = ActiveLimits {
            once: &once,
            thread_budget: &thread_budget,
            recognition: &recognition,
            latch_warned: &matching_latch_warned,
        };
        assert!(
            !matching_limits.install(Some(&first), 8, |_| {}),
            "a later call still installs nothing"
        );
        assert!(
            !matching_latch_warned.load(Ordering::Relaxed),
            "requesting the limit the process already fixed must not warn"
        );

        limits.install(Some(&first), 8, |_| {});
    });

    assert!(
        latch_warned.load(Ordering::Relaxed),
        "the discarded value must trip the warning guard"
    );
    assert_eq!(
        warn_event_count(&capture),
        1,
        "expected exactly one WARN across repeated calls, got {:?}",
        capture.levels.lock().unwrap()
    );
}

/// The memory reader must stay uncached. A latch here sizes every later
/// document in a long-lived process from whatever was free during the
/// first extraction, which is the server regression this branch removed.
///
/// Proven behaviorally through the seam [`available_memory_reading`] offers
/// rather than by inspecting source text: an injected reader that returns a
/// different value on every call must come back unmemoized on every call.
/// A prior version of this guard inspected `available_memory_bytes`'s own
/// one-line body for cache-shaped identifiers (`OnceLock`, `static`, ...)
/// and missed the identical regression reinstated one call frame deeper,
/// inside a platform `read_available_memory_bytes` body it never read
/// (GH#1751). ~keep
#[test]
fn available_memory_reading_returns_a_fresh_value_on_every_call() {
    let calls = std::cell::Cell::new(0_u64);
    let read = || {
        let call = calls.get();
        calls.set(call + 1);
        Some(call)
    };

    assert_eq!(available_memory_reading(read), Some(0));
    assert_eq!(available_memory_reading(read), Some(1));
    assert_eq!(available_memory_reading(read), Some(2));
}

/// Only the first call installs the pools, but every call reports the
/// budget its own config asks for.
#[test]
fn test_init_thread_pools_idempotent() {
    let two = ConcurrencyConfig {
        max_threads: Some(2),
        max_concurrent_ocr: None,
    };
    let four = ConcurrencyConfig {
        max_threads: Some(4),
        max_concurrent_ocr: None,
    };
    assert_eq!(init_thread_pools(Some(&two)), 2);
    assert_eq!(init_thread_pools(Some(&four)), 4);
}

#[test]
fn test_init_thread_pools_uses_total_configured_budget() {
    let config = ConcurrencyConfig {
        max_threads: Some(7),
        max_concurrent_ocr: None,
    };
    assert_eq!(init_thread_pools(Some(&config)), 7);
}

/// [`init_thread_pools`]'s own wrapper must pass the caller's `config`
/// through to [`ActiveLimits::install`], not a stand-in `None` (GH#1751):
/// recognition only reflects `max_concurrent_ocr` when `config` actually
/// reaches `resolve_recognition_concurrency` inside the fence. Exercises
/// [`init_thread_pools_with`] -- the entry point the real, process-global
/// `init_thread_pools` also runs through -- with cells this test owns, so
/// this closes the one gap `install`'s own direct tests cannot: whether
/// `init_thread_pools`'s call site forwards its argument at all. `999` is
/// far outside any value the automatic (no-config) path could resolve to
/// on a real host, so a `None` substitution cannot pass by coincidence.
#[test]
fn init_thread_pools_with_passes_the_callers_config_to_install() {
    let once = Once::new();
    let thread_budget = AtomicUsize::new(0);
    let recognition = AtomicUsize::new(0);
    let latch_warned = AtomicBool::new(false);
    let limits = ActiveLimits {
        once: &once,
        thread_budget: &thread_budget,
        recognition: &recognition,
        latch_warned: &latch_warned,
    };
    let config = ConcurrencyConfig {
        max_threads: Some(6),
        max_concurrent_ocr: Some(999),
    };
    let mut pool_built_with = None;

    let budget = init_thread_pools_with(Some(&config), &limits, |budget| pool_built_with = Some(budget));

    assert_eq!(
        budget, 6,
        "the returned budget always reflects resolve_thread_budget(config)"
    );
    assert_eq!(
        recognition.load(Ordering::Relaxed),
        999,
        "the installed recognition limit must come from the caller's config, not None"
    );
    assert_eq!(
        pool_built_with,
        Some(6),
        "the pool builder must run inside the same install call, with the resolved budget"
    );
}

#[test]
fn test_default() {
    let config = ConcurrencyConfig::default();
    assert!(config.max_threads.is_none());
}

#[test]
fn test_serde_roundtrip() {
    let json = r#"{"max_threads": 2}"#;
    let config: ConcurrencyConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.max_threads, Some(2));

    let serialized = serde_json::to_string(&config).unwrap();
    let roundtripped: ConcurrencyConfig = serde_json::from_str(&serialized).unwrap();
    assert_eq!(roundtripped.max_threads, Some(2));
}

#[test]
fn test_serde_empty() {
    let json = r#"{}"#;
    let config: ConcurrencyConfig = serde_json::from_str(json).unwrap();
    assert!(config.max_threads.is_none());
}
