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
 * grayquant_reg.c
 *
 *     Tests gray thresholding to 1, 2 and 4 bpp, with and without colormaps
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  THRESHOLD = 130;
    /* nlevels for 4 bpp output; anything between 2 and 16 is allowed */
static const l_int32  NLEVELS = 4;


int main(int    argc,
         char **argv)
{
const char   *str;
l_int32       index, w, h;
BOX          *box;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIXA         *pixa;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ------------------------------------------------------------- */

    pixs = pixRead("test8.jpg");
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixs, L_INSERT);

        /* threshold to 1 bpp */
    pix1 = pixThresholdToBinary(pixs, THRESHOLD);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */

        /* dither to 2 bpp, with and without colormap */
    pix1 = pixDitherTo2bpp(pixs, 1);
    pix2 = pixDitherTo2bpp(pixs, 0);
    pix3 = pixConvertGrayToColormap(pix2);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */
    if (rp->display) pixcmapWriteStream(stderr, pixGetColormap(pix3));
    regTestComparePix(rp, pix1, pix3);      /* 4 */

        /* threshold to 2 bpp, with and without colormap */
    pix1 = pixThresholdTo2bpp(pixs, 4, 1);
    pix2 = pixThresholdTo2bpp(pixs, 4, 0);
    pix3 = pixConvertGrayToColormap(pix2);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 6 */
    pixaAddPix(pixa, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 7 */
    if (rp->display) pixcmapWriteStream(stderr, pixGetColormap(pix3));
    regTestComparePix(rp, pix1, pix3);      /* 8 */

    pix1 = pixThresholdTo2bpp(pixs, 3, 1);
    pix2 = pixThresholdTo2bpp(pixs, 3, 0);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 9 */
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 10 */

        /* threshold to 4 bpp, with and without colormap */
    pix1 = pixThresholdTo4bpp(pixs, 9, 1);
    pix2 = pixThresholdTo4bpp(pixs, 9, 0);
    pix3 = pixConvertGrayToColormap(pix2);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 11 */
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 12 */
    pixaAddPix(pixa, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 13 */
    if (rp->display) pixcmapWriteStream(stderr, pixGetColormap(pix3));

        /* threshold on 8 bpp, with and without colormap */
    pix1 = pixThresholdOn8bpp(pixs, 9, 1);
    pix2 = pixThresholdOn8bpp(pixs, 9, 0);
    pix3 = pixConvertGrayToColormap(pix2);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 14 */
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 15 */
    pixaAddPix(pixa, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 16 */
    if (rp->display) pixcmapWriteStream(stderr, pixGetColormap(pix3));
    regTestComparePix(rp, pix1, pix3);      /* 17 */

        /* Optional display */
    if (rp->display) {
        lept_mkdir("lept/gquant");
        pix1 = pixaDisplayTiled(pixa, 2000, 0, 20);
        pixDisplay(pix1, 100, 100);
        pixWrite("/tmp/lept/gquant/mosaic1.png", pix1, IFF_PNG);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa);

    /* ------------------------------------------------------------- */

    pixs = pixRead("test8.jpg");
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixs, L_INSERT);

        /* Highlight 2 bpp with colormap */
    pix1 = pixThresholdTo2bpp(pixs, 3, 1);
    pixaAddPix(pixa, pix1, L_COPY);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 18 */
    cmap = pixGetColormap(pix1);
    if (rp->display) pixcmapWriteStream(stderr, cmap);
    box = boxCreate(278, 35, 122, 50);
    pixSetSelectCmap(pix1, box, 2, 255, 255, 100);
    boxDestroy(&box);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 19 */
    if (rp->display) pixcmapWriteStream(stderr, cmap);

        /* Test pixThreshold8() */
    pix1 = pixThreshold8(pixs, 1, 2, 1);  /* cmap */
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 20 */

    pix1 = pixThreshold8(pixs, 1, 2, 0);  /* no cmap */
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 21 */

    pix1 = pixThreshold8(pixs, 2, 3, 1);  /* highlight one box */
    box = boxCreate(278, 35, 122, 50);
    pixSetSelectCmap(pix1, box, 2, 255, 255, 100);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 22 */
    cmap = pixGetColormap(pix1);
    if (rp->display) pixcmapWriteStream(stderr, cmap);
    boxDestroy(&box);


    pix1 = pixThreshold8(pixs, 2, 4, 0);  /* no cmap */
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 23 */

    pix1 = pixThreshold8(pixs, 4, 6, 1);  /* highlight one box */
    box = boxCreate(278, 35, 122, 50);
    pixSetSelectCmap(pix1, box, 5, 255, 255, 100);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 24 */
    cmap = pixGetColormap(pix1);
    if (rp->display) pixcmapWriteStream(stderr, cmap);
    boxDestroy(&box);

    pix1 = pixThreshold8(pixs, 4, 6, 0);  /* no cmap */
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 25 */

        /* Highlight 4 bpp with 2 colormap entries */
        /* Note: We use 5 levels (0-4) for gray.           */
        /*       5 & 6 are used for highlight color.       */
    pix1 = pixThresholdTo4bpp(pixs, 5, 1);
    cmap = pixGetColormap(pix1);
    pixcmapGetIndex(cmap, 255, 255, 255, &index);
    box = boxCreate(278, 35, 122, 50);
    pixSetSelectCmap(pix1, box, index, 255, 255, 100);  /* use index 5 */
    pixaAddPix(pixa, pix1, L_COPY);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 26 */
    boxDestroy(&box);
    box = boxCreate(4, 6, 157, 33);
    pixSetSelectCmap(pix1, box, index, 100, 255, 255);  /* use index 6 */
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 27 */
    boxDestroy(&box);
    if (rp->display) pixcmapWriteStream(stderr, cmap);

        /* Optional display */
    if (rp->display) {
        pix1 = pixaDisplayTiled(pixa, 2000, 0, 20);
        pixDisplay(pix1, 200, 100);
        pixWrite("/tmp/lept/gquant/mosaic2.png", pix1, IFF_PNG);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa);

    /* ------------------------------------------------------------- */

    pixs = pixRead("feyn.tif");
    pixa = pixaCreate(0);

        /* Comparison 8 bpp jpeg with 2 bpp (highlight) */
    pix1 = pixScaleToGray4(pixs);
    pix2 = pixReduceRankBinaryCascade(pixs, 2, 2, 0, 0);
    pix3 = pixThresholdTo2bpp(pix1, 3, 1);
    box = boxCreate(175, 208, 228, 88);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 28 */
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 29 */
    pixSetSelectCmap(pix3, box, 2, 255, 255, 100);
    pixaAddPix(pixa, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 30 */
    cmap = pixGetColormap(pix3);
    if (rp->display) pixcmapWriteStream(stderr, cmap);
    boxDestroy(&box);

        /* Thresholding to 4 bpp (highlight); use pix1 from above */
    pix2 = pixThresholdTo4bpp(pix1, NLEVELS, 1);
    box = boxCreate(175, 208, 228, 83);
    pixSetSelectCmap(pix2, box, NLEVELS - 1, 255, 255, 100);
    boxDestroy(&box);
    box = boxCreate(232, 298, 110, 25);
    pixSetSelectCmap(pix2, box, NLEVELS - 1, 100, 255, 255);
    boxDestroy(&box);
    box = boxCreate(21, 698, 246, 82);
    pixSetSelectCmap(pix2, box, NLEVELS - 1, 225, 100, 255);
    boxDestroy(&box);
    pixaAddPix(pixa, pix2, L_INSERT);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 31 */
    cmap = pixGetColormap(pix2);
    if (rp->display) pixcmapWriteStream(stderr, cmap);
    pix3 = pixReduceRankBinaryCascade(pixs, 2, 2, 0, 0);
    pixaAddPix(pixa, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 32 */

       /* Thresholding to 4 bpp at 2, 3, 4, 5 and 6 levels */
    box = boxCreate(25, 202, 136, 37);
    pix2 = pixClipRectangle(pix1, box, NULL);
    pix3 = pixScale(pix2, 6., 6.);
    pixaAddPix(pixa, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 33 */
    pixGetDimensions(pix3, &w, &h, NULL);
    pix4 = pixCreate(w, 6 * h, 8);
    pixRasterop(pix4, 0, 0, w, h, PIX_SRC, pix3, 0, 0);
    pixDestroy(&pix2);
    boxDestroy(&box);

    pix5 = pixThresholdTo4bpp(pix3, 6, 1);
    pix6 = pixRemoveColormap(pix5, REMOVE_CMAP_TO_GRAYSCALE);
    pixRasterop(pix4, 0, h, w, h, PIX_SRC, pix6, 0, 0);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 34 */
    pixDestroy(&pix6);

    pix5 = pixThresholdTo4bpp(pix3, 5, 1);
    pix6 = pixRemoveColormap(pix5, REMOVE_CMAP_TO_GRAYSCALE);
    pixRasterop(pix4, 0, 2 * h, w, h, PIX_SRC, pix6, 0, 0);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 35 */
    pixDestroy(&pix6);

    pix5 = pixThresholdTo4bpp(pix3, 4, 1);
    pix6 = pixRemoveColormap(pix5, REMOVE_CMAP_TO_GRAYSCALE);
    pixRasterop(pix4, 0, 3 * h, w, h, PIX_SRC, pix6, 0, 0);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 36 */
    pixDestroy(&pix6);

    pix5 = pixThresholdTo4bpp(pix3, 3, 1);
    pix6 = pixRemoveColormap(pix5, REMOVE_CMAP_TO_GRAYSCALE);
    pixRasterop(pix4, 0, 4 * h, w, h, PIX_SRC, pix6, 0, 0);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 37 */
    pixDestroy(&pix6);

    pix5 = pixThresholdTo4bpp(pix3, 2, 1);
    pix6 = pixRemoveColormap(pix5, REMOVE_CMAP_TO_GRAYSCALE);
    pixRasterop(pix4, 0, 5 * h, w, h, PIX_SRC, pix6, 0, 0);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 38 */
    pixDestroy(&pix6);
    pixaAddPix(pixa, pix4, L_INSERT);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 39 */

        /* Optional display */
    if (rp->display) {
        pix1 = pixaDisplayTiled(pixa, 2000, 0, 20);
        pixDisplay(pix1, 300, 100);
        pixWrite("/tmp/lept/gquant/mosaic3.png", pix1, IFF_PNG);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa);
    pixDestroy(&pixs);

    /* ------------------------------------------------------------- */

       /* Thresholding with fixed and arbitrary bin boundaries */
    pixs = pixRead("stampede2.jpg");
    pixa = pixaCreate(0);

    pixaAddPix(pixa, pixs, L_INSERT);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 40 */
    pix1 = pixThresholdTo4bpp(pixs, 5, 1);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 41 */
    pix1 = pixThresholdTo4bpp(pixs, 7, 1);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 42 */
    cmap = pixGetColormap(pix1);
    if (rp->display) pixcmapWriteStream(stderr, cmap);
    pix1 = pixThresholdTo4bpp(pixs, 11, 1);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 43 */

    str = "45 75 115 185";
    pix1 = pixThresholdGrayArb(pixs, str, 8, 0, 0, 0);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 44 */
    str = "38 65 85 115 160 210";
    pix1 = pixThresholdGrayArb(pixs, str, 8, 0, 1, 1);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 45 */
    cmap = pixGetColormap(pix1);
    if (rp->display) pixcmapWriteStream(stderr, cmap);
    str = "38 60 75 90 110 130 155 185 208 239";
    pix1 = pixThresholdGrayArb(pixs, str, 8, 0, 0, 0);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 46 */
    str = "45 75 115 185";
    pix1 = pixThresholdGrayArb(pixs, str, 0, 1, 0, 1);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 47 */
    str = "38 65 85 115 160 210";
    pix1 = pixThresholdGrayArb(pixs, str, 0, 1, 0, 1);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 48 */
    cmap = pixGetColormap(pix1);
    if (rp->display) pixcmapWriteStream(stderr, cmap);
    str = "38 60 75 90 110 130 155 185 208 239";
    pix1 = pixThresholdGrayArb(pixs, str, 4, 1, 0, 1);
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 49 */

        /* Optional display */
    if (rp->display) {
        pix1 = pixaDisplayTiled(pixa, 2000, 0, 20);
        pixDisplay(pix1, 400, 100);
        pixWrite("/tmp/lept/gquant/mosaic4.png", pix1, IFF_PNG);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa);

    if (rp->display) {
            /* Upscale 2x and threshold to 1 bpp */
        pixs = pixRead("test8.jpg");
        startTimer();
        pix1 = pixScaleGray2xLIThresh(pixs, THRESHOLD);
        lept_stderr(" time for scale/dither = %7.3f sec\n", stopTimer());
        pixWrite("/tmp/lept/gquant/upscale1.png", pix1, IFF_PNG);
        pixDisplay(pix1, 0, 500);
        pixDestroy(&pix1);

            /* Upscale 4x and threshold to 1 bpp */
        startTimer();
        pix1 = pixScaleGray4xLIThresh(pixs, THRESHOLD);
        lept_stderr(" time for scale/dither = %7.3f sec\n", stopTimer());
        pixWrite("/tmp/lept/gquant/upscale2.png", pix1, IFF_PNG);
        pixDisplay(pix1, 700, 500);
        pixDestroy(&pix1);
        pixDestroy(&pixs);
    }

    return regTestCleanup(rp);
}

