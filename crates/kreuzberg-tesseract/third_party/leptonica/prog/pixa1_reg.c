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
 * pixa1_reg.c
 *
 *    Tests removal of connected components.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32 CONNECTIVITY = 8;

int main(int    argc,
         char **argv)
{
l_int32       size, i, n, n0;
BOXA         *boxa;
GPLOT        *gplot;
NUMA         *nax, *nay1, *nay2;
PIX          *pixs, *pix1, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn-fract.tif");
    lept_mkdir("lept/pixa");
    pixa = pixaCreate(2);

    /* ----------------  Remove small components --------------- */
    boxa = pixConnComp(pixs, NULL, 8);
    n0 = boxaGetCount(boxa);
    nax = numaMakeSequence(0, 2, 51);
    nay1 = numaCreate(51);
    nay2 = numaCreate(51);
    boxaDestroy(&boxa);

    if (rp->display) {
        lept_stderr("\n Select Large if Both\n");
        lept_stderr("Iter 0: n = %d\n", n0);
    }
    numaAddNumber(nay1, n0);
    for (i = 1; i <= 50; i++) {
        size = 2 * i;
        pixd = pixSelectBySize(pixs, size, size, CONNECTIVITY,
                               L_SELECT_IF_BOTH, L_SELECT_IF_GTE, NULL);
        boxa = pixConnComp(pixd, NULL, 8);
        n = boxaGetCount(boxa);
        numaAddNumber(nay1, n);
        if (rp->display) lept_stderr("Iter %d: n = %d\n", i, n);
        boxaDestroy(&boxa);
        pixDestroy(&pixd);
    }

    if (rp->display) {
        lept_stderr("\n Select Large if Either\n");
        lept_stderr("Iter 0: n = %d\n", n0);
    }
    numaAddNumber(nay2, n0);
    for (i = 1; i <= 50; i++) {
        size = 2 * i;
        pixd = pixSelectBySize(pixs, size, size, CONNECTIVITY,
                               L_SELECT_IF_EITHER, L_SELECT_IF_GTE, NULL);
        boxa = pixConnComp(pixd, NULL, 8);
        n = boxaGetCount(boxa);
        numaAddNumber(nay2, n);
        if (rp->display) lept_stderr("Iter %d: n = %d\n", i, n);
        boxaDestroy(&boxa);
        pixDestroy(&pixd);
    }

    gplot = gplotCreate("/tmp/lept/pixa/root1", GPLOT_PNG,
                        "Select large: number of cc vs size removed",
                        "min size", "number of c.c.");
    gplotAddPlot(gplot, nax, nay1, GPLOT_LINES, "select if both");
    gplotAddPlot(gplot, nax, nay2, GPLOT_LINES, "select if either");
    pix1 = gplotMakeOutputPix(gplot);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pix1, L_INSERT);
    gplotDestroy(&gplot);

    /* ----------------  Remove large components --------------- */
    numaEmpty(nay1);
    numaEmpty(nay2);

    if (rp->display) {
        lept_stderr("\n Select Small if Both\n");
        lept_stderr("Iter 0: n = %d\n", 0);
    }
    numaAddNumber(nay1, 0);
    for (i = 1; i <= 50; i++) {
        size = 2 * i;
        pixd = pixSelectBySize(pixs, size, size, CONNECTIVITY,
                               L_SELECT_IF_BOTH, L_SELECT_IF_LTE, NULL);
        boxa = pixConnComp(pixd, NULL, 8);
        n = boxaGetCount(boxa);
        numaAddNumber(nay1, n);
        if (rp->display) lept_stderr("Iter %d: n = %d\n", i, n);
        boxaDestroy(&boxa);
        pixDestroy(&pixd);
    }

    if (rp->display) {
        lept_stderr("\n Select Small if Either\n");
        lept_stderr("Iter 0: n = %d\n", 0);
    }
    numaAddNumber(nay2, 0);
    for (i = 1; i <= 50; i++) {
        size = 2 * i;
        pixd = pixSelectBySize(pixs, size, size, CONNECTIVITY,
                               L_SELECT_IF_EITHER, L_SELECT_IF_LTE, NULL);
        boxa = pixConnComp(pixd, NULL, 8);
        n = boxaGetCount(boxa);
        numaAddNumber(nay2, n);
        if (rp->display) lept_stderr("Iter %d: n = %d\n", i, n);
        boxaDestroy(&boxa);
        pixDestroy(&pixd);
    }

    gplot = gplotCreate("/tmp/lept/pixa/root2", GPLOT_PNG,
                        "Remove large: number of cc vs size removed",
                        "min size", "number of c.c.");
    gplotAddPlot(gplot, nax, nay1, GPLOT_LINES, "select if both");
    gplotAddPlot(gplot, nax, nay2, GPLOT_LINES, "select if either");
    pix1 = gplotMakeOutputPix(gplot);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pix1, L_INSERT);
    gplotDestroy(&gplot);

    pixd = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 20, 2);
    pixDisplayWithTitle(pixd, 100, 0, NULL, rp->display);
    pixWrite("/tmp/lept/pixa/root.png", pixd, IFF_PNG);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    numaDestroy(&nax);
    numaDestroy(&nay1);
    numaDestroy(&nay2);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}
