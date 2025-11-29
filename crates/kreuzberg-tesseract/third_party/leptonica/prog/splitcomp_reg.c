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
 * splitcomp_reg.c
 *
 *   Regression test for splittings of a single component and for an image
 *   composed of several components, using different components and
 *   parameters.  Note that:
 *      (1) All coverings must cover the fg of the mask.
 *      (2) The first set of parameters is small and generates
 *          a proper tiling, covering ONLY the mask fg.
 *      (3) The tilings generated on 90 degree rotated components
 *          are identical (rotated) to those on un-rotated components.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, j, w, h;
l_int32       minsum[5] =    { 2, 40, 50, 50, 70};
l_int32       skipdist[5] =  { 5,  5, 10, 10, 30};
l_int32       delta[5] =     { 2, 10, 10, 25, 40};
l_int32       maxbg[5] =     {10, 15, 10, 20, 40};
BOX          *box1, *box2, *box3, *box4;
BOXA         *boxa;
PIX          *pixs, *pixc, *pixt, *pixd, *pix32;
PIXA         *pixas, *pixad;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Generate and save 1 bpp masks */
    pixas = pixaCreate(0);
    pixs = pixCreate(300, 250, 1);
    pixSetAll(pixs);
    box1 = boxCreate(50, 0, 140, 25);
    box2 = boxCreate(120, 100, 100, 25);
    box3 = boxCreate(75, 170, 80, 20);
    box4 = boxCreate(150, 80, 25, 70);

    pixClearInRect(pixs, box1);
    pixaAddPix(pixas, pixs, L_COPY);
    pixt = pixRotateOrth(pixs, 1);
    pixaAddPix(pixas, pixt, L_INSERT);

    pixClearInRect(pixs, box2);
    pixaAddPix(pixas, pixs, L_COPY);
    pixt = pixRotateOrth(pixs, 1);
    pixaAddPix(pixas, pixt, L_INSERT);

    pixClearInRect(pixs, box3);
    pixaAddPix(pixas, pixs, L_COPY);
    pixt = pixRotateOrth(pixs, 1);
    pixaAddPix(pixas, pixt, L_INSERT);

    pixClearInRect(pixs, box4);
    pixaAddPix(pixas, pixs, L_COPY);
    pixt = pixRotateOrth(pixs, 1);
    pixaAddPix(pixas, pixt, L_INSERT);

    boxDestroy(&box1);
    boxDestroy(&box2);
    boxDestroy(&box3);
    boxDestroy(&box4);
    pixDestroy(&pixs);

        /* Do 5 splittings on each of the 8 masks */
    pixad = pixaCreate(0);
    for (j = 0; j < 8; j++) {
        pixt = pixaGetPix(pixas, j, L_CLONE);
        pixGetDimensions(pixt, &w, &h, NULL);
        pix32 = pixCreate(w, h, 32);
        pixSetAll(pix32);
        pixPaintThroughMask(pix32, pixt, 0, 0, 0xc0c0c000);
        pixaAddPix(pixad, pix32, L_INSERT);
        for (i = 0; i < 5; i++) {
            pixc = pixCopy(NULL, pix32);
            boxa = pixSplitComponentIntoBoxa(pixt, NULL, minsum[i], skipdist[i],
                                             delta[i], maxbg[i], 0, 1);
/*            boxaWriteStderr(boxa); */
            pixd = pixBlendBoxaRandom(pixc, boxa, 0.4);
            pixRenderBoxaArb(pixd, boxa, 2, 255, 0, 0);
            pixaAddPix(pixad, pixd, L_INSERT);
            pixDestroy(&pixc);
            boxaDestroy(&boxa);
        }
        pixDestroy(&pixt);
    }

        /* Display results */
    pixd = pixaDisplayTiledInColumns(pixad, 6, 1.0, 30, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixad);

        /* Put the 8 masks all together, and split 5 ways */
    pixad = pixaCreate(0);
    pixs = pixaDisplayOnLattice(pixas, 325, 325, NULL, NULL);
    pixGetDimensions(pixs, &w, &h, NULL);
    pix32 = pixCreate(w, h, 32);
    pixSetAll(pix32);
    pixPaintThroughMask(pix32, pixs, 0, 0, 0xc0c0c000);
    pixaAddPix(pixad, pix32, L_INSERT);
    for (i = 0; i < 5; i++) {
        pixc = pixCopy(NULL, pix32);
        boxa = pixSplitIntoBoxa(pixs, minsum[i], skipdist[i],
                                delta[i], maxbg[i], 0, 1);
/*        boxaWriteStderr(boxa); */
        pixd = pixBlendBoxaRandom(pixc, boxa, 0.4);
        pixRenderBoxaArb(pixd, boxa, 2, 255, 0, 0);
        pixaAddPix(pixad, pixd, L_INSERT);
        pixDestroy(&pixc);
        boxaDestroy(&boxa);
    }
    pixDestroy(&pixs);

        /* Display results */
    pixd = pixaDisplayTiledInColumns(pixad, 6, 1.0, 30, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pixd, 600, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixad);

    pixaDestroy(&pixas);
    return regTestCleanup(rp);
}
