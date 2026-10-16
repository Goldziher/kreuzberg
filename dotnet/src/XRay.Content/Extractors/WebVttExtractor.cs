// Ported from crates/xberg/src/extractors/vtt.rs (WebVttExtractor + parse_track).

using System.Text;
using System.Text.Json;
using XRay.Content.Core;
using XRay.Content.Types;

namespace XRay.Content.Extractors;

/// <summary>
/// WebVTT (Web Video Text Tracks) extractor. Emits one paragraph per cue.
/// </summary>
/// <remarks>
/// Only a cue's payload is content. Timings never enter the body text — they go on the cue
/// element's attributes, because the timing of a subtitle labels that cue rather than saying
/// anything about the document, and text metadata has nowhere to put them. The track's overall
/// duration is recorded in the document's metadata instead.
/// <para>
/// Malformed input degrades: a missing signature, an unparsable timing line or an empty cue each
/// produce a warning and the remaining cues are still extracted.
/// </para>
/// </remarks>
public sealed class WebVttExtractor : IExtractor
{
    public IEnumerable<string> SupportedMimeTypes => new[] { "text/vtt" };

    public int Priority => 50;

    /// <summary>The mandatory WebVTT file signature.</summary>
    private const string Signature = "WEBVTT";

    /// <summary>Separator between a cue's start and end timestamps.</summary>
    private const string TimingArrow = "-->";

    private const long MillisPerSecond = 1_000;
    private const long MillisPerMinute = 60 * MillisPerSecond;
    private const long MillisPerHour = 60 * MillisPerMinute;

    /// <summary>One parsed cue.</summary>
    /// <remarks>
    /// A block that carried text but no timing line has null timings. It is not a cue in the
    /// WebVTT sense, but its text is still content, so it is kept untimed rather than discarded.
    /// Zero is deliberately not used as a stand-in: it would fabricate a 00:00:00.000 start and
    /// skew the reported duration.
    /// </remarks>
    private sealed class Cue
    {
        public string? Identifier;
        public long? StartMillis;
        public long? EndMillis;
        public string? Speaker;
        public string Text = "";
    }

    public InternalDocument Extract(ReadOnlySpan<byte> content, string mimeType, ExtractionConfig config)
    {
        // Upstream's one-line growth guard: the input's own size is charged against
        // `max_content_size` before parsing, so a document too large to render is refused
        // rather than rendered and then found to be too large.
        SecurityBudget.FromConfig(config).AccountText(content.Length);
        string source = TextTransform.NormalizeLineEndings(Encoding.UTF8.GetString(content));
        var (title, cues, warnings) = ParseTrack(source);

        var builder = new InternalDocumentBuilder("vtt");
        var body = new StringBuilder();
        foreach (var cue in cues)
        {
            Cancel.ThrowIfRequested();
            string text = cue.Speaker is null ? cue.Text.Trim() : $"{cue.Speaker}: {cue.Text.Trim()}";
            uint index = builder.PushParagraph(text, new List<TextAnnotation>(), null, null);
            builder.SetAttributes(index, CueAttributes(cue));
            if (body.Length > 0) body.Append('\n');
            body.Append(text);
        }

        var metadata = new Metadata { Title = title };
        // Only timed cues are counted: an untimed block is recovered text, not a cue.
        int timedCueCount = cues.Count(c => c.StartMillis is not null);
        metadata.Additional["cue_count"] = JsonSerializer.SerializeToElement(timedCueCount, Json.Options);
        var ends = cues.Where(c => c.EndMillis is not null).Select(c => c.EndMillis!.Value).ToList();
        if (ends.Count > 0)
            metadata.Additional["duration"] =
                JsonSerializer.SerializeToElement(FormatTimestamp(ends.Max()), Json.Options);

        string bodyText = body.ToString();
        metadata.Format = FormatMetadata.Text(new TextMetadata
        {
            LineCount = (uint)CountLines(bodyText),
            WordCount = (uint)bodyText.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).Length,
            // Unicode scalar values, as Rust's `chars()` counts them — not UTF-16 code units.
            CharacterCount = (uint)bodyText.EnumerateRunes().Count(),
            // WebVTT has no headings, hyperlinks or code blocks to report.
        });
        builder.SetMetadata(metadata);

        var doc = builder.Build();
        foreach (var warning in warnings) doc.ProcessingWarnings.Add(warning);
        doc.MimeType = mimeType;
        return doc;
    }

    private static int CountLines(string text)
    {
        if (text.Length == 0) return 0;
        int count = 0;
        foreach (char c in text) if (c == '\n') count++;
        return text[^1] == '\n' ? count : count + 1;
    }

    // ── parsing ──────────────────────────────────────────────────────────────

    private static (string? Title, List<Cue> Cues, List<ProcessingWarning> Warnings) ParseTrack(string source)
    {
        var warnings = new List<ProcessingWarning>();
        string? title = null;
        var lines = source.Split('\n');

        string first = lines.Length > 0 ? lines[0].TrimStart() : "";
        int bodyStart;
        if (first.StartsWith(Signature, StringComparison.Ordinal))
        {
            // The signature line may carry a trailing label, which names the track.
            string trailing = first[Signature.Length..].Trim().TrimStart('-').Trim();
            if (trailing.Length > 0) title = trailing;
            bodyStart = 1;
        }
        else
        {
            warnings.Add(Warn("missing WEBVTT signature line; parsing as WebVTT anyway"));
            bodyStart = 0;
        }

        var cues = new List<Cue>();
        // Whether a block without a timing line is content depends on the rest of the file, which
        // is not known until every block has been read: beside real cues it is noise, but in a
        // track with no cues at all — a plain file mislabelled .vtt, or a track truncated before
        // its timings — it is the whole document. Hold such blocks back and resolve them after
        // the loop, keeping their warnings in source order either way.
        var untimed = new List<(int Slot, string Text)>();

        foreach (var block in SplitBlocks(lines, bodyStart))
        {
            switch (ClassifyBlock(block))
            {
                case BlockKind.Metadata:
                    break;
                case BlockKind.Cue:
                {
                    var (cue, error) = ParseCue(block);
                    if (cue is not null) cues.Add(cue);
                    else warnings.Add(Warn(error!));
                    break;
                }
                default:
                {
                    string text = string.Join("\n", block).Trim();
                    if (text.Length == 0) break;
                    warnings.Add(Warn("block without a timing line skipped"));
                    untimed.Add((warnings.Count - 1, text));
                    break;
                }
            }
        }

        if (cues.Count == 0)
        {
            foreach (var (slot, text) in untimed)
            {
                warnings[slot] = Warn("block without a timing line kept as untimed text");
                cues.Add(new Cue { Text = text });
            }
        }

        return (title, cues, warnings);
    }

    /// <summary>Split a line sequence into blank-line-separated blocks.</summary>
    private static List<List<string>> SplitBlocks(string[] lines, int from)
    {
        var blocks = new List<List<string>>();
        var current = new List<string>();
        for (int i = from; i < lines.Length; i++)
        {
            if (lines[i].Trim().Length == 0)
            {
                if (current.Count > 0) { blocks.Add(current); current = new List<string>(); }
            }
            else current.Add(lines[i]);
        }
        if (current.Count > 0) blocks.Add(current);
        return blocks;
    }

    private enum BlockKind { Metadata, Cue, Unknown }

    private static BlockKind ClassifyBlock(List<string> block)
    {
        if (block.Count == 0) return BlockKind.Unknown;
        string first = block[0].Trim();
        // NOTE, STYLE and REGION describe the track; none of them is ever body content.
        if (first == "NOTE" || first.StartsWith("NOTE ", StringComparison.Ordinal)
            || first == "STYLE"
            || first == "REGION" || first.StartsWith("REGION ", StringComparison.Ordinal))
            return BlockKind.Metadata;
        return block.Any(l => l.Contains(TimingArrow, StringComparison.Ordinal))
            ? BlockKind.Cue : BlockKind.Unknown;
    }

    /// <summary>Parse a cue block: optional identifier, timing line, payload.</summary>
    private static (Cue? Cue, string? Error) ParseCue(List<string> block)
    {
        int timing = block.FindIndex(l => l.Contains(TimingArrow, StringComparison.Ordinal));
        if (timing < 0) return (null, "cue block without a timing line skipped");

        string? identifier = timing == 0 ? null : string.Join(" ", block.Take(timing)).Trim();

        if (ParseTimingLine(block[timing]) is not { } timings)
            return (null, "cue with an unparsable timing line skipped");

        string payload = string.Join("\n", block.Skip(timing + 1));
        var (speaker, text) = StripCueTags(payload);
        if (text.Trim().Length == 0) return (null, "cue with an empty payload skipped");

        return (new Cue
        {
            Identifier = identifier,
            StartMillis = timings.Start,
            EndMillis = timings.End,
            Speaker = speaker,
            Text = text,
        }, null);
    }

    /// <summary>Parse <c>00:00:01.000 --&gt; 00:00:04.000 line:0</c> into start and end millis.</summary>
    private static (long Start, long End)? ParseTimingLine(string line)
    {
        int arrow = line.IndexOf(TimingArrow, StringComparison.Ordinal);
        if (arrow < 0) return null;
        string startText = line[..arrow].Trim();
        // Cue settings may follow the end timestamp on the same line.
        string endText = line[(arrow + TimingArrow.Length)..]
            .Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).FirstOrDefault() ?? "";
        if (ParseTimestamp(startText) is not { } start) return null;
        if (ParseTimestamp(endText) is not { } end) return null;
        return (start, end);
    }

    /// <summary>Parse <c>hh:mm:ss.mmm</c> or <c>mm:ss.mmm</c> into milliseconds.</summary>
    private static long? ParseTimestamp(string value)
    {
        int dot = value.IndexOf('.');
        if (dot < 0) return null;
        string fraction = value[(dot + 1)..];
        if (fraction.Length != 3 || !fraction.All(char.IsAsciiDigit)) return null;
        if (!long.TryParse(fraction, out long millis)) return null;

        var parts = value[..dot].Split(':');
        long hours, minutes;
        string secondsText;
        if (parts.Length == 3)
        {
            if (!long.TryParse(parts[0], out hours) || !long.TryParse(parts[1], out minutes)) return null;
            secondsText = parts[2];
        }
        else if (parts.Length == 2)
        {
            hours = 0;
            if (!long.TryParse(parts[0], out minutes)) return null;
            secondsText = parts[1];
        }
        else return null;

        if (!long.TryParse(secondsText, out long seconds)) return null;
        return hours * MillisPerHour + minutes * MillisPerMinute + seconds * MillisPerSecond + millis;
    }

    private static string FormatTimestamp(long millis)
    {
        long hours = millis / MillisPerHour;
        long minutes = millis % MillisPerHour / MillisPerMinute;
        long seconds = millis % MillisPerMinute / MillisPerSecond;
        long remainder = millis % MillisPerSecond;
        return $"{hours:D2}:{minutes:D2}:{seconds:D2}.{remainder:D3}";
    }

    /// <summary>
    /// Strip WebVTT cue markup, returning the voice span's speaker (if any) and clean text.
    /// </summary>
    /// <remarks>Removes <c>&lt;v Speaker&gt;</c>, <c>&lt;i&gt;</c>, <c>&lt;c.class&gt;</c> and
    /// inline karaoke timestamps, then decodes the escapes WebVTT defines.</remarks>
    private static (string? Speaker, string Text) StripCueTags(string payload)
    {
        string? speaker = null;
        var outBuf = new StringBuilder(payload.Length);
        int pos = 0;
        while (true)
        {
            int open = payload.IndexOf('<', pos);
            if (open < 0) break;
            outBuf.Append(payload, pos, open - pos);
            int close = payload.IndexOf('>', open + 1);
            if (close < 0)
            {
                // An unbalanced '<' is text, not markup; keeping it loses nothing.
                outBuf.Append(payload, open, payload.Length - open);
                return (speaker, DecodeEscapes(outBuf.ToString()));
            }
            string tag = payload[(open + 1)..close];
            speaker ??= ParseVoiceTag(tag);
            pos = close + 1;
        }
        outBuf.Append(payload, pos, payload.Length - pos);
        return (speaker, DecodeEscapes(outBuf.ToString()));
    }

    /// <summary>
    /// The speaker named by a voice-span tag body such as <c>v Roger</c> or <c>v.loud Roger</c>.
    /// Null for every other tag, including closing tags and karaoke timestamps.
    /// </summary>
    private static string? ParseVoiceTag(string tag)
    {
        if (!tag.StartsWith('v')) return null;
        string rest = tag[1..];
        if (rest.Length > 0 && rest[0] != '.' && !char.IsWhiteSpace(rest[0])) return null;
        // A voice span may carry any number of dotted classes before the name.
        while (rest.StartsWith('.'))
        {
            string afterDot = rest[1..];
            int end = afterDot.Length;
            for (int i = 0; i < afterDot.Length; i++)
                if (afterDot[i] == '.' || char.IsWhiteSpace(afterDot[i])) { end = i; break; }
            rest = afterDot[end..];
        }
        string name = rest.Trim();
        return name.Length == 0 ? null : name;
    }

    /// <summary>Decode the character escapes WebVTT defines for cue payloads.</summary>
    private static string DecodeEscapes(string text) => text
        .Replace("&lt;", "<")
        .Replace("&gt;", ">")
        .Replace("&nbsp;", " ")
        .Replace("&lrm;", "‎")
        .Replace("&rlm;", "‏")
        .Replace("&amp;", "&");

    private static ProcessingWarning Warn(string message) => new() { Source = "vtt", Message = message };

    private static Dictionary<string, string> CueAttributes(Cue cue)
    {
        var attributes = new Dictionary<string, string>(StringComparer.Ordinal);
        // An untimed block gets no start or end at all rather than a fabricated 00:00:00.000, so
        // a consumer can tell "not timed" from "starts at zero".
        if (cue.StartMillis is { } start) attributes["start"] = FormatTimestamp(start);
        if (cue.EndMillis is { } end) attributes["end"] = FormatTimestamp(end);
        if (cue.Identifier is not null) attributes["cue_id"] = cue.Identifier;
        if (cue.Speaker is not null) attributes["speaker"] = cue.Speaker;
        return attributes;
    }
}
