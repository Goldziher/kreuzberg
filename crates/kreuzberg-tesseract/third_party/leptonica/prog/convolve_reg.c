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
 *  convolve_reg.c
 *
 *    Tests a number of convolution functions.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const char  *kel1str = " 20    50   80  50   20 "
                              " 50   100  140  100  50 "
                              " 90   160  200  160  90 "
                              " 50   100  140  100  50 "
                              " 20    50   80   50  20 ";

static const char  *kel2str = " -20  -50  -80  -50  -20 "
                              " -50   50   80   50  -50 "
                              " -90   90  200   90  -90 "
                              " -50   50   80   50  -50 "
                              " -20  -50  -80  -50  -20 ";

static const char  *kel3xstr = " -70   40  100   40  -70 ";
static const char  *kel3ystr = "  20  -70   40  100   40  -70  20 ";


int main(int    argc,
         char **argv)
{
l_int32       i, j, sizex, sizey, bias;
FPIX         *fpixv, *fpixrv;
L_KERNEL     *kel1, *kel2, *kel3x, *kel3y;
PIX          *pixs, *pixacc, *pixg, *pixt, *pixd;
PIX          *pixb, *pixm, *pixms, *pixrv, *pix1, *pix2, *pix3, *pix4;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Test pixBlockconvGray() on 8 bpp */
    pixs = pixRead("test8.jpg");
    pixacc = pixBlockconvAccum(pixs);
    pixd = pixBlockconvGray(pixs, pixacc, 3, 5);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 0 */
    pixDisplayWithTitle(pixd, 100, 0, NULL, rp->display);
    pixDestroy(&pixacc);
    pixDestroy(&pixd);

        /* Test pixBlockconv() on 8 bpp */
    pixd = pixBlockconv(pixs, 9, 8);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 1 */
    pixDisplayWithTitle(pixd, 200, 0, NULL, rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixs);

        /* Test pixBlockrank() on 1 bpp */
    pixs = pixRead("test1.png");
    pixacc = pixBlockconvAccum(pixs);
    for (i = 0; i < 3; i++) {
        pixd = pixBlockrank(pixs, pixacc, 4, 4, 0.25 + 0.25 * i);
        regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 2 - 4 */
        pixDisplayWithTitle(pixd, 300 + 100 * i, 0, NULL, rp->display);
        pixDestroy(&pixd);
    }

        /* Test pixBlocksum() on 1 bpp */
    pixd = pixBlocksum(pixs, pixacc, 16, 16);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 5 */
    pixDisplayWithTitle(pixd, 700, 0, NULL, rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixacc);
    pixDestroy(&pixs);

        /* Test pixCensusTransform() */
    pixs = pixRead("test24.jpg");
    pixg = pixScaleRGBToGrayFast(pixs, 2, COLOR_GREEN);
    pixd = pixCensusTransform(pixg, 10, NULL);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pixd, 800, 0, NULL, rp->display);
    pixDestroy(&pixd);

        /* Test generic convolution with kel1 */
    kel1 = kernelCreateFromString(5, 5, 2, 2, kel1str);
    pixd = pixConvolve(pixg, kel1, 8, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 7 */
    pixDisplayWithTitle(pixd, 100, 500, NULL, rp->display);
    pixDestroy(&pixd);

        /* Test convolution with flat rectangular kel */
    kel2 = kernelCreate(11, 11);
    kernelSetOrigin(kel2, 5, 5);
    for (i = 0; i < 11; i++) {
        for (j = 0; j < 11; j++)
            kernelSetElement(kel2, i, j, 1);
    }
    pixd = pixConvolve(pixg, kel2, 8, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 8 */
    pixDisplayWithTitle(pixd, 200, 500, NULL, rp->display);
    pixDestroy(&pixd);
    kernelDestroy(&kel1);
    kernelDestroy(&kel2);

        /* Test pixBlockconv() on 32 bpp */
    pixt = pixScaleBySampling(pixs, 0.5, 0.5);
    pixd = pixBlockconv(pixt, 4, 6);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 9 */
    pixDisplayWithTitle(pixd, 300, 500, NULL, rp->display);
    pixDestroy(&pixt);
    pixDestroy(&pixs);
    pixDestroy(&pixg);
    pixDestroy(&pixd);

        /* Test bias convolution non-separable with kel2 */
    pixs = pixRead("marge.jpg");
    pixg = pixScaleRGBToGrayFast(pixs, 2, COLOR_GREEN);
    kel2 = kernelCreateFromString(5, 5, 2, 2, kel2str);
    pixd = pixConvolveWithBias(pixg, kel2, NULL, TRUE, &bias);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 10 */
    pixDisplayWithTitle(pixd, 400, 500, NULL, rp->display);
    lept_stderr("bias = %d\n", bias);
    kernelDestroy(&kel2);
    pixDestroy(&pixd);

        /* Test bias convolution separable with kel3x and kel3y */
    kel3x = kernelCreateFromString(1, 5, 0, 2, kel3xstr);
    kel3y = kernelCreateFromString(7, 1, 3, 0, kel3ystr);
    pixd = pixConvolveWithBias(pixg, kel3x, kel3y, TRUE, &bias);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 11 */
    pixDisplayWithTitle(pixd, 500, 500, NULL, rp->display);
    lept_stderr("bias = %d\n", bias);
    kernelDestroy(&kel3x);
    kernelDestroy(&kel3y);
    pixDestroy(&pixd);
    pixDestroy(&pixs);
    pixDestroy(&pixg);

        /* Test pixWindowedMean() and pixWindowedMeanSquare() on 8 bpp */
    pixs = pixRead("feyn-fract2.tif");
    pixg = pixConvertTo8(pixs, 0);
    sizex = 5;
    sizey = 20;
    pixb = pixAddBorderGeneral(pixg, sizex + 1, sizex + 1,
                               sizey + 1, sizey + 1, 0);
    pixm = pixWindowedMean(pixb, sizex, sizey, 1, 1);
    pixms = pixWindowedMeanSquare(pixb, sizex, sizey, 1);
    regTestWritePixAndCheck(rp, pixm, IFF_JFIF_JPEG);  /* 12 */
    pixDisplayWithTitle(pixm, 100, 0, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixb);

        /* Test pixWindowedVariance() on 8 bpp */
    pixWindowedVariance(pixm, pixms, &fpixv, &fpixrv);
    pixrv = fpixConvertToPix(fpixrv, 8, L_CLIP_TO_ZERO, 1);
    regTestWritePixAndCheck(rp, pixrv, IFF_JFIF_JPEG);  /* 13 */
    pixDisplayWithTitle(pixrv, 100, 250, NULL, rp->display);
    pix1 = fpixDisplayMaxDynamicRange(fpixv);
    pix2 = fpixDisplayMaxDynamicRange(fpixrv);
    pixDisplayWithTitle(pix1, 100, 500, "Variance", rp->display);
    pixDisplayWithTitle(pix2, 100, 750, "RMS deviation", rp->display);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 14 */
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 15 */
    fpixDestroy(&fpixv);
    fpixDestroy(&fpixrv);
    pixDestroy(&pixm);
    pixDestroy(&pixms);
    pixDestroy(&pixrv);

        /* Test again all windowed functions with simpler interface */
    pixWindowedStats(pixg, sizex, sizey, 0, NULL, NULL, &fpixv, &fpixrv);
    pix3 = fpixDisplayMaxDynamicRange(fpixv);
    pix4 = fpixDisplayMaxDynamicRange(fpixrv);
    regTestComparePix(rp, pix1, pix3);  /* 16 */
    regTestComparePix(rp, pix2, pix4);  /* 17 */
    pixDestroy(&pixg);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    fpixDestroy(&fpixv);
    fpixDestroy(&fpixrv);

    return regTestCleanup(rp);
}
