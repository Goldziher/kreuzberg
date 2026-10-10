// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_TRANSFORMPAGE_H_
#define PUBLIC_FPDF_TRANSFORMPAGE_H_

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif

// Set "MediaBox" entry to the page dictionary.
//
// page   - Handle to a page.
// left   - The left of the rectangle.
// bottom - The bottom of the rectangle.
// right  - The right of the rectangle.
// top    - The top of the rectangle.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_SetMediaBox(FPDF_PAGE page,
                                                    float left,
                                                    float bottom,
                                                    float right,
                                                    float top);

// Set "CropBox" entry to the page dictionary.
//
// page   - Handle to a page.
// left   - The left of the rectangle.
// bottom - The bottom of the rectangle.
// right  - The right of the rectangle.
// top    - The top of the rectangle.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_SetCropBox(FPDF_PAGE page,
                                                   float left,
                                                   float bottom,
                                                   float right,
                                                   float top);

// Set "BleedBox" entry to the page dictionary.
//
// page   - Handle to a page.
// left   - The left of the rectangle.
// bottom - The bottom of the rectangle.
// right  - The right of the rectangle.
// top    - The top of the rectangle.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_SetBleedBox(FPDF_PAGE page,
                                                    float left,
                                                    float bottom,
                                                    float right,
                                                    float top);

// Set "TrimBox" entry to the page dictionary.
//
// page   - Handle to a page.
// left   - The left of the rectangle.
// bottom - The bottom of the rectangle.
// right  - The right of the rectangle.
// top    - The top of the rectangle.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_SetTrimBox(FPDF_PAGE page,
                                                   float left,
                                                   float bottom,
                                                   float right,
                                                   float top);

// Set "ArtBox" entry to the page dictionary.
//
// page   - Handle to a page.
// left   - The left of the rectangle.
// bottom - The bottom of the rectangle.
// right  - The right of the rectangle.
// top    - The top of the rectangle.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_SetArtBox(FPDF_PAGE page,
                                                  float left,
                                                  float bottom,
                                                  float right,
                                                  float top);

// Get "MediaBox" entry from the page dictionary.
//
// page   - Handle to a page.
// left   - Pointer to a float value receiving the left of the rectangle.
// bottom - Pointer to a float value receiving the bottom of the rectangle.
// right  - Pointer to a float value receiving the right of the rectangle.
// top    - Pointer to a float value receiving the top of the rectangle.
//
// On success, return true and write to the out parameters. Otherwise return
// false and leave the out parameters unmodified.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPage_GetMediaBox(FPDF_PAGE page,
                                                         float* left,
                                                         float* bottom,
                                                         float* right,
                                                         float* top);

// Get "CropBox" entry from the page dictionary.
//
// page   - Handle to a page.
// left   - Pointer to a float value receiving the left of the rectangle.
// bottom - Pointer to a float value receiving the bottom of the rectangle.
// right  - Pointer to a float value receiving the right of the rectangle.
// top    - Pointer to a float value receiving the top of the rectangle.
//
// On success, return true and write to the out parameters. Otherwise return
// false and leave the out parameters unmodified.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPage_GetCropBox(FPDF_PAGE page,
                                                        float* left,
                                                        float* bottom,
                                                        float* right,
                                                        float* top);

// Get "BleedBox" entry from the page dictionary.
//
// page   - Handle to a page.
// left   - Pointer to a float value receiving the left of the rectangle.
// bottom - Pointer to a float value receiving the bottom of the rectangle.
// right  - Pointer to a float value receiving the right of the rectangle.
// top    - Pointer to a float value receiving the top of the rectangle.
//
// On success, return true and write to the out parameters. Otherwise return
// false and leave the out parameters unmodified.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPage_GetBleedBox(FPDF_PAGE page,
                                                         float* left,
                                                         float* bottom,
                                                         float* right,
                                                         float* top);

// Get "TrimBox" entry from the page dictionary.
//
// page   - Handle to a page.
// left   - Pointer to a float value receiving the left of the rectangle.
// bottom - Pointer to a float value receiving the bottom of the rectangle.
// right  - Pointer to a float value receiving the right of the rectangle.
// top    - Pointer to a float value receiving the top of the rectangle.
//
// On success, return true and write to the out parameters. Otherwise return
// false and leave the out parameters unmodified.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPage_GetTrimBox(FPDF_PAGE page,
                                                        float* left,
                                                        float* bottom,
                                                        float* right,
                                                        float* top);

// Get "ArtBox" entry from the page dictionary.
//
// page   - Handle to a page.
// left   - Pointer to a float value receiving the left of the rectangle.
// bottom - Pointer to a float value receiving the bottom of the rectangle.
// right  - Pointer to a float value receiving the right of the rectangle.
// top    - Pointer to a float value receiving the top of the rectangle.
//
// On success, return true and write to the out parameters. Otherwise return
// false and leave the out parameters unmodified.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPage_GetArtBox(FPDF_PAGE page,
                                                       float* left,
                                                       float* bottom,
                                                       float* right,
                                                       float* top);

// Apply transforms to |page|.
//
// If |matrix| is provided it will be applied to transform the page.
// If |clipRect| is provided it will be used to clip the resulting page.
// If neither |matrix| or |clipRect| are provided this method returns |false|.
// Returns |true| if transforms are applied.
//
// This function will transform the whole page, and would take effect to all the
// objects in the page.
//
// page        - Page handle.
// matrix      - Transform matrix.
// clipRect    - Clipping rectangle.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPage_TransFormWithClip(FPDF_PAGE page,
                           const FS_MATRIX* matrix,
                           const FS_RECTF* clipRect);

// Transform (scale, rotate, shear, move) the clip path of page object.
// page_object - Handle to a page object. Returned by
// FPDFPageObj_NewImageObj().
//
// a  - The coefficient "a" of the matrix.
// b  - The coefficient "b" of the matrix.
// c  - The coefficient "c" of the matrix.
// d  - The coefficient "d" of the matrix.
// e  - The coefficient "e" of the matrix.
// f  - The coefficient "f" of the matrix.
FPDF_EXPORT void FPDF_CALLCONV
FPDFPageObj_TransformClipPath(FPDF_PAGEOBJECT page_object,
                              double a,
                              double b,
                              double c,
                              double d,
                              double e,
                              double f);

// Experimental API.
// Get the clip path of the page object.
//
//   page object - Handle to a page object. Returned by e.g.
//                 FPDFPage_GetObject().
//
// Returns the handle to the clip path, or NULL on failure. The caller does not
// take ownership of the returned FPDF_CLIPPATH. Instead, it remains valid until
// FPDF_ClosePage() is called for the page containing |page_object|.
FPDF_EXPORT FPDF_CLIPPATH FPDF_CALLCONV
FPDFPageObj_GetClipPath(FPDF_PAGEOBJECT page_object);

// Experimental API.
// Get number of paths inside |clip_path|.
//
//   clip_path - handle to a clip_path.
//
// Returns the number of objects in |clip_path| or -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV FPDFClipPath_CountPaths(FPDF_CLIPPATH clip_path);

// Experimental API.
// Get number of segments inside one path of |clip_path|.
//
//   clip_path  - handle to a clip_path.
//   path_index - index into the array of paths of the clip path.
//
// Returns the number of segments or -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV
FPDFClipPath_CountPathSegments(FPDF_CLIPPATH clip_path, int path_index);

// Experimental API.
// Get segment in one specific path of |clip_path| at index.
//
//   clip_path     - handle to a clip_path.
//   path_index    - the index of a path.
//   segment_index - the index of a segment.
//
// Returns the handle to the segment, or NULL on failure. The caller does not
// take ownership of the returned FPDF_PATHSEGMENT. Instead, it remains valid
// until FPDF_ClosePage() is called for the page containing |clip_path|.
FPDF_EXPORT FPDF_PATHSEGMENT FPDF_CALLCONV
FPDFClipPath_GetPathSegment(FPDF_CLIPPATH clip_path,
                            int path_index,
                            int segment_index);

// Create a new clip path, with a rectangle inserted.
//
// Caller takes ownership of the returned FPDF_CLIPPATH. It should be freed with
// FPDF_DestroyClipPath().
//
// left   - The left of the clip box.
// bottom - The bottom of the clip box.
// right  - The right of the clip box.
// top    - The top of the clip box.
FPDF_EXPORT FPDF_CLIPPATH FPDF_CALLCONV FPDF_CreateClipPath(float left,
                                                            float bottom,
                                                            float right,
                                                            float top);

// Destroy the clip path.
//
// clipPath - A handle to the clip path. It will be invalid after this call.
FPDF_EXPORT void FPDF_CALLCONV FPDF_DestroyClipPath(FPDF_CLIPPATH clipPath);

// Clip the page content, the page content that outside the clipping region
// become invisible.
//
// A clip path will be inserted before the page content stream or content array.
// In this way, the page content will be clipped by this clip path.
//
// page        - A page handle.
// clipPath    - A handle to the clip path. (Does not take ownership.)
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_InsertClipPath(FPDF_PAGE page,
                                                       FPDF_CLIPPATH clipPath);

#ifdef __cplusplus
}
#endif

#endif  // PUBLIC_FPDF_TRANSFORMPAGE_H_
