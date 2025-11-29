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
 * rankbin_reg.c
 *
 *   Tests rank bin functions:
 *      (1) numaDiscretizeInBins()
 *      (2) numaGetRankBinValues()
 *      (3) pixRankBinByStrip()
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, n, w, h, nbins;
l_uint8      *data;
l_uint32     *carray;
size_t        nbytes;
BOXA         *boxa1, *boxa2, *boxa3;
NUMA         *naindex, *na1, *na2, *na3, *na4;
PIX          *pixs, *pix1, *pix2, *pix3;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "rankbin_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Generate arrays of word widths and heights */
    pixs = pixRead("feyn.tif");
    pix1 = pixReduceRankBinaryCascade(pixs, 1, 0, 0, 0);
    pixGetWordBoxesInTextlines(pix1, 6, 6, 500, 50, &boxa1, &naindex);
    n = boxaGetCount(boxa1);
    na1 = numaCreate(0);
    na2 = numaCreate(0);
    for (i = 0; i < n; i++) {
        boxaGetBoxGeometry(boxa1, i, NULL, NULL, &w, &h);
        numaAddNumber(na1, w);
        numaAddNumber(na2, h);
    }
    boxaDestroy(&boxa1);
    numaDestroy(&naindex);
    pixDestroy(&pix1);

        /* Make the rank bin arrays of average values, with 10 bins */
    pixa = pixaCreate(0);
    numaGetRankBinValues(na1, 10, &na3);
    numaGetRankBinValues(na2, 10, &na4);
    pix1 = gplotSimplePix1(na3, "width vs rank bins (10)");
    pix2 = gplotSimplePix1(na4, "height vs rank bins (10)");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    numaDestroy(&na3);
    numaDestroy(&na4);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Make the rank bin arrays of average values, with 30 bins */
    numaGetRankBinValues(na1, 30, &na3);
    numaGetRankBinValues(na2, 30, &na4);
    pix1 = gplotSimplePix1(na3, "width vs rank bins (30)");
    pix2 = gplotSimplePix1(na4, "height vs rank bins (30)");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 3 */
    numaDestroy(&na3);
    numaDestroy(&na4);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Display results for debugging */
    if (rp->display) {
        pix3 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 25, 0);
        pixDisplayWithTitle(pix3, 0, 0, NULL, 1);
        pixDestroy(&pix3);
    }
    pixaDestroy(&pixa);
    pixDestroy(&pixs);
    numaDestroy(&na1);
    numaDestroy(&na2);

        /* Test pixRankBinByStrip */
    pix1 = pixRead("pancrazi.15.jpg");
    pixa = pixaCreate(3);
    pix2 = pixRankBinByStrip(pix1, L_SCAN_HORIZONTAL, 16, 10, L_SELECT_HUE);
    pix3 = pixExpandReplicate(pix2, 20);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix2);
    pix2 = pixRankBinByStrip(pix1, L_SCAN_HORIZONTAL, 16, 10,
                             L_SELECT_SATURATION);
    pix3 = pixExpandReplicate(pix2, 20);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix2);
    pix2 = pixRankBinByStrip(pix1, L_SCAN_HORIZONTAL, 16, 10, L_SELECT_RED);
    pix3 = pixExpandReplicate(pix2, 20);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix2);
    pix2 = pixaDisplayTiledInRows(pixa, 32, 800, 1.0, 0, 30, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix2, 100, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixaDestroy(&pixa);

        /* Test numaGetRankBinValues() and numaDiscretize functions */
    boxa1 = boxaRead("boxa4.ba");
    boxaSplitEvenOdd(boxa1, 0, &boxa2, &boxa3);
    boxaGetSizes(boxa2, &na1, NULL);  /* 26 elements */
    numaWriteMem(&data, &nbytes, na1);
    regTestWriteDataAndCheck(rp, data, nbytes, ".na");  /* 5 */
    lept_free(data);
    n = numaGetCount(na1);
    nbins = L_MAX(5, n / 50);
    numaGetRankBinValues(na1, nbins, &na2);
    numaWriteMem(&data, &nbytes, na2);
    regTestWriteDataAndCheck(rp, data, nbytes, ".na");  /* 6 */
    lept_free(data);
    numaDestroy(&na2);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    boxaDestroy(&boxa3);

    na3 = numaSort(NULL, na1, L_SORT_INCREASING);
    numaDiscretizeSortedInBins(na3, nbins, &na4);
    numaWriteMem(&data, &nbytes, na4);
    regTestWriteDataAndCheck(rp, data, nbytes, ".na");  /* 7 */
    lept_free(data);
    numaDestroy(&na3);
    numaDestroy(&na4);

    na3 = numaMakeHistogram(na1, 100000, NULL, NULL);
    numaDiscretizeHistoInBins(na3, nbins, &na4, NULL);
    numaWriteMem(&data, &nbytes, na4);
    regTestWriteDataAndCheck(rp, data, nbytes, ".na");  /* 8 */
    lept_free(data);
    regTestCompareFiles(rp, 6, 7);  /* 9 */
    regTestCompareFiles(rp, 6, 8);  /* 10 */
    numaDestroy(&na1);
    numaDestroy(&na3);
    numaDestroy(&na4);

    pixa = pixaCreate(4);
    pix1 = pixRead("karen8.jpg");
    na1 = pixGetGrayHistogram(pix1, 1);
    numaDiscretizeHistoInBins(na1, 1000, &na2, &na3);
    pix2 = gplotSimplePix1(na3, "rank vs gray");
    pix3 = gplotSimplePix1(na2, "gray vs rank-binval");
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 11 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 12 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix1);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);

    pix1 = pixRead("wyom.jpg");
    pixGetRankColorArray(pix1, 20, L_SELECT_RED, 5,
                         &carray, NULL, 0);
    pix2 = pixDisplayColorArray(carray, 20, 200, 5, 6);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 13 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 30, 2);
    pixDisplayWithTitle(pix3, 800, 20, NULL, rp->display);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    lept_free(carray);

    return regTestCleanup(rp);
}
