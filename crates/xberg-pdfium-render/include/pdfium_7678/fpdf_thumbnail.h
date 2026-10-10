// Copyright 2019 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef PUBLIC_FPDF_THUMBNAIL_H_
#define PUBLIC_FPDF_THUMBNAIL_H_

#include <stdint.h>

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif

// Experimental API.
// Gets the decoded data from the thumbnail of |page| if it exists.
// This only modifies |buffer| if |buflen| less than or equal to the
// size of the decoded data. Returns the size of the decoded
// data or 0 if thumbnail DNE. Optional, pass null to just retrieve
// the size of the buffer needed.
//
//   page    - handle to a page.
//   buffer  - buffer for holding the decoded image data.
//   buflen  - length of the buffer in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFPage_GetDecodedThumbnailData(FPDF_PAGE page,
                                 void* buffer,
                                 unsigned long buflen);

// Experimental API.
// Gets the raw data from the thumbnail of |page| if it exists.
// This only modifies |buffer| if |buflen| is less than or equal to
// the size of the raw data. Returns the size of the raw data or 0
// if thumbnail DNE. Optional, pass null to just retrieve the size
// of the buffer needed.
//
//   page    - handle to a page.
//   buffer  - buffer for holding the raw image data.
//   buflen  - length of the buffer in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFPage_GetRawThumbnailData(FPDF_PAGE page,
                             void* buffer,
                             unsigned long buflen);

// Experimental API.
// Returns the thumbnail of |page| as a FPDF_BITMAP. Returns a nullptr
// if unable to access the thumbnail's stream.
//
//   page - handle to a page.
FPDF_EXPORT FPDF_BITMAP FPDF_CALLCONV
FPDFPage_GetThumbnailAsBitmap(FPDF_PAGE page);

#ifdef __cplusplus
}
#endif

#endif  // PUBLIC_FPDF_THUMBNAIL_H_
