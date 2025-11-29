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
 *   dewarptest3.c
 *
 *   This exercise functions in dewarp.c for dewarping based on lines
 *   of horizontal text, showing results for different interpolations
 *   (quadratic, cubic, quartic).
 *
 *   Inspection of the output pdf shows that using LS fitting beyond
 *   quadratic has a tendency to overfit.  So we choose to use
 *   quadratic LSF for the textlines.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
l_int32     i, n;
l_float32   a, b, c, d, e;
NUMA       *nax, *nafit;
PIX        *pixs, *pixn, *pixg, *pixb, *pixt1, *pixt2;
PIXA       *pixa;
PTA        *pta, *ptad;
PTAA       *ptaa1, *ptaa2;

    setLeptDebugOK(1);
    lept_mkdir("lept");

    pixs = pixRead("cat.035.jpg");
/*    pixs = pixRead("zanotti-78.jpg"); */

        /* Normalize for varying background and binarize */
    pixn = pixBackgroundNormSimple(pixs, NULL, NULL);
    pixg = pixConvertRGBToGray(pixn, 0.5, 0.3, 0.2);
    pixb = pixThresholdToBinary(pixg, 130);
    pixDestroy(&pixn);
    pixDestroy(&pixg);

        /* Get the textline centers */
    pixa = pixaCreate(6);
    ptaa1 = dewarpGetTextlineCenters(pixb, 0);
    pixt1 = pixCreateTemplate(pixs);
    pixSetAll(pixt1);
    pixt2 = pixDisplayPtaa(pixt1, ptaa1);
    pixWrite("/tmp/lept/textline1.png", pixt2, IFF_PNG);
    pixDisplayWithTitle(pixt2, 0, 100, "textline centers 1", 1);
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixDestroy(&pixt1);

        /* Remove short lines */
    lept_stderr("Num all lines = %d\n", ptaaGetCount(ptaa1));
    ptaa2 = dewarpRemoveShortLines(pixb, ptaa1, 0.8, 0);
    pixt1 = pixCreateTemplate(pixs);
    pixSetAll(pixt1);
    pixt2 = pixDisplayPtaa(pixt1, ptaa2);
    pixWrite("/tmp/lept/textline2.png", pixt2, IFF_PNG);
    pixDisplayWithTitle(pixt2, 300, 100, "textline centers 2", 1);
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixDestroy(&pixt1);
    n = ptaaGetCount(ptaa2);
    lept_stderr("Num long lines = %d\n", n);
    ptaaDestroy(&ptaa1);
    pixDestroy(&pixb);

        /* Long lines over input image */
    pixt1 = pixCopy(NULL, pixs);
    pixt2 = pixDisplayPtaa(pixt1, ptaa2);
    pixWrite("/tmp/lept/textline3.png", pixt2, IFF_PNG);
    pixDisplayWithTitle(pixt2, 600, 100, "textline centers 3", 1);
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixDestroy(&pixt1);

        /* Quadratic fit to curve */
    pixt1 = pixCopy(NULL, pixs);
    for (i = 0; i < n; i++) {
        pta = ptaaGetPta(ptaa2, i, L_CLONE);
        ptaGetArrays(pta, &nax, NULL);
        ptaGetQuadraticLSF(pta, &a, &b, &c, &nafit);
        lept_stderr("Quadratic: a = %10.6f, b = %7.3f, c = %7.3f\n", a, b, c);
        ptad = ptaCreateFromNuma(nax, nafit);
        pixDisplayPta(pixt1, pixt1, ptad);
        ptaDestroy(&pta);
        ptaDestroy(&ptad);
        numaDestroy(&nax);
        numaDestroy(&nafit);
    }
    pixWrite("/tmp/lept/textline4.png", pixt1, IFF_PNG);
    pixDisplayWithTitle(pixt1, 900, 100, "textline centers 4", 1);
    pixaAddPix(pixa, pixt1, L_INSERT);

        /* Cubic fit to curve */
    pixt1 = pixCopy(NULL, pixs);
    for (i = 0; i < n; i++) {
        pta = ptaaGetPta(ptaa2, i, L_CLONE);
        ptaGetArrays(pta, &nax, NULL);
        ptaGetCubicLSF(pta, &a, &b, &c, &d, &nafit);
        lept_stderr("Cubic: a = %10.6f, b = %10.6f, c = %7.3f, d = %7.3f\n",
                    a, b, c, d);
        ptad = ptaCreateFromNuma(nax, nafit);
        pixDisplayPta(pixt1, pixt1, ptad);
        ptaDestroy(&pta);
        ptaDestroy(&ptad);
        numaDestroy(&nax);
        numaDestroy(&nafit);
    }
    pixWrite("/tmp/lept/textline5.png", pixt1, IFF_PNG);
    pixDisplayWithTitle(pixt1, 1200, 100, "textline centers 5", 1);
    pixaAddPix(pixa, pixt1, L_INSERT);

        /* Quartic fit to curve */
    pixt1 = pixCopy(NULL, pixs);
    for (i = 0; i < n; i++) {
        pta = ptaaGetPta(ptaa2, i, L_CLONE);
        ptaGetArrays(pta, &nax, NULL);
        ptaGetQuarticLSF(pta, &a, &b, &c, &d, &e, &nafit);
        lept_stderr(
            "Quartic: a = %7.3f, b = %7.3f, c = %9.5f, d = %7.3f, e = %7.3f\n",
            a, b, c, d, e);
        ptad = ptaCreateFromNuma(nax, nafit);
        pixDisplayPta(pixt1, pixt1, ptad);
        ptaDestroy(&pta);
        ptaDestroy(&ptad);
        numaDestroy(&nax);
        numaDestroy(&nafit);
    }
    pixWrite("/tmp/lept/textline6.png", pixt1, IFF_PNG);
    pixDisplayWithTitle(pixt1, 1500, 100, "textline centers 6", 1);
    pixaAddPix(pixa, pixt1, L_INSERT);

    pixaConvertToPdf(pixa, 300, 0.5, L_JPEG_ENCODE, 75,
                     "LS fittings to textlines",
                     "/tmp/lept/dewarp_fittings.pdf");
    pixaDestroy(&pixa);
    pixDestroy(&pixs);
    ptaaDestroy(&ptaa2);
    return 0;
}
