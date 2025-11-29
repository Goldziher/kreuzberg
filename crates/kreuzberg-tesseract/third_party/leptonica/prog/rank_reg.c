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
 * rank_reg.c
 *
 *   Tests grayscale and color rank functions:
 *      (1) pixRankFilterGray()
 *      (1) pixRankFilterRGB()
 *      (2) pixScaleGrayMinMax()
 *      (3) pixScaleGrayRank2()
 *      (3) pixScaleGrayRankCascade()
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, j, w, h;
l_float32     t1, t2;
BOX          *box;
GPLOT        *gplot;
NUMA         *nax, *nay1, *nay2;
PIX          *pixs, *pix0, *pix1, *pix2, *pix3, *pix4;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "rank_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/rank");

    pixs = pixRead("lucasta.150.jpg");
    pixGetDimensions(pixs, &w, &h, NULL);

    startTimer();
    pix1 = pixRankFilterGray(pixs, 15, 15, 0.4);
    t1 = stopTimer();
    lept_stderr("pixRankFilterGray: %7.3f MPix/sec\n", 0.000001 * w * h / t1);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pixs, 0, 0, NULL, rp->display);
    pixDisplayWithTitle(pix1, 600, 0, NULL, rp->display);
    pixDestroy(&pix1);

    /* ---------- Compare grayscale morph with rank operator ---------- */
        /* Get results for dilation */
    startTimer();
    pix1 = pixDilateGray(pixs, 15, 15);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    t2 = stopTimer();
    lept_stderr("Rank filter time = %7.3f, Dilation time =  %7.3f sec\n",
                t1, t2);

        /* Get results for erosion */
    pix2 = pixErodeGray(pixs, 15, 15);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 2 */

        /* Get results using the rank filter for rank = 0.0 and 1.0.
         * Don't use 0.0 or 1.0, because those are dispatched
         * automatically to erosion and dilation! */
    pix3 = pixRankFilterGray(pixs, 15, 15, 0.0001);
    pix4 = pixRankFilterGray(pixs, 15, 15, 0.9999);
    regTestComparePix(rp, pix1, pix4);  /* 3 */
    regTestComparePix(rp, pix2, pix3);  /* 4 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    /* ------------- Timing and filter size experiments --------- */
    box = boxCreate(20, 200, 500, 125);
    pix0 = pixClipRectangle(pixs, box, NULL);
    boxDestroy(&box);
    nax = numaMakeSequence(1, 1, 20);
    nay1 = numaCreate(20);
    nay2 = numaCreate(20);
    gplot = gplotCreate("/tmp/lept/rank/plots", GPLOT_PNG,
                        "sec/MPix vs filter size", "size", "time");
    pixa = pixaCreate(20);
    for (i = 1; i <= 20; i++) {
        t1 = t2 = 0.0;
        for (j = 0; j < 5; j++) {
            startTimer();
            pix1 = pixRankFilterGray(pix0, i, 20 + 1, 0.5);
            t1 += stopTimer();
            pixDestroy(&pix1);
            startTimer();
            pix1 = pixRankFilterGray(pix0, 20 + 1, i, 0.5);
            t2 += stopTimer();
            if (j == 0)
                pixaAddPix(pixa, pix1, L_CLONE);
            pixDestroy(&pix1);
        }
        numaAddNumber(nay1, 1000000. * t1 / (5. * w * h));
        numaAddNumber(nay2, 1000000. * t2 / (5. * w * h));
    }
    gplotAddPlot(gplot, nax, nay1, GPLOT_LINES, "vertical");
    gplotAddPlot(gplot, nax, nay2, GPLOT_LINES, "horizontal");
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    pix1 = pixRead("/tmp/lept/rank/plots.png");
    pixDisplayWithTitle(pix1, 100, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix0);
    numaDestroy(&nax);
    numaDestroy(&nay1);
    numaDestroy(&nay2);

        /* Display tiled */
    pix1 = pixaDisplayTiledAndScaled(pixa, 8, 250, 5, 0, 25, 2);
    pixDisplayWithTitle(pix1, 100, 600, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    pixDestroy(&pixs);

    /* ------------------     Gray tests    ------------------ */
    pixs = pixRead("test8.jpg");
    pixa = pixaCreate(4);
    for (i = 1; i <= 4; i++) {
        pix1 = pixScaleGrayRank2(pixs, i);
        pixaAddPix(pixa, pix1, L_INSERT);
    }
    pix1 = pixaDisplayTiledInRows(pixa, 8, 1500, 1.0, 0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 5 */
    pixDisplayWithTitle(pix1, 100, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

    pixs = pixRead("test24.jpg");
    pix1 = pixConvertRGBToLuminance(pixs);
    pix2 = pixScale(pix1, 1.5, 1.5);
    pixa = pixaCreate(5);
    for (i = 1; i <= 4; i++) {
        for (j = 1; j <= 4; j++) {
            pix3 = pixScaleGrayRankCascade(pix2, i, j, 0, 0);
            pixaAddPix(pixa, pix3, L_INSERT);
        }
    }
    pix4 = pixaDisplayTiledInRows(pixa, 8, 1500, 0.7, 0, 20, 2);
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 6 */
    pixDisplayWithTitle(pix4, 100, 700, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix4);
    pixaDestroy(&pixa);

    /* ---------- Compare color morph with rank operator ---------- */
    pixs = pixRead("wyom.jpg");
    box = boxCreate(400, 220, 300, 250);
    pix0 = pixClipRectangle(pixs, box, NULL);
    boxDestroy(&box);
    pix1 = pixColorMorph(pix0, L_MORPH_DILATE, 11, 11);
    pix2 = pixColorMorph(pix0, L_MORPH_ERODE, 11, 11);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 7 */
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 8 */

        /* Get results using the rank filter for rank = 0.0 and 1.0.
         * Don't use 0.0 or 1.0, because those are dispatched
         * automatically to erosion and dilation! */
    pix3 = pixRankFilter(pix0, 11, 11, 0.0001);
    pix4 = pixRankFilter(pix0, 11, 11, 0.9999);
    regTestComparePix(rp, pix1, pix4);  /* 9 */
    regTestComparePix(rp, pix2, pix3);  /* 10 */
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    /* Show color results for different rank values */
    if (rp->display) {
        pixa = pixaCreate(10);
        pix1 = pixColorMorph(pix0, L_MORPH_ERODE, 13, 13);
        pixaAddPix(pixa, pix1, L_INSERT);
        for (i = 0; i <= 10; i++) {
            pix1 = pixRankFilter(pix0, 13, 13, 0.1 * i);
            pixaAddPix(pixa, pix1, L_INSERT);
        }
        pix1 = pixColorMorph(pix0, L_MORPH_DILATE, 13, 13);
        pixaAddPix(pixa, pix1, L_INSERT);
        pix1 = pixaDisplayTiledAndScaled(pixa, 32, 400, 3, 0, 25, 2);
        pixDisplayWithTitle(pix1, 500, 0, NULL, 1);
        pixaDestroy(&pixa);
        pixDestroy(&pix1);
    }
    pixDestroy(&pix0);

    return regTestCleanup(rp);
}


