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
 *  boxa2_reg.c
 *
 *  Low-level statistical operations that can be used to identify anomalous
 *  sized boxes in a boxa, where the widths and heights of the boxes
 *  are expected to be similar.
 *
 *  This tests a number of operations on boxes in a boxa, including
 *  separating them into subsets of different parity.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
l_int32       i, n, val, ne, no, nbins, minw, maxw, minh, maxh;
l_int32       mine, mino, maxe, maxo;
l_int32       w_diff, h_diff, median_w_diff, median_h_diff;
l_int32       noutw, nouth;
l_float32     medwe, medhe, medwo, medho;
BOXA         *boxa1, *boxa2, *boxae, *boxao;
NUMA         *na1, *nawe, *nahe, *nawo, *naho;
NUMA         *nadiffw, *nadiffh;  /* diff from median w and h */
NUMA         *naiw, *naih;  /* indicator arrays for small outlier dimensions */
NUMA         *narbwe, *narbhe, *narbwo, *narbho;  /* rank-binned w and h */
PIX          *pix1;
PIXA         *pixa1;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/boxa");
    boxa1 = boxaRead("boxa4.ba");

        /* Fill invalid boxes */
    n = boxaGetCount(boxa1);
    na1 = boxaFindInvalidBoxes(boxa1);
    if (na1)
        boxa2 = boxaFillSequence(boxa1, L_USE_SAME_PARITY_BOXES, 0);
    else
        boxa2 = boxaCopy(boxa1, L_CLONE);
    boxaDestroy(&boxa1);

        /* Get the widths and heights for even and odd parity */
    boxaSplitEvenOdd(boxa2, 0, &boxae, &boxao);
    boxaGetSizes(boxae, &nawe, &nahe);
    boxaGetSizes(boxao, &nawo, &naho);
    boxaDestroy(&boxa2);

        /* Find the medians */
    numaGetMedian(nawe, &medwe);
    numaGetMedian(nahe, &medhe);
    numaGetMedian(nawo, &medwo);
    numaGetMedian(naho, &medho);

        /* Find the median even/odd differences for width and height */
    median_w_diff = L_ABS(medwe - medwo);
    median_h_diff = L_ABS(medhe - medho);
    regTestCompareValues(rp, 210, median_w_diff, 0.0);  /* 0 */
    regTestCompareValues(rp, 15, median_h_diff, 0.0);  /* 1 */
    if (rp->display) {
        lept_stderr("diff of e/o median widths = %d\n", median_w_diff);
        lept_stderr("diff of e/o median heights = %d\n", median_h_diff);
    }

        /* Find the differences of box width and height from the median */
    nadiffw = numaMakeConstant(0, n);
    nadiffh = numaMakeConstant(0, n);
    ne = numaGetCount(nawe);
    no = numaGetCount(nawo);
    for (i = 0; i < ne; i++) {
        numaGetIValue(nawe, i, &val);
        numaSetValue(nadiffw, 2 * i, L_ABS(val - medwe));
        numaGetIValue(nahe, i, &val);
        numaSetValue(nadiffh, 2 * i, L_ABS(val - medhe));
    }
    for (i = 0; i < no; i++) {
        numaGetIValue(nawo, i, &val);
        numaSetValue(nadiffw, 2 * i + 1, L_ABS(val - medwo));
        numaGetIValue(naho, i, &val);
        numaSetValue(nadiffh, 2 * i + 1, L_ABS(val - medho));
    }

        /* Don't count invalid boxes; set the diffs to 0 for them */
    if (na1) {
        for (i = 0; i < n; i++) {
            numaGetIValue(na1, i, &val);
            if (val == 1) {
                numaSetValue(nadiffw, i, 0);
                numaSetValue(nadiffh, i, 0);
            }
        }
    }

        /* Make an indicator array for boxes that differ from the
         * median by more than a threshold value for outliers */
    naiw = numaMakeThresholdIndicator(nadiffw, 90, L_SELECT_IF_GT);
    naih = numaMakeThresholdIndicator(nadiffh, 90, L_SELECT_IF_GT);
    numaGetCountRelativeToZero(naiw, L_GREATER_THAN_ZERO, &noutw);
    numaGetCountRelativeToZero(naih, L_GREATER_THAN_ZERO, &nouth);
    regTestCompareValues(rp, 24, noutw, 0.0);  /* 2 */
    regTestCompareValues(rp, 0, nouth, 0.0);  /* 3 */
    if (rp->display)
        lept_stderr("num width outliers = %d, num height outliers = %d\n",
                    noutw, nouth);
    numaDestroy(&nadiffw);
    numaDestroy(&nadiffh);
    numaDestroy(&naiw);
    numaDestroy(&naih);

        /* Find the rank bins for width and height */
    nbins = L_MAX(5, ne / 50);  // up to 50 pages/bin
    numaGetRankBinValues(nawe, nbins, &narbwe);
    numaGetRankBinValues(nawo, nbins, &narbwo);
    numaGetRankBinValues(nahe, nbins, &narbhe);
    numaGetRankBinValues(naho, nbins, &narbho);
    numaDestroy(&nawe);
    numaDestroy(&nawo);
    numaDestroy(&nahe);
    numaDestroy(&naho);

        /* Find min and max binned widths and heights; get the max diffs */
    numaGetIValue(narbwe, 0, &mine);
    numaGetIValue(narbwe, nbins - 1, &maxe);
    numaGetIValue(narbwo, 0, &mino);
    numaGetIValue(narbwo, nbins - 1, &maxo);
    minw = L_MIN(mine, mino);
    maxw = L_MAX(maxe, maxo);
    w_diff = maxw - minw;
    numaGetIValue(narbhe, 0, &mine);
    numaGetIValue(narbhe, nbins - 1, &maxe);
    numaGetIValue(narbho, 0, &mino);
    numaGetIValue(narbho, nbins - 1, &maxo);
    minh = L_MIN(mine, mino);
    maxh = L_MAX(maxe, maxo);
    h_diff = maxh - minh;
    numaDestroy(&narbwe);
    numaDestroy(&narbhe);
    numaDestroy(&narbwo);
    numaDestroy(&narbho);
    regTestCompareValues(rp, 409, w_diff, 0.0);  /* 4 */
    regTestCompareValues(rp, 54, h_diff, 0.0);  /* 5 */
    if (rp->display)
        lept_stderr("Binned rank results: w_diff = %d, h_diff = %d\n",
                    w_diff, h_diff);

        /* Plot the results */
    if (noutw > 0 || nouth > 0) {
        pixa1 = pixaCreate(2);
        boxaPlotSizes(boxae, "even", NULL, NULL, &pix1);
        pixaAddPix(pixa1, pix1, L_INSERT);
        boxaPlotSizes(boxao, "odd", NULL, NULL, &pix1);
        pixaAddPix(pixa1, pix1, L_INSERT);
        pix1 = pixaDisplayTiledInRows(pixa1, 32, 1500, 1.0, 0, 30, 2);
        regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 6 */
        pixDisplayWithTitle(pix1, 100, 100, NULL, rp->display);
        pixDestroy(&pix1);
        pixaDestroy(&pixa1);
    }

    boxaDestroy(&boxae);
    boxaDestroy(&boxao);
    return regTestCleanup(rp);
}
