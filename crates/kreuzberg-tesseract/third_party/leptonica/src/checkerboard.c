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

/*
 * \file  checkerboard.c
 * <pre>
 *
 *    Find the checker corners where 4 squares come together
 *      PIX            *pixFindCheckerboardCorners()
 *
 *    Generate the hit-miss sels
 *      static SELA    *makeCheckerboardCornerSela()
 *      static PIXA    *makeCheckerboardCornerPixa()
 *
 * The functions in this file locate the corners where four squares
 * in a checkerboard come together.  With a perfectly aligned checkerboard,
 * the solution is trivial: take the union of two hit-miss transforms (HMTs),
 * each having a simple diagonal structuring element (sel).  The two
 * sels can be generated from strings such as these, using
 * selCreateFromString():
 *
 *  static const char *str1 = "o     x"
 *                            "       "
 *                            "       "
 *                            "   C   "
 *                            "       "
 *                            "       "
 *                            "x     o";
 *  static const char *str2 = "x     o"
 *                            "       "
 *                            "       "
 *                            "   C   "
 *                            "       "
 *                            "       "
 *                            "o     x";
 *
 * A more interesting problem is to consider the checkerboard viewed from
 * some arbitrary angle and orientation from the normal.  The method
 * developed here works for a camera located within a cone with an opening
 * half-angle of about 45 degrees, and with its axis along the normal
 * to the checkerboard.
 *
 * See prog/checkerboard_reg.c for usage.
 *
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Static helpers */
static SELA *makeCheckerboardCornerSela(l_int32 size, l_int32 dilation,
                                        l_int32 nsels, PIXA *pixadb);
static PIXA *makeCheckerboardCornerPixa(l_int32 size, l_int32 dilation,
                                        l_int32 nsels);

static const char selnames[64] = "s_diag1 s_diag2 s_cross1 s_cross2";

/*!
 * \brief   pixFindCheckerboardCorner()
 *
 * \param[in]    pixs           of checkerboard
 * \param[in]    size           size of HMT sel; >= 7, typ. 15; 0 for default
 * \param[in]    dilation       size of hit and miss squares; typ. 1 or 3; max 5
 * \param[in]    nsels          number to use (either 2 or 4)
 * \param[out]   ppix_corners   [optional] 1 bpp pix giving corner locations
 * \param[out]   ppta_corners   [optional] pta giving corner locations
 * \param[in]    pixadb         [optional] pass in pre-allocated
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Use %nsels = 4 if the checkerboard may be rotated by more
 *          than 20 deg.
 *      (2) The values of %size and %dilation that can be used depend on
 *          the square sizes.  Nominal values here are for squares of
 *          size 30 to 50.  In general, because of the viewing angle
 *          of the camera, the "squares" will appear approximately
 *          as a rotated rectangle.
 *      (3) The outputs pix_corners and pta_corners are optional.
 * </pre>
 */
l_ok
pixFindCheckerboardCorners(PIX     *pixs,
                           l_int32  size,
                           l_int32  dilation,
                           l_int32  nsels,
                           PIX    **ppix_corners,
                           PTA    **ppta_corners,
                           PIXA    *pixadb)
{
BOXA    *boxa1;
PIX     *pix1, *pix2, *pix3;
PTA     *pta1;
SEL     *sel;
SELA    *sela;

    if (ppix_corners) *ppix_corners = NULL;
    if (ppta_corners) *ppta_corners = NULL;
    if (!pixs)
        return ERROR_INT("pixs not defined", __func__, 1);
    if (size <= 0) size = 7;
    if (size < 7)
        return ERROR_INT("size too small", __func__, 1);
    if (dilation < 1 || dilation > 5)
        return ERROR_INT("dilation not in [1 ...5]", __func__, 1);
    if (nsels != 2 && nsels != 4)
        return ERROR_INT("nsels not 2 or 4", __func__, 1);

        /* Generate the hit-miss sels for finding corners */
    sela = makeCheckerboardCornerSela(size, dilation, nsels, pixadb);
    if (!sela)
        return ERROR_INT("sela not made", __func__, 1);
    if (pixadb) {
        pix1 = selaDisplayInPix(sela, 15, 3, 15, 2);
        pixaAddPix(pixadb, pix1, L_INSERT);
    }

        /* Do the hit-miss transform to find corner locations */
    pix1 = pixUnionOfMorphOps(pixs, sela, L_MORPH_HMT);
    if (pixadb) pixaAddPix(pixadb, pix1, L_CLONE);
    selaDestroy(&sela);

        /* Remove large noise c.c. */
    pix2 = pixSelectBySize(pix1, size, size, 8, L_SELECT_IF_BOTH,
                           L_SELECT_IF_LTE, NULL);
    if (pixadb) pixaAddPix(pixadb, pix2, L_CLONE);

        /* Thin remaining c.c. */
    pix3 = pixThinConnected(pix2, L_THIN_FG, 8, 0);
    if (pixadb) pixaAddPix(pixadb, pix3, L_CLONE);

        /* Extract the location of the center of each component */
    boxa1 = pixConnCompBB(pix3, 8);
    pta1 = boxaExtractCorners(boxa1, L_BOX_CENTER);
    boxaDestroy(&boxa1);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    if (pixadb) {  /* show the result as colored plus signs on the input */
        sel = selMakePlusSign(15, 2);
        pix1 = pixDisplaySelectedPixels(pixs, pix3, sel, 0xff000000);
        pixaAddPix(pixadb, pix1, L_INSERT);
        selDestroy(&sel);
    }

    if (ppix_corners)
        *ppix_corners = pix3;
    else
        pixDestroy(&pix3);
    if (ppta_corners)
        *ppta_corners = pta1;
    else
        ptaDestroy(&pta1);
    return 0;
}


/*!
 * \brief   makeCheckerboardCornerSela()
 *
 * \param[in]    size         size of HMT sel; >= 7, typ. 15; 0 for default
 * \param[in]    dilation     size of hit and miss squares; typ. 1 or 3; max 5
 * \param[in]    nsels        number to use (either 2 or 4)
 * \param[in]    pixadb       [optional] pass in pre-allocated
 * \return  sela   hit-miss sels for finding corners, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) Use 4 sels if the checkerboard may be rotated by more than 20 deg.
 * </pre>
 */
static SELA *
makeCheckerboardCornerSela(l_int32  size,
                           l_int32  dilation,
                           l_int32  nsels,
                           PIXA    *pixadb)
{
PIX     *pix1;
PIXA    *pixa1;
SARRAY  *sa;
SELA    *sela;

    if (size <= 0) size = 7;
    if (size < 7)
        return (SELA *)ERROR_PTR("size too small", __func__, NULL);
    if (dilation < 1 || dilation > 5)
        return (SELA *)ERROR_PTR("dilation not in [1 ...5]", __func__, NULL);
    if (nsels != 2 && nsels != 4)
        return (SELA *)ERROR_PTR("nsels not 2 or 4", __func__, NULL);

    if ((pixa1 = makeCheckerboardCornerPixa(size, dilation, nsels)) == NULL)
        return (SELA *)ERROR_PTR("pixa for sels not made", __func__, NULL);
    if (pixadb) {
        pix1 = pixaDisplayTiledInColumns(pixa1, 4, 8.0, 15, 2);
        pixaAddPix(pixadb, pix1, L_INSERT);
    }
    sa = sarrayCreateWordsFromString(selnames);
    sela = selaCreateFromColorPixa(pixa1, sa);
    pixaDestroy(&pixa1);
    sarrayDestroy(&sa);
    if (!sela)
        return (SELA *)ERROR_PTR("sela not made", __func__, NULL);
    return sela;
}


/*!
 * \brief   makeCheckerboardCornerPixa()
 *
 * \param[in]    size         size of HMT sel; >= 7, typ. 15; 0 for default
 * \param[in]    dilation     size of hit and miss squares; typ. 1 or 3; max 5
 * \param[in]    nsels        number to use (either 2 or 4)
 * \return  pixa   representing hit-miss sels for finding corners, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) Each pix can be used to generate a hit-miss sel, using the
 *          function selCreateFromColorPix().  See that function for the
 *          use of color and gray pixels to encode the hits, misses and
 *          center in the structuring element.
 * </pre>
 */
static PIXA *
makeCheckerboardCornerPixa(l_int32  size,
                           l_int32  dilation,
                           l_int32  nsels)
{
PIX   *pix1, *pix2, *pix3;
PIXA  *pixa1;

    pixa1 = pixaCreate(4);

        /* Represent diagonal neg slope hits and pos slope misses */
    pix1 = pixCreate(size, size, 32);
    pixSetAll(pix1);
    pix2 = pixCreate(size, size, 1);  /* slope -1 line (2 pixel) mask */
    pixSetPixel(pix2, 1, 1, 1);  /* UL corner */
    pixSetPixel(pix2, size - 2, size - 2, 1);  /* LR corner */
    if (dilation > 1)
        pixDilateBrick(pix2, pix2, dilation, dilation);  /* dilate each pixel */
    pixSetMasked(pix1, pix2, 0x00ff0000);  /* green hit */
    pix3 = pixRotate90(pix2, 1);  /* slope +1 line (2 pixel) mask */
    pixSetMasked(pix1, pix3, 0xff000000);  /* red miss */
    pixSetRGBPixel(pix1, size / 2, size / 2, 128, 128, 128);  /* gray center */
    pixaAddPix(pixa1, pix1, L_INSERT);

        /* Represent diagonal pos slope hits and neg slope misses */
    pix1 = pixCreate(size, size, 32);
    pixSetAll(pix1);
    pixSetMasked(pix1, pix2, 0xff000000);  /* red hit */
    pixSetMasked(pix1, pix3, 0x00ff0000);  /* green miss */
    pixSetRGBPixel(pix1, size / 2, size / 2, 128, 128, 128);  /* gray center */
    pixaAddPix(pixa1, pix1, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    if (nsels == 2)
        return pixa1;

        /* Represent cross: vertical hits and horizontal misses */
    pix1 = pixCreate(size, size, 32);
    pixSetAll(pix1);
    pix2 = pixCreate(size, size, 1);  /* vertical line (2 pixel) mask */
    pixSetPixel(pix2, size / 2, 1, 1);
    pixSetPixel(pix2, size / 2, size - 2, 1);
    if (dilation > 1)
        pixDilateBrick(pix2, pix2, dilation, dilation);  /* dilate each pixel */
    pixSetMasked(pix1, pix2, 0x00ff0000);  /* green hit */
    pix3 = pixRotate90(pix2, 1);  /* horizontal line (2 pixel) mask */
    pixSetMasked(pix1, pix3, 0xff000000);  /* red miss */
    pixSetRGBPixel(pix1, size / 2, size / 2, 128, 128, 128);  /* gray center */
    pixaAddPix(pixa1, pix1, L_INSERT);

        /* Represent cross: horizontal hits and vertical misses */
    pix1 = pixCreate(size, size, 32);
    pixSetAll(pix1);
    pixSetMasked(pix1, pix3, 0x00ff0000);  /* green hit */
    pixSetMasked(pix1, pix2, 0xff000000);  /* red miss */
    pixSetRGBPixel(pix1, size / 2, size / 2, 128, 128, 128);  /* gray center */
    pixaAddPix(pixa1, pix1, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    return pixa1;
}

