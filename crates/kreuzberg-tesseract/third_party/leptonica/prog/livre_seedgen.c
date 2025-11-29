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
 * livre_seedgen.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32  i;
PIX     *pixs, *pix1, *pix2, *pix3;
PIXA    *pixa;

    setLeptDebugOK(1);
    pixs = pixRead("pageseg2.tif");

    startTimer();
    for (i = 0; i < 100; i++) {
        pix1 = pixReduceRankBinaryCascade(pixs, 1, 4, 4, 3);
        pixDestroy(&pix1);
    }
    lept_stderr("Time: %8.4f sec\n", stopTimer() / 100.);

        /* 4 2x rank reductions (levels 1, 4, 4, 3), followed by 5x5 opening */
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixs, L_INSERT);
    pix1 = pixReduceRankBinaryCascade(pixs, 1, 4, 0, 0);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixReduceRankBinaryCascade(pix1, 4, 3, 0, 0);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixOpenBrick(pix2, pix2, 5, 5);
    pix3 = pixExpandBinaryReplicate(pix2, 2, 2);
    pixaAddPix(pixa, pix3, L_INSERT);

        /* Generate the output image */
    lept_mkdir("lept/livre");
    lept_stderr("Writing to: /tmp/lept/livre/seedgen.png\n");
    pix1 = pixaDisplayTiledAndScaled(pixa, 8, 350, 4, 0, 25, 2);
    pixWrite("/tmp/lept/livre/seedgen.png", pix1, IFF_PNG);
    pixDisplay(pix1, 1100, 0);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    return 0;
}

