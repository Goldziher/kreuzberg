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
 *  projection_reg.c
 *
 *    Tests projection stats for rows and columns.
 *    Just for interest, a number of different tests are done.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void TestProjection(L_REGPARAMS *rp, PIX *pix);

int main(int    argc,
         char **argv)
{
PIX          *pixs, *pix1, *pix2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Use for input two different images */
    pixs = pixRead("projectionstats.jpg");
    pix1 = pixConvertTo8(pixs, 0);
    pixDestroy(&pixs);
    pixs = pixRead("feyn.tif");
    pix2 = pixScaleToGray4(pixs);
    pixDestroy(&pixs);

    TestProjection(rp, pix1);
    TestProjection(rp, pix2);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    return regTestCleanup(rp);
}


/*
 *  Test both vertical and horizontal projections on this image.
 *  Rotate the image by 90 degrees for the horizontal projection,
 *  so that the two results should be identical.
 */
void
TestProjection(L_REGPARAMS  *rp,
               PIX          *pixs)
{
NUMA    *na1, *na2, *na3, *na4, *na5, *na6;
NUMA    *na7, *na8, *na9, *na10, *na11, *na12;
PIX     *pixd, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIX     *pix7, *pix8, *pix9, *pix10, *pix11, *pix12;
PIXA    *pixa;

    pixColumnStats(pixs, NULL, &na1, &na3, &na5, &na7, &na9, &na11);
    pixd = pixRotateOrth(pixs, 1);
    pixRowStats(pixd, NULL, &na2, &na4, &na6, &na8, &na10, &na12);
    pixDestroy(&pixd);

    pix1 = gplotSimplePix1(na1, "Mean value");
    pix2 = gplotSimplePix1(na2, "Mean value");
    pix3 = gplotSimplePix1(na3, "Median value");
    pix4 = gplotSimplePix1(na4, "Median value");
    pix5 = gplotSimplePix1(na5, "Mode value");
    pix6 = gplotSimplePix1(na6, "Mode value");
    pix7 = gplotSimplePix1(na7, "Mode count");
    pix8 = gplotSimplePix1(na8, "Mode count");
    pix9 = gplotSimplePix1(na9, "Variance");
    pix10 = gplotSimplePix1(na10, "Variance");
    pix11 = gplotSimplePix1(na11, "Square Root Variance");
    pix12 = gplotSimplePix1(na12, "Square Root Variance");

         /* This is run twice, on two different images */
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0, 19 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1, 20 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2, 21 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 3, 22 */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 4, 23 */
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 5, 24 */
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 6, 25 */
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 7, 26 */
    regTestWritePixAndCheck(rp, pix9, IFF_PNG);  /* 8, 27 */
    regTestWritePixAndCheck(rp, pix10, IFF_PNG);  /* 9, 28 */
    regTestWritePixAndCheck(rp, pix11, IFF_PNG);  /* 10, 29 */
    regTestWritePixAndCheck(rp, pix12, IFF_PNG);  /* 11, 30 */

        /* Compare by pairs */
    regTestComparePix(rp, pix1, pix2);  /* 12, 31 */
    regTestComparePix(rp, pix3, pix4);  /* 13, 32 */
    regTestComparePix(rp, pix5, pix6);  /* 14, 33 */
    regTestComparePix(rp, pix7, pix8);  /* 15, 34 */
    regTestComparePix(rp, pix9, pix10);  /* 16, 35 */
    regTestComparePix(rp, pix11, pix12);  /* 17, 36 */

    pixa = pixaCreate(0);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix5, L_INSERT);
    pixaAddPix(pixa, pix6, L_INSERT);
    pixaAddPix(pixa, pix7, L_INSERT);
    pixaAddPix(pixa, pix8, L_INSERT);
    pixaAddPix(pixa, pix9, L_INSERT);
    pixaAddPix(pixa, pix10, L_INSERT);
    pixaAddPix(pixa, pix11, L_INSERT);
    pixaAddPix(pixa, pix12, L_INSERT);

    pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 18, 37 */
    pixDisplayWithTitle(pix1, 100, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    numaDestroy(&na4);
    numaDestroy(&na5);
    numaDestroy(&na6);
    numaDestroy(&na7);
    numaDestroy(&na8);
    numaDestroy(&na9);
    numaDestroy(&na10);
    numaDestroy(&na11);
    numaDestroy(&na12);
    return;
}
