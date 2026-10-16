// Ported from crates/xberg/src/extractors/iwork/numbers.rs — the reverse-engineered subset
// of the Numbers schema needed for sheet and table names, string dictionaries, tiles, rows
// and cells, plus the flat-text fallback used when that walk finds nothing.

using System.Globalization;
using System.IO.Compression;
using XRay.Content.Types;

namespace XRay.Content.Internal.IWork;

/// <summary>One table, with the sheet it was drawn on when a sheet context exists.</summary>
internal sealed record NumbersTable(string? SheetName, string Name, List<List<string>> Cells);

internal sealed class NumbersData
{
    public List<NumbersTable> Tables { get; } = new();
    public Metadata Metadata { get; set; } = new();
    public List<ProcessingWarning> Warnings { get; } = new();
}

internal sealed class NumbersFormatException(string message)
    : Exception($"Failed to parse Numbers table data: {message}");

internal static class NumbersParser
{
    // The IWA object type identifiers below are stable identifiers embedded in Numbers'
    // archive metadata, not protobuf field numbers.
    private const uint DocumentArchiveType = 1;
    private const uint SheetArchiveType = 2;
    private const uint TableInfoArchiveType = 6000;
    private const uint TableModelArchiveType = 6001;
    private const uint TileArchiveType = 6002;
    private static readonly uint[] TableDataListTypes = { 6005, 6201 };
    private const uint RichTextPayloadArchiveType = 6218;
    private const uint TextStorageArchiveType = 2001;

    private const byte CellStorageVersion = 5;
    private const byte EmptyCellType = 0;
    private const byte NumberCellType = 2;
    private const byte TextCellType = 3;
    private const byte DateCellType = 5;
    private const byte BooleanCellType = 6;
    private const byte DurationCellType = 7;
    private const byte ErrorCellType = 8;
    private const byte RichTextCellType = 9;
    private const byte CurrencyCellType = 10;

    private const int CellHeaderLength = 12;
    private const uint CellDecimalFlag = 0x1;
    private const uint CellDoubleFlag = 0x2;
    private const uint CellDateFlag = 0x4;
    private const uint CellStringFlag = 0x8;
    private const uint CellRichTextFlag = 0x10;
    private const int DecimalValueLength = 16;
    private const int ScalarValueLength = 8;
    private const int StringKeyLength = 4;
    private const int OffsetEntryLength = 2;
    private const int DefaultTileSize = 256;
    private const int WideOffsetScale = 4;
    private const int Decimal128ExponentBias = 0x1820;
    private const long IworkEpochToUnixSeconds = 978_307_200;
    private const long SecondsPerDay = 86_400;

    private const byte OldV1MaxVersion = 1;
    private const byte OldV3MaxVersion = 3;
    private const int OldV1HeaderLength = 8;
    private const int OldCellTypeV1V3Offset = 2;
    private const int OldCellTypeV4Offset = 1;
    private const int OldFlagsOffset = 4;
    private const uint OldStringFlag = 0x10;
    private const uint OldDoubleFlag = 0x20;
    private const uint OldDateFlag = 0x40;
    private const uint OldRichTextFlag = 0x200;
    private const uint OldCellStyleFlag = 0x2;
    private const uint OldTextStyleFlag = 0x80;
    private const uint OldConditionalStyleFlag = 0x400;
    private const uint OldConditionalRuleFlag = 0x800;
    private const uint OldCurrentFormatFlag = 0x4;
    private const uint OldFormulaFlag = 0x8;
    private const uint OldFormulaErrorFlag = 0x100;
    private const uint OldCommentFlag = 0x1000;
    private const uint OldImportWarningFlag = 0x2000;
    private const uint OldNumberFormatFlag = 0x10000;
    private const uint OldCurrencyFormatFlag = 0x80000;
    private const uint OldDateFormatFlag = 0x20000;
    private const uint OldDurationFormatFlag = 0x40000;
    private const uint OldControlFormatFlag = 0x100000;
    private const uint OldCustomFormatFlag = 0x200000;
    private const uint OldBaseFormatFlag = 0x400000;
    private const uint OldChoiceFormatFlag = 0x800000;

    private static class Field
    {
        public const uint ArchiveIdentifier = 1;
        public const uint ArchiveMessageInfo = 2;
        public const uint ArchiveShouldMerge = 3;
        public const uint MessageType = 1;
        public const uint MessageLength = 3;
        public const uint MessageBaseIndex = 7;
        public const uint DocumentSheet = 1;
        public const uint SheetName = 1;
        public const uint SheetDrawable = 2;
        public const uint TableInfoModel = 2;
        public const uint TableDataStore = 4;
        public const uint TableRows = 6;
        public const uint TableColumns = 7;
        public const uint TableName = 8;
        public const uint DataStoreTiles = 3;
        public const uint DataStoreStrings = 4;
        public const uint DataStoreRichText = 17;
        public const uint DataListEntry = 3;
        public const uint DataListEntryKey = 1;
        public const uint DataListEntryString = 3;
        public const uint DataListEntryRichText = 9;
        public const uint RichTextStorage = 1;
        public const uint TextStorageText = 3;
        public const uint TileStorageTile = 1;
        public const uint TileStorageSize = 2;
        public const uint TileIndex = 1;
        public const uint TileReference = 2;
        public const uint TileRowInfo = 5;
        public const uint RowIndex = 1;
        public const uint RowCellStorage = 6;
        public const uint RowCellOffsets = 7;
        public const uint RowCellStoragePreBnc = 3;
        public const uint RowCellOffsetsPreBnc = 4;
        public const uint RowHasWideOffsets = 8;
    }

    private static class Wire
    {
        public const ulong Varint = 0;
        public const ulong Fixed64 = 1;
        public const ulong LengthDelimited = 2;
        public const ulong Fixed32 = 5;
        public const int FieldNumberShift = 3;
        public const ulong TypeMask = 0x7;
        public const ulong ReferenceTag = 8;
        public const int Fixed64Length = 8;
        public const int Fixed32Length = 4;
    }

    private sealed class IwaObject(uint objectType, ReadOnlyMemory<byte> payload, bool isMergePatch)
    {
        public uint ObjectType { get; } = objectType;
        public ReadOnlyMemory<byte> Payload { get; } = payload;
        public bool IsMergePatch { get; } = isMergePatch;
    }

    private readonly record struct ProtoField(uint Number, bool IsBytes, ulong Varint, ReadOnlyMemory<byte> Bytes);

    private readonly record struct IwaMessageInfo(uint ObjectType, ulong PayloadLength, int? BaseIndex);

    private sealed record SheetTableRefs(string Name, List<ulong> TableIds);

    private sealed class TableFillContext(
        Dictionary<int, string> strings,
        Dictionary<int, string> richStrings,
        string tableName,
        List<ProcessingWarning> warnings)
    {
        public Dictionary<int, string> Strings { get; } = strings;
        public Dictionary<int, string> RichStrings { get; } = richStrings;
        public string TableName { get; } = tableName;
        public List<ProcessingWarning> Warnings { get; } = warnings;
    }

    /// <summary>
    /// Parse a Numbers package: the schema-aware walk first, the flat-text fallback when it
    /// yields no tables or fails for a non-security reason.
    /// </summary>
    public static NumbersData Parse(ZipArchive archive)
    {
        try
        {
            var structured = ParseStructured(archive);
            if (structured.Tables.Count > 0) return structured;
        }
        catch (Exception error) when (error is NumbersFormatException or IwaFormatException)
        {
            // The schema walk found something it has no rule for; the flat scan still finds text.
        }
        return ParseLegacy(archive);
    }

    private static NumbersData ParseStructured(ZipArchive archive)
    {
        var data = new NumbersData();
        var objects = ReadIwaObjects(archive, data.Warnings);
        data.Metadata = IwaContainer.ExtractMetadataFromZip(archive);

        foreach (var sheet in DocumentSheets(objects, data.Warnings))
        {
            Cancel.ThrowIfRequested();
            // Carry the sheet name alongside each table so rendering can surface it as its own
            // heading, matching how the xlsx/ods path headings a sheet separately from its
            // table content rather than folding it into the table's title text.
            string? sheetName = sheet.Name.Length > 0 ? sheet.Name : null;
            foreach (var tableId in sheet.TableIds)
            {
                if (ParseTable(tableId, objects, data.Warnings) is { } table)
                    data.Tables.Add(new NumbersTable(sheetName, table.Name, table.Cells));
            }
        }

        return data;
    }

    private static NumbersData ParseLegacy(ZipArchive archive)
    {
        var data = new NumbersData { Metadata = IwaContainer.ExtractMetadataFromZip(archive) };
        var tableCells = new List<List<string>>();
        var otherCells = new List<List<string>>();
        var tableSeen = new HashSet<string>(StringComparer.Ordinal);
        var otherSeen = new HashSet<string>(StringComparer.Ordinal);

        foreach (var path in IwaContainer.CollectIwaPaths(archive))
        {
            Cancel.ThrowIfRequested();
            byte[] decompressed;
            try
            {
                decompressed = IwaContainer.ReadIwaFile(archive, path);
            }
            catch (Exception error) when (error is IwaFormatException or InvalidDataException)
            {
                IwaContainer.PushMemberParseWarning(data.Warnings, path, error);
                continue;
            }
            var texts = IwaContainer.ExtractTextFromProto(decompressed);
            bool isTable = path.Contains("Table", StringComparison.Ordinal)
                || path.Contains("DataStore", StringComparison.Ordinal);
            AppendLegacyCells(texts, isTable ? tableSeen : otherSeen, isTable ? tableCells : otherCells);
        }

        if (tableCells.Count > 0) data.Tables.Add(new NumbersTable(null, "Sheet Data", tableCells));
        if (otherCells.Count > 0) data.Tables.Add(new NumbersTable(null, "Document Info", otherCells));
        return data;
    }

    private static void AppendLegacyCells(List<string> texts, HashSet<string> seen, List<List<string>> cells)
    {
        foreach (var text in texts)
        {
            // No byte-length floor: a single alphanumeric character can be real content — a
            // numeric answer, a unit label.
            if (!IwaContainer.IsAlphanumeric(text) || !seen.Add(text)) continue;
            cells.Add(new List<string> { text });
        }
    }

    private static Dictionary<ulong, List<IwaObject>> ReadIwaObjects(
        ZipArchive archive, List<ProcessingWarning> warnings)
    {
        var objects = new Dictionary<ulong, List<IwaObject>>();
        foreach (var path in IwaContainer.CollectIwaPaths(archive))
        {
            Cancel.ThrowIfRequested();
            byte[] decompressed;
            try
            {
                decompressed = IwaContainer.ReadIwaFile(archive, path);
            }
            catch (Exception error) when (error is IwaFormatException or InvalidDataException)
            {
                IwaContainer.PushMemberParseWarning(warnings, path, error);
                continue;
            }

            var memberObjects = new Dictionary<ulong, List<IwaObject>>();
            try
            {
                ParseIwaSegments(decompressed, memberObjects);
            }
            catch (Exception error) when (error is NumbersFormatException or IwaFormatException)
            {
                IwaContainer.PushMemberParseWarning(warnings, path, error);
                continue;
            }

            foreach (var (identifier, messages) in memberObjects)
            {
                if (!objects.TryGetValue(identifier, out var list)) objects[identifier] = list = new List<IwaObject>();
                list.AddRange(messages);
            }
        }
        RejectRequiredMergePatches(objects);
        return objects;
    }

    private static void RejectRequiredMergePatches(Dictionary<ulong, List<IwaObject>> objects)
    {
        // Applying field-path protobuf diffs without the full schema could return stale tables.
        if (objects.Values.SelectMany(o => o).Any(o => o.IsMergePatch))
            throw new NumbersFormatException(
                "Numbers archive uses a merge patch that requires schema-aware reconstruction");
    }

    private static void ParseIwaSegments(ReadOnlyMemory<byte> data, Dictionary<ulong, List<IwaObject>> objects)
    {
        int position = 0;
        while (position < data.Length)
        {
            var (identifier, shouldMerge, messageInfos) = ParseIwaSegmentHeader(data, ref position);
            StoreIwaMessages(data, ref position, identifier, shouldMerge, messageInfos, objects);
        }
    }

    private static (ulong Identifier, bool ShouldMerge, List<IwaMessageInfo> MessageInfos) ParseIwaSegmentHeader(
        ReadOnlyMemory<byte> data, ref int position)
    {
        var (headerLength, prefixLength) = ReadVarintAt(data.Span, position);
        position += prefixLength;
        int headerEnd = CheckedEnd(position, headerLength, data.Length, "IWA archive header");
        var headerFields = ParseProtoFields(data[position..headerEnd]);
        position = headerEnd;

        ulong identifier = FieldVarint(headerFields, Field.ArchiveIdentifier)
            ?? throw new NumbersFormatException("IWA ArchiveInfo has no object identifier");
        bool shouldMerge = FieldVarint(headerFields, Field.ArchiveShouldMerge) is { } merge && merge != 0;
        var messageInfos = ParseIwaMessageInfos(headerFields);
        return (identifier, shouldMerge, messageInfos);
    }

    private static List<IwaMessageInfo> ParseIwaMessageInfos(List<ProtoField> fields)
    {
        var infos = new List<IwaMessageInfo>();
        foreach (var message in FieldBytes(fields, Field.ArchiveMessageInfo))
        {
            var messageFields = ParseProtoFields(message);
            ulong? rawType = FieldVarint(messageFields, Field.MessageType);
            if (rawType is not { } typeValue || typeValue > uint.MaxValue)
                throw new NumbersFormatException("IWA MessageInfo has no valid object type");
            ulong payloadLength = FieldVarint(messageFields, Field.MessageLength)
                ?? throw new NumbersFormatException("IWA MessageInfo has no payload length");
            infos.Add(new IwaMessageInfo((uint)typeValue, payloadLength, FieldInt(messageFields, Field.MessageBaseIndex)));
        }
        return infos;
    }

    private static void StoreIwaMessages(
        ReadOnlyMemory<byte> data,
        ref int position,
        ulong identifier,
        bool shouldMerge,
        List<IwaMessageInfo> messageInfos,
        Dictionary<ulong, List<IwaObject>> objects)
    {
        foreach (var message in messageInfos)
        {
            int payloadEnd = CheckedEnd(position, message.PayloadLength, data.Length, "IWA object payload");
            bool isMergePatch = shouldMerge && message.ObjectType == 0;
            uint effectiveType = EffectiveObjectType(message, messageInfos, isMergePatch);
            if (IsRequiredObjectType(effectiveType))
            {
                if (!objects.TryGetValue(identifier, out var list)) objects[identifier] = list = new List<IwaObject>();
                list.Add(new IwaObject(effectiveType, data[position..payloadEnd], isMergePatch));
            }
            position = payloadEnd;
        }
    }

    private static uint EffectiveObjectType(
        IwaMessageInfo message, List<IwaMessageInfo> messageInfos, bool isMergePatch)
    {
        if (!isMergePatch) return message.ObjectType;
        if (message.BaseIndex is { } index && index >= 0 && index < messageInfos.Count)
            return messageInfos[index].ObjectType;
        return 0;
    }

    private static bool IsRequiredObjectType(uint objectType) =>
        objectType is DocumentArchiveType or SheetArchiveType or TableInfoArchiveType or TableModelArchiveType
            or TileArchiveType or RichTextPayloadArchiveType or TextStorageArchiveType
        || TableDataListTypes.Contains(objectType);

    private static IwaObject? ObjectForType(
        Dictionary<ulong, List<IwaObject>> objects, ulong identifier, uint objectType)
    {
        if (!objects.TryGetValue(identifier, out var candidates)) return null;
        for (int i = candidates.Count - 1; i >= 0; i--)
        {
            if (candidates[i].ObjectType == objectType && !candidates[i].IsMergePatch) return candidates[i];
        }
        return null;
    }

    /// <summary>
    /// Walk every sheet referenced from the document archive, resolving each sheet's display
    /// name and the tables drawn on it. A drawable that resolves to some other archive type is
    /// named in a warning rather than vanishing silently, since xberg has no schema for
    /// reconstructing arbitrary iWork drawables.
    /// </summary>
    private static List<SheetTableRefs> DocumentSheets(
        Dictionary<ulong, List<IwaObject>> objects, List<ProcessingWarning> warnings)
    {
        IwaObject? document = null;
        foreach (var messages in objects.Values)
        {
            for (int i = messages.Count - 1; i >= 0 && document is null; i--)
            {
                if (messages[i].ObjectType == DocumentArchiveType && !messages[i].IsMergePatch) document = messages[i];
            }
            if (document is not null) break;
        }
        if (document is null) throw new NumbersFormatException("Numbers document archive is missing");

        var documentFields = ParseProtoFields(document.Payload);
        var sheets = new List<SheetTableRefs>();
        bool hasTable = false;

        foreach (var sheetReference in FieldBytes(documentFields, Field.DocumentSheet))
        {
            if (ReferenceIdentifier(sheetReference.Span) is not { } sheetId) continue;
            if (ObjectForType(objects, sheetId, SheetArchiveType) is not { } sheet) continue;

            var sheetFields = ParseProtoFields(sheet.Payload);
            string sheetName = ParseSheetName(sheetFields);
            var (tableIds, skippedTypes) = ResolveSheetDrawables(sheetFields, objects);
            hasTable |= tableIds.Count > 0;
            PushNonTableDrawableWarning(warnings, sheetName, skippedTypes);
            sheets.Add(new SheetTableRefs(sheetName, tableIds));
        }

        if (!hasTable) throw new NumbersFormatException("Numbers document has no readable table references");
        return sheets;
    }

    private static (List<ulong> TableIds, List<uint> SkippedTypes) ResolveSheetDrawables(
        List<ProtoField> sheetFields, Dictionary<ulong, List<IwaObject>> objects)
    {
        var tableIds = new List<ulong>();
        var skippedTypes = new List<uint>();
        foreach (var drawable in FieldBytes(sheetFields, Field.SheetDrawable))
        {
            if (ReferenceIdentifier(drawable.Span) is not { } drawableId) continue;
            if (ObjectForType(objects, drawableId, TableInfoArchiveType) is { } tableInfo)
            {
                var fields = ParseProtoFields(tableInfo.Payload);
                if (FirstBytes(fields, Field.TableInfoModel) is { } model
                    && ReferenceIdentifier(model.Span) is { } tableId) tableIds.Add(tableId);
                continue;
            }
            if (objects.TryGetValue(drawableId, out var candidates))
                skippedTypes.AddRange(candidates.Where(o => !o.IsMergePatch).Select(o => o.ObjectType));
        }
        return (tableIds, skippedTypes);
    }

    /// <summary>
    /// Record that a sheet placed drawables xberg cannot reconstruct. Naming the archive type
    /// keeps the warning actionable without inventing shape/image/text-box content that was
    /// never parsed.
    /// </summary>
    public static void PushNonTableDrawableWarning(
        List<ProcessingWarning> warnings, string sheetName, IReadOnlyList<uint> archiveTypes)
    {
        if (archiveTypes.Count == 0) return;
        string types = string.Join(", ", archiveTypes);
        IwaContainer.PushWarning(
            warnings,
            $"Sheet '{sheetName}' contains {archiveTypes.Count} non-table drawable object(s) "
            + $"(archive type(s): {types}) that xberg does not extract; only tables are supported");
    }

    private static string ParseSheetName(List<ProtoField> fields) =>
        FirstUtf8(fields, Field.SheetName) ?? "Sheet";

    private static string ParseTableName(List<ProtoField> fields) =>
        FirstUtf8(fields, Field.TableName) ?? "Table";

    private static string? FirstUtf8(List<ProtoField> fields, uint number) =>
        FirstBytes(fields, number) is { } value && IwaContainer.TryDecodeUtf8(value.Span, out string text)
            ? text
            : null;

    private static (string Name, List<List<string>> Cells)? ParseTable(
        ulong tableId, Dictionary<ulong, List<IwaObject>> objects, List<ProcessingWarning> warnings)
    {
        if (ObjectForType(objects, tableId, TableModelArchiveType) is not { } model) return null;
        var fields = ParseProtoFields(model.Payload);
        string name = ParseTableName(fields);
        // A dimension that does not fit an int is treated as absent: the grid below is
        // allocated eagerly, and upstream keeps that honest with a cell budget this port
        // has no equivalent of.
        int rows = FieldInt(fields, Field.TableRows) ?? 0;
        int columns = FieldInt(fields, Field.TableColumns) ?? 0;
        if (rows == 0 || columns == 0) return null;

        if (FirstBytes(fields, Field.TableDataStore) is not { } dataStore) return null;
        var dataStoreFields = ParseProtoFields(dataStore);
        var (strings, richStrings) = ParseTableDictionaries(dataStoreFields, objects);

        var cells = new List<List<string>>(rows);
        for (int row = 0; row < rows; row++) cells.Add(Enumerable.Repeat("", columns).ToList());

        if (FirstBytes(dataStoreFields, Field.DataStoreTiles) is { } tileStorage)
        {
            var context = new TableFillContext(strings, richStrings, name, warnings);
            FillTableTiles(tileStorage, objects, cells, context);
        }
        return (name, cells);
    }

    private static (Dictionary<int, string> Strings, Dictionary<int, string> RichStrings) ParseTableDictionaries(
        List<ProtoField> fields, Dictionary<ulong, List<IwaObject>> objects)
    {
        var strings = new Dictionary<int, string>();
        var richStrings = new Dictionary<int, string>();

        if (FirstBytes(fields, Field.DataStoreStrings) is { } stringsRef
            && ReferenceIdentifier(stringsRef.Span) is { } stringsId
            && ObjectForTableDataList(objects, stringsId) is { } stringsObject)
            strings = ParseStringTable(stringsObject.Payload);

        if (FirstBytes(fields, Field.DataStoreRichText) is { } richRef
            && ReferenceIdentifier(richRef.Span) is { } richId
            && ObjectForTableDataList(objects, richId) is { } richObject)
            richStrings = ParseRichTextTable(richObject.Payload, objects);

        return (strings, richStrings);
    }

    private static IwaObject? ObjectForTableDataList(Dictionary<ulong, List<IwaObject>> objects, ulong identifier)
    {
        foreach (var objectType in TableDataListTypes)
        {
            if (ObjectForType(objects, identifier, objectType) is { } found) return found;
        }
        return null;
    }

    private static Dictionary<int, string> ParseStringTable(ReadOnlyMemory<byte> payload)
    {
        var strings = new Dictionary<int, string>();
        foreach (var entry in FieldBytes(ParseProtoFields(payload), Field.DataListEntry))
        {
            var entryFields = ParseProtoFields(entry);
            if (FieldVarint(entryFields, Field.DataListEntryKey) is not { } rawKey || rawKey > int.MaxValue) continue;
            if (FirstBytes(entryFields, Field.DataListEntryString) is not { } value
                || !IwaContainer.TryDecodeUtf8(value.Span, out string text)) continue;
            strings[(int)rawKey] = text;
        }
        return strings;
    }

    private static Dictionary<int, string> ParseRichTextTable(
        ReadOnlyMemory<byte> payload, Dictionary<ulong, List<IwaObject>> objects)
    {
        var strings = new Dictionary<int, string>();
        foreach (var entry in FieldBytes(ParseProtoFields(payload), Field.DataListEntry))
        {
            var entryFields = ParseProtoFields(entry);
            if (FieldVarint(entryFields, Field.DataListEntryKey) is not { } rawKey || rawKey > int.MaxValue) continue;

            if (FirstBytes(entryFields, Field.DataListEntryRichText) is not { } payloadRef
                || ReferenceIdentifier(payloadRef.Span) is not { } payloadId) continue;
            if (ObjectForType(objects, payloadId, RichTextPayloadArchiveType) is not { } richPayload) continue;

            if (FirstBytes(ParseProtoFields(richPayload.Payload), Field.RichTextStorage) is not { } storageRef
                || ReferenceIdentifier(storageRef.Span) is not { } storageId) continue;
            if (ObjectForType(objects, storageId, TextStorageArchiveType) is not { } storage) continue;

            string value = string.Concat(
                FieldBytes(ParseProtoFields(storage.Payload), Field.TextStorageText)
                    .Select(bytes => IwaContainer.TryDecodeUtf8(bytes.Span, out string text) ? text : null)
                    .Where(text => text is not null));
            if (value.Length == 0) continue;
            strings[(int)rawKey] = value;
        }
        return strings;
    }

    private static void FillTableTiles(
        ReadOnlyMemory<byte> tileStorage,
        Dictionary<ulong, List<IwaObject>> objects,
        List<List<string>> cells,
        TableFillContext context)
    {
        var storageFields = ParseProtoFields(tileStorage);
        int tileSize = FieldInt(storageFields, Field.TileStorageSize) ?? DefaultTileSize;
        foreach (var tileEntry in FieldBytes(storageFields, Field.TileStorageTile))
        {
            var tileFields = ParseProtoFields(tileEntry);
            int tileIndex = FieldInt(tileFields, Field.TileIndex) ?? 0;
            if (FirstBytes(tileFields, Field.TileReference) is not { } tileRef
                || ReferenceIdentifier(tileRef.Span) is not { } tileId) continue;
            if (ObjectForType(objects, tileId, TileArchiveType) is not { } tile) continue;

            long rowOffset = (long)tileIndex * tileSize;
            if (rowOffset > int.MaxValue) throw new NumbersFormatException("Numbers tile row offset overflow");
            FillTile(tile.Payload, (int)rowOffset, cells, context);
        }
    }

    private static void FillTile(
        ReadOnlyMemory<byte> payload, int rowOffset, List<List<string>> cells, TableFillContext context)
    {
        foreach (var rowInfo in FieldBytes(ParseProtoFields(payload), Field.TileRowInfo))
        {
            var rowFields = ParseProtoFields(rowInfo);
            if (FieldInt(rowFields, Field.RowIndex) is not { } index) continue;
            long rowIndex = (long)rowOffset + index;
            if (rowIndex < 0 || rowIndex >= cells.Count) continue;

            var modernStorage = FirstBytes(rowFields, Field.RowCellStorage);
            var modernOffsets = FirstBytes(rowFields, Field.RowCellOffsets);

            ReadOnlyMemory<byte> storage, offsets;
            bool wideOffsets;
            if (modernStorage is { } cellStorage && modernOffsets is { } cellOffsets
                && (!cellStorage.IsEmpty || !cellOffsets.IsEmpty))
            {
                storage = cellStorage;
                offsets = cellOffsets;
                wideOffsets = FieldVarint(rowFields, Field.RowHasWideOffsets) is { } wide && wide != 0;
            }
            else
            {
                // Pre-BNC rows carry the same payloads under different field numbers.
                storage = FirstBytes(rowFields, Field.RowCellStoragePreBnc) ?? default;
                offsets = FirstBytes(rowFields, Field.RowCellOffsetsPreBnc) ?? default;
                wideOffsets = false;
            }

            FillRow(cells[(int)rowIndex], storage.Span, offsets.Span, wideOffsets, context);
        }
    }

    private static void FillRow(
        List<string> row,
        ReadOnlySpan<byte> storage,
        ReadOnlySpan<byte> offsets,
        bool wideOffsets,
        TableFillContext context)
    {
        var parsedOffsets = ParseCellOffsets(offsets, row.Count, wideOffsets);

        for (int column = 0; column < parsedOffsets.Count; column++)
        {
            if (parsedOffsets[column] is not { } start) continue;
            int end = storage.Length;
            for (int next = column + 1; next < parsedOffsets.Count; next++)
            {
                if (parsedOffsets[next] is { } candidate) { end = candidate; break; }
            }
            if (start > end || end > storage.Length)
                throw new NumbersFormatException("Numbers cell storage offset is out of bounds");
            if (ParseCellValue(storage[start..end], context) is { } value) row[column] = value;
        }
    }

    private static List<int?> ParseCellOffsets(ReadOnlySpan<byte> offsets, int columnCount, bool wideOffsets)
    {
        var parsed = new List<int?>();
        int scale = wideOffsets ? WideOffsetScale : 1;
        for (int i = 0; i + OffsetEntryLength <= offsets.Length && parsed.Count < columnCount; i += OffsetEntryLength)
        {
            ushort offset = (ushort)(offsets[i] | (offsets[i + 1] << 8));
            parsed.Add(offset == ushort.MaxValue ? null : offset * scale);
        }
        return parsed;
    }

    private static string? ParseCellValue(ReadOnlySpan<byte> storage, TableFillContext context)
    {
        if (storage.Length == 0) throw new NumbersFormatException("Numbers cell storage is truncated");
        byte version = storage[0];
        if (version == CellStorageVersion) return ParseV5Cell(storage, context);
        if (version <= 4) return ParseOldCell(storage, context);
        return null;
    }

    private static string? ParseV5Cell(ReadOnlySpan<byte> storage, TableFillContext context)
    {
        if (storage.Length < CellHeaderLength)
            throw new NumbersFormatException("Numbers v5 cell storage is truncated");
        byte cellType = storage[1];
        uint flags = (uint)(storage[8] | (storage[9] << 8) | (storage[10] << 16) | (storage[11] << 24));
        int cursor = CellHeaderLength;

        double? decimalValue = TakeFlagged(storage, ref cursor, flags, CellDecimalFlag, DecimalValueLength) is { } d
            ? DecodeDecimal128(storage.Slice(d.Start, d.Length)) : null;
        double? doubleValue = TakeFlagged(storage, ref cursor, flags, CellDoubleFlag, ScalarValueLength) is { } f
            ? DecodeF64(storage.Slice(f.Start, f.Length)) : null;
        double? seconds = TakeFlagged(storage, ref cursor, flags, CellDateFlag, ScalarValueLength) is { } s
            ? DecodeF64(storage.Slice(s.Start, s.Length)) : null;
        int? stringKey = TakeFlagged(storage, ref cursor, flags, CellStringFlag, StringKeyLength) is { } k
            ? DecodeI32(storage.Slice(k.Start, k.Length)) : null;
        int? richKey = TakeFlagged(storage, ref cursor, flags, CellRichTextFlag, StringKeyLength) is { } r
            ? DecodeI32(storage.Slice(r.Start, r.Length)) : null;

        // Source: numbers-parser cell.py::_from_storage defines the v5 layout and type IDs.
        return cellType switch
        {
            EmptyCellType or ErrorCellType => null,
            NumberCellType or CurrencyCellType => (decimalValue ?? doubleValue) is { } number
                ? FormatScalar(number) : null,
            TextCellType => LookUp(context.Strings, stringKey),
            DateCellType => seconds is { } value ? FormatIworkDate(value) : null,
            BooleanCellType => doubleValue is { } flag ? (flag > 0.0 ? "true" : "false") : null,
            DurationCellType => doubleValue is { } duration ? $"{FormatScalar(duration)}s" : null,
            RichTextCellType => LookUp(context.RichStrings, richKey),
            _ => null,
        };
    }

    private static string? LookUp(Dictionary<int, string> table, int? key) =>
        key is { } value && table.TryGetValue(value, out string? text) ? text : null;

    private sealed class OldCellFields
    {
        public int? StringKey;
        public int? RichKey;
        public double? Double;
        public double? Seconds;
        public bool HasFormula;
        public bool HasComment;
    }

    private static string? ParseOldCell(ReadOnlySpan<byte> storage, TableFillContext context)
    {
        byte version = storage[0];
        int headerLength = version <= OldV1MaxVersion ? OldV1HeaderLength : CellHeaderLength;
        if (storage.Length < headerLength)
            throw new NumbersFormatException("Numbers legacy cell storage is truncated");

        int cellTypeOffset = version <= OldV3MaxVersion ? OldCellTypeV1V3Offset : OldCellTypeV4Offset;
        byte cellType = storage[cellTypeOffset];
        uint flags = version <= OldV1MaxVersion
            ? (uint)(storage[OldFlagsOffset] | (storage[OldFlagsOffset + 1] << 8))
            : (uint)(storage[OldFlagsOffset] | (storage[OldFlagsOffset + 1] << 8)
                | (storage[OldFlagsOffset + 2] << 16) | (storage[OldFlagsOffset + 3] << 24));

        var fields = ParseOldCellFields(storage, headerLength, flags);
        // A legacy formula/comment key is a flag bit only: xberg has no schema for the pre-BNC
        // formula token stream or comment thread payload behind that key, so the presence is
        // surfaced via warning instead of guessing at text the parser cannot decode.
        PushLegacyFormulaCommentWarning(context.Warnings, context.TableName, fields.HasFormula, fields.HasComment);

        // Source: https://oss.sheetjs.com/notes/iwa/ documents pre-BNC fields 3/4 and masks.
        return cellType switch
        {
            EmptyCellType or ErrorCellType => null,
            NumberCellType or CurrencyCellType => fields.Double is { } number ? FormatScalar(number) : null,
            TextCellType => LookUp(context.Strings, fields.StringKey),
            DateCellType => fields.Seconds is { } value ? FormatIworkDate(value) : null,
            BooleanCellType => fields.Double is { } flag ? (flag > 0.0 ? "true" : "false") : null,
            DurationCellType => fields.Double is { } duration ? $"{FormatScalar(duration)}s" : null,
            RichTextCellType => LookUp(context.RichStrings, fields.RichKey),
            _ => null,
        };
    }

    /// <summary>
    /// Record that a legacy-format cell carries a formula and/or comment that xberg's
    /// wire-level cell parser does not decode. Identical warnings are collapsed, so a table
    /// with many such cells still surfaces a single line.
    /// </summary>
    public static void PushLegacyFormulaCommentWarning(
        List<ProcessingWarning> warnings, string tableName, bool hasFormula, bool hasComment)
    {
        if (hasFormula)
            IwaContainer.PushWarning(
                warnings,
                $"Table '{tableName}' has a cell with a legacy-format formula; xberg extracts the cell's cached "
                + "value but does not reconstruct the formula source text");
        if (hasComment)
            IwaContainer.PushWarning(
                warnings,
                $"Table '{tableName}' has a cell with a legacy-format comment; xberg does not extract cell comment text");
    }

    private static readonly (uint Flag, int Length)[] OldFieldLayout =
    {
        (OldCellStyleFlag, StringKeyLength),
        (OldTextStyleFlag, StringKeyLength),
        (OldConditionalStyleFlag, StringKeyLength),
        (OldConditionalRuleFlag, StringKeyLength),
        (OldCurrentFormatFlag, StringKeyLength),
        (OldFormulaFlag, StringKeyLength),
        (OldFormulaErrorFlag, StringKeyLength),
        (OldRichTextFlag, StringKeyLength),
        (OldCommentFlag, StringKeyLength),
        (OldImportWarningFlag, StringKeyLength),
        (OldStringFlag, StringKeyLength),
        (OldDoubleFlag, ScalarValueLength),
        (OldDateFlag, ScalarValueLength),
        (OldNumberFormatFlag, StringKeyLength),
        (OldCurrencyFormatFlag, StringKeyLength),
        (OldDateFormatFlag, StringKeyLength),
        (OldDurationFormatFlag, StringKeyLength),
        (OldControlFormatFlag, StringKeyLength),
        (OldCustomFormatFlag, StringKeyLength),
        (OldBaseFormatFlag, StringKeyLength),
        (OldChoiceFormatFlag, StringKeyLength),
    };

    private static OldCellFields ParseOldCellFields(ReadOnlySpan<byte> storage, int cursor, uint flags)
    {
        var fields = new OldCellFields();
        foreach (var (flag, length) in OldFieldLayout)
        {
            if (TakeFlagged(storage, ref cursor, flags, flag, length) is not { } slice) continue;
            var bytes = storage.Slice(slice.Start, slice.Length);
            switch (flag)
            {
                case OldStringFlag: fields.StringKey = DecodeI32(bytes); break;
                case OldRichTextFlag: fields.RichKey = DecodeI32(bytes); break;
                case OldDoubleFlag: fields.Double = DecodeF64(bytes); break;
                case OldDateFlag: fields.Seconds = DecodeF64(bytes); break;
                case OldFormulaFlag: fields.HasFormula = true; break;
                case OldCommentFlag: fields.HasComment = true; break;
            }
        }
        return fields;
    }

    private static (int Start, int Length)? TakeFlagged(
        ReadOnlySpan<byte> storage, ref int cursor, uint flags, uint flag, int length)
    {
        if ((flags & flag) == 0) return null;
        long end = (long)cursor + length;
        if (end > storage.Length) throw new NumbersFormatException("Numbers cell field is truncated");
        var slice = (cursor, length);
        cursor = (int)end;
        return slice;
    }

    private static int DecodeI32(ReadOnlySpan<byte> bytes) =>
        bytes[0] | (bytes[1] << 8) | (bytes[2] << 16) | (bytes[3] << 24);

    private static double DecodeF64(ReadOnlySpan<byte> bytes) => BitConverter.Int64BitsToDouble(
        bytes[0] | ((long)bytes[1] << 8) | ((long)bytes[2] << 16) | ((long)bytes[3] << 24)
        | ((long)bytes[4] << 32) | ((long)bytes[5] << 40) | ((long)bytes[6] << 48) | ((long)bytes[7] << 56));

    private static double DecodeDecimal128(ReadOnlySpan<byte> bytes)
    {
        UInt128 mantissa = UInt128.Zero;
        for (int i = 0; i < 14; i++) mantissa |= (UInt128)bytes[i] << (8 * i);
        mantissa |= (UInt128)(byte)(bytes[14] & 1) << (8 * 14);
        int exponent = ((bytes[15] & 0x7f) << 7) | (bytes[14] >> 1);
        double sign = (bytes[15] & 0x80) == 0 ? 1.0 : -1.0;
        return sign * (double)mantissa * Math.Pow(10, exponent - Decimal128ExponentBias);
    }

    /// <summary>
    /// Rust's <c>f64</c> Display never switches to exponent notation, so neither does this.
    /// </summary>
    public static string FormatScalar(double value)
    {
        if (double.IsFinite(value) && value % 1.0 == 0.0) return value.ToString("F0", CultureInfo.InvariantCulture);
        string text = value.ToString("R", CultureInfo.InvariantCulture);
        return text.Contains('E', StringComparison.Ordinal) ? ExpandExponent(text) : text;
    }

    private static string ExpandExponent(string text)
    {
        int marker = text.IndexOf('E');
        int exponent = int.Parse(text[(marker + 1)..], CultureInfo.InvariantCulture);
        string mantissa = text[..marker];
        string sign = mantissa.StartsWith('-') ? "-" : "";
        if (sign.Length > 0) mantissa = mantissa[1..];

        int point = mantissa.IndexOf('.');
        string digits = point < 0 ? mantissa : mantissa.Remove(point, 1);
        int pointPosition = (point < 0 ? mantissa.Length : point) + exponent;

        if (pointPosition <= 0) return sign + "0." + new string('0', -pointPosition) + digits;
        if (pointPosition >= digits.Length) return sign + digits + new string('0', pointPosition - digits.Length);
        return sign + digits[..pointPosition] + "." + digits[pointPosition..];
    }

    public static string FormatIworkDate(double seconds)
    {
        if (!double.IsFinite(seconds) || seconds < long.MinValue || seconds > long.MaxValue)
            return FormatScalar(seconds);
        long unixSeconds = (long)seconds + IworkEpochToUnixSeconds;
        long days = DivEuclid(unixSeconds, SecondsPerDay);
        long daySeconds = RemEuclid(unixSeconds, SecondsPerDay);
        var (year, month, day) = CivilDateFromUnixDays(days);
        long hour = daySeconds / 3_600;
        long minute = daySeconds % 3_600 / 60;
        long second = daySeconds % 60;
        return string.Create(
            CultureInfo.InvariantCulture,
            $"{year:D4}-{month:D2}-{day:D2}T{hour:D2}:{minute:D2}:{second:D2}Z");
    }

    private static (long Year, long Month, long Day) CivilDateFromUnixDays(long days)
    {
        const long civilEpochOffsetDays = 719_468;
        long adjusted = days + civilEpochOffsetDays;
        long era = DivEuclid(adjusted, 146_097);
        long dayOfEra = adjusted - era * 146_097;
        long yearOfEra = (dayOfEra - dayOfEra / 1_460 + dayOfEra / 36_524 - dayOfEra / 146_096) / 365;
        long year = yearOfEra + era * 400;
        long dayOfYear = dayOfEra - (365 * yearOfEra + yearOfEra / 4 - yearOfEra / 100);
        long monthPrime = (5 * dayOfYear + 2) / 153;
        long day = dayOfYear - (153 * monthPrime + 2) / 5 + 1;
        long month = monthPrime + (monthPrime < 10 ? 3 : -9);
        if (month <= 2) year += 1;
        return (year, month, day);
    }

    private static long DivEuclid(long value, long divisor)
    {
        long quotient = value / divisor;
        return value % divisor < 0 ? (divisor > 0 ? quotient - 1 : quotient + 1) : quotient;
    }

    private static long RemEuclid(long value, long divisor)
    {
        long remainder = value % divisor;
        return remainder < 0 ? remainder + Math.Abs(divisor) : remainder;
    }

    private static List<ProtoField> ParseProtoFields(ReadOnlyMemory<byte> data)
    {
        var fields = new List<ProtoField>();
        var span = data.Span;
        int position = 0;
        while (position < data.Length)
        {
            var (tag, tagLength) = ReadVarintAt(span, position);
            position += tagLength;
            ulong number = tag >> Wire.FieldNumberShift;
            if (number > uint.MaxValue) throw new NumbersFormatException("protobuf field number overflow");
            AppendProtoValue(data, ref position, tag & Wire.TypeMask, (uint)number, fields);
        }
        return fields;
    }

    private static void AppendProtoValue(
        ReadOnlyMemory<byte> data, ref int position, ulong wireType, uint number, List<ProtoField> fields)
    {
        switch (wireType)
        {
            case Wire.Varint:
            {
                var (value, length) = ReadVarintAt(data.Span, position);
                position += length;
                fields.Add(new ProtoField(number, false, value, default));
                break;
            }
            case Wire.Fixed64:
                position = CheckedEnd(position, Wire.Fixed64Length, data.Length, "protobuf fixed64");
                break;
            case Wire.LengthDelimited:
            {
                var (length, prefixLength) = ReadVarintAt(data.Span, position);
                position += prefixLength;
                int end = CheckedEnd(position, length, data.Length, "protobuf length-delimited field");
                fields.Add(new ProtoField(number, true, 0, data[position..end]));
                position = end;
                break;
            }
            case Wire.Fixed32:
                position = CheckedEnd(position, Wire.Fixed32Length, data.Length, "protobuf fixed32");
                break;
            default:
                throw new NumbersFormatException($"unsupported protobuf wire type {wireType}");
        }
    }

    private static (ulong Value, int Length) ReadVarintAt(ReadOnlySpan<byte> data, int position)
    {
        ulong value = 0;
        int shift = 0;
        int cursor = position;
        while (true)
        {
            if (cursor >= data.Length) throw new NumbersFormatException("truncated protobuf varint");
            byte b = data[cursor];
            cursor++;
            if (shift == 63 && b > 1) throw new NumbersFormatException("protobuf varint exceeds 64 bits");
            value |= (ulong)(b & 0x7f) << shift;
            if ((b & 0x80) == 0) return (value, cursor - position);
            shift += 7;
            if (shift >= 64) throw new NumbersFormatException("protobuf varint exceeds 64 bits");
        }
    }

    private static int CheckedEnd(int position, ulong length, int total, string description)
    {
        if (length > int.MaxValue) throw new NumbersFormatException($"{description} length overflow");
        long end = (long)position + (long)length;
        if (end > total) throw new NumbersFormatException($"{description} is truncated");
        return (int)end;
    }

    private static ulong? FieldVarint(List<ProtoField> fields, uint number)
    {
        foreach (var field in fields)
        {
            if (field.Number == number && !field.IsBytes) return field.Varint;
        }
        return null;
    }

    private static int? FieldInt(List<ProtoField> fields, uint number) =>
        FieldVarint(fields, number) is { } value && value <= int.MaxValue ? (int)value : null;

    private static IEnumerable<ReadOnlyMemory<byte>> FieldBytes(List<ProtoField> fields, uint number)
    {
        foreach (var field in fields)
        {
            if (field.Number == number && field.IsBytes) yield return field.Bytes;
        }
    }

    /// <summary>The first length-delimited field with this number, or null when there is none.
    /// A present-but-empty field is not the same as an absent one.</summary>
    private static ReadOnlyMemory<byte>? FirstBytes(List<ProtoField> fields, uint number)
    {
        foreach (var field in fields)
        {
            if (field.Number == number && field.IsBytes) return field.Bytes;
        }
        return null;
    }

    private static ulong? ReferenceIdentifier(ReadOnlySpan<byte> data)
    {
        if (!IwaContainer.TryReadVarint(data, 0, out ulong tag, out int tagLength)) return null;
        if (tag != Wire.ReferenceTag) return null;
        return IwaContainer.TryReadVarint(data, tagLength, out ulong value, out _) ? value : null;
    }
}
