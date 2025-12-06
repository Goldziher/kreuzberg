# BibTeX Extractor: Side-by-Side Comparison with Pandoc

This document shows exactly what Pandoc extracts vs. what our current implementation extracts for the same BibTeX entry.

---

## Test Entry

```bibtex
@article{comprehensive2023,
    author = {Smith, John and Doe, Jane and van der Berg, Hans},
    title = {A Comprehensive Study of BibTeX Parsing},
    journal = {Journal of Bibliography Management},
    year = {2023},
    volume = {42},
    number = {3},
    pages = {123--145},
    doi = {10.1234/jbm.2023.001},
    url = {https://example.com/article},
    keywords = {bibtex, parsing, bibliography}
}
```

---

## Pandoc's Output (CSL-JSON format)

```json
{
  "id": "comprehensive2023",
  "type": "article-journal",
  "author": [
    {
      "family": "Smith",
      "given": "John"
    },
    {
      "family": "Doe",
      "given": "Jane"
    },
    {
      "family": "Berg",
      "given": "Hans",
      "dropping-particle": "van der"
    }
  ],
  "title": "A comprehensive study of BibTeX parsing",
  "container-title": "Journal of Bibliography Management",
  "issued": "2023",
  "volume": "42",
  "issue": "3",
  "page": "123-145",
  "doi": "10.1234/jbm.2023.001",
  "url": "https://example.com/article",
  "keyword": "bibtex, parsing, bibliography"
}
```

### What Pandoc Does

1. **Entry Type Normalization**: `article` → `article-journal` (CSL type)
2. **Author Parsing**: Splits names into structured components:
   - `family`: Last name
   - `given`: First name
   - `dropping-particle`: Name particles like "van der", "von", "de"
3. **Field Mapping**: BibTeX fields → CSL fields:
   - `journal` → `container-title`
   - `number` → `issue` (for articles)
   - `year` → `issued`
   - `pages` → `page` (with range normalization: `--` → `-`)
4. **Title Case Normalization**: Converts to sentence case
5. **Structured Output**: Ready for citation processors

---

## Our Current Output

### Formatted Text Output

```
@article {
  key = comprehensive2023,
  author = Smith, John and Doe, Jane and van der Berg, Hans,
  title = A Comprehensive Study of BibTeX Parsing,
  journal = Journal of Bibliography Management,
  year = 2023,
  volume = 42,
  number = 3,
  pages = 123--145,
  doi = 10.1234/jbm.2023.001,
  url = https://example.com/article,
  keywords = bibtex, parsing, bibliography,
}
```

### Metadata (JSON)

```json
{
  "entry_count": 1,
  "authors": [
    "Smith, John",
    "Doe, Jane",
    "van der Berg, Hans"
  ],
  "year_range": {
    "min": 2023,
    "max": 2023,
    "years": [2023]
  },
  "entry_types": {
    "article": 1
  },
  "citation_keys": [
    "comprehensive2023"
  ]
}
```

### What We Do

1. **Entry Type Recognition**: Correctly identifies `article`
2. **Author Extraction**: Splits on "and", stores as flat strings
3. **Year Extraction**: Parses year, calculates range
4. **Field Preservation**: All fields preserved in formatted output
5. **Basic Metadata**: Counts, keys, types, authors, years

### What We Don't Do

1. **No Structured Authors**: Authors are strings, not structured objects
2. **No Field Mapping**: Keep BibTeX names (`journal` not `container-title`)
3. **No Entry Type Normalization**: Use BibTeX types (`article` not `article-journal`)
4. **No CSL-JSON Output**: Only BibTeX text format
5. **No Name Parsing**: Don't separate family/given/particle
6. **No Title Normalization**: Keep original case

---

## Feature Comparison Table

| Feature | Pandoc | Ours | Gap |
|---------|--------|------|-----|
| **Entry Type Recognition** | ✅ | ✅ | None |
| **Entry Type Normalization** | ✅ article → article-journal | ❌ Keep as 'article' | HIGH |
| **Author Extraction** | ✅ | ✅ | None |
| **Author Name Parsing** | ✅ family/given/particle | ❌ Flat strings | CRITICAL |
| **Field Extraction** | ✅ | ✅ | None |
| **Field Mapping** | ✅ journal → container-title | ❌ Keep raw names | HIGH |
| **Year Extraction** | ✅ | ✅ | None |
| **Date Parsing** | ✅ Full ISO dates | ⚠️ Year only | MEDIUM |
| **Page Range** | ✅ Normalized format | ⚠️ Raw format | LOW |
| **DOI** | ✅ | ✅ | None |
| **URL** | ✅ | ✅ | None |
| **Keywords** | ✅ | ✅ | None |
| **CSL-JSON Output** | ✅ | ❌ | CRITICAL |
| **Citation-Ready** | ✅ | ❌ | CRITICAL |
| **Title Normalization** | ✅ Sentence case | ⚠️ Original case | LOW |

---

## Detailed Feature Gaps

### 1. Author Name Parsing

**Pandoc Output:**
```json
{
  "family": "Berg",
  "given": "Hans",
  "dropping-particle": "van der"
}
```

**Our Output:**
```json
"van der Berg, Hans"
```

**Impact**: Cannot properly format citations, cannot search by last name only, cannot handle name ordering in different citation styles.

**Fix Complexity**: MEDIUM (1-2 days)

### 2. Field Mapping

**Pandoc Mapping:**
- `journal` → `container-title` (for articles)
- `booktitle` → `container-title` (for conference papers)
- `number` → `issue` (for articles)
- `number` → `number` (for reports)
- `address` → `publisher-place`
- `year` → `issued`

**Our Approach:**
- Keep all raw BibTeX field names

**Impact**: Applications need BibTeX-specific logic, can't use standard CSL processors.

**Fix Complexity**: LOW (4-8 hours)

### 3. Entry Type Normalization

**Pandoc Types (CSL):**
- `article` → `article-journal`
- `inproceedings` → `paper-conference`
- `phdthesis` → `thesis` (with genre: "PhD thesis")
- `misc` with URL → `webpage`

**Our Types:**
- Keep raw BibTeX types

**Impact**: Citation processors need BibTeX-specific type handling.

**Fix Complexity**: LOW (2-4 hours)

### 4. CSL-JSON Output

**Pandoc:**
- Provides structured JSON ready for citation processors
- Compatible with citeproc, CSL styles, Zotero, etc.

**Ours:**
- Only provides formatted BibTeX text
- Metadata is in custom format

**Impact**: Cannot integrate with standard citation tools.

**Fix Complexity**: HIGH (2-3 days, includes all above fixes)

---

## Example Use Cases

### Use Case 1: Citation Formatting

**Goal**: Format citation as "Smith, Doe, and van der Berg (2023)"

**With Pandoc Output:**
```javascript
const authors = entry.author.map(a => a.family).join(", ");
const year = entry.issued;
console.log(`${authors} (${year})`); // "Smith, Doe, Berg (2023)"
```

**With Our Output:**
```javascript
// Need to parse author strings manually
const authors = metadata.authors.map(a => {
  // Parse "Smith, John" → "Smith"
  // Parse "van der Berg, Hans" → "van der Berg" or "Berg"?
  // Complex logic needed!
});
```

### Use Case 2: Bibliography Search

**Goal**: Find all papers by author "Berg"

**With Pandoc Output:**
```javascript
papers.filter(p => p.author.some(a => a.family === "Berg"));
```

**With Our Output:**
```javascript
// Need to handle various formats
papers.filter(p =>
  p.metadata.authors.some(a =>
    a.includes("Berg") || a.includes("berg")
  )
);
// Might match "Bloomberg" or "Goldberg" incorrectly
```

### Use Case 3: Export to Citation Manager

**Goal**: Import into Zotero

**With Pandoc Output:**
- Direct CSL-JSON import supported
- All fields properly mapped
- Names properly structured

**With Our Output:**
- Need custom parser
- Must implement all field mappings
- Must parse author names manually
- Or export as BibTeX (circular dependency)

---

## Recommendations Priority

Based on this comparison, here are the priorities:

### CRITICAL (Blocks Production Use)

1. **Implement CSL-JSON Output** (3 days)
   - This is the root issue that causes all downstream problems
   - Without it, integrations are very difficult
   - With it, we become compatible with entire citation ecosystem

2. **Parse Author Names** (1-2 days)
   - Required for CSL-JSON output
   - Required for proper citation formatting
   - Cannot be worked around

3. **Map Fields to CSL Names** (4-8 hours)
   - Required for CSL-JSON output
   - Straightforward mapping table
   - Well-documented standard

### HIGH (Should Have for v1.0)

4. **Normalize Entry Types** (2-4 hours)
   - Easy addition to CSL-JSON output
   - Important for compatibility

5. **Handle Cross-References** (1-2 days)
   - Important for complete metadata
   - Common in conference proceedings

6. **Expand String Variables** (1-2 days)
   - Important for correct names
   - Common in large bibliographies

### MEDIUM (Nice to Have)

7. **LaTeX to Unicode** (1-2 days)
   - Important for display quality
   - Less critical if using CSL processors (they often handle it)

8. **Full Date Parsing** (1 day)
   - BibLaTeX extension
   - Not widely used yet

---

## Implementation Path

### Option A: Full CSL-JSON Support (Recommended)

**Effort**: 5-7 days
**Parity**: 85-90%

1. Implement author name parsing
2. Implement field mapping
3. Implement entry type normalization
4. Add CSL-JSON output format
5. Add cross-reference resolution
6. Add string variable expansion

**Result**: Production-ready extractor compatible with citation tools

### Option B: Minimal Improvements

**Effort**: 2-3 days
**Parity**: 70-75%

1. Implement author name parsing
2. Add basic field mapping
3. Keep BibTeX output format

**Result**: Better metadata but still not citation-tool compatible

### Option C: Status Quo

**Effort**: 0 days
**Parity**: 65%

**Result**: Works for basic extraction but poor for citation workflows

---

## Conclusion

Our BibTeX extractor handles the basics well but lacks the structured output needed for modern citation workflows. The main gap is **CSL-JSON output**, which requires:

1. Structured author name parsing
2. Field mapping to CSL schema
3. Entry type normalization

Implementing these features would bring us from **65% parity** to **~90% parity** and make the extractor production-ready for citation management applications.

**Recommended Investment**: 5-7 days for full CSL-JSON support (Option A)

---

## References

- CSL-JSON Schema: https://citeproc-js.readthedocs.io/en/latest/csl-json/markup.html
- Pandoc BibTeX: https://pandoc.org/MANUAL.html#citations
- Citation Style Language: https://citationstyles.org/
- BibTeX Format: http://www.bibtex.org/Format/

**Generated**: 2025-12-06
