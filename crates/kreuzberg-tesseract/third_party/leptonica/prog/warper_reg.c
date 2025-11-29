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
 *   warper_reg.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

static void DisplayResult(PIXA *pixac, PIX **ppixd, l_int32 newline);
static void DisplayCaptcha(PIXA *pixac, PIX *pixs, l_int32 nterms,
                           l_uint32 seed, l_int32 newline);

static const l_int32 size = 4;
static const l_float32 xmag[] = {3.0f, 4.0f, 5.0f, 7.0f};
static const l_float32 ymag[] = {5.0f, 6.0f, 8.0f, 10.0f};
static const l_float32 xfreq[] = {0.11f, 0.10f, 0.10f, 0.12f};
static const l_float32 yfreq[] = {0.11f, 0.13f, 0.13f, 0.15f};
static const l_int32 nx[] = {4, 3, 2, 1};
static const l_int32 ny[] = {4, 3, 2, 1};


int main(int    argc,
         char **argv)
{
l_int32       i, k, newline;
PIX          *pixs, *pixt, *pixg, *pixd;
PIXA         *pixac;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn-word.tif");
    pixt = pixAddBorder(pixs, 25, 0);
    pixg = pixConvertTo8(pixt, 0);

    for (k = 0; k < size; k++) {
        pixac = pixaCreate(0);
        for (i = 0; i < 50; i++) {
            pixd = pixRandomHarmonicWarp(pixg, xmag[k], ymag[k], xfreq[k],
                                         yfreq[k], nx[k], ny[k], 7 * i, 255);
            newline = (i % 10 == 0) ? 1 : 0;
            DisplayResult(pixac, &pixd, newline);
        }
        pixd = pixaDisplayTiledInColumns(pixac, 10, 1.0, 20, 0);
        regTestWritePixAndCheck(rp, pixd, IFF_PNG);
        pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
        pixaDestroy(&pixac);
        pixDestroy(&pixd);
    }

    pixDestroy(&pixt);
    pixDestroy(&pixg);

    for (k = 1; k <= 4; k++) {
        pixac = pixaCreate(0);
        for (i = 0; i < 50; i++) {
            newline = (i % 10 == 0) ? 1 : 0;
            DisplayCaptcha(pixac, pixs, k, 7 * i, newline);
        }
        pixd = pixaDisplayTiledInColumns(pixac, 10, 1.0, 20, 0);
        regTestWritePixAndCheck(rp, pixd, IFF_PNG);
        pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
        pixaDestroy(&pixac);
        pixDestroy(&pixd);
    }

    pixDestroy(&pixs);
    return regTestCleanup(rp);
}


static void
DisplayResult(PIXA    *pixac,
              PIX    **ppixd,
              l_int32  newline)
{
l_uint32  color;
PIX      *pix1;

    color = 0;
    color = ((rand() >> 16) & 0xff) << L_RED_SHIFT |
            ((rand() >> 16) & 0xff) << L_GREEN_SHIFT |
            ((rand() >> 16) & 0xff) << L_BLUE_SHIFT;
    pix1 = pixColorizeGray(*ppixd, color, 0);
    pixaAddPix(pixac, pix1, L_INSERT);
    pixDestroy(ppixd);
    return;
}


static void
DisplayCaptcha(PIXA    *pixac,
              PIX      *pixs,
              l_int32   nterms,
              l_uint32  seed,
              l_int32   newline)
{
l_uint32  color;
PIX      *pixd;

    color = 0;
    color = ((rand() >> 16) & 0xff) << L_RED_SHIFT |
            ((rand() >> 16) & 0xff) << L_GREEN_SHIFT |
            ((rand() >> 16) & 0xff) << L_BLUE_SHIFT;
    pixd = pixSimpleCaptcha(pixs, 25, nterms, seed, color, 0);
    pixaAddPix(pixac, pixd, L_INSERT);
    return;
}
