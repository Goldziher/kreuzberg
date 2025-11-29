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
 * lineremoval_reg.c
 *
 *     A fun little application, saved as a regression test.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_float32     angle, conf, deg2rad;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5;
PIX          *pix6, *pix7, *pix8, *pix9;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    deg2rad = 3.14159 / 180.;
    pixs = pixRead("dave-orig.png");
    pixa = pixaCreate(0);

        /* Threshold to binary, extracting much of the lines */
    pix1 = pixThresholdToBinary(pixs, 170);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Find the skew angle and deskew using an interpolated
         * rotator for anti-aliasing (to avoid jaggies) */
    pixFindSkew(pix1, &angle, &conf);
    pix2 = pixRotateAMGray(pixs, deg2rad * angle, 255);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Extract the lines to be removed */
    pix3 = pixCloseGray(pix2, 51, 1);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pix3, L_INSERT);

        /* Solidify the lines to be removed */
    pix4 = pixErodeGray(pix3, 1, 5);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 3 */
    pixaAddPix(pixa, pix4, L_INSERT);

        /* Clean the background of those lines */
    pix5 = pixThresholdToValue(NULL, pix4, 210, 255);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 4 */
    pixaAddPix(pixa, pix5, L_INSERT);

    pix6 = pixThresholdToValue(NULL, pix5, 200, 0);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pix6, L_COPY);

        /* Get paint-through mask for changed pixels */
    pix7 = pixThresholdToBinary(pix6, 210);
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 6 */
    pixaAddPix(pixa, pix7, L_INSERT);

        /* Add the inverted, cleaned lines to orig.  Because
         * the background was cleaned, the inversion is 0,
         * so when you add, it doesn't lighten those pixels.
         * It only lightens (to white) the pixels in the lines! */
    pixInvert(pix6, pix6);
    pix8 = pixAddGray(NULL, pix2, pix6);
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 7 */
    pixaAddPix(pixa, pix8, L_COPY);

    pix9 = pixOpenGray(pix8, 1, 9);
    regTestWritePixAndCheck(rp, pix9, IFF_PNG);  /* 8 */
    pixaAddPix(pixa, pix9, L_INSERT);
    pixCombineMasked(pix8, pix9, pix7);
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 9 */
    pixaAddPix(pixa, pix8, L_INSERT);

    if (rp->display) {
        lept_rmdir("lept/lines");
        lept_mkdir("lept/lines");
        lept_stderr("Writing to: /tmp/lept/lines/lineremoval.pdf\n");
        pixaConvertToPdf(pixa, 0, 1.0, L_FLATE_ENCODE, 0, "lineremoval example",
                         "/tmp/lept/lines/lineremoval.pdf");
        pix1 = pixaDisplayTiledInColumns(pixa, 5, 0.5, 30, 2);
        pixWrite("/tmp/lept/lines/lineremoval.jpg", pix1, IFF_JFIF_JPEG);
        pixDisplay(pix1, 100, 100);
        pixDestroy(&pix1);
    }

    pixaDestroy(&pixa);
    pixDestroy(&pixs);
    pixDestroy(&pix6);
    return regTestCleanup(rp);
}

