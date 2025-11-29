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
 * translate_reg.c
 *
 *    Regression test for in-place translation
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   BINARY_IMAGE        "test1.png"
#define   GRAYSCALE_IMAGE     "test8.jpg"
#define   FOUR_BPP_IMAGE      "weasel4.8g.png"
#define   COLORMAP_IMAGE      "dreyfus8.png"
#define   RGB_IMAGE           "marge.jpg"

void TranslateAndSave1(PIXA *pixa, l_int32 depth, PIX *pix,
                       l_int32 xshift, l_int32 yshift);

void TranslateAndSave2(PIXA *pixa, PIX *pix, l_int32 xshift, l_int32 yshift);


int main(int    argc,
         char **argv)
{
BOX          *box;
PIX          *pixs, *pixd;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Set up images */
    pix1 = pixRead("weasel2.4c.png");
    pix2 = pixScaleBySampling(pix1, 3.0, 3.0);
    box = boxCreate(0, 0, 209, 214);
    pixs = pixClipRectangle(pix2, box, NULL);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxDestroy(&box);
    pix1 = pixRemoveColormap(pixs, REMOVE_CMAP_TO_GRAYSCALE);
    pix2 = pixRemoveColormap(pixs, REMOVE_CMAP_TO_FULL_COLOR);
    pix3 = pixConvertTo1(pixs, 128);
    pix4 = pixRotateAM(pix1, 0.25, L_BRING_IN_BLACK);
    pix5 = pixRotateAM(pix1, -0.25, L_BRING_IN_WHITE);
    pix6 = pixRotateAM(pix2, -0.15, L_BRING_IN_BLACK);
    pix7 = pixRotateAM(pix2, +0.15, L_BRING_IN_WHITE);

    pixa = pixaCreate(0);
    TranslateAndSave1(pixa, 32, pixs, 30, 30);
    TranslateAndSave1(pixa, 32, pix1, 35, 20);
    TranslateAndSave1(pixa, 32, pix2, 20, 35);
    TranslateAndSave1(pixa, 32, pix3, 20, 35);
    pixd = pixaDisplayTiledInColumns(pixa, 4, 1.0, 30, 3);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pixd, 0, 0, "trans0", rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    pixa = pixaCreate(0);
    TranslateAndSave1(pixa, 8, pix1, 35, 20);
    TranslateAndSave1(pixa, 8, pix4, 35, 20);
    pixd = pixaDisplayTiledInColumns(pixa, 4, 1.0, 30, 3);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pixd, 250, 0, "trans1", rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    pixa = pixaCreate(0);
    TranslateAndSave2(pixa, pixs, 30, 30);
    TranslateAndSave2(pixa, pix1, 30, 30);
    TranslateAndSave2(pixa, pix2, 35, 20);
    TranslateAndSave2(pixa, pix3, 20, 35);
    TranslateAndSave2(pixa, pix4, 25, 25);
    TranslateAndSave2(pixa, pix5, 25, 25);
    TranslateAndSave2(pixa, pix6, 25, 25);
    TranslateAndSave2(pixa, pix7, 25, 25);
    pixd = pixaDisplayTiledInColumns(pixa, 4, 1.0, 30, 3);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pixd, 500, 0, "trans2", rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    return regTestCleanup(rp);
}


void
TranslateAndSave1(PIXA    *pixa,
                  l_int32  depth,
                  PIX     *pix,
                  l_int32  xshift,
                  l_int32  yshift)
{
PIX  *pix1, *pix2, *pix3, *pix4;

    pix1 = pixTranslate(NULL, pix, xshift, yshift, L_BRING_IN_WHITE);
    pix2 = pixTranslate(NULL, pix, xshift, yshift, L_BRING_IN_BLACK);
    pix3 = pixTranslate(NULL, pix, -xshift, -yshift, L_BRING_IN_WHITE);
    pix4 = pixTranslate(NULL, pix, -xshift, -yshift, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    return;
}

void
TranslateAndSave2(PIXA    *pixa,
                  PIX     *pix,
                  l_int32  xshift,
                  l_int32  yshift)
{
PIX  *pix1, *pix2, *pix3, *pix4;

    pix1 = pixTranslate(NULL, pix, xshift, yshift, L_BRING_IN_WHITE);
    pix2 = pixTranslate(NULL, pix, xshift, yshift, L_BRING_IN_BLACK);
    pix3 = pixTranslate(NULL, pix, -xshift, -yshift, L_BRING_IN_WHITE);
    pix4 = pixTranslate(NULL, pix, -xshift, -yshift, L_BRING_IN_BLACK);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    return;
}
