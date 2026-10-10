// Copyright 2017 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#ifndef PUBLIC_FPDF_ANNOT_H_
#define PUBLIC_FPDF_ANNOT_H_

#include <stddef.h>

// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

// NOLINTNEXTLINE(build/include)
#include "fpdf_formfill.h"

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

#define FPDF_ANNOT_UNKNOWN 0
#define FPDF_ANNOT_TEXT 1
#define FPDF_ANNOT_LINK 2
#define FPDF_ANNOT_FREETEXT 3
#define FPDF_ANNOT_LINE 4
#define FPDF_ANNOT_SQUARE 5
#define FPDF_ANNOT_CIRCLE 6
#define FPDF_ANNOT_POLYGON 7
#define FPDF_ANNOT_POLYLINE 8
#define FPDF_ANNOT_HIGHLIGHT 9
#define FPDF_ANNOT_UNDERLINE 10
#define FPDF_ANNOT_SQUIGGLY 11
#define FPDF_ANNOT_STRIKEOUT 12
#define FPDF_ANNOT_STAMP 13
#define FPDF_ANNOT_CARET 14
#define FPDF_ANNOT_INK 15
#define FPDF_ANNOT_POPUP 16
#define FPDF_ANNOT_FILEATTACHMENT 17
#define FPDF_ANNOT_SOUND 18
#define FPDF_ANNOT_MOVIE 19
#define FPDF_ANNOT_WIDGET 20
#define FPDF_ANNOT_SCREEN 21
#define FPDF_ANNOT_PRINTERMARK 22
#define FPDF_ANNOT_TRAPNET 23
#define FPDF_ANNOT_WATERMARK 24
#define FPDF_ANNOT_THREED 25
#define FPDF_ANNOT_RICHMEDIA 26
#define FPDF_ANNOT_XFAWIDGET 27
#define FPDF_ANNOT_REDACT 28

// Refer to PDF Reference (6th edition) table 8.16 for all annotation flags.
#define FPDF_ANNOT_FLAG_NONE 0
#define FPDF_ANNOT_FLAG_INVISIBLE (1 << 0)
#define FPDF_ANNOT_FLAG_HIDDEN (1 << 1)
#define FPDF_ANNOT_FLAG_PRINT (1 << 2)
#define FPDF_ANNOT_FLAG_NOZOOM (1 << 3)
#define FPDF_ANNOT_FLAG_NOROTATE (1 << 4)
#define FPDF_ANNOT_FLAG_NOVIEW (1 << 5)
#define FPDF_ANNOT_FLAG_READONLY (1 << 6)
#define FPDF_ANNOT_FLAG_LOCKED (1 << 7)
#define FPDF_ANNOT_FLAG_TOGGLENOVIEW (1 << 8)

#define FPDF_ANNOT_APPEARANCEMODE_NORMAL 0
#define FPDF_ANNOT_APPEARANCEMODE_ROLLOVER 1
#define FPDF_ANNOT_APPEARANCEMODE_DOWN 2
#define FPDF_ANNOT_APPEARANCEMODE_COUNT 3

// Refer to PDF Reference version 1.7 table 8.70 for field flags common to all
// interactive form field types.
#define FPDF_FORMFLAG_NONE 0
#define FPDF_FORMFLAG_READONLY (1 << 0)
#define FPDF_FORMFLAG_REQUIRED (1 << 1)
#define FPDF_FORMFLAG_NOEXPORT (1 << 2)

// Refer to PDF Reference version 1.7 table 8.77 for field flags specific to
// interactive form text fields.
#define FPDF_FORMFLAG_TEXT_MULTILINE (1 << 12)
#define FPDF_FORMFLAG_TEXT_PASSWORD (1 << 13)

// Refer to PDF Reference version 1.7 table 8.79 for field flags specific to
// interactive form choice fields.
#define FPDF_FORMFLAG_CHOICE_COMBO (1 << 17)
#define FPDF_FORMFLAG_CHOICE_EDIT (1 << 18)
#define FPDF_FORMFLAG_CHOICE_MULTI_SELECT (1 << 21)

// Additional actions type of form field:
//   K, on key stroke, JavaScript action.
//   F, on format, JavaScript action.
//   V, on validate, JavaScript action.
//   C, on calculate, JavaScript action.
#define FPDF_ANNOT_AACTION_KEY_STROKE 12
#define FPDF_ANNOT_AACTION_FORMAT 13
#define FPDF_ANNOT_AACTION_VALIDATE 14
#define FPDF_ANNOT_AACTION_CALCULATE 15

typedef enum FPDFANNOT_COLORTYPE {
  FPDFANNOT_COLORTYPE_Color = 0,
  FPDFANNOT_COLORTYPE_InteriorColor
} FPDFANNOT_COLORTYPE;

// Experimental API.
// Check if an annotation subtype is currently supported for creation.
// Currently supported subtypes:
//    - circle
//    - fileattachment
//    - freetext
//    - highlight
//    - ink
//    - link
//    - popup
//    - square,
//    - squiggly
//    - stamp
//    - strikeout
//    - text
//    - underline
//
//   subtype   - the subtype to be checked.
//
// Returns true if this subtype supported.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_IsSupportedSubtype(FPDF_ANNOTATION_SUBTYPE subtype);

// Experimental API.
// Create an annotation in |page| of the subtype |subtype|. If the specified
// subtype is illegal or unsupported, then a new annotation will not be created.
// Must call FPDFPage_CloseAnnot() when the annotation returned by this
// function is no longer needed.
//
//   page      - handle to a page.
//   subtype   - the subtype of the new annotation.
//
// Returns a handle to the new annotation object, or NULL on failure.
FPDF_EXPORT FPDF_ANNOTATION FPDF_CALLCONV
FPDFPage_CreateAnnot(FPDF_PAGE page, FPDF_ANNOTATION_SUBTYPE subtype);

// Experimental API.
// Get the number of annotations in |page|.
//
//   page   - handle to a page.
//
// Returns the number of annotations in |page|.
FPDF_EXPORT int FPDF_CALLCONV FPDFPage_GetAnnotCount(FPDF_PAGE page);

// Experimental API.
// Get annotation in |page| at |index|. Must call FPDFPage_CloseAnnot() when the
// annotation returned by this function is no longer needed.
//
//   page  - handle to a page.
//   index - the index of the annotation.
//
// Returns a handle to the annotation object, or NULL on failure.
FPDF_EXPORT FPDF_ANNOTATION FPDF_CALLCONV FPDFPage_GetAnnot(FPDF_PAGE page,
                                                            int index);

// Experimental API.
// Get the index of |annot| in |page|. This is the opposite of
// FPDFPage_GetAnnot().
//
//   page  - handle to the page that the annotation is on.
//   annot - handle to an annotation.
//
// Returns the index of |annot|, or -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV FPDFPage_GetAnnotIndex(FPDF_PAGE page,
                                                     FPDF_ANNOTATION annot);

// Experimental API.
// Close an annotation. Must be called when the annotation returned by
// FPDFPage_CreateAnnot() or FPDFPage_GetAnnot() is no longer needed. This
// function does not remove the annotation from the document.
//
//   annot  - handle to an annotation.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_CloseAnnot(FPDF_ANNOTATION annot);

// Experimental API.
// Remove the annotation in |page| at |index|.
//
//   page  - handle to a page.
//   index - the index of the annotation.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPage_RemoveAnnot(FPDF_PAGE page,
                                                         int index);

// Experimental API.
// Get the subtype of an annotation.
//
//   annot  - handle to an annotation.
//
// Returns the annotation subtype.
FPDF_EXPORT FPDF_ANNOTATION_SUBTYPE FPDF_CALLCONV
FPDFAnnot_GetSubtype(FPDF_ANNOTATION annot);

// Experimental API.
// Check if an annotation subtype is currently supported for object extraction,
// update, and removal.
// Currently supported subtypes: ink and stamp.
//
//   subtype   - the subtype to be checked.
//
// Returns true if this subtype supported.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_IsObjectSupportedSubtype(FPDF_ANNOTATION_SUBTYPE subtype);

// Experimental API.
// Update |obj| in |annot|. |obj| must be in |annot| already and must have
// been retrieved by FPDFAnnot_GetObject(). Currently, only ink and stamp
// annotations are supported by this API. Also note that only path, image, and
// text objects have APIs for modification; see FPDFPath_*(), FPDFText_*(), and
// FPDFImageObj_*().
//
//   annot  - handle to an annotation.
//   obj    - handle to the object that |annot| needs to update.
//
// Return true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_UpdateObject(FPDF_ANNOTATION annot, FPDF_PAGEOBJECT obj);

// Experimental API.
// Add a new InkStroke, represented by an array of points, to the InkList of
// |annot|. The API creates an InkList if one doesn't already exist in |annot|.
// This API works only for ink annotations. Please refer to ISO 32000-1:2008
// spec, section 12.5.6.13.
//
//   annot       - handle to an annotation.
//   points      - pointer to a FS_POINTF array representing input points.
//   point_count - number of elements in |points| array. This should not exceed
//                 the maximum value that can be represented by an int32_t).
//
// Returns the 0-based index at which the new InkStroke is added in the InkList
// of the |annot|. Returns -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV FPDFAnnot_AddInkStroke(FPDF_ANNOTATION annot,
                                                     const FS_POINTF* points,
                                                     size_t point_count);

// Experimental API.
// Removes an InkList in |annot|.
// This API works only for ink annotations.
//
//   annot  - handle to an annotation.
//
// Return true on successful removal of /InkList entry from context of the
// non-null ink |annot|. Returns false on failure.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_RemoveInkList(FPDF_ANNOTATION annot);

// Experimental API.
// Add |obj| to |annot|. |obj| must have been created by
// FPDFPageObj_CreateNew{Path|Rect}() or FPDFPageObj_New{Text|Image}Obj(), and
// will be owned by |annot|. Note that an |obj| cannot belong to more than one
// |annot|. Currently, only ink and stamp annotations are supported by this API.
// Also note that only path, image, and text objects have APIs for creation.
//
//   annot  - handle to an annotation.
//   obj    - handle to the object that is to be added to |annot|.
//
// Return true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_AppendObject(FPDF_ANNOTATION annot, FPDF_PAGEOBJECT obj);

// Experimental API.
// Get the total number of objects in |annot|, including path objects, text
// objects, external objects, image objects, and shading objects.
//
//   annot  - handle to an annotation.
//
// Returns the number of objects in |annot|.
FPDF_EXPORT int FPDF_CALLCONV FPDFAnnot_GetObjectCount(FPDF_ANNOTATION annot);

// Experimental API.
// Get the object in |annot| at |index|.
//
//   annot  - handle to an annotation.
//   index  - the index of the object.
//
// Return a handle to the object, or NULL on failure.
FPDF_EXPORT FPDF_PAGEOBJECT FPDF_CALLCONV
FPDFAnnot_GetObject(FPDF_ANNOTATION annot, int index);

// Experimental API.
// Remove the object in |annot| at |index|.
//
//   annot  - handle to an annotation.
//   index  - the index of the object to be removed.
//
// Return true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_RemoveObject(FPDF_ANNOTATION annot, int index);

// Experimental API.
// Set the color of an annotation. Fails when called on annotations with
// appearance streams already defined; instead use
// FPDFPageObj_Set{Stroke|Fill}Color().
//
//   annot    - handle to an annotation.
//   type     - type of the color to be set.
//   R, G, B  - buffer to hold the RGB value of the color. Ranges from 0 to 255.
//   A        - buffer to hold the opacity. Ranges from 0 to 255.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_SetColor(FPDF_ANNOTATION annot,
                                                       FPDFANNOT_COLORTYPE type,
                                                       unsigned int R,
                                                       unsigned int G,
                                                       unsigned int B,
                                                       unsigned int A);

// Experimental API.
// Get the color of an annotation. If no color is specified, default to yellow
// for highlight annotation, black for all else. Fails when called on
// annotations with appearance streams already defined; instead use
// FPDFPageObj_Get{Stroke|Fill}Color().
//
//   annot    - handle to an annotation.
//   type     - type of the color requested.
//   R, G, B  - buffer to hold the RGB value of the color. Ranges from 0 to 255.
//   A        - buffer to hold the opacity. Ranges from 0 to 255.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_GetColor(FPDF_ANNOTATION annot,
                                                       FPDFANNOT_COLORTYPE type,
                                                       unsigned int* R,
                                                       unsigned int* G,
                                                       unsigned int* B,
                                                       unsigned int* A);

// Experimental API.
// Check if the annotation is of a type that has attachment points
// (i.e. quadpoints). Quadpoints are the vertices of the rectangle that
// encompasses the texts affected by the annotation. They provide the
// coordinates in the page where the annotation is attached. Only text markup
// annotations (i.e. highlight, strikeout, squiggly, and underline) and link
// annotations have quadpoints.
//
//   annot  - handle to an annotation.
//
// Returns true if the annotation is of a type that has quadpoints, false
// otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_HasAttachmentPoints(FPDF_ANNOTATION annot);

// Experimental API.
// Replace the attachment points (i.e. quadpoints) set of an annotation at
// |quad_index|. This index needs to be within the result of
// FPDFAnnot_CountAttachmentPoints().
// If the annotation's appearance stream is defined and this annotation is of a
// type with quadpoints, then update the bounding box too if the new quadpoints
// define a bigger one.
//
//   annot       - handle to an annotation.
//   quad_index  - index of the set of quadpoints.
//   quad_points - the quadpoints to be set.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_SetAttachmentPoints(FPDF_ANNOTATION annot,
                              size_t quad_index,
                              const FS_QUADPOINTSF* quad_points);

// Experimental API.
// Append to the list of attachment points (i.e. quadpoints) of an annotation.
// If the annotation's appearance stream is defined and this annotation is of a
// type with quadpoints, then update the bounding box too if the new quadpoints
// define a bigger one.
//
//   annot       - handle to an annotation.
//   quad_points - the quadpoints to be set.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_AppendAttachmentPoints(FPDF_ANNOTATION annot,
                                 const FS_QUADPOINTSF* quad_points);

// Experimental API.
// Get the number of sets of quadpoints of an annotation.
//
//   annot  - handle to an annotation.
//
// Returns the number of sets of quadpoints, or 0 on failure.
FPDF_EXPORT size_t FPDF_CALLCONV
FPDFAnnot_CountAttachmentPoints(FPDF_ANNOTATION annot);

// Experimental API.
// Get the attachment points (i.e. quadpoints) of an annotation.
//
//   annot       - handle to an annotation.
//   quad_index  - index of the set of quadpoints.
//   quad_points - receives the quadpoints; must not be NULL.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_GetAttachmentPoints(FPDF_ANNOTATION annot,
                              size_t quad_index,
                              FS_QUADPOINTSF* quad_points);

// Experimental API.
// Set the annotation rectangle defining the location of the annotation. If the
// annotation's appearance stream is defined and this annotation is of a type
// without quadpoints, then update the bounding box too if the new rectangle
// defines a bigger one.
//
//   annot  - handle to an annotation.
//   rect   - the annotation rectangle to be set.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_SetRect(FPDF_ANNOTATION annot,
                                                      const FS_RECTF* rect);

// Experimental API.
// Get the annotation rectangle defining the location of the annotation.
//
//   annot  - handle to an annotation.
//   rect   - receives the rectangle; must not be NULL.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_GetRect(FPDF_ANNOTATION annot,
                                                      FS_RECTF* rect);

// Experimental API.
// Get the vertices of a polygon or polyline annotation. |buffer| is an array of
// points of the annotation. If |length| is less than the returned length, or
// |annot| or |buffer| is NULL, |buffer| will not be modified.
//
//   annot  - handle to an annotation, as returned by e.g. FPDFPage_GetAnnot()
//   buffer - buffer for holding the points.
//   length - length of the buffer in points.
//
// Returns the number of points if the annotation is of type polygon or
// polyline, 0 otherwise.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetVertices(FPDF_ANNOTATION annot,
                      FS_POINTF* buffer,
                      unsigned long length);

// Experimental API.
// Get the number of paths in the ink list of an ink annotation.
//
//   annot  - handle to an annotation, as returned by e.g. FPDFPage_GetAnnot()
//
// Returns the number of paths in the ink list if the annotation is of type ink,
// 0 otherwise.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetInkListCount(FPDF_ANNOTATION annot);

// Experimental API.
// Get a path in the ink list of an ink annotation. |buffer| is an array of
// points of the path. If |length| is less than the returned length, or |annot|
// or |buffer| is NULL, |buffer| will not be modified.
//
//   annot  - handle to an annotation, as returned by e.g. FPDFPage_GetAnnot()
//   path_index - index of the path
//   buffer - buffer for holding the points.
//   length - length of the buffer in points.
//
// Returns the number of points of the path if the annotation is of type ink, 0
// otherwise.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetInkListPath(FPDF_ANNOTATION annot,
                         unsigned long path_index,
                         FS_POINTF* buffer,
                         unsigned long length);

// Experimental API.
// Get the starting and ending coordinates of a line annotation.
//
//   annot  - handle to an annotation, as returned by e.g. FPDFPage_GetAnnot()
//   start - starting point
//   end - ending point
//
// Returns true if the annotation is of type line, |start| and |end| are not
// NULL, false otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_GetLine(FPDF_ANNOTATION annot,
                                                      FS_POINTF* start,
                                                      FS_POINTF* end);

// Experimental API.
// Set the characteristics of the annotation's border (rounded rectangle).
//
//   annot              - handle to an annotation
//   horizontal_radius  - horizontal corner radius, in default user space units
//   vertical_radius    - vertical corner radius, in default user space units
//   border_width       - border width, in default user space units
//
// Returns true if setting the border for |annot| succeeds, false otherwise.
//
// If |annot| contains an appearance stream that overrides the border values,
// then the appearance stream will be removed on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_SetBorder(FPDF_ANNOTATION annot,
                                                        float horizontal_radius,
                                                        float vertical_radius,
                                                        float border_width);

// Experimental API.
// Get the characteristics of the annotation's border (rounded rectangle).
//
//   annot              - handle to an annotation
//   horizontal_radius  - horizontal corner radius, in default user space units
//   vertical_radius    - vertical corner radius, in default user space units
//   border_width       - border width, in default user space units
//
// Returns true if |horizontal_radius|, |vertical_radius| and |border_width| are
// not NULL, false otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_GetBorder(FPDF_ANNOTATION annot,
                    float* horizontal_radius,
                    float* vertical_radius,
                    float* border_width);

// Experimental API.
// Get the JavaScript of an event of the annotation's additional actions.
// |buffer| is only modified if |buflen| is large enough to hold the whole
// JavaScript string. If |buflen| is smaller, the total size of the JavaScript
// is still returned, but nothing is copied.  If there is no JavaScript for
// |event| in |annot|, an empty string is written to |buf| and 2 is returned,
// denoting the size of the null terminator in the buffer.  On other errors,
// nothing is written to |buffer| and 0 is returned.
//
//    hHandle     -   handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//    annot       -   handle to an interactive form annotation.
//    event       -   event type, one of the FPDF_ANNOT_AACTION_* values.
//    buffer      -   buffer for holding the value string, encoded in UTF-16LE.
//    buflen      -   length of the buffer in bytes.
//
// Returns the length of the string value in bytes, including the 2-byte
// null terminator.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetFormAdditionalActionJavaScript(FPDF_FORMHANDLE hHandle,
                                            FPDF_ANNOTATION annot,
                                            int event,
                                            FPDF_WCHAR* buffer,
                                            unsigned long buflen);

// Experimental API.
// Check if |annot|'s dictionary has |key| as a key.
//
//   annot  - handle to an annotation.
//   key    - the key to look for, encoded in UTF-8.
//
// Returns true if |key| exists.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_HasKey(FPDF_ANNOTATION annot,
                                                     FPDF_BYTESTRING key);

// Experimental API.
// Get the type of the value corresponding to |key| in |annot|'s dictionary.
//
//   annot  - handle to an annotation.
//   key    - the key to look for, encoded in UTF-8.
//
// Returns the type of the dictionary value.
FPDF_EXPORT FPDF_OBJECT_TYPE FPDF_CALLCONV
FPDFAnnot_GetValueType(FPDF_ANNOTATION annot, FPDF_BYTESTRING key);

// Experimental API.
// Set the string value corresponding to |key| in |annot|'s dictionary,
// overwriting the existing value if any. The value type would be
// FPDF_OBJECT_STRING after this function call succeeds.
//
//   annot  - handle to an annotation.
//   key    - the key to the dictionary entry to be set, encoded in UTF-8.
//   value  - the string value to be set, encoded in UTF-16LE.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_SetStringValue(FPDF_ANNOTATION annot,
                         FPDF_BYTESTRING key,
                         FPDF_WIDESTRING value);

// Experimental API.
// Get the string value corresponding to |key| in |annot|'s dictionary. |buffer|
// is only modified if |buflen| is longer than the length of contents. Note that
// if |key| does not exist in the dictionary or if |key|'s corresponding value
// in the dictionary is not a string (i.e. the value is not of type
// FPDF_OBJECT_STRING or FPDF_OBJECT_NAME), then an empty string would be copied
// to |buffer| and the return value would be 2. On other errors, nothing would
// be added to |buffer| and the return value would be 0.
//
//   annot  - handle to an annotation.
//   key    - the key to the requested dictionary entry, encoded in UTF-8.
//   buffer - buffer for holding the value string, encoded in UTF-16LE.
//   buflen - length of the buffer in bytes.
//
// Returns the length of the string value in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetStringValue(FPDF_ANNOTATION annot,
                         FPDF_BYTESTRING key,
                         FPDF_WCHAR* buffer,
                         unsigned long buflen);

// Experimental API.
// Get the float value corresponding to |key| in |annot|'s dictionary. Writes
// value to |value| and returns True if |key| exists in the dictionary and
// |key|'s corresponding value is a number (FPDF_OBJECT_NUMBER), False
// otherwise.
//
//   annot  - handle to an annotation.
//   key    - the key to the requested dictionary entry, encoded in UTF-8.
//   value  - receives the value, must not be NULL.
//
// Returns True if value found, False otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_GetNumberValue(FPDF_ANNOTATION annot,
                         FPDF_BYTESTRING key,
                         float* value);

// Experimental API.
// Set the AP (appearance string) in |annot|'s dictionary for a given
// |appearanceMode|.
//
//   annot          - handle to an annotation.
//   appearanceMode - the appearance mode (normal, rollover or down) for which
//                    to get the AP.
//   value          - the string value to be set, encoded in UTF-16LE. If
//                    nullptr is passed, the AP is cleared for that mode. If the
//                    mode is Normal, APs for all modes are cleared.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_SetAP(FPDF_ANNOTATION annot,
                FPDF_ANNOT_APPEARANCEMODE appearanceMode,
                FPDF_WIDESTRING value);

// Experimental API.
// Get the AP (appearance string) from |annot|'s dictionary for a given
// |appearanceMode|.
// |buffer| is only modified if |buflen| is large enough to hold the whole AP
// string. If |buflen| is smaller, the total size of the AP is still returned,
// but nothing is copied.
// If there is no appearance stream for |annot| in |appearanceMode|, an empty
// string is written to |buf| and 2 is returned.
// On other errors, nothing is written to |buffer| and 0 is returned.
//
//   annot          - handle to an annotation.
//   appearanceMode - the appearance mode (normal, rollover or down) for which
//                    to get the AP.
//   buffer         - buffer for holding the value string, encoded in UTF-16LE.
//   buflen         - length of the buffer in bytes.
//
// Returns the length of the string value in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetAP(FPDF_ANNOTATION annot,
                FPDF_ANNOT_APPEARANCEMODE appearanceMode,
                FPDF_WCHAR* buffer,
                unsigned long buflen);

// Experimental API.
// Get the annotation corresponding to |key| in |annot|'s dictionary. Common
// keys for linking annotations include "IRT" and "Popup". Must call
// FPDFPage_CloseAnnot() when the annotation returned by this function is no
// longer needed.
//
//   annot  - handle to an annotation.
//   key    - the key to the requested dictionary entry, encoded in UTF-8.
//
// Returns a handle to the linked annotation object, or NULL on failure.
FPDF_EXPORT FPDF_ANNOTATION FPDF_CALLCONV
FPDFAnnot_GetLinkedAnnot(FPDF_ANNOTATION annot, FPDF_BYTESTRING key);

// Experimental API.
// Get the annotation flags of |annot|.
//
//   annot    - handle to an annotation.
//
// Returns the annotation flags.
FPDF_EXPORT int FPDF_CALLCONV FPDFAnnot_GetFlags(FPDF_ANNOTATION annot);

// Experimental API.
// Set the |annot|'s flags to be of the value |flags|.
//
//   annot      - handle to an annotation.
//   flags      - the flag values to be set.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_SetFlags(FPDF_ANNOTATION annot,
                                                       int flags);

// Experimental API.
// Get the annotation flags of |annot|.
//
//    hHandle     -   handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//    annot       -   handle to an interactive form annotation.
//
// Returns the annotation flags specific to interactive forms.
FPDF_EXPORT int FPDF_CALLCONV
FPDFAnnot_GetFormFieldFlags(FPDF_FORMHANDLE handle,
                            FPDF_ANNOTATION annot);

// Experimental API.
// Sets the form field flags for an interactive form annotation.
//
//   handle       -   the handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//   annot        -   handle to an interactive form annotation.
//   flags        -   the form field flags to be set.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_SetFormFieldFlags(FPDF_FORMHANDLE handle,
                            FPDF_ANNOTATION annot,
                            int flags);

// Experimental API.
// Retrieves an interactive form annotation whose rectangle contains a given
// point on a page. Must call FPDFPage_CloseAnnot() when the annotation returned
// is no longer needed.
//
//
//    hHandle     -   handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//    page        -   handle to the page, returned by FPDF_LoadPage function.
//    point       -   position in PDF "user space".
//
// Returns the interactive form annotation whose rectangle contains the given
// coordinates on the page. If there is no such annotation, return NULL.
FPDF_EXPORT FPDF_ANNOTATION FPDF_CALLCONV
FPDFAnnot_GetFormFieldAtPoint(FPDF_FORMHANDLE hHandle,
                              FPDF_PAGE page,
                              const FS_POINTF* point);

// Experimental API.
// Gets the name of |annot|, which is an interactive form annotation.
// |buffer| is only modified if |buflen| is longer than the length of contents.
// In case of error, nothing will be added to |buffer| and the return value will
// be 0. Note that return value of empty string is 2 for "\0\0".
//
//    hHandle     -   handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//    annot       -   handle to an interactive form annotation.
//    buffer      -   buffer for holding the name string, encoded in UTF-16LE.
//    buflen      -   length of the buffer in bytes.
//
// Returns the length of the string value in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetFormFieldName(FPDF_FORMHANDLE hHandle,
                           FPDF_ANNOTATION annot,
                           FPDF_WCHAR* buffer,
                           unsigned long buflen);

// Experimental API.
// Gets the alternate name of |annot|, which is an interactive form annotation.
// |buffer| is only modified if |buflen| is longer than the length of contents.
// In case of error, nothing will be added to |buffer| and the return value will
// be 0. Note that return value of empty string is 2 for "\0\0".
//
//    hHandle     -   handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//    annot       -   handle to an interactive form annotation.
//    buffer      -   buffer for holding the alternate name string, encoded in
//                    UTF-16LE.
//    buflen      -   length of the buffer in bytes.
//
// Returns the length of the string value in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetFormFieldAlternateName(FPDF_FORMHANDLE hHandle,
                                    FPDF_ANNOTATION annot,
                                    FPDF_WCHAR* buffer,
                                    unsigned long buflen);

// Experimental API.
// Gets the form field type of |annot|, which is an interactive form annotation.
//
//    hHandle     -   handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//    annot       -   handle to an interactive form annotation.
//
// Returns the type of the form field (one of the FPDF_FORMFIELD_* values) on
// success. Returns -1 on error.
// See field types in fpdf_formfill.h.
FPDF_EXPORT int FPDF_CALLCONV
FPDFAnnot_GetFormFieldType(FPDF_FORMHANDLE hHandle, FPDF_ANNOTATION annot);

// Experimental API.
// Gets the value of |annot|, which is an interactive form annotation.
// |buffer| is only modified if |buflen| is longer than the length of contents.
// In case of error, nothing will be added to |buffer| and the return value will
// be 0. Note that return value of empty string is 2 for "\0\0".
//
//    hHandle     -   handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//    annot       -   handle to an interactive form annotation.
//    buffer      -   buffer for holding the value string, encoded in UTF-16LE.
//    buflen      -   length of the buffer in bytes.
//
// Returns the length of the string value in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetFormFieldValue(FPDF_FORMHANDLE hHandle,
                            FPDF_ANNOTATION annot,
                            FPDF_WCHAR* buffer,
                            unsigned long buflen);

// Experimental API.
// Get the number of options in the |annot|'s "Opt" dictionary. Intended for
// use with listbox and combobox widget annotations.
//
//   hHandle - handle to the form fill module, returned by
//             FPDFDOC_InitFormFillEnvironment.
//   annot   - handle to an annotation.
//
// Returns the number of options in "Opt" dictionary on success. Return value
// will be -1 if annotation does not have an "Opt" dictionary or other error.
FPDF_EXPORT int FPDF_CALLCONV FPDFAnnot_GetOptionCount(FPDF_FORMHANDLE hHandle,
                                                       FPDF_ANNOTATION annot);

// Experimental API.
// Get the string value for the label of the option at |index| in |annot|'s
// "Opt" dictionary. Intended for use with listbox and combobox widget
// annotations. |buffer| is only modified if |buflen| is longer than the length
// of contents. If index is out of range or in case of other error, nothing
// will be added to |buffer| and the return value will be 0. Note that
// return value of empty string is 2 for "\0\0".
//
//   hHandle - handle to the form fill module, returned by
//             FPDFDOC_InitFormFillEnvironment.
//   annot   - handle to an annotation.
//   index   - numeric index of the option in the "Opt" array
//   buffer  - buffer for holding the value string, encoded in UTF-16LE.
//   buflen  - length of the buffer in bytes.
//
// Returns the length of the string value in bytes.
// If |annot| does not have an "Opt" array, |index| is out of range or if any
// other error occurs, returns 0.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetOptionLabel(FPDF_FORMHANDLE hHandle,
                         FPDF_ANNOTATION annot,
                         int index,
                         FPDF_WCHAR* buffer,
                         unsigned long buflen);

// Experimental API.
// Determine whether or not the option at |index| in |annot|'s "Opt" dictionary
// is selected. Intended for use with listbox and combobox widget annotations.
//
//   handle  - handle to the form fill module, returned by
//             FPDFDOC_InitFormFillEnvironment.
//   annot   - handle to an annotation.
//   index   - numeric index of the option in the "Opt" array.
//
// Returns true if the option at |index| in |annot|'s "Opt" dictionary is
// selected, false otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_IsOptionSelected(FPDF_FORMHANDLE handle,
                           FPDF_ANNOTATION annot,
                           int index);

// Experimental API.
// Get the float value of the font size for an |annot| with variable text.
// If 0, the font is to be auto-sized: its size is computed as a function of
// the height of the annotation rectangle.
//
//   hHandle - handle to the form fill module, returned by
//             FPDFDOC_InitFormFillEnvironment.
//   annot   - handle to an annotation.
//   value   - Required. Float which will be set to font size on success.
//
// Returns true if the font size was set in |value|, false on error or if
// |value| not provided.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_GetFontSize(FPDF_FORMHANDLE hHandle,
                      FPDF_ANNOTATION annot,
                      float* value);

// Experimental API.
// Set the text color of an annotation.
//
//   handle   - handle to the form fill module, returned by
//              FPDFDOC_InitFormFillEnvironment.
//   annot    - handle to an annotation.
//   R        - the red component for the text color.
//   G        - the green component for the text color.
//   B        - the blue component for the text color.
//
// Returns true if successful.
//
// Currently supported subtypes: freetext.
// The range for the color components is 0 to 255.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_SetFontColor(FPDF_FORMHANDLE handle,
                       FPDF_ANNOTATION annot,
                       unsigned int R,
                       unsigned int G,
                       unsigned int B);

// Experimental API.
// Get the RGB value of the font color for an |annot| with variable text.
//
//   hHandle  - handle to the form fill module, returned by
//              FPDFDOC_InitFormFillEnvironment.
//   annot    - handle to an annotation.
//   R, G, B  - buffer to hold the RGB value of the color. Ranges from 0 to 255.
//
// Returns true if the font color was set, false on error or if the font
// color was not provided.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_GetFontColor(FPDF_FORMHANDLE hHandle,
                       FPDF_ANNOTATION annot,
                       unsigned int* R,
                       unsigned int* G,
                       unsigned int* B);

// Experimental API.
// Determine if |annot| is a form widget that is checked. Intended for use with
// checkbox and radio button widgets.
//
//   hHandle - handle to the form fill module, returned by
//             FPDFDOC_InitFormFillEnvironment.
//   annot   - handle to an annotation.
//
// Returns true if |annot| is a form widget and is checked, false otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_IsChecked(FPDF_FORMHANDLE hHandle,
                                                        FPDF_ANNOTATION annot);

// Experimental API.
// Set the list of focusable annotation subtypes. Annotations of subtype
// FPDF_ANNOT_WIDGET are by default focusable. New subtypes set using this API
// will override the existing subtypes.
//
//   hHandle  - handle to the form fill module, returned by
//              FPDFDOC_InitFormFillEnvironment.
//   subtypes - list of annotation subtype which can be tabbed over.
//   count    - total number of annotation subtype in list.
// Returns true if list of annotation subtype is set successfully, false
// otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_SetFocusableSubtypes(FPDF_FORMHANDLE hHandle,
                               const FPDF_ANNOTATION_SUBTYPE* subtypes,
                               size_t count);

// Experimental API.
// Get the count of focusable annotation subtypes as set by host
// for a |hHandle|.
//
//   hHandle  - handle to the form fill module, returned by
//              FPDFDOC_InitFormFillEnvironment.
// Returns the count of focusable annotation subtypes or -1 on error.
// Note : Annotations of type FPDF_ANNOT_WIDGET are by default focusable.
FPDF_EXPORT int FPDF_CALLCONV
FPDFAnnot_GetFocusableSubtypesCount(FPDF_FORMHANDLE hHandle);

// Experimental API.
// Get the list of focusable annotation subtype as set by host.
//
//   hHandle  - handle to the form fill module, returned by
//              FPDFDOC_InitFormFillEnvironment.
//   subtypes - receives the list of annotation subtype which can be tabbed
//              over. Caller must have allocated |subtypes| more than or
//              equal to the count obtained from
//              FPDFAnnot_GetFocusableSubtypesCount() API.
//   count    - size of |subtypes|.
// Returns true on success and set list of annotation subtype to |subtypes|,
// false otherwise.
// Note : Annotations of type FPDF_ANNOT_WIDGET are by default focusable.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFAnnot_GetFocusableSubtypes(FPDF_FORMHANDLE hHandle,
                               FPDF_ANNOTATION_SUBTYPE* subtypes,
                               size_t count);

// Experimental API.
// Gets FPDF_LINK object for |annot|. Intended to use for link annotations.
//
//   annot   - handle to an annotation.
//
// Returns FPDF_LINK from the FPDF_ANNOTATION and NULL on failure,
// if the input annot is NULL or input annot's subtype is not link.
FPDF_EXPORT FPDF_LINK FPDF_CALLCONV FPDFAnnot_GetLink(FPDF_ANNOTATION annot);

// Experimental API.
// Gets the count of annotations in the |annot|'s control group.
// A group of interactive form annotations is collectively called a form
// control group. Here, |annot|, an interactive form annotation, should be
// either a radio button or a checkbox.
//
//   hHandle - handle to the form fill module, returned by
//             FPDFDOC_InitFormFillEnvironment.
//   annot   - handle to an annotation.
//
// Returns number of controls in its control group or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDFAnnot_GetFormControlCount(FPDF_FORMHANDLE hHandle, FPDF_ANNOTATION annot);

// Experimental API.
// Gets the index of |annot| in |annot|'s control group.
// A group of interactive form annotations is collectively called a form
// control group. Here, |annot|, an interactive form annotation, should be
// either a radio button or a checkbox.
//
//   hHandle - handle to the form fill module, returned by
//             FPDFDOC_InitFormFillEnvironment.
//   annot   - handle to an annotation.
//
// Returns index of a given |annot| in its control group or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDFAnnot_GetFormControlIndex(FPDF_FORMHANDLE hHandle, FPDF_ANNOTATION annot);

// Experimental API.
// Gets the export value of |annot| which is an interactive form annotation.
// Intended for use with radio button and checkbox widget annotations.
// |buffer| is only modified if |buflen| is longer than the length of contents.
// In case of error, nothing will be added to |buffer| and the return value
// will be 0. Note that return value of empty string is 2 for "\0\0".
//
//    hHandle     -   handle to the form fill module, returned by
//                    FPDFDOC_InitFormFillEnvironment().
//    annot       -   handle to an interactive form annotation.
//    buffer      -   buffer for holding the value string, encoded in UTF-16LE.
//    buflen      -   length of the buffer in bytes.
//
// Returns the length of the string value in bytes.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFAnnot_GetFormFieldExportValue(FPDF_FORMHANDLE hHandle,
                                  FPDF_ANNOTATION annot,
                                  FPDF_WCHAR* buffer,
                                  unsigned long buflen);

// Experimental API.
// Add a URI action to |annot|, overwriting the existing action, if any.
//
//   annot  - handle to a link annotation.
//   uri    - the URI to be set, encoded in 7-bit ASCII.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFAnnot_SetURI(FPDF_ANNOTATION annot,
                                                     const char* uri);

// Experimental API.
// Get the attachment from |annot|.
//
//   annot - handle to a file annotation.
//
// Returns the handle to the attachment object, or NULL on failure.
FPDF_EXPORT FPDF_ATTACHMENT FPDF_CALLCONV
FPDFAnnot_GetFileAttachment(FPDF_ANNOTATION annot);

// Experimental API.
// Add an embedded file with |name| to |annot|.
//
//   annot    - handle to a file annotation.
//   name     - name of the new attachment.
//
// Returns a handle to the new attachment object, or NULL on failure.
FPDF_EXPORT FPDF_ATTACHMENT FPDF_CALLCONV
FPDFAnnot_AddFileAttachment(FPDF_ANNOTATION annot, FPDF_WIDESTRING name);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_ANNOT_H_
