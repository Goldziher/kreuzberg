// Copyright 2019 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef PUBLIC_FPDF_JAVASCRIPT_H_
#define PUBLIC_FPDF_JAVASCRIPT_H_

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

// Experimental API.
// Get the number of JavaScript actions in |document|.
//
//   document - handle to a document.
//
// Returns the number of JavaScript actions in |document| or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDFDoc_GetJavaScriptActionCount(FPDF_DOCUMENT document);

// Experimental API.
// Get the JavaScript action at |index| in |document|.
//
//   document - handle to a document.
//   index    - the index of the requested JavaScript action.
//
// Returns the handle to the JavaScript action, or NULL on failure.
// Caller owns the returned handle and must close it with
// FPDFDoc_CloseJavaScriptAction().
FPDF_EXPORT FPDF_JAVASCRIPT_ACTION FPDF_CALLCONV
FPDFDoc_GetJavaScriptAction(FPDF_DOCUMENT document, int index);

// Experimental API.
// Close a loaded FPDF_JAVASCRIPT_ACTION object.

//   javascript - Handle to a JavaScript action.
FPDF_EXPORT void FPDF_CALLCONV
FPDFDoc_CloseJavaScriptAction(FPDF_JAVASCRIPT_ACTION javascript);

// Experimental API.
// Get the name from the |javascript| handle. |buffer| is only modified if
// |buflen| is longer than the length of the name. On errors, |buffer| is
// unmodified and the returned length is 0.
//
//   javascript - handle to an JavaScript action.
//   buffer     - buffer for holding the name, encoded in UTF-16LE.
//   buflen     - length of the buffer in bytes.
//
// Returns the length of the JavaScript action name in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFJavaScriptAction_GetName(FPDF_JAVASCRIPT_ACTION javascript,
                             FPDF_WCHAR* buffer,
                             unsigned long buflen);

// Experimental API.
// Get the script from the |javascript| handle. |buffer| is only modified if
// |buflen| is longer than the length of the script. On errors, |buffer| is
// unmodified and the returned length is 0.
//
//   javascript - handle to an JavaScript action.
//   buffer     - buffer for holding the name, encoded in UTF-16LE.
//   buflen     - length of the buffer in bytes.
//
// Returns the length of the JavaScript action name in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFJavaScriptAction_GetScript(FPDF_JAVASCRIPT_ACTION javascript,
                               FPDF_WCHAR* buffer,
                               unsigned long buflen);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_JAVASCRIPT_H_
