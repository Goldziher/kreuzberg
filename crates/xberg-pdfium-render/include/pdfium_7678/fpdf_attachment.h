// Copyright 2017 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef PUBLIC_FPDF_ATTACHMENT_H_
#define PUBLIC_FPDF_ATTACHMENT_H_

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

// Experimental API.
// Get the number of embedded files in |document|.
//
//   document - handle to a document.
//
// Returns the number of embedded files in |document|.
FPDF_EXPORT int FPDF_CALLCONV
FPDFDoc_GetAttachmentCount(FPDF_DOCUMENT document);

// Experimental API.
// Add an embedded file with |name| in |document|. If |name| is empty, or if
// |name| is the name of a existing embedded file in |document|, or if
// |document|'s embedded file name tree is too deep (i.e. |document| has too
// many embedded files already), then a new attachment will not be added.
//
//   document - handle to a document.
//   name     - name of the new attachment.
//
// Returns a handle to the new attachment object, or NULL on failure.
FPDF_EXPORT FPDF_ATTACHMENT FPDF_CALLCONV
FPDFDoc_AddAttachment(FPDF_DOCUMENT document, FPDF_WIDESTRING name);

// Experimental API.
// Get the embedded attachment at |index| in |document|. Note that the returned
// attachment handle is only valid while |document| is open.
//
//   document - handle to a document.
//   index    - the index of the requested embedded file.
//
// Returns the handle to the attachment object, or NULL on failure.
FPDF_EXPORT FPDF_ATTACHMENT FPDF_CALLCONV
FPDFDoc_GetAttachment(FPDF_DOCUMENT document, int index);

// Experimental API.
// Delete the embedded attachment at |index| in |document|. Note that this does
// not remove the attachment data from the PDF file; it simply removes the
// file's entry in the embedded files name tree so that it does not appear in
// the attachment list. This behavior may change in the future.
//
//   document - handle to a document.
//   index    - the index of the embedded file to be deleted.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFDoc_DeleteAttachment(FPDF_DOCUMENT document, int index);

// Experimental API.
// Get the name of the |attachment| file. |buffer| is only modified if |buflen|
// is longer than the length of the file name. On errors, |buffer| is unmodified
// and the returned length is 0.
//
//   attachment - handle to an attachment.
//   buffer     - buffer for holding the file name, encoded in UTF-16LE.
//   buflen     - length of the buffer in bytes.
//
// Returns the length of the file name in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAttachment_GetName(FPDF_ATTACHMENT attachment,
                       FPDF_WCHAR* buffer,
                       unsigned long buflen);

// Experimental API.
// Check if the params dictionary of |attachment| has |key| as a key.
//
//   attachment - handle to an attachment.
//   key        - the key to look for, encoded in UTF-8.
//
// Returns true if |key| exists.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAttachment_HasKey(FPDF_ATTACHMENT attachment, FPDF_BYTESTRING key);

// Experimental API.
// Get the type of the value corresponding to |key| in the params dictionary of
// the embedded |attachment|.
//
//   attachment - handle to an attachment.
//   key        - the key to look for, encoded in UTF-8.
//
// Returns the type of the dictionary value.
FPDF_EXPORT FPDF_OBJECT_TYPE FPDF_CALLCONV
FPDFAttachment_GetValueType(FPDF_ATTACHMENT attachment, FPDF_BYTESTRING key);

// Experimental API.
// Set the string value corresponding to |key| in the params dictionary of the
// embedded file |attachment|, overwriting the existing value if any. The value
// type should be FPDF_OBJECT_STRING after this function call succeeds.
//
//   attachment - handle to an attachment.
//   key        - the key to the dictionary entry, encoded in UTF-8.
//   value      - the string value to be set, encoded in UTF-16LE.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAttachment_SetStringValue(FPDF_ATTACHMENT attachment,
                              FPDF_BYTESTRING key,
                              FPDF_WIDESTRING value);

// Experimental API.
// Get the string value corresponding to |key| in the params dictionary of the
// embedded file |attachment|. |buffer| is only modified if |buflen| is longer
// than the length of the string value. Note that if |key| does not exist in the
// dictionary or if |key|'s corresponding value in the dictionary is not a
// string (i.e. the value is not of type FPDF_OBJECT_STRING or
// FPDF_OBJECT_NAME), then an empty string would be copied to |buffer| and the
// return value would be 2. On other errors, nothing would be added to |buffer|
// and the return value would be 0.
//
//   attachment - handle to an attachment.
//   key        - the key to the requested string value, encoded in UTF-8.
//   buffer     - buffer for holding the string value encoded in UTF-16LE.
//   buflen     - length of the buffer in bytes.
//
// Returns the length of the dictionary value string in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAttachment_GetStringValue(FPDF_ATTACHMENT attachment,
                              FPDF_BYTESTRING key,
                              FPDF_WCHAR* buffer,
                              unsigned long buflen);

// Experimental API.
// Set the file data of |attachment|, overwriting the existing file data if any.
// The creation date and checksum will be updated, while all other dictionary
// entries will be deleted. Note that only contents with |len| smaller than
// INT_MAX is supported.
//
//   attachment - handle to an attachment.
//   contents   - buffer holding the file data to write to |attachment|.
//   len        - length of file data in bytes.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAttachment_SetFile(FPDF_ATTACHMENT attachment,
                       FPDF_DOCUMENT document,
                       const void* contents,
                       unsigned long len);

// Experimental API.
// Get the file data of |attachment|.
// When the attachment file data is readable, true is returned, and |out_buflen|
// is updated to indicate the file data size. |buffer| is only modified if
// |buflen| is non-null and long enough to contain the entire file data. Callers
// must check both the return value and the input |buflen| is no less than the
// returned |out_buflen| before using the data.
//
// Otherwise, when the attachment file data is unreadable or when |out_buflen|
// is null, false is returned and |buffer| and |out_buflen| remain unmodified.
//
//   attachment - handle to an attachment.
//   buffer     - buffer for holding the file data from |attachment|.
//   buflen     - length of the buffer in bytes.
//   out_buflen - pointer to the variable that will receive the minimum buffer
//                size to contain the file data of |attachment|.
//
// Returns true on success, false otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAttachment_GetFile(FPDF_ATTACHMENT attachment,
                       void* buffer,
                       unsigned long buflen,
                       unsigned long* out_buflen);

// Experimental API.
// Get the MIME type (Subtype) of the embedded file |attachment|. |buffer| is
// only modified if |buflen| is longer than the length of the MIME type string.
// If the Subtype is not found or if there is no file stream, an empty string
// would be copied to |buffer| and the return value would be 2. On other errors,
// nothing would be added to |buffer| and the return value would be 0.
//
//   attachment - handle to an attachment.
//   buffer     - buffer for holding the MIME type string encoded in UTF-16LE.
//   buflen     - length of the buffer in bytes.
//
// Returns the length of the MIME type string in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAttachment_GetSubtype(FPDF_ATTACHMENT attachment,
                          FPDF_WCHAR* buffer,
                          unsigned long buflen);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_ATTACHMENT_H_
