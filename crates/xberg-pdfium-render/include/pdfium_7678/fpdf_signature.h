// Copyright 2020 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef PUBLIC_FPDF_SIGNATURE_H_
#define PUBLIC_FPDF_SIGNATURE_H_

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

// Experimental API.
// Function: FPDF_GetSignatureCount
//          Get total number of signatures in the document.
// Parameters:
//          document    -   Handle to document. Returned by FPDF_LoadDocument().
// Return value:
//          Total number of signatures in the document on success, -1 on error.
FPDF_EXPORT int FPDF_CALLCONV FPDF_GetSignatureCount(FPDF_DOCUMENT document);

// Experimental API.
// Function: FPDF_GetSignatureObject
//          Get the Nth signature of the document.
// Parameters:
//          document    -   Handle to document. Returned by FPDF_LoadDocument().
//          index       -   Index into the array of signatures of the document.
// Return value:
//          Returns the handle to the signature, or NULL on failure. The caller
//          does not take ownership of the returned FPDF_SIGNATURE. Instead, it
//          remains valid until FPDF_CloseDocument() is called for the document.
FPDF_EXPORT FPDF_SIGNATURE FPDF_CALLCONV
FPDF_GetSignatureObject(FPDF_DOCUMENT document, int index);

// Experimental API.
// Function: FPDFSignatureObj_GetContents
//          Get the contents of a signature object.
// Parameters:
//          signature   -   Handle to the signature object. Returned by
//                          FPDF_GetSignatureObject().
//          buffer      -   The address of a buffer that receives the contents.
//          length      -   The size, in bytes, of |buffer|.
// Return value:
//          Returns the number of bytes in the contents on success, 0 on error.
//
// For public-key signatures, |buffer| is either a DER-encoded PKCS#1 binary or
// a DER-encoded PKCS#7 binary. If |length| is less than the returned length, or
// |buffer| is NULL, |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFSignatureObj_GetContents(FPDF_SIGNATURE signature,
                             void* buffer,
                             unsigned long length);

// Experimental API.
// Function: FPDFSignatureObj_GetByteRange
//          Get the byte range of a signature object.
// Parameters:
//          signature   -   Handle to the signature object. Returned by
//                          FPDF_GetSignatureObject().
//          buffer      -   The address of a buffer that receives the
//                          byte range.
//          length      -   The size, in ints, of |buffer|.
// Return value:
//          Returns the number of ints in the byte range on
//          success, 0 on error.
//
// |buffer| is an array of pairs of integers (starting byte offset,
// length in bytes) that describes the exact byte range for the digest
// calculation. If |length| is less than the returned length, or
// |buffer| is NULL, |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFSignatureObj_GetByteRange(FPDF_SIGNATURE signature,
                              int* buffer,
                              unsigned long length);

// Experimental API.
// Function: FPDFSignatureObj_GetSubFilter
//          Get the encoding of the value of a signature object.
// Parameters:
//          signature   -   Handle to the signature object. Returned by
//                          FPDF_GetSignatureObject().
//          buffer      -   The address of a buffer that receives the encoding.
//          length      -   The size, in bytes, of |buffer|.
// Return value:
//          Returns the number of bytes in the encoding name (including the
//          trailing NUL character) on success, 0 on error.
//
// The |buffer| is always encoded in 7-bit ASCII. If |length| is less than the
// returned length, or |buffer| is NULL, |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFSignatureObj_GetSubFilter(FPDF_SIGNATURE signature,
                              char* buffer,
                              unsigned long length);

// Experimental API.
// Function: FPDFSignatureObj_GetReason
//          Get the reason (comment) of the signature object.
// Parameters:
//          signature   -   Handle to the signature object. Returned by
//                          FPDF_GetSignatureObject().
//          buffer      -   The address of a buffer that receives the reason.
//          length      -   The size, in bytes, of |buffer|.
// Return value:
//          Returns the number of bytes in the reason on success, 0 on error.
//
// Regardless of the platform, the |buffer| is always in UTF-16LE encoding. The
// string is terminated by a UTF16 NUL character. If |length| is less than the
// returned length, or |buffer| is NULL, |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFSignatureObj_GetReason(FPDF_SIGNATURE signature,
                           void* buffer,
                           unsigned long length);

// Experimental API.
// Function: FPDFSignatureObj_GetTime
//          Get the time of signing of a signature object.
// Parameters:
//          signature   -   Handle to the signature object. Returned by
//                          FPDF_GetSignatureObject().
//          buffer      -   The address of a buffer that receives the time.
//          length      -   The size, in bytes, of |buffer|.
// Return value:
//          Returns the number of bytes in the encoding name (including the
//          trailing NUL character) on success, 0 on error.
//
// The |buffer| is always encoded in 7-bit ASCII. If |length| is less than the
// returned length, or |buffer| is NULL, |buffer| will not be modified.
//
// The format of time is expected to be D:YYYYMMDDHHMMSS+XX'YY', i.e. it's
// percision is seconds, with timezone information. This value should be used
// only when the time of signing is not available in the (PKCS#7 binary)
// signature.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFSignatureObj_GetTime(FPDF_SIGNATURE signature,
                         char* buffer,
                         unsigned long length);

// Experimental API.
// Function: FPDFSignatureObj_GetDocMDPPermission
//          Get the DocMDP permission of a signature object.
// Parameters:
//          signature   -   Handle to the signature object. Returned by
//                          FPDF_GetSignatureObject().
// Return value:
//          Returns the permission (1, 2 or 3) on success, 0 on error.
FPDF_EXPORT unsigned int FPDF_CALLCONV
FPDFSignatureObj_GetDocMDPPermission(FPDF_SIGNATURE signature);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_SIGNATURE_H_
