using SixLabors.ImageSharp;
using SixLabors.ImageSharp.PixelFormats;

namespace XRay.Content.Internal.Layout;

/// <summary>Which detector a layout engine runs.</summary>
internal enum LayoutBackend
{
    /// <summary>RT-DETR v2, seventeen classes at 640×640, and free of duplicate boxes.</summary>
    RtDetr,

    /// <summary>PP-DocLayout-V3, twenty-five classes at 800×800.</summary>
    PpDocLayoutV3,

    /// <summary>A model file the caller supplies, whose architecture they name.</summary>
    Custom,
}

/// <summary>Which architecture a caller-supplied model file follows.</summary>
internal enum CustomModelVariant
{
    RtDetr,
    PpDocLayoutV3,

    /// <summary>YOLO trained on DocLayNet: eleven classes, 640×640.</summary>
    YoloDocLayNet,

    /// <summary>DocLayout-YOLO trained on DocStructBench: ten classes, 1024×1024.</summary>
    YoloDocStructBench,

    /// <summary>YOLOX, whose input dimensions the caller states.</summary>
    Yolox,
}

/// <summary>How a layout engine is set up.</summary>
/// <remarks>
/// A record so two configurations can be compared: an engine holds a loaded model, and whether
/// a cached one can be reused is exactly the question of whether its configuration matches.
/// </remarks>
internal sealed record LayoutEngineConfig
{
    public LayoutBackend Backend { get; init; } = LayoutBackend.RtDetr;

    /// <summary>The model file, for <see cref="LayoutBackend.Custom"/>.</summary>
    public string? ModelPath { get; init; }

    /// <summary>The architecture of that file.</summary>
    public CustomModelVariant Variant { get; init; } = CustomModelVariant.RtDetr;

    /// <summary>Input width for a YOLOX model, which is not square.</summary>
    public int YoloxInputWidth { get; init; } = 768;

    /// <summary>Input height for a YOLOX model.</summary>
    public int YoloxInputHeight { get; init; } = 1024;

    /// <summary>Confidence threshold, or <c>null</c> for the model's own default.</summary>
    public float? ConfidenceThreshold { get; init; }

    /// <summary>Whether to run the postprocessing heuristics over what the model found.</summary>
    public bool ApplyHeuristics { get; init; } = true;

    /// <summary>Where model files are cached, or <c>null</c> for the standard location.</summary>
    public string? CacheDirectory { get; init; }
}

/// <summary>
/// Layout detection: model loading, inference and postprocessing behind one object.
/// </summary>
/// <remarks>
/// <para>
/// Ports Rust <c>layout::engine</c>. The models differ in what they need — one wants the page
/// size alongside the pixels, one decodes anchors against a grid, one is already free of
/// duplicates — and this is where those differences stop mattering to a caller.
/// </para>
/// <para>
/// The heuristics run here rather than inside a model because they are about the page rather
/// than the architecture: a header near the bottom of the page is wrong whichever detector
/// found it.
/// </para>
/// </remarks>
internal sealed class LayoutEngine
{
    private readonly LayoutEngineConfig _config;
    private readonly RtDetrModel? _rtDetr;
    private readonly PpDocLayoutV3Model? _ppDocLayout;
    private readonly YoloModel? _yolo;

    private LayoutEngine(
        LayoutEngineConfig config,
        RtDetrModel? rtDetr = null,
        PpDocLayoutV3Model? ppDocLayout = null,
        YoloModel? yolo = null)
    {
        _config = config;
        _rtDetr = rtDetr;
        _ppDocLayout = ppDocLayout;
        _yolo = yolo;
    }

    /// <summary>The configuration this engine was built from.</summary>
    public LayoutEngineConfig Config => _config;

    /// <summary>Whether a cached engine can serve this configuration.</summary>
    public bool Matches(LayoutEngineConfig config) => _config == config;

    /// <summary>The name of the model behind this engine.</summary>
    public string ModelName =>
        _rtDetr?.Name ?? _ppDocLayout?.Name ?? _yolo?.Name
        ?? throw new InvalidOperationException("Layout engine holds no model");

    /// <summary>
    /// Build an engine, downloading and verifying the model when one is not supplied.
    /// </summary>
    public static async Task<LayoutEngine> CreateAsync(
        LayoutEngineConfig config, Core.XRayOptions? options = null,
        CancellationToken cancellationToken = default)
    {
        if (config.Backend == LayoutBackend.Custom)
        {
            if (string.IsNullOrEmpty(config.ModelPath))
                throw new InvalidOperationException(
                    "A custom layout backend needs the path of the model file to load");
            return FromFile(config, config.ModelPath);
        }

        var manager = new LayoutModelManager(config.CacheDirectory, options);
        string modelType = config.Backend == LayoutBackend.RtDetr ? "rtdetr" : "pp_doclayout_v3";
        string path = await manager.EnsureModelAsync(modelType, cancellationToken).ConfigureAwait(false);

        return config.Backend == LayoutBackend.RtDetr
            ? new LayoutEngine(config, rtDetr: RtDetrModel.FromFile(path))
            : new LayoutEngine(config, ppDocLayout: PpDocLayoutV3Model.FromFile(path));
    }

    /// <summary>Build an engine over a model file the caller already has.</summary>
    public static LayoutEngine FromFile(LayoutEngineConfig config, string path) => config.Variant switch
    {
        CustomModelVariant.RtDetr => new LayoutEngine(config, rtDetr: RtDetrModel.FromFile(path)),
        CustomModelVariant.PpDocLayoutV3 =>
            new LayoutEngine(config, ppDocLayout: PpDocLayoutV3Model.FromFile(path)),
        CustomModelVariant.YoloDocLayNet => new LayoutEngine(config, yolo: YoloModel.FromFile(
            path, YoloVariant.DocLayNet, 640, 640, "Custom-YOLO-DocLayNet")),
        CustomModelVariant.YoloDocStructBench => new LayoutEngine(config, yolo: YoloModel.FromFile(
            path, YoloVariant.DocStructBench, 1024, 1024, "Custom-DocLayout-YOLO")),
        CustomModelVariant.Yolox => new LayoutEngine(config, yolo: YoloModel.FromFile(
            path, YoloVariant.Yolox, config.YoloxInputWidth, config.YoloxInputHeight, "Custom-YOLOX")),
        _ => throw new InvalidOperationException($"Unknown custom model variant {config.Variant}"),
    };

    /// <summary>Detect the layout regions of one page.</summary>
    public DetectionResult Detect(Image<Rgb24> page) => DetectBatch([page])[0];

    /// <summary>
    /// <see cref="Detect(Image{Rgb24})"/>, cancellable: the token is polled between the network's
    /// operators and its parallel tiles, so a cancelled detection stops within one operator.
    /// </summary>
    public DetectionResult Detect(Image<Rgb24> page, CancellationToken cancellationToken) =>
        DetectBatch([page], cancellationToken)[0];

    /// <summary><see cref="DetectBatch(IReadOnlyList{Image{Rgb24}})"/>, cancellable.</summary>
    public IReadOnlyList<DetectionResult> DetectBatch(IReadOnlyList<Image<Rgb24>> pages, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        using var scope = Cancel.Begin(cancellationToken);
        return DetectBatch(pages);
    }

    /// <summary>
    /// Detect over several pages at once.
    /// </summary>
    /// <remarks>
    /// Only the transformer detectors batch: their weights dominate the runtime and are re-read
    /// per call, so stacking pages amortises that traffic. YOLO runs a page at a time, which is
    /// what the Rust does too.
    /// </remarks>
    public IReadOnlyList<DetectionResult> DetectBatch(IReadOnlyList<Image<Rgb24>> pages)
    {
        if (pages.Count == 0) return [];
        Cancel.ThrowIfRequested();

        IReadOnlyList<IReadOnlyList<LayoutDetection>> perPage;
        if (_rtDetr is not null)
        {
            perPage = _config.ConfidenceThreshold is { } threshold
                ? _rtDetr.DetectBatch(pages, threshold)
                : _rtDetr.DetectBatch(pages);
        }
        else if (_ppDocLayout is not null)
        {
            perPage = _config.ConfidenceThreshold is { } threshold
                ? _ppDocLayout.DetectBatch(pages, threshold)
                : _ppDocLayout.DetectBatch(pages);
        }
        else if (_yolo is not null)
        {
            var found = new List<IReadOnlyList<LayoutDetection>>(pages.Count);
            foreach (var page in pages)
                found.Add(_config.ConfidenceThreshold is { } threshold
                    ? _yolo.Detect(page, threshold)
                    : _yolo.Detect(page));
            perPage = found;
        }
        else
        {
            throw new InvalidOperationException("Layout engine holds no model");
        }

        var results = new List<DetectionResult>(pages.Count);
        for (int i = 0; i < pages.Count; i++)
        {
            var detections = perPage[i];
            if (_config.ApplyHeuristics)
                detections = LayoutPostprocessing.ApplyHeuristics(
                    [.. detections], pages[i].Width, pages[i].Height);
            results.Add(new DetectionResult(pages[i].Width, pages[i].Height, detections));
        }
        return results;
    }
}
