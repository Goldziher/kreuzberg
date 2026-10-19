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
mod tests;
