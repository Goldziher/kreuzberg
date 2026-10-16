// Ported from crates/xberg/src/extraction/pst.rs.
//
// Walks a PST store's folder tree and turns every message it finds into the same
// EmailExtractionResult the .eml and .msg paths produce, collecting non-fatal problems as
// processing warnings instead of aborting the extraction.

using System.Globalization;
using System.Text;
using XRay.Content.Core;
using XRay.Content.Extractors;
using XRay.Content.Internal.Email;
using XRay.Content.Types;

namespace XRay.Content.Internal.Pst;

internal static class PstExtraction
{
    private const string WarningSource = "pst_extraction";

    /// <summary>
    /// Safety cap on rows read from a single PST contents/hierarchy table in one pass.
    ///
    /// Folder and table structures are attacker-controllable: a corrupt or hostile table can
    /// claim an effectively unbounded number of rows. The per-folder recursion cap does not help
    /// there — it bounds how deeply folders nest, not how many rows one table yields — so reading
    /// stops here and says so.
    /// </summary>
    private const int MaxTableRows = 100_000;

    /// <summary>Maximum folder nesting walked before a subtree is abandoned.</summary>
    private const int MaxFolderDepth = 50;

    // MAPI property ids read off a message.
    private const ushort PropertySubject = 0x0037;
    private const ushort PropertySenderName = 0x0C1A;
    private const ushort PropertySenderEmail = 0x0C1F;
    private const ushort PropertyBodyPlain = 0x1000;
    private const ushort PropertyBodyRtfCompressed = 0x1009;
    private const ushort PropertyBodyHtml = 0x1013;
    private const ushort PropertyDeliveryTime = 0x0E06;
    private const ushort PropertyRecipientType = 0x0C15;
    private const ushort PropertyDisplayName = 0x3001;
    private const ushort PropertyEmailAddress = 0x3003;
    private const ushort PropertySmtpAddress = 0x39FE;
    private const ushort PropertyAttachData = 0x3701;
    private const ushort PropertyAttachShortFilename = 0x3704;
    private const ushort PropertyAttachLongFilename = 0x3707;

    /// <summary>A folder queued for traversal: the folder, its depth, and its display path.</summary>
    private readonly record struct FolderSeed(PstFolder Folder, int Depth, string Path);

    /// <summary>
    /// Extract every message in a PST image, alongside the warnings raised while doing so.
    /// Throws only when the store itself cannot be opened.
    /// </summary>
    public static (List<EmailExtractionResult> Messages, List<ProcessingWarning> Warnings) ExtractMessages(
        ReadOnlySpan<byte> content)
    {
        PstStore store;
        try
        {
            store = PstStore.Open(content);
        }
        catch (Exception ex) when (ex is InvalidDataException or IOException or ArgumentException)
        {
            throw new InvalidDataException($"Failed to open PST file: {ex.Message}", ex);
        }

        return ExtractFromStore(store);
    }

    private static (List<EmailExtractionResult>, List<ProcessingWarning>) ExtractFromStore(PstStore store)
    {
        var warnings = new List<ProcessingWarning>();

        PstEntryId ipmEntry;
        try
        {
            ipmEntry = store.IpmSubTreeEntryId();
        }
        catch (Exception ex) when (Cancel.NotCancelling)
        {
            warnings.Add(Warn($"Failed to locate IPM (mail) sub-tree in PST store: {ex.Message}"));
            return (new List<EmailExtractionResult>(), warnings);
        }

        PstFolder rootFolder;
        try
        {
            rootFolder = store.OpenFolder(ipmEntry);
        }
        catch (Exception ex) when (Cancel.NotCancelling)
        {
            warnings.Add(Warn($"Failed to open IPM (mail) sub-tree root folder: {ex.Message}"));
            return (new List<EmailExtractionResult>(), warnings);
        }

        string rootName = rootFolder.DisplayName ?? "Top of Personal Folders";
        var seeds = new List<FolderSeed> { new(rootFolder, 0, rootName) };

        seeds.AddRange(DiscoverNonIpmTopLevelFolders(store, ipmEntry.NodeId, warnings));

        var messages = WalkFolderTree(store, seeds, warnings);
        return (messages, warnings);
    }

    /// <summary>
    /// Enumerate the store's true top-level folders and return every one that is not the
    /// already-handled IPM sub-tree, so non-mail top-level folders are walked too.
    ///
    /// The root folder is reached through the ordinary public path — build its entry id, open it
    /// as a folder, read its hierarchy table — rather than through the store's own root-hierarchy
    /// accessor. Every id is either opened as a seed or reported as a warning: traversal never
    /// aborts because one top-level folder failed to open.
    /// </summary>
    private static List<FolderSeed> DiscoverNonIpmTopLevelFolders(
        PstStore store,
        uint ipmNodeId,
        List<ProcessingWarning> warnings)
    {
        var seeds = new List<FolderSeed>();

        PstFolder rootFolder;
        try
        {
            rootFolder = store.OpenFolder(store.MakeEntryId(PstNodeType.RootFolderNid));
        }
        catch (Exception ex) when (Cancel.NotCancelling)
        {
            warnings.Add(Warn($"Failed to open PST root folder while enumerating non-IPM top-level folders: {ex.Message}"));
            return seeds;
        }

        PstTableContext? rootTable;
        try
        {
            rootTable = rootFolder.HierarchyTable();
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            rootTable = null;
        }

        if (rootTable is null)
        {
            warnings.Add(Warn("PST root folder has no hierarchy table; cannot enumerate non-IPM top-level folders"));
            return seeds;
        }

        var (topLevelIds, truncated) = CollectRowIds(rootTable);
        if (truncated)
        {
            warnings.Add(Warn(
                $"PST store root exceeds the maximum top-level folder limit ({MaxTableRows}); remaining top-level folders skipped"));
        }

        foreach (uint nodeId in topLevelIds)
        {
            if (nodeId == ipmNodeId) continue;

            PstFolder folder;
            try
            {
                folder = store.OpenFolder(store.MakeEntryId(nodeId));
            }
            catch (Exception ex) when (Cancel.NotCancelling)
            {
                warnings.Add(Warn($"Failed to open non-IPM top-level folder (node 0x{nodeId:X}): {ex.Message}; folder skipped"));
                continue;
            }

            seeds.Add(new FolderSeed(folder, 0, folder.DisplayName ?? $"(unnamed non-IPM folder, node 0x{nodeId:X})"));
        }

        return seeds;
    }

    /// <summary>
    /// Walk the seeded folders and their subtrees, extracting every message.
    ///
    /// Termination rests on two independent bounds: <see cref="MaxFolderDepth"/> caps how deeply
    /// (or, for a cyclic tree, how many times) folders nest, and <see cref="CollectRowIds"/> caps
    /// how many rows any one table yields — without the second, a table whose row list never ends
    /// stalls inside a single folder, before depth is ever consulted.
    /// </summary>
    private static List<EmailExtractionResult> WalkFolderTree(
        PstStore store,
        List<FolderSeed> seeds,
        List<ProcessingWarning> warnings)
    {
        var messages = new List<EmailExtractionResult>();
        var stack = new List<FolderSeed>(seeds);

        while (stack.Count > 0)
        {
            var (folder, depth, folderPath) = stack[^1];
            stack.RemoveAt(stack.Count - 1);

            if (depth > MaxFolderDepth)
            {
                warnings.Add(Warn($"Folder '{folderPath}' exceeds maximum traversal depth ({MaxFolderDepth}); subtree truncated"));
                continue;
            }

            var contents = TryReadTable(folder.ContentsTable);
            if (contents is not null)
            {
                var (ids, truncated) = CollectRowIds(contents);
                if (truncated)
                {
                    warnings.Add(Warn(
                        $"Folder '{folderPath}' contents table exceeds the maximum row limit ({MaxTableRows}); remaining messages skipped"));
                }

                foreach (uint nodeId in ids)
                {
                    var entryId = store.MakeEntryId(nodeId);
                    PstMessage message;
                    try
                    {
                        message = store.OpenMessage(entryId);
                    }
                    catch (Exception ex) when (Cancel.NotCancelling)
                    {
                        warnings.Add(Warn($"Failed to open message 0x{nodeId:X}: {ex.Message}"));
                        continue;
                    }

                    messages.Add(ExtractMessageContent(message, entryId, folderPath));
                }
            }

            var hierarchy = TryReadTable(folder.HierarchyTable);
            if (hierarchy is null) continue;

            var (subIds, subTruncated) = CollectRowIds(hierarchy);
            if (subTruncated)
            {
                warnings.Add(Warn(
                    $"Folder '{folderPath}' hierarchy table exceeds the maximum row limit ({MaxTableRows}); remaining subfolders skipped"));
            }

            foreach (uint nodeId in subIds)
            {
                PstFolder subFolder;
                try
                {
                    subFolder = store.OpenFolder(store.MakeEntryId(nodeId));
                }
                catch (Exception ex) when (Cancel.NotCancelling)
                {
                    warnings.Add(Warn($"Failed to open folder 0x{nodeId:X}: {ex.Message}"));
                    continue;
                }

                string subName = subFolder.DisplayName ?? $"(unnamed folder, node 0x{nodeId:X})";
                stack.Add(new FolderSeed(subFolder, depth + 1, $"{folderPath}/{subName}"));
            }
        }

        return messages;
    }

    private static PstTableContext? TryReadTable(Func<PstTableContext?> read)
    {
        try
        {
            return read();
        }
        catch (Exception) when (Cancel.NotCancelling)
        {
            return null;
        }
    }

    /// <summary>
    /// Read row ids from a table, stopping at <see cref="MaxTableRows"/>. The flag says whether
    /// reading stopped only because the cap was reached, so the caller can warn about truncation.
    /// </summary>
    private static (List<uint> Ids, bool Truncated) CollectRowIds(PstTableContext table)
    {
        var ids = new List<uint>();
        foreach (var row in table.Rows)
        {
            if (ids.Count >= MaxTableRows) return (ids, true);
            ids.Add(PstTableContext.RowId(row));
        }
        return (ids, false);
    }

    private static EmailExtractionResult ExtractMessageContent(PstMessage message, PstEntryId entryId, string folderPath)
    {
        string? subject = message.GetString(PropertySubject);
        string? fromEmail = message.GetString(PropertySenderEmail) ?? message.GetString(PropertySenderName);

        string? plainText = message.GetString(PropertyBodyPlain);
        string? htmlContent = message.GetString(PropertyBodyHtml);

        // PR_RTF_COMPRESSED is the body of last resort, decompressed and stripped with the same
        // helpers the .msg path uses.
        string? rtfBody = null;
        if (message.GetBinary(PropertyBodyRtfCompressed) is { } compressed)
        {
            byte[]? rtf = MsgExtractor.DecompressRtf(compressed);
            if (rtf is not null)
            {
                string stripped = MsgExtractor.StripRtfToPlainText(rtf);
                if (stripped.Length > 0) rtfBody = stripped;
            }
        }

        string content = ResolveBody(plainText, htmlContent, rtfBody);
        string? date = message.GetTime(PropertyDeliveryTime) is { } filetime ? WindowsFileTimeToString(filetime) : null;

        var toEmails = new List<string>();
        var ccEmails = new List<string>();
        var bccEmails = new List<string>();

        if (message.RecipientTable is { } recipients)
        {
            foreach (var row in recipients.Rows)
            {
                var values = recipients.ReadRow(row);

                long recipientType = values.TryGetValue(PropertyRecipientType, out var type) && type.Type == PstPropertyType.Integer32
                    ? type.Integer
                    : 1;
                string? displayName = values.TryGetValue(PropertyDisplayName, out var name) ? name.AsString() : null;
                string? smtpEmail =
                    (values.TryGetValue(PropertySmtpAddress, out var smtp) ? smtp.AsString() : null)
                    ?? (values.TryGetValue(PropertyEmailAddress, out var email) ? email.AsString() : null);

                string recipient = smtpEmail ?? displayName ?? "";
                if (recipient.Length == 0) continue;

                switch (recipientType)
                {
                    case 1: toEmails.Add(recipient); break;
                    case 2: ccEmails.Add(recipient); break;
                    case 3: bccEmails.Add(recipient); break;
                }
            }
        }

        var attachments = new List<EmailAttachment>();
        if (message.AttachmentTable is { } attachmentTable)
        {
            foreach (var row in attachmentTable.Rows)
            {
                var values = attachmentTable.ReadRow(row);

                string? longFilename = values.TryGetValue(PropertyAttachLongFilename, out var lf) ? lf.AsString() : null;
                string? shortFilename = values.TryGetValue(PropertyAttachShortFilename, out var sf) ? sf.AsString() : null;
                byte[]? data = values.TryGetValue(PropertyAttachData, out var d) && d.Type == PstPropertyType.Binary
                    ? d.Bytes
                    : null;

                string? filename = longFilename ?? shortFilename;
                string? mimeType = filename is null ? null : MimeForFilename(filename);

                attachments.Add(new EmailAttachment
                {
                    Name = filename,
                    Filename = filename,
                    MimeType = mimeType,
                    Size = data?.Length,
                    IsImage = mimeType?.StartsWith("image/", StringComparison.Ordinal) ?? false,
                    Data = data,
                });
            }
        }

        return new EmailExtractionResult
        {
            Subject = subject,
            FromEmail = fromEmail,
            ToEmails = toEmails,
            CcEmails = ccEmails,
            BccEmails = bccEmails,
            Date = date,
            MessageId = null,
            PlainText = plainText,
            HtmlContent = htmlContent,
            Content = content,
            Attachments = attachments,
            Metadata = new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["entry_id"] = EntryIdHex(entryId),
                ["folder_path"] = folderPath,
            },
        };
    }

    /// <summary>
    /// Resolve the message body in the same precedence order the .msg path uses: plain text
    /// first, then cleaned HTML, then the RTF-derived plain text, else nothing.
    /// </summary>
    internal static string ResolveBody(string? plainText, string? htmlContent, string? rtfBody)
    {
        if (!string.IsNullOrEmpty(plainText)) return plainText;
        if (!string.IsNullOrEmpty(htmlContent)) return MimeParser.CleanHtmlContent(htmlContent);
        if (!string.IsNullOrEmpty(rtfBody)) return rtfBody;
        return "";
    }

    /// <summary>The entry id as the flat MAPI hex string: flags, store record key, node id.</summary>
    private static string EntryIdHex(PstEntryId entryId)
    {
        var builder = new StringBuilder(48);
        foreach (byte b in entryId.ToBytes()) builder.Append(b.ToString("X2", CultureInfo.InvariantCulture));
        return builder.ToString();
    }

    /// <summary>A Windows FILETIME as an RFC 3339 timestamp with second precision.</summary>
    internal static string WindowsFileTimeToString(long filetime)
    {
        const long EpochDifference100Ns = 116_444_736_000_000_000L;
        if (filetime < EpochDifference100Ns) return $"(invalid timestamp: {filetime})";

        long unix100Ns = filetime - EpochDifference100Ns;
        long seconds = unix100Ns / 10_000_000;
        try
        {
            return DateTimeOffset.FromUnixTimeSeconds(seconds).UtcDateTime
                .ToString("yyyy-MM-ddTHH:mm:ss'Z'", CultureInfo.InvariantCulture);
        }
        catch (ArgumentOutOfRangeException)
        {
            return $"(invalid timestamp: {filetime})";
        }
    }

    private static readonly Dictionary<string, string> MimeByExtension = BuildMimeByExtension();

    private static Dictionary<string, string> BuildMimeByExtension()
    {
        var map = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        foreach (var (extension, mimeType) in Mime.ListSupportedFormats())
            map[extension] = mimeType;
        return map;
    }

    private static string? MimeForFilename(string filename)
    {
        int dot = filename.LastIndexOf('.');
        if (dot < 0 || dot == filename.Length - 1) return null;
        return MimeByExtension.TryGetValue(filename[(dot + 1)..], out var mimeType) ? mimeType : null;
    }

    private static ProcessingWarning Warn(string message) => new() { Source = WarningSource, Message = message };
}
