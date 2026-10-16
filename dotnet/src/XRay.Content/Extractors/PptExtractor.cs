using System.Text;
using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Internal.Cfb;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// Native PowerPoint 97-2003 binary (.ppt) extractor. Ports <c>extractors/ppt.rs</c> +
/// <c>extraction/ppt/mod.rs</c>: opens the OLE/CFB container, reads the "PowerPoint Document"
/// stream, and scans record headers for TextCharsAtom (UTF-16LE) / TextBytesAtom (CP1252) text,
/// grouping by the RT_SLIDE and RT_NOTES containers it falls inside (master slides skipped).
/// </summary>
public sealed class PptExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "application/vnd.ms-powerpoint" };
    public int Priority => 60;

    private const int RtTextCharsAtom = 0x0FA0;
    private const int RtTextBytesAtom = 0x0FA8;
    // A single slide's persisted content container (SlideAtom + shapes/text). SlideListWithText
    // (0x0FF0) is a per-document container of SlidePersistAtom entries for the outline view: it does
    // not enclose the slides' text and does not occur once per slide, so it is not a slide boundary.
    private const int RtSlide = 0x03EE;
    private const int RtMainMaster = 0x03F8;
    private const int RtNotes = 0x03F0;

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        var comp = CompoundFile.Open(content);

        var meta = new OleUtil.OleMetadata();
        if (comp.TryReadStream("/\x05SummaryInformation") is { } si) OleUtil.ParseSummaryInfo(si, meta);

        byte[] ppt = comp.TryReadStream("/PowerPoint Document")
            ?? throw new InvalidDataException("Failed to open stream 'PowerPoint Document'");
        if (ppt.Length == 0) throw new InvalidDataException("PowerPoint Document stream is empty");

        var warnings = new List<ProcessingWarning>();
        var (slides, looseTexts, speakerNotes) = ExtractTextsFromRecords(ppt, warnings);

        // A stream with no RT_SLIDE containers at all but with text outside any slide/notes
        // container: surface it as a single synthetic slide rather than dropping it.
        if (slides.Count == 0 && looseTexts.Count > 0)
            slides.Add(new PptSlideText(1, string.Join("\n", looseTexts)));
        int slideCount = slides.Count;

        var doc = BuildInternalDocument(slides, speakerNotes);
        doc.MimeType = mimeType;
        doc.ProcessingWarnings.AddRange(warnings);

        var additional = new Dictionary<string, JsonElement>
        {
            ["slide_count"] = JsonNum(slideCount),
            ["extraction_method"] = JsonStr("native_ole"),
        };
        if (speakerNotes.Count > 0)
            additional["speaker_notes"] = JsonArr(speakerNotes);

        PageStructure? pages = slideCount > 0 ? new PageStructure
        {
            TotalCount = (uint)slideCount,
            UnitType = PageUnitType.Slide,
            Pages = Enumerable.Range(1, slideCount).Select(n => (object)new PageInfoDto((uint)n)).ToList(),
        } : null;

        List<string>? authors = meta.Author is { } a ? new List<string> { a } : null;
        doc.Metadata = new Metadata
        {
            Title = meta.Title,
            Subject = meta.Subject,
            Authors = authors,
            CreatedBy = meta.Author,
            ModifiedBy = meta.LastAuthor,
            Pages = pages,
            Additional = additional,
        };
        return doc;
    }


    /// <summary>One slide's text, numbered by its position in the deck's own persist order.</summary>
    private readonly record struct PptSlideText(uint Number, string Text);

    /// <summary>
    /// Build the document from the deck's real per-slide structure: slide numbers come from
    /// <see cref="ExtractTextsFromRecords"/>'s persist order, never from re-splitting rendered
    /// text (a slide's own text can contain a blank line, which makes that ambiguous).
    /// </summary>
    private static InternalDocument BuildInternalDocument(List<PptSlideText> slides, List<string> speakerNotes)
    {
        var b = new InternalDocumentBuilder("ppt");
        for (int i = 0; i < slides.Count; i++)
        {
            string trimmed = slides[i].Text.Trim();
            var lines = trimmed.Split('\n');
            string firstLine = lines[0];
            string? title = firstLine.Length > 0 && firstLine.Length <= 80 && lines.Length > 1 ? firstLine : null;
            b.PushSlide(slides[i].Number, title, null);

            if (trimmed.Length > 0)
            {
                if (title is not null)
                {
                    for (int li = 1; li < lines.Length; li++)
                    {
                        string lt = lines[li].Trim();
                        if (lt.Length > 0) b.PushParagraph(lt, new(), null, null);
                    }
                }
                else
                {
                    b.PushParagraph(trimmed, new(), null, null);
                }
            }

            if (i < speakerNotes.Count && speakerNotes[i].Length > 0)
                b.PushFootnoteDefinition(speakerNotes[i], $"slide-{slides[i].Number}-notes", null);
        }
        return b.Build();
    }

    /// <summary>
    /// Walk the PowerPoint Document record stream, returning one entry per RT_SLIDE container in
    /// persist order (empty slides included, so numbering stays contiguous with the real deck),
    /// the text found outside any slide/notes container, and the speaker notes.
    /// </summary>
    private static (List<PptSlideText> Slides, List<string> Loose, List<string> Notes) ExtractTextsFromRecords(
        byte[] data, List<ProcessingWarning> warnings)
    {
        var slides = new List<PptSlideText>();
        var looseTexts = new List<string>();
        var speakerNotes = new List<string>();
        uint currentSlideNumber = 0;
        int pos = 0;
        bool inSlideText = false;
        int? slideEnd = null;
        var currentSlideTexts = new List<string>();
        bool inNotes = false;
        int? notesEnd = null;
        var currentNotesTexts = new List<string>();

        while (pos + 8 <= data.Length)
        {
            Cancel.ThrowIfRequested();
            // A slide/notes container's text only spans its own declared byte range; close it out
            // as soon as the walk passes that range's end, rather than leaving the container open
            // until the next one starts (which would run the last slide on to end of stream).
            if (slideEnd is { } se && pos >= se)
            {
                // Push even when empty: a slide with no text atoms still exists and must keep its
                // persist-order number rather than shifting every later slide's number down.
                slides.Add(new PptSlideText(currentSlideNumber, string.Join("\n", currentSlideTexts)));
                currentSlideTexts.Clear();
                inSlideText = false;
                slideEnd = null;
            }
            if (notesEnd is { } ne && pos >= ne)
            {
                FlushNotes(currentNotesTexts, speakerNotes);
                inNotes = false;
                notesEnd = null;
            }

            int recVerInstance = OleUtil.U16(data, pos);
            int recVer = recVerInstance & 0x000F;
            int recType = OleUtil.U16(data, pos + 2);
            long recLen = OleUtil.U32(data, pos + 4);
            if (recLen > data.Length - pos)
            {
                PushWarning(warnings,
                    "Record stream ended with a truncated record; the remaining presentation content was not extracted");
                break;
            }

            bool isContainer = recVer == 0x0F;
            int contentStart = pos + 8;
            int contentEnd = contentStart + (int)recLen;

            if (recType == RtSlide)
            {
                if (inSlideText)
                {
                    slides.Add(new PptSlideText(currentSlideNumber, string.Join("\n", currentSlideTexts)));
                    currentSlideTexts.Clear();
                }
                currentSlideNumber++;
                inSlideText = true;
                slideEnd = contentEnd;
                pos += 8;
                continue;
            }
            if (recType == RtNotes)
            {
                if (inNotes) FlushNotes(currentNotesTexts, speakerNotes);
                inNotes = true;
                notesEnd = contentEnd;
                pos += 8;
                continue;
            }
            if (recType == RtMainMaster)
            {
                pos = contentEnd;
                continue;
            }
            if (recType == RtTextCharsAtom)
            {
                if (contentEnd <= data.Length)
                {
                    var sb = new StringBuilder();
                    for (int k = contentStart; k + 1 < contentEnd; k += 2)
                        sb.Append((char)(ushort)(data[k] | (data[k + 1] << 8)));
                    AddText(CleanPptText(sb.ToString()), inNotes, inSlideText, currentNotesTexts, currentSlideTexts, looseTexts);
                }
                pos = contentEnd;
                continue;
            }
            if (recType == RtTextBytesAtom)
            {
                if (contentEnd <= data.Length)
                {
                    var sb = new StringBuilder(contentEnd - contentStart);
                    for (int k = contentStart; k < contentEnd; k++) sb.Append(OleUtil.Cp1252ToChar(data[k]));
                    AddText(CleanPptText(sb.ToString()), inNotes, inSlideText, currentNotesTexts, currentSlideTexts, looseTexts);
                }
                pos = contentEnd;
                continue;
            }

            pos = isContainer ? pos + 8 : contentEnd;
        }

        // The stream ended while still inside a slide's declared byte range (e.g. a truncated
        // record broke the walk early): still record it, rather than dropping that slide.
        if (inSlideText) slides.Add(new PptSlideText(currentSlideNumber, string.Join("\n", currentSlideTexts)));
        FlushNotes(currentNotesTexts, speakerNotes);

        return (slides, looseTexts, speakerNotes);
    }

    private static void FlushNotes(List<string> currentNotesTexts, List<string> speakerNotes)
    {
        if (currentNotesTexts.Count == 0) return;
        string trimmed = string.Join("\n", currentNotesTexts).Trim();
        if (trimmed.Length > 0) speakerNotes.Add(trimmed);
        currentNotesTexts.Clear();
    }

    private static void PushWarning(List<ProcessingWarning> warnings, string message)
    {
        if (warnings.Any(w => w.Source == "ppt" && w.Message == message)) return;
        warnings.Add(new ProcessingWarning { Source = "ppt", Message = message });
    }

    private static void AddText(string cleaned, bool inNotes, bool inSlideText,
        List<string> currentNotes, List<string> currentSlide, List<string> looseTexts)
    {
        if (cleaned.Length == 0) return;
        if (inNotes) currentNotes.Add(cleaned);
        if (inSlideText) currentSlide.Add(cleaned);
        else if (!inNotes) looseTexts.Add(cleaned);
    }

    private static string CleanPptText(string text)
    {
        var result = new StringBuilder(text.Length);
        foreach (char c in text)
        {
            if (c == '\r' || c == '\x0B') result.Append('\n');
            else if (c < '\x20' && c != '\n' && c != '\t') { }
            else result.Append(c);
        }
        // Trim trailing whitespace from each line.
        string cleaned = string.Join("\n", result.ToString().Split('\n').Select(l => l.TrimEnd()));
        string trimmed = cleaned.Trim();
        if (trimmed.All(c => c == '*' || c == '\n' || char.IsWhiteSpace(c))) return "";
        return cleaned;
    }

    private static JsonElement JsonStr(string s) => JsonDocument.Parse(JsonSerializer.Serialize(s)).RootElement.Clone();
    private static JsonElement JsonNum(int n) => JsonDocument.Parse(n.ToString(System.Globalization.CultureInfo.InvariantCulture)).RootElement.Clone();
    private static JsonElement JsonArr(List<string> items) => JsonDocument.Parse(JsonSerializer.Serialize(items)).RootElement.Clone();
}
