// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

// This is the main header file for embedders of PDFium. It provides APIs to
// initialize the library, load documents, and render pages, amongst other
// things.
//
// NOTE: None of the PDFium APIs are thread-safe. They expect to be called
// from a single thread. Barring that, embedders are required to ensure (via
// a mutex or similar) that only a single PDFium call can be made at a time.
//
// NOTE: External docs refer to this file as "fpdfview.h", so do not rename
// despite lack of consistency with other public files.

#ifndef PUBLIC_FPDFVIEW_H_
#define PUBLIC_FPDFVIEW_H_

// clang-format off

#include <stddef.h>

#if defined(_WIN32) && !defined(__WINDOWS__)
#include <windows.h>
#endif

#ifdef PDF_ENABLE_XFA
// PDF_USE_XFA is set in confirmation that this version of PDFium can support
// XFA forms as requested by the PDF_ENABLE_XFA setting.
#define PDF_USE_XFA
#endif  // PDF_ENABLE_XFA

// PDF object types
#define FPDF_OBJECT_UNKNOWN 0
#define FPDF_OBJECT_BOOLEAN 1
#define FPDF_OBJECT_NUMBER 2
#define FPDF_OBJECT_STRING 3
#define FPDF_OBJECT_NAME 4
#define FPDF_OBJECT_ARRAY 5
#define FPDF_OBJECT_DICTIONARY 6
#define FPDF_OBJECT_STREAM 7
#define FPDF_OBJECT_NULLOBJ 8
#define FPDF_OBJECT_REFERENCE 9

// PDF text rendering modes
typedef enum {
  FPDF_TEXTRENDERMODE_UNKNOWN = -1,
  FPDF_TEXTRENDERMODE_FILL = 0,
  FPDF_TEXTRENDERMODE_STROKE = 1,
  FPDF_TEXTRENDERMODE_FILL_STROKE = 2,
  FPDF_TEXTRENDERMODE_INVISIBLE = 3,
  FPDF_TEXTRENDERMODE_FILL_CLIP = 4,
  FPDF_TEXTRENDERMODE_STROKE_CLIP = 5,
  FPDF_TEXTRENDERMODE_FILL_STROKE_CLIP = 6,
  FPDF_TEXTRENDERMODE_CLIP = 7,
  FPDF_TEXTRENDERMODE_LAST = FPDF_TEXTRENDERMODE_CLIP,
} FPDF_TEXT_RENDERMODE;

// PDF types - use incomplete types (never completed) to force API type safety.
typedef struct fpdf_action_t__* FPDF_ACTION;
typedef struct fpdf_annotation_t__* FPDF_ANNOTATION;
typedef struct fpdf_attachment_t__* FPDF_ATTACHMENT;
typedef struct fpdf_avail_t__* FPDF_AVAIL;
typedef struct fpdf_bitmap_t__* FPDF_BITMAP;
typedef struct fpdf_bookmark_t__* FPDF_BOOKMARK;
typedef struct fpdf_clippath_t__* FPDF_CLIPPATH;
typedef struct fpdf_dest_t__* FPDF_DEST;
typedef struct fpdf_document_t__* FPDF_DOCUMENT;
typedef struct fpdf_font_t__* FPDF_FONT;
typedef struct fpdf_form_handle_t__* FPDF_FORMHANDLE;
typedef const struct fpdf_glyphpath_t__* FPDF_GLYPHPATH;
typedef struct fpdf_javascript_action_t* FPDF_JAVASCRIPT_ACTION;
typedef struct fpdf_link_t__* FPDF_LINK;
typedef struct fpdf_page_t__* FPDF_PAGE;
typedef struct fpdf_pagelink_t__* FPDF_PAGELINK;
typedef struct fpdf_pageobject_t__* FPDF_PAGEOBJECT;  // (text, path, etc.)
typedef struct fpdf_pageobjectmark_t__* FPDF_PAGEOBJECTMARK;
typedef const struct fpdf_pagerange_t__* FPDF_PAGERANGE;
typedef const struct fpdf_pathsegment_t* FPDF_PATHSEGMENT;
typedef struct fpdf_schhandle_t__* FPDF_SCHHANDLE;
typedef const struct fpdf_signature_t__* FPDF_SIGNATURE;
typedef void* FPDF_SKIA_CANVAS;  // Passed into Skia as an SkCanvas.
typedef struct fpdf_structelement_t__* FPDF_STRUCTELEMENT;
typedef const struct fpdf_structelement_attr_t__* FPDF_STRUCTELEMENT_ATTR;
typedef const struct fpdf_structelement_attr_value_t__*
FPDF_STRUCTELEMENT_ATTR_VALUE;
typedef struct fpdf_structtree_t__* FPDF_STRUCTTREE;
typedef struct fpdf_textpage_t__* FPDF_TEXTPAGE;
typedef struct fpdf_widget_t__* FPDF_WIDGET;
typedef struct fpdf_xobject_t__* FPDF_XOBJECT;

// Basic data types
typedef int FPDF_BOOL;
typedef int FPDF_RESULT;
typedef unsigned long FPDF_DWORD;
typedef float FS_FLOAT;

// Duplex types
typedef enum _FPDF_DUPLEXTYPE_ {
  DuplexUndefined = 0,
  Simplex,
  DuplexFlipShortEdge,
  DuplexFlipLongEdge
} FPDF_DUPLEXTYPE;

// String types
typedef unsigned short FPDF_WCHAR;

// The public PDFium API uses three types of strings: byte string, wide string
// (UTF-16LE encoded), and platform dependent string.

// Public PDFium API type for byte strings.
typedef const char* FPDF_BYTESTRING;

// The public PDFium API always uses UTF-16LE encoded wide strings, each
// character uses 2 bytes (except surrogation), with the low byte first.
typedef const FPDF_WCHAR* FPDF_WIDESTRING;

// Structure for persisting a string beyond the duration of a callback.
// Note: although represented as a char*, string may be interpreted as
// a UTF-16LE formated string. Used only by XFA callbacks.
typedef struct FPDF_BSTR_ {
  char* str;  // String buffer, manipulate only with FPDF_BStr_* methods.
  int len;    // Length of the string, in bytes.
} FPDF_BSTR;

// For Windows programmers: In most cases it's OK to treat FPDF_WIDESTRING as a
// Windows unicode string, however, special care needs to be taken if you
// expect to process Unicode larger than 0xffff.
//
// For Linux/Unix programmers: most compiler/library environments use 4 bytes
// for a Unicode character, and you have to convert between FPDF_WIDESTRING and
// system wide string by yourself.
typedef const char* FPDF_STRING;

// Matrix for transformation, in the form [a b c d e f], equivalent to:
// | a  b  0 |
// | c  d  0 |
// | e  f  1 |
//
// Translation is performed with [1 0 0 1 tx ty].
// Scaling is performed with [sx 0 0 sy 0 0].
// See PDF Reference 1.7, 4.2.2 Common Transformations for more.
typedef struct _FS_MATRIX_ {
  float a;
  float b;
  float c;
  float d;
  float e;
  float f;
} FS_MATRIX;

// Rectangle area(float) in device or page coordinate system.
typedef struct _FS_RECTF_ {
  // The x-coordinate of the left-top corner.
  float left;
  // The y-coordinate of the left-top corner.
  float top;
  // The x-coordinate of the right-bottom corner.
  float right;
  // The y-coordinate of the right-bottom corner.
  float bottom;
} * FS_LPRECTF, FS_RECTF;

// Const Pointer to FS_RECTF structure.
typedef const FS_RECTF* FS_LPCRECTF;

// Rectangle size. Coordinate system agnostic.
typedef struct FS_SIZEF_ {
  float width;
  float height;
} * FS_LPSIZEF, FS_SIZEF;

// Const Pointer to FS_SIZEF structure.
typedef const FS_SIZEF* FS_LPCSIZEF;

// 2D Point. Coordinate system agnostic.
typedef struct FS_POINTF_ {
  float x;
  float y;
} * FS_LPPOINTF, FS_POINTF;

// Const Pointer to FS_POINTF structure.
typedef const FS_POINTF* FS_LPCPOINTF;

typedef struct _FS_QUADPOINTSF {
  FS_FLOAT x1;
  FS_FLOAT y1;
  FS_FLOAT x2;
  FS_FLOAT y2;
  FS_FLOAT x3;
  FS_FLOAT y3;
  FS_FLOAT x4;
  FS_FLOAT y4;
} FS_QUADPOINTSF;

// Annotation enums.
typedef int FPDF_ANNOTATION_SUBTYPE;
typedef int FPDF_ANNOT_APPEARANCEMODE;

// Dictionary value types.
typedef int FPDF_OBJECT_TYPE;

#if defined(COMPONENT_BUILD)
// FPDF_EXPORT should be consistent with |export| in the pdfium_fuzzer
// template in testing/fuzzers/BUILD.gn.
#if defined(WIN32)
#if defined(FPDF_IMPLEMENTATION)
#define FPDF_EXPORT __declspec(dllexport)
#else
#define FPDF_EXPORT __declspec(dllimport)
#endif  // defined(FPDF_IMPLEMENTATION)
#else
#if defined(FPDF_IMPLEMENTATION)
#define FPDF_EXPORT __attribute__((visibility("default")))
#else
#define FPDF_EXPORT
#endif  // defined(FPDF_IMPLEMENTATION)
#endif  // defined(WIN32)
#else
#define FPDF_EXPORT
#endif  // defined(COMPONENT_BUILD)

#if defined(WIN32) && defined(FPDFSDK_EXPORTS)
#define FPDF_CALLCONV __stdcall
#else
#define FPDF_CALLCONV
#endif

// Exported Functions
#ifdef __cplusplus
extern "C" {
#endif

// PDF renderer types - Experimental.
// Selection of 2D graphics library to use for rendering to FPDF_BITMAPs.
typedef enum {
  // Anti-Grain Geometry - https://sourceforge.net/projects/agg/
  FPDF_RENDERERTYPE_AGG = 0,
  // Skia - https://skia.org/
  FPDF_RENDERERTYPE_SKIA = 1,
} FPDF_RENDERER_TYPE;

// Process-wide options for initializing the library.
typedef struct FPDF_LIBRARY_CONFIG_ {
  // Version number of the interface. Currently must be 2.
  // Support for version 1 will be deprecated in the future.
  int version;

  // Array of paths to scan in place of the defaults when using built-in
  // FXGE font loading code. The array is terminated by a NULL pointer.
  // The Array may be NULL itself to use the default paths. May be ignored
  // entirely depending upon the platform.
  const char** m_pUserFontPaths;

  // Version 2.

  // Pointer to the v8::Isolate to use, or NULL to force PDFium to create one.
  void* m_pIsolate;

  // The embedder data slot to use in the v8::Isolate to store PDFium's
  // per-isolate data. The value needs to be in the range
  // [0, |v8::Internals::kNumIsolateDataLots|). Note that 0 is fine for most
  // embedders.
  unsigned int m_v8EmbedderSlot;

  // Version 3 - Experimental.

  // Pointer to the V8::Platform to use.
  void* m_pPlatform;

  // Version 4 - Experimental.

  // Explicit specification of core renderer to use. |m_RendererType| must be
  // a valid value for |FPDF_LIBRARY_CONFIG| versions of this level or higher,
  // or else the initialization will fail with an immediate crash.
  // Note that use of a specified |FPDF_RENDERER_TYPE| value for which the
  // corresponding render library is not included in the build will similarly
  // fail with an immediate crash.
  FPDF_RENDERER_TYPE m_RendererType;
} FPDF_LIBRARY_CONFIG;

// Function: FPDF_InitLibraryWithConfig
//          Initialize the PDFium library and allocate global resources for it.
// Parameters:
//          config - configuration information as above.
// Return value:
//          None.
// Comments:
//          You have to call this function before you can call any PDF
//          processing functions.
FPDF_EXPORT void FPDF_CALLCONV
FPDF_InitLibraryWithConfig(const FPDF_LIBRARY_CONFIG* config);

// Function: FPDF_InitLibrary
//          Initialize the PDFium library (alternative form).
// Parameters:
//          None
// Return value:
//          None.
// Comments:
//          Convenience function to call FPDF_InitLibraryWithConfig() with a
//          default configuration for backwards compatibility purposes. New
//          code should call FPDF_InitLibraryWithConfig() instead. This will
//          be deprecated in the future.
FPDF_EXPORT void FPDF_CALLCONV FPDF_InitLibrary();

// Function: FPDF_DestroyLibrary
//          Release global resources allocated to the PDFium library by
//          FPDF_InitLibrary() or FPDF_InitLibraryWithConfig().
// Parameters:
//          None.
// Return value:
//          None.
// Comments:
//          After this function is called, you must not call any PDF
//          processing functions.
//
//          Calling this function does not automatically close other
//          objects. It is recommended to close other objects before
//          closing the library with this function.
FPDF_EXPORT void FPDF_CALLCONV FPDF_DestroyLibrary();

// Policy for accessing the local machine time.
#define FPDF_POLICY_MACHINETIME_ACCESS 0

// Function: FPDF_SetSandBoxPolicy
//          Set the policy for the sandbox environment.
// Parameters:
//          policy -   The specified policy for setting, for example:
//                     FPDF_POLICY_MACHINETIME_ACCESS.
//          enable -   True to enable, false to disable the policy.
// Return value:
//          None.
FPDF_EXPORT void FPDF_CALLCONV FPDF_SetSandBoxPolicy(FPDF_DWORD policy,
                                                     FPDF_BOOL enable);

#if defined(_WIN32)
// Experimental API.
// Function: FPDF_SetPrintMode
//          Set printing mode when printing on Windows.
// Parameters:
//          mode - FPDF_PRINTMODE_EMF to output EMF (default)
//                 FPDF_PRINTMODE_TEXTONLY to output text only (for charstream
//                 devices)
//                 FPDF_PRINTMODE_POSTSCRIPT2 to output level 2 PostScript into
//                 EMF as a series of GDI comments.
//                 FPDF_PRINTMODE_POSTSCRIPT3 to output level 3 PostScript into
//                 EMF as a series of GDI comments.
//                 FPDF_PRINTMODE_POSTSCRIPT2_PASSTHROUGH to output level 2
//                 PostScript via ExtEscape() in PASSTHROUGH mode.
//                 FPDF_PRINTMODE_POSTSCRIPT3_PASSTHROUGH to output level 3
//                 PostScript via ExtEscape() in PASSTHROUGH mode.
//                 FPDF_PRINTMODE_EMF_IMAGE_MASKS to output EMF, with more
//                 efficient processing of documents containing image masks.
//                 FPDF_PRINTMODE_POSTSCRIPT3_TYPE42 to output level 3
//                 PostScript with embedded Type 42 fonts, when applicable, into
//                 EMF as a series of GDI comments.
//                 FPDF_PRINTMODE_POSTSCRIPT3_TYPE42_PASSTHROUGH to output level
//                 3 PostScript with embedded Type 42 fonts, when applicable,
//                 via ExtEscape() in PASSTHROUGH mode.
// Return value:
//          True if successful, false if unsuccessful (typically invalid input).
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_SetPrintMode(int mode);
#endif  // defined(_WIN32)

// Function: FPDF_LoadDocument
//          Open and load a PDF document.
// Parameters:
//          file_path -  Path to the PDF file (including extension).
//          password  -  A string used as the password for the PDF file.
//                       If no password is needed, empty or NULL can be used.
//                       See comments below regarding the encoding.
// Return value:
//          A handle to the loaded document, or NULL on failure.
// Comments:
//          Loaded document can be closed by FPDF_CloseDocument().
//          If this function fails, you can use FPDF_GetLastError() to retrieve
//          the reason why it failed.
//
//          The encoding for |file_path| is UTF-8.
//
//          The encoding for |password| can be either UTF-8 or Latin-1. PDFs,
//          depending on the security handler revision, will only accept one or
//          the other encoding. If |password|'s encoding and the PDF's expected
//          encoding do not match, FPDF_LoadDocument() will automatically
//          convert |password| to the other encoding.
FPDF_EXPORT FPDF_DOCUMENT FPDF_CALLCONV
FPDF_LoadDocument(FPDF_STRING file_path, FPDF_BYTESTRING password);

// Function: FPDF_LoadMemDocument
//          Open and load a PDF document from memory.
// Parameters:
//          data_buf    -   Pointer to a buffer containing the PDF document.
//          size        -   Number of bytes in the PDF document.
//          password    -   A string used as the password for the PDF file.
//                          If no password is needed, empty or NULL can be used.
// Return value:
//          A handle to the loaded document, or NULL on failure.
// Comments:
//          The memory buffer must remain valid when the document is open.
//          The loaded document can be closed by FPDF_CloseDocument.
//          If this function fails, you can use FPDF_GetLastError() to retrieve
//          the reason why it failed.
//
//          See the comments for FPDF_LoadDocument() regarding the encoding for
//          |password|.
// Notes:
//          If PDFium is built with the XFA module, the application should call
//          FPDF_LoadXFA() function after the PDF document loaded to support XFA
//          fields defined in the fpdfformfill.h file.
FPDF_EXPORT FPDF_DOCUMENT FPDF_CALLCONV
FPDF_LoadMemDocument(const void* data_buf, int size, FPDF_BYTESTRING password);

// Experimental API.
// Function: FPDF_LoadMemDocument64
//          Open and load a PDF document from memory.
// Parameters:
//          data_buf    -   Pointer to a buffer containing the PDF document.
//          size        -   Number of bytes in the PDF document.
//          password    -   A string used as the password for the PDF file.
//                          If no password is needed, empty or NULL can be used.
// Return value:
//          A handle to the loaded document, or NULL on failure.
// Comments:
//          The memory buffer must remain valid when the document is open.
//          The loaded document can be closed by FPDF_CloseDocument.
//          If this function fails, you can use FPDF_GetLastError() to retrieve
//          the reason why it failed.
//
//          See the comments for FPDF_LoadDocument() regarding the encoding for
//          |password|.
// Notes:
//          If PDFium is built with the XFA module, the application should call
//          FPDF_LoadXFA() function after the PDF document loaded to support XFA
//          fields defined in the fpdfformfill.h file.
FPDF_EXPORT FPDF_DOCUMENT FPDF_CALLCONV
FPDF_LoadMemDocument64(const void* data_buf,
                       size_t size,
                       FPDF_BYTESTRING password);

// Structure for custom file access.
typedef struct {
  // File length, in bytes.
  unsigned long m_FileLen;

  // A function pointer for getting a block of data from a specific position.
  // Position is specified by byte offset from the beginning of the file.
  // The pointer to the buffer is never NULL and the size is never 0.
  // The position and size will never go out of range of the file length.
  // It may be possible for PDFium to call this function multiple times for
  // the same position.
  // Return value: should be non-zero if successful, zero for error.
  int (*m_GetBlock)(void* param,
                    unsigned long position,
                    unsigned char* pBuf,
                    unsigned long size);

  // A custom pointer for all implementation specific data.  This pointer will
  // be used as the first parameter to the m_GetBlock callback.
  void* m_Param;
} FPDF_FILEACCESS;

// Structure for file reading or writing (I/O).
//
// Note: This is a handler and should be implemented by callers,
// and is only used from XFA.
typedef struct FPDF_FILEHANDLER_ {
  // User-defined data.
  // Note: Callers can use this field to track controls.
  void* clientData;

  // Callback function to release the current file stream object.
  //
  // Parameters:
  //       clientData   -  Pointer to user-defined data.
  // Returns:
  //       None.
  void (*Release)(void* clientData);

  // Callback function to retrieve the current file stream size.
  //
  // Parameters:
  //       clientData   -  Pointer to user-defined data.
  // Returns:
  //       Size of file stream.
  FPDF_DWORD (*GetSize)(void* clientData);

  // Callback function to read data from the current file stream.
  //
  // Parameters:
  //       clientData   -  Pointer to user-defined data.
  //       offset       -  Offset position starts from the beginning of file
  //                       stream. This parameter indicates reading position.
  //       buffer       -  Memory buffer to store data which are read from
  //                       file stream. This parameter should not be NULL.
  //       size         -  Size of data which should be read from file stream,
  //                       in bytes. The buffer indicated by |buffer| must be
  //                       large enough to store specified data.
  // Returns:
  //       0 for success, other value for failure.
  FPDF_RESULT (*ReadBlock)(void* clientData,
                           FPDF_DWORD offset,
                           void* buffer,
                           FPDF_DWORD size);

  // Callback function to write data into the current file stream.
  //
  // Parameters:
  //       clientData   -  Pointer to user-defined data.
  //       offset       -  Offset position starts from the beginning of file
  //                       stream. This parameter indicates writing position.
  //       buffer       -  Memory buffer contains data which is written into
  //                       file stream. This parameter should not be NULL.
  //       size         -  Size of data which should be written into file
  //                       stream, in bytes.
  // Returns:
  //       0 for success, other value for failure.
  FPDF_RESULT (*WriteBlock)(void* clientData,
                            FPDF_DWORD offset,
                            const void* buffer,
                            FPDF_DWORD size);
  // Callback function to flush all internal accessing buffers.
  //
  // Parameters:
  //       clientData   -  Pointer to user-defined data.
  // Returns:
  //       0 for success, other value for failure.
  FPDF_RESULT (*Flush)(void* clientData);

  // Callback function to change file size.
  //
  // Description:
  //       This function is called under writing mode usually. Implementer
  //       can determine whether to realize it based on application requests.
  // Parameters:
  //       clientData   -  Pointer to user-defined data.
  //       size         -  New size of file stream, in bytes.
  // Returns:
  //       0 for success, other value for failure.
  FPDF_RESULT (*Truncate)(void* clientData, FPDF_DWORD size);
} FPDF_FILEHANDLER;

// Function: FPDF_LoadCustomDocument
//          Load PDF document from a custom access descriptor.
// Parameters:
//          pFileAccess -   A structure for accessing the file.
//          password    -   Optional password for decrypting the PDF file.
// Return value:
//          A handle to the loaded document, or NULL on failure.
// Comments:
//          The application must keep the file resources |pFileAccess| points to
//          valid until the returned FPDF_DOCUMENT is closed. |pFileAccess|
//          itself does not need to outlive the FPDF_DOCUMENT.
//
//          The loaded document can be closed with FPDF_CloseDocument().
//
//          See the comments for FPDF_LoadDocument() regarding the encoding for
//          |password|.
// Notes:
//          If PDFium is built with the XFA module, the application should call
//          FPDF_LoadXFA() function after the PDF document loaded to support XFA
//          fields defined in the fpdfformfill.h file.
FPDF_EXPORT FPDF_DOCUMENT FPDF_CALLCONV
FPDF_LoadCustomDocument(FPDF_FILEACCESS* pFileAccess, FPDF_BYTESTRING password);

// Function: FPDF_GetFileVersion
//          Get the file version of the given PDF document.
// Parameters:
//          doc         -   Handle to a document.
//          fileVersion -   The PDF file version. File version: 14 for 1.4, 15
//                          for 1.5, ...
// Return value:
//          True if succeeds, false otherwise.
// Comments:
//          If the document was created by FPDF_CreateNewDocument,
//          then this function will always fail.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_GetFileVersion(FPDF_DOCUMENT doc,
                                                        int* fileVersion);

#define FPDF_ERR_SUCCESS 0    // No error.
#define FPDF_ERR_UNKNOWN 1    // Unknown error.
#define FPDF_ERR_FILE 2       // File not found or could not be opened.
#define FPDF_ERR_FORMAT 3     // File not in PDF format or corrupted.
#define FPDF_ERR_PASSWORD 4   // Password required or incorrect password.
#define FPDF_ERR_SECURITY 5   // Unsupported security scheme.
#define FPDF_ERR_PAGE 6       // Page not found or content error.
#ifdef PDF_ENABLE_XFA
#define FPDF_ERR_XFALOAD 7    // Load XFA error.
#define FPDF_ERR_XFALAYOUT 8  // Layout XFA error.
#endif  // PDF_ENABLE_XFA

// Function: FPDF_GetLastError
//          Get last error code when a function fails.
// Parameters:
//          None.
// Return value:
//          A 32-bit integer indicating error code as defined above.
// Comments:
//          If the previous SDK call succeeded, the return value of this
//          function is not defined. This function only works in conjunction
//          with APIs that mention FPDF_GetLastError() in their documentation.
FPDF_EXPORT unsigned long FPDF_CALLCONV FPDF_GetLastError();

// Experimental API.
// Function: FPDF_DocumentHasValidCrossReferenceTable
//          Whether the document's cross reference table is valid or not.
// Parameters:
//          document    -   Handle to a document. Returned by FPDF_LoadDocument.
// Return value:
//          True if the PDF parser did not encounter problems parsing the cross
//          reference table. False if the parser could not parse the cross
//          reference table and the table had to be rebuild from other data
//          within the document.
// Comments:
//          The return value can change over time as the PDF parser evolves.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_DocumentHasValidCrossReferenceTable(FPDF_DOCUMENT document);

// Experimental API.
// Function: FPDF_GetTrailerEnds
//          Get the byte offsets of trailer ends.
// Parameters:
//          document    -   Handle to document. Returned by FPDF_LoadDocument().
//          buffer      -   The address of a buffer that receives the
//                          byte offsets.
//          length      -   The size, in ints, of |buffer|.
// Return value:
//          Returns the number of ints in the buffer on success, 0 on error.
//
// |buffer| is an array of integers that describes the exact byte offsets of the
// trailer ends in the document. If |length| is less than the returned length,
// or |document| or |buffer| is NULL, |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_GetTrailerEnds(FPDF_DOCUMENT document,
                    unsigned int* buffer,
                    unsigned long length);

// Function: FPDF_GetDocPermissions
//          Get file permission flags of the document.
// Parameters:
//          document    -   Handle to a document. Returned by FPDF_LoadDocument.
// Return value:
//          A 32-bit integer indicating permission flags. Please refer to the
//          PDF Reference for detailed descriptions. If the document is not
//          protected or was unlocked by the owner, 0xffffffff will be returned.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_GetDocPermissions(FPDF_DOCUMENT document);

// Function: FPDF_GetDocUserPermissions
//          Get user file permission flags of the document.
// Parameters:
//          document    -   Handle to a document. Returned by FPDF_LoadDocument.
// Return value:
//          A 32-bit integer indicating permission flags. Please refer to the
//          PDF Reference for detailed descriptions. If the document is not
//          protected, 0xffffffff will be returned. Always returns user
//          permissions, even if the document was unlocked by the owner.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_GetDocUserPermissions(FPDF_DOCUMENT document);

// Function: FPDF_GetSecurityHandlerRevision
//          Get the revision for the security handler.
// Parameters:
//          document    -   Handle to a document. Returned by FPDF_LoadDocument.
// Return value:
//          The security handler revision number. Please refer to the PDF
//          Reference for a detailed description. If the document is not
//          protected, -1 will be returned.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_GetSecurityHandlerRevision(FPDF_DOCUMENT document);

// Function: FPDF_GetPageCount
//          Get total number of pages in the document.
// Parameters:
//          document    -   Handle to document. Returned by FPDF_LoadDocument.
// Return value:
//          Total number of pages in the document.
FPDF_EXPORT int FPDF_CALLCONV FPDF_GetPageCount(FPDF_DOCUMENT document);

// Function: FPDF_LoadPage
//          Load a page inside the document.
// Parameters:
//          document    -   Handle to document. Returned by FPDF_LoadDocument
//          page_index  -   Index number of the page. 0 for the first page.
// Return value:
//          A handle to the loaded page, or NULL if page load fails.
// Comments:
//          The loaded page can be rendered to devices using FPDF_RenderPage.
//          The loaded page can be closed using FPDF_ClosePage.
FPDF_EXPORT FPDF_PAGE FPDF_CALLCONV FPDF_LoadPage(FPDF_DOCUMENT document,
                                                  int page_index);

// Experimental API
// Function: FPDF_GetPageWidthF
//          Get page width.
// Parameters:
//          page        -   Handle to the page. Returned by FPDF_LoadPage().
// Return value:
//          Page width (excluding non-displayable area) measured in points.
//          One point is 1/72 inch (around 0.3528 mm).
// Comments:
//          Changing the rotation of |page| affects the return value.
FPDF_EXPORT float FPDF_CALLCONV FPDF_GetPageWidthF(FPDF_PAGE page);

// Function: FPDF_GetPageWidth
//          Get page width.
// Parameters:
//          page        -   Handle to the page. Returned by FPDF_LoadPage.
// Return value:
//          Page width (excluding non-displayable area) measured in points.
//          One point is 1/72 inch (around 0.3528 mm).
// Note:
//          Prefer FPDF_GetPageWidthF() above. This will be deprecated in the
//          future.
// Comments:
//          Changing the rotation of |page| affects the return value.
FPDF_EXPORT double FPDF_CALLCONV FPDF_GetPageWidth(FPDF_PAGE page);

// Experimental API
// Function: FPDF_GetPageHeightF
//          Get page height.
// Parameters:
//          page        -   Handle to the page. Returned by FPDF_LoadPage().
// Return value:
//          Page height (excluding non-displayable area) measured in points.
//          One point is 1/72 inch (around 0.3528 mm)
// Comments:
//          Changing the rotation of |page| affects the return value.
FPDF_EXPORT float FPDF_CALLCONV FPDF_GetPageHeightF(FPDF_PAGE page);

// Function: FPDF_GetPageHeight
//          Get page height.
// Parameters:
//          page        -   Handle to the page. Returned by FPDF_LoadPage.
// Return value:
//          Page height (excluding non-displayable area) measured in points.
//          One point is 1/72 inch (around 0.3528 mm)
// Note:
//          Prefer FPDF_GetPageHeightF() above. This will be deprecated in the
//          future.
// Comments:
//          Changing the rotation of |page| affects the return value.
FPDF_EXPORT double FPDF_CALLCONV FPDF_GetPageHeight(FPDF_PAGE page);

// Experimental API.
// Function: FPDF_GetPageBoundingBox
//          Get the bounding box of the page. This is the intersection between
//          its media box and its crop box.
// Parameters:
//          page        -   Handle to the page. Returned by FPDF_LoadPage.
//          rect        -   Pointer to a rect to receive the page bounding box.
//                          On an error, |rect| won't be filled.
// Return value:
//          True for success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_GetPageBoundingBox(FPDF_PAGE page,
                                                            FS_RECTF* rect);

// Experimental API.
// Function: FPDF_GetPageSizeByIndexF
//          Get the size of the page at the given index.
// Parameters:
//          document    -   Handle to document. Returned by FPDF_LoadDocument().
//          page_index  -   Page index, zero for the first page.
//          size        -   Pointer to a FS_SIZEF to receive the page size.
//                          (in points).
// Return value:
//          Non-zero for success. 0 for error (document or page not found).
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_GetPageSizeByIndexF(FPDF_DOCUMENT document,
                         int page_index,
                         FS_SIZEF* size);

// Function: FPDF_GetPageSizeByIndex
//          Get the size of the page at the given index.
// Parameters:
//          document    -   Handle to document. Returned by FPDF_LoadDocument.
//          page_index  -   Page index, zero for the first page.
//          width       -   Pointer to a double to receive the page width
//                          (in points).
//          height      -   Pointer to a double to receive the page height
//                          (in points).
// Return value:
//          Non-zero for success. 0 for error (document or page not found).
// Note:
//          Prefer FPDF_GetPageSizeByIndexF() above. This will be deprecated in
//          the future.
FPDF_EXPORT int FPDF_CALLCONV FPDF_GetPageSizeByIndex(FPDF_DOCUMENT document,
                                                      int page_index,
                                                      double* width,
                                                      double* height);

// Page rendering flags. They can be combined with bit-wise OR.
//
// Set if annotations are to be rendered.
#define FPDF_ANNOT 0x01
// Set if using text rendering optimized for LCD display. This flag will only
// take effect if anti-aliasing is enabled for text.
#define FPDF_LCD_TEXT 0x02
// Don't use the native text output available on some platforms
#define FPDF_NO_NATIVETEXT 0x04
// Grayscale output.
#define FPDF_GRAYSCALE 0x08
// Obsolete, has no effect, retained for compatibility.
#define FPDF_DEBUG_INFO 0x80
// Obsolete, has no effect, retained for compatibility.
#define FPDF_NO_CATCH 0x100
// Limit image cache size.
#define FPDF_RENDER_LIMITEDIMAGECACHE 0x200
// Always use halftone for image stretching.
#define FPDF_RENDER_FORCEHALFTONE 0x400
// Render for printing.
#define FPDF_PRINTING 0x800
// Set to disable anti-aliasing on text. This flag will also disable LCD
// optimization for text rendering.
#define FPDF_RENDER_NO_SMOOTHTEXT 0x1000
// Set to disable anti-aliasing on images.
#define FPDF_RENDER_NO_SMOOTHIMAGE 0x2000
// Set to disable anti-aliasing on paths.
#define FPDF_RENDER_NO_SMOOTHPATH 0x4000
// Set whether to render in a reverse Byte order, this flag is only used when
// rendering to a bitmap.
#define FPDF_REVERSE_BYTE_ORDER 0x10
// Set whether fill paths need to be stroked. This flag is only used when
// FPDF_COLORSCHEME is passed in, since with a single fill color for paths the
// boundaries of adjacent fill paths are less visible.
#define FPDF_CONVERT_FILL_TO_STROKE 0x20

// Struct for color scheme.
// Each should be a 32-bit value specifying the color, in 8888 ARGB format.
typedef struct FPDF_COLORSCHEME_ {
  FPDF_DWORD path_fill_color;
  FPDF_DWORD path_stroke_color;
  FPDF_DWORD text_fill_color;
  FPDF_DWORD text_stroke_color;
} FPDF_COLORSCHEME;

#ifdef _WIN32
// Function: FPDF_RenderPage
//          Render contents of a page to a device (screen, bitmap, or printer).
//          This function is only supported on Windows.
// Parameters:
//          dc          -   Handle to the device context.
//          page        -   Handle to the page. Returned by FPDF_LoadPage.
//          start_x     -   Left pixel position of the display area in
//                          device coordinates.
//          start_y     -   Top pixel position of the display area in device
//                          coordinates.
//          size_x      -   Horizontal size (in pixels) for displaying the page.
//          size_y      -   Vertical size (in pixels) for displaying the page.
//          rotate      -   Page orientation:
//                            0 (normal)
//                            1 (rotated 90 degrees clockwise)
//                            2 (rotated 180 degrees)
//                            3 (rotated 90 degrees counter-clockwise)
//          flags       -   0 for normal display, or combination of flags
//                          defined above.
// Return value:
//          Returns true if the page is rendered successfully, false otherwise.

FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_RenderPage(HDC dc,
                                                    FPDF_PAGE page,
                                                    int start_x,
                                                    int start_y,
                                                    int size_x,
                                                    int size_y,
                                                    int rotate,
                                                    int flags);
#endif

// Function: FPDF_RenderPageBitmap
//          Render contents of a page to a device independent bitmap.
// Parameters:
//          bitmap      -   Handle to the device independent bitmap (as the
//                          output buffer). The bitmap handle can be created
//                          by FPDFBitmap_Create or retrieved from an image
//                          object by FPDFImageObj_GetBitmap.
//          page        -   Handle to the page. Returned by FPDF_LoadPage
//          start_x     -   Left pixel position of the display area in
//                          bitmap coordinates.
//          start_y     -   Top pixel position of the display area in bitmap
//                          coordinates.
//          size_x      -   Horizontal size (in pixels) for displaying the page.
//          size_y      -   Vertical size (in pixels) for displaying the page.
//          rotate      -   Page orientation:
//                            0 (normal)
//                            1 (rotated 90 degrees clockwise)
//                            2 (rotated 180 degrees)
//                            3 (rotated 90 degrees counter-clockwise)
//          flags       -   0 for normal display, or combination of the Page
//                          Rendering flags defined above. With the FPDF_ANNOT
//                          flag, it renders all annotations that do not require
//                          user-interaction, which are all annotations except
//                          widget and popup annotations.
// Return value:
//          None.
FPDF_EXPORT void FPDF_CALLCONV FPDF_RenderPageBitmap(FPDF_BITMAP bitmap,
                                                     FPDF_PAGE page,
                                                     int start_x,
                                                     int start_y,
                                                     int size_x,
                                                     int size_y,
                                                     int rotate,
                                                     int flags);

// Function: FPDF_RenderPageBitmapWithMatrix
//          Render contents of a page to a device independent bitmap.
// Parameters:
//          bitmap      -   Handle to the device independent bitmap (as the
//                          output buffer). The bitmap handle can be created
//                          by FPDFBitmap_Create or retrieved by
//                          FPDFImageObj_GetBitmap.
//          page        -   Handle to the page. Returned by FPDF_LoadPage.
//          matrix      -   The transform matrix, which must be invertible.
//                          See PDF Reference 1.7, 4.2.2 Common Transformations.
//          clipping    -   The rect to clip to in device coords.
//          flags       -   0 for normal display, or combination of the Page
//                          Rendering flags defined above. With the FPDF_ANNOT
//                          flag, it renders all annotations that do not require
//                          user-interaction, which are all annotations except
//                          widget and popup annotations.
// Return value:
//          None. Note that behavior is undefined if det of |matrix| is 0.
FPDF_EXPORT void FPDF_CALLCONV
FPDF_RenderPageBitmapWithMatrix(FPDF_BITMAP bitmap,
                                FPDF_PAGE page,
                                const FS_MATRIX* matrix,
                                const FS_RECTF* clipping,
                                int flags);

#if defined(PDF_USE_SKIA)
// Experimental API.
// Function: FPDF_RenderPageSkia
//          Render contents of a page to a Skia SkCanvas.
// Parameters:
//          canvas      -   SkCanvas to render to.
//          page        -   Handle to the page.
//          size_x      -   Horizontal size (in pixels) for displaying the page.
//          size_y      -   Vertical size (in pixels) for displaying the page.
// Return value:
//          None.
FPDF_EXPORT void FPDF_CALLCONV FPDF_RenderPageSkia(FPDF_SKIA_CANVAS canvas,
                                                   FPDF_PAGE page,
                                                   int size_x,
                                                   int size_y);
#endif

// Function: FPDF_ClosePage
//          Close a loaded PDF page.
// Parameters:
//          page        -   Handle to the loaded page.
// Return value:
//          None.
FPDF_EXPORT void FPDF_CALLCONV FPDF_ClosePage(FPDF_PAGE page);

// Function: FPDF_CloseDocument
//          Close a loaded PDF document.
// Parameters:
//          document    -   Handle to the loaded document.
// Return value:
//          None.
FPDF_EXPORT void FPDF_CALLCONV FPDF_CloseDocument(FPDF_DOCUMENT document);

// Function: FPDF_DeviceToPage
//          Convert the screen coordinates of a point to page coordinates.
// Parameters:
//          page        -   Handle to the page. Returned by FPDF_LoadPage.
//          start_x     -   Left pixel position of the display area in
//                          device coordinates.
//          start_y     -   Top pixel position of the display area in device
//                          coordinates.
//          size_x      -   Horizontal size (in pixels) for displaying the page.
//          size_y      -   Vertical size (in pixels) for displaying the page.
//          rotate      -   Page orientation:
//                            0 (normal)
//                            1 (rotated 90 degrees clockwise)
//                            2 (rotated 180 degrees)
//                            3 (rotated 90 degrees counter-clockwise)
//          device_x    -   X value in device coordinates to be converted.
//          device_y    -   Y value in device coordinates to be converted.
//          page_x      -   A pointer to a double receiving the converted X
//                          value in page coordinates.
//          page_y      -   A pointer to a double receiving the converted Y
//                          value in page coordinates.
// Return value:
//          Returns true if the conversion succeeds, and |page_x| and |page_y|
//          successfully receives the converted coordinates.
// Comments:
//          The page coordinate system has its origin at the left-bottom corner
//          of the page, with the X-axis on the bottom going to the right, and
//          the Y-axis on the left side going up.
//
//          NOTE: this coordinate system can be altered when you zoom, scroll,
//          or rotate a page, however, a point on the page should always have
//          the same coordinate values in the page coordinate system.
//
//          The device coordinate system is device dependent. For screen device,
//          its origin is at the left-top corner of the window. However this
//          origin can be altered by the Windows coordinate transformation
//          utilities.
//
//          You must make sure the start_x, start_y, size_x, size_y
//          and rotate parameters have exactly same values as you used in
//          the FPDF_RenderPage() function call.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_DeviceToPage(FPDF_PAGE page,
                                                      int start_x,
                                                      int start_y,
                                                      int size_x,
                                                      int size_y,
                                                      int rotate,
                                                      int device_x,
                                                      int device_y,
                                                      double* page_x,
                                                      double* page_y);

// Function: FPDF_PageToDevice
//          Convert the page coordinates of a point to screen coordinates.
// Parameters:
//          page        -   Handle to the page. Returned by FPDF_LoadPage.
//          start_x     -   Left pixel position of the display area in
//                          device coordinates.
//          start_y     -   Top pixel position of the display area in device
//                          coordinates.
//          size_x      -   Horizontal size (in pixels) for displaying the page.
//          size_y      -   Vertical size (in pixels) for displaying the page.
//          rotate      -   Page orientation:
//                            0 (normal)
//                            1 (rotated 90 degrees clockwise)
//                            2 (rotated 180 degrees)
//                            3 (rotated 90 degrees counter-clockwise)
//          page_x      -   X value in page coordinates.
//          page_y      -   Y value in page coordinate.
//          device_x    -   A pointer to an integer receiving the result X
//                          value in device coordinates.
//          device_y    -   A pointer to an integer receiving the result Y
//                          value in device coordinates.
// Return value:
//          Returns true if the conversion succeeds, and |device_x| and
//          |device_y| successfully receives the converted coordinates.
// Comments:
//          See comments for FPDF_DeviceToPage().
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_PageToDevice(FPDF_PAGE page,
                                                      int start_x,
                                                      int start_y,
                                                      int size_x,
                                                      int size_y,
                                                      int rotate,
                                                      double page_x,
                                                      double page_y,
                                                      int* device_x,
                                                      int* device_y);

// Function: FPDFBitmap_Create
//          Create a device independent bitmap (FXDIB).
// Parameters:
//          width       -   The number of pixels in width for the bitmap.
//                          Must be greater than 0.
//          height      -   The number of pixels in height for the bitmap.
//                          Must be greater than 0.
//          alpha       -   A flag indicating whether the alpha channel is used.
//                          Non-zero for using alpha, zero for not using.
// Return value:
//          The created bitmap handle, or NULL if a parameter error or out of
//          memory.
// Comments:
//          The bitmap always uses 4 bytes per pixel. The first byte is always
//          double word aligned.
//
//          The byte order is BGRx (the last byte unused if no alpha channel) or
//          BGRA.
//
//          The pixels in a horizontal line are stored side by side, with the
//          left most pixel stored first (with lower memory address).
//          Each line uses width * 4 bytes.
//
//          Lines are stored one after another, with the top most line stored
//          first. There is no gap between adjacent lines.
//
//          This function allocates enough memory for holding all pixels in the
//          bitmap, but it doesn't initialize the buffer. Applications can use
//          FPDFBitmap_FillRect() to fill the bitmap using any color. If the OS
//          allows it, this function can allocate up to 4 GB of memory.
FPDF_EXPORT FPDF_BITMAP FPDF_CALLCONV FPDFBitmap_Create(int width,
                                                        int height,
                                                        int alpha);

// More DIB formats
// Unknown or unsupported format.
// All of the colors are listed in order of LSB to MSB.
#define FPDFBitmap_Unknown 0
// Gray scale bitmap, one byte per pixel.
#define FPDFBitmap_Gray 1
// 3 bytes per pixel, byte order: blue, green, red.
#define FPDFBitmap_BGR 2
// 4 bytes per pixel, byte order: blue, green, red, unused.
#define FPDFBitmap_BGRx 3
// 4 bytes per pixel, byte order: blue, green, red, alpha.
// Pixel components are independent of alpha.
#define FPDFBitmap_BGRA 4
// 4 bytes per pixel, byte order: blue, green, red, alpha.
// Pixel components are premultiplied by alpha.
// Note that this is experimental and only supported when rendering with
// |FPDF_RENDERER_TYPE| is set to |FPDF_RENDERERTYPE_SKIA|.
#define FPDFBitmap_BGRA_Premul 5

// Function: FPDFBitmap_CreateEx
//          Create a device independent bitmap (FXDIB)
// Parameters:
//          width       -   The number of pixels in width for the bitmap.
//                          Must be greater than 0.
//          height      -   The number of pixels in height for the bitmap.
//                          Must be greater than 0.
//          format      -   A number indicating for bitmap format, as defined
//                          above.
//          first_scan  -   A pointer to the first byte of the first line if
//                          using an external buffer. If this parameter is NULL,
//                          then a new buffer will be created.
//          stride      -   Number of bytes for each scan line. The value must
//                          be 0 or greater. When the value is 0,
//                          FPDFBitmap_CreateEx() will automatically calculate
//                          the appropriate value using |width| and |format|.
//                          When using an external buffer, it is recommended for
//                          the caller to pass in the value.
//                          When not using an external buffer, it is recommended
//                          for the caller to pass in 0.
// Return value:
//          The bitmap handle, or NULL if parameter error or out of memory.
// Comments:
//          Similar to FPDFBitmap_Create function, but allows for more formats
//          and an external buffer is supported. The bitmap created by this
//          function can be used in any place that a FPDF_BITMAP handle is
//          required.
//
//          If an external buffer is used, then the caller should destroy the
//          buffer. FPDFBitmap_Destroy() will not destroy the buffer.
//
//          It is recommended to use FPDFBitmap_GetStride() to get the stride
//          value.
FPDF_EXPORT FPDF_BITMAP FPDF_CALLCONV FPDFBitmap_CreateEx(int width,
                                                          int height,
                                                          int format,
                                                          void* first_scan,
                                                          int stride);

// Function: FPDFBitmap_GetFormat
//          Get the format of the bitmap.
// Parameters:
//          bitmap      -   Handle to the bitmap. Returned by FPDFBitmap_Create
//                          or FPDFImageObj_GetBitmap.
// Return value:
//          The format of the bitmap.
// Comments:
//          Only formats supported by FPDFBitmap_CreateEx are supported by this
//          function; see the list of such formats above.
FPDF_EXPORT int FPDF_CALLCONV FPDFBitmap_GetFormat(FPDF_BITMAP bitmap);

// Function: FPDFBitmap_FillRect
//          Fill a rectangle in a bitmap.
// Parameters:
//          bitmap      -   The handle to the bitmap. Returned by
//                          FPDFBitmap_Create.
//          left        -   The left position. Starting from 0 at the
//                          left-most pixel.
//          top         -   The top position. Starting from 0 at the
//                          top-most line.
//          width       -   Width in pixels to be filled.
//          height      -   Height in pixels to be filled.
//          color       -   A 32-bit value specifing the color, in 8888 ARGB
//                          format.
// Return value:
//          Returns whether the operation succeeded or not.
// Comments:
//          This function sets the color and (optionally) alpha value in the
//          specified region of the bitmap.
//
//          NOTE: If the alpha channel is used, this function does NOT
//          composite the background with the source color, instead the
//          background will be replaced by the source color and the alpha.
//
//          If the alpha channel is not used, the alpha parameter is ignored.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFBitmap_FillRect(FPDF_BITMAP bitmap,
                                                        int left,
                                                        int top,
                                                        int width,
                                                        int height,
                                                        FPDF_DWORD color);

// Function: FPDFBitmap_GetBuffer
//          Get data buffer of a bitmap.
// Parameters:
//          bitmap      -   Handle to the bitmap. Returned by FPDFBitmap_Create
//                          or FPDFImageObj_GetBitmap.
// Return value:
//          The pointer to the first byte of the bitmap buffer.
// Comments:
//          The stride may be more than width * number of bytes per pixel
//
//          Applications can use this function to get the bitmap buffer pointer,
//          then manipulate any color and/or alpha values for any pixels in the
//          bitmap.
//
//          Use FPDFBitmap_GetFormat() to find out the format of the data.
FPDF_EXPORT void* FPDF_CALLCONV FPDFBitmap_GetBuffer(FPDF_BITMAP bitmap);

// Function: FPDFBitmap_GetWidth
//          Get width of a bitmap.
// Parameters:
//          bitmap      -   Handle to the bitmap. Returned by FPDFBitmap_Create
//                          or FPDFImageObj_GetBitmap.
// Return value:
//          The width of the bitmap in pixels.
FPDF_EXPORT int FPDF_CALLCONV FPDFBitmap_GetWidth(FPDF_BITMAP bitmap);

// Function: FPDFBitmap_GetHeight
//          Get height of a bitmap.
// Parameters:
//          bitmap      -   Handle to the bitmap. Returned by FPDFBitmap_Create
//                          or FPDFImageObj_GetBitmap.
// Return value:
//          The height of the bitmap in pixels.
FPDF_EXPORT int FPDF_CALLCONV FPDFBitmap_GetHeight(FPDF_BITMAP bitmap);

// Function: FPDFBitmap_GetStride
//          Get number of bytes for each line in the bitmap buffer.
// Parameters:
//          bitmap      -   Handle to the bitmap. Returned by FPDFBitmap_Create
//                          or FPDFImageObj_GetBitmap.
// Return value:
//          The number of bytes for each line in the bitmap buffer.
// Comments:
//          The stride may be more than width * number of bytes per pixel.
FPDF_EXPORT int FPDF_CALLCONV FPDFBitmap_GetStride(FPDF_BITMAP bitmap);

// Function: FPDFBitmap_Destroy
//          Destroy a bitmap and release all related buffers.
// Parameters:
//          bitmap      -   Handle to the bitmap. Returned by FPDFBitmap_Create
//                          or FPDFImageObj_GetBitmap.
// Return value:
//          None.
// Comments:
//          This function will not destroy any external buffers provided when
//          the bitmap was created.
FPDF_EXPORT void FPDF_CALLCONV FPDFBitmap_Destroy(FPDF_BITMAP bitmap);

// Function: FPDF_VIEWERREF_GetPrintScaling
//          Whether the PDF document prefers to be scaled or not.
// Parameters:
//          document    -   Handle to the loaded document.
// Return value:
//          None.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_VIEWERREF_GetPrintScaling(FPDF_DOCUMENT document);

// Function: FPDF_VIEWERREF_GetNumCopies
//          Returns the number of copies to be printed.
// Parameters:
//          document    -   Handle to the loaded document.
// Return value:
//          The number of copies to be printed.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_VIEWERREF_GetNumCopies(FPDF_DOCUMENT document);

// Function: FPDF_VIEWERREF_GetPrintPageRange
//          Page numbers to initialize print dialog box when file is printed.
// Parameters:
//          document    -   Handle to the loaded document.
// Return value:
//          The print page range to be used for printing.
FPDF_EXPORT FPDF_PAGERANGE FPDF_CALLCONV
FPDF_VIEWERREF_GetPrintPageRange(FPDF_DOCUMENT document);

// Experimental API.
// Function: FPDF_VIEWERREF_GetPrintPageRangeCount
//          Returns the number of elements in a FPDF_PAGERANGE.
// Parameters:
//          pagerange   -   Handle to the page range.
// Return value:
//          The number of elements in the page range. Returns 0 on error.
FPDF_EXPORT size_t FPDF_CALLCONV
FPDF_VIEWERREF_GetPrintPageRangeCount(FPDF_PAGERANGE pagerange);

// Experimental API.
// Function: FPDF_VIEWERREF_GetPrintPageRangeElement
//          Returns an element from a FPDF_PAGERANGE.
// Parameters:
//          pagerange   -   Handle to the page range.
//          index       -   Index of the element.
// Return value:
//          The value of the element in the page range at a given index.
//          Returns -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_VIEWERREF_GetPrintPageRangeElement(FPDF_PAGERANGE pagerange, size_t index);

// Function: FPDF_VIEWERREF_GetDuplex
//          Returns the paper handling option to be used when printing from
//          the print dialog.
// Parameters:
//          document    -   Handle to the loaded document.
// Return value:
//          The paper handling option to be used when printing.
FPDF_EXPORT FPDF_DUPLEXTYPE FPDF_CALLCONV
FPDF_VIEWERREF_GetDuplex(FPDF_DOCUMENT document);

// Function: FPDF_VIEWERREF_GetName
//          Gets the contents for a viewer ref, with a given key. The value must
//          be of type "name".
// Parameters:
//          document    -   Handle to the loaded document.
//          key         -   Name of the key in the viewer pref dictionary,
//                          encoded in UTF-8.
//          buffer      -   Caller-allocate buffer to receive the key, or NULL
//                      -   to query the required length.
//          length      -   Length of the buffer.
// Return value:
//          The number of bytes in the contents, including the NULL terminator.
//          Thus if the return value is 0, then that indicates an error, such
//          as when |document| is invalid. If |length| is less than the required
//          length, or |buffer| is NULL, |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_VIEWERREF_GetName(FPDF_DOCUMENT document,
                       FPDF_BYTESTRING key,
                       char* buffer,
                       unsigned long length);

// Function: FPDF_CountNamedDests
//          Get the count of named destinations in the PDF document.
// Parameters:
//          document    -   Handle to a document
// Return value:
//          The count of named destinations.
FPDF_EXPORT FPDF_DWORD FPDF_CALLCONV
FPDF_CountNamedDests(FPDF_DOCUMENT document);

// Function: FPDF_GetNamedDestByName
//          Get a the destination handle for the given name.
// Parameters:
//          document    -   Handle to the loaded document.
//          name        -   The name of a destination.
// Return value:
//          The handle to the destination.
FPDF_EXPORT FPDF_DEST FPDF_CALLCONV
FPDF_GetNamedDestByName(FPDF_DOCUMENT document, FPDF_BYTESTRING name);

// Function: FPDF_GetNamedDest
//          Get the named destination by index.
// Parameters:
//          document        -   Handle to a document
//          index           -   The index of a named destination.
//          buffer          -   The buffer to store the destination name,
//                              used as wchar_t*.
//          buflen [in/out] -   Size of the buffer in bytes on input,
//                              length of the result in bytes on output
//                              or -1 if the buffer is too small.
// Return value:
//          The destination handle for a given index, or NULL if there is no
//          named destination corresponding to |index|.
// Comments:
//          Call this function twice to get the name of the named destination:
//            1) First time pass in |buffer| as NULL and get buflen.
//            2) Second time pass in allocated |buffer| and buflen to retrieve
//               |buffer|, which should be used as wchar_t*.
//
//         If buflen is not sufficiently large, it will be set to -1 upon
//         return.
FPDF_EXPORT FPDF_DEST FPDF_CALLCONV FPDF_GetNamedDest(FPDF_DOCUMENT document,
                                                      int index,
                                                      void* buffer,
                                                      long* buflen);

// Experimental API.
// Function: FPDF_GetXFAPacketCount
//          Get the number of valid packets in the XFA entry.
// Parameters:
//          document - Handle to the document.
// Return value:
//          The number of valid packets, or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV FPDF_GetXFAPacketCount(FPDF_DOCUMENT document);

// Experimental API.
// Function: FPDF_GetXFAPacketName
//          Get the name of a packet in the XFA array.
// Parameters:
//          document - Handle to the document.
//          index    - Index number of the packet. 0 for the first packet.
//          buffer   - Buffer for holding the name of the XFA packet.
//          buflen   - Length of |buffer| in bytes.
// Return value:
//          The length of the packet name in bytes, or 0 on error.
//
// |document| must be valid and |index| must be in the range [0, N), where N is
// the value returned by FPDF_GetXFAPacketCount().
// |buffer| is only modified if it is non-NULL and |buflen| is greater than or
// equal to the length of the packet name. The packet name includes a
// terminating NUL character. |buffer| is unmodified on error.
FPDF_EXPORT unsigned long FPDF_CALLCONV FPDF_GetXFAPacketName(
    FPDF_DOCUMENT document,
    int index,
    void* buffer,
    unsigned long buflen);

// Experimental API.
// Function: FPDF_GetXFAPacketContent
//          Get the content of a packet in the XFA array.
// Parameters:
//          document   - Handle to the document.
//          index      - Index number of the packet. 0 for the first packet.
//          buffer     - Buffer for holding the content of the XFA packet.
//          buflen     - Length of |buffer| in bytes.
//          out_buflen - Pointer to the variable that will receive the minimum
//                       buffer size needed to contain the content of the XFA
//                       packet.
// Return value:
//          Whether the operation succeeded or not.
//
// |document| must be valid and |index| must be in the range [0, N), where N is
// the value returned by FPDF_GetXFAPacketCount(). |out_buflen| must not be
// NULL. When the aforementioned arguments are valid, the operation succeeds,
// and |out_buflen| receives the content size. |buffer| is only modified if
// |buffer| is non-null and long enough to contain the content. Callers must
// check both the return value and the input |buflen| is no less than the
// returned |out_buflen| before using the data in |buffer|.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDF_GetXFAPacketContent(
    FPDF_DOCUMENT document,
    int index,
    void* buffer,
    unsigned long buflen,
    unsigned long* out_buflen);

#ifdef PDF_ENABLE_V8
// Function: FPDF_GetRecommendedV8Flags
//          Returns a space-separated string of command line flags that are
//          recommended to be passed into V8 via V8::SetFlagsFromString()
//          prior to initializing the PDFium library.
// Parameters:
//          None.
// Return value:
//          NUL-terminated string of the form "--flag1 --flag2".
//          The caller must not attempt to modify or free the result.
FPDF_EXPORT const char* FPDF_CALLCONV FPDF_GetRecommendedV8Flags();

// Experimental API.
// Function: FPDF_GetArrayBufferAllocatorSharedInstance()
//          Helper function for initializing V8 isolates that will
//          use PDFium's internal memory management.
// Parameters:
//          None.
// Return Value:
//          Pointer to a suitable v8::ArrayBuffer::Allocator, returned
//          as void for C compatibility.
// Notes:
//          Use is optional, but allows external creation of isolates
//          matching the ones PDFium will make when none is provided
//          via |FPDF_LIBRARY_CONFIG::m_pIsolate|.
//
//          Can only be called when the library is in an uninitialized or
//          destroyed state.
FPDF_EXPORT void* FPDF_CALLCONV FPDF_GetArrayBufferAllocatorSharedInstance();
#endif  // PDF_ENABLE_V8

#ifdef PDF_ENABLE_XFA
// Function: FPDF_BStr_Init
//          Helper function to initialize a FPDF_BSTR.
FPDF_EXPORT FPDF_RESULT FPDF_CALLCONV FPDF_BStr_Init(FPDF_BSTR* bstr);

// Function: FPDF_BStr_Set
//          Helper function to copy string data into the FPDF_BSTR.
FPDF_EXPORT FPDF_RESULT FPDF_CALLCONV FPDF_BStr_Set(FPDF_BSTR* bstr,
                                                    const char* cstr,
                                                    int length);

// Function: FPDF_BStr_Clear
//          Helper function to clear a FPDF_BSTR.
FPDF_EXPORT FPDF_RESULT FPDF_CALLCONV FPDF_BStr_Clear(FPDF_BSTR* bstr);
#endif  // PDF_ENABLE_XFA

#ifdef __cplusplus
}
#endif

#endif  // PUBLIC_FPDFVIEW_H_
