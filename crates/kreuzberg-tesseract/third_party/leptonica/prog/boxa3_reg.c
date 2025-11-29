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
 *  boxa3_reg.c
 *
 *  Higher-level operations that can search for anomalous-sized boxes
 *  in a boxa, where the widths and heights of the boxes are expected
 *  to be similar.  These can be corrected by moving the appropriate
 *  sides of the outlier boxes.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const char  *boxafiles[3] = {"boxap1.ba", "boxap2.ba", "boxap3.ba"};
static l_float32  varp[3] = {0.0165, 0.0432, 0.0716};
static l_float32  varm[3] = {0.0088, 0.0213, 0.0357};
static l_int32  same[3] = {1, -1, -1};

static void TestBoxa(L_REGPARAMS *rp, l_int32 index);
static void PlotBoxa(L_REGPARAMS *rp, l_int32 index);

l_int32 main(int    argc,
             char **argv)
{
l_int32       i;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "boxa3_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    for (i = 0; i < 3; i++)
        TestBoxa(rp, i);
    for (i = 0; i < 3; i++)
        PlotBoxa(rp, i);
    return regTestCleanup(rp);
}

static void
TestBoxa(L_REGPARAMS  *rp,
         l_int32       index)
{
l_uint8   *data;
l_int32    w, h, medw, medh, isame;
size_t     size;
l_float32  scalefact, ratiowh, fvarp, fvarm;
BOXA      *boxa1, *boxa2, *boxa3;
PIX       *pix1;

    boxa1 = boxaRead(boxafiles[index]);

        /* Read and display initial boxa */
    boxaGetExtent(boxa1, &w, &h, NULL);
    scalefact = 100.0 / (l_float32)w;
    boxa2 = boxaTransform(boxa1, 0, 0, scalefact, scalefact);
    boxaWriteMem(&data, &size, boxa2);
    regTestWriteDataAndCheck(rp, data, size, "ba");  /* 0, 13, 26 */
    lept_free(data);
    pix1 = boxaDisplayTiled(boxa2, NULL, 0, -1, 2200, 2, 1.0, 0, 3, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);   /* 1, 14, 27 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixDestroy(&pix1);

        /* Find the median sizes */
    boxaMedianDimensions(boxa2, &medw, &medh, NULL, NULL, NULL, NULL,
                         NULL, NULL);
    if (rp->display)
        lept_stderr("median width = %d, median height = %d\n", medw, medh);

        /* Check for deviations from median by pairs */
    boxaSizeConsistency(boxa2, L_CHECK_HEIGHT, 0.0, 0.0,
                        &fvarp, &fvarm, &isame);
    regTestCompareValues(rp, varp[index], fvarp, 0.003);  /* 2, 15, 28 */
    regTestCompareValues(rp, varm[index], fvarm, 0.003);  /* 3, 16, 29 */
    regTestCompareValues(rp, same[index], isame, 0);  /* 4, 17, 30 */
    if (rp->display)
        lept_stderr("fvarp = %7.4f, fvarm = %7.4f, same = %d\n",
                    fvarp, fvarm, isame);

        /* Reconcile widths */
    boxa3 = boxaReconcileSizeByMedian(boxa2, L_CHECK_WIDTH, 0.05, 0.04, 1.03,
                                      NULL, NULL, &ratiowh);
    boxaWriteMem(&data, &size, boxa3);
    regTestWriteDataAndCheck(rp, data, size, "ba");  /* 5, 18, 31 */
    lept_free(data);
    pix1 = boxaDisplayTiled(boxa3, NULL, 0, -1, 2200, 2, 1.0, 0, 3, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);   /* 6, 19, 32 */
    pixDisplayWithTitle(pix1, 500, 0, NULL, rp->display);
    if (rp->display)
        lept_stderr("ratio median width/height = %6.3f\n", ratiowh);
    boxaDestroy(&boxa3);
    pixDestroy(&pix1);

        /* Reconcile heights */
    boxa3 = boxaReconcileSizeByMedian(boxa2, L_CHECK_HEIGHT, 0.05, 0.04, 1.03,
                                      NULL, NULL, NULL);
    boxaWriteMem(&data, &size, boxa3);
    regTestWriteDataAndCheck(rp, data, size, "ba");  /* 7, 20, 33 */
    lept_free(data);
    pix1 = boxaDisplayTiled(boxa3, NULL, 0, -1, 2200, 2, 1.0, 0, 3, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);   /* 8, 21, 34 */
    pixDisplayWithTitle(pix1, 1000, 0, NULL, rp->display);
    boxaDestroy(&boxa3);
    pixDestroy(&pix1);

        /* Reconcile both widths and heights */
    boxa3 = boxaReconcileSizeByMedian(boxa2, L_CHECK_BOTH, 0.05, 0.04, 1.03,
                                      NULL, NULL, NULL);
    boxaWriteMem(&data, &size, boxa3);
    regTestWriteDataAndCheck(rp, data, size, "ba");  /* 9, 22, 35 */
    lept_free(data);
    pix1 = boxaDisplayTiled(boxa3, NULL, 0, -1, 2200, 2, 1.0, 0, 3, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);   /* 10, 23, 36 */
    pixDisplayWithTitle(pix1, 1500, 0, NULL, rp->display);
    boxaDestroy(&boxa3);
    pixDestroy(&pix1);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    boxaDestroy(&boxa3);
}


static void
PlotBoxa(L_REGPARAMS  *rp,
         l_int32       index)
{
BOXA  *boxa1, *boxa2;
PIX   *pix1, *pix2, *pix3;
PIXA  *pixa;

    boxa1 = boxaRead(boxafiles[index]);

        /* Read and display initial boxa */
    boxaPlotSizes(boxa1, NULL, NULL, NULL, &pix1);
    boxaPlotSides(boxa1, NULL, NULL, NULL, NULL, NULL, &pix2);
    pixa = pixaCreate(2);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 39, 41, 43 */
    pixDisplayWithTitle(pix3, 0 + 800 * index, 500, NULL, rp->display);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);

        /* Read and display reconciled boxa */
    boxa2 = boxaReconcileSizeByMedian(boxa1, L_CHECK_BOTH, 0.05, 0.04, 1.03,
                                      NULL, NULL, NULL);
    boxaPlotSizes(boxa2, NULL, NULL, NULL, &pix1);
    boxaPlotSides(boxa2, NULL, NULL, NULL, NULL, NULL, &pix2);
    pixa = pixaCreate(2);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 40, 42, 44 */
    pixDisplayWithTitle(pix3, 0 + 800 * index, 920, NULL, rp->display);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
}
