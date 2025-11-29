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
 * grayfill_reg.c
 *
 *   Regression test for gray filling operations
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void PixTestEqual(L_REGPARAMS *rp, PIX *pixs1, PIX *pixs2, PIX *pixm,
                  l_int32 set, l_int32 connectivity);

int main(int    argc,
         char **argv)
{
l_int32       i, j;
PIX          *pixm, *pixmi, *pixs1, *pixs1_8, *pix1;
PIX          *pixs2, *pixs2_8, *pixs3, *pixs3_8;
PIX          *pixb1, *pixb2, *pixb3, *pixmin;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Mask */
    pixm = pixCreate(200, 200, 8);
    for (i = 0; i < 200; i++)
        for (j = 0; j < 200; j++)
            pixSetPixel(pixm, j, i, 20 + L_ABS((100 - i) * (100 - j)) / 50);
    pixmi = pixInvert(NULL, pixm);

        /* Seed1 */
    pixs1 = pixCreate(200, 200, 8);
    for (i = 99; i <= 101; i++)
        for (j = 99; j <= 101; j++)
            pixSetPixel(pixs1, j, i, 50 - i/100 - j/100);
    pixs1_8 = pixCopy(NULL, pixs1);

        /* Seed2 */
    pixs2 = pixCreate(200, 200, 8);
    for (i = 99; i <= 101; i++)
        for (j = 99; j <= 101; j++)
            pixSetPixel(pixs2, j, i, 205 - i/100 - j/100);
    pixs2_8 = pixCopy(NULL, pixs2);

        /* Inverse grayscale fill */
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixm, L_COPY);
    regTestWritePixAndCheck(rp, pixm, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pixs1, L_COPY);
    regTestWritePixAndCheck(rp, pixs1, IFF_PNG);  /* 1 */
    pixSeedfillGrayInv(pixs1, pixm, 4);
    pixSeedfillGrayInv(pixs1_8, pixm, 8);
    pixaAddPix(pixa, pixs1, L_COPY);
    regTestWritePixAndCheck(rp, pixs1, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pixs1_8, L_COPY);
    regTestWritePixAndCheck(rp, pixs1_8, IFF_PNG);  /* 3 */
    pixb1 = pixThresholdToBinary(pixs1, 20);
    pixaAddPix(pixa, pixb1, L_COPY);
    regTestWritePixAndCheck(rp, pixb1, IFF_PNG);  /* 4 */
    pixCombineMasked(pixs1, pixm, pixb1);
    pixaAddPix(pixa, pixs1, L_COPY);
    regTestWritePixAndCheck(rp, pixs1, IFF_PNG);  /* 5 */
    pix1 = pixaDisplayTiledInColumns(pixa, 6, 1.0, 15, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pix1, 100, 0, "inverse gray fill", rp->display);
    pixDestroy(&pixs1);
    pixDestroy(&pixs1_8);
    pixDestroy(&pixb1);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* Standard grayscale fill */
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixmi, L_COPY);
    regTestWritePixAndCheck(rp, pixmi, IFF_PNG);  /* 7 */
    pixaAddPix(pixa, pixs2, L_COPY);
    regTestWritePixAndCheck(rp, pixs2, IFF_PNG);  /* 8 */
    pixSeedfillGray(pixs2, pixmi, 4);
    pixSeedfillGray(pixs2_8, pixmi, 8);
    pixaAddPix(pixa, pixs2, L_COPY);
    regTestWritePixAndCheck(rp, pixs2, IFF_PNG);  /* 9 */
    pixaAddPix(pixa, pixs2_8, L_COPY);
    regTestWritePixAndCheck(rp, pixs2_8, IFF_PNG);  /* 10 */
    pixb2 = pixThresholdToBinary(pixs2, 205);
    regTestWritePixAndCheck(rp, pixb2, IFF_PNG);  /* 11 */
    pixaAddPix(pixa, pixb2, L_INSERT);
    pix1 = pixaDisplayTiledInColumns(pixa, 5, 1.0, 15, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pix1, 100, 200, "standard gray fill", rp->display);
    pixDestroy(&pixs2);
    pixDestroy(&pixs2_8);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* Basin fill from minima as seed */
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixm, L_COPY);
    regTestWritePixAndCheck(rp, pixm, IFF_PNG);  /* 13 */
    pixLocalExtrema(pixm, 0, 0, &pixmin, NULL);
    pixaAddPix(pixa, pixmin, L_COPY);
    regTestWritePixAndCheck(rp, pixmin, IFF_PNG);  /* 14 */
    pixs3 = pixSeedfillGrayBasin(pixmin, pixm, 30, 4);
    pixs3_8 = pixSeedfillGrayBasin(pixmin, pixm, 30, 8);
    pixaAddPix(pixa, pixs3, L_INSERT);
    regTestWritePixAndCheck(rp, pixs3, IFF_PNG);  /* 15 */
    pixaAddPix(pixa, pixs3_8, L_INSERT);
    regTestWritePixAndCheck(rp, pixs3_8, IFF_PNG);  /* 15 */
    pixb3 = pixThresholdToBinary(pixs3, 60);
    regTestWritePixAndCheck(rp, pixb3, IFF_PNG);  /* 17 */
    pixaAddPix(pixa, pixb3, L_INSERT);
    pix1 = pixaDisplayTiledInColumns(pixa, 5, 1.0, 15, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 18 */
    pixDisplayWithTitle(pix1, 100, 400, "gray fill form seed", rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* Compare hybrid and iterative gray seedfills */
    pixs1 = pixCopy(NULL, pixm);
    pixs2 = pixCopy(NULL, pixm);
    pixAddConstantGray(pixs1, -30);
    pixAddConstantGray(pixs2, 60);

    PixTestEqual(rp, pixs1, pixs2, pixm, 1, 4);  /* 19 - 22 */
    PixTestEqual(rp, pixs1, pixs2, pixm, 2, 8);  /* 23 - 26 */
    PixTestEqual(rp, pixs2, pixs1, pixm, 3, 4);  /* 27 - 30 */
    PixTestEqual(rp, pixs2, pixs1, pixm, 4, 8);  /* 31 - 34 */
    pixDestroy(&pixs1);
    pixDestroy(&pixs2);

    pixDestroy(&pixm);
    pixDestroy(&pixmi);
    pixDestroy(&pixmin);
    return regTestCleanup(rp);;
}

void
PixTestEqual(L_REGPARAMS  *rp,
             PIX          *pixs1,
             PIX          *pixs2,
             PIX          *pixm,
             l_int32       set,
             l_int32       connectivity)
{
PIX  *pixc11, *pixc12, *pixc21, *pixc22, *pixmi;

    pixmi = pixInvert(NULL, pixm);
    pixc11 = pixCopy(NULL, pixs1);
    pixc12 = pixCopy(NULL, pixs1);
    pixc21 = pixCopy(NULL, pixs2);
    pixc22 = pixCopy(NULL, pixs2);

        /* Test inverse seed filling */
    pixSeedfillGrayInv(pixc11, pixm, connectivity);
    regTestWritePixAndCheck(rp, pixc11, IFF_PNG);  /* '1' */
    pixSeedfillGrayInvSimple(pixc12, pixm, connectivity);
    regTestComparePix(rp, pixc11, pixc12);  /* '2' */

        /* Test seed filling */
    pixSeedfillGray(pixc21, pixm, connectivity);
    regTestWritePixAndCheck(rp, pixc21, IFF_PNG);  /* '3' */
    pixSeedfillGraySimple(pixc22, pixm, connectivity);
    regTestComparePix(rp, pixc21, pixc22);  /* '4' */

       /* Display the filling results */
/*    pixDisplay(pixc11, 220 * (set - 1), 100);
    pixDisplay(pixc21, 220 * (set - 1), 320); */

    pixDestroy(&pixmi);
    pixDestroy(&pixc11);
    pixDestroy(&pixc12);
    pixDestroy(&pixc21);
    pixDestroy(&pixc22);
    return;
}


