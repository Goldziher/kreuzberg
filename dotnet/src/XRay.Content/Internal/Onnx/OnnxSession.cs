using XRay.Content.Internal.Onnx.Ops;

namespace XRay.Content.Internal.Onnx;

/// <summary>
/// Executes a parsed <see cref="OnnxModel"/>.
/// <para>
/// ONNX stores nodes in topological order, so execution is a single forward pass with a
/// name-to-tensor environment. The one thing the pass does beyond dispatch is <em>free
/// intermediates as soon as their last consumer has run</em>: RT-DETR's live values total
/// well over a gigabyte if every activation is retained to the end, and keeping only the
/// live set turns that into a working set the collector can actually recycle.
/// </para>
/// <para>
/// This is deliberately not a general ONNX runtime. It covers the operators the layout
/// models use, at the opset they were exported with, and throws on anything else rather
/// than approximating it — a wrong-but-plausible kernel is far more expensive to find than
/// a missing one.
/// </para>
/// </summary>
internal sealed class OnnxSession
{
    private readonly OnnxModel _model;
    /// <summary>For each node index, the values whose last use is that node.</summary>
    private readonly List<string>[] _lastUse;
    /// <summary>Recycled activation storage, reused across runs of this session.</summary>
    private readonly TensorPool _pool = new();

    /// <summary>The buffer pool, for tests and profiling to inspect its hit rate.</summary>
    internal TensorPool Pool => _pool;

    /// <param name="optimize">
    /// Rewrite the graph before executing it (see <see cref="GraphOptimizer"/>). Fusing
    /// removes intermediate values, so the parity harness turns this off when it needs to
    /// compare every node against a reference dump; whole-graph outputs are unaffected either
    /// way.
    /// </param>
    /// <param name="reuseBuffers">
    /// Let an element-wise node write its result over an operand that dies at it. Off is the
    /// same graph computed into fresh buffers throughout, which is what the reuse is checked
    /// against — see <c>--check-reuse</c> in <c>tools/XRay.Content.OnnxParity</c>.
    /// </param>
    public OnnxSession(OnnxModel model, bool optimize = true, bool reuseBuffers = true)
    {
        _model = optimize ? GraphOptimizer.Optimize(model) : model;
        _lastUse = ComputeLastUse(_model);
        _reuseBuffers = reuseBuffers;
    }

    private readonly bool _reuseBuffers;

    public static OnnxSession Load(string path) => new(OnnxModel.Load(path));

    public OnnxModel Model => _model;

    /// <summary>Names of the values the caller must supply.</summary>
    public IEnumerable<string> InputNames => _model.FeedInputs.Select(i => i.Name);

    /// <summary>Names the graph declares as outputs, in declaration order.</summary>
    public IReadOnlyList<string> OutputNames => _model.Outputs.Select(o => o.Name).ToArray();

    /// <summary>
    /// Run the graph and return its declared outputs.
    /// </summary>
    public Dictionary<string, Tensor> Run(IReadOnlyDictionary<string, Tensor> feeds)
        => Run(feeds, capture: null);

    /// <summary>
    /// Run the graph, optionally capturing every intermediate value.
    /// <para>
    /// <paramref name="capture"/> is what the parity harness uses: with it set, no value is
    /// released early and the caller sees the same per-node tensors the Python reference
    /// dumps, so a divergence is attributed to the first node that produced it.
    /// </para>
    /// </summary>
    public Dictionary<string, Tensor> Run(
        IReadOnlyDictionary<string, Tensor> feeds, Dictionary<string, Tensor>? capture) =>
        Run(feeds, capture, profile: null);

    /// <summary>
    /// Per-node timings from one execution, for attributing runtime to individual nodes
    /// rather than to operator types. An aggregate says <c>Conv</c> is expensive; only the
    /// per-node view says <em>which</em> convolution, at what shape, and therefore what to do
    /// about it.
    /// </summary>
    public sealed class ExecutionProfile
    {
        public required double[] NodeMicroseconds { get; init; }
        /// <summary>First output's shape per node, for reading the cost alongside the size.</summary>
        public required string[] NodeOutputShapes { get; init; }

        /// <summary>Whether each node wrote its result over an operand rather than a new buffer.</summary>
        public bool[]? NodeReusedOperand { get; init; }
    }

    /// <summary>
    /// Run the graph, optionally capturing intermediates and/or recording per-node timings.
    /// </summary>
    public Dictionary<string, Tensor> Run(
        IReadOnlyDictionary<string, Tensor> feeds,
        Dictionary<string, Tensor>? capture,
        ExecutionProfile? profile)
    {
        // Buffers are recycled across runs, so the steady state allocates almost nothing;
        // capture mode keeps everything alive, so it opts out.
        using var pooling = capture is null ? _pool.Activate() : default;

        var env = new Dictionary<string, Tensor>(StringComparer.Ordinal);
        foreach (var (name, tensor) in _model.Initializers) Bind(env, name, tensor);
        foreach (var (name, tensor) in feeds) Bind(env, name, tensor);

        foreach (var input in _model.FeedInputs)
        {
            if (!env.ContainsKey(input.Name))
                throw new InvalidOperationException($"onnx: missing input '{input.Name}'");
        }

        var declaredOutputs = new HashSet<string>(OutputNames, StringComparer.Ordinal);

        for (int i = 0; i < _model.Nodes.Length; i++)
        {
            var node = _model.Nodes[i];
            Tensor?[] outputs;
            long startTicks = profile is null ? 0 : System.Diagnostics.Stopwatch.GetTimestamp();
            try
            {
                // A capturing run has to keep every intermediate, so nothing may be overwritten.
                var reusable = _reuseBuffers && capture is null
                    ? FindReusableOperand(node, env, _lastUse[i], declaredOutputs)
                    : null;
                outputs = Execute(node, env, reusable);

                if (profile?.NodeReusedOperand is { } reused && reusable is not null)
                    reused[i] = outputs.Length > 0
                                && ReferenceEquals(outputs[0]?.Buffer, reusable.Buffer);
            }
            catch (Exception ex) when (ex is not NotSupportedException && Cancel.NotCancelling)
            {
                throw new InvalidOperationException(
                    $"onnx: node #{i} {node.OpType} ('{node.Name}') failed: {ex.Message}", ex);
            }

            if (profile is not null)
            {
                profile.NodeMicroseconds[i] = (System.Diagnostics.Stopwatch.GetTimestamp() - startTicks)
                                              * 1_000_000.0 / System.Diagnostics.Stopwatch.Frequency;
                profile.NodeOutputShapes[i] = outputs.Length > 0 && outputs[0] is { } first
                    ? $"[{string.Join(",", first.Shape)}]"
                    : "";
            }

            for (int o = 0; o < node.Outputs.Length && o < outputs.Length; o++)
            {
                if (node.Outputs[o].Length == 0 || outputs[o] is null) continue;
                Bind(env, node.Outputs[o], outputs[o]!);
                capture?[node.Outputs[o]] = outputs[o]!;
            }

            if (capture is null)
            {
                foreach (string dead in _lastUse[i])
                    if (!declaredOutputs.Contains(dead)) Unbind(env, dead);
            }
        }

        var result = new Dictionary<string, Tensor>(StringComparer.Ordinal);
        foreach (string name in OutputNames)
        {
            if (env.TryGetValue(name, out var tensor)) result[name] = tensor;
            else throw new InvalidOperationException($"onnx: graph output '{name}' was never produced");
        }
        return result;
    }

    /// <summary>
    /// Bind a value to a name, taking one reference on its storage.
    /// <para>
    /// The reference belongs to the <em>name</em>, not the tensor, which is what keeps views
    /// safe: <c>Identity</c> binds the same tensor under a second name and <c>Reshape</c>
    /// binds a different tensor over the same array, and in both cases the storage now has
    /// two holders and must survive the first of them dying.
    /// </para>
    /// </summary>
    private static void Bind(Dictionary<string, Tensor> env, string name, Tensor tensor)
    {
        if (env.TryGetValue(name, out var existing)) existing.Buffer?.Release();
        tensor.Buffer?.AddReference();
        env[name] = tensor;
    }

    /// <summary>Drop a name and the reference it held.</summary>
    private static void Unbind(Dictionary<string, Tensor> env, string name)
    {
        if (!env.Remove(name, out var tensor)) return;
        tensor.Buffer?.Release();
    }

    /// <summary>
    /// An operand this node may write its output over, or <c>null</c> when there is none.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The element-wise operators are memory-bound: they read a buffer and write another one
    /// the same size, and in this graph they are 17% of runtime across four hundred nodes. When
    /// the operand they read is dead the moment they are done with it, the second buffer is
    /// pure waste — the result can go back over the input, whose lines the producing node left
    /// in cache.
    /// </para>
    /// <para>
    /// Five conditions have to hold together, and each rules out a way this would be wrong:
    /// the pool owns the array (an initializer is a constant shared by every run and a feed
    /// belongs to the caller); exactly one value holds it (<c>Identity</c> and <c>Reshape</c>
    /// bind a second name over the same storage, and that name would see the overwrite); the
    /// name dies at this node (anything later reading it would read the result instead); it is
    /// not a graph output (which the caller is about to be handed); and this node names it once
    /// (an operand named twice is held once but read twice, so a count alone would call it
    /// free).
    /// </para>
    /// </remarks>
    private static Tensor? FindReusableOperand(
        OnnxNode node, Dictionary<string, Tensor> env,
        List<string> dying, HashSet<string> declaredOutputs)
    {
        if (dying.Count == 0) return null;

        foreach (string input in node.Inputs)
        {
            if (input.Length == 0) continue;
            if (!dying.Contains(input)) continue;
            if (declaredOutputs.Contains(input)) continue;
            if (!env.TryGetValue(input, out var tensor)) continue;
            if (!tensor.IsFloat) continue;
            if (tensor.Buffer is not { IsPooled: true, References: 1 }) continue;

            // An operand named twice by the same node is held once but read twice; writing
            // over it while the other read is still to come would corrupt the second.
            int uses = 0;
            foreach (string other in node.Inputs) if (other == input) uses++;
            if (uses != 1) continue;

            return tensor;
        }

        return null;
    }

    /// <summary>
    /// For each node, the values that are consumed for the last time by it. Graph outputs and
    /// initializers are excluded — the former are the result, the latter are shared constants.
    /// </summary>
    private static List<string>[] ComputeLastUse(OnnxModel model)
    {
        var lastUse = new List<string>[model.Nodes.Length];
        for (int i = 0; i < lastUse.Length; i++) lastUse[i] = [];

        var lastIndex = new Dictionary<string, int>(StringComparer.Ordinal);
        for (int i = 0; i < model.Nodes.Length; i++)
        {
            foreach (string input in model.Nodes[i].Inputs)
                if (input.Length > 0) lastIndex[input] = i;

            // A subgraph reads outer-scope values by name, so a control-flow node keeps alive
            // everything its body touches. Without this, a value used only inside a loop is
            // released at its last *top-level* use and the loop finds it gone.
            foreach (var attribute in model.Nodes[i].Attributes)
                if (attribute.Graph is { } subgraph) MarkSubgraphReads(subgraph, i, lastIndex);
        }

        foreach (var (name, index) in lastIndex)
            if (!model.Initializers.ContainsKey(name)) lastUse[index].Add(name);
        return lastUse;
    }

    /// <summary>
    /// Attribute every name a subgraph reads to the control-flow node that owns it, recursing
    /// through nested subgraphs.
    /// </summary>
    /// <remarks>
    /// Names the subgraph defines for itself are recorded too. That is deliberate: they are never
    /// bound in the outer environment, so releasing them is a no-op, and separating them out
    /// would buy nothing but a way to get the analysis subtly wrong.
    /// </remarks>
    private static void MarkSubgraphReads(OnnxSubgraph graph, int nodeIndex, Dictionary<string, int> lastIndex)
    {
        foreach (var inner in graph.Nodes)
        {
            foreach (string input in inner.Inputs)
                if (input.Length > 0) lastIndex[input] = nodeIndex;
            foreach (var attribute in inner.Attributes)
                if (attribute.Graph is { } nested) MarkSubgraphReads(nested, nodeIndex, lastIndex);
        }
    }

    /// <summary>Fetch an operand, or null for an omitted optional input.</summary>
    /// <summary>
    /// ONNX <c>Loop</c>: run the body subgraph until the trip count runs out or its condition
    /// goes false.
    /// </summary>
    /// <remarks>
    /// <para>
    /// The body's formal inputs are, in order: the iteration number, the incoming condition, and
    /// one per loop-carried value. Its outputs are the outgoing condition, the updated
    /// loop-carried values, and then any scan outputs — per-iteration results that are stacked
    /// along a new leading axis when the loop finishes.
    /// </para>
    /// <para>
    /// Both the trip count and the initial condition are optional, and an omitted one means
    /// "unbounded" rather than zero: a loop with neither would run forever, which the spec allows
    /// and a body's own condition is expected to stop.
    /// </para>
    /// </remarks>
    private Tensor?[] ExecuteLoop(OnnxNode node, Dictionary<string, Tensor> env)
    {
        var body = node.Attr("body")?.Graph
            ?? throw new InvalidDataException("Loop: no body subgraph");

        var tripCountTensor = Optional(node, env, 0);
        long tripCount = tripCountTensor is { Count: > 0 } ? tripCountTensor.GetLong(0) : long.MaxValue;

        var conditionTensor = Optional(node, env, 1);
        bool condition = conditionTensor is not { Count: > 0 } || conditionTensor.GetLong(0) != 0;

        int carriedCount = Math.Max(node.Inputs.Length - 2, 0);
        var carried = new Tensor[carriedCount];
        for (int i = 0; i < carriedCount; i++) carried[i] = Required(node, env, i + 2);

        // Body outputs past the condition and the carried values are scan outputs, stacked at the
        // end rather than carried between iterations.
        int scanCount = Math.Max(body.Outputs.Length - 1 - carriedCount, 0);
        var scans = new List<Tensor>[scanCount];
        for (int i = 0; i < scanCount; i++) scans[i] = new List<Tensor>();

        for (long iteration = 0; iteration < tripCount && condition; iteration++)
        {
            var inner = new Dictionary<string, Tensor>(StringComparer.Ordinal);
            if (body.Inputs.Length > 0)
                inner[body.Inputs[0].Name] = Tensor.Scalar(iteration);
            if (body.Inputs.Length > 1)
                inner[body.Inputs[1].Name] = Tensor.FromLongs([condition ? 1 : 0], ElementType.Bool);
            for (int i = 0; i < carriedCount && i + 2 < body.Inputs.Length; i++)
                inner[body.Inputs[i + 2].Name] = carried[i];

            var produced = RunSubgraph(body, env, inner);

            condition = produced[body.Outputs[0].Name].GetLong(0) != 0;
            for (int i = 0; i < carriedCount; i++)
                carried[i] = produced[body.Outputs[i + 1].Name];
            for (int i = 0; i < scanCount; i++)
                scans[i].Add(produced[body.Outputs[carriedCount + 1 + i].Name]);
        }

        var result = new Tensor?[carriedCount + scanCount];
        for (int i = 0; i < carriedCount; i++) result[i] = carried[i];
        for (int i = 0; i < scanCount; i++) result[carriedCount + i] = StackScanOutput(scans[i]);
        return result;
    }

    /// <summary>Stack per-iteration scan results along a new leading axis.</summary>
    /// <remarks>
    /// A loop that never ran produces a zero-length leading axis, which is a legal shape and must
    /// not be turned into an error or a scalar.
    /// </remarks>
    private static Tensor StackScanOutput(List<Tensor> slices)
    {
        if (slices.Count == 0) return Tensor.AllocateFloat(0);

        var first = slices[0];
        var shape = new int[first.Shape.Length + 1];
        shape[0] = slices.Count;
        Array.Copy(first.Shape, 0, shape, 1, first.Shape.Length);

        var result = first.IsFloat ? Tensor.AllocateFloat(shape) : Tensor.AllocateLong(first.Type, shape);
        int stride = first.Count;
        for (int i = 0; i < slices.Count; i++)
        {
            if (first.IsFloat) Array.Copy(slices[i].Floats, 0, result.Floats, i * stride, stride);
            else Array.Copy(slices[i].Longs, 0, result.Longs, i * stride, stride);
        }
        return result;
    }

    /// <summary>
    /// Run a subgraph, returning every value it produced.
    /// </summary>
    /// <remarks>
    /// A subgraph reads outer-scope values by name, so the outer environment seeds the inner one;
    /// the explicit bindings then shadow it, which is what makes a body input named the same as
    /// an outer value resolve to the iteration's own copy.
    /// </remarks>
    private Dictionary<string, Tensor> RunSubgraph(
        OnnxSubgraph graph, IReadOnlyDictionary<string, Tensor> outer, Dictionary<string, Tensor> bindings)
    {
        var env = new Dictionary<string, Tensor>(outer, StringComparer.Ordinal);
        foreach (var (name, tensor) in graph.Initializers) env[name] = tensor;
        foreach (var (name, tensor) in bindings) env[name] = tensor;

        foreach (var inner in graph.Nodes)
        {
            // A subgraph's values are bound by name into a shared scope and never unbound, so
            // nothing here is known to be dead and nothing may be overwritten.
            var outputs = Execute(inner, env, null);
            for (int o = 0; o < inner.Outputs.Length && o < outputs.Length; o++)
            {
                if (inner.Outputs[o].Length == 0 || outputs[o] is null) continue;
                env[inner.Outputs[o]] = outputs[o]!;
            }
        }

        foreach (var output in graph.Outputs)
            if (!env.ContainsKey(output.Name))
                throw new InvalidOperationException($"onnx: subgraph output '{output.Name}' was never produced");
        return env;
    }

    private static Tensor? Optional(OnnxNode node, IReadOnlyDictionary<string, Tensor> env, int index)
    {
        if (index >= node.Inputs.Length) return null;
        string name = node.Inputs[index];
        if (name.Length == 0) return null;
        return env.TryGetValue(name, out var tensor)
            ? tensor
            : throw new InvalidOperationException($"onnx: value '{name}' is not available");
    }

    private static Tensor Required(OnnxNode node, IReadOnlyDictionary<string, Tensor> env, int index) =>
        Optional(node, env, index) ?? throw new InvalidOperationException(
            $"onnx: {node.OpType} requires input #{index}");

    /// <summary>
    /// Read an operand that may be given either as an input (newer opsets) or as an
    /// attribute (older ones). Several ops moved axes and sizes across that boundary, and the
    /// pinned models straddle the change.
    /// </summary>
    private static long[] IntsFromInputOrAttribute(
        OnnxNode node, IReadOnlyDictionary<string, Tensor> env, int inputIndex, string attributeName)
    {
        if (Optional(node, env, inputIndex) is { } tensor)
        {
            var values = new long[tensor.Count];
            for (int i = 0; i < tensor.Count; i++) values[i] = tensor.GetLong(i);
            return values;
        }
        return node.AttrInts(attributeName) ?? [];
    }

    /// <summary>
    /// Run a single node against an environment the caller supplies.
    /// <para>
    /// This is how a kernel is tested in isolation: feed the node the reference's own
    /// recorded inputs and compare only its output. Whole-graph comparison cannot separate a
    /// wrong kernel from a correct one amplifying drift that arrived from upstream; this can.
    /// </para>
    /// </summary>
    public Tensor?[] ExecuteNode(OnnxNode node, Dictionary<string, Tensor> env) => Execute(node, env, null);

    private Tensor?[] Execute(OnnxNode node, Dictionary<string, Tensor> env, Tensor? reusable)
    {
        switch (node.OpType)
        {
            case "Constant":
                return [ConstantValue(node)];

            case "Identity":
                return [Required(node, env, 0)];

            case "Add": return [Elementwise.Binary(Required(node, env, 0), Required(node, env, 1), BinaryKind.Add, reusable)];
            case "Sub": return [Elementwise.Binary(Required(node, env, 0), Required(node, env, 1), BinaryKind.Sub, reusable)];
            case "Mul": return [Elementwise.Binary(Required(node, env, 0), Required(node, env, 1), BinaryKind.Mul, reusable)];
            case "Div": return [Elementwise.Binary(Required(node, env, 0), Required(node, env, 1), BinaryKind.Div, reusable)];
            case "Pow": return [Elementwise.Binary(Required(node, env, 0), Required(node, env, 1), BinaryKind.Pow, reusable)];

            case "Min":
            case "Max":
            {
                var kind = node.OpType == "Min" ? BinaryKind.Min : BinaryKind.Max;
                var accumulator = Required(node, env, 0);
                for (int i = 1; i < node.Inputs.Length; i++)
                    accumulator = Elementwise.Binary(accumulator, Required(node, env, i), kind);
                return [accumulator];
            }

            case "Loop":
                return ExecuteLoop(node, env);

            case "If":
            {
                bool condition = Required(node, env, 0).GetLong(0) != 0;
                var branch = node.Attr(condition ? "then_branch" : "else_branch")?.Graph
                    ?? throw new InvalidDataException($"If: no {(condition ? "then" : "else")} branch");
                var produced = RunSubgraph(branch, env, new Dictionary<string, Tensor>(StringComparer.Ordinal));
                return branch.Outputs.Select(o => (Tensor?)produced[o.Name]).ToArray();
            }

            case "OneHot":
                return [Indexing.OneHot(
                    Required(node, env, 0), Required(node, env, 1), Required(node, env, 2),
                    (int)node.AttrInt("axis", -1))];

            case "ScatterElements":
                return [Indexing.ScatterElements(
                    Required(node, env, 0), Required(node, env, 1), Required(node, env, 2),
                    (int)node.AttrInt("axis", 0))];

            case "Not": return [Elementwise.Not(Required(node, env, 0))];

            case "And":
            case "Or":
            case "Xor":
            {
                var kind = node.OpType switch
                {
                    "And" => LogicalKind.And,
                    "Or" => LogicalKind.Or,
                    _ => LogicalKind.Xor,
                };
                return [Elementwise.Logical(Required(node, env, 0), Required(node, env, 1), kind)];
            }

            case "Floor": return [Elementwise.Floor(Required(node, env, 0))];
            case "Sin": return [Elementwise.Sin(Required(node, env, 0))];
            case "Cos": return [Elementwise.Cos(Required(node, env, 0))];

            case "CumSum":
                return [Reductions.CumSum(
                    Required(node, env, 0), (int)Required(node, env, 1).GetLong(0),
                    node.AttrInt("exclusive", 0) != 0, node.AttrInt("reverse", 0) != 0)];

            case "DynamicQuantizeLinear":
            {
                var (quantized, scale, zeroPoint) = Quantized.DynamicQuantizeLinear(Required(node, env, 0));
                return [quantized, scale, zeroPoint];
            }

            case "MatMulInteger":
                return [Quantized.MatMulInteger(
                    Required(node, env, 0), Required(node, env, 1),
                    Optional(node, env, 2), Optional(node, env, 3))];

            case "ConvInteger":
                return [Quantized.ConvInteger(
                    Required(node, env, 0), Required(node, env, 1),
                    Optional(node, env, 2), Optional(node, env, 3),
                    node.AttrInts("strides"), node.AttrInts("pads"), node.AttrInts("dilations"),
                    node.AttrInt("group", 1), node.AttrString("auto_pad", "NOTSET"))];

            case "Greater":
            case "GreaterOrEqual":
            case "Less":
            case "LessOrEqual":
            case "Equal":
            {
                var kind = node.OpType switch
                {
                    "Greater" => CompareKind.Greater,
                    "GreaterOrEqual" => CompareKind.GreaterOrEqual,
                    "Less" => CompareKind.Less,
                    "LessOrEqual" => CompareKind.LessOrEqual,
                    _ => CompareKind.Equal,
                };
                return [Elementwise.Compare(Required(node, env, 0), Required(node, env, 1), kind)];
            }

            case "Mod":
                return [Elementwise.Mod(
                    Required(node, env, 0), Required(node, env, 1), node.AttrInt("fmod", 0) != 0)];

            case "GatherND":
                return [Indexing.GatherND(
                    Required(node, env, 0), Required(node, env, 1), (int)node.AttrInt("batch_dims", 0))];

            case "ScatterND":
                return [Indexing.ScatterND(
                    Required(node, env, 0), Required(node, env, 1), Required(node, env, 2))];

            case "EyeLike":
            {
                var dtype = node.Attr("dtype");
                return [Indexing.EyeLike(
                    Required(node, env, 0), (int)node.AttrInt("k", 0),
                    dtype is null ? null : (ElementType)dtype.Int)];
            }

            case "Einsum":
            {
                var operands = new List<Tensor>(node.Inputs.Length);
                for (int i = 0; i < node.Inputs.Length; i++) operands.Add(Required(node, env, i));
                return [EinsumKernel.Apply(node.AttrString("equation", ""), operands)];
            }

            case "Range":
                return [Shapes.Range(
                    Required(node, env, 0), Required(node, env, 1), Required(node, env, 2))];

            case "Where":
                return [Elementwise.Where(
                    Required(node, env, 0), Required(node, env, 1), Required(node, env, 2))];

            case "Relu": return [Elementwise.Relu(Required(node, env, 0), reusable)];
            case "Sigmoid": return [Elementwise.Sigmoid(Required(node, env, 0), reusable)];
            case "Sqrt": return [Elementwise.Sqrt(Required(node, env, 0))];
            case "Exp": return [Elementwise.Exp(Required(node, env, 0))];
            case "Log": return [Elementwise.Log(Required(node, env, 0))];
            case "Abs": return [Elementwise.Abs(Required(node, env, 0))];
            case "Tanh": return [Elementwise.Tanh(Required(node, env, 0))];
            case "Neg": return [Elementwise.Neg(Required(node, env, 0))];
            case "Erf": return [Elementwise.Erf(Required(node, env, 0))];
            case "HardSwish": return [Elementwise.HardSwish(Required(node, env, 0))];

            case "HardSigmoid":
                return [Elementwise.HardSigmoid(
                    Required(node, env, 0), node.AttrFloat("alpha", 0.2f), node.AttrFloat("beta", 0.5f))];

            case "Clip":
            {
                // Opset 11 moved min and max from attributes to optional inputs; an omitted
                // bound means "unbounded on that side", not zero.
                var value = Required(node, env, 0);
                float min = Optional(node, env, 1) is { Count: > 0 } lo ? lo.GetFloat(0) : node.AttrFloat("min", float.NegativeInfinity);
                float max = Optional(node, env, 2) is { Count: > 0 } hi ? hi.GetFloat(0) : node.AttrFloat("max", float.PositiveInfinity);
                return [Elementwise.Clip(value, min, max)];
            }

            case "Cast":
                return [Elementwise.Cast(Required(node, env, 0), (ElementType)node.AttrInt("to", (long)ElementType.Float))];

            case "Shape":
            {
                var attr = node.Attr("end");
                return [Shapes.Shape(Required(node, env, 0), node.AttrInt("start", 0), attr is null ? null : attr.Int)];
            }

            case "Reshape":
                return [Shapes.Reshape(Required(node, env, 0), Required(node, env, 1), node.AttrInt("allowzero", 0) != 0)];

            case "Unsqueeze":
                return [Shapes.Unsqueeze(Required(node, env, 0), IntsFromInputOrAttribute(node, env, 1, "axes"))];

            case "Squeeze":
                return [Shapes.Squeeze(Required(node, env, 0), IntsFromInputOrAttribute(node, env, 1, "axes"))];

            case "Flatten":
                return [Shapes.Flatten(Required(node, env, 0), node.AttrInt("axis", 1))];

            case "Transpose":
                return [Shapes.Transpose(Required(node, env, 0), node.AttrInts("perm") ?? [])];

            case "Concat":
            {
                var inputs = new List<Tensor>(node.Inputs.Length);
                for (int i = 0; i < node.Inputs.Length; i++) inputs.Add(Required(node, env, i));
                return [Shapes.Concat(inputs, node.AttrInt("axis", 0))];
            }

            case "Slice":
                return [Shapes.Slice(
                    Required(node, env, 0), Required(node, env, 1), Required(node, env, 2),
                    Optional(node, env, 3), Optional(node, env, 4))];

            case "Gather":
                return [Shapes.Gather(Required(node, env, 0), Required(node, env, 1), node.AttrInt("axis", 0))];

            case "GatherElements":
                return [Shapes.GatherElements(Required(node, env, 0), Required(node, env, 1), node.AttrInt("axis", 0))];

            case "Expand":
                return [Shapes.Expand(Required(node, env, 0), Required(node, env, 1))];

            case "Tile":
                return [Shapes.Tile(Required(node, env, 0), Required(node, env, 1))];

            case "ConstantOfShape":
                return [Shapes.ConstantOfShape(Required(node, env, 0), node.Attr("value")?.Tensor)];

            case "Split":
            {
                var sizesTensor = Optional(node, env, 1);
                int[]? sizes = sizesTensor?.ToIntArray() ?? node.AttrInts("split")?.Select(v => (int)v).ToArray();
                int count = (int)node.AttrInt("num_outputs", node.Outputs.Length);
                return Shapes.Split(Required(node, env, 0), node.AttrInt("axis", 0), sizes, count);
            }

            case "ReduceSum":
                return [Reductions.Reduce(
                    Required(node, env, 0), IntsFromInputOrAttribute(node, env, 1, "axes"),
                    node.AttrInt("keepdims", 1) != 0, node.AttrInt("noop_with_empty_axes", 0) != 0, ReduceKind.Sum)];

            case "ReduceMean":
                return [Reductions.Reduce(
                    Required(node, env, 0), IntsFromInputOrAttribute(node, env, 1, "axes"),
                    node.AttrInt("keepdims", 1) != 0, node.AttrInt("noop_with_empty_axes", 0) != 0, ReduceKind.Mean)];

            case "ReduceMax":
                return [Reductions.Reduce(
                    Required(node, env, 0), IntsFromInputOrAttribute(node, env, 1, "axes"),
                    node.AttrInt("keepdims", 1) != 0, node.AttrInt("noop_with_empty_axes", 0) != 0, ReduceKind.Max)];

            case "ReduceMin":
                return [Reductions.Reduce(
                    Required(node, env, 0), IntsFromInputOrAttribute(node, env, 1, "axes"),
                    node.AttrInt("keepdims", 1) != 0, node.AttrInt("noop_with_empty_axes", 0) != 0, ReduceKind.Min)];

            case "ArgMax":
                return [Reductions.ArgMax(
                    Required(node, env, 0), node.AttrInt("axis", 0),
                    node.AttrInt("keepdims", 1) != 0, node.AttrInt("select_last_index", 0) != 0)];

            case "Softmax":
                return [Reductions.Softmax(Required(node, env, 0), node.AttrInt("axis", -1))];

            case "TopK":
            {
                var k = Required(node, env, 1);
                var (values, indices) = Reductions.TopK(
                    Required(node, env, 0), (int)k.GetLong(0), node.AttrInt("axis", -1),
                    node.AttrInt("largest", 1) != 0, node.AttrInt("sorted", 1) != 0);
                return [values, indices];
            }

            case "MatMul":
                return [Linear.MatMul(Required(node, env, 0), Required(node, env, 1))];

            case "Gemm":
                return [Linear.Gemm(
                    Required(node, env, 0), Required(node, env, 1), Optional(node, env, 2),
                    node.AttrFloat("alpha", 1f), node.AttrFloat("beta", 1f),
                    node.AttrInt("transA", 0) != 0, node.AttrInt("transB", 0) != 0)];

            case "Conv":
                return [Convolution.Conv(
                    Required(node, env, 0), Required(node, env, 1), Optional(node, env, 2),
                    node.AttrInts("strides"), node.AttrInts("pads"), node.AttrInts("dilations"),
                    node.AttrInt("group", 1), node.AttrString("auto_pad", "NOTSET"), node.Activation)];

            case "BatchNormalization":
                return [Convolution.BatchNormalization(
                    Required(node, env, 0), Required(node, env, 1), Required(node, env, 2),
                    Required(node, env, 3), Required(node, env, 4), node.AttrFloat("epsilon", 1e-5f))];

            case "LayerNormalization":
                return [Convolution.LayerNormalization(
                    Required(node, env, 0), Required(node, env, 1), Optional(node, env, 2),
                    node.AttrInt("axis", -1), node.AttrFloat("epsilon", 1e-5f))];

            case "GlobalAveragePool":
                return [Pooling.GlobalAveragePool(Required(node, env, 0))];

            case "MaxPool":
                return [Pooling.MaxPool(
                    Required(node, env, 0), node.AttrInts("kernel_shape"), node.AttrInts("strides"),
                    node.AttrInts("pads"), node.AttrInts("dilations"),
                    node.AttrString("auto_pad", "NOTSET"), node.AttrInt("ceil_mode", 0) != 0)];

            case "AveragePool":
                return [Pooling.AveragePool(
                    Required(node, env, 0), node.AttrInts("kernel_shape"), node.AttrInts("strides"),
                    node.AttrInts("pads"), node.AttrString("auto_pad", "NOTSET"),
                    node.AttrInt("ceil_mode", 0) != 0, node.AttrInt("count_include_pad", 0) != 0)];

            case "Resize":
                // Input 1 is `roi`, used only by tf_crop_and_resize, which these models do not use.
                return [Sampling.Resize(
                    Required(node, env, 0), Optional(node, env, 2), Optional(node, env, 3),
                    node.AttrString("mode", "nearest"),
                    node.AttrString("coordinate_transformation_mode", "half_pixel"),
                    node.AttrString("nearest_mode", "round_prefer_floor"))];

            case "GridSample":
                return [Sampling.GridSample(
                    Required(node, env, 0), Required(node, env, 1),
                    node.AttrString("mode", "bilinear"), node.AttrString("padding_mode", "zeros"),
                    node.AttrInt("align_corners", 0) != 0)];

            default:
                throw new NotSupportedException($"onnx: operator '{node.OpType}' is not implemented");
        }
    }

    /// <summary>
    /// A Constant node's value. The tensor form covers everything these graphs emit; the
    /// scalar and list attribute forms are accepted because exporters use them freely.
    /// </summary>
    private static Tensor ConstantValue(OnnxNode node)
    {
        if (node.Attr("value")?.Tensor is { } tensor) return tensor;
        if (node.Attr("value_float") is { } f) return Tensor.Scalar(f.Float);
        if (node.Attr("value_int") is { } i) return Tensor.Scalar(i.Int);
        if (node.Attr("value_floats") is { } fs) return Tensor.FromFloats(fs.Floats, fs.Floats.Length);
        if (node.Attr("value_ints") is { } ints) return Tensor.FromLongs(ints.Ints, ElementType.Int64, ints.Ints.Length);
        throw new InvalidDataException($"onnx: Constant node '{node.Name}' has no recognised value attribute");
    }
}
