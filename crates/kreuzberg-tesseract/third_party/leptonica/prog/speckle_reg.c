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
 *  speckle_reg.c
 *
 *    Image normalization to get an image with speckle background
 *    noise, followed by attempts to remove some of the speckle.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* HMT (with just misses) for speckle up to 2x2 */
static const char *selstr2 = "oooo"
                             "oC o"
                             "o  o"
                             "oooo";
    /* HMT (with just misses) for speckle up to 3x3 */
static const char *selstr3 = "ooooo"
                             "oC  o"
                             "o   o"
                             "o   o"
                             "ooooo";

int main(int    argc,
         char **argv)
{
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5;
PIX          *pix6, *pix7, *pix8, *pix9, *pix10;
PIXA         *pixa1;
SEL          *sel1, *sel2, *sel3, *sel4;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /*  Normalize for rapidly varying background */
    pixa1 = pixaCreate(0);
    pixs = pixRead("w91frag.jpg");
    pixaAddPix(pixa1, pixs, L_INSERT);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 0 */
    pix1 = pixBackgroundNormFlex(pixs, 7, 7, 1, 1, 10);
    pixaAddPix(pixa1, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 1 */

        /* Remove the background */
    pix2 = pixGammaTRCMasked(NULL, pix1, NULL, 1.0, 100, 175);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 2 */

        /* Binarize */
    pix3 = pixThresholdToBinary(pix2, 180);
    pixaAddPix(pixa1, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */

        /* Remove the speckle noise up to 2x2 */
    sel1 = selCreateFromString(selstr2, 4, 4, "speckle2");
    pix4 = pixHMT(NULL, pix3, sel1);
    pixaAddPix(pixa1, pix4, L_INSERT);
    sel2 = selCreateBrick(2, 2, 0, 0, SEL_HIT);
    pix5 = pixDilate(NULL, pix4, sel2);
    pixaAddPix(pixa1, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 4 */
    pix6 = pixSubtract(NULL, pix3, pix5);
    pixaAddPix(pixa1, pix6, L_INSERT);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 5 */

        /* Remove the speckle noise up to 3x3 */
    sel3 = selCreateFromString(selstr3, 5, 5, "speckle3");
    pix7 = pixHMT(NULL, pix3, sel3);
    pixaAddPix(pixa1, pix7, L_INSERT);
    sel4 = selCreateBrick(3, 3, 0, 0, SEL_HIT);
    pix8 = pixDilate(NULL, pix7, sel4);
    pixaAddPix(pixa1, pix8, L_INSERT);
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 6 */
    pix9 = pixSubtract(NULL, pix3, pix8);
    pixaAddPix(pixa1, pix9, L_INSERT);
    regTestWritePixAndCheck(rp, pix9, IFF_PNG);  /* 7 */

    pix10 = pixaDisplayTiledInColumns(pixa1, 3, 1.0, 30, 2);
    pixDisplayWithTitle(pix10, 0, 0, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix10, IFF_JFIF_JPEG);  /* 8 */
    selDestroy(&sel1);
    selDestroy(&sel2);
    selDestroy(&sel3);
    selDestroy(&sel4);
    pixDestroy(&pix2);
    pixDestroy(&pix10);
    pixaDestroy(&pixa1);
    return regTestCleanup(rp);
}


