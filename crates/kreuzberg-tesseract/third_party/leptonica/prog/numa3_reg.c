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
 * numa3_reg.c
 *
 *   Tests:
 *     * rank extraction
 *     * numa-morphology
 *     * find threshold from numa
 *     * insertion in sorted array
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char          buf1[64], buf2[64];
l_int32       i, hw, thresh, same, ival;
l_float32     val, maxval, rank;
BOX          *box1;
NUMA         *na, *nax, *nay, *nap, *nasy, *na1, *na2, *na3, *na4;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "numa3_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/numa3");

    /* -------------------------------------------------------------------*
     *                             Rank extraction                        *
     * -------------------------------------------------------------------*/
        /* Rank extraction with interpolation */
    pixs = pixRead("test8.jpg");
    nasy= pixGetGrayHistogramMasked(pixs, NULL, 0, 0, 1);
    numaMakeRankFromHistogram(0.0, 1.0, nasy, 350, &nax, &nay);
    pix1 = gplotGeneralPix2(nax, nay, GPLOT_LINES, "/tmp/lept/numa3/rank1",
                            "test rank extractor", "pix val", "rank val");
    numaDestroy(&nasy);
    numaDestroy(&nax);
    numaDestroy(&nay);
    pixDestroy(&pixs);

        /* Rank extraction, point by point */
    pixs = pixRead("test8.jpg");
    nap = numaCreate(200);
    pixGetRankValueMasked(pixs, NULL, 0, 0, 2, 0.0, &val, &na);
    for (i = 0; i < 101; i++) {
      rank = 0.01 * i;
      numaHistogramGetValFromRank(na, rank, &val);
      numaAddNumber(nap, val);
    }
    pix2 = gplotGeneralPix1(nap, GPLOT_LINES, "/tmp/lept/numa3/rank2",
                            "rank value", NULL, NULL);
    pixa = pixaCreate(2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    if (rp->display) {
        pixd = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 20, 2);
        pixDisplayWithTitle(pixd, 900, 0, NULL, 1);
        pixDestroy(&pixd);
    }
    pixaDestroy(&pixa);
    numaDestroy(&na);
    numaDestroy(&nap);
    pixDestroy(&pixs);

    /* -------------------------------------------------------------------*
     *                           Numa-morphology                          *
     * -------------------------------------------------------------------*/
    na = numaRead("lyra.5.na");
    pix1 = gplotGeneralPix1(na, GPLOT_LINES, "/tmp/lept/numa3/lyra1",
                            "Original", NULL, NULL);
    na1 = numaErode(na, 21);
    pix2 = gplotGeneralPix1(na1, GPLOT_LINES, "/tmp/lept/numa3/lyra2",
                            "Erosion", NULL, NULL);
    na2 = numaDilate(na, 21);
    pix3 = gplotGeneralPix1(na2, GPLOT_LINES, "/tmp/lept/numa3/lyra3",
                            "Dilation", NULL, NULL);
    na3 = numaOpen(na, 21);
    pix4 = gplotGeneralPix1(na3, GPLOT_LINES, "/tmp/lept/numa3/lyra4",
                            "Opening", NULL, NULL);
    na4 = numaClose(na, 21);
    pix5 = gplotGeneralPix1(na4, GPLOT_LINES, "/tmp/lept/numa3/lyra5",
                            "Closing", NULL, NULL);
    pixa = pixaCreate(2);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 3 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 4 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 5 */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 6 */
    if (rp->display) {
        pixd = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 20, 2);
        pixDisplayWithTitle(pixd, 1200, 0, NULL, 1);
        pixDestroy(&pixd);
    }
    pixaDestroy(&pixa);
    numaDestroy(&na);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    numaDestroy(&na4);
    pixaDestroy(&pixa);

    /* -------------------------------------------------------------------*
     *                   Find threshold from numa                         *
     * -------------------------------------------------------------------*/
    na1 = numaRead("two-peak-histo.na");
    na4 = numaCreate(0);
    pixa = pixaCreate(0);
    for (hw = 2; hw < 21; hw += 2) {
        na2 = numaWindowedMean(na1, hw);  /* smoothing */
        numaGetMax(na2, &maxval, NULL);
        na3 = numaTransform(na2, 0.0, 1.0 / maxval);
        numaFindLocForThreshold(na3, 0, &thresh, NULL);
        numaAddNumber(na4, thresh);
        snprintf(buf1, sizeof(buf1), "/tmp/lept/numa3/histoplot-%d", hw);
        snprintf(buf2, sizeof(buf2), "halfwidth = %d, skip = 20, thresh = %d",
                 hw, thresh);
        pix1 = gplotGeneralPix1(na3, GPLOT_LINES, buf1, buf2, NULL, NULL);
        if (hw == 4 || hw == 20)
            regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 7, 8 */
        pixaAddPix(pixa, pix1, L_INSERT);
        numaDestroy(&na2);
        numaDestroy(&na3);
    }
    numaWrite("/tmp/lept/numa3/threshvals.na", na4);
    regTestCheckFile(rp, "/tmp/lept/numa3/threshvals.na");  /* 9 */
    L_INFO("writing /tmp/lept/numa3/histoplots.pdf\n", "numa3_reg");
    pixaConvertToPdf(pixa, 0, 1.0, L_FLATE_ENCODE, 0,
                     "Effect of smoothing on threshold value",
                     "/tmp/lept/numa3/histoplots.pdf");
    numaDestroy(&na1);
    numaDestroy(&na4);
    pixaDestroy(&pixa);

    pixs = pixRead("lyra.005.jpg");
    box1 = boxCreate(0, 173, 350, 580);
    pix1 = pixClipRectangle(pixs, box1, 0);
    pix2 = pixRotateOrth(pix1, 1);
    pix3 = pixConvertTo8(pix2, 0);
    pixThresholdByHisto(pix3, 1, 0, 0, &ival, &pix4, &na1, &pix5);
    pixa = pixaCreate(4);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix5, L_INSERT);
    pixd = pixaDisplayTiledInColumns(pixa, 1,1.0, 25, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 10 */
    pixDisplayWithTitle(pixd, 0, 500, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pixd);
    boxDestroy(&box1);
    numaDestroy(&na1);
    pixaDestroy(&pixa);

    /* -------------------------------------------------------------------*
     *                      Insertion in a sorted array                   *
     * -------------------------------------------------------------------*/
    na1 = numaCreate(0);
    srand(5);
    numaAddNumber(na1, 27);
    numaAddNumber(na1, 13);
    for (i = 0; i < 70; i++) {
        genRandomIntOnInterval(0, 200, 0, &ival);
        numaAddSorted(na1, ival);
    }
    if (rp->display) numaWriteStderr(na1);
    na2 = numaSort(NULL, na1, L_SORT_INCREASING);
    numaReverse(na2, na2);
    numaSimilar(na1, na2, 0.0, &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 11 */
    numaDestroy(&na1);
    numaDestroy(&na2);

    na1 = numaCreate(0);
    srand(6);
    numaAddNumber(na1, 13);
    numaAddNumber(na1, 27);
    for (i = 0; i < 70; i++) {
        genRandomIntOnInterval(0, 200, 0, &ival);
        numaAddSorted(na1, ival);
    }
    if (rp->display) numaWriteStderr(na1);
    na2 = numaSort(NULL, na1, L_SORT_DECREASING);
    numaReverse(na2, na2);
    numaSimilar(na1, na2, 0.0, &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 12 */
    numaDestroy(&na1);
    numaDestroy(&na2);

    return regTestCleanup(rp);
}
