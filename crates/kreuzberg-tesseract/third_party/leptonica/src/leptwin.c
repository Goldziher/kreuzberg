/*====================================================================*
 -  Copyright (C) 2001 Leptonica.  All rights reserved.
 -
 -  Redistribution and use in source and binary forms, with or without
 -  modification, are permitted provided that the following conditions
 -  are met:
 -  1. Redistributions of source code must retain the above copyright
 -     notice, this list of conditions and the following disclaimer.
 -  2. Redistributions in binary form must reproduce the above
 -     copyright notice, this list of conditions and the following
 -     disclaimer in the documentation and/or other materials
 -     provided with the distribution.
 -
 -  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 -  ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 -  LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
 -  A PARTICULAR PURPOSE ARE DISCLAIMED.  IN NO EVENT SHALL ANY
 -  CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
 -  EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
 -  PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 -  PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY
 -  OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
 -  NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 -  SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 *====================================================================*/

/*!
 * \file leptwin.c
 * <pre>
 *
 *    This file contains Leptonica routines needed only on Microsoft Windows
 *
 *    Currently it only contains one public function
 *    (based on dibsectn.c by jmh, 03-30-98):
 *
 *      HBITMAP    pixGetWindowsHBITMAP(PIX *pix)
 * </pre>
 */

#ifdef _WIN32
#include <stdlib.h>
#include <string.h>
#include "allheaders.h"
#include "leptwin.h"

/* Macro to determine the number of bytes per line in the DIB bits.
 * This accounts for DWORD alignment by adding 31 bits,
 * then dividing by 32, then rounding up to the next highest
 * count of 4-bytes.  Then, we multiply by 4 to get the total byte count.  */
#define BYTESPERLINE(Width, BPP) ((l_int32)((((DWORD)(Width) * (DWORD)(BPP) + 31) >> 5)) << 2)


/* **********************************************************************
  DWORD DSImageBitsSize(LPBITMAPINFO pbmi)

  PARAMETERS:
    LPBITMAPINFO - pointer to a BITMAPINFO describing a DIB

  RETURNS:
    DWORD    - the size, in bytes, of the DIB's image bits

  REMARKS:
    Calculates and returns the size, in bytes, of the image bits for
    the DIB described by the BITMAPINFO.
********************************************************************** */
static DWORD
DSImageBitsSize(LPBITMAPINFO pbmi)
{
    switch(pbmi->bmiHeader.biCompression)
    {
    case BI_RLE8:  /* wrong if haven't called DSCreateDIBSection or
                    * CreateDIBSection with this pbmi */
    case BI_RLE4:
        return pbmi->bmiHeader.biSizeImage;

	default:  /* should not have to use "default" */
    case BI_RGB:
    case BI_BITFIELDS:
        return BYTESPERLINE(pbmi->bmiHeader.biWidth, \
                   pbmi->bmiHeader.biBitCount * pbmi->bmiHeader.biPlanes) *
               pbmi->bmiHeader.biHeight;
    }
}

/* **********************************************************************
  DWORD ImageBitsSize(HBITMAP hbitmap)

  PARAMETERS:
    HBITMAP - hbitmap

  RETURNS:
    DWORD    - the size, in bytes, of the HBITMAP's image bits

  REMARKS:
    Calculates and returns the size, in bytes, of the image bits for
    the DIB described by the HBITMAP.
********************************************************************** */
static DWORD
ImageBitsSize(HBITMAP hBitmap)
{
    DIBSECTION  ds;

    GetObject(hBitmap, sizeof(DIBSECTION), &ds);
    switch( ds.dsBmih.biCompression )
    {
    case BI_RLE8:  /* wrong if haven't called DSCreateDIBSection or
                    * CreateDIBSection with this pbmi */
    case BI_RLE4:
        return ds.dsBmih.biSizeImage;

	default:  /* should not have to use "default" */
    case BI_RGB:
    case BI_BITFIELDS:
        return BYTESPERLINE(ds.dsBmih.biWidth, \
                            ds.dsBmih.biBitCount * ds.dsBmih.biPlanes) *
                            ds.dsBmih.biHeight;
    }
}

/*!
 * \brief   setColormap(LPBITMAPINFO pbmi, PIXCMAP *cmap)
 *
 * \param[in]    pbmi pointer to a BITMAPINFO describing a DIB
 * \param[in]    cmap leptonica colormap
 * \return  number of colors in cmap
 */
static int
setColormap(LPBITMAPINFO  pbmi,
            PIXCMAP      *cmap)
{
l_int32  i, nColors, rval, gval, bval;

    nColors = pixcmapGetCount(cmap);
    for (i = 0; i < nColors; i++) {
        pixcmapGetColor(cmap, i, &rval, &gval, &bval);
        pbmi->bmiColors[i].rgbRed = rval;
        pbmi->bmiColors[i].rgbGreen = gval;
        pbmi->bmiColors[i].rgbBlue = bval;
        pbmi->bmiColors[i].rgbReserved = 0;
    }
    pbmi->bmiHeader.biClrUsed = nColors;
    return nColors;
}

/* **********************************************************************
  HBITMAP DSCreateBitmapInfo(l_int32 width, l_int32 height, l_int32 depth,
                             PIXCMAP *cmap)

  PARAMETERS:
    l_int32 width - Desired width of the DIBSection
    l_int32 height - Desired height of the DIBSection
    l_int32 depth - Desired bit-depth of the DIBSection
    PIXCMAP cmap - leptonica colormap for depths < 16

  RETURNS:
    LPBITMAPINFO - a ptr to BITMAPINFO of the desired size and bit-depth
                   NULL on failure

  REMARKS:
    Creates a BITMAPINFO based on the criteria passed in as parameters.

********************************************************************** */
static LPBITMAPINFO
DSCreateBitmapInfo(l_int32 width,
                   l_int32 height,
                   l_int32 depth,
                   PIXCMAP *cmap)
{
l_int32       nInfoSize;
LPBITMAPINFO  pbmi;
LPDWORD       pMasks;

    nInfoSize = sizeof(BITMAPINFOHEADER);
    if( depth <= 8 )
        nInfoSize += sizeof(RGBQUAD) * (1 << depth);
    if((depth == 16) || (depth == 32))
        nInfoSize += (3 * sizeof(DWORD));

        /* Create the header big enough to contain color table and
         * bitmasks if needed. */
    pbmi = (LPBITMAPINFO)malloc(nInfoSize);
    if (!pbmi)
        return NULL;

    ZeroMemory(pbmi, nInfoSize);
    pbmi->bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
    pbmi->bmiHeader.biWidth = width;
    pbmi->bmiHeader.biHeight = height;
    pbmi->bmiHeader.biPlanes = 1;
    pbmi->bmiHeader.biBitCount = depth;

        /* override below for 16 and 32 bpp */
    pbmi->bmiHeader.biCompression = BI_RGB;

        /*  ?? not sure if this is right?  */
    pbmi->bmiHeader.biSizeImage = DSImageBitsSize(pbmi);

    pbmi->bmiHeader.biXPelsPerMeter = 0;
    pbmi->bmiHeader.biYPelsPerMeter = 0;
    pbmi->bmiHeader.biClrUsed = 0;  /* override below */
    pbmi->bmiHeader.biClrImportant = 0;

    switch(depth)
    {
        case 24:
                /* 24bpp requires no special handling */
            break;
        case 16:
                /* if it's 16bpp, fill in the masks and override the
                 * compression.  These are the default masks -- you
                 * could change them if needed. */
            pMasks = (LPDWORD)(pbmi->bmiColors);
            pMasks[0] = 0x00007c00;
            pMasks[1] = 0x000003e0;
            pMasks[2] = 0x0000001f;
            pbmi->bmiHeader.biCompression = BI_BITFIELDS;
            break;
        case 32:
                /* if it's 32 bpp, fill in the masks and override
                 * the compression */
            pMasks = (LPDWORD)(pbmi->bmiColors);
            /*pMasks[0] = 0x00ff0000; */
            /*pMasks[1] = 0x0000ff00; */
            /*pMasks[2] = 0x000000ff; */
            pMasks[0] = 0xff000000;
            pMasks[1] = 0x00ff0000;
            pMasks[2] = 0x0000ff00;

            pbmi->bmiHeader.biCompression = BI_BITFIELDS;
            break;
        case 8:
        case 4:
        case 1:
            setColormap(pbmi, cmap);
            break;
    }
    return pbmi;
}

/* **********************************************************************
  HBITMAP DSCreateDIBSection(l_int32 width, l_int32 height, l_int32 depth,
                             PIXCMAP *cmap)

  PARAMETERS:
    l_int32 width - Desired width of the DIBSection
    l_int32 height - Desired height of the DIBSection
    l_int32 depth - Desired bit-depth of the DIBSection
    PIXCMAP cmap - leptonica colormap for depths < 16

  RETURNS:
    HBITMAP      - a DIBSection HBITMAP of the desired size and bit-depth
                   NULL on failure

  REMARKS:
    Creates a DIBSection based on the criteria passed in as parameters.

********************************************************************** */
static HBITMAP
DSCreateDIBSection(l_int32  width,
                   l_int32  height,
                   l_int32  depth,
                   PIXCMAP  *cmap)
{
HBITMAP       hBitmap;
l_int32       nInfoSize;
LPBITMAPINFO  pbmi;
HDC           hRefDC;
LPBYTE        pBits;

    pbmi = DSCreateBitmapInfo (width, height, depth, cmap);
    if (!pbmi)
        return NULL;

    hRefDC = GetDC(NULL);
    hBitmap = CreateDIBSection(hRefDC, pbmi, DIB_RGB_COLORS,
                               (void **) &pBits, NULL, 0);
    nInfoSize = GetLastError();
    ReleaseDC(NULL, hRefDC);
    free(pbmi);

    return hBitmap;
}


/*!
 * \brief   pixGetWindowsHBITMAP()
 *
 * \param[in]    pix
 * \return  Windows hBitmap, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) It's the responsibility of the caller to destroy the
 *          returned hBitmap with a call to DeleteObject (or with
 *          something that eventually calls DeleteObject).
 * </pre>
 */
HBITMAP
pixGetWindowsHBITMAP(PIX  *pix)
{
l_int32    width, height, depth;
l_uint32  *data;
HBITMAP    hBitmap = NULL;
BITMAP     bm;
DWORD      imageBitsSize;
PIX       *pixt = NULL;
PIXCMAP   *cmap;

    if (!pix)
        return (HBITMAP)ERROR_PTR("pix not defined", __func__, NULL);

    pixGetDimensions(pix, &width, &height, &depth);
    cmap = pixGetColormap(pix);

    if (depth == 24) depth = 32;
    if (depth == 2) {
        pixt = pixConvert2To8(pix, 0, 85, 170, 255, TRUE);
        if (!pixt)
            return (HBITMAP)ERROR_PTR("unable to convert pix from 2bpp to 8bpp",
                    __func__, NULL);
        depth = pixGetDepth(pixt);
        cmap = pixGetColormap(pixt);
    }

    if (depth < 16) {
        if (!cmap)
            cmap = pixcmapCreateLinear(depth, 1<<depth);
    }

    hBitmap = DSCreateDIBSection(width, height, depth, cmap);
    if (!hBitmap)
        return (HBITMAP)ERROR_PTR("Unable to create HBITMAP", __func__, NULL);

        /* By default, Windows assumes bottom up images */
    if (pixt)
        pixt = pixFlipTB(pixt, pixt);
    else
        pixt = pixFlipTB(NULL, pix);

        /* "standard" color table assumes bit off=black */
    if (depth == 1) {
        pixInvert(pixt, pixt);
    }

        /* Don't byte swap until done manipulating pix! */
    if (depth <= 16)
        pixEndianByteSwap(pixt);

    GetObject (hBitmap, sizeof(BITMAP), &bm);
    imageBitsSize = ImageBitsSize(hBitmap);
    data = pixGetData(pixt);
    if (data) {
        memcpy (bm.bmBits, data, imageBitsSize);
    } else {
        DeleteObject (hBitmap);
        hBitmap = NULL;
    }
    pixDestroy(&pixt);

    return hBitmap;
}

#endif   /* _WIN32 */
