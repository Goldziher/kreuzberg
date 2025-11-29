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
 * circle_reg.c
 *
 *    Extract the digits from within a circle.  In some cases the circle
 *    touches the digits, so this cannot be done by simply selecting
 *    connected components.
 *
 *    Method:
 *      (1) Find a solid circle that covers the fg pixels.
 *      (2) Progressively erode the circle, computing the number of
 *          8-connected components after each successive 3x3 erosion.
 *      (3) Stop when the minimum number of components is first reached,
 *          after passing the maximum number of components.  Disregard the
 *          original image in the counting, because it can have noise.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  num_erodes = 8;

int main(int    argc,
         char **argv)
{
l_int32       i, k, count, n, maxloc, maxval, minval;
NUMA         *na;
PIX          *pixs, *pixsi, *pixc, *pixoc, *pix1, *pix2, *pix3;
PIXA         *pixas, *pixa1, *pixa2;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "circle_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Read the images */
    pixas = pixaRead("circles.pa");
    n = pixaGetCount(pixas);

    pixa2 = pixaCreate(0);
    for (k = 0; k < n; k++) {
        pixa1 = pixaCreate(0);
        na = numaCreate(0);
        pixs = pixaGetPix(pixas, k, L_COPY);
        pixaAddPix(pixa1, pixs, L_INSERT);
        pixsi = pixInvert(NULL, pixs);
        pixc = pixCreateTemplate(pixs);
        pixSetOrClearBorder(pixc, 1, 1, 1, 1, PIX_SET);
        pixSeedfillBinary(pixc, pixc, pixsi, 4);
        pixInvert(pixc, pixc);
        pixoc = pixCopy(NULL, pixc);  /* original circle */
        pixaAddPix(pixa1, pixoc, L_INSERT);
        pix1 = pixAnd(NULL, pixs, pixc);
        pixaAddPix(pixa1, pix1, L_INSERT);
        pixCountConnComp(pix1, 8, &count);
        numaAddNumber(na, count);
        if (rp->display) lept_stderr("count[0] = %d\n", count);
        for (i = 1; i < num_erodes; i++) {
            pixErodeBrick(pixc, pixc, 3, 3);
            pix1 = pixAnd(NULL, pixs, pixc);
            pixaAddPix(pixa1, pix1, L_INSERT);
            pixCountConnComp(pix1, 8, &count);
            numaAddNumber(na, count);
            if (rp->display) lept_stderr("count[%d] = %d\n", i, count);
        }

            /* Find the max value, not including the original image, which
             * may have noise.  Then find the first occurrence of the
             * min value that follows this max value. */
        maxval = maxloc = 0;
        for (i = 1; i < num_erodes; i++) {  /* get max value starting at 1 */
            numaGetIValue(na, i, &count);
            if (count > maxval) {
                maxval = count;
                maxloc = i;
            }
        }
        minval = 1000;
        for (i = maxloc + 1; i < num_erodes; i++) {  /* get the min value */
            numaGetIValue(na, i, &count);
            if (count < minval) minval = count;
        }
        for (i = maxloc + 1; i < num_erodes; i++) {  /* get first occurrence */
            numaGetIValue(na, i, &count);
            if (count == minval) break;
        }

        pix1 = pixErodeBrick(NULL, pixoc, 2 * i + 1, 2 * i + 1);
        pix2 = pixAnd(NULL, pixs, pix1);
        pixaAddPix(pixa1, pix2, L_INSERT);
        pix3 = pixaDisplayTiledInColumns(pixa1, 11, 1.0, 10, 2);
        regTestWritePixAndCheck(rp, pix3, IFF_PNG);
        pixaAddPix(pixa2, pix3, L_INSERT);
        pixaDestroy(&pixa1);
        pixDestroy(&pix1);
        pixDestroy(&pixsi);
        pixDestroy(&pixc);
        numaDestroy(&na);
    }

    pix1 = pixaDisplayTiledInColumns(pixa2, 1, 1.0, 10, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);
    pixDisplayWithTitle(pix1, 100, 100, NULL, rp->display);
    pixaDestroy(&pixas);
    pixaDestroy(&pixa2);
    pixDestroy(&pix1);
    return regTestCleanup(rp);
}
