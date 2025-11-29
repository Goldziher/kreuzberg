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
 * expand_reg.c
 *
 *   Regression test for replicative and power-of-2 expansion (and
 *   corresponding reductions)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  BINARY_IMAGE             "test1.png"
#define  TWO_BPP_IMAGE_NO_CMAP    "weasel2.4g.png"
#define  TWO_BPP_IMAGE_CMAP       "weasel2.4c.png"
#define  FOUR_BPP_IMAGE_NO_CMAP   "weasel4.16g.png"
#define  FOUR_BPP_IMAGE_CMAP      "weasel4.16c.png"
#define  EIGHT_BPP_IMAGE_NO_CMAP  "weasel8.149g.png"
#define  EIGHT_BPP_IMAGE_CMAP     "weasel8.240c.png"
#define  RGB_IMAGE                "marge.jpg"


int main(int    argc,
         char **argv)
{
l_int32      i, w, h, format;
char         filename[][64] = {BINARY_IMAGE,
                               TWO_BPP_IMAGE_NO_CMAP, TWO_BPP_IMAGE_CMAP,
                               FOUR_BPP_IMAGE_NO_CMAP, FOUR_BPP_IMAGE_CMAP,
                               EIGHT_BPP_IMAGE_NO_CMAP, EIGHT_BPP_IMAGE_CMAP,
                               RGB_IMAGE};
BOX          *box;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(0);
    for (i = 0; i < 8; i++) {
        pixs = pixRead(filename[i]);
        pix1 = pixExpandReplicate(pixs, 2);
        format = (i == 7) ? IFF_JFIF_JPEG : IFF_PNG;
        regTestWritePixAndCheck(rp, pix1, format);  /* 0 - 7 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pixDestroy(&pixs);
    }
    for (i = 0; i < 8; i++) {
        pixs = pixRead(filename[i]);
        pix1 = pixExpandReplicate(pixs, 3);
        format = (i == 7) ? IFF_JFIF_JPEG : IFF_PNG;
        regTestWritePixAndCheck(rp, pix1, format);  /* 8 - 15 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pixDestroy(&pixs);
    }

    pixs = pixRead("test1.png");
    pixGetDimensions(pixs, &w, &h, NULL);
    for (i = 1; i <= 15; i++) {
        box = boxCreate(13 * i, 13 * i, w - 13 * i, h - 13 * i);
        pix1 = pixClipRectangle(pixs, box, NULL);
        pix2 = pixExpandReplicate(pix1, 3);
        regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 16 - 30 */
        pixaAddPix(pixa, pix2, L_INSERT);
        boxDestroy(&box);
        pixDestroy(&pix1);
    }
    pixDestroy(&pixs);

    /* --------- Power of 2 expansion and reduction -------- */
    pixs = pixRead("speckle.png");

        /* Test 2x expansion of 1 bpp */
    pix1 = pixExpandBinaryPower2(pixs, 2);
    pix2 = pixReduceRankBinary2(pix1, 4, NULL);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 31 */
    regTestComparePix(rp, pixs, pix2);  /* 32 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Test 2x expansion of 2 bpp */
    pix1 = pixConvert1To2(NULL, pixs, 3, 0);
    pix2 = pixExpandReplicate(pix1, 2);
    pix3 = pixConvertTo8(pix2, FALSE);
    pix4 = pixThresholdToBinary(pix3, 250);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 33 */
    pix5 = pixReduceRankBinary2(pix4, 4, NULL);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 34 */
    regTestComparePix(rp, pixs, pix5);  /* 35 */
    pixaAddPix(pixa, pix5, L_INSERT);
    pix6 = pixExpandBinaryPower2(pix5, 2);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 36 */
    pixaAddPix(pixa, pix6, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Test 4x expansion of 4 bpp */
    pix1 = pixConvert1To4(NULL, pixs, 15, 0);
    pix2 = pixExpandReplicate(pix1, 4);
    pix3 = pixConvertTo8(pix2, FALSE);
    pix4 = pixThresholdToBinary(pix3, 250);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 37 */
    pixaAddPix(pixa, pix4, L_INSERT);
    pix5 = pixReduceRankBinaryCascade(pix4, 4, 4, 0, 0);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 38 */
    regTestComparePix(rp, pixs, pix5);  /* 39 */
    pixaAddPix(pixa, pix5, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Test 8x expansion of 8 bpp */
    pix1 = pixConvertTo8(pixs, FALSE);
    pix2 = pixExpandReplicate(pix1, 8);
    pix3 = pixThresholdToBinary(pix2, 250);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 40 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pix4 = pixReduceRankBinaryCascade(pix3, 4, 4, 4, 0);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 41 */
    regTestComparePix(rp, pixs, pix4);  /* 42 */
    pixaAddPix(pixa, pix4, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pixs);

    if (rp->display) {
        lept_stderr("Writing to: /tmp/lept/expand/test.pdf\n");
        pixaConvertToPdf(pixa, 0, 1.0, 0, 0, "Replicative expansion",
                         "/tmp/lept/expand/test.pdf");
    }
    pixaDestroy(&pixa);

    return regTestCleanup(rp);
}

