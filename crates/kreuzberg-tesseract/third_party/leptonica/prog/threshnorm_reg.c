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
 * threshnorm__reg.c
 *
 *      Regression test for adaptive threshold normalization.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

static void AddTestSet(PIXA *pixa, PIX *pixs,
                       l_int32 filtertype, l_int32 edgethresh,
                       l_int32 smoothx, l_int32 smoothy,
                       l_float32 gamma, l_int32 minval,
                       l_int32 maxval, l_int32 targetthresh);

int main(int    argc,
         char **argv)
{
PIX          *pixs, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("stampede2.jpg");
    pixa = pixaCreate(0);

    AddTestSet(pixa, pixs, L_SOBEL_EDGE, 18, 40, 40, 0.7, -25, 280, 128);
    AddTestSet(pixa, pixs, L_TWO_SIDED_EDGE, 18, 40, 40, 0.7, -25, 280, 128);
    AddTestSet(pixa, pixs, L_SOBEL_EDGE, 10, 40, 40, 0.7, -15, 305, 128);
    AddTestSet(pixa, pixs, L_TWO_SIDED_EDGE, 10, 40, 40, 0.7, -15, 305, 128);
    AddTestSet(pixa, pixs, L_SOBEL_EDGE, 15, 40, 40, 0.6, -45, 285, 158);
    AddTestSet(pixa, pixs, L_TWO_SIDED_EDGE, 15, 40, 40, 0.6, -45, 285, 158);

    pixDestroy(&pixs);
    pixd = pixaDisplayTiledInColumns(pixa, 6, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 0 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);

    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    return regTestCleanup(rp);
}


void
AddTestSet(PIXA      *pixa,
           PIX       *pixs,
           l_int32    filtertype,
           l_int32    edgethresh,
           l_int32    smoothx,
           l_int32    smoothy,
           l_float32  gamma,
           l_int32    minval,
           l_int32    maxval,
           l_int32    targetthresh)
{
PIX  *pix1, *pix2, *pix3;

    pixThresholdSpreadNorm(pixs, filtertype, edgethresh,
                           smoothx, smoothy, gamma, minval,
                           maxval, targetthresh, &pix1, NULL, &pix2);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixThresholdToBinary(pix2, targetthresh - 20);
    pixaAddPix(pixa, pix3, L_INSERT);
    pix3 = pixThresholdToBinary(pix2, targetthresh);
    pixaAddPix(pixa, pix3, L_INSERT);
    pix3 = pixThresholdToBinary(pix2, targetthresh + 20);
    pixaAddPix(pixa, pix3, L_INSERT);
    pix3 = pixThresholdToBinary(pix2, targetthresh + 40);
    pixaAddPix(pixa, pix3, L_INSERT);
    return;
}
