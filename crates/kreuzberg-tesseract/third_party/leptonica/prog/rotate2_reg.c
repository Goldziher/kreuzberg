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
 *   rotate2_reg.c
 *
 *    Regression test for rotation by shear, sampling and area mapping.
 *    Displays results from all the various types of rotations.
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

static const l_float32  ANGLE1 = 3.14159265 / 30.;
static const l_float32  ANGLE2 = 3.14159265 / 7.;

void RotateTest(PIX *pixs, l_float32 scale, L_REGPARAMS *rp);


int main(int    argc,
         char **argv)
{
PIX          *pixs, *pixd;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_stderr("Test binary image:\n");
    pixs = pixRead(BINARY_IMAGE);
    RotateTest(pixs, 1.0, rp);
    pixDestroy(&pixs);

    lept_stderr("Test 2 bpp cmapped image with filled cmap:\n");
    pixs = pixRead(TWO_BPP_IMAGE);
    RotateTest(pixs, 1.0, rp);
    pixDestroy(&pixs);

    lept_stderr("Test 4 bpp cmapped image with unfilled cmap:\n");
    pixs = pixRead(FOUR_BPP_IMAGE1);
    RotateTest(pixs, 1.0, rp);
    pixDestroy(&pixs);

    lept_stderr("Test 4 bpp cmapped image with filled cmap:\n");
    pixs = pixRead(FOUR_BPP_IMAGE2);
    RotateTest(pixs, 1.0, rp);
    pixDestroy(&pixs);

    lept_stderr("Test 8 bpp grayscale image:\n");
    pixs = pixRead(EIGHT_BPP_IMAGE);
    RotateTest(pixs, 1.0, rp);
    pixDestroy(&pixs);

    lept_stderr("Test 8 bpp grayscale cmap image:\n");
    pixs = pixRead(EIGHT_BPP_CMAP_IMAGE1);
    RotateTest(pixs, 1.0, rp);
    pixDestroy(&pixs);

    lept_stderr("Test 8 bpp color cmap image:\n");
    pixs = pixRead(EIGHT_BPP_CMAP_IMAGE2);
    pixd = pixOctreeColorQuant(pixs, 200, 0);
    RotateTest(pixd, 0.5, rp);
    pixDestroy(&pixs);
    pixDestroy(&pixd);

    lept_stderr("Test rgb image:\n");
    pixs = pixRead(RGB_IMAGE);
    RotateTest(pixs, 0.25, rp);
    pixDestroy(&pixs);

    return regTestCleanup(rp);
}


void
RotateTest(PIX          *pixs,
           l_float32     scale,
           L_REGPARAMS  *rp)
{
l_int32   w, h, d, outformat;
PIX      *pix1, *pix2, *pix3, *pixd;
PIXA     *pixa;

    pixGetDimensions(pixs, &w, &h, &d);
    outformat = (d == 8 || d == 32) ? IFF_JFIF_JPEG : IFF_PNG;

    pixa = pixaCreate(0);
    pix1 = pixRotate(pixs, ANGLE1, L_ROTATE_SHEAR, L_BRING_IN_WHITE, w, h);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate(pixs, ANGLE1, L_ROTATE_SHEAR, L_BRING_IN_BLACK, w, h);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix1 = pixRotate(pixs, ANGLE1, L_ROTATE_SHEAR, L_BRING_IN_WHITE, 0, 0);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate(pixs, ANGLE1, L_ROTATE_SHEAR, L_BRING_IN_BLACK, 0, 0);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix1 = pixRotate(pixs, ANGLE2, L_ROTATE_SHEAR, L_BRING_IN_WHITE, w, h);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate(pixs, ANGLE2, L_ROTATE_SHEAR, L_BRING_IN_BLACK, w, h);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix1 = pixRotate(pixs, ANGLE2, L_ROTATE_SHEAR, L_BRING_IN_WHITE, 0, 0);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate(pixs, ANGLE2, L_ROTATE_SHEAR, L_BRING_IN_BLACK, 0, 0);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixd = pixaDisplayTiledInColumns(pixa, 2, scale, 20, 0);
    regTestWritePixAndCheck(rp, pixd, outformat);
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    pixa = pixaCreate(0);
    pix1 = pixRotate(pixs, ANGLE2, L_ROTATE_SAMPLING, L_BRING_IN_WHITE, w, h);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate(pixs, ANGLE2, L_ROTATE_SAMPLING, L_BRING_IN_BLACK, w, h);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix1 = pixRotate(pixs, ANGLE2, L_ROTATE_SAMPLING, L_BRING_IN_WHITE, 0, 0);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate(pixs, ANGLE2, L_ROTATE_SAMPLING, L_BRING_IN_BLACK, 0, 0);
    pixaAddPix(pixa, pix2, L_INSERT);

    if (pixGetDepth(pixs) == 1)
        pix1 = pixScaleToGray2(pixs);
    else
        pix1 = pixClone(pixs);
    pix2 = pixRotate(pix1, ANGLE2, L_ROTATE_AREA_MAP, L_BRING_IN_WHITE, w, h);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixRotate(pix1, ANGLE2, L_ROTATE_AREA_MAP, L_BRING_IN_BLACK, w, h);
    pixaAddPix(pixa, pix3, L_INSERT);
    pix2 = pixRotate(pix1, ANGLE2, L_ROTATE_AREA_MAP, L_BRING_IN_WHITE, 0, 0);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixRotate(pix1, ANGLE2, L_ROTATE_AREA_MAP, L_BRING_IN_BLACK, 0, 0);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix1);
    pixd = pixaDisplayTiledInColumns(pixa, 2, scale, 20, 0);
    regTestWritePixAndCheck(rp, pixd, outformat);
    pixDisplayWithTitle(pixd, 600, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    return;
}
