// Copyright 2016 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_STRUCTTREE_H_
#define PUBLIC_FPDF_STRUCTTREE_H_

// clang-format off
// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

#ifdef __cplusplus
extern "C" {
#endif

// Function: FPDF_StructTree_GetForPage
//          Get the structure tree for a page.
// Parameters:
//          page        -   Handle to the page, as returned by FPDF_LoadPage().
// Return value:
//          A handle to the structure tree or NULL on error. The caller owns the
//          returned handle and must use FPDF_StructTree_Close() to release it.
//          The handle should be released before |page| gets released.
FPDF_EXPORT FPDF_STRUCTTREE FPDF_CALLCONV
FPDF_StructTree_GetForPage(FPDF_PAGE page);

// Function: FPDF_StructTree_Close
//          Release a resource allocated by FPDF_StructTree_GetForPage().
// Parameters:
//          struct_tree -   Handle to the structure tree, as returned by
//                          FPDF_StructTree_LoadPage().
// Return value:
//          None.
FPDF_EXPORT void FPDF_CALLCONV
FPDF_StructTree_Close(FPDF_STRUCTTREE struct_tree);

// Function: FPDF_StructTree_CountChildren
//          Count the number of children for the structure tree.
// Parameters:
//          struct_tree -   Handle to the structure tree, as returned by
//                          FPDF_StructTree_LoadPage().
// Return value:
//          The number of children, or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructTree_CountChildren(FPDF_STRUCTTREE struct_tree);

// Function: FPDF_StructTree_GetChildAtIndex
//          Get a child in the structure tree.
// Parameters:
//          struct_tree -   Handle to the structure tree, as returned by
//                          FPDF_StructTree_LoadPage().
//          index       -   The index for the child, 0-based.
// Return value:
//          The child at the n-th index or NULL on error. The caller does not
//          own the handle. The handle remains valid as long as |struct_tree|
//          remains valid.
// Comments:
//          The |index| must be less than the FPDF_StructTree_CountChildren()
//          return value.
FPDF_EXPORT FPDF_STRUCTELEMENT FPDF_CALLCONV
FPDF_StructTree_GetChildAtIndex(FPDF_STRUCTTREE struct_tree, int index);

// Function: FPDF_StructElement_GetAltText
//          Get the alt text for a given element.
// Parameters:
//          struct_element -   Handle to the struct element.
//          buffer         -   A buffer for output the alt text. May be NULL.
//          buflen         -   The length of the buffer, in bytes. May be 0.
// Return value:
//          The number of bytes in the alt text, including the terminating NUL
//          character. The number of bytes is returned regardless of the
//          |buffer| and |buflen| parameters.
// Comments:
//          Regardless of the platform, the |buffer| is always in UTF-16LE
//          encoding. The string is terminated by a UTF16 NUL character. If
//          |buflen| is less than the required length, or |buffer| is NULL,
//          |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_StructElement_GetAltText(FPDF_STRUCTELEMENT struct_element,
                              void* buffer,
                              unsigned long buflen);

// Experimental API.
// Function: FPDF_StructElement_GetActualText
//          Get the actual text for a given element.
// Parameters:
//          struct_element -   Handle to the struct element.
//          buffer         -   A buffer for output the actual text. May be NULL.
//          buflen         -   The length of the buffer, in bytes. May be 0.
// Return value:
//          The number of bytes in the actual text, including the terminating
//          NUL character. The number of bytes is returned regardless of the
//          |buffer| and |buflen| parameters.
// Comments:
//          Regardless of the platform, the |buffer| is always in UTF-16LE
//          encoding. The string is terminated by a UTF16 NUL character. If
//          |buflen| is less than the required length, or |buffer| is NULL,
//          |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_StructElement_GetActualText(FPDF_STRUCTELEMENT struct_element,
                                 void* buffer,
                                 unsigned long buflen);

// Function: FPDF_StructElement_GetID
//          Get the ID for a given element.
// Parameters:
//          struct_element -   Handle to the struct element.
//          buffer         -   A buffer for output the ID string. May be NULL.
//          buflen         -   The length of the buffer, in bytes. May be 0.
// Return value:
//          The number of bytes in the ID string, including the terminating NUL
//          character. The number of bytes is returned regardless of the
//          |buffer| and |buflen| parameters.
// Comments:
//          Regardless of the platform, the |buffer| is always in UTF-16LE
//          encoding. The string is terminated by a UTF16 NUL character. If
//          |buflen| is less than the required length, or |buffer| is NULL,
//          |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_StructElement_GetID(FPDF_STRUCTELEMENT struct_element,
                         void* buffer,
                         unsigned long buflen);

// Experimental API.
// Function: FPDF_StructElement_GetLang
//          Get the case-insensitive IETF BCP 47 language code for an element.
// Parameters:
//          struct_element -   Handle to the struct element.
//          buffer         -   A buffer for output the lang string. May be NULL.
//          buflen         -   The length of the buffer, in bytes. May be 0.
// Return value:
//          The number of bytes in the ID string, including the terminating NUL
//          character. The number of bytes is returned regardless of the
//          |buffer| and |buflen| parameters.
// Comments:
//          Regardless of the platform, the |buffer| is always in UTF-16LE
//          encoding. The string is terminated by a UTF16 NUL character. If
//          |buflen| is less than the required length, or |buffer| is NULL,
//          |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_StructElement_GetLang(FPDF_STRUCTELEMENT struct_element,
                           void* buffer,
                           unsigned long buflen);

// Experimental API.
// Function: FPDF_StructElement_GetStringAttribute
//          Get a struct element attribute of type "name" or "string".
// Parameters:
//          struct_element -   Handle to the struct element.
//          attr_name      -   The name of the attribute to retrieve.
//          buffer         -   A buffer for output. May be NULL.
//          buflen         -   The length of the buffer, in bytes. May be 0.
// Return value:
//          The number of bytes in the attribute value, including the
//          terminating NUL character. The number of bytes is returned
//          regardless of the |buffer| and |buflen| parameters.
// Comments:
//          Regardless of the platform, the |buffer| is always in UTF-16LE
//          encoding. The string is terminated by a UTF16 NUL character. If
//          |buflen| is less than the required length, or |buffer| is NULL,
//          |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_StructElement_GetStringAttribute(FPDF_STRUCTELEMENT struct_element,
                                      FPDF_BYTESTRING attr_name,
                                      void* buffer,
                                      unsigned long buflen);

// Function: FPDF_StructElement_GetMarkedContentID
//          Get the marked content ID for a given element.
// Parameters:
//          struct_element -   Handle to the struct element.
// Return value:
//          The marked content ID of the element. If no ID exists, returns
//          -1.
// Comments:
//          FPDF_StructElement_GetMarkedContentIdAtIndex() may be able to
//          extract more marked content IDs out of |struct_element|. This API
//          may be deprecated in the future.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructElement_GetMarkedContentID(FPDF_STRUCTELEMENT struct_element);

// Function: FPDF_StructElement_GetType
//           Get the type (/S) for a given element.
// Parameters:
//           struct_element - Handle to the struct element.
//           buffer         - A buffer for output. May be NULL.
//           buflen         - The length of the buffer, in bytes. May be 0.
// Return value:
//           The number of bytes in the type, including the terminating NUL
//           character. The number of bytes is returned regardless of the
//           |buffer| and |buflen| parameters.
// Comments:
//           Regardless of the platform, the |buffer| is always in UTF-16LE
//           encoding. The string is terminated by a UTF16 NUL character. If
//           |buflen| is less than the required length, or |buffer| is NULL,
//           |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_StructElement_GetType(FPDF_STRUCTELEMENT struct_element,
                           void* buffer,
                           unsigned long buflen);

// Experimental API.
// Function: FPDF_StructElement_GetObjType
//           Get the object type (/Type) for a given element.
// Parameters:
//           struct_element - Handle to the struct element.
//           buffer         - A buffer for output. May be NULL.
//           buflen         - The length of the buffer, in bytes. May be 0.
// Return value:
//           The number of bytes in the object type, including the terminating
//           NUL character. The number of bytes is returned regardless of the
//           |buffer| and |buflen| parameters.
// Comments:
//           Regardless of the platform, the |buffer| is always in UTF-16LE
//           encoding. The string is terminated by a UTF16 NUL character. If
//           |buflen| is less than the required length, or |buffer| is NULL,
//           |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_StructElement_GetObjType(FPDF_STRUCTELEMENT struct_element,
                              void* buffer,
                              unsigned long buflen);

// Function: FPDF_StructElement_GetTitle
//           Get the title (/T) for a given element.
// Parameters:
//           struct_element - Handle to the struct element.
//           buffer         - A buffer for output. May be NULL.
//           buflen         - The length of the buffer, in bytes. May be 0.
// Return value:
//           The number of bytes in the title, including the terminating NUL
//           character. The number of bytes is returned regardless of the
//           |buffer| and |buflen| parameters.
// Comments:
//           Regardless of the platform, the |buffer| is always in UTF-16LE
//           encoding. The string is terminated by a UTF16 NUL character. If
//           |buflen| is less than the required length, or |buffer| is NULL,
//           |buffer| will not be modified.
FPDF_EXPORT unsigned long FPDF_CALLCONV
FPDF_StructElement_GetTitle(FPDF_STRUCTELEMENT struct_element,
                            void* buffer,
                            unsigned long buflen);

// Function: FPDF_StructElement_CountChildren
//          Count the number of children for the structure element.
// Parameters:
//          struct_element -   Handle to the struct element.
// Return value:
//          The number of children, or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructElement_CountChildren(FPDF_STRUCTELEMENT struct_element);

// Function: FPDF_StructElement_GetChildAtIndex
//          Get a child in the structure element.
// Parameters:
//          struct_element -   Handle to the struct element.
//          index          -   The index for the child, 0-based.
// Return value:
//          The child at the n-th index or NULL on error.
// Comments:
//          If the child exists but is not an element, then this function will
//          return NULL. This will also return NULL for out of bounds indices.
//          The |index| must be less than the FPDF_StructElement_CountChildren()
//          return value.
FPDF_EXPORT FPDF_STRUCTELEMENT FPDF_CALLCONV
FPDF_StructElement_GetChildAtIndex(FPDF_STRUCTELEMENT struct_element,
                                   int index);

// Experimental API.
// Function: FPDF_StructElement_GetChildMarkedContentID
//          Get the child's content id
// Parameters:
//          struct_element -   Handle to the struct element.
//          index          -   The index for the child, 0-based.
// Return value:
//          The marked content ID of the child. If no ID exists, returns -1.
// Comments:
//          If the child exists but is not a stream or object, then this
//          function will return -1. This will also return -1 for out of bounds
//          indices. Compared to FPDF_StructElement_GetMarkedContentIdAtIndex,
//          it is scoped to the current page.
//          The |index| must be less than the FPDF_StructElement_CountChildren()
//          return value.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructElement_GetChildMarkedContentID(FPDF_STRUCTELEMENT struct_element,
                                           int index);

// Experimental API.
// Function: FPDF_StructElement_GetParent
//          Get the parent of the structure element.
// Parameters:
//          struct_element -   Handle to the struct element.
// Return value:
//          The parent structure element or NULL on error.
// Comments:
//          If structure element is StructTreeRoot, then this function will
//          return NULL.
FPDF_EXPORT FPDF_STRUCTELEMENT FPDF_CALLCONV
FPDF_StructElement_GetParent(FPDF_STRUCTELEMENT struct_element);

// Function: FPDF_StructElement_GetAttributeCount
//          Count the number of attributes for the structure element.
// Parameters:
//          struct_element -   Handle to the struct element.
// Return value:
//          The number of attributes, or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructElement_GetAttributeCount(FPDF_STRUCTELEMENT struct_element);

// Experimental API.
// Function: FPDF_StructElement_GetAttributeAtIndex
//          Get an attribute object in the structure element.
// Parameters:
//          struct_element -   Handle to the struct element.
//          index          -   The index for the attribute object, 0-based.
// Return value:
//          The attribute object at the n-th index or NULL on error.
// Comments:
//          If the attribute object exists but is not a dict, then this
//          function will return NULL. This will also return NULL for out of
//          bounds indices. The caller does not own the handle. The handle
//          remains valid as long as |struct_element| remains valid.
//          The |index| must be less than the
//          FPDF_StructElement_GetAttributeCount() return value.
FPDF_EXPORT FPDF_STRUCTELEMENT_ATTR FPDF_CALLCONV
FPDF_StructElement_GetAttributeAtIndex(FPDF_STRUCTELEMENT struct_element, int index);

// Experimental API.
// Function: FPDF_StructElement_Attr_GetCount
//          Count the number of attributes in a structure element attribute map.
// Parameters:
//          struct_attribute - Handle to the struct element attribute.
// Return value:
//          The number of attributes, or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructElement_Attr_GetCount(FPDF_STRUCTELEMENT_ATTR struct_attribute);


// Experimental API.
// Function: FPDF_StructElement_Attr_GetName
//          Get the name of an attribute in a structure element attribute map.
// Parameters:
//          struct_attribute   - Handle to the struct element attribute.
//          index              - The index of attribute in the map.
//          buffer             - A buffer for output. May be NULL. This is only
//                               modified if |buflen| is longer than the length
//                               of the key. Optional, pass null to just
//                               retrieve the size of the buffer needed.
//          buflen             - The length of the buffer.
//          out_buflen         - A pointer to variable that will receive the
//                               minimum buffer size to contain the key. Not
//                               filled if FALSE is returned.
// Return value:
//          TRUE if the operation was successful, FALSE otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_StructElement_Attr_GetName(FPDF_STRUCTELEMENT_ATTR struct_attribute,
                                int index,
                                void* buffer,
                                unsigned long buflen,
                                unsigned long* out_buflen);
// Experimental API.
// Function: FPDF_StructElement_Attr_GetValue
//           Get a handle to a value for an attribute in a structure element
//           attribute map.
// Parameters:
//           struct_attribute   - Handle to the struct element attribute.
//           name               - The attribute name.
// Return value:
//           Returns a handle to the value associated with the input, if any.
//           Returns NULL on failure. The caller does not own the handle.
//           The handle remains valid as long as |struct_attribute| remains
//           valid.
FPDF_EXPORT FPDF_STRUCTELEMENT_ATTR_VALUE FPDF_CALLCONV
FPDF_StructElement_Attr_GetValue(FPDF_STRUCTELEMENT_ATTR struct_attribute,
                                 FPDF_BYTESTRING name);

// Experimental API.
// Function: FPDF_StructElement_Attr_GetType
//           Get the type of an attribute in a structure element attribute map.
// Parameters:
//           value - Handle to the value.
// Return value:
//           Returns the type of the value, or FPDF_OBJECT_UNKNOWN in case of
//           failure. Note that this will never return FPDF_OBJECT_REFERENCE, as
//           references are always dereferenced.
FPDF_EXPORT FPDF_OBJECT_TYPE FPDF_CALLCONV
FPDF_StructElement_Attr_GetType(FPDF_STRUCTELEMENT_ATTR_VALUE value);

// Experimental API.
// Function: FPDF_StructElement_Attr_GetBooleanValue
//           Get the value of a boolean attribute in an attribute map as
//           FPDF_BOOL. FPDF_StructElement_Attr_GetType() should have returned
//           FPDF_OBJECT_BOOLEAN for this property.
// Parameters:
//           value     - Handle to the value.
//           out_value - A pointer to variable that will receive the value. Not
//                       filled if false is returned.
// Return value:
//           Returns TRUE if the attribute maps to a boolean value, FALSE
//           otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_StructElement_Attr_GetBooleanValue(FPDF_STRUCTELEMENT_ATTR_VALUE value,
                                        FPDF_BOOL* out_value);

// Experimental API.
// Function: FPDF_StructElement_Attr_GetNumberValue
//           Get the value of a number attribute in an attribute map as float.
//           FPDF_StructElement_Attr_GetType() should have returned
//           FPDF_OBJECT_NUMBER for this property.
// Parameters:
//           value     - Handle to the value.
//           out_value - A pointer to variable that will receive the value. Not
//                       filled if false is returned.
// Return value:
//           Returns TRUE if the attribute maps to a number value, FALSE
//           otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_StructElement_Attr_GetNumberValue(FPDF_STRUCTELEMENT_ATTR_VALUE value,
                                       float* out_value);

// Experimental API.
// Function: FPDF_StructElement_Attr_GetStringValue
//           Get the value of a string attribute in an attribute map as string.
//           FPDF_StructElement_Attr_GetType() should have returned
//           FPDF_OBJECT_STRING or FPDF_OBJECT_NAME for this property.
// Parameters:
//           value      - Handle to the value.
//           buffer     - A buffer for holding the returned key in UTF-16LE.
//                        This is only modified if |buflen| is longer than the
//                        length of the key. Optional, pass null to just
//                        retrieve the size of the buffer needed.
//           buflen     - The length of the buffer.
//           out_buflen - A pointer to variable that will receive the minimum
//                        buffer size to contain the key. Not filled if FALSE is
//                        returned.
// Return value:
//           Returns TRUE if the attribute maps to a string value, FALSE
//           otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_StructElement_Attr_GetStringValue(FPDF_STRUCTELEMENT_ATTR_VALUE value,
                                       void* buffer,
                                       unsigned long buflen,
                                       unsigned long* out_buflen);

// Experimental API.
// Function: FPDF_StructElement_Attr_GetBlobValue
//           Get the value of a blob attribute in an attribute map as string.
// Parameters:
//           value      - Handle to the value.
//           buffer     - A buffer for holding the returned value. This is only
//                        modified if |buflen| is at least as long as the length
//                        of the value. Optional, pass null to just retrieve the
//                        size of the buffer needed.
//           buflen     - The length of the buffer.
//           out_buflen - A pointer to variable that will receive the minimum
//                        buffer size to contain the key. Not filled if FALSE is
//                        returned.
// Return value:
//           Returns TRUE if the attribute maps to a string value, FALSE
//           otherwise.
FPDF_EXPORT FPDF_BOOL FPDF_CALLCONV
FPDF_StructElement_Attr_GetBlobValue(FPDF_STRUCTELEMENT_ATTR_VALUE value,
                                     void* buffer,
                                     unsigned long buflen,
                                     unsigned long* out_buflen);

// Experimental API.
// Function: FPDF_StructElement_Attr_CountChildren
//           Count the number of children values in an attribute.
// Parameters:
//           value - Handle to the value.
// Return value:
//           The number of children, or -1 on error.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructElement_Attr_CountChildren(FPDF_STRUCTELEMENT_ATTR_VALUE value);

// Experimental API.
// Function: FPDF_StructElement_Attr_GetChildAtIndex
//           Get a child from an attribute.
// Parameters:
//           value - Handle to the value.
//           index - The index for the child, 0-based.
// Return value:
//           The child at the n-th index or NULL on error.
// Comments:
//           The |index| must be less than the
//           FPDF_StructElement_Attr_CountChildren() return value.
FPDF_EXPORT FPDF_STRUCTELEMENT_ATTR_VALUE FPDF_CALLCONV
FPDF_StructElement_Attr_GetChildAtIndex(FPDF_STRUCTELEMENT_ATTR_VALUE value,
                                        int index);

// Experimental API.
// Function: FPDF_StructElement_GetMarkedContentIdCount
//          Get the count of marked content ids for a given element.
// Parameters:
//          struct_element -   Handle to the struct element.
// Return value:
//          The count of marked content ids or -1 if none exists.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructElement_GetMarkedContentIdCount(FPDF_STRUCTELEMENT struct_element);

// Experimental API.
// Function: FPDF_StructElement_GetMarkedContentIdAtIndex
//          Get the marked content id at a given index for a given element.
// Parameters:
//          struct_element -   Handle to the struct element.
//          index          -   The index of the marked content id, 0-based.
// Return value:
//          The marked content ID of the element. If no ID exists, returns
//          -1.
// Comments:
//          The |index| must be less than the
//          FPDF_StructElement_GetMarkedContentIdCount() return value.
//          This will likely supersede FPDF_StructElement_GetMarkedContentID().
FPDF_EXPORT int FPDF_CALLCONV
FPDF_StructElement_GetMarkedContentIdAtIndex(FPDF_STRUCTELEMENT struct_element,
                                             int index);

#ifdef __cplusplus
}  // extern "C"
#endif

#endif  // PUBLIC_FPDF_STRUCTTREE_H_
