using System.Text.Json.Serialization;

namespace XRay.Content.Core;

/// <summary>
/// Limits applied to hostile input during extraction, ported from Rust
/// <c>extractors/security.rs</c>.
/// </summary>
/// <remarks>
/// Deliberately conservative: every value here is high enough that no legitimate document in
/// the corpus comes near it, and low enough that a document engineered to exhaust memory or CPU
/// is stopped before it does. The defaults are upstream's, value for value.
/// </remarks>
public sealed class SecurityLimits
{
    /// <summary>Maximum total uncompressed size of an archive (500 MiB).</summary>
    [JsonPropertyName("max_archive_size")]
    public long MaxArchiveSize { get; set; } = 500L * 1024 * 1024;

    /// <summary>Maximum compression ratio before an entry is called a bomb (100:1).</summary>
    [JsonPropertyName("max_compression_ratio")]
    public long MaxCompressionRatio { get; set; } = 100;

    /// <summary>Maximum number of entries in an archive.</summary>
    [JsonPropertyName("max_files_in_archive")]
    public long MaxFilesInArchive { get; set; } = 10_000;

    /// <summary>Maximum nesting depth for non-XML structures (protobuf, JSON).</summary>
    [JsonPropertyName("max_nesting_depth")]
    public long MaxNestingDepth { get; set; } = 1024;

    /// <summary>
    /// Maximum length of any single entity, attribute value or token (1 MiB). A per-token cap,
    /// not a total: a billion-laughs expansion where one entity becomes hundreds of megabytes is
    /// caught here, while a genuinely long paragraph or CDATA block is left to
    /// <see cref="MaxContentSize"/>.
    /// </summary>
    [JsonPropertyName("max_entity_length")]
    public long MaxEntityLength { get; set; } = 1024 * 1024;

    /// <summary>Maximum total text accumulated for one document (100 MiB).</summary>
    [JsonPropertyName("max_content_size")]
    public long MaxContentSize { get; set; } = 100L * 1024 * 1024;

    /// <summary>Maximum parser-loop turns for one document.</summary>
    [JsonPropertyName("max_iterations")]
    public long MaxIterations { get; set; } = 10_000_000;

    /// <summary>Maximum XML element nesting depth.</summary>
    [JsonPropertyName("max_xml_depth")]
    public long MaxXmlDepth { get; set; } = 1024;

    /// <summary>Maximum total table cells across one document.</summary>
    [JsonPropertyName("max_table_cells")]
    public long MaxTableCells { get; set; } = 100_000;

    /// <summary>
    /// Maximum number of pages in a single document. <c>null</c> means unlimited.
    /// </summary>
    /// <remarks>
    /// Checked once the page count is known and before any per-page work starts. Byte-size limits
    /// do not bound page count: a scanned page can compress to a few kilobytes, so a document well
    /// under <see cref="MaxContentSize"/> or <see cref="MaxArchiveSize"/> can still hold thousands
    /// of pages of per-page work. Defaults to unlimited because a real ceiling here is
    /// workload-specific and a low default would silently reject legitimate large documents;
    /// callers that want a ceiling set this explicitly. Upstream GH#1451, GH#764.
    /// </remarks>
    [JsonPropertyName("max_pages")]
    public long? MaxPages { get; set; }
}

/// <summary>Which limit a <see cref="SecurityException"/> reports.</summary>
public enum SecurityViolation
{
    ZipBombDetected,
    ArchiveTooLarge,
    TooManyFiles,
    NestingTooDeep,
    ContentTooLarge,
    EntityTooLong,
    TooManyIterations,
    XmlDepthExceeded,
    TooManyCells,
    TooManyPages,
    UnreadableEntry,
}

/// <summary>
/// A security limit was exceeded. Upstream returns <c>SecurityError</c>, which
/// <c>From&lt;SecurityError&gt; for XbergError</c> turns into <c>XbergError::Security</c>;
/// here the same information travels as an exception, and
/// <see cref="Extractor"/> maps it to the <c>security</c> / 1006 error item upstream emits.
/// </summary>
public sealed class SecurityException : Exception
{
    public SecurityViolation Violation { get; }

    private SecurityException(SecurityViolation violation, string message) : base(message) =>
        Violation = violation;

    // Message text matches Rust's `Display for SecurityError` so a caller comparing strings
    // across the two implementations sees the same thing.
    internal static SecurityException ZipBomb(ulong compressed, ulong uncompressed, double ratio) =>
        new(SecurityViolation.ZipBombDetected,
            $"Potential ZIP bomb detected: compressed {compressed}B -> uncompressed {uncompressed}B "
            + $"(ratio: {ratio.ToString("F1", System.Globalization.CultureInfo.InvariantCulture)}:1)");

    internal static SecurityException ArchiveTooLarge(ulong size, long max) =>
        new(SecurityViolation.ArchiveTooLarge, $"Archive too large: {size} bytes (max: {max} bytes)");

    internal static SecurityException TooManyFiles(long count, long max) =>
        new(SecurityViolation.TooManyFiles, $"Archive has too many files: {count} (max: {max})");

    internal static SecurityException NestingTooDeep(long depth, long max) =>
        new(SecurityViolation.NestingTooDeep, $"Nesting too deep: {depth} levels (max: {max})");

    internal static SecurityException ContentTooLarge(long size, long max) =>
        new(SecurityViolation.ContentTooLarge, $"Content too large: {size} bytes (max: {max} bytes)");

    internal static SecurityException EntityTooLong(long length, long max) =>
        new(SecurityViolation.EntityTooLong, $"Entity too long: {length} chars (max: {max})");

    internal static SecurityException TooManyIterations(long count, long max) =>
        new(SecurityViolation.TooManyIterations, $"Too many iterations: {count} (max: {max})");

    internal static SecurityException XmlDepthExceeded(long depth, long max) =>
        new(SecurityViolation.XmlDepthExceeded, $"XML depth exceeded: {depth} (max: {max})");

    internal static SecurityException TooManyCells(long cells, long max) =>
        new(SecurityViolation.TooManyCells, $"Too many table cells: {cells} (max: {max})");

    internal static SecurityException TooManyPages(long count, long max) =>
        new(SecurityViolation.TooManyPages,
            $"Document has too many pages: {count} (max: {max}). Raise `security_limits.max_pages` "
            + "if this document is legitimate, or split it before extraction.");

    internal static SecurityException UnreadableEntry(int index, string reason) =>
        new(SecurityViolation.UnreadableEntry,
            $"Archive entry {index} could not be read for security accounting: {reason}");
}

/// <summary>
/// The running counters for one document's extraction: nesting depth, loop turns, accumulated
/// text, per-token length, and table cells. One instance threaded into a parser enforces every
/// limit <see cref="SecurityLimits"/> advertises.
/// </summary>
/// <remarks>
/// Every counter accumulates with saturating arithmetic. The numbers being added come from
/// attacker-controlled headers and can each sit near the top of the range, so an unchecked add
/// would wrap a running total back down to something small and let the document through.
/// </remarks>
public sealed class SecurityBudget
{
    private readonly long _maxDepth;
    private readonly long _maxIterations;
    private readonly long _maxEntityLength;
    private readonly long _maxContentSize;
    private readonly long _maxTableCells;

    private long _depth;
    private long _iterations;
    private long _contentSize;
    private long _cells;

    public SecurityBudget(SecurityLimits limits)
    {
        // Both knobs bound the same parse, so the budget honours the tighter of the two. Taking
        // the looser one would silently discard a caller's attempt to clamp nesting via either.
        _maxDepth = Math.Min(limits.MaxXmlDepth, limits.MaxNestingDepth);
        _maxIterations = limits.MaxIterations;
        _maxEntityLength = limits.MaxEntityLength;
        _maxContentSize = limits.MaxContentSize;
        _maxTableCells = limits.MaxTableCells;
    }

    /// <summary>The budget a config asks for, or the defaults when it names none.</summary>
    public static SecurityBudget FromConfig(ExtractionConfig config) =>
        new(config.SecurityLimits ?? new SecurityLimits());

    /// <summary>A budget at the default limits, for call sites with no config to inherit from.</summary>
    public static SecurityBudget Default() => new(new SecurityLimits());

    /// <summary>
    /// A budget for iWork's protobuf containers, which have no XML depth to speak of, so the
    /// format-agnostic nesting limit is the one that applies.
    /// </summary>
    public static SecurityBudget ForIWork(SecurityLimits limits)
    {
        var l = new SecurityLimits
        {
            MaxArchiveSize = limits.MaxArchiveSize,
            MaxCompressionRatio = limits.MaxCompressionRatio,
            MaxFilesInArchive = limits.MaxFilesInArchive,
            MaxNestingDepth = limits.MaxNestingDepth,
            MaxEntityLength = limits.MaxEntityLength,
            MaxContentSize = limits.MaxContentSize,
            MaxIterations = limits.MaxIterations,
            MaxXmlDepth = limits.MaxNestingDepth,
            MaxTableCells = limits.MaxTableCells,
            MaxPages = limits.MaxPages,
        };
        return new SecurityBudget(l);
    }

    /// <summary>One parser-loop turn. Call before reading each event.</summary>
    public void Step()
    {
        _iterations = SaturatingAdd(_iterations, 1);
        if (_iterations > _maxIterations)
            throw SecurityException.TooManyIterations(_iterations, _maxIterations);
    }

    /// <summary>Enter one level of nesting, on a start event.</summary>
    public void Enter()
    {
        _depth = SaturatingAdd(_depth, 1);
        if (_depth > _maxDepth)
            throw SecurityException.NestingTooDeep(_depth, _maxDepth);
    }

    /// <summary>Leave one level of nesting. Saturates at zero, so an unbalanced close event in a
    /// malformed document cannot drive the counter negative.</summary>
    public void Leave()
    {
        if (_depth > 0) _depth--;
    }

    /// <summary>Account for <paramref name="length"/> more bytes of emitted text.</summary>
    public void AccountText(long length)
    {
        _contentSize = SaturatingAdd(_contentSize, length);
        if (_contentSize > _maxContentSize)
            throw SecurityException.ContentTooLarge(_contentSize, _maxContentSize);
    }

    /// <summary>Check one entity or token against the per-token cap.</summary>
    public void CheckEntity(string value)
    {
        // Upstream measures `str::len()`, which is UTF-8 bytes; a .NET string's Length is UTF-16
        // code units, and the two disagree on any non-ASCII input.
        long length = System.Text.Encoding.UTF8.GetByteCount(value);
        if (length > _maxEntityLength)
            throw SecurityException.EntityTooLong(length, _maxEntityLength);
    }

    /// <summary>Check one attribute. The name is taken for the call shape upstream uses; the cap
    /// applies to the value, attribute names being short by construction.</summary>
    public void CheckAttr(string name, string value) => CheckEntity(value);

    /// <summary>Account for <paramref name="count"/> more table cells.</summary>
    public void AddCells(long count)
    {
        _cells = SaturatingAdd(_cells, count);
        if (_cells > _maxTableCells)
            throw SecurityException.TooManyCells(_cells, _maxTableCells);
    }

    /// <summary>
    /// <see cref="Step"/> that reports rather than throws.
    /// </summary>
    /// <remarks>
    /// Some formats stop at the limit instead of failing on it — an EPUB that runs out of budget
    /// mid-spine keeps the chapters it already read rather than returning nothing. That is
    /// upstream's choice per call site, not a global one, so both shapes exist here.
    /// </remarks>
    public bool TryStep()
    {
        try { Step(); return true; }
        catch (SecurityException) { return false; }
    }

    /// <summary><see cref="AccountText"/> that reports rather than throws.</summary>
    public bool TryAccountText(long length)
    {
        try { AccountText(length); return true; }
        catch (SecurityException) { return false; }
    }

    /// <summary><see cref="AddCells"/> that reports rather than throws.</summary>
    public bool TryAddCells(long count)
    {
        try { AddCells(count); return true; }
        catch (SecurityException) { return false; }
    }

    private static long SaturatingAdd(long a, long b)
    {
        long sum = unchecked(a + b);
        // Overflow iff the operands share a sign that the result does not.
        if (((a ^ sum) & (b ^ sum)) < 0) return long.MaxValue;
        return sum;
    }
}

/// <summary>
/// Document-level checks that do not belong to any one parser.
/// </summary>
public static class DocumentLimits
{
    /// <summary>
    /// Reject a document whose page (or slide, or frame) count exceeds
    /// <see cref="SecurityLimits.MaxPages"/>.
    /// </summary>
    /// <remarks>
    /// Rejects rather than truncates, matching every other primary-document limit. Call this once
    /// the count is known and before any per-page work begins; it short-circuits entirely when the
    /// limit is unset, which is the default. Upstream GH#1451.
    /// </remarks>
    public static void EnforcePageCount(long count, SecurityLimits? limits)
    {
        if (limits?.MaxPages is not { } max) return;
        if (count > max) throw SecurityException.TooManyPages(count, max);
    }
}

/// <summary>
/// Archive-level checks, applied to a zip central directory before anything is decompressed.
/// </summary>
public static class ZipBombValidator
{
    /// <summary>
    /// Validate a zip archive's declared sizes.
    /// </summary>
    /// <remarks>
    /// Every entry in the central directory is accounted for, from the header alone — no
    /// decompressor is built — so an entry using an unsupported method or needing a password
    /// still contributes to the totals instead of dropping out of them. An entry whose header
    /// cannot be read is reported rather than skipped: an unaccounted entry means the aggregate
    /// totals no longer bound what extraction will do.
    /// </remarks>
    /// <summary>
    /// Open a zip and validate it before anything is read out of it. The archive is disposed if
    /// validation fails, so a caller's <c>using</c> is not what stands between a rejected bomb and
    /// a leaked handle.
    /// </summary>
    public static System.IO.Compression.ZipArchive OpenValidated(Stream stream, SecurityLimits? limits)
    {
        var archive = new System.IO.Compression.ZipArchive(stream, System.IO.Compression.ZipArchiveMode.Read);
        try
        {
            Validate(archive, limits ?? new SecurityLimits());
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            archive.Dispose();
            throw;
        }
        return archive;
    }

    /// <summary>
    /// Smallest uncompressed member size the per-member ratio cap applies to.
    /// </summary>
    /// <remarks>
    /// The ratio cap guards against one member that inflates to hundreds of megabytes. A member
    /// measured in kilobytes cannot exhaust memory whatever its ratio, and blank-page JPEGs, empty
    /// stylesheets and whitespace-padded pages routinely deflate past 100:1 — one 76 KB blank-page
    /// JPEG rejected whole EPUBs (upstream GH#1496). The total-size cap and the whole-archive ratio
    /// cap still bound the aggregate.
    /// </remarks>
    private const ulong MemberRatioFloor = 1024 * 1024;

    public static void Validate(System.IO.Compression.ZipArchive archive, SecurityLimits limits)
    {
        var entries = archive.Entries;
        if (entries.Count > limits.MaxFilesInArchive)
            throw SecurityException.TooManyFiles(entries.Count, limits.MaxFilesInArchive);

        ulong maxArchiveSize = (ulong)limits.MaxArchiveSize;
        double maxRatio = limits.MaxCompressionRatio;
        ulong totalUncompressed = 0;
        ulong totalCompressed = 0;

        for (int index = 0; index < entries.Count; index++)
        {
            ulong compressed, uncompressed;
            try
            {
                compressed = (ulong)entries[index].CompressedLength;
                uncompressed = (ulong)entries[index].Length;
            }
            catch (Exception e) when (Cancel.NotCancelling)
            {
                throw SecurityException.UnreadableEntry(index, e.Message);
            }

            totalUncompressed = SaturatingAdd(totalUncompressed, uncompressed);
            totalCompressed = SaturatingAdd(totalCompressed, compressed);

            if (uncompressed > 0 && (compressed == 0 || uncompressed >= MemberRatioFloor))
            {
                // A zero compressed size against a non-zero uncompressed one is not something any
                // compressor produces; calling the ratio infinite stops the entry slipping past
                // this check on a division that never happens.
                double ratio = compressed == 0 ? double.PositiveInfinity : (double)uncompressed / compressed;
                if (ratio > maxRatio)
                    throw SecurityException.ZipBomb(compressed, uncompressed, ratio);
            }

            if (totalUncompressed > maxArchiveSize)
                throw SecurityException.ArchiveTooLarge(totalUncompressed, limits.MaxArchiveSize);
        }

        if (totalCompressed > 0)
        {
            double ratio = (double)totalUncompressed / totalCompressed;
            if (ratio > maxRatio)
                throw SecurityException.ZipBomb(totalCompressed, totalUncompressed, ratio);
        }
    }

    private static ulong SaturatingAdd(ulong a, ulong b)
    {
        ulong sum = unchecked(a + b);
        return sum < a ? ulong.MaxValue : sum;
    }
}

/// <summary>Path-safety predicate shared by the archive and container extractors.</summary>
public static class PathSafety
{
    /// <summary>
    /// Whether <paramref name="path"/> contains a parent-directory component.
    /// </summary>
    /// <remarks>
    /// Split into components rather than searched as a string, so a normalised `a/../b` is caught
    /// while a benign `1..2` — a list-numbering prefix, say — is not.
    /// </remarks>
    public static bool HasPathTraversal(string path)
    {
        foreach (var part in path.Split('/', '\\'))
            if (part == "..") return true;
        return false;
    }
}

/// <summary>
/// Input failed a structural precondition. Mirrors upstream's <c>XbergError::Validation</c>,
/// which is what the archive and HWPX extractors turn a security failure into rather than
/// letting it surface as <c>XbergError::Security</c> — the two produce different error items and
/// the difference is upstream's, not this port's.
/// </summary>
public sealed class ValidationException : Exception
{
    public ValidationException(string message) : base(message) { }
}

/// <summary>Charging rules for tabular input, shared by the spreadsheet extractors.</summary>
public static class TableBudget
{
    /// <summary>
    /// Charge a workbook's cells and cell text against <paramref name="budget"/>.
    /// </summary>
    /// <remarks>
    /// A sheet's extent is declared, not counted: a workbook can claim a billion cells in a few
    /// hundred bytes. The charge is rows times the widest row for that reason — the shape the
    /// consumer will have to materialise, not the cells the file bothered to write down.
    /// </remarks>
    public static void ChargeSheets(SecurityBudget budget, IEnumerable<List<List<string>>?> sheets)
    {
        foreach (var cells in sheets)
        {
            if (cells is null) continue;
            long rowCount = cells.Count;
            long colCount = 0;
            foreach (var row in cells) colCount = Math.Max(colCount, row.Count);
            budget.AddCells(SaturatingMul(rowCount, colCount));
            foreach (var row in cells)
                foreach (var cell in row)
                    budget.AccountText(System.Text.Encoding.UTF8.GetByteCount(cell));
        }
    }

    private static long SaturatingMul(long a, long b)
    {
        if (a == 0 || b == 0) return 0;
        long product = unchecked(a * b);
        if (product / b != a) return long.MaxValue;
        return product;
    }
}
