// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_SAVE_H_
#define PUBLIC_FPDF_SAVE_H_

// clang-format off
// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif

// Structure for custom file write
typedef struct FPDF_FILEWRITE_ {
  //
  // Version number of the interface. Currently must be 1.
  //
  int version;

  // Method: WriteBlock
  //          Output a block of data in your custom way.
  // Interface Version:
  //          1
  // Implementation Required:
  //          Yes
  // Comments:
  //          Called by function FPDF_SaveDocument
  // Parameters:
  //          self        -   Pointer to the structure itself
  //          data        -   Pointer to a buffer to output
  //          size        -   The size of the buffer.
  // Return value:
  //          Should be non-zero if successful, zero for error.
  int (*WriteBlock)(struct FPDF_FILEWRITE_* self,
                    const void* data,
                    unsigned long size);
} FPDF_FILEWRITE;

 // Flags for FPDF_SaveAsCopy()
#define FPDF_INCREMENTAL 1
#define FPDF_NO_INCREMENTAL 2
#define FPDF_REMOVE_SECURITY 3

// Function: FPDF_SaveAsCopy
//          Saves the copy of specified document in custom way.
// Parameters:
//          document        -   Handle to document, as returned by
//                              FPDF_LoadDocument() or FPDF_CreateNewDocument().
//          file_write      -   A pointer to a custom file write structure.
//          flags           -   Flags above that affect how the PDF gets saved.
//                              Pass in 0 when there are no flags.
// Return value:
//          TRUE for succeed, FALSE for failed.
//
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_SaveAsCopy(FPDF_DOCUMENT document,
                                                    FPDF_FILEWRITE* file_write,
                                                    FPDF_DWORD flags);

// Function: FPDF_SaveWithVersion
//          Same as FPDF_SaveAsCopy(), except the file version of the
//          saved document can be specified by the caller.
// Parameters:
//          document        -   Handle to document.
//          file_write      -   A pointer to a custom file write structure.
//          flags           -   The creating flags.
//          file_version    -   The PDF file version. File version: 14 for 1.4,
//                              15 for 1.5, ...
// Return value:
//          TRUE if succeed, FALSE if failed.
//
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_SaveWithVersion(FPDF_DOCUMENT document,
                     FPDF_FILEWRITE* file_write,
                     FPDF_DWORD flags,
                     int file_version);

#ifdef __cplusplus
}
#endif

#endif  // PUBLIC_FPDF_SAVE_H_
