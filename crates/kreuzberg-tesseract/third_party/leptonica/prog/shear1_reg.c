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
 *   shear1_reg.c
 *
 *    Regression test for shear, both IP and to new pix.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   BINARY_IMAGE              "test1.png"
#define   TWO_BPP_IMAGE             "weasel2.4c.png"
#define   FOUR_BPP_IMAGE1           "weasel4.11c.png"
#define   FOUR_BPP_IMAGE2           "weasel4.16g.png"
#define   EIGHT_BPP_IMAGE           "test8.jpg"
#define   EIGHT_BPP_CMAP_IMAGE1     "dreyfus8.png"
#define   EIGHT_BPP_CMAP_IMAGE2     "test24.jpg"
#define   RGB_IMAGE                 "marge.jpg"

static PIX *shearTest1(PIX *pixs, l_float32 scale);
static PIX *shearTest2(PIX *pixs, L_REGPARAMS *rp);

static const l_float32  ANGLE1 = 3.14159265 / 12.;

l_int32 main(int    argc,
             char **argv)
{
l_int32       index;
PIX          *pixs, *pix1, *pixc, *pixd;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_stderr("Test binary image:\n");
    pixs = pixRead(BINARY_IMAGE);
    pixd = shearTest1(pixs, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixd);

        /* We change the black to dark red so that we can see
         * that the IP shear does brings in that color.  It
         * can't bring in black because the cmap is filled. */
    lept_stderr("Test 2 bpp cmapped image with filled cmap:\n");
    pixs = pixRead(TWO_BPP_IMAGE);
    cmap = pixGetColormap(pixs);
    pixcmapGetIndex(cmap, 40, 44, 40, &index);
    pixcmapResetColor(cmap, index, 100, 0, 0);
    pixd = shearTest1(pixs, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixd);

    lept_stderr("Test 4 bpp cmapped image with unfilled cmap:\n");
    pixs = pixRead(FOUR_BPP_IMAGE1);
    pixd = shearTest1(pixs, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixd);

    lept_stderr("Test 4 bpp cmapped image with filled cmap:\n");
    pixs = pixRead(FOUR_BPP_IMAGE2);
    pixd = shearTest1(pixs, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixd);

    lept_stderr("Test 8 bpp grayscale image:\n");
    pixs = pixRead(EIGHT_BPP_IMAGE);
    pix1 = pixScale(pixs, 0.5, 0.5);
    pixd = shearTest1(pixs, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 4 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pixd);

    lept_stderr("Test 8 bpp grayscale cmap image:\n");
    pixs = pixRead(EIGHT_BPP_CMAP_IMAGE1);
    pixd = shearTest1(pixs, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixd);

    lept_stderr("Test 8 bpp color cmap image:\n");
    pixs = pixRead(EIGHT_BPP_CMAP_IMAGE2);
    pix1 = pixScale(pixs, 0.3, 0.3);
    pixd = pixOctreeColorQuant(pix1, 200, 0);
    pixc = shearTest1(pixd, 1.0);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 6 */
    pixDisplayWithTitle(pixc, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pixd);
    pixDestroy(&pixc);

    lept_stderr("Test rgb image:\n");
    pixs = pixRead(RGB_IMAGE);
    pix1 = pixScale(pixs, 0.3, 0.3);
    pixd = shearTest1(pix1, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 7 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pixd);

#if 1
    lept_stderr("Test in-place shear on 4 bpp cmapped image:\n");
    pixs = pixRead(FOUR_BPP_IMAGE1);
    pixd = shearTest2(pixs, rp);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pixd, 800, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
#endif

    return regTestCleanup(rp);
}

/* ------------------------------------------------------------- */
static PIX *
shearTest1(PIX       *pixs,
           l_float32  scale)
{
l_int32  w, h, d;
PIX     *pix1, *pix2, *pixd;
PIXA    *pixa;

    pixa = pixaCreate(0);
    pixGetDimensions(pixs, &w, &h, &d);

    pix1 = pixHShear(NULL, pixs, 0, ANGLE1, L_BRING_IN_WHITE);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixHShear(NULL, pixs, h / 2, ANGLE1, L_BRING_IN_WHITE);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix1 = pixHShear(NULL, pixs, 0, ANGLE1, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixHShear(NULL, pixs, h / 2, ANGLE1, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix2, L_INSERT);

    if (!pixGetColormap(pixs)) {
        pix1 = pixCopy(NULL, pixs);
        pixHShearIP(pix1, 0, ANGLE1, L_BRING_IN_WHITE);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixCopy(NULL, pixs);
        pixHShearIP(pix2, h / 2, ANGLE1, L_BRING_IN_WHITE);
        pixaAddPix(pixa, pix2, L_INSERT);
        pix1 = pixCopy(NULL, pixs);
        pixHShearIP(pix1, 0, ANGLE1, L_BRING_IN_BLACK);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixCopy(NULL, pixs);
        pixHShearIP(pix2, h / 2, ANGLE1, L_BRING_IN_BLACK);
        pixaAddPix(pixa, pix2, L_INSERT);
    }

    if (d == 8 || d == 32 || pixGetColormap(pixs)) {
        pix1 = pixHShearLI(pixs, 0, ANGLE1, L_BRING_IN_WHITE);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixHShearLI(pixs, w / 2, ANGLE1, L_BRING_IN_WHITE);
        pixaAddPix(pixa, pix2, L_INSERT);
        pix1 = pixHShearLI(pixs, 0, ANGLE1, L_BRING_IN_BLACK);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixHShearLI(pixs, w / 2, ANGLE1, L_BRING_IN_BLACK);
        pixaAddPix(pixa, pix2, L_INSERT);
    }

    pix1 = pixVShear(NULL, pixs, 0, ANGLE1, L_BRING_IN_WHITE);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixVShear(NULL, pixs, w / 2, ANGLE1, L_BRING_IN_WHITE);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix1 = pixVShear(NULL, pixs, 0, ANGLE1, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixVShear(NULL, pixs, w / 2, ANGLE1, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix2, L_INSERT);

    if (!pixGetColormap(pixs)) {
        pix1 = pixCopy(NULL, pixs);
        pixVShearIP(pix1, 0, ANGLE1, L_BRING_IN_WHITE);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixCopy(NULL, pixs);
        pixVShearIP(pix2, w / 2, ANGLE1, L_BRING_IN_WHITE);
        pixaAddPix(pixa, pix2, L_INSERT);
        pix1 = pixCopy(NULL, pixs);
        pixVShearIP(pix1, 0, ANGLE1, L_BRING_IN_BLACK);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixCopy(NULL, pixs);
        pixVShearIP(pix2, w / 2, ANGLE1, L_BRING_IN_BLACK);
        pixaAddPix(pixa, pix2, L_INSERT);
    }

    if (d == 8 || d == 32 || pixGetColormap(pixs)) {
        pix1 = pixVShearLI(pixs, 0, ANGLE1, L_BRING_IN_WHITE);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixVShearLI(pixs, w / 2, ANGLE1, L_BRING_IN_WHITE);
        pixaAddPix(pixa, pix2, L_INSERT);
        pix1 = pixVShearLI(pixs, 0, ANGLE1, L_BRING_IN_BLACK);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixVShearLI(pixs, w / 2, ANGLE1, L_BRING_IN_BLACK);
        pixaAddPix(pixa, pix2, L_INSERT);
    }

    pixd = pixaDisplayTiledInColumns(pixa, 4, scale, 20, 0);
    pixaDestroy(&pixa);
    return pixd;
}

/* ------------------------------------------------------------- */
static PIX *
shearTest2(PIX          *pixs,
           L_REGPARAMS  *rp)
{
l_int32  w, h;
PIX     *pix1, *pix2, *pixd;
PIXA    *pixa;

    pixa = pixaCreate(0);
    pixGetDimensions(pixs, &w, &h, NULL);

    pix1 = pixHShear(NULL, pixs, h / 2, ANGLE1, L_BRING_IN_WHITE);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixCopy(NULL, pixs);
    pixHShear(pix2, pix2, h / 2, ANGLE1, L_BRING_IN_WHITE);
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestComparePix(rp, pix1, pix2);  /* 8 */
    pix1 = pixHShear(NULL, pixs, h / 2, ANGLE1, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixCopy(NULL, pixs);
    pixHShear(pix2, pix2, h / 2, ANGLE1, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestComparePix(rp, pix1, pix2);  /* 9 */

    pix1 = pixVShear(NULL, pixs, w / 2, ANGLE1, L_BRING_IN_WHITE);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixCopy(NULL, pixs);
    pixVShear(pix2, pix2, w / 2, ANGLE1, L_BRING_IN_WHITE);
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestComparePix(rp, pix1, pix2);  /* 10 */
    pix1 = pixVShear(NULL, pixs, w / 2, ANGLE1, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixCopy(NULL, pixs);
    pixVShear(pix2, pix2, w / 2, ANGLE1, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestComparePix(rp, pix1, pix2);  /* 11 */

    pixd = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 0);
    pixaDestroy(&pixa);
    return pixd;
}
