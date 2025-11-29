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
 *   colormask_reg.c
 *
 *   This tests the ability to identify regions in HSV color space
 *   by analyzing the HS histogram and building masks that cover
 *   peaks in HS.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, j, x, y, rval, gval, bval;
l_uint32      pixel1, pixel2;
l_float32     frval, fgval, fbval;
NUMA         *nahue, *nasat, *napk;
PIX          *pixs, *pixhsv, *pixh, *pixg, *pixf, *pixd, *pixr;
PIX          *pix1, *pix2, *pix3;
PIXA         *pixa, *pixapk;
PTA          *ptapk;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "colormask_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Make a graded frame color */
    pixs = pixCreate(650, 900, 32);
    for (i = 0; i < 900; i++) {
        rval = 40 + i / 30;
        for (j = 0; j < 650; j++) {
            gval = 255 - j / 30;
            bval = 70 + j / 30;
            composeRGBPixel(rval, gval, bval, &pixel1);
            pixSetPixel(pixs, j, i, pixel1);
        }
    }

        /* Place an image inside the frame and convert to HSV */
    pix1 = pixRead("1555.003.jpg");
    pix2 = pixScale(pix1, 0.5, 0.5);
    pixRasterop(pixs, 100, 100, 2000, 2000, PIX_SRC, pix2, 0, 0);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDisplayWithTitle(pixs, 400, 0, "Input image", rp->display);
    pixa = pixaCreate(0);
    pixhsv = pixConvertRGBToHSV(NULL, pixs);

        /* Work in the HS projection of HSV */
    pixh = pixMakeHistoHS(pixhsv, 5, &nahue, &nasat);
    pixg = pixMaxDynamicRange(pixh, L_LOG_SCALE);
    pixf = pixConvertGrayToFalseColor(pixg, 1.0);
    regTestWritePixAndCheck(rp, pixf, IFF_PNG);   /* 0 */
    pixDisplayWithTitle(pixf, 100, 0, "False color HS histo", rp->display);
    pixaAddPix(pixa, pixs, L_COPY);
    pixaAddPix(pixa, pixhsv, L_INSERT);
    pixaAddPix(pixa, pixg, L_INSERT);
    pixaAddPix(pixa, pixf, L_INSERT);
    gplotSimple1(nahue, GPLOT_PNG, "/tmp/lept/regout/junkhue",
                 "Histogram of hue values");
    pix3 = pixRead("/tmp/lept/regout/junkhue.png");
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix3, 100, 300, "Histo of hue", rp->display);
    pixaAddPix(pixa, pix3, L_INSERT);
    gplotSimple1(nasat, GPLOT_PNG, "/tmp/lept/regout/junksat",
                 "Histogram of saturation values");
    pix3 = pixRead("/tmp/lept/regout/junksat.png");
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix3, 100, 800, "Histo of saturation", rp->display);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixd = pixaDisplayTiledAndScaled(pixa, 32, 270, 7, 0, 30, 3);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pixd, 0, 400, "Hue and Saturation Mosaic", rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    numaDestroy(&nahue);
    numaDestroy(&nasat);

        /* Find all the peaks */
    pixFindHistoPeaksHSV(pixh, L_HS_HISTO, 20, 20, 6, 2.0,
                         &ptapk, &napk, &pixapk);
    numaWriteStderr(napk);
    ptaWriteStream(stderr, ptapk, 1);
    pixd = pixaDisplayTiledInRows(pixapk, 32, 1400, 1.0, 0, 30, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pixd, 0, 550, "Peaks in HS", rp->display);
    pixDestroy(&pixh);
    pixDestroy(&pixd);
    pixaDestroy(&pixapk);

        /* Make masks for each of the peaks */
    pixa = pixaCreate(0);
    pixr = pixScaleBySampling(pixs, 0.4, 0.4);
    for (i = 0; i < 6; i++) {
        ptaGetIPt(ptapk, i, &x, &y);
        pix1 = pixMakeRangeMaskHS(pixr, y, 20, x, 20, L_INCLUDE_REGION);
        pixaAddPix(pixa, pix1, L_INSERT);
        pixGetAverageMaskedRGB(pixr, pix1, 0, 0, 1, L_MEAN_ABSVAL,
                               &frval, &fgval, &fbval);
        composeRGBPixel((l_int32)frval, (l_int32)fgval, (l_int32)fbval,
                        &pixel1);
        pixGetPixelAverage(pixr, pix1, 0, 0, 1, &pixel2);
        regTestCompareValues(rp, pixel1 >> 8, pixel2 >> 8, 0.0);  /* 5 - 10 */
        pix2 = pixCreateTemplate(pixr);
        pixSetAll(pix2);
        pixPaintThroughMask(pix2, pix1, 0, 0, pixel1);
        pixaAddPix(pixa, pix2, L_INSERT);
        pix3 = pixCreateTemplate(pixr);
        pixSetAllArbitrary(pix3, pixel1);
        pixaAddPix(pixa, pix3, L_INSERT);
    }
    pixd = pixaDisplayTiledAndScaled(pixa, 32, 225, 3, 0, 30, 3);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 8 */
    pixDisplayWithTitle(pixd, 600, 0, "Masks over peaks", rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixr);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    ptaDestroy(&ptapk);
    numaDestroy(&napk);

    return regTestCleanup(rp);
}
