// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_EDIT_H_
#define PUBLIC_FPDF_EDIT_H_

#include <stdint.h>

// NOLINTNEXTLINE(build/include_directory)
#include "fpdfview.h"

#define FPDF_ARGB(a, r, g, b)                                      \
  ((uint32_t)(((uint32_t)(b)&0xff) | (((uint32_t)(g)&0xff) << 8) | \
              (((uint32_t)(r)&0xff) << 16) | (((uint32_t)(a)&0xff) << 24)))
#define FPDF_GetBValue(argb) ((uint8_t)(argb))
#define FPDF_GetGValue(argb) ((uint8_t)(((uint16_t)(argb)) >> 8))
#define FPDF_GetRValue(argb) ((uint8_t)((argb) >> 16))
#define FPDF_GetAValue(argb) ((uint8_t)((argb) >> 24))

// Refer to PDF Reference version 1.7 table 4.12 for all color space families.
#define FPDF_COLORSPACE_UNKNOWN 0
#define FPDF_COLORSPACE_DEVICEGRAY 1
#define FPDF_COLORSPACE_DEVICERGB 2
#define FPDF_COLORSPACE_DEVICECMYK 3
#define FPDF_COLORSPACE_CALGRAY 4
#define FPDF_COLORSPACE_CALRGB 5
#define FPDF_COLORSPACE_LAB 6
#define FPDF_COLORSPACE_ICCBASED 7
#define FPDF_COLORSPACE_SEPARATION 8
#define FPDF_COLORSPACE_DEVICEN 9
#define FPDF_COLORSPACE_INDEXED 10
#define FPDF_COLORSPACE_PATTERN 11

// The page object constants.
#define FPDF_PAGEOBJ_UNKNOWN 0
#define FPDF_PAGEOBJ_TEXT 1
#define FPDF_PAGEOBJ_PATH 2
#define FPDF_PAGEOBJ_IMAGE 3
#define FPDF_PAGEOBJ_SHADING 4
#define FPDF_PAGEOBJ_FORM 5

// The path segment constants.
#define FPDF_SEGMENT_UNKNOWN -1
#define FPDF_SEGMENT_LINETO 0
#define FPDF_SEGMENT_BEZIERTO 1
#define FPDF_SEGMENT_MOVETO 2

#define FPDF_FILLMODE_NONE 0
#define FPDF_FILLMODE_ALTERNATE 1
#define FPDF_FILLMODE_WINDING 2

#define FPDF_FONT_TYPE1 1
#define FPDF_FONT_TRUETYPE 2

#define FPDF_LINECAP_BUTT 0
#define FPDF_LINECAP_ROUND 1
#define FPDF_LINECAP_PROJECTING_SQUARE 2

#define FPDF_LINEJOIN_MITER 0
#define FPDF_LINEJOIN_ROUND 1
#define FPDF_LINEJOIN_BEVEL 2

// See FPDF_SetPrintMode() for descriptions.
#define FPDF_PRINTMODE_EMF 0
#define FPDF_PRINTMODE_TEXTONLY 1
#define FPDF_PRINTMODE_POSTSCRIPT2 2
#define FPDF_PRINTMODE_POSTSCRIPT3 3
#define FPDF_PRINTMODE_POSTSCRIPT2_PASSTHROUGH 4
#define FPDF_PRINTMODE_POSTSCRIPT3_PASSTHROUGH 5
#define FPDF_PRINTMODE_EMF_IMAGE_MASKS 6
#define FPDF_PRINTMODE_POSTSCRIPT3_TYPE42 7
#define FPDF_PRINTMODE_POSTSCRIPT3_TYPE42_PASSTHROUGH 8

typedef struct FPDF_IMAGEOBJ_METADATA {
  // The image width in pixels.
  unsigned int width;
  // The image height in pixels.
  unsigned int height;
  // The image's horizontal pixel-per-inch.
  float horizontal_dpi;
  // The image's vertical pixel-per-inch.
  float vertical_dpi;
  // The number of bits used to represent each pixel.
  unsigned int bits_per_pixel;
  // The image's colorspace. See above for the list of FPDF_COLORSPACE_*.
  int colorspace;
  // The image's marked content ID. Useful for pairing with associated alt-text.
  // A value of -1 indicates no ID.
  int marked_content_id;
} FPDF_IMAGEOBJ_METADATA;

#ifdef __cplusplus
extern "C" {
#endif  // __cplusplus

// Create a new PDF document.
//
// Returns a handle to a new document, or NULL on failure.
FPDF_EXPORT FPDF_DOCUMENT FPDF_CALLCONV FPDF_CreateNewDocument();

// Create a new PDF page.
//
//   document   - handle to document.
//   page_index - suggested 0-based index of the page to create. If it is larger
//                than document's current last index(L), the created page index
//                is the next available index -- L+1.
//   width      - the page width in points.
//   height     - the page height in points.
//
// Returns the handle to the new page or NULL on failure.
//
// The page should be closed with FPDF_ClosePage() when finished as
// with any other page in the document.
FPDF_EXPORT FPDF_PAGE FPDF_CALLCONV FPDFPage_New(FPDF_DOCUMENT document,
                                                 int page_index,
                                                 double width,
                                                 double height);

// Delete the page at |page_index|.
//
//   document   - handle to document.
//   page_index - the index of the page to delete.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_Delete(FPDF_DOCUMENT document,
                                               int page_index);

// Experimental API.
// Move the given pages to a new index position.
//
//  page_indices     - the ordered list of pages to move. No duplicates allowed.
//  page_indices_len - the number of elements in |page_indices|
//  dest_page_index  - the new index position to which the pages in
//                     |page_indices| are moved.
//
// Returns TRUE on success. If it returns FALSE, the document may be left in an
// indeterminate state.
//
// Example: The PDF document starts out with pages [A, B, C, D], with indices
// [0, 1, 2, 3].
//
// >  Move(doc, [3, 2], 2, 1); // returns true
// >  // The document has pages [A, D, C, B].
// >
// >  Move(doc, [0, 4, 3], 3, 1); // returns false
// >  // Returned false because index 4 is out of range.
// >
// >  Move(doc, [0, 3, 1], 3, 2); // returns false
// >  // Returned false because index 2 is out of range for 3 page indices.
// >
// >  Move(doc, [2, 2], 2, 0); // returns false
// >  // Returned false because [2, 2] contains duplicates.
//
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_MovePages(FPDF_DOCUMENT document,
               const int* page_indices,
               unsigned long page_indices_len,
               int dest_page_index);

// Get the rotation of |page|.
//
//   page - handle to a page
//
// Returns one of the following indicating the page rotation:
//   0 - No rotation.
//   1 - Rotated 90 degrees clockwise.
//   2 - Rotated 180 degrees clockwise.
//   3 - Rotated 270 degrees clockwise.
FPDF_EXPORT int FPDF_CALLCONV FPDFPage_GetRotation(FPDF_PAGE page);

// Set rotation for |page|.
//
//   page   - handle to a page.
//   rotate - the rotation value, one of:
//              0 - No rotation.
//              1 - Rotated 90 degrees clockwise.
//              2 - Rotated 180 degrees clockwise.
//              3 - Rotated 270 degrees clockwise.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_SetRotation(FPDF_PAGE page, int rotate);

// Insert |page_object| into |page|.
//
//   page        - handle to a page
//   page_object - handle to a page object. The |page_object| will be
//                 automatically freed.
FPDF_EXPORT void FPDF_CALLCONV
FPDFPage_InsertObject(FPDF_PAGE page, FPDF_PAGEOBJECT page_object);

// Insert |page_object| into |page| at the specified |index|.
//
//   page        - handle to a page
//   page_object - handle to a page object as previously obtained by
//                 FPDFPageObj_CreateNew{Path|Rect}() or
//                 FPDFPageObj_New{Text|Image}Obj(). Ownership of the object
//                 is transferred back to PDFium.
//   index       - the index position to insert the object at. If index equals
//                 the current object count, the object will be appended to the
//                 end. If index is greater than the object count, the function
//                 will fail and return false.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPage_InsertObjectAtIndex(FPDF_PAGE page,
                             FPDF_PAGEOBJECT page_object,
                             size_t index);

// Experimental API.
// Remove |page_object| from |page|.
//
//   page        - handle to a page
//   page_object - handle to a page object to be removed.
//
// Returns TRUE on success.
//
// Ownership is transferred to the caller. Call FPDFPageObj_Destroy() to free
// it.
// Note that when removing a |page_object| of type FPDF_PAGEOBJ_TEXT, all
// FPDF_TEXTPAGE handles for |page| are no longer valid.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPage_RemoveObject(FPDF_PAGE page, FPDF_PAGEOBJECT page_object);

// Get number of page objects inside |page|.
//
//   page - handle to a page.
//
// Returns the number of objects in |page|.
FPDF_EXPORT int FPDF_CALLCONV FPDFPage_CountObjects(FPDF_PAGE page);

// Get object in |page| at |index|.
//
//   page  - handle to a page.
//   index - the index of a page object.
//
// Returns the handle to the page object, or NULL on failed.
FPDF_EXPORT FPDF_PAGEOBJECT FPDF_CALLCONV FPDFPage_GetObject(FPDF_PAGE page,
                                                             int index);

// Checks if |page| contains transparency.
//
//   page - handle to a page.
//
// Returns TRUE if |page| contains transparency.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPage_HasTransparency(FPDF_PAGE page);

// Generate the content of |page|.
//
//   page - handle to a page.
//
// Returns TRUE on success.
//
// Before you save the page to a file, or reload the page, you must call
// |FPDFPage_GenerateContent| or any changes to |page| will be lost.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPage_GenerateContent(FPDF_PAGE page);

// Destroy |page_object| by releasing its resources. |page_object| must have
// been created by FPDFPageObj_CreateNew{Path|Rect}() or
// FPDFPageObj_New{Text|Image}Obj(). This function must be called on
// newly-created objects if they are not added to a page through
// FPDFPage_InsertObject() or to an annotation through FPDFAnnot_AppendObject().
//
//   page_object - handle to a page object.
FPDF_EXPORT void FPDF_CALLCONV FPDFPageObj_Destroy(FPDF_PAGEOBJECT page_object);

// Checks if |page_object| contains transparency.
//
//   page_object - handle to a page object.
//
// Returns TRUE if |page_object| contains transparency.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_HasTransparency(FPDF_PAGEOBJECT page_object);

// Get type of |page_object|.
//
//   page_object - handle to a page object.
//
// Returns one of the FPDF_PAGEOBJ_* values on success, FPDF_PAGEOBJ_UNKNOWN on
// error.
FPDF_EXPORT int FPDF_CALLCONV FPDFPageObj_GetType(FPDF_PAGEOBJECT page_object);

// Experimental API.
// Gets active state for |page_object| within page.
//
//   page_object - handle to a page object.
//   active      - pointer to variable that will receive if the page object is
//                 active. This is a required parameter. Not filled if FALSE
//                 is returned.
//
// For page objects where |active| is filled with FALSE, the |page_object| is
// treated as if it wasn't in the document even though it is still held
// internally.
//
// Returns TRUE if the operation succeeded, FALSE if it failed.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetIsActive(FPDF_PAGEOBJECT page_object, FPDF_BOOL* active);

// Experimental API.
// Sets if |page_object| is active within page.
//
//   page_object - handle to a page object.
//   active      - a boolean specifying if the object is active.
//
// Returns TRUE on success.
//
// Page objects all start in the active state by default, and remain in that
// state unless this function is called.
//
// When |active| is false, this makes the |page_object| be treated as if it
// wasn't in the document even though it is still held internally.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetIsActive(FPDF_PAGEOBJECT page_object, FPDF_BOOL active);

// Transform |page_object| by the given matrix.
//
//   page_object - handle to a page object.
//   a           - matrix value.
//   b           - matrix value.
//   c           - matrix value.
//   d           - matrix value.
//   e           - matrix value.
//   f           - matrix value.
//
// The matrix is composed as:
//   |a c e|
//   |b d f|
// and can be used to scale, rotate, shear and translate the |page_object|.
FPDF_EXPORT void FPDF_CALLCONV
FPDFPageObj_Transform(FPDF_PAGEOBJECT page_object,
                      double a,
                      double b,
                      double c,
                      double d,
                      double e,
                      double f);

// Experimental API.
// Transform |page_object| by the given matrix.
//
//   page_object - handle to a page object.
//   matrix      - the transform matrix.
//
// Returns TRUE on success.
//
// This can be used to scale, rotate, shear and translate the |page_object|.
// It is an improved version of FPDFPageObj_Transform() that does not do
// unnecessary double to float conversions, and only uses 1 parameter for the
// matrix. It also returns whether the operation succeeded or not.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_TransformF(FPDF_PAGEOBJECT page_object, const FS_MATRIX* matrix);

// Experimental API.
// Get the transform matrix of a page object.
//
//   page_object - handle to a page object.
//   matrix      - pointer to struct to receive the matrix value.
//
// The matrix is composed as:
//   |a c e|
//   |b d f|
// and used to scale, rotate, shear and translate the page object.
//
// For page objects outside form objects, the matrix values are relative to the
// page that contains it.
// For page objects inside form objects, the matrix values are relative to the
// form that contains it.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetMatrix(FPDF_PAGEOBJECT page_object, FS_MATRIX* matrix);

// Experimental API.
// Set the transform matrix of a page object.
//
//   page_object - handle to a page object.
//   matrix      - pointer to struct with the matrix value.
//
// The matrix is composed as:
//   |a c e|
//   |b d f|
// and can be used to scale, rotate, shear and translate the page object.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetMatrix(FPDF_PAGEOBJECT page_object, const FS_MATRIX* matrix);

// Transform all annotations in |page|.
//
//   page - handle to a page.
//   a    - matrix value.
//   b    - matrix value.
//   c    - matrix value.
//   d    - matrix value.
//   e    - matrix value.
//   f    - matrix value.
//
// The matrix is composed as:
//   |a c e|
//   |b d f|
// and can be used to scale, rotate, shear and translate the |page| annotations.
FPDF_EXPORT void FPDF_CALLCONV FPDFPage_TransformAnnots(FPDF_PAGE page,
                                                        double a,
                                                        double b,
                                                        double c,
                                                        double d,
                                                        double e,
                                                        double f);

// Create a new image object.
//
//   document - handle to a document.
//
// Returns a handle to a new image object.
FPDF_EXPORT FPDF_PAGEOBJECT FPDF_CALLCONV
FPDFPageObj_NewImageObj(FPDF_DOCUMENT document);

// Experimental API.
// Get the marked content ID for the object.
//
//   page_object - handle to a page object.
//
// Returns the page object's marked content ID, or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDFPageObj_GetMarkedContentID(FPDF_PAGEOBJECT page_object);

// Experimental API.
// Get number of content marks in |page_object|.
//
//   page_object - handle to a page object.
//
// Returns the number of content marks in |page_object|, or -1 in case of
// failure.
FPDF_EXPORT int FPDF_CALLCONV
FPDFPageObj_CountMarks(FPDF_PAGEOBJECT page_object);

// Experimental API.
// Get content mark in |page_object| at |index|.
//
//   page_object - handle to a page object.
//   index       - the index of a page object.
//
// Returns the handle to the content mark, or NULL on failure. The handle is
// still owned by the library, and it should not be freed directly. It becomes
// invalid if the page object is destroyed, either directly or indirectly by
// unloading the page.
FPDF_EXPORT FPDF_PAGEOBJECTMARK FPDF_CALLCONV
FPDFPageObj_GetMark(FPDF_PAGEOBJECT page_object, unsigned long index);

// Experimental API.
// Add a new content mark to a |page_object|.
//
//   page_object - handle to a page object.
//   name        - the name (tag) of the mark.
//
// Returns the handle to the content mark, or NULL on failure. The handle is
// still owned by the library, and it should not be freed directly. It becomes
// invalid if the page object is destroyed, either directly or indirectly by
// unloading the page.
FPDF_EXPORT FPDF_PAGEOBJECTMARK FPDF_CALLCONV
FPDFPageObj_AddMark(FPDF_PAGEOBJECT page_object, FPDF_BYTESTRING name);

// Experimental API.
// Removes a content |mark| from a |page_object|.
// The mark handle will be invalid after the removal.
//
//   page_object - handle to a page object.
//   mark        - handle to a content mark in that object to remove.
//
// Returns TRUE if the operation succeeded, FALSE if it failed.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_RemoveMark(FPDF_PAGEOBJECT page_object, FPDF_PAGEOBJECTMARK mark);

// Experimental API.
// Get the name of a content mark.
//
//   mark       - handle to a content mark.
//   buffer     - buffer for holding the returned name in UTF-16LE. This is only
//                modified if |buflen| is large enough to store the name.
//                Optional, pass null to just retrieve the size of the buffer
//                needed.
//   buflen     - length of the buffer in bytes.
//   out_buflen - pointer to variable that will receive the minimum buffer size
//                in bytes to contain the name. This is a required parameter.
//                Not filled if FALSE is returned.
//
// Returns TRUE if the operation succeeded, FALSE if it failed.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_GetName(FPDF_PAGEOBJECTMARK mark,
                        FPDF_WCHAR* buffer,
                        unsigned long buflen,
                        unsigned long* out_buflen);

// Experimental API.
// Get the number of key/value pair parameters in |mark|.
//
//   mark   - handle to a content mark.
//
// Returns the number of key/value pair parameters |mark|, or -1 in case of
// failure.
FPDF_EXPORT int FPDF_CALLCONV
FPDFPageObjMark_CountParams(FPDF_PAGEOBJECTMARK mark);

// Experimental API.
// Get the key of a property in a content mark.
//
//   mark       - handle to a content mark.
//   index      - index of the property.
//   buffer     - buffer for holding the returned key in UTF-16LE. This is only
//                modified if |buflen| is large enough to store the key.
//                Optional, pass null to just retrieve the size of the buffer
//                needed.
//   buflen     - length of the buffer in bytes.
//   out_buflen - pointer to variable that will receive the minimum buffer size
//                in bytes to contain the name. This is a required parameter.
//                Not filled if FALSE is returned.
//
// Returns TRUE if the operation was successful, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_GetParamKey(FPDF_PAGEOBJECTMARK mark,
                            unsigned long index,
                            FPDF_WCHAR* buffer,
                            unsigned long buflen,
                            unsigned long* out_buflen);

// Experimental API.
// Get the type of the value of a property in a content mark by key.
//
//   mark   - handle to a content mark.
//   key    - string key of the property.
//
// Returns the type of the value, or FPDF_OBJECT_UNKNOWN in case of failure.
FPDF_EXPORT FPDF_OBJECT_TYPE FPDF_CALLCONV
FPDFPageObjMark_GetParamValueType(FPDF_PAGEOBJECTMARK mark,
                                  FPDF_BYTESTRING key);

// Experimental API.
// Get the value of a number property in a content mark by key as int.
// FPDFPageObjMark_GetParamValueType() should have returned FPDF_OBJECT_NUMBER
// for this property.
//
//   mark      - handle to a content mark.
//   key       - string key of the property.
//   out_value - pointer to variable that will receive the value. Not filled if
//               false is returned.
//
// Returns TRUE if the key maps to a number value, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_GetParamIntValue(FPDF_PAGEOBJECTMARK mark,
                                 FPDF_BYTESTRING key,
                                 int* out_value);

// Experimental API.
// Get the value of a number property in a content mark by key as float.
// FPDFPageObjMark_GetParamValueType() should have returned FPDF_OBJECT_NUMBER
// for this property.
//
//   mark      - handle to a content mark.
//   key       - string key of the property.
//   out_value - pointer to variable that will receive the value. Not filled if
//               false is returned.
//
// Returns TRUE if the key maps to a number value, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_GetParamFloatValue(FPDF_PAGEOBJECTMARK mark,
                                   FPDF_BYTESTRING key,
                                   float* out_value);

// Experimental API.
// Get the value of a string property in a content mark by key.
//
//   mark       - handle to a content mark.
//   key        - string key of the property.
//   buffer     - buffer for holding the returned value in UTF-16LE. This is
//                only modified if |buflen| is large enough to store the value.
//                Optional, pass null to just retrieve the size of the buffer
//                needed.
//   buflen     - length of the buffer in bytes.
//   out_buflen - pointer to variable that will receive the minimum buffer size
//                in bytes to contain the name. This is a required parameter.
//                Not filled if FALSE is returned.
//
// Returns TRUE if the key maps to a string/blob value, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_GetParamStringValue(FPDF_PAGEOBJECTMARK mark,
                                    FPDF_BYTESTRING key,
                                    FPDF_WCHAR* buffer,
                                    unsigned long buflen,
                                    unsigned long* out_buflen);

// Experimental API.
// Get the value of a blob property in a content mark by key.
//
//   mark       - handle to a content mark.
//   key        - string key of the property.
//   buffer     - buffer for holding the returned value. This is only modified
//                if |buflen| is large enough to store the value.
//                Optional, pass null to just retrieve the size of the buffer
//                needed.
//   buflen     - length of the buffer in bytes.
//   out_buflen - pointer to variable that will receive the minimum buffer size
//                in bytes to contain the name. This is a required parameter.
//                Not filled if FALSE is returned.
//
// Returns TRUE if the key maps to a string/blob value, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_GetParamBlobValue(FPDF_PAGEOBJECTMARK mark,
                                  FPDF_BYTESTRING key,
                                  unsigned char* buffer,
                                  unsigned long buflen,
                                  unsigned long* out_buflen);

// Experimental API.
// Set the value of an int property in a content mark by key. If a parameter
// with key |key| exists, its value is set to |value|. Otherwise, it is added as
// a new parameter.
//
//   document    - handle to the document.
//   page_object - handle to the page object with the mark.
//   mark        - handle to a content mark.
//   key         - string key of the property.
//   value       - int value to set.
//
// Returns TRUE if the operation succeeded, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_SetIntParam(FPDF_DOCUMENT document,
                            FPDF_PAGEOBJECT page_object,
                            FPDF_PAGEOBJECTMARK mark,
                            FPDF_BYTESTRING key,
                            int value);

// Experimental API.
// Set the value of a float property in a content mark by key. If a parameter
// with key |key| exists, its value is set to |value|. Otherwise, it is added as
// a new parameter.
//
//   document    - handle to the document.
//   page_object - handle to the page object with the mark.
//   mark        - handle to a content mark.
//   key         - string key of the property.
//   value       - float value to set.
//
// Returns TRUE if the operation succeeded, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_SetFloatParam(FPDF_DOCUMENT document,
                              FPDF_PAGEOBJECT page_object,
                              FPDF_PAGEOBJECTMARK mark,
                              FPDF_BYTESTRING key,
                              float value);

// Experimental API.
// Set the value of a string property in a content mark by key. If a parameter
// with key |key| exists, its value is set to |value|. Otherwise, it is added as
// a new parameter.
//
//   document    - handle to the document.
//   page_object - handle to the page object with the mark.
//   mark        - handle to a content mark.
//   key         - string key of the property.
//   value       - string value to set.
//
// Returns TRUE if the operation succeeded, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_SetStringParam(FPDF_DOCUMENT document,
                               FPDF_PAGEOBJECT page_object,
                               FPDF_PAGEOBJECTMARK mark,
                               FPDF_BYTESTRING key,
                               FPDF_BYTESTRING value);

// Experimental API.
// Set the value of a blob property in a content mark by key. If a parameter
// with key |key| exists, its value is set to |value|. Otherwise, it is added as
// a new parameter.
//
//   document    - handle to the document.
//   page_object - handle to the page object with the mark.
//   mark        - handle to a content mark.
//   key         - string key of the property.
//   value       - pointer to blob value to set.
//   value_len   - size in bytes of |value|.
//
// Returns TRUE if the operation succeeded, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_SetBlobParam(FPDF_DOCUMENT document,
                             FPDF_PAGEOBJECT page_object,
                             FPDF_PAGEOBJECTMARK mark,
                             FPDF_BYTESTRING key,
                             const unsigned char* value,
                             unsigned long value_len);

// Experimental API.
// Removes a property from a content mark by key.
//
//   page_object - handle to the page object with the mark.
//   mark        - handle to a content mark.
//   key         - string key of the property.
//
// Returns TRUE if the operation succeeded, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObjMark_RemoveParam(FPDF_PAGEOBJECT page_object,
                            FPDF_PAGEOBJECTMARK mark,
                            FPDF_BYTESTRING key);

// Load an image from a JPEG image file and then set it into |image_object|.
//
//   pages        - pointer to the start of all loaded pages, may be NULL.
//   count        - number of |pages|, may be 0.
//   image_object - handle to an image object.
//   file_access  - file access handler which specifies the JPEG image file.
//
// Returns TRUE on success.
//
// The image object might already have an associated image, which is shared and
// cached by the loaded pages. In that case, we need to clear the cached image
// for all the loaded pages. Pass |pages| and page count (|count|) to this API
// to clear the image cache. If the image is not previously shared, or NULL is a
// valid |pages| value.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFImageObj_LoadJpegFile(FPDF_PAGE* pages,
                          int count,
                          FPDF_PAGEOBJECT image_object,
                          FPDF_FILEACCESS* file_access);

// Load an image from a JPEG image file and then set it into |image_object|.
//
//   pages        - pointer to the start of all loaded pages, may be NULL.
//   count        - number of |pages|, may be 0.
//   image_object - handle to an image object.
//   file_access  - file access handler which specifies the JPEG image file.
//
// Returns TRUE on success.
//
// The image object might already have an associated image, which is shared and
// cached by the loaded pages. In that case, we need to clear the cached image
// for all the loaded pages. Pass |pages| and page count (|count|) to this API
// to clear the image cache. If the image is not previously shared, or NULL is a
// valid |pages| value. This function loads the JPEG image inline, so the image
// content is copied to the file. This allows |file_access| and its associated
// data to be deleted after this function returns.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFImageObj_LoadJpegFileInline(FPDF_PAGE* pages,
                                int count,
                                FPDF_PAGEOBJECT image_object,
                                FPDF_FILEACCESS* file_access);

// TODO(thestig): Start deprecating this once FPDFPageObj_SetMatrix() is stable.
//
// Set the transform matrix of |image_object|.
//
//   image_object - handle to an image object.
//   a            - matrix value.
//   b            - matrix value.
//   c            - matrix value.
//   d            - matrix value.
//   e            - matrix value.
//   f            - matrix value.
//
// The matrix is composed as:
//   |a c e|
//   |b d f|
// and can be used to scale, rotate, shear and translate the |image_object|.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFImageObj_SetMatrix(FPDF_PAGEOBJECT image_object,
                       double a,
                       double b,
                       double c,
                       double d,
                       double e,
                       double f);

// Set |bitmap| to |image_object|.
//
//   pages        - pointer to the start of all loaded pages, may be NULL.
//   count        - number of |pages|, may be 0.
//   image_object - handle to an image object.
//   bitmap       - handle of the bitmap.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFImageObj_SetBitmap(FPDF_PAGE* pages,
                       int count,
                       FPDF_PAGEOBJECT image_object,
                       FPDF_BITMAP bitmap);

// Get a bitmap rasterization of |image_object|. FPDFImageObj_GetBitmap() only
// operates on |image_object| and does not take the associated image mask into
// account. It also ignores the matrix for |image_object|.
// The returned bitmap will be owned by the caller, and FPDFBitmap_Destroy()
// must be called on the returned bitmap when it is no longer needed.
//
//   image_object - handle to an image object.
//
// Returns the bitmap.
FPDF_EXPORT FPDF_BITMAP FPDF_CALLCONV
FPDFImageObj_GetBitmap(FPDF_PAGEOBJECT image_object);

// Experimental API.
// Get a bitmap rasterization of |image_object| that takes the image mask and
// image matrix into account. To render correctly, the caller must provide the
// |document| associated with |image_object|. If there is a |page| associated
// with |image_object|, the caller should provide that as well.
// The returned bitmap will be owned by the caller, and FPDFBitmap_Destroy()
// must be called on the returned bitmap when it is no longer needed.
//
//   document     - handle to a document associated with |image_object|.
//   page         - handle to an optional page associated with |image_object|.
//   image_object - handle to an image object.
//
// Returns the bitmap or NULL on failure.
FPDF_EXPORT FPDF_BITMAP FPDF_CALLCONV
FPDFImageObj_GetRenderedBitmap(FPDF_DOCUMENT document,
                               FPDF_PAGE page,
                               FPDF_PAGEOBJECT image_object);

// Get the decoded image data of |image_object|. The decoded data is the
// uncompressed image data, i.e. the raw image data after having all filters
// applied. |buffer| is only modified if |buflen| is longer than the length of
// the decoded image data.
//
//   image_object - handle to an image object.
//   buffer       - buffer for holding the decoded image data.
//   buflen       - length of the buffer in bytes.
//
// Returns the length of the decoded image data.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFImageObj_GetImageDataDecoded(FPDF_PAGEOBJECT image_object,
                                 void* buffer,
                                 unsigned long buflen);

// Get the raw image data of |image_object|. The raw data is the image data as
// stored in the PDF without applying any filters. |buffer| is only modified if
// |buflen| is longer than the length of the raw image data.
//
//   image_object - handle to an image object.
//   buffer       - buffer for holding the raw image data.
//   buflen       - length of the buffer in bytes.
//
// Returns the length of the raw image data.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFImageObj_GetImageDataRaw(FPDF_PAGEOBJECT image_object,
                             void* buffer,
                             unsigned long buflen);

// Get the number of filters (i.e. decoders) of the image in |image_object|.
//
//   image_object - handle to an image object.
//
// Returns the number of |image_object|'s filters.
FPDF_EXPORT int FPDF_CALLCONV
FPDFImageObj_GetImageFilterCount(FPDF_PAGEOBJECT image_object);

// Get the filter at |index| of |image_object|'s list of filters. Note that the
// filters need to be applied in order, i.e. the first filter should be applied
// first, then the second, etc. |buffer| is only modified if |buflen| is longer
// than the length of the filter string.
//
//   image_object - handle to an image object.
//   index        - the index of the filter requested.
//   buffer       - buffer for holding filter string, encoded in UTF-8.
//   buflen       - length of the buffer.
//
// Returns the length of the filter string.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFImageObj_GetImageFilter(FPDF_PAGEOBJECT image_object,
                            int index,
                            void* buffer,
                            unsigned long buflen);

// Get the image metadata of |image_object|, including dimension, DPI, bits per
// pixel, and colorspace. If the |image_object| is not an image object or if it
// does not have an image, then the return value will be false. Otherwise,
// failure to retrieve any specific parameter would result in its value being 0.
//
//   image_object - handle to an image object.
//   page         - handle to the page that |image_object| is on. Required for
//                  retrieving the image's bits per pixel and colorspace.
//   metadata     - receives the image metadata; must not be NULL.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFImageObj_GetImageMetadata(FPDF_PAGEOBJECT image_object,
                              FPDF_PAGE page,
                              FPDF_IMAGEOBJ_METADATA* metadata);

// Experimental API.
// Get the image size in pixels. Faster method to get only image size.
//
//   image_object - handle to an image object.
//   width        - receives the image width in pixels; must not be NULL.
//   height       - receives the image height in pixels; must not be NULL.
//
// Returns true if successful.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFImageObj_GetImagePixelSize(FPDF_PAGEOBJECT image_object,
                               unsigned int* width,
                               unsigned int* height);

// Experimental API.
// Get ICC profile decoded data of |image_object|. If the |image_object| is not
// an image object or if it does not have an image, then the return value will
// be false. It also returns false if the |image_object| has no ICC profile.
// |buffer| is only modified if ICC profile exists and |buflen| is longer than
// the length of the ICC profile decoded data.
//
//   image_object - handle to an image object; must not be NULL.
//   page         - handle to the page containing |image_object|; must not be
//                  NULL. Required for retrieving the image's colorspace.
//   buffer       - Buffer to receive ICC profile data; may be NULL if querying
//                  required size via |out_buflen|.
//   buflen       - Length of the buffer in bytes. Ignored if |buffer| is NULL.
//   out_buflen   - Pointer to receive the ICC profile data size in bytes; must
//                  not be NULL. Will be set if this API returns true.
//
// Returns true if |out_buflen| is not null and an ICC profile exists for the
// given |image_object|.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFImageObj_GetIccProfileDataDecoded(FPDF_PAGEOBJECT image_object,
                                      FPDF_PAGE page,
                                      uint8_t* buffer,
                                      size_t buflen,
                                      size_t* out_buflen);

// Create a new path object at an initial position.
//
//   x - initial horizontal position.
//   y - initial vertical position.
//
// Returns a handle to a new path object.
FPDF_EXPORT FPDF_PAGEOBJECT FPDF_CALLCONV FPDFPageObj_CreateNewPath(float x,
                                                                    float y);

// Create a closed path consisting of a rectangle.
//
//   x - horizontal position for the left boundary of the rectangle.
//   y - vertical position for the bottom boundary of the rectangle.
//   w - width of the rectangle.
//   h - height of the rectangle.
//
// Returns a handle to the new path object.
FPDF_EXPORT FPDF_PAGEOBJECT FPDF_CALLCONV FPDFPageObj_CreateNewRect(float x,
                                                                    float y,
                                                                    float w,
                                                                    float h);

// Get the bounding box of |page_object|.
//
// page_object  - handle to a page object.
// left         - pointer where the left coordinate will be stored
// bottom       - pointer where the bottom coordinate will be stored
// right        - pointer where the right coordinate will be stored
// top          - pointer where the top coordinate will be stored
//
// On success, returns TRUE and fills in the 4 coordinates.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetBounds(FPDF_PAGEOBJECT page_object,
                      float* left,
                      float* bottom,
                      float* right,
                      float* top);

// Experimental API.
// Get the quad points that bounds |page_object|.
//
// page_object  - handle to a page object.
// quad_points  - pointer where the quadrilateral points will be stored.
//
// On success, returns TRUE and fills in |quad_points|.
//
// Similar to FPDFPageObj_GetBounds(), this returns the bounds of a page
// object. When the object is rotated by a non-multiple of 90 degrees, this API
// returns a tighter bound that cannot be represented with just the 4 sides of
// a rectangle.
//
// Currently only works the following |page_object| types: FPDF_PAGEOBJ_TEXT and
// FPDF_PAGEOBJ_IMAGE.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetRotatedBounds(FPDF_PAGEOBJECT page_object,
                             FS_QUADPOINTSF* quad_points);

// Set the blend mode of |page_object|.
//
// page_object  - handle to a page object.
// blend_mode   - string containing the blend mode.
//
// Blend mode can be one of following: Color, ColorBurn, ColorDodge, Darken,
// Difference, Exclusion, HardLight, Hue, Lighten, Luminosity, Multiply, Normal,
// Overlay, Saturation, Screen, SoftLight
FPDF_EXPORT void FPDF_CALLCONV
FPDFPageObj_SetBlendMode(FPDF_PAGEOBJECT page_object,
                         FPDF_BYTESTRING blend_mode);

// Set the stroke RGBA of a page object. Range of values: 0 - 255.
//
// page_object  - the handle to the page object.
// R            - the red component for the object's stroke color.
// G            - the green component for the object's stroke color.
// B            - the blue component for the object's stroke color.
// A            - the stroke alpha for the object.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetStrokeColor(FPDF_PAGEOBJECT page_object,
                           unsigned int R,
                           unsigned int G,
                           unsigned int B,
                           unsigned int A);

// Get the stroke RGBA of a page object. Range of values: 0 - 255.
//
// page_object  - the handle to the page object.
// R            - the red component of the path stroke color.
// G            - the green component of the object's stroke color.
// B            - the blue component of the object's stroke color.
// A            - the stroke alpha of the object.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetStrokeColor(FPDF_PAGEOBJECT page_object,
                           unsigned int* R,
                           unsigned int* G,
                           unsigned int* B,
                           unsigned int* A);

// Set the stroke width of a page object.
//
// path   - the handle to the page object.
// width  - the width of the stroke.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetStrokeWidth(FPDF_PAGEOBJECT page_object, float width);

// Get the stroke width of a page object.
//
// path   - the handle to the page object.
// width  - the width of the stroke.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetStrokeWidth(FPDF_PAGEOBJECT page_object, float* width);

// Get the line join of |page_object|.
//
// page_object  - handle to a page object.
//
// Returns the line join, or -1 on failure.
// Line join can be one of following: FPDF_LINEJOIN_MITER, FPDF_LINEJOIN_ROUND,
// FPDF_LINEJOIN_BEVEL
FPDF_EXPORT int FPDF_CALLCONV
FPDFPageObj_GetLineJoin(FPDF_PAGEOBJECT page_object);

// Set the line join of |page_object|.
//
// page_object  - handle to a page object.
// line_join    - line join
//
// Line join can be one of following: FPDF_LINEJOIN_MITER, FPDF_LINEJOIN_ROUND,
// FPDF_LINEJOIN_BEVEL
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetLineJoin(FPDF_PAGEOBJECT page_object, int line_join);

// Get the line cap of |page_object|.
//
// page_object - handle to a page object.
//
// Returns the line cap, or -1 on failure.
// Line cap can be one of following: FPDF_LINECAP_BUTT, FPDF_LINECAP_ROUND,
// FPDF_LINECAP_PROJECTING_SQUARE
FPDF_EXPORT int FPDF_CALLCONV
FPDFPageObj_GetLineCap(FPDF_PAGEOBJECT page_object);

// Set the line cap of |page_object|.
//
// page_object - handle to a page object.
// line_cap    - line cap
//
// Line cap can be one of following: FPDF_LINECAP_BUTT, FPDF_LINECAP_ROUND,
// FPDF_LINECAP_PROJECTING_SQUARE
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetLineCap(FPDF_PAGEOBJECT page_object, int line_cap);

// Set the fill RGBA of a page object. Range of values: 0 - 255.
//
// page_object  - the handle to the page object.
// R            - the red component for the object's fill color.
// G            - the green component for the object's fill color.
// B            - the blue component for the object's fill color.
// A            - the fill alpha for the object.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetFillColor(FPDF_PAGEOBJECT page_object,
                         unsigned int R,
                         unsigned int G,
                         unsigned int B,
                         unsigned int A);

// Get the fill RGBA of a page object. Range of values: 0 - 255.
//
// page_object  - the handle to the page object.
// R            - the red component of the object's fill color.
// G            - the green component of the object's fill color.
// B            - the blue component of the object's fill color.
// A            - the fill alpha of the object.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetFillColor(FPDF_PAGEOBJECT page_object,
                         unsigned int* R,
                         unsigned int* G,
                         unsigned int* B,
                         unsigned int* A);

// Experimental API.
// Get the line dash |phase| of |page_object|.
//
// page_object - handle to a page object.
// phase - pointer where the dashing phase will be stored.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetDashPhase(FPDF_PAGEOBJECT page_object, float* phase);

// Experimental API.
// Set the line dash phase of |page_object|.
//
// page_object - handle to a page object.
// phase - line dash phase.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetDashPhase(FPDF_PAGEOBJECT page_object, float phase);

// Experimental API.
// Get the line dash array of |page_object|.
//
// page_object - handle to a page object.
//
// Returns the line dash array size or -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV
FPDFPageObj_GetDashCount(FPDF_PAGEOBJECT page_object);

// Experimental API.
// Get the line dash array of |page_object|.
//
// page_object - handle to a page object.
// dash_array - pointer where the dashing array will be stored.
// dash_count - number of elements in |dash_array|.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_GetDashArray(FPDF_PAGEOBJECT page_object,
                         float* dash_array,
                         size_t dash_count);

// Experimental API.
// Set the line dash array of |page_object|.
//
// page_object - handle to a page object.
// dash_array - the dash array.
// dash_count - number of elements in |dash_array|.
// phase - the line dash phase.
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPageObj_SetDashArray(FPDF_PAGEOBJECT page_object,
                         const float* dash_array,
                         size_t dash_count,
                         float phase);

// Get number of segments inside |path|.
//
//   path - handle to a path.
//
// A segment is a command, created by e.g. FPDFPath_MoveTo(),
// FPDFPath_LineTo() or FPDFPath_BezierTo().
//
// Returns the number of objects in |path| or -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV FPDFPath_CountSegments(FPDF_PAGEOBJECT path);

// Get segment in |path| at |index|.
//
//   path  - handle to a path.
//   index - the index of a segment.
//
// Returns the handle to the segment, or NULL on faiure.
FPDF_EXPORT FPDF_PATHSEGMENT FPDF_CALLCONV
FPDFPath_GetPathSegment(FPDF_PAGEOBJECT path, int index);

// Get coordinates of |segment|.
//
//   segment  - handle to a segment.
//   x      - the horizontal position of the segment.
//   y      - the vertical position of the segment.
//
// Returns TRUE on success, otherwise |x| and |y| is not set.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPathSegment_GetPoint(FPDF_PATHSEGMENT segment, float* x, float* y);

// Get type of |segment|.
//
//   segment - handle to a segment.
//
// Returns one of the FPDF_SEGMENT_* values on success,
// FPDF_SEGMENT_UNKNOWN on error.
FPDF_EXPORT int FPDF_CALLCONV FPDFPathSegment_GetType(FPDF_PATHSEGMENT segment);

// Gets if the |segment| closes the current subpath of a given path.
//
//   segment - handle to a segment.
//
// Returns close flag for non-NULL segment, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFPathSegment_GetClose(FPDF_PATHSEGMENT segment);

// Move a path's current point.
//
// path   - the handle to the path object.
// x      - the horizontal position of the new current point.
// y      - the vertical position of the new current point.
//
// Note that no line will be created between the previous current point and the
// new one.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPath_MoveTo(FPDF_PAGEOBJECT path,
                                                    float x,
                                                    float y);

// Add a line between the current point and a new point in the path.
//
// path   - the handle to the path object.
// x      - the horizontal position of the new point.
// y      - the vertical position of the new point.
//
// The path's current point is changed to (x, y).
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPath_LineTo(FPDF_PAGEOBJECT path,
                                                    float x,
                                                    float y);

// Add a cubic Bezier curve to the given path, starting at the current point.
//
// path   - the handle to the path object.
// x1     - the horizontal position of the first Bezier control point.
// y1     - the vertical position of the first Bezier control point.
// x2     - the horizontal position of the second Bezier control point.
// y2     - the vertical position of the second Bezier control point.
// x3     - the horizontal position of the ending point of the Bezier curve.
// y3     - the vertical position of the ending point of the Bezier curve.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPath_BezierTo(FPDF_PAGEOBJECT path,
                                                      float x1,
                                                      float y1,
                                                      float x2,
                                                      float y2,
                                                      float x3,
                                                      float y3);

// Close the current subpath of a given path.
//
// path   - the handle to the path object.
//
// This will add a line between the current point and the initial point of the
// subpath, thus terminating the current subpath.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPath_Close(FPDF_PAGEOBJECT path);

// Set the drawing mode of a path.
//
// path     - the handle to the path object.
// fillmode - the filling mode to be set: one of the FPDF_FILLMODE_* flags.
// stroke   - a boolean specifying if the path should be stroked or not.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPath_SetDrawMode(FPDF_PAGEOBJECT path,
                                                         int fillmode,
                                                         FPDF_BOOL stroke);

// Get the drawing mode of a path.
//
// path     - the handle to the path object.
// fillmode - the filling mode of the path: one of the FPDF_FILLMODE_* flags.
// stroke   - a boolean specifying if the path is stroked or not.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFPath_GetDrawMode(FPDF_PAGEOBJECT path,
                                                         int* fillmode,
                                                         FPDF_BOOL* stroke);

// Create a new text object using one of the standard PDF fonts.
//
// document   - handle to the document.
// font       - string containing the font name, without spaces.
// font_size  - the font size for the new text object.
//
// Returns a handle to a new text object, or NULL on failure
FPDF_EXPORT FPDF_PAGEOBJECT FPDF_CALLCONV
FPDFPageObj_NewTextObj(FPDF_DOCUMENT document,
                       FPDF_BYTESTRING font,
                       float font_size);

// Set the text for a text object. If it had text, it will be replaced.
//
// text_object  - handle to the text object.
// text         - the UTF-16LE encoded string containing the text to be added.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFText_SetText(FPDF_PAGEOBJECT text_object, FPDF_WIDESTRING text);

// Experimental API.
// Set the text using charcodes for a text object. If it had text, it will be
// replaced.
//
// text_object  - handle to the text object.
// charcodes    - pointer to an array of charcodes to be added.
// count        - number of elements in |charcodes|.
//
// Returns TRUE on success
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFText_SetCharcodes(FPDF_PAGEOBJECT text_object,
                      const uint32_t* charcodes,
                      size_t count);

// Returns a font object loaded from a stream of data. The font is loaded
// into the document. Various font data structures, such as the ToUnicode data,
// are auto-generated based on the inputs.
//
// document  - handle to the document.
// data      - the stream of font data, which will be copied by the font object.
// size      - the size of the font data, in bytes.
// font_type - FPDF_FONT_TYPE1 or FPDF_FONT_TRUETYPE depending on the font type.
// cid       - a boolean specifying if the font is a CID font or not.
//
// The loaded font can be closed using FPDFFont_Close().
//
// Returns NULL on failure
FPDF_EXPORT FPDF_FONT FPDF_CALLCONV FPDFText_LoadFont(FPDF_DOCUMENT document,
                                                      const uint8_t* data,
                                                      uint32_t size,
                                                      int font_type,
                                                      FPDF_BOOL cid);

// Experimental API.
// Loads one of the standard 14 fonts per PDF spec 1.7 page 416. The preferred
// way of using font style is using a dash to separate the name from the style,
// for example 'Helvetica-BoldItalic'.
//
// document   - handle to the document.
// font       - string containing the font name, without spaces.
//
// The loaded font can be closed using FPDFFont_Close().
//
// Returns NULL on failure.
FPDF_EXPORT FPDF_FONT FPDF_CALLCONV
FPDFText_LoadStandardFont(FPDF_DOCUMENT document, FPDF_BYTESTRING font);

// Experimental API.
// Returns a font object loaded from a stream of data for a type 2 CID font. The
// font is loaded into the document. Unlike FPDFText_LoadFont(), the ToUnicode
// data and the CIDToGIDMap data are caller provided, instead of auto-generated.
//
// document                 - handle to the document.
// font_data                - the stream of font data, which will be copied by
//                            the font object.
// font_data_size           - the size of the font data, in bytes.
// to_unicode_cmap          - the ToUnicode data.
// cid_to_gid_map_data      - the stream of CIDToGIDMap data.
// cid_to_gid_map_data_size - the size of the CIDToGIDMap data, in bytes.
//
// The loaded font can be closed using FPDFFont_Close().
//
// Returns NULL on failure.
FPDF_EXPORT FPDF_FONT FPDF_CALLCONV
FPDFText_LoadCidType2Font(FPDF_DOCUMENT document,
                          const uint8_t* font_data,
                          uint32_t font_data_size,
                          FPDF_BYTESTRING to_unicode_cmap,
                          const uint8_t* cid_to_gid_map_data,
                          uint32_t cid_to_gid_map_data_size);

// Get the font size of a text object.
//
//   text - handle to a text.
//   size - pointer to the font size of the text object, measured in points
//   (about 1/72 inch)
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFTextObj_GetFontSize(FPDF_PAGEOBJECT text, float* size);

// Close a loaded PDF font.
//
// font   - Handle to the loaded font.
FPDF_EXPORT void FPDF_CALLCONV FPDFFont_Close(FPDF_FONT font);

// Create a new text object using a loaded font.
//
// document   - handle to the document.
// font       - handle to the font object.
// font_size  - the font size for the new text object.
//
// Returns a handle to a new text object, or NULL on failure
FPDF_EXPORT FPDF_PAGEOBJECT FPDF_CALLCONV
FPDFPageObj_CreateTextObj(FPDF_DOCUMENT document,
                          FPDF_FONT font,
                          float font_size);

// Get the text rendering mode of a text object.
//
// text     - the handle to the text object.
//
// Returns one of the known FPDF_TEXT_RENDERMODE enum values on success,
// FPDF_TEXTRENDERMODE_UNKNOWN on error.
FPDF_EXPORT FPDF_TEXT_RENDERMODE FPDF_CALLCONV
FPDFTextObj_GetTextRenderMode(FPDF_PAGEOBJECT text);

// Experimental API.
// Set the text rendering mode of a text object.
//
// text         - the handle to the text object.
// render_mode  - the FPDF_TEXT_RENDERMODE enum value to be set (cannot set to
//                FPDF_TEXTRENDERMODE_UNKNOWN).
//
// Returns TRUE on success.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFTextObj_SetTextRenderMode(FPDF_PAGEOBJECT text,
                              FPDF_TEXT_RENDERMODE render_mode);

// Get the text of a text object.
//
// text_object      - the handle to the text object.
// text_page        - the handle to the text page.
// buffer           - the address of a buffer that receives the text.
// length           - the size, in bytes, of |buffer|.
//
// Returns the number of bytes in the text (including the trailing NUL
// character) on success, 0 on error.
//
// Regardless of the platform, the |buffer| is always in UTF-16LE encoding.
// If |length| is less than the returned length, or |buffer| is NULL, |buffer|
// will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDFTextObj_GetText(FPDF_PAGEOBJECT text_object,
                    FPDF_TEXTPAGE text_page,
                    FPDF_WCHAR* buffer,
                    unsigned long length);

// Experimental API.
// Get a bitmap rasterization of |text_object|. To render correctly, the caller
// must provide the |document| associated with |text_object|. If there is a
// |page| associated with |text_object|, the caller should provide that as well.
// The returned bitmap will be owned by the caller, and FPDFBitmap_Destroy()
// must be called on the returned bitmap when it is no longer needed.
//
//   document    - handle to a document associated with |text_object|.
//   page        - handle to an optional page associated with |text_object|.
//   text_object - handle to a text object.
//   scale       - the scaling factor, which must be greater than 0.
//
// Returns the bitmap or NULL on failure.
FPDF_EXPORT FPDF_BITMAP FPDF_CALLCONV
FPDFTextObj_GetRenderedBitmap(FPDF_DOCUMENT document,
                              FPDF_PAGE page,
                              FPDF_PAGEOBJECT text_object,
                              float scale);

// Experimental API.
// Get the font of a text object.
//
// text - the handle to the text object.
//
// Returns a handle to the font object held by |text| which retains ownership.
FPDF_EXPORT FPDF_FONT FPDF_CALLCONV FPDFTextObj_GetFont(FPDF_PAGEOBJECT text);

// Experimental API.
// Get the base name of a font.
//
// font   - the handle to the font object.
// buffer - the address of a buffer that receives the base font name.
// length - the size, in bytes, of |buffer|.
//
// Returns the number of bytes in the base name (including the trailing NUL
// character) on success, 0 on error. The base name is typically the font's
// PostScript name. See descriptions of "BaseFont" in ISO 32000-1:2008 spec.
//
// Regardless of the platform, the |buffer| is always in UTF-8 encoding.
// If |length| is less than the returned length, or |buffer| is NULL, |buffer|
// will not be modified.
FPDF_EXPORT size_t FPDF_CALLCONV FPDFFont_GetBaseFontName(FPDF_FONT font,
                                                          char* buffer,
                                                          size_t length);

// Experimental API.
// Get the family name of a font.
//
// font   - the handle to the font object.
// buffer - the address of a buffer that receives the font name.
// length - the size, in bytes, of |buffer|.
//
// Returns the number of bytes in the family name (including the trailing NUL
// character) on success, 0 on error.
//
// Regardless of the platform, the |buffer| is always in UTF-8 encoding.
// If |length| is less than the returned length, or |buffer| is NULL, |buffer|
// will not be modified.
FPDF_EXPORT size_t FPDF_CALLCONV FPDFFont_GetFamilyName(FPDF_FONT font,
                                                        char* buffer,
                                                        size_t length);

// Experimental API.
// Get the decoded data from the |font| object.
//
// font       - The handle to the font object. (Required)
// buffer     - The address of a buffer that receives the font data.
// buflen     - Length of the buffer.
// out_buflen - Pointer to variable that will receive the minimum buffer size
//              to contain the font data. Not filled if the return value is
//              FALSE. (Required)
//
// Returns TRUE on success. In which case, |out_buflen| will be filled, and
// |buffer| will be filled if it is large enough. Returns FALSE if any of the
// required parameters are null.
//
// The decoded data is the uncompressed font data. i.e. the raw font data after
// having all stream filters applied, when the data is embedded.
//
// If the font is not embedded, then this API will instead return the data for
// the substitution font it is using.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFFont_GetFontData(FPDF_FONT font,
                                                         uint8_t* buffer,
                                                         size_t buflen,
                                                         size_t* out_buflen);

// Experimental API.
// Get whether |font| is embedded or not.
//
// font - the handle to the font object.
//
// Returns 1 if the font is embedded, 0 if it not, and -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV FPDFFont_GetIsEmbedded(FPDF_FONT font);

// Experimental API.
// Get the descriptor flags of a font.
//
// font - the handle to the font object.
//
// Returns the bit flags specifying various characteristics of the font as
// defined in ISO 32000-1:2008, table 123, -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV FPDFFont_GetFlags(FPDF_FONT font);

// Experimental API.
// Get the font weight of a font.
//
// font - the handle to the font object.
//
// Returns the font weight, -1 on failure.
// Typical values are 400 (normal) and 700 (bold).
FPDF_EXPORT int FPDF_CALLCONV FPDFFont_GetWeight(FPDF_FONT font);

// Experimental API.
// Get the italic angle of a font.
//
// font  - the handle to the font object.
// angle - pointer where the italic angle will be stored
//
// The italic angle of a |font| is defined as degrees counterclockwise
// from vertical. For a font that slopes to the right, this will be negative.
//
// Returns TRUE on success; |angle| unmodified on failure.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFFont_GetItalicAngle(FPDF_FONT font,
                                                            int* angle);

// Experimental API.
// Get ascent distance of a font.
//
// font       - the handle to the font object.
// font_size  - the size of the |font|.
// ascent     - pointer where the font ascent will be stored
//
// Ascent is the maximum distance in points above the baseline reached by the
// glyphs of the |font|. One point is 1/72 inch (around 0.3528 mm).
//
// Returns TRUE on success; |ascent| unmodified on failure.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFFont_GetAscent(FPDF_FONT font,
                                                       float font_size,
                                                       float* ascent);

// Experimental API.
// Get descent distance of a font.
//
// font       - the handle to the font object.
// font_size  - the size of the |font|.
// descent    - pointer where the font descent will be stored
//
// Descent is the maximum distance in points below the baseline reached by the
// glyphs of the |font|. One point is 1/72 inch (around 0.3528 mm).
//
// Returns TRUE on success; |descent| unmodified on failure.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFFont_GetDescent(FPDF_FONT font,
                                                        float font_size,
                                                        float* descent);

// Experimental API.
// Get the width of a glyph in a font.
//
// font       - the handle to the font object.
// glyph      - the glyph.
// font_size  - the size of the font.
// width      - pointer where the glyph width will be stored
//
// Glyph width is the distance from the end of the prior glyph to the next
// glyph. This will be the vertical distance for vertical writing.
//
// Returns TRUE on success; |width| unmodified on failure.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV FPDFFont_GetGlyphWidth(FPDF_FONT font,
                                                           uint32_t glyph,
                                                           float font_size,
                                                           float* width);

// Experimental API.
// Get the glyphpath describing how to draw a font glyph.
//
// font       - the handle to the font object.
// glyph      - the glyph being drawn.
// font_size  - the size of the font.
//
// Returns the handle to the segment, or NULL on faiure.
FPDF_EXPORT FPDF_GLYPHPATH FPDF_CALLCONV FPDFFont_GetGlyphPath(FPDF_FONT font,
                                                               uint32_t glyph,
                                                               float font_size);

// Experimental API.
// Get number of segments inside glyphpath.
//
// glyphpath - handle to a glyph path.
//
// Returns the number of objects in |glyphpath| or -1 on failure.
FPDF_EXPORT int FPDF_CALLCONV
FPDFGlyphPath_CountGlyphSegments(FPDF_GLYPHPATH glyphpath);

// Experimental API.
// Get segment in glyphpath at index.
//
// glyphpath  - handle to a glyph path.
// index      - the index of a segment.
//
// Returns the handle to the segment, or NULL on faiure.
FPDF_EXPORT FPDF_PATHSEGMENT FPDF_CALLCONV
FPDFGlyphPath_GetGlyphPathSegment(FPDF_GLYPHPATH glyphpath, int index);

// Get number of page objects inside |form_object|.
//
//   form_object - handle to a form object.
//
// Returns the number of objects in |form_object| on success, -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDFFormObj_CountObjects(FPDF_PAGEOBJECT form_object);

// Get page object in |form_object| at |index|.
//
//   form_object - handle to a form object.
//   index       - the 0-based index of a page object.
//
// Returns the handle to the page object, or NULL on error.
FPDF_EXPORT FPDF_PAGEOBJECT FPDF_CALLCONV
FPDFFormObj_GetObject(FPDF_PAGEOBJECT form_object, unsigned long index);

// Experimental API.
//
// Remove |page_object| from |form_object|.
//
//   form_object - handle to a form object.
//   page_object - handle to a page object to be removed from the form.
//
// Returns TRUE on success.
//
// Ownership of the removed |page_object| is transferred to the caller.
// Call FPDFPageObj_Destroy() on the removed page_object to free it.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDFFormObj_RemoveObject(FPDF_PAGEOBJECT form_object,
                         FPDF_PAGEOBJECT page_object);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  // PUBLIC_FPDF_EDIT_H_
