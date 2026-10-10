// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_SEARCHEX_H_
#define PUBLIC_FPDF_SEARCHEX_H_

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

// Get the character index in |text_page| internal character list.
//
//   text_page  - a text page information structure.
//   nTextIndex - index of the text returned from FPDFText_GetText().
//
// Returns the index of the character in internal character list. -1 for error.
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_GetCharIndexFromTextIndex(FPDF_TEXTPAGE text_page, int nTextIndex);

// Get the text index in |text_page| internal character list.
//
//   text_page  - a text page information structure.
//   nCharIndex - index of the character in internal character list.
//
// Returns the index of the text returned from FPDFText_GetText(). -1 for error.
FPDF_EXPORT int FPDF_CALLCONV
FPDFText_GetTextIndexFromCharIndex(FPDF_TEXTPAGE text_page, int nCharIndex);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_SEARCHEX_H_
