// Copyright 2014 The PDFium Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

// Original code copyright 2014 Foxit Software Inc. http://www.foxitsoftware.com

#ifndef PUBLIC_FPDF_PROGRESSIVE_H_
#define PUBLIC_FPDF_PROGRESSIVE_H_

// clang-format off
// NOLINTNEXTLINE(build/include)
#include "fpdfview.h"

// Flags for progressive process status.
#define FPDF_RENDER_READY 0
#define FPDF_RENDER_TOBECONTINUED 1
#define FPDF_RENDER_DONE 2
#define FPDF_RENDER_FAILED 3

#ifdef __cplusplus
extern "C" {
#endif

// IFPDF_RENDERINFO interface.
typedef struct _IFSDK_PAUSE {
  // Version number of the interface. Currently must be 1.
  int version;

  // Method: NeedToPauseNow
  //           Check if we need to pause a progressive process now.
  // Interface Version:
  //           1
  // Implementation Required:
  //           yes
  // Parameters:
  //           pThis       -   Pointer to the interface structure itself
  // Return Value:
  //           Non-zero for pause now, 0 for continue.
  FPDF_BOOL (*NeedToPauseNow)(struct _IFSDK_PAUSE* pThis);

  // A user defined data pointer, used by user's application. Can be NULL.
  void* user;
} IFSDK_PAUSE;

// Experimental API.
// Function: FPDF_RenderPageBitmapWithColorScheme_Start
//          Start to render page contents to a device independent bitmap
//          progressively with a specified color scheme for the content.
// Parameters:
//          bitmap       -   Handle to the device independent bitmap (as the
//                           output buffer). Bitmap handle can be created by
//                           FPDFBitmap_Create function.
//          page         -   Handle to the page as returned by FPDF_LoadPage
//                           function.
//          start_x      -   Left pixel position of the display area in the
//                           bitmap coordinate.
//          start_y      -   Top pixel position of the display area in the
//                           bitmap coordinate.
//          size_x       -   Horizontal size (in pixels) for displaying the
//                           page.
//          size_y       -   Vertical size (in pixels) for displaying the page.
//          rotate       -   Page orientation: 0 (normal), 1 (rotated 90
//                           degrees clockwise), 2 (rotated 180 degrees),
//                           3 (rotated 90 degrees counter-clockwise).
//          flags        -   0 for normal display, or combination of flags
//                           defined in fpdfview.h. With FPDF_ANNOT flag, it
//                           renders all annotations that does not require
//                           user-interaction, which are all annotations except
//                           widget and popup annotations.
//          color_scheme -   Color scheme to be used in rendering the |page|.
//                           If null, this function will work similar to
//                           FPDF_RenderPageBitmap_Start().
//          pause        -   The IFSDK_PAUSE interface. A callback mechanism
//                           allowing the page rendering process.
// Return value:
//          Rendering Status. See flags for progressive process status for the
//          details.
FPDF_EXPORT int FPDF_CALLCONV
FPDF_RenderPageBitmapWithColorScheme_Start(FPDF_BITMAP bitmap,
                                           FPDF_PAGE page,
                                           int start_x,
                                           int start_y,
                                           int size_x,
                                           int size_y,
                                           int rotate,
                                           int flags,
                                           const FPDF_COLORSCHEME* color_scheme,
                                           IFSDK_PAUSE* pause);

// Function: FPDF_RenderPageBitmap_Start
//          Start to render page contents to a device independent bitmap
//          progressively.
// Parameters:
//          bitmap      -   Handle to the device independent bitmap (as the
//                          output buffer). Bitmap handle can be created by
//                          FPDFBitmap_Create().
//          page        -   Handle to the page, as returned by FPDF_LoadPage().
//          start_x     -   Left pixel position of the display area in the
//                          bitmap coordinates.
//          start_y     -   Top pixel position of the display area in the bitmap
//                          coordinates.
//          size_x      -   Horizontal size (in pixels) for displaying the page.
//          size_y      -   Vertical size (in pixels) for displaying the page.
//          rotate      -   Page orientation: 0 (normal), 1 (rotated 90 degrees
//                          clockwise), 2 (rotated 180 degrees), 3 (rotated 90
//                          degrees counter-clockwise).
//          flags       -   0 for normal display, or combination of flags
//                          defined in fpdfview.h. With FPDF_ANNOT flag, it
//                          renders all annotations that does not require
//                          user-interaction, which are all annotations except
//                          widget and popup annotations.
//          pause       -   The IFSDK_PAUSE interface.A callback mechanism
//                          allowing the page rendering process
// Return value:
//          Rendering Status. See flags for progressive process status for the
//          details.
FPDF_EXPORT int FPDF_CALLCONV FPDF_RenderPageBitmap_Start(FPDF_BITMAP bitmap,
                                                          FPDF_PAGE page,
                                                          int start_x,
                                                          int start_y,
                                                          int size_x,
                                                          int size_y,
                                                          int rotate,
                                                          int flags,
                                                          IFSDK_PAUSE* pause);

// Function: FPDF_RenderPage_Continue
//          Continue rendering a PDF page.
// Parameters:
//          page        -   Handle to the page, as returned by FPDF_LoadPage().
//          pause       -   The IFSDK_PAUSE interface (a callback mechanism
//                          allowing the page rendering process to be paused
//                          before it's finished). This can be NULL if you
//                          don't want to pause.
// Return value:
//          The rendering status. See flags for progressive process status for
//          the details.
FPDF_EXPORT int FPDF_CALLCONV FPDF_RenderPage_Continue(FPDF_PAGE page,
                                                       IFSDK_PAUSE* pause);

// Function: FPDF_RenderPage_Close
//          Release the resource allocate during page rendering. Need to be
//          called after finishing rendering or
//          cancel the rendering.
// Parameters:
//          page        -   Handle to the page, as returned by FPDF_LoadPage().
// Return value:
//          None.
FPDF_EXPORT void FPDF_CALLCONV FPDF_RenderPage_Close(FPDF_PAGE page);

#ifdef __cplusplus
}
#endif

#endif  // PUBLIC_FPDF_PROGRESSIVE_H_
