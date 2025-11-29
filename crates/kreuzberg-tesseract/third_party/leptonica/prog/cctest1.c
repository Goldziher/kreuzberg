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
 * cctest1.c
 *
 *    This is a test of the following function:
 *
 *        BOXA   *pixConnComp(PIX *pixs, PIXA  **ppixa, l_int32 connectivity)
 *
 *                    pixs:  input pix
 *                    ppixa: &pixa (<optional> pixa of each c.c.)
 *                    connectivity (4 or 8)
 *                    boxa: returned array of boxes of c.c.
 *
 *        Use NULL for &pixa if you don't want the pixa array.
 *
 *    It also demonstrates a few display modes.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  NTIMES             2

l_int32 main(l_int32  argc,
             char   **argv)
{
char     *filein;
l_int32   i, n, count;
BOX      *box;
BOXA     *boxa;
PIX      *pixs, *pixd;
PIXA     *pixa;
PIXCMAP  *cmap;

    if (argc != 2)
        return ERROR_INT(" Syntax:  cctest1 filein", __func__, 1);
    filein = argv[1];

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    if (pixGetDepth(pixs) != 1)
        return ERROR_INT("pixs not 1 bpp", __func__, 1);

        /* Test speed of pixCountConnComp() */
    startTimer();
    for (i = 0; i < NTIMES; i++)
        pixCountConnComp(pixs, 4, &count);
    lept_stderr("Time to compute 4-cc: %6.3f sec\n", stopTimer()/NTIMES);
    lept_stderr("Number of 4-cc: %d\n", count);
    startTimer();
    for (i = 0; i < NTIMES; i++)
        pixCountConnComp(pixs, 8, &count);
    lept_stderr("Time to compute 8-cc: %6.3f sec\n", stopTimer()/NTIMES);
    lept_stderr("Number of 8-cc: %d\n", count);

        /* Test speed of pixConnComp(), with only boxa output  */
    startTimer();
    for (i = 0; i < NTIMES; i++) {
        boxa = pixConnComp(pixs, NULL, 4);
        boxaDestroy(&boxa);
    }
    lept_stderr("Time to compute 4-cc: %6.3f sec\n", stopTimer()/NTIMES);
    startTimer();
    for (i = 0; i < NTIMES; i++) {
        boxa = pixConnComp(pixs, NULL, 8);
        boxaDestroy(&boxa);
    }
    lept_stderr("Time to compute 8-cc: %6.3f sec\n", stopTimer()/NTIMES);

        /* Draw outline of each c.c. box */
    boxa = pixConnComp(pixs, NULL, 4);
    n = boxaGetCount(boxa);
    lept_stderr("Num 4-cc boxes: %d\n", n);
    for (i = 0; i < n; i++) {
        box = boxaGetBox(boxa, i, L_CLONE);
        pixRenderBox(pixs, box, 3, L_FLIP_PIXELS);
        boxDestroy(&box);   /* remember, clones need to be destroyed */
    }
    boxaDestroy(&boxa);

        /* Display each component as a random color in cmapped 8 bpp.
         * Background is color 0; it is set to white. */
    boxa = pixConnComp(pixs, &pixa, 4);
    pixd = pixaDisplayRandomCmap(pixa, pixGetWidth(pixs), pixGetHeight(pixs));
    cmap = pixGetColormap(pixd);
    pixcmapResetColor(cmap, 0, 255, 255, 255);  /* reset background to white */
    pixDisplay(pixd, 100, 100);
    boxaDestroy(&boxa);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    pixDestroy(&pixs);
    return 0;
}


