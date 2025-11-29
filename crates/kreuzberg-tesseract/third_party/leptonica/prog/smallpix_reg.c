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
 *  smallpix_reg.c
 *
 *  This is a regression test for scaling and rotation.
 *
 *  The question to be answered is: in the quantization, where, if
 *  anywhere, do we add 0.5?
 *
 *  The answer is that it should usually, but not always, be omitted.
 *  To see this, we operate on a very small pix and for visualization,
 *  scale up with replication to avoid aliasing and shifting.
 *
 *  To determine that the current implementations in scalelow.c,
 *  rotate.c and rotateamlow.c are better, change the specific
 *  implementations and re-run.
 *
 *  In all cases here, the pix to be operated on is of odd size
 *  so that the center pixel is symmetrically located, and there
 *  are a couple of black pixels outside the pattern so that edge
 *  effects (e.g., in pixScaleSmooth()) do not affect the results.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void SaveAndDisplayPix(L_REGPARAMS *rp, PIXA **ppixa, l_int32 x, l_int32 y);

int main(int    argc,
         char **argv)
{
l_int32       i;
l_float32     pi, scale, angle;
PIX          *pixc, *pixm, *pix1, *pix2, *pix3;
PIXA         *pixa;
PTA          *pta1, *pta2, *pta3, *pta4;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Make a small test image, the hard way! */
    pi = 3.1415926535;
    pixc = pixCreate(9, 9, 32);
    pixm = pixCreate(9, 9, 1);
    pta1 = generatePtaLineFromPt(4, 4, 3.1, 0.0);
    pta2 = generatePtaLineFromPt(4, 4, 3.1, 0.5 * pi);
    pta3 = generatePtaLineFromPt(4, 4, 3.1, pi);
    pta4 = generatePtaLineFromPt(4, 4, 3.1, 1.5 * pi);
    ptaJoin(pta1, pta2, 0, -1);
    ptaJoin(pta1, pta3, 0, -1);
    ptaJoin(pta1, pta4, 0, -1);
    pixRenderPta(pixm, pta1, L_SET_PIXELS);
    pixPaintThroughMask(pixc, pixm, 0, 0, 0x00ff0000);
    ptaDestroy(&pta1);
    ptaDestroy(&pta2);
    ptaDestroy(&pta3);
    ptaDestroy(&pta4);
    pixDestroy(&pixm);

        /* Results differ for scaleSmoothLow() w/ and w/out + 0.5.
         * Neither is properly symmetric (with symm pattern on odd-sized
         * pix, because the smoothing is destroying the symmetry. */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 2);
    for (i = 0; i < 11; i++) {
        scale = 0.30 + 0.035 * (l_float32)i;
        pix2 = pixScaleSmooth(pix1, scale, scale);
        pix3 = pixExpandReplicate(pix2, 6);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 100);  /* 0 */

        /* Results same for pixScaleAreaMap w/ and w/out + 0.5 */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 2);
    for (i = 0; i < 11; i++) {
        scale = 0.30 + 0.035 * (l_float32)i;
        pix2 = pixScaleAreaMap(pix1, scale, scale);
        pix3 = pixExpandReplicate(pix2, 6);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 200);  /* 1 */

        /* Results better for pixScaleBySampling with + 0.5, for small,
         * odd-dimension pix.  */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 2);
    for (i = 0; i < 11; i++) {
        scale = 0.30 + 0.035 * (l_float32)i;
        pix2 = pixScaleBySampling(pix1, scale, scale);
        pix3 = pixExpandReplicate(pix2, 6);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 300);  /* 2 */

        /* Results same for pixRotateAM w/ and w/out + 0.5 */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 1);
    for (i = 0; i < 11; i++) {
        angle = 0.10 + 0.05 * (l_float32)i;
        pix2 = pixRotateAM(pix1, angle, L_BRING_IN_BLACK);
        pix3 = pixExpandReplicate(pix2, 8);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 400);  /* 3 */

        /* If the size is odd, we express the center exactly, and the
         * results are better for pixRotateBySampling() w/out 0.5
         * However, if the size is even, the center value is not
         * exact, and if we choose it 0.5 smaller than the actual
         * center, we get symmetrical results with +0.5.
         * So we choose not to include + 0.5. */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 1);
    for (i = 0; i < 11; i++) {
        angle = 0.10 + 0.05 * (l_float32)i;
        pix2 = pixRotateBySampling(pix1, 4, 4, angle, L_BRING_IN_BLACK);
        pix3 = pixExpandReplicate(pix2, 8);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 500);  /* 4 */

        /* Results same for pixRotateAMCorner w/ and w/out + 0.5 */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 1);
    for (i = 0; i < 11; i++) {
        angle = 0.10 + 0.05 * (l_float32)i;
        pix2 = pixRotateAMCorner(pix1, angle, L_BRING_IN_BLACK);
        pix3 = pixExpandReplicate(pix2, 8);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 600);  /* 5 */

        /* Results better for pixRotateAMColorFast without + 0.5 */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 1);
    for (i = 0; i < 11; i++) {
        angle = 0.10 + 0.05 * (l_float32)i;
        pix2 = pixRotateAMColorFast(pix1, angle, 0);
        pix3 = pixExpandReplicate(pix2, 8);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 700);  /* 6 */

        /* Results slightly better for pixScaleColorLI() w/out + 0.5 */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 1);
    for (i = 0; i < 11; i++) {
        scale = 1.0 + 0.2 * (l_float32)i;
        pix2 = pixScaleColorLI(pix1, scale, scale);
        pix3 = pixExpandReplicate(pix2, 4);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 800);  /* 7 */

        /* Results slightly better for pixScaleColorLI() w/out + 0.5 */
    pixa = pixaCreate(11);
    pix1 = pixExpandReplicate(pixc, 1);
    for (i = 0; i < 11; i++) {
        scale = 1.0 + 0.2 * (l_float32)i;
        pix2 = pixScaleLI(pix1, scale, scale);
        pix3 = pixExpandReplicate(pix2, 4);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pixDestroy(&pix1);
    SaveAndDisplayPix(rp, &pixa, 100, 940);  /* 8 */

    pixDestroy(&pixc);
    return regTestCleanup(rp);
}

void
SaveAndDisplayPix(L_REGPARAMS  *rp,
                  PIXA        **ppixa,
                  l_int32       x,
                  l_int32       y)
{
PIX  *pix1;

    pix1 = pixaDisplayTiledInColumns(*ppixa, 12, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);
    pixDisplayWithTitle(pix1, x, y, NULL, rp->display);
    pixaDestroy(ppixa);
    pixDestroy(&pix1);
}
