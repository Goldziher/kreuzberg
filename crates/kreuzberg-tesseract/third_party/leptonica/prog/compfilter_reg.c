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
 *  compfilter_reg.c
 *
 *     Regression test for filters that select connected components
 *     based on size, using logical combinations of indicator arrays.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void Count_pieces(L_REGPARAMS *rp, PIX  *pix, l_int32 nexp);
static void Count_pieces2(L_REGPARAMS *rp, BOXA *boxa, l_int32 nexp);
static l_int32 Count_ones(L_REGPARAMS *rp, NUMA  *na, l_int32 nexp,
                          l_int32 index, const char *name);

static const l_float32 edges[13] = {0.0f, 0.2f, 0.3f, 0.35f, 0.4f, 0.45f, 0.5f,
                                    0.55f, 0.6f, 0.7f, 0.8f, 0.9f, 1.0f};

    /* for feyn.tif */
static const l_int32 band[12] = {1, 11, 48, 264, 574, 704, 908, 786, 466,
                                 157, 156, 230};
static const l_int32 total[12] = {1, 12, 60, 324, 898, 1602, 2510, 3296,
                                  3762, 3919, 4075, 4305};
#if 0
    /* for rabi.png */
static const l_int32 band[12] = {24, 295, 490, 817, 1768, 962, 8171,
                                 63, 81, 51, 137, 8619};
static const l_int32 total[12] = {24, 319, 809, 1626, 3394, 4356, 12527,
                                  12590, 12671, 12722, 12859, 21478};
#endif


int main(int    argc,
         char **argv)
{
l_int32       w, h, n, i, sum, sumi, empty;
BOX          *box1, *box2, *box3, *box4;
BOXA         *boxa1, *boxa2;
NUMA         *na1, *na2, *na3, *na4, *na5;
NUMA         *na2i, *na3i, *na4i, *nat, *naw, *nah;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4;
PIXA         *pixa1, *pixa2, *pixa3;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Draw 4 filled boxes of different sizes */
    pixs = pixCreate(200, 200, 1);
    box1 = boxCreate(10, 10, 20, 30);
    box2 = boxCreate(50, 10, 40, 20);
    box3 = boxCreate(110, 10, 35, 5);
    box4 = boxCreate(160, 10, 5, 15);
    boxa1 = boxaCreate(4);
    boxaAddBox(boxa1, box1, L_INSERT);
    boxaAddBox(boxa1, box2, L_INSERT);
    boxaAddBox(boxa1, box3, L_INSERT);
    boxaAddBox(boxa1, box4, L_INSERT);
    pixRenderBox(pixs, box1, 1, L_SET_PIXELS);
    pixRenderBox(pixs, box2, 1, L_SET_PIXELS);
    pixRenderBox(pixs, box3, 1, L_SET_PIXELS);
    pixRenderBox(pixs, box4, 1, L_SET_PIXELS);
    pix1 = pixFillClosedBorders(pixs, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pix2 = pixCreateTemplate(pixs);
    pixRenderHashBox(pix2, box1, 6, 4, L_POS_SLOPE_LINE, 1, L_SET_PIXELS);
    pixRenderHashBox(pix2, box2, 7, 2, L_POS_SLOPE_LINE, 1, L_SET_PIXELS);
    pixRenderHashBox(pix2, box3, 4, 2, L_VERTICAL_LINE, 1, L_SET_PIXELS);
    pixRenderHashBox(pix2, box4, 3, 1, L_HORIZONTAL_LINE, 1, L_SET_PIXELS);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */

        /* Exercise the parameters:  Reg indices 2-27  */
    pix3 = pixSelectBySize(pix1, 0, 22, 8, L_SELECT_HEIGHT,
                           L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 1);
    pix3 = pixSelectBySize(pix1, 0, 30, 8, L_SELECT_HEIGHT,
                           L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 3);
    pix3 = pixSelectBySize(pix1, 0, 5, 8, L_SELECT_HEIGHT,
                           L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 3);
    pix3 = pixSelectBySize(pix1, 0, 6, 8, L_SELECT_HEIGHT,
                           L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 1);
    pix3 = pixSelectBySize(pix1, 20, 0, 8, L_SELECT_WIDTH,
                           L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 2);
    pix3 = pixSelectBySize(pix1, 31, 0, 8, L_SELECT_WIDTH,
                           L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 2);
    pix3 = pixSelectBySize(pix1, 21, 10, 8, L_SELECT_IF_EITHER,
                           L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 3);
    pix3 = pixSelectBySize(pix1, 20, 30, 8, L_SELECT_IF_EITHER,
                           L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 2);
    pix3 = pixSelectBySize(pix1, 22, 32, 8, L_SELECT_IF_BOTH,
                           L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 2);
    pix3 = pixSelectBySize(pix1, 6, 32, 8, L_SELECT_IF_BOTH,
                           L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 1);
    pix3 = pixSelectBySize(pix1, 5, 25, 8, L_SELECT_IF_BOTH,
                           L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 1);
    pix3 = pixSelectBySize(pix1, 25, 5, 8, L_SELECT_IF_BOTH,
                           L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 1);

    pix3 = pixSelectByPerimToAreaRatio(pix1, 0.3, 8, L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 2);
    pix3 = pixSelectByPerimToAreaRatio(pix1, 0.15, 8, L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 3);
    pix3 = pixSelectByPerimToAreaRatio(pix1, 0.4, 8, L_SELECT_IF_LTE, NULL);
    Count_pieces(rp, pix3, 2);
    pix3 = pixSelectByPerimToAreaRatio(pix1, 0.45, 8, L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 3);

    pix3 = pixSelectByPerimSizeRatio(pix2, 2.3, 8, L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 2);
    pix3 = pixSelectByPerimSizeRatio(pix2, 1.2, 8, L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 3);
    pix3 = pixSelectByPerimSizeRatio(pix2, 1.7, 8, L_SELECT_IF_LTE, NULL);
    Count_pieces(rp, pix3, 1);
    pix3 = pixSelectByPerimSizeRatio(pix2, 2.9, 8, L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 3);

    pix3 = pixSelectByAreaFraction(pix2, 0.3, 8, L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 0);
    pix3 = pixSelectByAreaFraction(pix2, 0.9, 8, L_SELECT_IF_LT, NULL);
    Count_pieces(rp, pix3, 4);
    pix3 = pixSelectByAreaFraction(pix2, 0.5, 8, L_SELECT_IF_GTE, NULL);
    Count_pieces(rp, pix3, 3);
    pix3 = pixSelectByAreaFraction(pix2, 0.7, 8, L_SELECT_IF_GT, NULL);
    Count_pieces(rp, pix3, 2);

    boxa2 = boxaSelectBySize(boxa1, 21, 10, L_SELECT_IF_EITHER,
                             L_SELECT_IF_LT, NULL);
    Count_pieces2(rp, boxa2, 3);
    boxa2 = boxaSelectBySize(boxa1, 22, 32, L_SELECT_IF_BOTH,
                             L_SELECT_IF_LT, NULL);
    Count_pieces2(rp, boxa2, 2);
    boxaDestroy(&boxa1);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Here's the most general method for selecting components.   *
         * We do it for area fraction, but any combination of         *
         * size, area/perimeter ratio and area fraction can be used.  *
         * Reg indices 28-85                                          */
    pixs = pixRead("feyn.tif");
    pix1 = pixCopy(NULL, pixs);  /* subtract bands from this */
    pix2 = pixCreateTemplate(pixs);  /* add bands to this */
    pixGetDimensions(pixs, &w, &h, NULL);
    boxa1 = pixConnComp(pixs, &pixa1, 8);
    n = boxaGetCount(boxa1);
    na1 = pixaFindAreaFraction(pixa1);
    nat = numaCreate(0);
    numaSetCount(nat, n);  /* initialize to all 0 */
    sum = sumi = 0;
    pixa3 = pixaCreate(0);
    for (i = 0; i < 12; i++) {
            /* Compute within the intervals using an intersection. */
        na2 = numaMakeThresholdIndicator(na1, edges[i], L_SELECT_IF_GTE);
        if (i != 11)
            na3 = numaMakeThresholdIndicator(na1, edges[i + 1], L_SELECT_IF_LT);
        else
            na3 = numaMakeThresholdIndicator(na1, edges[i + 1],
                                             L_SELECT_IF_LTE);
        na4 = numaLogicalOp(NULL, na2, na3, L_INTERSECTION);
        sum += Count_ones(rp, na4, 0, 0, NULL);

            /* Compute outside the intervals using a union, and invert */
        na2i = numaMakeThresholdIndicator(na1, edges[i], L_SELECT_IF_LT);
        if (i != 11)
            na3i = numaMakeThresholdIndicator(na1, edges[i + 1],
                                              L_SELECT_IF_GTE);
        else
            na3i = numaMakeThresholdIndicator(na1, edges[i + 1],
                                              L_SELECT_IF_GT);
        na4i = numaLogicalOp(NULL, na3i, na2i, L_UNION);
        numaInvert(na4i, na4i);
        sumi += Count_ones(rp, na4i, 0, 0, NULL);

            /* Compare the two methods */
        if (sum != sumi)
            lept_stderr("WRONG: sum = %d, sumi = %d\n", sum, sumi);

            /* Reconstruct the image, band by band. */
        numaLogicalOp(nat, nat, na4, L_UNION);
        pixa2 = pixaSelectWithIndicator(pixa1, na4, NULL);
        pix3 = pixaDisplay(pixa2, w, h);
        pixOr(pix2, pix2, pix3);  /* add them in */
        pix4 = pixCopy(NULL, pix2);  /* destroyed by Count_pieces */
        Count_ones(rp, na4, band[i], i, "band");
        Count_pieces(rp, pix3, band[i]);
        Count_ones(rp, nat, total[i], i, "total");
        Count_pieces(rp, pix4, total[i]);
        pixaDestroy(&pixa2);

            /* Remove band successively from full image */
        pixRemoveWithIndicator(pix1, pixa1, na4);
        pixaAddPix(pixa3, pix1, L_COPY);
        numaDestroy(&na2);
        numaDestroy(&na3);
        numaDestroy(&na4);
        numaDestroy(&na2i);
        numaDestroy(&na3i);
        numaDestroy(&na4i);
    }

        /* Did we remove all components from pix1? */
    pixZero(pix1, &empty);
    regTestCompareValues(rp, 1, empty, 0.0);
    if (!empty)
        lept_stderr("\nWRONG: not all pixels removed from pix1\n");

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxaDestroy(&boxa1);
    pixaDestroy(&pixa1);
    numaDestroy(&na1);
    numaDestroy(&nat);

        /* One last extraction.  Get all components that have either
         * a height of at least 50 or a width of between 30 and 35,
         * and also have a relatively large perimeter/area ratio. */
    pixs = pixRead("feyn.tif");
    boxa1 = pixConnComp(pixs, &pixa1, 8);
    n = boxaGetCount(boxa1);
    pixaFindDimensions(pixa1, &naw, &nah);
    na1 = pixaFindPerimToAreaRatio(pixa1);
    na2 = numaMakeThresholdIndicator(nah, 50, L_SELECT_IF_GTE);
    na3 = numaMakeThresholdIndicator(naw, 30, L_SELECT_IF_GTE);
    na4 = numaMakeThresholdIndicator(naw, 35, L_SELECT_IF_LTE);
    na5 = numaMakeThresholdIndicator(na1, 0.4, L_SELECT_IF_GTE);
    numaLogicalOp(na3, na3, na4, L_INTERSECTION);
    numaLogicalOp(na2, na2, na3, L_UNION);
    numaLogicalOp(na2, na2, na5, L_INTERSECTION);
    numaInvert(na2, na2);  /* get components to be removed */
    pixRemoveWithIndicator(pixs, pixa1, na2);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 86 */
    pixaAddPix(pixa3, pixs, L_INSERT);
    boxaDestroy(&boxa1);
    pixaDestroy(&pixa1);
    numaDestroy(&naw);
    numaDestroy(&nah);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    numaDestroy(&na4);
    numaDestroy(&na5);

    if (rp->display) {
        pix1 = pixaDisplayTiledInColumns(pixa3, 2, 0.25, 25, 2);
        pixDisplay(pix1, 100, 100);
        pixWrite("/tmp/lept/filter/result.png", pix1, IFF_PNG);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa3);

    return regTestCleanup(rp);
}


/* ---------------------  Helpers -------------------------- */

static void
Count_pieces(L_REGPARAMS *rp, PIX  *pix, l_int32 nexp)
{
l_int32  n;
BOXA    *boxa;

    if (rp->index > 28 && rp->index < 55)
        regTestWritePixAndCheck(rp, pix, IFF_PNG);  /* ... */
    boxa = pixConnComp(pix, NULL, 8);
    n = boxaGetCount(boxa);
    regTestCompareValues(rp, nexp, n, 0.0);
    if (n != nexp)
        lept_stderr("WRONG!: Num. comps = %d; expected = %d\n", n, nexp);
    boxaDestroy(&boxa);
    pixDestroy(&pix);
}

static void
Count_pieces2(L_REGPARAMS *rp, BOXA  *boxa, l_int32 nexp)
{
l_int32  n;

    n = boxaGetCount(boxa);
    regTestCompareValues(rp, nexp, n, 0.0);
    if (n != nexp)
        lept_stderr("WRONG!: Num. boxes = %d; expected = %d\n", n, nexp);
    boxaDestroy(&boxa);
}

static l_int32
Count_ones(L_REGPARAMS *rp, NUMA  *na, l_int32 nexp,
           l_int32 index, const char *name)
{
l_int32  i, n, val, sum;

    n = numaGetCount(na);
    sum = 0;
    for (i = 0; i < n; i++) {
        numaGetIValue(na, i, &val);
        if (val == 1) sum++;
    }
    if (!name) return sum;
    regTestCompareValues(rp, nexp, sum, 0.0);
    if (nexp != sum)
        lept_stderr("WRONG! %s[%d]: num. ones = %d; expected = %d\n",
                name, index, sum, nexp);
    return 0;
}
