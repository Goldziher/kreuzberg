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
 * blend5_reg.c
 *
 *    Regression test for these functions in blend.c:
 *      -  pixSnapColor(): used here to color the background on images
 *                         in index.html
 *      -  pixLinearEdgeFade()
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_uint32   LEPTONICA_YELLOW = 0xffffe400;

int main(int    argc,
         char **argv)
{
l_uint32      val32;
PIX          *pixs, *pix1, *pix2;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(0);

        /* First, snap the color directly on the input rgb image. */
    pixs = pixRead("Leptonica.jpg");
    pixaAddPix(pixa, pixs, L_COPY);
    pixDisplayWithTitle(pixs, 0, 0, NULL, rp->display);
    pix1 = pixSnapColor(NULL, pixs, 0xffffff00, LEPTONICA_YELLOW, 30);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 0 */
    pixDisplayWithTitle(pix1, 480, 0, NULL, rp->display);

        /* Then make a colormapped version and snap the color */
    pix1 = pixOctreeQuantNumColors(pixs, 250, 0);
    pixaAddPix(pixa, pix1, L_COPY);
    pixSnapColor(pix1, pix1, 0xffffff00, LEPTONICA_YELLOW, 30);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix1, 880, 0, NULL, rp->display);
    pixDestroy(&pixs);

        /* Set the background of the google searchbox to yellow.
         * The input image is colormapped with all 256 colors used. */
    pixs = pixRead("google-searchbox.png");
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 0, 200, NULL, rp->display);
    pix1 = pixSnapColor(NULL, pixs, 0xffffff00, LEPTONICA_YELLOW, 30);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 220, 200, NULL, rp->display);

        /* A couple of more, setting pixels near white to strange colors */
    pixs = pixRead("weasel4.11c.png");
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplayWithTitle(pixs, 0, 300, NULL, rp->display);
    pix1 = pixSnapColor(NULL, pixs, 0xfefefe00, 0x80800000, 50);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix1, 200, 300, NULL, rp->display);

    pixs = pixRead("wyom.jpg");
    pix1 = pixFixedOctcubeQuant256(pixs, 0);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix1, 0, 450, NULL, rp->display);
    pix2 = pixSnapColor(NULL, pix1, 0xf0f0f000, 0x80008000, 100);
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix2, 900, 450, NULL, rp->display);
    pixDestroy(&pixs);

        /* --- Display results --- */
    pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 6 */
    pixDisplayWithTitle(pix1, 500, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* Test linear fade to black */
    composeRGBPixel(90, 170, 145, &val32);
    pix1 = pixCreate(300, 300, 32);
    pixSetAllArbitrary(pix1, val32);
    pixLinearEdgeFade(pix1, L_FROM_LEFT, L_BLEND_TO_BLACK, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_RIGHT, L_BLEND_TO_BLACK, 0.5, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 7 */
    pixDisplayWithTitle(pix1, 900, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pix1 = pixCreate(300, 300, 32);
    pixSetAllArbitrary(pix1, val32);
    pixLinearEdgeFade(pix1, L_FROM_TOP, L_BLEND_TO_BLACK, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_BOT, L_BLEND_TO_BLACK, 0.5, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 8 */
    pixDisplayWithTitle(pix1, 1250, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pix1 = pixCreate(300, 300, 32);
    pixSetAllArbitrary(pix1, val32);
    pixLinearEdgeFade(pix1, L_FROM_LEFT, L_BLEND_TO_BLACK, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_RIGHT, L_BLEND_TO_BLACK, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_TOP, L_BLEND_TO_BLACK, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_BOT, L_BLEND_TO_BLACK, 0.5, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 9 */
    pixDisplayWithTitle(pix1, 1600, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pix1 = pixCreate(300, 300, 8);   /* 8 bpp */
    pixSetAll(pix1);
    pixLinearEdgeFade(pix1, L_FROM_LEFT, L_BLEND_TO_BLACK, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_RIGHT, L_BLEND_TO_BLACK, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_TOP, L_BLEND_TO_BLACK, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_BOT, L_BLEND_TO_BLACK, 0.5, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10 */
    pixDisplayWithTitle(pix1, 1950, 0, NULL, rp->display);
    pixDestroy(&pix1);

        /* Test linear fade to white */
    composeRGBPixel(170, 110, 200, &val32);
    pix1 = pixCreate(300, 300, 32);
    pixSetAllArbitrary(pix1, val32);
    pixLinearEdgeFade(pix1, L_FROM_LEFT, L_BLEND_TO_WHITE, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_RIGHT, L_BLEND_TO_WHITE, 0.5, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 11 */
    pixDisplayWithTitle(pix1, 900, 380, NULL, rp->display);
    pixDestroy(&pix1);
    pix1 = pixCreate(300, 300, 32);
    pixSetAllArbitrary(pix1, val32);
    pixLinearEdgeFade(pix1, L_FROM_TOP, L_BLEND_TO_WHITE, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_BOT, L_BLEND_TO_WHITE, 0.5, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pix1, 1250, 380, NULL, rp->display);
    pixDestroy(&pix1);
    pix1 = pixCreate(300, 300, 32);
    pixSetAllArbitrary(pix1, val32);
    pixLinearEdgeFade(pix1, L_FROM_LEFT, L_BLEND_TO_WHITE, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_RIGHT, L_BLEND_TO_WHITE, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_TOP, L_BLEND_TO_WHITE, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_BOT, L_BLEND_TO_WHITE, 0.5, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 13 */
    pixDisplayWithTitle(pix1, 1600, 380, NULL, rp->display);
    pixDestroy(&pix1);
    pix1 = pixCreate(300, 300, 8);   /* 8 bpp */
    pixLinearEdgeFade(pix1, L_FROM_LEFT, L_BLEND_TO_WHITE, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_RIGHT, L_BLEND_TO_WHITE, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_TOP, L_BLEND_TO_WHITE, 0.5, 0.8);
    pixLinearEdgeFade(pix1, L_FROM_BOT, L_BLEND_TO_WHITE, 0.5, 0.8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 14 */
    pixDisplayWithTitle(pix1, 1950, 380, NULL, rp->display);
    pixDestroy(&pix1);

    return regTestCleanup(rp);
}
