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
 * adaptmap_reg.c
 *
 *   Regression test demonstrating adaptive mappings in both gray and color
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

   /* Location of image region in wet-day.jpg */
static const l_int32  XS = 151;
static const l_int32  YS = 225;
static const l_int32  WS = 913;
static const l_int32  HS = 1285;

static const l_int32  SIZE_X = 10;
static const l_int32  SIZE_Y = 30;
static const l_int32  BINTHRESH = 50;
static const l_int32  MINCOUNT = 30;

static const l_int32  BGVAL = 200;
static const l_int32  SMOOTH_X = 2;
static const l_int32  SMOOTH_Y = 1;

int main(int    argc,
         char **argv)
{
l_int32       w, h;
PIX          *pixs, *pixg, *pixim, *pixgm, *pixmi, *pix1, *pix2;
PIX          *pixmr, *pixmg, *pixmb, *pixmri, *pixmgi, *pixmbi;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/adapt");  // REMOVE?

    pixs = pixRead("wet-day.jpg");
    pixa = pixaCreate(0);
    pixg = pixConvertRGBToGray(pixs, 0.33, 0.34, 0.33);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixaAddPix(pixa, pixg, L_INSERT);
    pixGetDimensions(pixs, &w, &h, NULL);

        /* Process in grayscale */
    startTimer();
    pixim = pixCreate(w, h, 1);
    pixRasterop(pixim, XS, YS, WS, HS, PIX_SET, NULL, 0, 0);
    pixGetBackgroundGrayMap(pixg, pixim, SIZE_X, SIZE_Y,
                            BINTHRESH, MINCOUNT, &pixgm);
    lept_stderr("Time for gray adaptmap gen: %7.3f\n", stopTimer());
    regTestWritePixAndCheck(rp, pixgm, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pixgm, L_INSERT);

    startTimer();
    pixmi = pixGetInvBackgroundMap(pixgm, BGVAL, SMOOTH_X, SMOOTH_Y);
    lept_stderr("Time for gray inv map generation: %7.3f\n", stopTimer());
    regTestWritePixAndCheck(rp, pixmi, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pixmi, L_INSERT);

    startTimer();
    pix1 = pixApplyInvBackgroundGrayMap(pixg, pixmi, SIZE_X, SIZE_Y);
    lept_stderr("Time to apply gray inv map: %7.3f\n", stopTimer());
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 2 */
    pixaAddPix(pixa, pix1, L_INSERT);

    pix2 = pixGammaTRCMasked(NULL, pix1, pixim, 1.0, 0, 190);
    pixInvert(pixim, pixim);
    pixGammaTRCMasked(pix2, pix2, pixim, 1.0, 60, 190);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 3 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pixim);

        /* Process in color */
    startTimer();
    pixim = pixCreate(w, h, 1);
    pixRasterop(pixim, XS, YS, WS, HS, PIX_SET, NULL, 0, 0);
    pixGetBackgroundRGBMap(pixs, pixim, NULL, SIZE_X, SIZE_Y,
                           BINTHRESH, MINCOUNT,
                           &pixmr, &pixmg, &pixmb);
    lept_stderr("Time for color adaptmap gen: %7.3f\n", stopTimer());
    regTestWritePixAndCheck(rp, pixmr, IFF_PNG);  /* 4 */
    regTestWritePixAndCheck(rp, pixmg, IFF_PNG);  /* 5 */
    regTestWritePixAndCheck(rp, pixmb, IFF_PNG);  /* 6 */
    pixaAddPix(pixa, pixmr, L_INSERT);
    pixaAddPix(pixa, pixmg, L_INSERT);
    pixaAddPix(pixa, pixmb, L_INSERT);

    startTimer();
    pixmri = pixGetInvBackgroundMap(pixmr, BGVAL, SMOOTH_X, SMOOTH_Y);
    pixmgi = pixGetInvBackgroundMap(pixmg, BGVAL, SMOOTH_X, SMOOTH_Y);
    pixmbi = pixGetInvBackgroundMap(pixmb, BGVAL, SMOOTH_X, SMOOTH_Y);
    lept_stderr("Time for color inv map generation: %7.3f\n", stopTimer());
    regTestWritePixAndCheck(rp, pixmri, IFF_PNG);  /* 7 */
    regTestWritePixAndCheck(rp, pixmgi, IFF_PNG);  /* 8 */
    regTestWritePixAndCheck(rp, pixmbi, IFF_PNG);  /* 9 */
    pixaAddPix(pixa, pixmri, L_INSERT);
    pixaAddPix(pixa, pixmgi, L_INSERT);
    pixaAddPix(pixa, pixmbi, L_INSERT);

    startTimer();
    pix1 = pixApplyInvBackgroundRGBMap(pixs, pixmri, pixmgi, pixmbi,
                                       SIZE_X, SIZE_Y);
    lept_stderr("Time to apply color inv maps: %7.3f\n", stopTimer());
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 10 */
    pixaAddPix(pixa, pix1, L_INSERT);

    pix2 = pixGammaTRCMasked(NULL, pix1, pixim, 1.0, 0, 190);
    pixInvert(pixim, pixim);
    pixGammaTRCMasked(pix2, pix2, pixim, 1.0, 60, 190);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 11 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pixim);

        /* Process at higher level in color */
    startTimer();
    pixim = pixCreate(w, h, 1);
    pixRasterop(pixim, XS, YS, WS, HS, PIX_SET, NULL, 0, 0);
    pix1 = pixBackgroundNorm(pixs, pixim, NULL, 5, 10, BINTHRESH, 20,
                             BGVAL, SMOOTH_X, SMOOTH_Y);
    lept_stderr("Time for bg normalization: %7.3f\n", stopTimer());
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 12 */
    pixaAddPix(pixa, pix1, L_INSERT);

    pix2 = pixGammaTRCMasked(NULL, pix1, pixim, 1.0, 0, 190);
    pixInvert(pixim, pixim);
    pixGammaTRCMasked(pix2, pix2, pixim, 1.0, 60, 190);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 13 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pixim);

        /* Display results */
    pix1 = pixaDisplayTiledAndScaled(pixa, 32, 400, 4, 0, 20, 2);
    pixWrite("/tmp/lept/adapt/results.jpg", pix1, IFF_JFIF_JPEG);
    pixDisplayWithTitle(pix1, 50, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

        /* Check pixFillMapHoles() */
    pixa = pixaCreate(3);
    pix1 = pixRead("weasel8.png");  /* use this as the map */
    pixGammaTRC(pix1, pix1, 1.0, 0, 270);  /* darken white pixels */
    pixaAddPix(pixa, pix1, L_COPY);
    pixGetDimensions(pix1, &w, &h, NULL);
    pixRasterop(pix1, 0, 0, 5, h, PIX_SET, NULL, 0, 0);  /* add white holes */
    pixRasterop(pix1, 20, 0, 2, h, PIX_SET, NULL, 0, 0);
    pixRasterop(pix1, 40, 0, 3, h, PIX_SET, NULL, 0, 0);
    pixRasterop(pix1, 0, 0, w, 3, PIX_SET, NULL, 0, 0);
    pixRasterop(pix1, 0, 15, w, 3, PIX_SET, NULL, 0, 0);
    pixRasterop(pix1, 0, 35, w, 2, PIX_SET, NULL, 0, 0);
    pixaAddPix(pixa, pix1, L_COPY);
    pixFillMapHoles(pix1, w, h, L_FILL_WHITE);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 1);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 14 */
    pixDisplayWithTitle(pix2, 50, 850, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix2);

        /* An even simpler check of pixFillMapHoles() */
    pixa = pixaCreate(2);
    pix1 = pixCreate(3, 3, 8);
    pixSetPixel(pix1, 1, 0, 128);
    pix2 = pixExpandReplicate(pix1, 25);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixFillMapHoles(pix1, 3, 3, L_FILL_BLACK);
    pix2 = pixExpandReplicate(pix1, 25);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pix1);
    pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 15 */
    pixDisplayWithTitle(pix1, 50, 1000, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);

    return regTestCleanup(rp);
}

