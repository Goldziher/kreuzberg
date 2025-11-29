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
 * rankhisto_reg.c
 *
 *   Tests grayscale rank functions:
 *      (1) pixGetRankColorArray()
 *      (2) numaDiscretizeHistoInBins()
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, w, h, nbins, factor;
l_int32       spike;
l_uint32     *array, *marray;
NUMA         *na, *nabinval, *narank;
PIX          *pixs, *pix1, *pix2;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "rankhisto_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Find the rank bin colors */
    pixs = pixRead("map1.jpg");
    pixGetDimensions(pixs, &w, &h, NULL);
    factor = L_MAX(1, (l_int32)sqrt((l_float64)(w * h / 20000.0)));
    nbins = 10;
    pixa = pixaCreate(0);
    pixGetRankColorArray(pixs, nbins, L_SELECT_MIN, factor, &array, pixa, 6);
    if (!array)
        return ERROR_INT("\n\n\nFAILURE!\n\n\n", rp->testname, 1);
    pix1 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix1, 1000, 100, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);
    for (i = 0; i < nbins; i++)
        lept_stderr("%d: %x\n", i, array[i]);
    pix1 = pixDisplayColorArray(array, nbins, 200, 5, 6);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix1, 0, 100, NULL, rp->display);
    pixDestroy(&pix1);

        /* Modify the rank bin colors by mapping them such
         * that the lightest color is mapped to white */
    marray = (l_uint32 *)lept_calloc(nbins, sizeof(l_uint32));
    for (i = 0; i < nbins; i++)
        pixelLinearMapToTargetColor(array[i], array[nbins - 1],
                                    0xffffff00, &marray[i]);
    pix1 = pixDisplayColorArray(marray, nbins, 200, 5, 6);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 0, 600, NULL, rp->display);
    pixDestroy(&pix1);
    lept_free(marray);

        /* Map to the lightest bin; then do TRC adjustment */
    pix1 = pixLinearMapToTargetColor(NULL, pixs, array[nbins - 1], 0xffffff00);
    pix2 = pixGammaTRC(NULL, pix1, 1.0, 0, 240);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix2, 1000, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Now test the edge case where all the histo data is piled up
         * at one place.  We only require that the result be sensible. */
    pixa = pixaCreate(0);
    for (i = 0; i < 3; i++) {
        if (i == 0)
            spike = 1;
        else if (i == 1)
            spike = 50;
        else
            spike = 99;
        na = numaMakeConstant(0, 100);
        numaReplaceNumber(na, spike, 200.0);
        numaDiscretizeHistoInBins(na, 10, &nabinval, &narank);
        pix1 = gplotSimplePix1(na, "Histogram");
        pixaAddPix(pixa, pix1, L_INSERT);
        pix1 = gplotSimplePix1(nabinval, "Gray value vs rank bin");
        pixaAddPix(pixa, pix1, L_INSERT);
        pix1 = gplotSimplePix1(narank, "rank value vs. gray value");
        pixaAddPix(pixa, pix1, L_INSERT);
        numaDestroy(&na);
        numaDestroy(&nabinval);
        numaDestroy(&narank);
    }
    pix1 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix1, 1000, 800, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);

    pixDestroy(&pixs);
    lept_free(array);
    return regTestCleanup(rp);
}

