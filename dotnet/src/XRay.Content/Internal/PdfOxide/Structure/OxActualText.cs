// Ported from pdf_oxide `structure/traversal.rs`: `build_actualtext_index` (551-557),
// `walk_actualtext` (578-662), `first_page_in_subtree` (664-685), `has_any_mcr` (686-702)
// and the MCR half of `traverse_structure_tree_all_pages` (272-343); plus `document.rs`:
// `ActualTextAction` (874-885), `actualtext_index` (3901-3919),
// `actual_text_is_destructive` (10847-10851), `actualtext_actions_for_page` (10852-10950),
// `cached_mcid_order_for_page` (10963-10995) and `apply_actualtext_to_spans` (10595-10673).
//
// §14.9.4 makes /ActualText on a structure element the replacement for everything its
// subtree draws. The extractor already honours the in-stream `BDC /ActualText` form; this
// is the structure-tree form, which the extractor cannot see because it never reads
// /StructTreeRoot.
using System;
using System.Collections.Generic;
using System.Runtime.CompilerServices;
using System.Text;
using XRay.Content.Internal.Pdf;

namespace XRay.Content.Internal.PdfOxide.Structure;

/// <summary>
/// Every `(scope, mcid)` an /ActualText scope covers, and what each one does about it.
/// </summary>
internal sealed class OxActualTextIndex
{
    /// <summary>`(scope, mcid)` to its innermost replacement — a descendant's /ActualText
    /// overrides its ancestor's for the keys inside the descendant's subtree.</summary>
    public readonly Dictionary<(OxMcidScope Scope, int Mcid), string> McidToActualText = new();

    /// <summary>Every covered key, i.e. every key whose raw glyphs are suppressed.</summary>
    public readonly HashSet<(OxMcidScope Scope, int Mcid)> CoveredMcids = new();

    /// <summary>
    /// Keys that are covered but must not emit: a page-scoped subtree spanning several
    /// pages emits once on its first page and suppresses the rest. Form- and pattern-scoped
    /// subtrees never land here — each covers a single namespace.
    /// </summary>
    public readonly HashSet<(OxMcidScope Scope, int Mcid)> SuppressOnly = new();

    public bool IsEmpty => CoveredMcids.Count == 0;
}

/// <summary>What a page's assembly does with one covered `(scope, mcid)`.</summary>
internal readonly struct OxActualTextAction
{
    /// <summary>The replacement to emit here, or null when this key is suppressed only.</summary>
    public readonly string? Replacement;
    private OxActualTextAction(string? replacement) => Replacement = replacement;

    /// <summary>Emit the replacement on this key and drop the rest of its run.</summary>
    public static OxActualTextAction EmitAndSuppress(string replacement) => new(replacement);

    /// <summary>Drop this key's raw glyphs without emitting anything.</summary>
    public static OxActualTextAction Suppress() => new(null);
}

internal static class OxActualText
{
    private static readonly ConditionalWeakTable<PdfDocument, Cache> Caches = new();

    private sealed class Cache
    {
        public bool TreeLoaded;
        public OxStructTreeRoot? Tree;
        public bool IndexLoaded;
        public OxActualTextIndex? Index;
        public Dictionary<int, List<(OxMcidScope Scope, int Mcid)>>? McidOrder;
    }

    /// <summary>
    /// Apply the structure-tree /ActualText to one page's ordered spans, in place.
    /// </summary>
    /// <param name="mcWins">
    /// MCIDs whose in-stream `BDC /ActualText` the extractor already applied. §14.9.4 gives
    /// the innermost declaration precedence, so an ancestor structure element must not
    /// overwrite them.
    /// </param>
    public static void ApplyToSpans(
        PdfDocument doc, int pageIndex, List<OxTextSpan> spans, IReadOnlySet<int> mcWins)
    {
        var idx = Index(doc);
        if (idx is null || idx.IsEmpty) return;

        var defaultScope = OxMcidScope.Page(pageIndex);

        // "Visible" means at least one span carries the key and survived the artifact, layer
        // and region filters upstream. The glyph text is accumulated alongside for the
        // §14.9.4 conformance gate below.
        var present = new HashSet<(OxMcidScope, int)>();
        var glyphText = new Dictionary<(OxMcidScope, int), StringBuilder>();
        foreach (var s in spans)
        {
            if (s.Mcid is not { } m) continue;
            var key = (s.McidScope ?? defaultScope, m);
            present.Add(key);
            if (!glyphText.TryGetValue(key, out var sb)) glyphText[key] = sb = new StringBuilder();
            sb.Append(s.Text);
        }

        // Walk the structure tree's own MCID order so the consecutive-run dedup below groups
        // the same runs the assemblers see.
        var mcidOrder = McidOrderForPage(doc, pageIndex);
        var actions = ActionsForPage(idx, mcidOrder, present, mcWins, glyphText);
        if (actions.Count == 0) return;

        // The first span of an emitting key takes the replacement; later spans of that key
        // are dropped, so a key with several spans collapses to one. A suppressed key drops
        // all of its spans.
        var emitUsed = new HashSet<(OxMcidScope, int)>();
        var kept = new List<OxTextSpan>(spans.Count);
        foreach (var s in spans)
        {
            if (s.Mcid is not { } m) { kept.Add(s); continue; }
            var key = (s.McidScope ?? defaultScope, m);
            if (!actions.TryGetValue(key, out var action)) { kept.Add(s); continue; }

            // A suppressed key keeps none of its spans; an emitting key keeps its first,
            // rewritten to the replacement.
            if (action.Replacement is { } repl && emitUsed.Add(key))
            {
                s.Text = repl;
                kept.Add(s);
            }
        }
        if (kept.Count == spans.Count) return;
        spans.Clear();
        spans.AddRange(kept);
    }

    /// <summary>
    /// The document's ActualText index, built once. Null for untagged documents and for
    /// tagged ones whose structure tree carries no /ActualText.
    /// </summary>
    /// <remarks>
    /// Decoupled from `/MarkInfo /Suspects`: that flag says the producer's *reading order*
    /// may be unreliable, while /ActualText is content replacement — a producer that supplied
    /// the replacement is asserting what the run is meant to read as either way.
    /// </remarks>
    internal static OxActualTextIndex? Index(PdfDocument doc)
    {
        var cache = Caches.GetValue(doc, static _ => new Cache());
        if (cache.IndexLoaded) return cache.Index;
        cache.IndexLoaded = true;
        var tree = Tree(doc);
        if (tree is null) { cache.Index = null; return null; }
        var built = BuildIndex(tree);
        cache.Index = built.IsEmpty ? null : built;
        return cache.Index;
    }

    private static OxStructTreeRoot? Tree(PdfDocument doc)
    {
        var cache = Caches.GetValue(doc, static _ => new Cache());
        if (cache.TreeLoaded) return cache.Tree;
        cache.TreeLoaded = true;
        try { cache.Tree = OxStructTree.Parse(doc); }
        catch (Exception) when (Cancel.NotCancelling) { cache.Tree = null; }
        return cache.Tree;
    }

    /// <summary>Pre-order `(scope, mcid)` sequence for one page, from the structure tree.</summary>
    internal static List<(OxMcidScope Scope, int Mcid)> McidOrderForPage(PdfDocument doc, int pageIndex)
    {
        var cache = Caches.GetValue(doc, static _ => new Cache());
        if (cache.McidOrder is null)
        {
            var order = new Dictionary<int, List<(OxMcidScope, int)>>();
            if (Tree(doc) is { } tree)
                foreach (var root in tree.RootElements)
                    CollectMcidOrder(root, order);
            cache.McidOrder = order;
        }
        return cache.McidOrder.TryGetValue(pageIndex, out var list)
            ? list
            : new List<(OxMcidScope, int)>();
    }

    private static void CollectMcidOrder(
        OxStructElem elem, Dictionary<int, List<(OxMcidScope, int)>> result)
    {
        foreach (var child in elem.Children)
        {
            switch (child)
            {
                case OxStructChild.Mcr mcr:
                    if (!result.TryGetValue(mcr.Page, out var list))
                        result[mcr.Page] = list = new List<(OxMcidScope, int)>();
                    list.Add((mcr.Scope, mcr.Mcid));
                    break;
                case OxStructChild.Elem e:
                    CollectMcidOrder(e.Value, result);
                    break;
            }
        }
    }

    // ── index construction ──────────────────────────────────────────────────────

    /// <summary>One /ActualText scope, threaded down the walk.</summary>
    private readonly struct ActiveScope
    {
        public readonly string Text;

        /// <summary>
        /// First page, in pre-order, carrying a page-scoped descendant MCID of this scope.
        /// The emit-once rule applies only to those: form- and pattern-scoped descendants
        /// live in their own per-stream namespace (§14.7.4.3) and each emits at its own
        /// anchor. Null when the subtree has no page-scoped descendant.
        /// </summary>
        public readonly int? FirstPage;

        public ActiveScope(string text, int? firstPage) { Text = text; FirstPage = firstPage; }
    }

    internal static OxActualTextIndex BuildIndex(OxStructTreeRoot tree)
    {
        var idx = new OxActualTextIndex();
        foreach (var root in tree.RootElements) Walk(root, null, idx);
        return idx;
    }

    private static void Walk(OxStructElem elem, ActiveScope? inherited, OxActualTextIndex idx)
    {
        ActiveScope? active = null;
        if (elem.ActualText is { Length: > 0 } ownText && HasAnyMcr(elem))
        {
            // A subtree with no marked-content reference of any kind has nothing to attach
            // the scope to, so the scope is dropped rather than inherited into nothing.
            active = new ActiveScope(ownText, FirstPageInSubtree(elem));
        }

        // Inner wins: our own scope, when we have one, overrides the inherited scope for
        // every descendant.
        ActiveScope? scope = active ?? inherited;

        foreach (var child in elem.Children)
        {
            switch (child)
            {
                case OxStructChild.Mcr mcr:
                {
                    if (scope is not { } s) break;
                    var key = (mcr.Scope, mcr.Mcid);
                    idx.CoveredMcids.Add(key);

                    // A page-scoped subtree emits on the first page it reaches and suppresses
                    // the others; a form or pattern namespace emits on every covered key,
                    // because one element covers at most one such stream per MCID.
                    bool shouldEmit = mcr.Scope.ScopeKind != OxMcidScope.Kind.Page
                        || s.FirstPage == mcr.Page;

                    if (shouldEmit) idx.McidToActualText[key] = s.Text;
                    else idx.SuppressOnly.Add(key);
                    break;
                }
                case OxStructChild.Elem e:
                    Walk(e.Value, scope, idx);
                    break;
            }
        }
    }

    private static int? FirstPageInSubtree(OxStructElem elem)
    {
        foreach (var child in elem.Children)
        {
            switch (child)
            {
                case OxStructChild.Mcr mcr when mcr.Scope.ScopeKind == OxMcidScope.Kind.Page:
                    return mcr.Page;
                case OxStructChild.Elem e when FirstPageInSubtree(e.Value) is { } p:
                    return p;
            }
        }
        return null;
    }

    private static bool HasAnyMcr(OxStructElem elem)
    {
        foreach (var child in elem.Children)
        {
            if (child is OxStructChild.Mcr) return true;
            if (child is OxStructChild.Elem e && HasAnyMcr(e.Value)) return true;
        }
        return false;
    }

    // ── per-page actions ────────────────────────────────────────────────────────

    /// <summary>
    /// §14.9.4 conformance test. The spec calls /ActualText "text that is equivalent to what
    /// a person would see", and §14.8.2.4 NOTE 2 leaves it to the reader whether to use it,
    /// so a replacement that would swallow letters or digits while carrying none itself —
    /// a producer tagging whole words with " " or "-" — is declined and the rendered glyphs
    /// kept. A legitimate replacement (the spec's own hyphenation example, a ligature or
    /// soft-hyphen substitution) is alphanumeric and passes.
    /// </summary>
    internal static bool ActualTextIsDestructive(string replacement, string coveredGlyphs)
    {
        bool glyphsAlnum = false;
        foreach (char c in coveredGlyphs) if (char.IsLetterOrDigit(c)) { glyphsAlnum = true; break; }
        if (!glyphsAlnum) return false;
        foreach (char c in replacement) if (char.IsLetterOrDigit(c)) return false;
        return true;
    }

    internal static Dictionary<(OxMcidScope Scope, int Mcid), OxActualTextAction> ActionsForPage(
        OxActualTextIndex? idx,
        IReadOnlyList<(OxMcidScope Scope, int Mcid)> mcidOrder,
        IReadOnlySet<(OxMcidScope, int)> visible,
        IReadOnlySet<int> mcWins,
        IReadOnlyDictionary<(OxMcidScope, int), StringBuilder> glyphText)
    {
        var outActions = new Dictionary<(OxMcidScope, int), OxActualTextAction>();
        if (idx is null || idx.CoveredMcids.Count == 0) return outActions;

        // Two passes, so a run can span the whole input order: collect this page's covered
        // keys with their replacements, then group consecutive equal replacements into runs.
        // A suppress-only entry carries no replacement, and those collapse into runs too.
        var entries = new List<(OxMcidScope Scope, int Mcid, string? Replacement)>();
        foreach (var (scope, m) in mcidOrder)
        {
            var key = (scope, m);
            if (!idx.CoveredMcids.Contains(key)) continue;
            if (idx.SuppressOnly.Contains(key)) { entries.Add((scope, m, null)); continue; }
            entries.Add((scope, m, idx.McidToActualText.TryGetValue(key, out var t) ? t : null));
        }

        int i = 0;
        while (i < entries.Count)
        {
            string? repl = entries[i].Replacement;
            int j = i;
            while (j < entries.Count && string.Equals(entries[j].Replacement, repl, StringComparison.Ordinal)) j++;

            if (repl is not null)
            {
                var runGlyphs = new StringBuilder();
                for (int k = i; k < j; k++)
                    if (glyphText.TryGetValue((entries[k].Scope, entries[k].Mcid), out var sb))
                        runGlyphs.Append(sb);
                if (ActualTextIsDestructive(repl, runGlyphs.ToString())) { i = j; continue; }

                // The run emits at its first entry that is both visible and not already
                // carrying an in-stream replacement.
                (OxMcidScope, int)? emitPick = null;
                for (int k = i; k < j; k++)
                {
                    var key = (entries[k].Scope, entries[k].Mcid);
                    if (visible.Contains(key) && !mcWins.Contains(entries[k].Mcid)) { emitPick = key; break; }
                }

                for (int k = i; k < j; k++)
                {
                    if (mcWins.Contains(entries[k].Mcid)) continue;
                    var key = (entries[k].Scope, entries[k].Mcid);
                    outActions[key] = emitPick is { } pick && pick.Equals(key)
                        ? OxActualTextAction.EmitAndSuppress(repl)
                        : OxActualTextAction.Suppress();
                }
            }
            else
            {
                for (int k = i; k < j; k++)
                {
                    if (mcWins.Contains(entries[k].Mcid)) continue;
                    outActions[(entries[k].Scope, entries[k].Mcid)] = OxActualTextAction.Suppress();
                }
            }

            i = j;
        }

        return outActions;
    }
}
