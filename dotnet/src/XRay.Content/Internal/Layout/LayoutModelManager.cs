using System.Net.Http;
using System.Security.Cryptography;

using XRay.Content.Core;

namespace XRay.Content.Internal.Layout;

/// <summary>One model the layout runtime can fetch, pinned by repository revision and digest.</summary>
/// <param name="ModelType">The name callers ask for.</param>
/// <param name="HfRepoId">Hugging Face repository.</param>
/// <param name="HfRevision">The exact commit this build is pinned to.</param>
/// <param name="RemoteFilename">Path within the repository.</param>
/// <param name="LocalFilename">File name under the staging cache.</param>
/// <param name="Sha256">Expected digest of the file's bytes.</param>
/// <param name="SizeBytes">Expected size, used to reject a truncated cache entry cheaply.</param>
internal readonly record struct LayoutModelDefinition(
    string ModelType,
    string HfRepoId,
    string HfRevision,
    string RemoteFilename,
    string LocalFilename,
    string Sha256,
    long SizeBytes);

/// <summary>
/// Download and cache the layout models, ported from Rust <c>layout/model_manager.rs</c>.
/// </summary>
/// <remarks>
/// Every model is pinned to a repository revision <em>and</em> a SHA-256 digest, and the digest
/// is what decides: a cached file of the right size that hashes wrong is re-fetched, and a
/// download that hashes wrong never reaches the destination. Publication is atomic — the file
/// lands under a staging name and is renamed into place — so a concurrent reader sees either the
/// old file or the new one, never a half-written one.
/// </remarks>
internal sealed class LayoutModelManager
{
    /// <summary>Gives each publish a unique staging name, so concurrent publishes cannot collide.</summary>
    private static long _publishCounter;

    /// <summary>Serialises publishes of the same destination within this process.</summary>
    private static readonly Dictionary<string, SemaphoreSlim> PublishLocks = new(StringComparer.Ordinal);

    public static readonly LayoutModelDefinition[] Models =
    {
        new("rtdetr", "xberg-io/layout-models", "c6bf493e2f7b0b9a29a5870da9880c14e20ff0a3",
            "rtdetr/model.onnx", "model.onnx",
            "3bf2fb0ee6df87435b7ae47f0f3930ec3dc97ec56fd824acc6d57bc7a6b89ef2", 169_089_059),
        new("tatr", "xberg-io/layout-models", "c6bf493e2f7b0b9a29a5870da9880c14e20ff0a3",
            "tatr/model.onnx", "tatr.onnx",
            "c11f4033da75e9c4d41c403ef356e89caa0a37a7d111b55461e7d5ba856bb6b6", 30_158_413),
        new("slanet_wired", "xberg-io/paddleocr-onnx-models", "bfaf0b492cfc1dee0c73245fc5860bfdcf2c3443",
            "v2/table/SLANeXt_wired.onnx", "slanet_wired.onnx",
            "64990fa026a7e2e2c2d4ad2c810bc9c6992da76d5f91b54771dfc900927ca3d0", 365_355_622),
        new("slanet_wireless", "xberg-io/paddleocr-onnx-models", "bfaf0b492cfc1dee0c73245fc5860bfdcf2c3443",
            "v2/table/SLANeXt_wireless.onnx", "slanet_wireless.onnx",
            "b29ae2b4fe0ff8bbf7efd73fda0951227eb1abaedcaa046ad016191c779b7766", 365_355_622),
        new("slanet_plus", "xberg-io/paddleocr-onnx-models", "bfaf0b492cfc1dee0c73245fc5860bfdcf2c3443",
            "v2/table/SLANet_plus.onnx", "slanet_plus.onnx",
            "e48a401a4ebcddd47fe3822427db24d867a557324f58e438692f588bbe9231de", 7_781_309),
        new("table_classifier", "xberg-io/paddleocr-onnx-models", "bfaf0b492cfc1dee0c73245fc5860bfdcf2c3443",
            "v2/classifiers/PP-LCNet_x1_0_table_cls.onnx", "table_cls.onnx",
            "f02bf087e924dadfb109e3b7887d7d56dc961b80e08c64cacf1030f97345b3c3", 6_775_213),
        new("pp_doclayout_v3", "xberg-io/layout-models", "c6bf493e2f7b0b9a29a5870da9880c14e20ff0a3",
            "pp_doclayout_v3/model.onnx", "pp_doclayout_v3.onnx",
            "93d1197e55f1c9cb6720275a89684e7ea61cd5830008a837d8c51b19d47926c1", 131_731_131),
    };

    private readonly string _cacheDir;
    private readonly bool _stageInLocalCache;
    private readonly XRayOptions _options;

    /// <summary>
    /// A manager over the standard Hugging Face cache, or over an explicit directory.
    /// </summary>
    /// <param name="cacheDir">
    /// <c>null</c> uses the Hugging Face cache directly, without duplicating model files into a
    /// second cache. An explicit directory keeps the standalone staged layout, for callers that
    /// need the files somewhere they control.
    /// </param>
    /// <param name="options">
    /// Supplies the cache root when <paramref name="cacheDir"/> is <c>null</c>, and whether
    /// downloads are allowed at all. Defaults to <see cref="XRayOptions.Default"/>.
    /// </param>
    public LayoutModelManager(string? cacheDir = null, XRayOptions? options = null)
    {
        _options = options ?? XRayOptions.Default;
        if (cacheDir is null)
        {
            _cacheDir = _options.ModelCacheDirectory ?? DefaultHfCacheDir();
            _stageInLocalCache = false;
        }
        else
        {
            _cacheDir = cacheDir;
            _stageInLocalCache = true;
        }
    }

    public string CacheDir => _cacheDir;

    /// <summary>Every pinned (repository, revision) pair, deduplicated.</summary>
    /// <remarks>A revision is only meaningful within its own repository, so the pair is the unit.</remarks>
    public static List<(string Repo, string Revision)> PinnedRepoRevisions()
    {
        var seen = new HashSet<(string, string)>();
        var pairs = new List<(string, string)>();
        foreach (var model in Models)
            if (seen.Add((model.HfRepoId, model.HfRevision)))
                pairs.Add((model.HfRepoId, model.HfRevision));
        return pairs;
    }

    public Task<string> EnsureRtDetrModelAsync(CancellationToken ct = default) => EnsureModelAsync("rtdetr", ct);
    public Task<string> EnsureTatrModelAsync(CancellationToken ct = default) => EnsureModelAsync("tatr", ct);
    public Task<string> EnsureSlanetModelAsync(string variant, CancellationToken ct = default) =>
        EnsureModelAsync(variant, ct);
    public Task<string> EnsureTableClassifierAsync(CancellationToken ct = default) =>
        EnsureModelAsync("table_classifier", ct);
    public Task<string> EnsurePpDocLayoutV3ModelAsync(CancellationToken ct = default) =>
        EnsureModelAsync("pp_doclayout_v3", ct);

    public bool IsRtDetrCached() => IsModelCached("rtdetr");
    public bool IsTatrCached() => IsModelCached("tatr");
    public bool IsPpDocLayoutV3Cached() => IsModelCached("pp_doclayout_v3");

    /// <summary>Find a model locally, downloading it if it is not there or does not verify.</summary>
    public async Task<string> EnsureModelAsync(string modelType, CancellationToken ct = default)
    {
        var definition = Find(modelType);

        if (!_stageInLocalCache) return await ResolveVerifiedHfModelAsync(definition, ct).ConfigureAwait(false);

        string modelDir = Path.Combine(_cacheDir, modelType);
        string modelFile = Path.Combine(modelDir, definition.LocalFilename);

        // A staged copy that already verifies is used as-is; anything else is re-resolved.
        if (File.Exists(modelFile) && VerifyFile(modelFile, definition.Sha256)) return modelFile;

        string source = await ResolveVerifiedHfModelAsync(definition, ct).ConfigureAwait(false);
        Directory.CreateDirectory(modelDir);
        AtomicPublish(source, modelFile, modelDir, definition.Sha256, modelType);
        return modelFile;
    }

    /// <summary>Whether a verified copy is already on disk, with no network access.</summary>
    public bool IsModelCached(string modelType)
    {
        var definition = Find(modelType);
        string path = _stageInLocalCache
            ? Path.Combine(_cacheDir, modelType, definition.LocalFilename)
            : Path.Combine(_cacheDir, HfCacheRelativePath(definition));
        return File.Exists(path) && VerifyFile(path, definition.Sha256);
    }

    private static LayoutModelDefinition Find(string modelType)
    {
        foreach (var model in Models)
            if (model.ModelType == modelType) return model;
        throw new InvalidOperationException($"Unknown model type: {modelType}");
    }

    /// <summary>
    /// The path a file occupies inside a Hugging Face cache: the repository name with slashes
    /// doubled into dashes, then the revision snapshot, then the file's own path in the repo.
    /// </summary>
    internal static string HfCacheRelativePath(LayoutModelDefinition definition)
    {
        string repo = definition.HfRepoId.Replace("/", "--");
        return Path.Combine($"models--{repo}", "snapshots", definition.HfRevision,
                            definition.RemoteFilename.Replace('/', Path.DirectorySeparatorChar));
    }

    /// <summary>The Hugging Face cache root used when no other directory is configured.</summary>
    /// <remarks>
    /// The hub's own precedence — <c>HF_HUB_CACHE</c>, <c>HUGGINGFACE_HUB_CACHE</c>,
    /// <c>HF_HOME</c>, <c>XDG_CACHE_HOME</c> — lives in <see cref="XRayOptions.FromEnvironment"/>,
    /// because no library code here reads ambient process state. What is left is the location
    /// the hub falls back to, which is a fixed path.
    /// </remarks>
    internal static string DefaultHfCacheDir() => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
        ".cache", "huggingface", "hub");

    /// <summary>
    /// A verified copy of a model in the Hugging Face cache, fetching it if needed.
    /// </summary>
    /// <remarks>
    /// Cache-first, always: the network is only reached when there is no local copy or the local
    /// copy fails its digest. A bad cached entry is replaced rather than trusted, because a
    /// truncated download that was interrupted mid-write looks exactly like a good one to a
    /// size check alone.
    /// </remarks>
    private async Task<string> ResolveVerifiedHfModelAsync(
        LayoutModelDefinition definition, CancellationToken ct)
    {
        string path = Path.Combine(_cacheDir, HfCacheRelativePath(definition));

        if (File.Exists(path) && VerifyFile(path, definition.Sha256)) return path;

        if (_options.ModelDownloadsDisabled)
            throw new InvalidOperationException(
                $"Model '{definition.ModelType}' is not in the cache at {path} and model downloads "
                + "are disabled, so it cannot be fetched");

        string? dir = Path.GetDirectoryName(path);
        if (dir is not null) Directory.CreateDirectory(dir);

        string url = $"https://huggingface.co/{definition.HfRepoId}/resolve/{definition.HfRevision}/"
                     + definition.RemoteFilename;

        string staged = StagingPath(path);
        try
        {
            using (var client = new HttpClient { Timeout = TimeSpan.FromMinutes(30) })
            using (var response = await client.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, ct)
                                              .ConfigureAwait(false))
            {
                response.EnsureSuccessStatusCode();
                await using var source = await response.Content.ReadAsStreamAsync(ct).ConfigureAwait(false);
                await using var destination = File.Create(staged);
                await source.CopyToAsync(destination, ct).ConfigureAwait(false);
            }

            // Verified before publication, so a bad download never becomes the cached copy.
            if (!VerifyFile(staged, definition.Sha256))
                throw new InvalidOperationException(
                    $"Downloaded model '{definition.ModelType}' does not match its pinned SHA-256; "
                    + "the file was discarded rather than cached");

            AtomicPublish(staged, path, dir ?? ".", definition.Sha256, definition.ModelType);
        }
        finally
        {
            if (File.Exists(staged)) File.Delete(staged);
        }

        return path;
    }

    private static string StagingPath(string destination)
    {
        long id = Interlocked.Increment(ref _publishCounter);
        return destination + $".tmp{Environment.ProcessId}-{id}";
    }

    /// <summary>
    /// Move a staged file into place, keeping the old one recoverable until the new one verifies.
    /// </summary>
    /// <remarks>
    /// A rename is atomic within a filesystem, so a concurrent reader sees the old file or the
    /// new one and never a partial write. The destination is moved aside first rather than
    /// overwritten, so a rename that succeeds but leaves a file that does not verify can be
    /// rolled back to what was there before.
    /// </remarks>
    internal static void AtomicPublish(
        string source, string destination, string destinationDir, string sha256, string label)
    {
        Directory.CreateDirectory(destinationDir);

        var gate = LockFor(destination);
        gate.Wait();
        try
        {
            string backup = destination + ".backup";
            bool hadDestination = File.Exists(destination);

            if (hadDestination)
            {
                if (File.Exists(backup)) File.Delete(backup);
                File.Move(destination, backup);
            }

            try
            {
                File.Move(source, destination, overwrite: false);
                if (!VerifyFile(destination, sha256))
                    throw new InvalidOperationException(
                        $"Published model '{label}' does not match its pinned SHA-256");

                if (File.Exists(backup)) File.Delete(backup);
            }
            catch (Exception) when (Cancel.NotCancelling)
            {
                // Put back whatever was there, so a failed publish leaves a working cache rather
                // than an empty one.
                if (File.Exists(destination)) File.Delete(destination);
                if (hadDestination && File.Exists(backup)) File.Move(backup, destination);
                throw;
            }
        }
        finally
        {
            gate.Release();
        }
    }

    private static SemaphoreSlim LockFor(string path)
    {
        lock (PublishLocks)
        {
            if (!PublishLocks.TryGetValue(path, out var gate))
            {
                gate = new SemaphoreSlim(1, 1);
                PublishLocks[path] = gate;
            }
            return gate;
        }
    }

    /// <summary>Whether a file's contents hash to the expected digest.</summary>
    internal static bool VerifyFile(string path, string expectedSha256)
    {
        try
        {
            using var stream = File.OpenRead(path);
            byte[] hash = SHA256.HashData(stream);
            return Convert.ToHexStringLower(hash) == expectedSha256.ToLowerInvariant();
        }
        catch (IOException)
        {
            return false;
        }
    }
}
