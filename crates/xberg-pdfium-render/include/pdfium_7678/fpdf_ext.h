// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_EXT_H_
#define PUBLIC_FPDF_EXT_H_

#include <time.h>

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

// Unsupported XFA form.
#define FPDF_UNSP_DOC_XFAFORM 1
// Unsupported portable collection.
#define FPDF_UNSP_DOC_PORTABLECOLLECTION 2
// Unsupported attachment.
#define FPDF_UNSP_DOC_ATTACHMENT 3
// Unsupported security.
#define FPDF_UNSP_DOC_SECURITY 4
// Unsupported shared review.
#define FPDF_UNSP_DOC_SHAREDREVIEW 5
// Unsupported shared form, acrobat.
#define FPDF_UNSP_DOC_SHAREDFORM_ACROBAT 6
// Unsupported shared form, filesystem.
#define FPDF_UNSP_DOC_SHAREDFORM_FILESYSTEM 7
// Unsupported shared form, email.
#define FPDF_UNSP_DOC_SHAREDFORM_EMAIL 8
// Unsupported 3D annotation.
#define FPDF_UNSP_ANNOT_3DANNOT 11
// Unsupported movie annotation.
#define FPDF_UNSP_ANNOT_MOVIE 12
// Unsupported sound annotation.
#define FPDF_UNSP_ANNOT_SOUND 13
// Unsupported screen media annotation.
#define FPDF_UNSP_ANNOT_SCREEN_MEDIA 14
// Unsupported screen rich media annotation.
#define FPDF_UNSP_ANNOT_SCREEN_RICHMEDIA 15
// Unsupported attachment annotation.
#define FPDF_UNSP_ANNOT_ATTACHMENT 16
// Unsupported signature annotation.
#define FPDF_UNSP_ANNOT_SIG 17

// Interface for unsupported feature notifications.
typedef struct _UNSUPPORT_INFO {
  // Version number of the interface. Must be 1.
  int version;

  // Unsupported object notification function.
  // Interface Version: 1
  // Implementation Required: Yes
  //
  //   pThis - pointer to the interface structure.
  //   nType - the type of unsupported object. One of the |FPDF_UNSP_*| entries.
  void (*FSDK_UnSupport_Handler)(struct _UNSUPPORT_INFO* pThis, int nType);
} UNSUPPORT_INFO;

// Setup an unsupported object handler.
//
//   unsp_info - Pointer to an UNSUPPORT_INFO structure.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FSDK_SetUnSpObjProcessHandler(UNSUPPORT_INFO* unsp_info);

// Set replacement function for calls to time().
//
// This API is intended to be used only for testing, thus may cause PDFium to
// behave poorly in production environments.
//
//   func - Function pointer to alternate implementation of time(), or
//          NULL to restore to actual time() call itself.
FPDF_EXPORT void FPDF_CALLCONV FSDK_SetTimeFunction(time_t (*func)());

// Set replacement function for calls to localtime().
//
// This API is intended to be used only for testing, thus may cause PDFium to
// behave poorly in production environments.
//
//   func - Function pointer to alternate implementation of localtime(), or
//          NULL to restore to actual localtime() call itself.
FPDF_EXPORT void FPDF_CALLCONV
FSDK_SetLocaltimeFunction(struct tm* (*func)(const time_t*));

// Unknown page mode.
#define PAGEMODE_UNKNOWN -1
// Document outline, and thumbnails hidden.
#define PAGEMODE_USENONE 0
// Document outline visible.
#define PAGEMODE_USEOUTLINES 1
// Thumbnail images visible.
#define PAGEMODE_USETHUMBS 2
// Full-screen mode, no menu bar, window controls, or other decorations visible.
#define PAGEMODE_FULLSCREEN 3
// Optional content group panel visible.
#define PAGEMODE_USEOC 4
// Attachments panel visible.
#define PAGEMODE_USEATTACHMENTS 5

// Get the document's PageMode.
//
//   doc - Handle to document.
//
// Returns one of the |PAGEMODE_*| flags defined above.
//
// The page mode defines how the document should be initially displayed.
FPDF_EXPORT int FPDF_CALLCONV FPDFDoc_GetPageMode(FPDF_DOCUMENT document);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_EXT_H_
