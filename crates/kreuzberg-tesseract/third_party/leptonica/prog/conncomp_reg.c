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
 * conncomp_reg.c
 *
 *      Regression test for connected components (both 4 and 8
 *      connected), including regeneration of the original
 *      image from the components.  This is also an implicit
 *      test of rasterop.
 *
 *      Also tests iterative covering of connected components by
 *      minimum spanning rectangles.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_uint8      *array1, *array2;
l_int32       i, n1, n2, n3;
size_t        size1, size2;
FILE         *fp;
BOXA         *boxa1, *boxa2;
PIX          *pixs, *pix1, *pix2, *pix3;
PIXA         *pixa1;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn.tif");

    /* --------------------------------------------------------------- *
     *         Test pixConnComp() and pixCountConnComp(),              *
     *            with output to both boxa and pixa                    *
     * --------------------------------------------------------------- */
        /* First, test with 4-cc */
    boxa1= pixConnComp(pixs, &pixa1, 4);
    n1 = boxaGetCount(boxa1);
    boxa2= pixConnComp(pixs, NULL, 4);
    n2 = boxaGetCount(boxa2);
    pixCountConnComp(pixs, 4, &n3);
    lept_stderr("Number of 4 c.c.:  n1 = %d; n2 = %d, n3 = %d\n", n1, n2, n3);
    regTestCompareValues(rp, n1, n2, 0);  /* 0 */
    regTestCompareValues(rp, n1, n3, 0);  /* 1 */
    regTestCompareValues(rp, n1, 4452, 0);  /* 2 */
    pix1 = pixaDisplay(pixa1, pixGetWidth(pixs), pixGetHeight(pixs));
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    regTestComparePix(rp, pixs, pix1);  /* 4 */
    pixaDestroy(&pixa1);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    pixDestroy(&pix1);

        /* Test with 8-cc */
    boxa1= pixConnComp(pixs, &pixa1, 8);
    n1 = boxaGetCount(boxa1);
    boxa2= pixConnComp(pixs, NULL, 8);
    n2 = boxaGetCount(boxa2);
    pixCountConnComp(pixs, 8, &n3);
    lept_stderr("Number of 8 c.c.:  n1 = %d; n2 = %d, n3 = %d\n", n1, n2, n3);
    regTestCompareValues(rp, n1, n2, 0);  /* 5 */
    regTestCompareValues(rp, n1, n3, 0);  /* 6 */
    regTestCompareValues(rp, n1, 4305, 0);  /* 7 */
    pix1 = pixaDisplay(pixa1, pixGetWidth(pixs), pixGetHeight(pixs));
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 8 */
    regTestComparePix(rp, pixs, pix1);  /* 9 */
    pixaDestroy(&pixa1);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    pixDestroy(&pix1);

    /* --------------------------------------------------------------- *
     *                        Test boxa I/O                            *
     * --------------------------------------------------------------- */
    lept_mkdir("lept/conn");
    boxa1 = pixConnComp(pixs, NULL, 4);
    fp = lept_fopen("/tmp/lept/conn/boxa1.ba", "wb+");
    boxaWriteStream(fp, boxa1);
    lept_fclose(fp);
    fp = lept_fopen("/tmp/lept/conn/boxa1.ba", "rb");
    boxa2 = boxaReadStream(fp);
    lept_fclose(fp);
    fp = lept_fopen("/tmp/lept/conn/boxa2.ba", "wb+");
    boxaWriteStream(fp, boxa2);
    lept_fclose(fp);
    array1 = l_binaryRead("/tmp/lept/conn/boxa1.ba", &size1);
    array2 = l_binaryRead("/tmp/lept/conn/boxa2.ba", &size2);
    regTestCompareStrings(rp, array1, size1, array2, size2);  /* 10 */
    lept_free(array1);
    lept_free(array2);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);

    /* --------------------------------------------------------------- *
     *    Just for fun, display each component as a random color in    *
     *    cmapped 8 bpp.  Background is color 0; it is set to white.   *
     * --------------------------------------------------------------- */
    boxa1 = pixConnComp(pixs, &pixa1, 4);
    pix1 = pixaDisplayRandomCmap(pixa1, pixGetWidth(pixs), pixGetHeight(pixs));
    cmap = pixGetColormap(pix1);
    pixcmapResetColor(cmap, 0, 255, 255, 255);  /* reset background to white */
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 11 */
    if (rp->display) pixDisplay(pix1, 100, 0);
    boxaDestroy(&boxa1);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);
    pixDestroy(&pixs);

    /* --------------------------------------------------------------- *
     *  Test iterative covering of connected components by rectangles  *
     * --------------------------------------------------------------- */
    pixa1 = pixaCreate(0);
    pix1 = pixRead("rabi.png");
    pix2 = pixReduceRankBinaryCascade(pix1, 1, 1, 1, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 12 -  */
    pixaAddPix(pixa1, pix2, L_INSERT);
    for (i = 1; i < 6; i++) {
        pix3 = pixMakeCoveringOfRectangles(pix2, i);
        regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 13 - 17 */
        pixaAddPix(pixa1, pix3, L_INSERT);
    }
    pix3 = pixaDisplayTiledInRows(pixa1, 1, 2500, 1.0, 0, 30, 0);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 18 */
    pixDisplayWithTitle(pix3, 100, 900, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa1);

    return regTestCleanup(rp);
}


