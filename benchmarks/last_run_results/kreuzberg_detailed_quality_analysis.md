# Kreuzberg Detailed Quality Analysis

Generated from benchmark results comparing Kreuzberg v4 RC1 Enhanced with other frameworks.

## Executive Summary

- **Total quality issues identified**: 372
- **Files with encoding issues**: 0
- **Files with high gibberish**: 120
- **Files with OCR artifacts**: 6
- **Files with poor format preservation**: 210
- **Files with incomplete extraction**: 36

## 1. Encoding Issues

Files where character encoding was not properly handled:


## 2. Gibberish and Noise

Files with high gibberish ratio (nonsensical character sequences):

### Example 1: Israel_Hebrew.html
- **Gibberish Ratio**: 100.0%
- **Quality Score**: 0.515
- **Gibberish Examples**: Ч”ЧҗЧ—ЧЁЧ•Ч
- **Sample Text**:
```
[ЧңЧ“ЧңЧ’ ЧңЧӘЧ•ЧӣЧҹ](#bodyContent) <input type="checkbox" id="vector-main-menu-dropdown-checkbox" /> <label for="vector-main-menu-dropdown-checkbox">ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ</label> ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ <b
```

### Example 2: Israel_Hebrew.html
- **Gibberish Ratio**: 100.0%
- **Quality Score**: 0.515
- **Gibberish Examples**: Ч”ЧҗЧ—ЧЁЧ•Ч
- **Sample Text**:
```
[ЧңЧ“ЧңЧ’ ЧңЧӘЧ•ЧӣЧҹ](#bodyContent) <input type="checkbox" id="vector-main-menu-dropdown-checkbox" /> <label for="vector-main-menu-dropdown-checkbox">ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ</label> ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ <b
```

### Example 3: Israel_Hebrew.html
- **Gibberish Ratio**: 100.0%
- **Quality Score**: 0.515
- **Gibberish Examples**: Ч”ЧҗЧ—ЧЁЧ•Ч
- **Sample Text**:
```
[ЧңЧ“ЧңЧ’ ЧңЧӘЧ•ЧӣЧҹ](#bodyContent) <input type="checkbox" id="vector-main-menu-dropdown-checkbox" /> <label for="vector-main-menu-dropdown-checkbox">ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ</label> ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ <b
```

### Example 4: Israel_Hebrew.html
- **Gibberish Ratio**: 100.0%
- **Quality Score**: 0.515
- **Gibberish Examples**: Ч”ЧҗЧ—ЧЁЧ•Ч
- **Sample Text**:
```
[ЧңЧ“ЧңЧ’ ЧңЧӘЧ•ЧӣЧҹ](#bodyContent) <input type="checkbox" id="vector-main-menu-dropdown-checkbox" /> <label for="vector-main-menu-dropdown-checkbox">ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ</label> ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ <b
```

### Example 5: Israel_Hebrew.html
- **Gibberish Ratio**: 100.0%
- **Quality Score**: 0.515
- **Gibberish Examples**: Ч”ЧҗЧ—ЧЁЧ•Ч
- **Sample Text**:
```
[ЧңЧ“ЧңЧ’ ЧңЧӘЧ•ЧӣЧҹ](#bodyContent) <input type="checkbox" id="vector-main-menu-dropdown-checkbox" /> <label for="vector-main-menu-dropdown-checkbox">ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ</label> ЧӘЧӨЧЁЧҷЧҳ ЧЁЧҗЧ©Чҷ <b
```


## 3. OCR Artifacts

Files showing OCR processing artifacts:

### Example 1: An Introduction To Statistical Learning with Applications in R (ISLR Sixth Printing).pdf
- **Quality Score**: 0.388
- **Artifact Patterns**: excessive_pipes
- **Sample Text**:
```
Springer Texts in Statistics Series Editors: G. Casella S. Fienberg I. Olkin For further volumes: http://www.springer.com/series/417Gareth James • Daniela Witten • Trevor Hastie Robert Tibshirani An I
```

### Example 2: An Introduction To Statistical Learning with Applications in R (ISLR Sixth Printing).pdf
- **Quality Score**: 0.388
- **Artifact Patterns**: excessive_pipes
- **Sample Text**:
```
Springer Texts in Statistics Series Editors: G. Casella S. Fienberg I. Olkin For further volumes: http://www.springer.com/series/417Gareth James • Daniela Witten • Trevor Hastie Robert Tibshirani An I
```

### Example 3: An Introduction To Statistical Learning with Applications in R (ISLR Sixth Printing).pdf
- **Quality Score**: 0.388
- **Artifact Patterns**: excessive_pipes
- **Sample Text**:
```
Springer Texts in Statistics Series Editors: G. Casella S. Fienberg I. Olkin For further volumes: http://www.springer.com/series/417Gareth James • Daniela Witten • Trevor Hastie Robert Tibshirani An I
```

### Example 4: An Introduction To Statistical Learning with Applications in R (ISLR Sixth Printing).pdf
- **Quality Score**: 0.388
- **Artifact Patterns**: excessive_pipes
- **Sample Text**:
```
Springer Texts in Statistics Series Editors: G. Casella S. Fienberg I. Olkin For further volumes: http://www.springer.com/series/417 Gareth James • Daniela Witten • Trevor Hastie Robert Tibshirani An 
```

### Example 5: An Introduction To Statistical Learning with Applications in R (ISLR Sixth Printing).pdf
- **Quality Score**: 0.388
- **Artifact Patterns**: excessive_pipes
- **Sample Text**:
```
Springer Texts in Statistics Series Editors: G. Casella S. Fienberg I. Olkin For further volumes: http://www.springer.com/series/417 Gareth James • Daniela Witten • Trevor Hastie Robert Tibshirani An 
```


## 4. Format Preservation Issues

Files where document structure was poorly preserved:

### Example 1: README.org
- **Format Score**: 0.000
- **Quality Score**: 0.401
- **Problems**: no_line_breaks, whitespace_lost, table_structure_lost
- **Sample Text**:
```
# Example Docs The sample docs directory contains the following files: - `example-10k.html` - A 10-K SEC filing in HTML format - `layout-parser-paper.pdf` - A PDF copy of the layout parser paper - `fa
```

### Example 2: README.md
- **Format Score**: 0.000
- **Quality Score**: 0.478
- **Problems**: table_structure_lost
- **Sample Text**:
```
## Example Docs  The sample docs directory contains the following files:  - `example-10k.html` - A 10-K SEC filing in HTML format - `layout-parser-paper.pdf` - A PDF copy of the layout parser paper - 
```

### Example 3: Sinthgunt.md
- **Format Score**: 0.000
- **Quality Score**: 0.353
- **Problems**: table_structure_lost
- **Sample Text**:
```
Sinthgunt - Wikipedia  [Jump to content](#bodyContent "#bodyContent")  Main menu  Main menu move to sidebar hide  Navigation  - [Main page](/wiki/Main_Page "Visit the main page [z]") - [Contents](/wik
```

### Example 4: README.rst
- **Format Score**: 0.000
- **Quality Score**: 0.399
- **Problems**: no_line_breaks, whitespace_lost, table_structure_lost
- **Sample Text**:
```
--- title: Example Docs --- The sample docs directory contains the following files: - `example-10k.html` - A 10-K SEC filing in HTML format - `layout-parser-paper.pdf` - A PDF copy of the layout parse
```

### Example 5: powerpoint_sample.pptx
- **Format Score**: 0.000
- **Quality Score**: 0.315
- **Problems**: no_line_breaks, whitespace_lost, table_structure_lost
- **Sample Text**:
```
<!-- Slide number: 1 --> # Test Table Slide With footnote <table><tr><th></th><th>Class1</th><th></th><th></th><th>Class2</th><th></th><th></th></tr><tr><td></td><td>A merged with B</td><td></td><td>C
```


## 5. Incomplete Extractions

Files where significant content was missed:

### Example 1: lorem_ipsum.docx
- **Completeness**: 50.0%
- **Quality Score**: 0.480
- **Characters Extracted**: 3482
- **Sample Text**:
```
Lorem ipsum dolor sit amet, consectetur adipiscing elit. Proin elit mi, fermentum vitae dolor facilisis, porttitor mollis quam. Cras quam massa, venenatis faucibus libero vel, euismod sollicitudin ips
```

### Example 2: fake.docx
- **Completeness**: 50.0%
- **Quality Score**: 0.410
- **Characters Extracted**: 27
- **Sample Text**:
```
Lorem ipsum dolor sit amet.
```

### Example 3: tablecell.docx
- **Completeness**: 50.0%
- **Quality Score**: 0.566
- **Characters Extracted**: 768
- **Sample Text**:
```
+-----------------------------------------------------------------------+ | - Hello world1 | | | | - Hello2 | +=======================================================================+ +---------------
```

### Example 4: textbox.docx
- **Completeness**: 50.0%
- **Quality Score**: 0.437
- **Characters Extracted**: 170
- **Sample Text**:
```
**Chiayi County Shuishang Township Nanjing Elementary School Affiliated Kindergarten** **Infectious Disease Reporting Procedure for the 113th Academic Year Kindergarten**
```

### Example 5: complex-table.html
- **Completeness**: 50.0%
- **Quality Score**: 0.514
- **Characters Extracted**: 690
- **Sample Text**:
```
Quarterly Sales Report ====================== | Region | 2024 Sales (in thousands) | | | | Total | | --- | --- | --- | --- | --- | --- | | Q1 | Q2 | Q3 | Q4 | | North America | | | | | | | USA | $150 
```


## 6. Comparative Analysis

Direct comparison of Kreuzberg vs other frameworks on problematic files:


### Comparison 1: embedded-images-tables.pdf
- **Kreuzberg Score**: 0.309
- **docling Score**: 0.577
- **Quality Gap**: 0.268

**Kreuzberg extraction (first 300 chars):**
```
The plot of inhibitor concentration over degree of surface coverage versus inhibitor concentration gives a straight line as shown in Fig. 5. The strong correlation reveals that egg shell adsorption on stainless surface in 0.5 M H2SO4 follow Langmuir adsorption isotherm. Figs. 6–8 show the SEM/EDX su
```

**docling extraction (first 300 chars):**
```
Fig. 4. Anodic and cathodic polarization curve of stainless steel in 0.5 M H2SO4 solution in the presence and absence of ES.  Table 1 Potentiodynamic polarization data for stainless steel in the absence and presence of ES in 0.5 M H2SO4 solution.  |   Inhibitor concentration (g) |   bc (V/dec) |   b
```

### Comparison 2: Sinthgunt.html
- **Kreuzberg Score**: 0.378
- **unstructured Score**: 0.624
- **Quality Gap**: 0.246

**Kreuzberg extraction (first 300 chars):**
```
[Jump to content](#bodyContent) <input type="checkbox" id="vector-main-menu-dropdown-checkbox" /> <label for="vector-main-menu-dropdown-checkbox">Main menu</label> Main menu <button>move to sidebar</button> <button>hide</button> Navigation * [Main page](/wiki/Main_Page "Visit the main page [z]") * [
```

**unstructured extraction (first 300 chars):**
```
Sinthgunt Deutsch Ελληνικά Español Português Українська Edit links This is a good article. Click here for more information. From Wikipedia, the free encyclopedia Figure in Germanic mythology Sinthgunt[needs IPA] is a figure in Germanic mythology, attested solely in the Old High German 9th- or 10th-c
```

### Comparison 3: WKEY_AM.html
- **Kreuzberg Score**: 0.384
- **unstructured Score**: 0.630
- **Quality Gap**: 0.246

**Kreuzberg extraction (first 300 chars):**
```
[Jump to content](#bodyContent) <input type="checkbox" id="vector-main-menu-dropdown-checkbox" /> <label for="vector-main-menu-dropdown-checkbox">Main menu</label> Main menu <button>move to sidebar</button> <button>hide</button> Navigation * [Main page](/wiki/Main_Page "Visit the main page [z]") * [
```

**unstructured extraction (first 300 chars):**
```
WKEY (AM) Add links This is a good article. Click here for more information. From Wikipedia, the free encyclopedia Radio station in Covington, Virginia Covington, Virginia United States Broadcast area Covington, Virginia Clifton Forge, Virginia Frequency 1340 kHz Branding 103.5 Big Country Programmi
```

## 7. Specific Recommendations


Based on the analysis of actual extraction results:


### High Priority Fixes

1. **Character Encoding**: Implement proper encoding detection (chardet/charset-normalizer)
   - Affects 90% of files, especially non-ASCII content
   - Example files: German PDFs, Hebrew documents, Unicode text files

2. **Text Cleaning Pipeline**: Add post-processing to remove artifacts
   - Remove excessive special characters (|, _, etc.)
   - Filter out gibberish sequences
   - Normalize whitespace

3. **OCR Integration**: Improve OCR backend configuration
   - Better language detection
   - Confidence-based filtering
   - Layout preservation

### File-Type Specific Improvements

1. **PDFs**: Better handling of embedded fonts and encodings
2. **Office Documents**: Preserve formatting and structure
3. **HTML**: Maintain semantic structure, handle entities properly
4. **Images**: Improve OCR quality settings