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
 * enhance_reg.c
 *
 *   This tests the following global "enhancement" functions:
 *     * TRC transforms with variation of gamma and black point
 *     * HSV transforms with variation of hue, saturation and intensity
 *     * Contrast variation
 *     * Sharpening
 *     * Color mapping to lighten background with constant hue
 *     * Linear color transform without mixing (diagonal)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"


int main(int    argc,
         char **argv)
{
char          textstr[256];
l_int32       i, k, w, h;
l_uint32      srcval;
l_float32     scalefact, sat, fract;
L_BMF        *bmf10;
L_KERNEL     *kel;
NUMA         *na1, *na2, *na3;
PIX          *pix, *pixs, *pixs1, *pixs2, *pixd;
PIX          *pix0, *pix1, *pix2, *pix3, *pix4;
PIXA         *pixa1, *pixa2, *pixaf;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "enhance_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/enhance");
    pix = pixRead("test24.jpg");  /* rgb */
    w = pixGetWidth(pix);
    scalefact = 150.0 / (l_float32)w;  /* scale to w = 150 */
    pixs = pixScale(pix, scalefact, scalefact);
    w = pixGetWidth(pixs);
    pixaf = pixaCreate(5);
    pixDestroy(&pix);

        /* TRC: vary gamma */
    pixa1 = pixaCreate(20);
    for (i = 0; i < 20; i++) {
        pix0 = pixGammaTRC(NULL, pixs, 0.3 + 0.15 * i, 0, 255);
        pixaAddPix(pixa1, pix0, L_INSERT);
    }
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, w, 5, 0, 10, 2);
    pixaAddPix(pixaf, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix1, 0, 100, "TRC Gamma", rp->display);
    pixaDestroy(&pixa1);

        /* TRC: vary black point */
    pixa1 = pixaCreate(20);
    for (i = 0; i < 20; i++) {
        pix0 = pixGammaTRC(NULL, pixs, 1.0, 5 * i, 255);
        pixaAddPix(pixa1, pix0, L_INSERT);
    }
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, w, 5, 0, 10, 2);
    pixaAddPix(pixaf, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix1, 300, 100, "TRC", rp->display);
    pixaDestroy(&pixa1);

        /* Vary hue */
    pixa1 = pixaCreate(20);
    for (i = 0; i < 20; i++) {
        pix0 = pixModifyHue(NULL, pixs, 0.01 + 0.05 * i);
        pixaAddPix(pixa1, pix0, L_INSERT);
    }
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, w, 5, 0, 10, 2);
    pixaAddPix(pixaf, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 600, 100, "Hue", rp->display);
    pixaDestroy(&pixa1);

        /* Vary saturation */
    pixa1 = pixaCreate(20);
    na1 = numaCreate(20);
    for (i = 0; i < 20; i++) {
        pix0 = pixModifySaturation(NULL, pixs, -0.9 + 0.1 * i);
        pixMeasureSaturation(pix0, 1, &sat);
        pixaAddPix(pixa1, pix0, L_INSERT);
        numaAddNumber(na1, sat);
    }
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, w, 5, 0, 10, 2);
    pixaAddPix(pixaf, pix1, L_INSERT);
    gplotSimple1(na1, GPLOT_PNG, "/tmp/lept/regout/enhance.7",
                 "Average Saturation");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix1, 900, 100, "Saturation", rp->display);
    numaDestroy(&na1);
    pixaDestroy(&pixa1);

        /* Vary contrast */
    pixa1 = pixaCreate(20);
    for (i = 0; i < 20; i++) {
        pix0 = pixContrastTRC(NULL, pixs, 0.1 * i);
        pixaAddPix(pixa1, pix0, L_INSERT);
    }
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, w, 5, 0, 10, 2);
    pixaAddPix(pixaf, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix1, 0, 400, "Contrast", rp->display);
    pixaDestroy(&pixa1);

        /* Vary sharpening */
    pixa1 = pixaCreate(20);
    for (i = 0; i < 20; i++) {
        pix0 = pixUnsharpMasking(pixs, 3, 0.01 + 0.15 * i);
        pixaAddPix(pixa1, pix0, L_INSERT);
    }
    pix1 = pixaDisplayTiledAndScaled(pixa1, 32, w, 5, 0, 10, 2);
    pixaAddPix(pixaf, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix1, 300, 400, "Sharp", rp->display);
    pixaDestroy(&pixa1);

        /* Hue constant mapping to lighter background */
    pixa2 = pixaCreate(2);
    bmf10 = bmfCreate("fonts", 10);
    pix0 = pixRead("candelabrum.011.jpg");
    composeRGBPixel(230, 185, 144, &srcval);  /* select typical bg pixel */
    for (k = 1; k > -2; k -= 2) {
        pixa1 = pixaCreate(11);
        for (i = 0; i <= 10; i++) {
            fract = k * 0.10 * i;
            pix1 = pixMapWithInvariantHue(NULL, pix0, srcval, fract);
            snprintf(textstr, 50, "Fract = %5.1f", fract);
            pix2 = pixAddSingleTextblock(pix1, bmf10, textstr, 0xff000000,
                                          L_ADD_BELOW, NULL);
            pixaAddPix(pixa1, pix2, L_INSERT);
            pixDestroy(&pix1);
        }
        pix3 = pixaDisplayTiledInColumns(pixa1, 4, 1.0, 30, 2);
        pixaAddPix(pixa2, pix3, L_INSERT);
        pixaDestroy(&pixa1);
    }
    pixd = pixaDisplayTiledInColumns(pixa2, 2, 0.5, 30, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 6 */
    pixDisplayWithTitle(pixd, 600, 400, "Constant hue", rp->display);
    pixaDestroy(&pixa2);
    bmfDestroy(&bmf10);
    pixDestroy(&pix0);
    pixDestroy(&pixd);

        /* Delayed testing of saturation plot */
    regTestCheckFile(rp, "/tmp/lept/regout/enhance.7.png");  /* 7 */

        /* Display results */
    pixd = pixaDisplayTiledInColumns(pixaf, 1, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 8 */
    pixDisplayWithTitle(pixd, 100, 100, "All", rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixaf);

        /* Test color shifts */
    pixd = pixMosaicColorShiftRGB(pixs, -0.1, 0.0, 0.0, 0.0999, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 9 */
    pixDisplayWithTitle(pixd, 1000, 100, "Color shift", rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixs);

        /* More trc testing */
    pix = pixRead("test24.jpg");  /* rgb */
    pixs = pixScale(pix, 0.3, 0.3);
    pixDestroy(&pix);
    pixa1 = pixaCreate(5);
    pixaAddPix(pixa1, pixs, L_COPY);

    na1 = numaGammaTRC(0.6, 40, 200);
    na2 = numaGammaTRC(1.2, 40, 225);
    na3 = numaGammaTRC(0.6, 40, 255);
    pixGetDimensions(pixs, &w, &h, NULL);
    pix1 = pixCopy(NULL, pixs);
    pix2 = pixMakeSymmetricMask(w, h, 0.5, 0.5, L_USE_INNER);
    pixaAddPix(pixa1, pix2, L_COPY);
    pixTRCMapGeneral(pix1, pix2, na1, na2, na3);
    pixaAddPix(pixa1, pix1, L_COPY);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10 */
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

    na1 = numaGammaTRC(1.0, 0, 255);
    na2 = numaGammaTRC(1.0, 0, 255);
    na3 = numaGammaTRC(1.0, 0, 255);
    pix1 = pixCopy(NULL, pixs);
    pixTRCMapGeneral(pix1, NULL, na1, na2, na3);
    regTestComparePix(rp, pixs, pix1);  /* 11 */
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    pixDestroy(&pix1);

    na1 = numaGammaTRC(1.7, 150, 255);
    na2 = numaGammaTRC(0.7, 0, 150);
    na3 = numaGammaTRC(1.2, 80, 200);
    pixTRCMapGeneral(pixs, NULL, na1, na2, na3);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 12 */
    pixaAddPix(pixa1, pixs, L_COPY);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    pixDestroy(&pixs);

    na1 = numaGammaTRC(0.8, 0, 220);
    na2 = numaGammaTRC(1.0, 40, 220);
    gplotSimple2(na1, na2, GPLOT_PNG, "/tmp/lept/enhance/junkp", NULL);
    pix1 = pixRead("/tmp/lept/enhance/junkp.png");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 13 */
    pixaAddPix(pixa1, pix1, L_COPY);
    numaDestroy(&na1);
    numaDestroy(&na2);
    pixDestroy(&pix1);

    pixd = pixaDisplayTiledInColumns(pixa1, 4, 1.0, 30, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 14 */
    pixDisplayWithTitle(pixd, 100, 800, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa1);

    /* -----------------------------------------------*
     *           Test global color transforms         *
     * -----------------------------------------------*/
        /* Make identical cmap and rgb images */
    pix = pixRead("wet-day.jpg");
    pixs1 = pixOctreeColorQuant(pix, 200, 0);
    pixs2 = pixRemoveColormap(pixs1, REMOVE_CMAP_TO_FULL_COLOR);
    regTestComparePix(rp, pixs1, pixs2);  /* 15 */

        /* Make a diagonal color transform matrix */
    kel = kernelCreate(3, 3);
    kernelSetElement(kel, 0, 0, 0.7);
    kernelSetElement(kel, 1, 1, 0.4);
    kernelSetElement(kel, 2, 2, 1.3);

        /* Apply to both cmap and rgb images. */
    pix1 = pixMultMatrixColor(pixs1, kel);
    pix2 = pixMultMatrixColor(pixs2, kel);
    regTestComparePix(rp, pix1, pix2);  /* 16 */
    kernelDestroy(&kel);

        /* Apply the same transform in the simpler interface */
    pix3 = pixMultConstantColor(pixs1, 0.7, 0.4, 1.3);
    pix4 = pixMultConstantColor(pixs2, 0.7, 0.4, 1.3);
    regTestComparePix(rp, pix3, pix4);  /* 17 */
    regTestComparePix(rp, pix1, pix3);  /* 18 */
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 19 */

    pixDestroy(&pix);
    pixDestroy(&pixs1);
    pixDestroy(&pixs2);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    return regTestCleanup(rp);
}
