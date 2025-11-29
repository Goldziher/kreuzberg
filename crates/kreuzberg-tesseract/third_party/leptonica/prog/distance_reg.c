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
 * distance_reg.c
 *
 *   This tests pixDistanceFunction for a variety of usage
 *   with all 8 combinations of these parameters:
 *
 *     connectivity :   4 or 8
 *     dest depth :     8 or 16
 *     boundary cond :  L_BOUNDARY_BG or L_BOUNDARY_FG
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void TestDistance(PIXA *pixa, PIX *pixs, l_int32 conn,
                         l_int32 depth, l_int32 bc, L_REGPARAMS *rp);

#define  DEBUG    0


int main(int    argc,
         char **argv)
{
l_int32       i, j, k, index, conn, depth, bc;
BOX          *box;
PIX          *pix, *pixs, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pix = pixRead("feyn.tif");
    box = boxCreate(383, 338, 1480, 1050);
    pixs = pixClipRectangle(pix, box, NULL);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 0 */

    for (i = 0; i < 2; i++) {
        conn = 4 + 4 * i;
        for (j = 0; j < 2; j++) {
            depth = 8 + 8 * j;
            for (k = 0; k < 2; k++) {
                bc = k + 1;
                index = 4 * i + 2 * j + k;
                lept_stderr("Set %d\n", index);
                if (DEBUG) {
                    lept_stderr("%d: conn = %d, depth = %d, bc = %d\n",
                                rp->index + 1, conn, depth, bc);
                }
                pixa = pixaCreate(0);
                pixaAddPix(pixa, pixs, L_COPY);
                TestDistance(pixa, pixs, conn, depth, bc, rp);
                pixd = pixaDisplayTiledInColumns(pixa, 4, 1.0, 20, 2);
                pixDisplayWithTitle(pixd, 0, 0, NULL, rp->display);
                pixaDestroy(&pixa);
                pixDestroy(&pixd);
            }
        }
    }

    boxDestroy(&box);
    pixDestroy(&pix);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}


static void
TestDistance(PIXA         *pixa,
             PIX          *pixs,
             l_int32       conn,
             l_int32       depth,
             l_int32       bc,
             L_REGPARAMS  *rp)
{
PIX  *pixt1, *pixt2, *pixt3, *pixt4, *pixt5;

        /* Test the distance function and display */
    pixInvert(pixs, pixs);
    pixt1 = pixDistanceFunction(pixs, conn, depth, bc);
    regTestWritePixAndCheck(rp, pixt1, IFF_PNG);  /* a + 1 */
    pixaAddPix(pixa, pixt1, L_INSERT);
    pixInvert(pixs, pixs);
    pixt2 = pixMaxDynamicRange(pixt1, L_LOG_SCALE);
    regTestWritePixAndCheck(rp, pixt2, IFF_JFIF_JPEG);  /* a + 2 */
    pixaAddPix(pixa, pixt2, L_INSERT);

        /* Test the distance function and display with contour rendering */
    pixInvert(pixs, pixs);
    pixt1 = pixDistanceFunction(pixs, conn, depth, bc);
    regTestWritePixAndCheck(rp, pixt1, IFF_PNG);  /* a + 3 */
    pixaAddPix(pixa, pixt1, L_INSERT);
    pixInvert(pixs, pixs);
    pixt2 = pixRenderContours(pixt1, 2, 4, 1);  /* binary output */
    regTestWritePixAndCheck(rp, pixt2, IFF_PNG);  /* a + 4 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixt3 = pixRenderContours(pixt1, 2, 4, depth);
    pixt4 = pixMaxDynamicRange(pixt3, L_LINEAR_SCALE);
    regTestWritePixAndCheck(rp, pixt4, IFF_JFIF_JPEG);  /* a + 5 */
    pixaAddPix(pixa, pixt4, L_INSERT);
    pixt5 = pixMaxDynamicRange(pixt3, L_LOG_SCALE);
    regTestWritePixAndCheck(rp, pixt5, IFF_JFIF_JPEG);  /* a + 6 */
    pixaAddPix(pixa, pixt5, L_INSERT);
    pixDestroy(&pixt3);

        /* Label all pixels in each c.c. with a color equal to the
         * max distance of any pixel within that c.c. from the bg.
         * Note that we've normalized so the dynamic range extends
         * to 255.  For the image here, each unit of distance is
         * represented by about 21 grayscale units.  The largest
         * distance is 12.  */
    if (depth == 8) {
        pixt1 = pixDistanceFunction(pixs, conn, depth, bc);
        pixt4 = pixMaxDynamicRange(pixt1, L_LOG_SCALE);
        regTestWritePixAndCheck(rp, pixt4, IFF_JFIF_JPEG);  /* b + 1 */
        pixaAddPix(pixa, pixt4, L_INSERT);
        pixt2 = pixCreateTemplate(pixt1);
        pixSetMasked(pixt2, pixs, 255);
        regTestWritePixAndCheck(rp, pixt2, IFF_JFIF_JPEG);  /* b + 2 */
        pixaAddPix(pixa, pixt2, L_INSERT);
        pixSeedfillGray(pixt1, pixt2, 4);
        pixt3 = pixMaxDynamicRange(pixt1, L_LINEAR_SCALE);
        regTestWritePixAndCheck(rp, pixt3, IFF_JFIF_JPEG);  /* b + 3 */
        pixaAddPix(pixa, pixt3, L_INSERT);
        pixDestroy(&pixt1);
    }

    return;
}
