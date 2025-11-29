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
 *  underlinetest.c
 *
 *  Example program for removing lines under text
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const char *files[] = {"underline1.jpg", "underline2.jpg",
                              "underline3.jpg", "underline4.jpg",
                              "underline5.jpg", "underline6.jpg",
                              "underline7.jpg"};

int main(int    argc,
         char **argv)
{
l_int32  i;
PIX     *pixs, *pixg, *pixg2, *pixb, *pixm, *pixsd, *pixsdd, *pixt, *pixd;
PIXA    *pixa;

    setLeptDebugOK(1);
    lept_mkdir("lept/underline");

    pixa = pixaCreate(0);
    for (i = 0; i < 7; i++) {
        lept_stderr("%d...", i + 1);
        pixs = pixRead(files[i]);
        pixg = pixConvertTo8(pixs, 0);
        pixg2 = pixBackgroundNorm(pixg, NULL, NULL, 15, 15, 70, 105, 200, 5, 5);
        pixSauvolaBinarizeTiled(pixg2, 8, 0.34, 1, 1, NULL, &pixb);
        pixaAddPix(pixa, pixg, L_INSERT);
        pixaAddPix(pixa, pixb, L_INSERT);
        pixDestroy(&pixs);
        pixDestroy(&pixg2);

        /* Get a seed image; try to have at least one pixel
         * in each underline c.c  */
        pixsd = pixMorphSequence(pixb, "c3.1 + o60.1", 0);

        /* Get a mask image for the underlines.
         * The o30.1 tries to remove accidental connections to text. */
        pixm = pixMorphSequence(pixb, "c7.1 + o30.1", 0);

        /* Fill into the seed, clipping to the mask  */
        pixSeedfillBinary(pixsd, pixsd, pixm, 8);
        pixDestroy(&pixm);

        /* Small vertical dilation for better removal  */
        pixsdd = pixMorphSequence(pixsd, "d1.3", 0);
        pixaAddPix(pixa, pixsdd, L_INSERT);
        pixDestroy(&pixsd);

        /* Subtracat to get text without underlines  */
        pixt = pixSubtract(NULL, pixb, pixsdd);
        pixaAddPix(pixa, pixt, L_INSERT);
    }
    lept_stderr("\n");

    pixd = pixaDisplayTiledInColumns(pixa, 4, 0.6, 20, 2);
    pixWrite("/tmp/lept/underline/result.png", pixd, IFF_PNG);
    pixDisplay(pixd, 100, 100);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);
    return 0;
}
