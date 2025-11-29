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
 *  adaptnorm_reg.c
 *
 *    Image normalization for two extreme cases:
 *       * variable and low contrast
 *       * good contrast but fast varying background
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32      w, h;
l_float32    mps;
PIX         *pixs, *pixmin, *pix1, *pix2, *pix3, *pix4, *pix5;
PIX         *pix6, *pix7, *pix8, *pix9, *pix10, *pix11;
PIXA        *pixa1;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ---------------------------------------------------------- *
     *     Normalize by adaptively expanding the dynamic range    *
     * ---------------------------------------------------------- */
    pixa1 = pixaCreate(0);
    pixs = pixRead("lighttext.jpg");
    pixGetDimensions(pixs, &w, &h, NULL);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 0 */
    pixaAddPix(pixa1, pixs, L_INSERT);
    startTimer();
    pix1 = pixContrastNorm(NULL, pixs, 10, 10, 40, 2, 2);
    mps = 0.000001 * w * h / stopTimer();
    lept_stderr("Time: Contrast norm: %7.3f Mpix/sec\n", mps);
    pixaAddPix(pixa1, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 1 */

         /* Apply a gamma to clean up the remaining background */
    pix2 = pixGammaTRC(NULL, pix1, 1.5, 50, 235);
    pixaAddPix(pixa1, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 2 */

         /* Here are two possible output display images; a dithered
          * 2 bpp image and a 7 level thresholded 4 bpp image */
    pix3 = pixDitherTo2bpp(pix2, 1);
    pixaAddPix(pixa1, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */
    pix4 = pixThresholdTo4bpp(pix2, 7, 1);
    pixaAddPix(pixa1, pix4, L_INSERT);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 4 */

         /* Binary image produced from 8 bpp normalized ones,
          * before and after the gamma correction. */
    pix5 = pixThresholdToBinary(pix1, 180);
    pixaAddPix(pixa1, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 5 */
    pix6 = pixThresholdToBinary(pix2, 200);
    pixaAddPix(pixa1, pix6, L_INSERT);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 6 */

    pix7 = pixaDisplayTiledInColumns(pixa1, 3, 1.0, 30, 2);
    pixDisplayWithTitle(pix7, 0, 0, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix7, IFF_JFIF_JPEG);  /* 7 */
    pixDestroy(&pix7);
    pixaDestroy(&pixa1);

    /* ---------------------------------------------------------- *
     *          Normalize for rapidly varying background          *
     * ---------------------------------------------------------- */
    pixa1 = pixaCreate(0);
    pixs = pixRead("w91frag.jpg");
    pixGetDimensions(pixs, &w, &h, NULL);
    pixaAddPix(pixa1, pixs, L_INSERT);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 8 */
    startTimer();
    pix1 = pixBackgroundNormFlex(pixs, 7, 7, 1, 1, 10);
    mps = 0.000001 * w * h / stopTimer();
    lept_stderr("Time: Flexible bg norm: %7.3f Mpix/sec\n", mps);
    pixaAddPix(pixa1, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 9 */

        /* Now do it again in several steps */
    pix2 = pixScaleSmooth(pixs, 1./7., 1./7.);
    pix3 = pixScale(pix2, 7.0, 7.0);
    pixaAddPix(pixa1, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 10 */
    pixLocalExtrema(pix2, 0, 0, &pixmin, NULL);  /* 1's at minima */
    pix4 = pixExpandBinaryReplicate(pixmin, 7, 7);
    pixaAddPix(pixa1, pix4, L_INSERT);
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 11 */
    pix5 = pixSeedfillGrayBasin(pixmin, pix2, 10, 4);
    pix6 = pixExtendByReplication(pix5, 1, 1);
    regTestWritePixAndCheck(rp, pix6, IFF_JFIF_JPEG);  /* 12 */
    pixDestroy(&pixmin);
    pixDestroy(&pix5);
    pixDestroy(&pix2);
    pix7 = pixGetInvBackgroundMap(pix6, 200, 1, 1);  /* smoothing incl. */
    pix8 = pixApplyInvBackgroundGrayMap(pixs, pix7, 7, 7);
    pixaAddPix(pixa1, pix8, L_INSERT);
    regTestWritePixAndCheck(rp, pix8, IFF_JFIF_JPEG);  /* 13 */
    pixDestroy(&pix6);
    pixDestroy(&pix7);

        /* Process the result for gray and binary output */
    pix9 = pixGammaTRCMasked(NULL, pix1, NULL, 1.0, 100, 175);
    pixaAddPix(pixa1, pix9, L_INSERT);
    regTestWritePixAndCheck(rp, pix9, IFF_JFIF_JPEG);  /* 14 */
    pix10 = pixThresholdTo4bpp(pix9, 10, 1);
    pixaAddPix(pixa1, pix10, L_INSERT);
    regTestWritePixAndCheck(rp, pix10, IFF_JFIF_JPEG);  /* 15 */
    pix11 = pixThresholdToBinary(pix9, 190);
    pixaAddPix(pixa1, pix11, L_INSERT);
    regTestWritePixAndCheck(rp, pix11, IFF_JFIF_JPEG);  /* 16 */

    pix2 = pixaDisplayTiledInColumns(pixa1, 3, 1.0, 30, 2);
    pixDisplayWithTitle(pix2, 0, 700, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 17 */
    pixDestroy(&pix2);
    pixaDestroy(&pixa1);
    return regTestCleanup(rp);
}


