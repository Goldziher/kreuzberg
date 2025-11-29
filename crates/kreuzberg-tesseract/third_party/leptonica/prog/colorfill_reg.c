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
 *   colorfill_reg.c
 *
 *   This tests the utility that does color segmentation by region growing.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static PIX *makeSmallTestPix(l_uint32 c1, l_uint32 c2);


int main(int    argc,
         char **argv)
{
L_COLORFILL  *cf;
PIX          *pix1, *pix2, *pix3, *pix4;
PIXA         *pixa1, *pixa2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Test on a small image */
    pix1 = makeSmallTestPix(0x3070A000, 0xA0703000);
    pix2 = pixExpandReplicate(pix1, 15);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix2, 0, 0, NULL, rp->display);
    pixDestroy(&pix2);
    cf = l_colorfillCreate(pix1, 1, 1);
    pixColorContentByLocation(cf, 0, 0, 0, 70, 15, 3, 1, 1);
    pix2 = pixaDisplayTiledInColumns(cf->pixadb, cf->nx, 1.0, 10, 1);
    pix3 = pixExpandReplicate(pix2, 10);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix3, 300, 0, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    l_colorfillDestroy(&cf);

        /* Test on simple random image with many colors (1 tile and 4 tiles */
    pixa1 = makeColorfillTestData(350, 350, 35, 100);
    pixa2 = pixaCreate(5);
    pix1 = pixaGetPix(pixa1, 0, L_COPY);
    pix2 = pixConvertTo32(pix1);
    pixDestroy(&pix1);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 2 */
    pixaAddPix(pixa2, pix2, L_INSERT);
    cf = l_colorfillCreate(pix2, 1, 1);  /* 1 tile */
    pixColorContentByLocation(cf, 0, 0, 0, 70, 30, 500, 1, 1);
    pix3 = pixaDisplayTiledInColumns(cf->pixam, cf->nx, 1.0, 10, 1);
    pix4 = pixaDisplayTiledInColumns(cf->pixadb, cf->nx, 1.0, 10, 1);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 4 */
    pixaAddPix(pixa2, pix3, L_INSERT);
    pixaAddPix(pixa2, pix4, L_INSERT);
    l_colorfillDestroy(&cf);

    cf = l_colorfillCreate(pix2, 2, 2);  /* 4 tiles */
    pixColorContentByLocation(cf, 0, 0, 0, 70, 30, 500, 1, 1);
    pix3 = pixaDisplayTiledInColumns(cf->pixam, cf->nx, 1.0, 10, 1);
    pix4 = pixaDisplayTiledInColumns(cf->pixadb, cf->nx, 1.0, 10, 1);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 5 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 6 */
    pixaAddPix(pixa2, pix3, L_INSERT);
    pixaAddPix(pixa2, pix4, L_INSERT);
    if (rp->display) {
        pix1 = pixaDisplayTiledInColumns(pixa2, 5, 1.0, 15, 2);
        pixDisplay(pix1, 0, 400);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    l_colorfillDestroy(&cf);

        /* Test on an image with lots of color (with 1 tile and 9 tiles) */
    pix1 = pixRead("lyra.005.jpg");
    pix2 = pixScale(pix1, 0.5, 0.5);
    pixDestroy(&pix1);
    cf = l_colorfillCreate(pix2, 1, 1);  /* 1 tile */
    pixColorContentByLocation(cf, 0, 0, 0, 70, 30, 100, 1, 1);
    pix3 = pixaDisplayTiledInColumns(cf->pixam, cf->nx, 1.0, 10, 1);
    pix4 = pixaDisplayTiledInColumns(cf->pixadb, cf->nx, 1.0, 10, 1);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 7 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 8 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 9 */
    pixa2 = pixaCreate(3);
    pixaAddPix(pixa2, pix2, L_COPY);
    pixaAddPix(pixa2, pix3, L_INSERT);
    pixaAddPix(pixa2, pix4, L_INSERT);
    if (rp->display) {
        pix1 = pixaDisplayTiledInColumns(pixa2, 3, 0.8, 15, 2);
        pixDisplay(pix1, 0, 650);
        pixDestroy(&pix1);
    }
    l_colorfillDestroy(&cf);
    pixaDestroy(&pixa2);

    cf = l_colorfillCreate(pix2, 3, 3);  /* 9 tiles */
    pixColorContentByLocation(cf, 0, 0, 0, 70, 30, 100, 1, 1);
    pix3 = pixaDisplayTiledInColumns(cf->pixam, cf->nx, 1.0, 10, 1);
    pix4 = pixaDisplayTiledInColumns(cf->pixadb, cf->nx, 1.0, 10, 1);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 10 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 11 */
    pixa2 = pixaCreate(2);
    pixaAddPix(pixa2, pix3, L_INSERT);
    pixaAddPix(pixa2, pix4, L_INSERT);
    if (rp->display) {
        pix1 = pixaDisplayTiledInColumns(pixa2, 2, 0.8, 15, 2);
        pixDisplay(pix1, 0, 1000);
        pixDestroy(&pix1);
    }
    l_colorfillDestroy(&cf);
    pixDestroy(&pix2);
    pixaDestroy(&pixa2);

    return regTestCleanup(rp);
}

static PIX *makeSmallTestPix(l_uint32 c1, l_uint32 c2)
{
l_int32  i, j;
PIX     *pix1;

    pix1 = pixCreate(17, 17, 32);
    pixSetAllArbitrary(pix1, c1);
    for (i = 0; i < 15; i++) {
        for (j = 0; j < i; j++)
            pixSetPixel(pix1, j, i, c2);
    }
    for (i = 0; i < 15; i++) {
        for (j = 17 - i; j < 17; j++)
            pixSetPixel(pix1, j, i, c2);
    }
    for (i = 9; i < 17; i++)
        pixSetPixel(pix1, 8, i, c1);
    return pix1;
}
