// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_SYSFONTINFO_H_
#define PUBLIC_FPDF_SYSFONTINFO_H_

#include <stddef.h>

// clang-format off
// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

// Character sets for the font
#define FXFONT_ANSI_CHARSET 0
#define FXFONT_DEFAULT_CHARSET 1
#define FXFONT_SYMBOL_CHARSET 2
#define FXFONT_SHIFTJIS_CHARSET 128
#define FXFONT_HANGEUL_CHARSET 129
#define FXFONT_GB2312_CHARSET 134
#define FXFONT_CHINESEBIG5_CHARSET 136
#define FXFONT_GREEK_CHARSET 161
#define FXFONT_VIETNAMESE_CHARSET 163
#define FXFONT_HEBREW_CHARSET 177
#define FXFONT_ARABIC_CHARSET 178
#define FXFONT_CYRILLIC_CHARSET 204
#define FXFONT_THAI_CHARSET 222
#define FXFONT_EASTERNEUROPEAN_CHARSET 238

// Font pitch and family flags
#define FXFONT_FF_FIXEDPITCH (1 << 0)
#define FXFONT_FF_ROMAN (1 << 4)
#define FXFONT_FF_SCRIPT (4 << 4)

// Typical weight values
#define FXFONT_FW_NORMAL 400
#define FXFONT_FW_BOLD 700

// Exported Functions
#ifdef __cplusplus
extern "C" {
#endif

// Interface: FPDF_SYSFONTINFO
//          Interface for getting system font information and font mapping
typedef struct _FPDF_SYSFONTINFO {
  // Version number of the interface. Currently must be 1 or 2.
  // Version 1: Traditional behavior - calls EnumFonts during initialization.
  // Version 2: Per-request behavior - skips EnumFonts, relies on MapFont.
  //            Experimental: Subject to change based on feedback.
  int version;

  // Method: Release
  //          Give implementation a chance to release any data after the
  //          interface is no longer used.
  // Interface Version:
  //          1 and 2
  // Implementation Required:
  //          No
  // Parameters:
  //          pThis       -   Pointer to the interface structure itself
  // Return Value:
  //          None
  // Comments:
  //          Called by PDFium during the final cleanup process.
  void (*Release)(struct _FPDF_SYSFONTINFO* pThis);

  // Method: EnumFonts
  //          Enumerate all fonts installed on the system
  // Interface Version:
  //          1
  // Implementation Required:
  //          No
  // Parameters:
  //          pThis       -   Pointer to the interface structure itself
  //          pMapper     -   An opaque pointer to internal font mapper, used
  //                          when calling FPDF_AddInstalledFont().
  // Return Value:
  //          None
  // Comments:
  //          Implementations should call FPDF_AddInstalledFont() function for
  //          each font found. Only TrueType/OpenType and Type1 fonts are
  //          accepted by PDFium.
  //          NOTE: This method will not be called when version is set to 2.
  //          Version 2 relies entirely on MapFont() for per-request matching.
  void (*EnumFonts)(struct _FPDF_SYSFONTINFO* pThis, void* pMapper);

  // Method: MapFont
  //          Use the system font mapper to get a font handle from requested
  //          parameters.
  // Interface Version:
  //          1 and 2
  // Implementation Required:
  //          Required if GetFont method is not implemented.
  // Parameters:
  //          pThis       -   Pointer to the interface structure itself
  //          weight      -   Weight of the requested font. 400 is normal and
  //                          700 is bold.
  //          bItalic     -   Italic option of the requested font, TRUE or
  //                          FALSE.
  //          charset     -   Character set identifier for the requested font.
  //                          See above defined constants.
  //          pitch_family -  A combination of flags. See above defined
  //                          constants.
  //          face        -   Typeface name. Currently use system local encoding
  //                          only.
  //          bExact      -   Obsolete: this parameter is now ignored.
  // Return Value:
  //          An opaque pointer for font handle, or NULL if system mapping is
  //          not supported.
  // Comments:
  //          If the system supports native font mapper (like Windows),
  //          implementation can implement this method to get a font handle.
  //          Otherwise, PDFium will do the mapping and then call GetFont
  //          method. Only TrueType/OpenType and Type1 fonts are accepted
  //          by PDFium.
  void* (*MapFont)(struct _FPDF_SYSFONTINFO* pThis,
                   int weight,
                   FPDF_BOOL bItalic,
                   int charset,
                   int pitch_family,
                   const char* face,
                   FPDF_BOOL* bExact);

  // Method: GetFont
  //          Get a handle to a particular font by its internal ID
  // Interface Version:
  //          1 and 2
  // Implementation Required:
  //          Required if MapFont method is not implemented.
  // Return Value:
  //          An opaque pointer for font handle.
  // Parameters:
  //          pThis       -   Pointer to the interface structure itself
  //          face        -   Typeface name in system local encoding.
  // Comments:
  //          If the system mapping not supported, PDFium will do the font
  //          mapping and use this method to get a font handle.
  void* (*GetFont)(struct _FPDF_SYSFONTINFO* pThis, const char* face);

  // Method: GetFontData
  //          Get font data from a font
  // Interface Version:
  //          1 and 2
  // Implementation Required:
  //          Yes
  // Parameters:
  //          pThis       -   Pointer to the interface structure itself
  //          hFont       -   Font handle returned by MapFont or GetFont method
  //          table       -   TrueType/OpenType table identifier (refer to
  //                          TrueType specification), or 0 for the whole file.
  //          buffer      -   The buffer receiving the font data. Can be NULL if
  //                          not provided.
  //          buf_size    -   Buffer size, can be zero if not provided.
  // Return Value:
  //          Number of bytes needed, if buffer not provided or not large
  //          enough, or number of bytes written into buffer otherwise.
  // Comments:
  //          Can read either the full font file, or a particular
  //          TrueType/OpenType table.
  unsigned long (*GetFontData)(struct _FPDF_SYSFONTINFO* pThis,
                               void* hFont,
                               unsigned int table,
                               unsigned char* buffer,
                               unsigned long buf_size);

  // Method: GetFaceName
  //          Get face name from a font handle
  // Interface Version:
  //          1 and 2
  // Implementation Required:
  //          No
  // Parameters:
  //          pThis       -   Pointer to the interface structure itself
  //          hFont       -   Font handle returned by MapFont or GetFont method
  //          buffer      -   The buffer receiving the face name. Can be NULL if
  //                          not provided
  //          buf_size    -   Buffer size, can be zero if not provided
  // Return Value:
  //          Number of bytes needed, if buffer not provided or not large
  //          enough, or number of bytes written into buffer otherwise.
  unsigned long (*GetFaceName)(struct _FPDF_SYSFONTINFO* pThis,
                               void* hFont,
                               char* buffer,
                               unsigned long buf_size);

  // Method: GetFontCharset
  //          Get character set information for a font handle
  // Interface Version:
  //          1 and 2
  // Implementation Required:
  //          No
  // Parameters:
  //          pThis       -   Pointer to the interface structure itself
  //          hFont       -   Font handle returned by MapFont or GetFont method
  // Return Value:
  //          Character set identifier. See defined constants above.
  int (*GetFontCharset)(struct _FPDF_SYSFONTINFO* pThis, void* hFont);

  // Method: DeleteFont
  //          Delete a font handle
  // Interface Version:
  //          1 and 2
  // Implementation Required:
  //          Yes
  // Parameters:
  //          pThis       -   Pointer to the interface structure itself
  //          hFont       -   Font handle returned by MapFont or GetFont method
  // Return Value:
  //          None
  void (*DeleteFont)(struct _FPDF_SYSFONTINFO* pThis, void* hFont);
} FPDF_SYSFONTINFO;

// Struct: FPDF_CharsetFontMap
//    Provides the name of a font to use for a given charset value.
typedef struct FPDF_CharsetFontMap_ {
  int charset;  // Character Set Enum value, see FXFONT_*_CHARSET above.
  const char* fontname;  // Name of default font to use with that charset.
} FPDF_CharsetFontMap;

// Function: FPDF_GetDefaultTTFMap
//    Returns a pointer to the default character set to TT Font name map. The
//    map is an array of FPDF_CharsetFontMap structs, with its end indicated
//    by a { -1, NULL } entry.
// Parameters:
//     None.
// Return Value:
//     Pointer to the Charset Font Map.
// Note:
//     Once FPDF_GetDefaultTTFMapCount() and FPDF_GetDefaultTTFMapEntry() are no
//     longer experimental, this API will be marked as deprecated.
//     See https://crbug.com/348468114
FPDF_EXPORT const FPDF_CharsetFontMap* FPDF_CALLCONV FPDF_GetDefaultTTFMap();

// Experimental API.
//
// Function: FPDF_GetDefaultTTFMapCount
//    Returns the number of entries in the default character set to TT Font name
//    map.
// Parameters:
//    None.
// Return Value:
//    The number of entries in the map.
FPDF_EXPORT size_t FPDF_CALLCONV FPDF_GetDefaultTTFMapCount();

// Experimental API.
//
// Function: FPDF_GetDefaultTTFMapEntry
//    Returns an entry in the default character set to TT Font name map.
// Parameters:
//    index    -   The index to the entry in the map to retrieve.
// Return Value:
//     A pointer to the entry, if it is in the map, or NULL if the index is out
//     of bounds.
FPDF_EXPORT const FPDF_CharsetFontMap* FPDF_CALLCONV
FPDF_GetDefaultTTFMapEntry(size_t index);

// Function: FPDF_AddInstalledFont
//          Add a system font to the list in PDFium.
// Comments:
//          This function is only called during the system font list building
//          process.
// Parameters:
//          mapper          -   Opaque pointer to Foxit font mapper
//          face            -   The font face name
//          charset         -   Font character set. See above defined constants.
// Return Value:
//          None.
FPDF_EXPORT void FPDF_CALLCONV FPDF_AddInstalledFont(void* mapper,
                                                     const char* face,
                                                     int charset);

// Function: FPDF_SetSystemFontInfo
//          Set the system font info interface into PDFium
// Parameters:
//          font_info       -   Pointer to a FPDF_SYSFONTINFO structure
// Return Value:
//          None
// Comments:
//          Platform support implementation should implement required methods of
//          FFDF_SYSFONTINFO interface, then call this function during PDFium
//          initialization process.
//
//          Call this with NULL to tell PDFium to stop using a previously set
//          |FPDF_SYSFONTINFO|.
FPDF_EXPORT void FPDF_CALLCONV
FPDF_SetSystemFontInfo(FPDF_SYSFONTINFO* font_info);

// Function: FPDF_GetDefaultSystemFontInfo
//          Get default system font info interface for current platform
// Parameters:
//          None
// Return Value:
//          Pointer to a FPDF_SYSFONTINFO structure describing the default
//          interface, or NULL if the platform doesn't have a default interface.
//          Application should call FPDF_FreeDefaultSystemFontInfo to free the
//          returned pointer.
// Comments:
//          For some platforms, PDFium implements a default version of system
//          font info interface. The default implementation can be passed to
//          FPDF_SetSystemFontInfo().
FPDF_EXPORT FPDF_SYSFONTINFO* FPDF_CALLCONV FPDF_GetDefaultSystemFontInfo();

// Function: FPDF_FreeDefaultSystemFontInfo
//           Free a default system font info interface
// Parameters:
//           font_info       -   Pointer to a FPDF_SYSFONTINFO structure
// Return Value:
//           None
// Comments:
//           This function should be called on the output from
//           FPDF_GetDefaultSystemFontInfo() once it is no longer needed.
FPDF_EXPORT void FPDF_CALLCONV
FPDF_FreeDefaultSystemFontInfo(FPDF_SYSFONTINFO* font_info);

#ifdef __cplusplus
}
#endif

#endif  // PUBLIC_FPDF_SYSFONTINFO_H_
