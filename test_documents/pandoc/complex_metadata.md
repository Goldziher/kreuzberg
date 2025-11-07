______________________________________________________________________

title: Complex Metadata Test Document
subtitle: Testing Nested and Advanced Metadata Structures
author:

- name: Alice Johnson
  affiliation: University A
  email: <alice@example.com>
- name: Bob Smith
  affiliation:
  - Institute B
  - Research Lab C
    orcid: 0000-0002-1825-0097
    date: 2025-09-27
    abstract: |
    This document tests complex metadata extraction scenarios.

It includes multiple paragraph abstracts with various formatting:

- Bullet points
- _Emphasis_ and **strong** text
- Even `inline code`

The abstract continues here.
keywords:

- complex metadata
- nested structures
- pandoc testing
- multi-level data
  tags: [test, document, metadata, extraction]
  lang: en-US
  bibliography: references.bib
  csl: apa.csl
  link-citations: true
  toc: true
  toc-depth: 3
  documentclass: article
  classoption:
- 11pt
- letterpaper
  geometry:
- margin=1in
  header-includes:
- \\usepackage{graphicx}
- \\usepackage{amsmath}
  variables:
  version: "2.0.1"
  status: "draft"
  confidential: true
  custom-metadata:
  project-id: "PROJ-2025-001"
  review-status: "pending"
  internal-notes: |
  This is for internal use only.
  Do not distribute.

______________________________________________________________________

# Document Content

This section contains the actual document content after the complex metadata block.
