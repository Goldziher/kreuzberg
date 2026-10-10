// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_DATAAVAIL_H_
#define PUBLIC_FPDF_DATAAVAIL_H_

#include <stddef.h>

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#define PDF_LINEARIZATION_UNKNOWN -1
#define PDF_NOT_LINEARIZED 0
#define PDF_LINEARIZED 1

#define PDF_DATA_ERROR -1
#define PDF_DATA_NOTAVAIL 0
#define PDF_DATA_AVAIL 1

#define PDF_FORM_ERROR -1
#define PDF_FORM_NOTAVAIL 0
#define PDF_FORM_AVAIL 1
#define PDF_FORM_NOTEXIST 2

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

// Interface for checking whether sections of the file are available.
typedef struct _FX_FILEAVAIL {
  // Version number of the interface. Must be 1.
  int version;

  // Reports if the specified data section is currently available. A section is
  // available if all bytes in the section are available.
  //
  // Interface Version: 1
  // Implementation Required: Yes
  //
  //   pThis  - pointer to the interface structure.
  //   offset - the offset of the data section in the file.
  //   size   - the size of the data section.
  //
  // Returns true if the specified data section at |offset| of |size|
  // is available.
  FPDF_BOOL (*IsDataAvail)(struct _FX_FILEAVAIL* pThis,
                           size_t offset,
                           size_t size);
} FX_FILEAVAIL;

// Create a document availability provider.
//
//   file_avail - pointer to file availability interface.
//   file       - pointer to a file access interface.
//
// Returns a handle to the document availability provider, or NULL on error.
//
// FPDFAvail_Destroy() must be called when done with the availability provider.
FPDF_EXPORT FPDF_AVAIL FPDF_CALLCONV FPDFAvail_Create(FX_FILEAVAIL* file_avail,
                                                      FPDF_FILEACCESS* file);

// Destroy the |avail| document availability provider.
//
//   avail - handle to document availability provider to be destroyed.
FPDF_EXPORT void FPDF_CALLCONV FPDFAvail_Destroy(FPDF_AVAIL avail);

// Download hints interface. Used to receive hints for further downloading.
typedef struct _FX_DOWNLOADHINTS {
  // Version number of the interface. Must be 1.
  int version;

  // Add a section to be downloaded.
  //
  // Interface Version: 1
  // Implementation Required: Yes
  //
  //   pThis  - pointer to the interface structure.
  //   offset - the offset of the hint reported to be downloaded.
  //   size   - the size of the hint reported to be downloaded.
  //
  // The |offset| and |size| of the section may not be unique. Part of the
  // section might be already available. The download manager must deal with
  // overlapping sections.
  void (*AddSegment)(struct _FX_DOWNLOADHINTS* pThis,
                     size_t offset,
                     size_t size);
} FX_DOWNLOADHINTS;

// Checks if the document is ready for loading, if not, gets download hints.
//
//   avail - handle to document availability provider.
//   hints - pointer to a download hints interface.
//
// Returns one of:
//   PDF_DATA_ERROR: A common error is returned. Data availability unknown.
//   PDF_DATA_NOTAVAIL: Data not yet available.
//   PDF_DATA_AVAIL: Data available.
//
// Applications should call this function whenever new data arrives, and process
// all the generated download hints, if any, until the function returns
// |PDF_DATA_ERROR| or |PDF_DATA_AVAIL|.
// if hints is nullptr, the function just check current document availability.
//
// Once all data is available, call FPDFAvail_GetDocument() to get a document
// handle.
FPDF_EXPORT int FPDF_CALLCONV FPDFAvail_IsDocAvail(FPDF_AVAIL avail,
                                                   FX_DOWNLOADHINTS* hints);

// Get document from the availability provider.
//
//   avail    - handle to document availability provider.
//   password - password for decrypting the PDF file. Optional.
//
// Returns a handle to the document.
//
// When FPDFAvail_IsDocAvail() returns TRUE, call FPDFAvail_GetDocument() to
// retrieve the document handle.
// See the comments for FPDF_LoadDocument() regarding the encoding for
// |password|.
FPDF_EXPORT FPDF_DOCUMENT FPDF_CALLCONV
FPDFAvail_GetDocument(FPDF_AVAIL avail, FPDF_BYTESTRING password);

// Get the page number for the first available page in a linearized PDF.
//
//   doc - document handle.
//
// Returns the zero-based index for the first available page.
//
// For most linearized PDFs, the first available page will be the first page,
// however, some PDFs might make another page the first available page.
// For non-linearized PDFs, this function will always return zero.
FPDF_EXPORT int FPDF_CALLCONV FPDFAvail_GetFirstPageNum(FPDF_DOCUMENT doc);

// Check if |page_index| is ready for loading, if not, get the
// |FX_DOWNLOADHINTS|.
//
//   avail      - handle to document availability provider.
//   page_index - index number of the page. Zero for the first page.
//   hints      - pointer to a download hints interface. Populated if
//                |page_index| is not available.
//
// Returns one of:
//   PDF_DATA_ERROR: A common error is returned. Data availability unknown.
//   PDF_DATA_NOTAVAIL: Data not yet available.
//   PDF_DATA_AVAIL: Data available.
//
// This function can be called only after FPDFAvail_GetDocument() is called.
// Applications should call this function whenever new data arrives and process
// all the generated download |hints|, if any, until this function returns
// |PDF_DATA_ERROR| or |PDF_DATA_AVAIL|. Applications can then perform page
// loading.
// if hints is nullptr, the function just check current availability of
// specified page.
FPDF_EXPORT int FPDF_CALLCONV FPDFAvail_IsPageAvail(FPDF_AVAIL avail,
                                                    int page_index,
                                                    FX_DOWNLOADHINTS* hints);

// Check if form data is ready for initialization, if not, get the
// |FX_DOWNLOADHINTS|.
//
//   avail - handle to document availability provider.
//   hints - pointer to a download hints interface. Populated if form is not
//           ready for initialization.
//
// Returns one of:
//   PDF_FORM_ERROR: A common eror, in general incorrect parameters.
//   PDF_FORM_NOTAVAIL: Data not available.
//   PDF_FORM_AVAIL: Data available.
//   PDF_FORM_NOTEXIST: No form data.
//
// This function can be called only after FPDFAvail_GetDocument() is called.
// The application should call this function whenever new data arrives and
// process all the generated download |hints|, if any, until the function
// |PDF_FORM_ERROR|, |PDF_FORM_AVAIL| or |PDF_FORM_NOTEXIST|.
// if hints is nullptr, the function just check current form availability.
//
// Applications can then perform page loading. It is recommend to call
// FPDFDOC_InitFormFillEnvironment() when |PDF_FORM_AVAIL| is returned.
FPDF_EXPORT int FPDF_CALLCONV FPDFAvail_IsFormAvail(FPDF_AVAIL avail,
                                                    FX_DOWNLOADHINTS* hints);

// Check whether a document is a linearized PDF.
//
//   avail - handle to document availability provider.
//
// Returns one of:
//   PDF_LINEARIZED
//   PDF_NOT_LINEARIZED
//   PDF_LINEARIZATION_UNKNOWN
//
// FPDFAvail_IsLinearized() will return |PDF_LINEARIZED| or |PDF_NOT_LINEARIZED|
// when we have 1k  of data. If the files size less than 1k, it returns
// |PDF_LINEARIZATION_UNKNOWN| as there is insufficient information to determine
// if the PDF is linearlized.
FPDF_EXPORT int FPDF_CALLCONV FPDFAvail_IsLinearized(FPDF_AVAIL avail);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_DATAAVAIL_H_
