//! Concurrency and thread pool configuration.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Once, OnceLock};

use serde::{Deserialize, Serialize};

/// Controls thread usage for constrained environments.
///
/// Set `max_threads` to cap all internal thread pools (Rayon, ONNX Runtime
/// intra-op), batch concurrency and Tesseract recognition to a single limit.
/// Set `max_concurrent_ocr` to give recognition a limit of its own, which is
/// the knob to reach for when the host has cores to spare but not the memory
/// to run a recognition session on each of them. It is applied as given and
/// is not capped by `max_threads`. The first extraction in a process fixes
/// it for that process — see the field's own documentation.
///
/// # Default budget when `max_threads` is unset
///
/// Without an explicit `max_threads`, the effective budget is
/// `min(detected_cpu_cores, 8)` — a deliberate ceiling chosen for
/// serverless/shared-tenant defaults, not a full-host auto-scale. On a host
/// with more than 8 cores this means the extra cores go **unused** unless one
/// of the following applies:
///
/// - `max_threads` is set explicitly above 8 (the only way to exceed the
///   ceiling on a bare-metal or VM host with no CPU quota).
/// - The process runs under a Linux cgroup CPU quota (containers, Kubernetes
///   `resources.limits.cpu`); in that case the quota itself is used as the
///   ceiling instead of the hardcoded 8, since the quota already reflects a
///   deliberately-configured resource limit.
///
/// When neither applies and the host has more than 8 cores, a single
/// `WARN`-level log is emitted the first time the budget is resolved,
/// naming the detected core count and the applied cap, so the ceiling is
/// discoverable without reading source.
///
/// # Example
///
/// ```rust
/// use xberg::core::config::ConcurrencyConfig;
///
/// let config = ConcurrencyConfig {
///     max_threads: Some(2),
///     max_concurrent_ocr: None,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct ConcurrencyConfig {
    /// Maximum number of threads for all internal thread pools.
    ///
    /// Caps Rayon global pool size, ONNX Runtime intra-op threads, and the
    /// combined document/inner-task budget for batch extraction. When `None`,
    /// the effective budget is `min(detected_cpu_cores, 8)` unless a Linux
    /// cgroup CPU quota is present, in which case the quota is used as the
    /// ceiling instead. On hosts with more than 8 cores and no cgroup quota,
    /// set `max_threads` explicitly to use the additional cores — the
    /// default will not scale past 8 on its own.
    pub max_threads: Option<usize>,

    /// Maximum number of Tesseract recognition sessions that run at once.
    ///
    /// When `None`, recognition follows `max_threads`, reduced to the number
    /// of sessions the host's free memory holds. Each session keeps its own
    /// page image and recognition working set resident, so a host with many
    /// cores and little memory needs this lower than the thread budget. Set
    /// it to `4` to keep the fixed limit that releases up to 1.2.6 applied.
    ///
    /// A value set here is applied as given: neither the thread budget nor
    /// the memory reading reduces it. Both of those bound the automatic
    /// limit, and a caller who names a number has already decided what the
    /// host can carry.
    ///
    /// The first extraction in a process fixes the limit for the rest of that
    /// process, and a later extraction that names a different value keeps the
    /// first one. The two limiters that enforce it — the admission semaphore
    /// in the Tesseract backend and the handle pool behind it — are built once
    /// inside a backend the plugin registry holds for the life of the process,
    /// and the pool's capacity is fixed when it is constructed. A later value
    /// could therefore be reported but never enforced. Set it on the first
    /// extraction, or run one process per value.
    pub max_concurrent_ocr: Option<usize>,
}

static POOL_INIT: Once = Once::new();
static ACTIVE_THREAD_BUDGET: AtomicUsize = AtomicUsize::new(0);
static ACTIVE_RECOGNITION_CONCURRENCY: AtomicUsize = AtomicUsize::new(0);

/// Ceiling applied to the auto-detected thread budget when `max_threads` is
/// unset and no tighter resource limit (e.g. a cgroup CPU quota) is found.
///
/// This is a deliberate serverless/shared-tenant default, not a scaling
/// limit of the underlying pipelines — see [`ConcurrencyConfig::max_threads`].
const DEFAULT_THREAD_CAP: usize = 8;

/// Guards the one-time startup warning for the default-cap fallback so it
/// fires at most once per process, not once per extraction.
static DEFAULT_CAP_WARNED: AtomicBool = AtomicBool::new(false);

/// Emit the "unused cores" warning at most once per `already_warned` guard.
///
/// The guard is a parameter rather than a direct read of [`DEFAULT_CAP_WARNED`]
/// so that the once-per-process property can be tested against a guard the test
/// owns. Asserting on the global is not merely racy but unfixably so: any test
/// in the binary that runs a real extraction reaches `resolve_thread_budget`
/// and trips the global first on a host with more than [`DEFAULT_THREAD_CAP`]
/// cores, and `#[serial]` cannot help because it only excludes other `#[serial]`
/// tests. That is the same defect class as #215.
fn warn_default_thread_cap_once(already_warned: &AtomicBool, host_cpus: usize) {
    if already_warned
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        tracing::warn!(
            host_cpus,
            thread_cap = DEFAULT_THREAD_CAP,
            "detected {host_cpus} CPU cores but no `max_threads` is configured and no cgroup CPU \
             quota was found; capping the thread budget at {DEFAULT_THREAD_CAP} \
             (min(cpu_cores, {DEFAULT_THREAD_CAP})). Set `ConcurrencyConfig::max_threads` above \
             {DEFAULT_THREAD_CAP} to use the remaining cores."
        );
    }
}

/// The cgroup CPU quota, resolved at most once per process.
///
/// `resolve_thread_budget` runs per extracted document (see
/// `core::extractor::file`), and the quota cannot change under a running
/// process, so reading `/sys/fs/cgroup/...` on every call would be two
/// syscalls per document for a value that never moves.
static CGROUP_QUOTA_CORES: OnceLock<Option<usize>> = OnceLock::new();

/// Detect an effective CPU core cap from the process's Linux cgroup CPU
/// quota, if any.
///
/// Returns `None` when no quota is configured (bare metal, most developer
/// machines, and non-Linux platforms) — callers then fall back to
/// [`DEFAULT_THREAD_CAP`]. Returns `Some(cores)` when a cgroup v2 (`cpu.max`)
/// or v1 (`cpu.cfs_quota_us` / `cpu.cfs_period_us`) quota is present and
/// finite, rounded up to the nearest whole core. Never panics: any read or
/// parse failure is treated as "no quota".
fn cgroup_cpu_quota_cores() -> Option<usize> {
    *CGROUP_QUOTA_CORES.get_or_init(read_cgroup_cpu_quota_cores)
}

#[cfg(target_os = "linux")]
fn read_cgroup_cpu_quota_cores() -> Option<usize> {
    cgroup_v2_quota_cores().or_else(cgroup_v1_quota_cores)
}

#[cfg(not(target_os = "linux"))]
fn read_cgroup_cpu_quota_cores() -> Option<usize> {
    None
}

#[cfg(target_os = "linux")]
fn cgroup_v2_quota_cores() -> Option<usize> {
    let contents = std::fs::read_to_string("/sys/fs/cgroup/cpu.max").ok()?;
    let mut fields = contents.split_whitespace();
    let quota_field = fields.next()?;
    let period_field = fields.next()?;
    if quota_field == "max" {
        return None;
    }
    quota_period_to_cores(quota_field.parse().ok()?, period_field.parse().ok()?)
}

#[cfg(target_os = "linux")]
fn cgroup_v1_quota_cores() -> Option<usize> {
    let quota: f64 = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_quota_us")
        .ok()?
        .trim()
        .parse()
        .ok()?;
    let period: f64 = std::fs::read_to_string("/sys/fs/cgroup/cpu/cpu.cfs_period_us")
        .ok()?
        .trim()
        .parse()
        .ok()?;
    quota_period_to_cores(quota, period)
}

/// `cpu.cfs_quota_us` of `-1` (v1) and a bare `max` period (v2, handled by the
/// caller) both mean "unlimited" and must resolve to `None`, not a huge cap.
#[cfg(target_os = "linux")]
fn quota_period_to_cores(quota: f64, period: f64) -> Option<usize> {
    if quota <= 0.0 || period <= 0.0 {
        return None;
    }
    Some((quota / period).ceil().max(1.0) as usize)
}

/// Resident working set to reserve for one concurrent recognition session.
///
/// A session owns a Tesseract API handle, the Leptonica image it recognises and
/// the intermediate page data, so the cost is per session, not per document.
/// Measured on an 84-page scanned PDF: 237 MiB peak RSS at one session, 441 MiB
/// at four and 709 MiB at eight — 68 MiB for each session the run added.
///
/// The reservation is set well above that measurement because the working set
/// scales with the page raster, and a large page at a high scan resolution costs
/// several times a letter-sized one. Reserving too much costs throughput on a
/// small host; reserving too little costs the process.
const TESSERACT_SESSION_MEMORY_BYTES: u64 = 512 * 1024 * 1024;

/// Memory this process can grow into, read afresh on every call.
///
/// This is the one memory reader in the crate. The OCR batch sizer in
/// `extractors::pdf::ocr::pipeline` reads it through `get_available_memory`
/// rather than keeping a second copy: two readers of the same three files drift,
/// and the first draft of this one had already drifted twice — it compared the
/// cgroup limit without subtracting current usage, and reported nothing at all
/// on macOS, so the memory bound was a no-op there. ~keep
///
/// Deliberately uncached. The batch sizer asks once per document, and free
/// memory moves between documents, so a cached reading would size every later
/// document in a long-lived server process from whatever happened to be free
/// during the first extraction. The recognition limit does latch, but it
/// latches the resolved session count in [`init_thread_pools`], not this
/// reading. ~keep
///
/// `None` means no limit was found: a Linux host whose `/proc` is not mounted, a
/// macOS host whose `sysctlbyname` call fails, and every other platform. Callers then apply
/// no memory bound, the same "no tighter limit was found" branch
/// [`cgroup_cpu_quota_cores`] takes. It is not a reading of zero.
pub(crate) fn available_memory_bytes() -> Option<u64> {
    available_memory_reading(read_available_memory_bytes)
}

/// The behavioral seam [`available_memory_bytes`] delegates through.
///
/// The reader is a parameter rather than a direct call so a test can prove this
/// function adds no caching of its own: inject a reader whose reading changes on
/// every call and observe that every call here reflects it, rather than
/// inspecting source text for cache-shaped identifiers. The latter missed the
/// identical process-lifetime-cache regression once already, reinstated one
/// call frame deeper than a guard that only ever read `available_memory_bytes`'s
/// own one-line body could see (GH#1751). ~keep
fn available_memory_reading(read: impl Fn() -> Option<u64>) -> Option<u64> {
    read()
}

/// Take the lower of the cgroup headroom and the host's free memory.
///
/// A container can carry both: a 64 GiB host that is nearly full still refuses
/// an allocation the 8 GiB cgroup limit would have allowed, and vice versa.
#[cfg(target_os = "linux")]
fn read_available_memory_bytes() -> Option<u64> {
    let cgroup = cgroup_headroom_bytes();
    let host = std::fs::read_to_string("/proc/meminfo")
        .ok()
        .and_then(|contents| parse_mem_available_bytes(&contents));
    match (cgroup, host) {
        (Some(cgroup), Some(host)) => Some(cgroup.min(host)),
        (limit, None) | (None, limit) => limit,
    }
}

/// What the cgroup will still hand out: its limit less what it already holds.
///
/// The limit alone is not headroom. A 8 GiB cgroup already using 6 GiB grants
/// 2 GiB, and sizing a session count against the 8 gets the process killed.
#[cfg(target_os = "linux")]
fn cgroup_headroom_bytes() -> Option<u64> {
    if let (Ok(max), Ok(current)) = (
        std::fs::read_to_string("/sys/fs/cgroup/memory.max"),
        std::fs::read_to_string("/sys/fs/cgroup/memory.current"),
    ) && let Some(headroom) = parse_cgroup_headroom(&max, &current)
    {
        return Some(headroom);
    }
    let limit = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.limit_in_bytes").ok()?;
    let usage = std::fs::read_to_string("/sys/fs/cgroup/memory/memory.usage_in_bytes").ok()?;
    parse_cgroup_headroom(&limit, &usage)
}

/// A cgroup memory limit at or above this is read as no limit at all.
///
/// cgroup v2 writes the literal `max` when no limit applies, but cgroup v1 has
/// no such word and writes a near-`i64::MAX` byte count instead. No host holds
/// four exbibytes of memory, so a number that large is the sentinel.
#[cfg(target_os = "linux")]
const CGROUP_MEMORY_UNLIMITED_FLOOR: u64 = 1 << 62;

/// Read a cgroup limit and its current usage, and return the difference.
///
/// Both spellings of "no limit" — the v2 word `max` and v1's near-`i64::MAX`
/// count — return `None`, so an unlimited cgroup leaves the host reading alone.
#[cfg(target_os = "linux")]
fn parse_cgroup_headroom(limit: &str, usage: &str) -> Option<u64> {
    let limit = limit.trim();
    if limit == "max" {
        return None;
    }
    let limit: u64 = limit.parse().ok()?;
    if limit == 0 || limit >= CGROUP_MEMORY_UNLIMITED_FLOOR {
        return None;
    }
    let usage: u64 = usage.trim().parse().ok()?;
    Some(limit.saturating_sub(usage))
}

/// Read `MemAvailable` out of `/proc/meminfo`, in bytes.
///
/// `MemAvailable` rather than `MemFree`: the kernel's own estimate of what a new
/// allocation can take already excludes the reclaimable page cache, which on a
/// busy extraction host is most of the memory `MemFree` reports as gone.
#[cfg(target_os = "linux")]
fn parse_mem_available_bytes(contents: &str) -> Option<u64> {
    let value = contents
        .lines()
        .find_map(|line| line.strip_prefix("MemAvailable:"))?
        .split_whitespace()
        .next()?
        .parse::<u64>()
        .ok()?;
    value.checked_mul(1024)
}

/// macOS publishes no per-process headroom, so take half of physical memory.
///
/// Half rather than all: `hw.memsize` is what the machine has, not what is free,
/// and the previous reader in the OCR batch sizer has used this same halving
/// since it was written. Keeping the figure identical means moving that caller
/// onto this reader does not change what it decides on a Mac. ~keep
#[cfg(target_os = "macos")]
fn read_available_memory_bytes() -> Option<u64> {
    half_of_physical_memory(sysctl_hw_memsize()?)
}

/// Half of the machine's physical memory, or `None` when the reading is zero.
#[cfg(any(target_os = "macos", test))]
fn half_of_physical_memory(total: u64) -> Option<u64> {
    (total > 0).then_some(total / 2)
}

/// Read `hw.memsize` in-process through `sysctlbyname(3)`.
///
/// Not a `sysctl -n hw.memsize` child process: `Command::output()` waits on the
/// child with `waitpid`, and a host that sets `SIGCHLD` to `SIG_IGN` (the BEAM
/// does) has the kernel reap the child itself, so the wait never returns and
/// the first extraction hangs forever (GH#2117). ~keep
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
fn sysctl_hw_memsize() -> Option<u64> {
    let mut value: u64 = 0;
    let mut len: libc::size_t = std::mem::size_of::<u64>();
    // SAFETY: the name is a NUL-terminated C string literal; `oldp` points at a live
    // `u64` and `oldlenp` at a live `size_t` holding that buffer's size, both valid
    // for writes for the duration of the call; `newp` is null with `newlen` 0, so
    // nothing is set.
    let rc = unsafe {
        libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&raw mut value).cast::<libc::c_void>(),
            &raw mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (rc == 0 && len == std::mem::size_of::<u64>()).then_some(value)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn read_available_memory_bytes() -> Option<u64> {
    None
}

/// Guard for [`warn_recognition_memory_clamp_once`], as [`DEFAULT_CAP_WARNED`].
static MEMORY_CLAMP_WARNED: AtomicBool = AtomicBool::new(false);

/// Say so when memory, not the thread budget, is what bounds recognition.
///
/// The sibling `resolve_thread_budget` warns once when its own ceiling binds.
/// Without this, a host budgeted 32 threads that runs 6 recognition sessions
/// looks like the fixed-four defect all over again, and nothing in the log
/// distinguishes "capped by memory" from "the budget was never applied".
fn warn_recognition_memory_clamp_once(already_warned: &AtomicBool, sessions: usize, thread_budget: usize) {
    if already_warned
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::warn!(
            sessions,
            thread_budget,
            session_reserve_mb = TESSERACT_SESSION_MEMORY_BYTES / (1024 * 1024),
            "free memory holds fewer concurrent OCR recognition sessions than the thread budget, \
             so recognition runs narrower than the configured budget; raise the memory available \
             to the process or set max_concurrent_ocr explicitly to silence this"
        );
    }
}

/// Resolve how many Tesseract recognition sessions may run at once.
///
/// An explicit `max_concurrent_ocr` wins outright: it is floored at one and
/// nothing else reduces it, the same way an explicit `max_threads` wins over
/// [`DEFAULT_THREAD_CAP`] in the sibling resolver. Clamping it to the thread
/// budget would put the caller back where this change found them, with a
/// number they set and a limit that ignores it. ~keep
///
/// Otherwise recognition follows the general thread budget, bounded by the
/// number of sessions the available memory holds — see
/// [`TESSERACT_SESSION_MEMORY_BYTES`] for the per-session cost this divides by.
///
/// Named for recognition rather than for OCR at large: `resolve_ocr_concurrency`
/// was the VLM-concurrency function removed under GH#1465, and a doc comment in
/// `extraction::image_ocr` still describes that one by name. ~keep
pub(crate) fn resolve_recognition_concurrency(config: Option<&ConcurrencyConfig>) -> usize {
    resolve_recognition_concurrency_with_guard(
        config,
        resolve_thread_budget(config),
        available_memory_bytes(),
        &MEMORY_CLAMP_WARNED,
    )
}

/// Pure core of [`resolve_recognition_concurrency`], parameterized on the thread
/// budget, the available memory and the warning guard so tests cover every
/// branch on any machine without touching the process-global guard.
fn resolve_recognition_concurrency_with_guard(
    config: Option<&ConcurrencyConfig>,
    thread_budget: usize,
    available_memory: Option<u64>,
    already_warned: &AtomicBool,
) -> usize {
    if let Some(requested) = config.and_then(|c| c.max_concurrent_ocr) {
        return requested.max(1);
    }
    let Some(bytes) = available_memory else {
        return thread_budget.max(1);
    };
    let memory_bound = usize::try_from(bytes / TESSERACT_SESSION_MEMORY_BYTES).unwrap_or(usize::MAX);
    let sessions = thread_budget.min(memory_bound).max(1);
    if sessions < thread_budget {
        warn_recognition_memory_clamp_once(already_warned, sessions, thread_budget);
    }
    sessions
}

/// Recognition sessions this process allows, fixed when the pools were initialized.
///
/// [`init_thread_pools`] is the one writer, so the admission semaphore in the
/// Tesseract backend and the handle pool behind it read a number no later
/// extraction can move. Reading never writes: an accessor that latched on first
/// read would install the automatic limit for whichever caller ran before
/// initialization and leave the configured value unreachable, which is the same
/// silently-ignored setting this change exists to remove. Before initialization
/// it resolves to the automatic limit, exactly as `active_thread_budget` does,
/// and the zero the static starts at is the "not initialized yet" sentinel
/// rather than a session count. ~keep
#[cfg(feature = "ocr")]
pub(crate) fn recognition_concurrency() -> usize {
    match ACTIVE_RECOGNITION_CONCURRENCY.load(Ordering::Relaxed) {
        0 => resolve_recognition_concurrency(None),
        sessions => sessions,
    }
}

/// Resolve the effective thread budget from config or auto-detection.
///
/// User-set `max_threads` takes priority. Otherwise auto-detects from
/// `num_cpus`, preferring a detected Linux cgroup CPU quota as the ceiling
/// when present, and falling back to [`DEFAULT_THREAD_CAP`] otherwise. See
/// the [`ConcurrencyConfig`] docs for the full default-budget explanation.
///
/// # Example
///
/// ```ignore
/// use xberg::core::config::ConcurrencyConfig;
/// use xberg::core::config::concurrency::resolve_thread_budget;
///
/// let config = ConcurrencyConfig { max_threads: Some(4), max_concurrent_ocr: None };
/// assert_eq!(resolve_thread_budget(Some(&config)), 4);
/// assert!(resolve_thread_budget(None) >= 1);
/// ```
pub(crate) fn resolve_thread_budget(config: Option<&ConcurrencyConfig>) -> usize {
    resolve_thread_budget_inner(config, num_cpus::get(), cgroup_cpu_quota_cores())
}

/// Resolve the asynchronous request limit for a single LLM-backed feature.
///
/// An explicit per-LLM limit takes precedence over the general extraction
/// thread budget. This keeps remote request fan-out independently tunable while
/// preserving the historical behavior for configurations that do not opt in.
#[cfg(feature = "captioning")]
pub(crate) fn resolve_llm_concurrency(
    llm_config: &crate::core::config::LlmConfig,
    concurrency: Option<&ConcurrencyConfig>,
) -> usize {
    llm_config
        .max_concurrency
        .unwrap_or_else(|| resolve_thread_budget(concurrency))
        .max(1)
}

/// Pure core of [`resolve_thread_budget`], parameterized on the host CPU
/// count and any detected cgroup quota so tests can exercise every branch
/// deterministically regardless of the machine actually running the tests.
fn resolve_thread_budget_inner(
    config: Option<&ConcurrencyConfig>,
    host_cpus: usize,
    quota_cores: Option<usize>,
) -> usize {
    resolve_thread_budget_with_guard(config, host_cpus, quota_cores, &DEFAULT_CAP_WARNED)
}

/// As [`resolve_thread_budget_inner`], but against a caller-supplied
/// "already warned" guard — see [`warn_default_thread_cap_once`] for why the
/// warning tests cannot share the process-global one.
fn resolve_thread_budget_with_guard(
    config: Option<&ConcurrencyConfig>,
    host_cpus: usize,
    quota_cores: Option<usize>,
    already_warned: &AtomicBool,
) -> usize {
    if let Some(n) = config.and_then(|c| c.max_threads) {
        return n.max(1);
    }
    match quota_cores {
        // `clamp` rather than `min().max()`: both bounds are known non-zero here
        // (`cgroup_cpu_quota_cores` floors its result at 1), so it cannot panic. ~keep
        Some(quota_cores) => host_cpus.clamp(1, quota_cores.max(1)),
        None => {
            if host_cpus > DEFAULT_THREAD_CAP {
                warn_default_thread_cap_once(already_warned, host_cpus);
            }
            host_cpus.clamp(1, DEFAULT_THREAD_CAP)
        }
    }
}

/// Internal worker/session allocation for one batch extraction.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(
        test,
        feature = "tokio-runtime",
        feature = "late-interaction",
        feature = "reranker",
        feature = "sparse-embeddings"
    )
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BatchExecutionPlan {
    pub workers: usize,
    pub thread_budget: usize,
}

/// How strongly a batch is known to exercise native layout inference.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(
        test,
        feature = "tokio-runtime",
        feature = "late-interaction",
        feature = "reranker",
        feature = "sparse-embeddings"
    )
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutBatchWorkload {
    /// No input has layout inference configured.
    None,
    /// Layout may run for only part of the batch or through a non-PDF path.
    #[cfg(layout_detection)]
    Mixed,
    /// Every input is a PDF using layout inference for Markdown extraction.
    #[cfg(layout_detection)]
    All,
}

/// Allocate batch workers and per-worker model threads without oversubscription.
///
/// The total configured budget is divided between document workers so nested
/// per-document parallelism cannot multiply the process-wide CPU budget.
/// All-layout PDF batches use one document worker with the full thread budget.
/// RT-DETR inference does not scale enough across two half-budget sessions to
/// justify their additional resident memory. Mixed or uncertain layout batches
/// retain the previous two-worker cap, while non-layout batches use the normal
/// worker ceiling. `max_concurrent` is always a ceiling and cannot expand
/// execution beyond the total thread budget.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(
        test,
        feature = "tokio-runtime",
        feature = "late-interaction",
        feature = "reranker",
        feature = "sparse-embeddings"
    )
))]
pub(crate) fn resolve_batch_execution_plan(
    config: Option<&ConcurrencyConfig>,
    layout_workload: LayoutBatchWorkload,
    input_count: usize,
    max_concurrent: Option<usize>,
) -> BatchExecutionPlan {
    #[cfg(layout_detection)]
    const MAX_NATIVE_LAYOUT_BATCH_WORKERS: usize = 1;
    #[cfg(layout_detection)]
    const MAX_MIXED_LAYOUT_BATCH_WORKERS: usize = 2;

    let total_budget = resolve_thread_budget(config);
    let available_inputs = input_count.max(1);
    let worker_ceiling = max_concurrent
        .unwrap_or(total_budget)
        .max(1)
        .min(total_budget)
        .min(available_inputs);
    let workers = match layout_workload {
        LayoutBatchWorkload::None => worker_ceiling,
        #[cfg(layout_detection)]
        LayoutBatchWorkload::Mixed => worker_ceiling.min(MAX_MIXED_LAYOUT_BATCH_WORKERS),
        #[cfg(layout_detection)]
        LayoutBatchWorkload::All => worker_ceiling.min(MAX_NATIVE_LAYOUT_BATCH_WORKERS),
    }
    .max(1);
    let thread_budget = (total_budget / workers).max(1);

    debug_assert!(workers * thread_budget <= total_budget);
    BatchExecutionPlan { workers, thread_budget }
}

/// Resolve concurrency for model-level batches outside document extraction.
#[cfg(all(
    not(target_arch = "wasm32"),
    any(feature = "late-interaction", feature = "reranker", feature = "sparse-embeddings")
))]
pub(crate) fn resolve_batch_concurrency(config: Option<&ConcurrencyConfig>, model_threads_active: bool) -> usize {
    let budget = resolve_thread_budget(config);
    if !model_threads_active {
        return budget;
    }
    let cores = num_cpus::get().max(1);
    (cores / budget).max(1).min(budget)
}

/// Guard for [`warn_recognition_limit_latched_once`], as [`DEFAULT_CAP_WARNED`].
static RECOGNITION_LATCH_WARNED: AtomicBool = AtomicBool::new(false);

/// Say so when a later extraction names a recognition limit the process has
/// already fixed.
///
/// The limit cannot follow the second caller, for the reason given on
/// [`ConcurrencyConfig::max_concurrent_ocr`]. Discarding the value in silence
/// is the defect this change removes everywhere else, so the one case that
/// survives says so in the log. ~keep
fn warn_recognition_limit_latched_once(already_warned: &AtomicBool, requested: usize, active: usize) {
    if already_warned
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        tracing::warn!(
            requested,
            active,
            "an earlier extraction in this process already fixed the concurrent OCR recognition \
             limit, so this max_concurrent_ocr is not applied; set it on the first extraction, or \
             run one process per value"
        );
    }
}

/// The process-wide limits [`init_thread_pools`] installs, and the guard for
/// the warning a later extraction gets.
///
/// The cells are borrowed rather than read from the statics directly so a test
/// can own a whole set. Asserting against the globals is not merely racy but
/// unfixably so: whichever test in the binary runs an extraction first trips
/// [`POOL_INIT`], and `#[serial]` cannot help because it only excludes other
/// `#[serial]` tests. That is the constraint [`DEFAULT_CAP_WARNED`] already
/// documents for its own guard. ~keep
struct ActiveLimits<'a> {
    once: &'a Once,
    thread_budget: &'a AtomicUsize,
    recognition: &'a AtomicUsize,
    latch_warned: &'a AtomicBool,
}

impl ActiveLimits<'_> {
    /// Store the resolved budgets, once, and report whether this call stored
    /// them. A later call keeps the installed values and warns when it asked
    /// for a different recognition limit.
    ///
    /// `build_pool` runs inside this same `call_once` fence, not after it
    /// (GH#1750). It used to run only when this call reported `true`, outside
    /// the closure: `call_once` released a second concurrent caller as soon as
    /// the winner's closure returned, before the winner had built the global
    /// Rayon pool the atomics describe. A released caller whose own extraction
    /// reached a `par_iter` before that build finished installed Rayon's
    /// *default* pool first, so the winner's own `build_global` then failed
    /// against a pool already up — silently dropping the configured thread
    /// budget for the rest of the process. Building inside the fence means no
    /// caller observes the installed atomics, and so cannot run a parallel op,
    /// before the pool they describe actually exists. ~keep
    fn install(&self, config: Option<&ConcurrencyConfig>, budget: usize, build_pool: impl FnOnce(usize)) -> bool {
        let mut installed = false;
        self.once.call_once(|| {
            installed = true;
            self.thread_budget.store(budget.max(1), Ordering::Relaxed);
            self.recognition
                .store(resolve_recognition_concurrency(config).max(1), Ordering::Relaxed);
            build_pool(budget);
        });
        if !installed && let Some(requested) = config.and_then(|c| c.max_concurrent_ocr).map(|value| value.max(1)) {
            let active = self.recognition.load(Ordering::Relaxed);
            if requested != active {
                warn_recognition_limit_latched_once(self.latch_warned, requested, active);
            }
        }
        installed
    }
}

/// Build the process-wide Rayon global pool with `budget` threads.
///
/// A build failure means a global pool already exists. The only way that can
/// happen for a caller reached through [`ActiveLimits::install`] is another
/// pool builder outside this crate's fence, since every in-crate path runs
/// this from inside `install`'s `call_once` closure. Reported at `debug!`
/// rather than escalated: the process still runs, just with whatever pool
/// got there first.
#[cfg(not(target_arch = "wasm32"))]
fn build_global_rayon_pool(budget: usize) {
    if let Err(_err) = rayon::ThreadPoolBuilder::new().num_threads(budget).build_global() {
        tracing::debug!(
            budget,
            "global rayon pool already initialized; reusing the existing pool \
             (xberg thread budget not applied)"
        );
    }
}

/// wasm32 has no OS threads for Rayon's work-stealing pool to run on, so there
/// is nothing to build here — matching the sequential fallback the other Rayon
/// call sites in this crate take on this target.
#[cfg(target_arch = "wasm32")]
fn build_global_rayon_pool(_budget: usize) {}

/// Initialize the process-wide CPU pools from `config` and return the budget.
///
/// Sizes the global Rayon pool and fixes the recognition-session limit that
/// [`recognition_concurrency`] reports. Safe to call multiple times — only the first
/// call takes effect, so a later extraction with a different `max_threads` or
/// `max_concurrent_ocr` reads back the values the first one installed. See
/// [`ConcurrencyConfig::max_concurrent_ocr`] for why the recognition limit
/// cannot follow a later extraction.
///
/// # Example
///
/// ```ignore
/// use xberg::core::config::ConcurrencyConfig;
/// use xberg::core::config::concurrency::init_thread_pools;
///
/// let config = ConcurrencyConfig { max_threads: Some(4), ..Default::default() };
/// assert_eq!(init_thread_pools(Some(&config)), 4);
/// ```
pub(crate) fn init_thread_pools(config: Option<&ConcurrencyConfig>) -> usize {
    let limits = ActiveLimits {
        once: &POOL_INIT,
        thread_budget: &ACTIVE_THREAD_BUDGET,
        recognition: &ACTIVE_RECOGNITION_CONCURRENCY,
        latch_warned: &RECOGNITION_LATCH_WARNED,
    };
    init_thread_pools_with(config, &limits, build_global_rayon_pool)
}

/// Pure core of [`init_thread_pools`], parameterized on the limits cells and
/// the pool builder so a test can install into cells it owns.
///
/// This is the entry point that must pass `config` through to
/// [`ActiveLimits::install`] rather than a stand-in `None` (GH#1751): the
/// process-global statics [`init_thread_pools`] closes over cannot be
/// asserted against directly, for the reason [`ActiveLimits`] documents on
/// its own fields, so a test exercising `install` alone can never observe
/// whether this wrapper's own call site actually forwards the caller's
/// config. ~keep
fn init_thread_pools_with(
    config: Option<&ConcurrencyConfig>,
    limits: &ActiveLimits<'_>,
    build_pool: impl FnOnce(usize),
) -> usize {
    let budget = resolve_thread_budget(config);
    limits.install(config, budget, build_pool);
    budget
}

/// Return the process thread budget selected when the shared pools were initialized.
///
/// Model backends with private pools use this value to obey the same extraction
/// budget. Before initialization, this resolves to the standard automatic limit.
#[cfg(sceptre_ocr)]
pub(crate) fn active_thread_budget() -> usize {
    match ACTIVE_THREAD_BUDGET.load(Ordering::Relaxed) {
        0 => resolve_thread_budget(None),
        budget => budget,
    }
}

#[cfg(test)]
mod tests {
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
            for input_count in 0..=12 {
                for max_concurrent in [None, Some(0), Some(1), Some(3), Some(16)] {
                    #[cfg(layout_detection)]
                    let layout_workloads = [
                        LayoutBatchWorkload::None,
                        LayoutBatchWorkload::Mixed,
                        LayoutBatchWorkload::All,
                    ];
                    #[cfg(not(layout_detection))]
                    let layout_workloads = [LayoutBatchWorkload::None];
                    for layout_workload in layout_workloads {
                        let plan =
                            resolve_batch_execution_plan(Some(&config), layout_workload, input_count, max_concurrent);
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
}
