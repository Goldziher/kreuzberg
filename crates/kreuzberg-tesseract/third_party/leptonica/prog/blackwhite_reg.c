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
 * blackwhite_reg.c
 *
 *   Tests functions that handle black and white pixels in an image.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

const char *fnames[11] = {"test1.png", "speckle2.png", "weasel2.4g.png",
                          "speckle4.png", "weasel4.11c.png",
                          "dreyfus8.png", "weasel8.240c.png",
                          "test16.tif", "marge.jpg",
                          "test-cmap-alpha.png", "test-gray-alpha.png"};
const l_int32  setsize = 11;


int main(int    argc,
         char **argv)
{
l_int32       i, spp;
l_uint32      bval, wval;
PIX          *pixs, *pix1, *pix2, *pix3, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Scale each image and add a white boundary */
    pixa = pixaCreate(setsize);
    for (i = 0; i < setsize; i++) {
        pixs = pixRead(fnames[i]);
        spp = pixGetSpp(pixs);
        pixGetBlackOrWhiteVal(pixs, L_GET_WHITE_VAL, &wval);
        pixGetBlackOrWhiteVal(pixs, L_GET_BLACK_VAL, &bval);
        lept_stderr("d = %d, spp = %d, bval = %x, wval = %x\n",
                pixGetDepth(pixs), spp, bval, wval);
        if (spp == 4)  /* remove alpha, using white background */
            pix1 = pixAlphaBlendUniform(pixs, wval);
        else
            pix1 = pixClone(pixs);
        pix2 = pixScaleToSize(pix1, 150, 150);
        pixGetBlackOrWhiteVal(pix2, L_GET_WHITE_VAL, &wval);
        pix3 = pixAddBorderGeneral(pix2, 30, 30, 20, 20, wval);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pixs);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    pixd = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 1, 30, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    pixDisplayWithTitle(pixd, 0, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

        /* Scale each image and add a black boundary */
    pixa = pixaCreate(setsize);
    for (i = 0; i < setsize; i++) {
        pixs = pixRead(fnames[i]);
        spp = pixGetSpp(pixs);
        pixGetBlackOrWhiteVal(pixs, L_GET_WHITE_VAL, &wval);
        pixGetBlackOrWhiteVal(pixs, L_GET_BLACK_VAL, &bval);
        lept_stderr("d = %d, spp = %d, bval = %x, wval = %x\n",
                pixGetDepth(pixs), spp, bval, wval);
        if (spp == 4)  /* remove alpha, using white background */
            pix1 = pixAlphaBlendUniform(pixs, wval);
        else
            pix1 = pixClone(pixs);
        pix2 = pixScaleToSize(pix1, 150, 150);
        pixGetBlackOrWhiteVal(pixs, L_GET_BLACK_VAL, &bval);
        pix3 = pixAddBorderGeneral(pix2, 30, 30, 20, 20, bval);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pixs);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    pixd = pixaDisplayTiledInRows(pixa, 32, 1200, 1.0, 0, 30, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    pixDisplayWithTitle(pixd, 1000, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    return regTestCleanup(rp);
}
