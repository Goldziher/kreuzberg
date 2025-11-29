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
 *   misctest1.c
 *        * Combine two grayscale images using a mask
 *        * Combine two binary images using a mask
 *        * Do a restricted seedfill
 *        * Colorize a grayscale image
 *        * Convert color to gray
 *        * Extract text lines
 *        * Plot box side locations and dimension of a boxa
 *        * Extract and display rank sized components
 *        * Extract parts of an image using a boxa
 *        * Display pixaa in row major order by component pixa.
 *        * Test zlib compression in png
 *        * Show sampled scaling with and without source indexing shift
 *        * Display differences in images with pixDisplayDiff()
 *        * Demonstrate read of cmap+alpha png, and I/O of rgba pnm, bmp, webp
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   SHOW    0

static const size_t  zlibsize[5] = {1047868, 215039, 195778, 189709, 180987};

int main(int    argc,
         char **argv)
{
l_int32   w, h, bx, by, bw, bh, i, j, same;
size_t    size;
BOX      *box1, *box2;
BOXA     *boxa1, *boxa2, *boxae, *boxao;
PIX      *pixs, *pix1, *pix2, *pix3, *pix4, *pixg, *pixb, *pixd, *pixc;
PIX      *pixm, *pixm2, *pixd2, *pixs2;
PIXA     *pixa1, *pixa2;
PIXAA    *paa;
PIXCMAP  *cmap, *cmapg;

    setLeptDebugOK(1);
    lept_mkdir("lept/misc");
    paa = pixaaCreate(0);

        /* Combine two grayscale images using a mask */
    lept_stderr("Combine two grayscale images using a mask\n");
    pixa1 = pixaCreate(0);
    pixd = pixRead("feyn.tif");
    pixs = pixRead("rabi.png");
    pixm = pixRead("pageseg2-seed.png");
    pixd2 = pixScaleToGray2(pixd);
    pixs2 = pixScaleToGray2(pixs);
    pixaAddPix(pixa1, pixd2, L_COPY);
    pixaAddPix(pixa1, pixs2, L_INSERT);
    pixaAddPix(pixa1, pixm, L_COPY);
    pixCombineMaskedGeneral(pixd2, pixs2, pixm, 100, 100);
    pixaAddPix(pixa1, pixd2, L_INSERT);
    pixDisplayWithTitle(pixd2, 100, 100, NULL, SHOW);
    pixaaAddPixa(paa, pixa1, L_INSERT);

        /* Combine two binary images using a mask */
    lept_stderr("Combine two binary images using a mask\n");
    pixa1 = pixaCreate(0);
    pixm2 = pixExpandBinaryReplicate(pixm, 2, 2);
    pix1 = pixCopy(NULL, pixd);
    pixCombineMaskedGeneral(pixd, pixs, pixm2, 200, 200);
    pixaAddPix(pixa1, pixd, L_COPY);
    pixDisplayWithTitle(pixd, 700, 100, NULL, SHOW);
    pixCombineMasked(pix1, pixs, pixm2);
    pixaAddPix(pixa1, pix1, L_INSERT);
    pixaaAddPixa(paa, pixa1, L_INSERT);
    pixDestroy(&pixd);
    pixDestroy(&pixs);
    pixDestroy(&pixm);
    pixDestroy(&pixm2);

        /* Do a restricted seedfill */
    lept_stderr("Do a restricted seedfill\n");
    pixa1 = pixaCreate(0);
    pixs = pixRead("pageseg2-seed.png");
    pixm = pixRead("pageseg2-mask.png");
    pixd = pixSeedfillBinaryRestricted(NULL, pixs, pixm, 8, 50, 175);
    pixaAddPix(pixa1, pixs, L_INSERT);
    pixaAddPix(pixa1, pixm, L_INSERT);
    pixaAddPix(pixa1, pixd, L_INSERT);
    pixaaAddPixa(paa, pixa1, L_INSERT);
    pix1 = pixaaDisplayByPixa(paa, 10, 0.5, 40, 40, 2);
    pixWrite("/tmp/lept/misc/mos1.png", pix1, IFF_PNG);
    pixDisplay(pix1, 100, 100);
    pixaaDestroy(&paa);
    pixDestroy(&pix1);

        /* Colorize a grayscale image */
    lept_stderr("Colorize a grayscale image\n");
    paa = pixaaCreate(0);
    pixa1 = pixaCreate(0);
    pixs = pixRead("lucasta.150.jpg");
    pixGetDimensions(pixs, &w, &h, NULL);
    pixb = pixThresholdToBinary(pixs, 128);
    boxa1 = pixConnComp(pixb, &pixa2, 8);
    pixaAddPix(pixa1, pixs, L_COPY);
    cmap = pixcmapGrayToColor(0x6f90c0);
    pixSetColormap(pixs, cmap);
    pixaAddPix(pixa1, pixs, L_COPY);
    pixc = pixaDisplayRandomCmap(pixa2, w, h);
    pixcmapResetColor(pixGetColormap(pixc), 0, 255, 255, 255);
    pixaAddPix(pixa1, pixc, L_INSERT);
    pixaaAddPixa(paa, pixa1, L_INSERT);
    pixDestroy(&pixs);
    pixDestroy(&pixb);
    boxaDestroy(&boxa1);
    pixaDestroy(&pixa2);

        /* Convert color to gray */
    lept_stderr("Convert color to gray\n");
    pixa1 = pixaCreate(0);
    pixs = pixRead("weasel4.16c.png");
    pixaAddPix(pixa1, pixs, L_INSERT);
    pixc = pixConvertTo32(pixs);
    pix1 = pixConvertRGBToGray(pixc, 3., 7., 5.);  /* bad weights */
    pixaAddPix(pixa1, pix1, L_INSERT);
    pix2 = pixConvertRGBToGrayFast(pixc);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixg = pixCopy(NULL, pixs);
    cmap = pixGetColormap(pixs);
    cmapg = pixcmapColorToGray(cmap, 4., 6., 3.);
    pixSetColormap(pixg, cmapg);
    pixaAddPix(pixa1, pixg, L_INSERT);
    pixaaAddPixa(paa, pixa1, L_INSERT);
    pixDestroy(&pixc);

    pix1 = pixaaDisplayByPixa(paa, 10, 1.0, 20, 20, 0);
    pixWrite("/tmp/lept/misc/mos2.png", pix1, IFF_PNG);
    pixDisplay(pix1, 400, 100);
    pixaaDestroy(&paa);
    pixDestroy(&pix1);

        /* Extract text lines */
    lept_stderr("Extract text lines\n");
    pix1 = pixRead("feyn.tif");
    pixa1 = pixExtractTextlines(pix1, 150, 150, 0, 0, 5, 5, NULL);
    boxa1 = pixaGetBoxa(pixa1, L_CLONE);
    boxaWrite("/tmp/lept/misc/lines1.ba", boxa1);
    pix2 = pixaDisplayRandomCmap(pixa1, 0, 0);
    pixcmapResetColor(pixGetColormap(pix2), 0, 255, 255, 255);
    pixDisplay(pix2, 400, 0);
    pixWrite("/tmp/lept/misc/lines1.png", pix2, IFF_PNG);
    boxaDestroy(&boxa1);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixaDestroy(&pixa1);

    pix1 = pixRead("arabic.png");
    pixa1 = pixExtractTextlines(pix1, 150, 150, 0, 0, 5, 5, NULL);
    pix2 = pixaDisplayRandomCmap(pixa1, 0, 0);
    pixcmapResetColor(pixGetColormap(pix2), 0, 255, 255, 255);
    pixDisplay(pix2, 400, 400);
    pixWrite("/tmp/lept/misc/lines2.png", pix2, IFF_PNG);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixaDestroy(&pixa1);

    pix1 = pixRead("arabic2.png");
    pixa1 = pixExtractTextlines(pix1, 150, 150, 0, 0, 5, 5, NULL);
    pix2 = pixaDisplayRandomCmap(pixa1, 0, 0);
    pixcmapResetColor(pixGetColormap(pix2), 0, 255, 255, 255);
    pixDisplay(pix2, 400, 800);
    pixWrite("/tmp/lept/misc/lines3.png", pix2, IFF_PNG);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixaDestroy(&pixa1);

        /* Plot box side locations and dimensions of a boxa */
    lept_stderr("Plot box side locations and dimensions of a boxa\n");
    pixa1 = pixaCreate(0);
    boxa1 = boxaRead("boxa2.ba");
    boxaSplitEvenOdd(boxa1, 0, &boxae, &boxao);
    boxaPlotSides(boxae, "1-sides-even", NULL, NULL, NULL, NULL, &pix1);
    pixaAddPix(pixa1, pix1, L_INSERT);
    boxaPlotSides(boxao, "1-sides-odd", NULL, NULL, NULL, NULL, &pix1);
    pixaAddPix(pixa1, pix1, L_INSERT);
    boxaPlotSizes(boxae, "1-sizes-even", NULL, NULL, &pix1);
    pixaAddPix(pixa1, pix1, L_INSERT);
    boxaPlotSizes(boxao, "1-sizes-odd", NULL, NULL, &pix1);
    pixaAddPix(pixa1, pix1, L_INSERT);
    boxaDestroy(&boxae);
    boxaDestroy(&boxao);
    boxaDestroy(&boxa1);
    boxa1 = boxaRead("boxa3.ba");
    boxaSplitEvenOdd(boxa1, 0, &boxae, &boxao);
    boxaPlotSides(boxae, "2-sides-even", NULL, NULL, NULL, NULL, &pix1);
    pixaAddPix(pixa1, pix1, L_INSERT);
    boxaPlotSides(boxao, "2-sides-odd", NULL, NULL, NULL, NULL, &pix1);
    pixaAddPix(pixa1, pix1, L_INSERT);
    boxaPlotSizes(boxae, "2-sizes-even", NULL, NULL, &pix1);
    pixaAddPix(pixa1, pix1, L_INSERT);
    boxaPlotSizes(boxao, "2-sizes-odd", NULL, NULL, &pix1);
    pixaAddPix(pixa1, pix1, L_INSERT);
    boxaDestroy(&boxae);
    boxaDestroy(&boxao);
    boxaDestroy(&boxa1);
    pix1 = pixaDisplayTiledInRows(pixa1, 32, 1500, 1.0, 0, 30, 2);
    pixWrite("/tmp/lept/misc/boxaplots.png", pix1, IFF_PNG);
    pixDisplay(pix1, 800, 0);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);

        /* Extract and display rank sized components */
    lept_stderr("Extract and display rank sized components\n");
    pixs = pixRead("rabi-tiny.png");
    pixa1 = pixaCreate(0);
    for (i = 1; i <= 5; i++) {
        pixaAddPix(pixa1, pixs, L_COPY);
        pixGetDimensions(pixs, &w, &h, NULL);
        pixd = pixCreate(w, h, 32);
        pixSetAll(pixd);
        for (j = 0; j < 6; j++) {
            pix1 = pixSelectComponentBySize(pixs, j, i, 8, &box1);
            pix2 = pixConvertTo32(pix1);
            boxGetGeometry(box1, &bx, &by, &bw, &bh);
            pixRasterop(pixd, bx, by, bw, bh, PIX_SRC, pix2, 0, 0);
            box2 = boxAdjustSides(NULL, box1, -2, 2, -2, 2);
            pixRenderBoxArb(pixd, box2, 2, 255, 0, 0);
            pixaAddPix(pixa1, pixd, L_COPY);
            pixDestroy(&pix1);
            pixDestroy(&pix2);
            boxDestroy(&box1);
            boxDestroy(&box2);
        }
        pixDestroy(&pixd);
    }
    pix3 = pixaDisplayTiledAndScaled(pixa1, 32, 300, 7, 0, 30, 2);
    pixWrite("/tmp/lept/misc/comps.png", pix3, IFF_PNG);
    pixDisplay(pix3, 600, 300);
    pixaDestroy(&pixa1);
    pixDestroy(&pixs);
    pixDestroy(&pix3);

        /* Extract parts of an image using a boxa */
    lept_stderr("Extract parts of an image using a boxa\n");
    pix1 = pixRead("feyn-fract.tif");
    boxa1 = pixConnCompBB(pix1, 4);
    boxa2 = boxaSelectBySize(boxa1, 0, 28, L_SELECT_HEIGHT, L_SELECT_IF_GT,
                             NULL),
    pix2 = pixCopyWithBoxa(pix1, boxa2, L_SET_WHITE);
    pixWrite("/tmp/lept/misc/tallcomps.png", pix2, IFF_PNG);
    pixDisplay(pix2, 600, 600);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);

        /* Display pixaa in row major order by component pixa. */
    lept_stderr("Display pixaa in row major order by component pixa\n");
    pix1 = pixRead("char.tif");
    paa = pixaaCreate(100);
    for (i = 0; i < 50; i++) {
        pixa1 = pixaCreate(100);
        for (j = 0; j < 125 - 2 * i; j++)
            pixaAddPix(pixa1, pix1, L_COPY);
        pixaaAddPixa(paa, pixa1, L_INSERT);
    }
    pix2 = pixaaDisplayByPixa(paa, 50, 1.0, 10, 5, 0);
    pixWrite("/tmp/lept/misc/display.png", pix2, IFF_PNG);
    pixDisplay(pix2, 100, 100);
    pixaaDestroy(&paa);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Test the set and clear block functions in cmapped pix */
    lept_stderr("Test the set and clear block functions in cmapped pix\n");
    lept_stderr("******************************************************\n");
    lept_stderr("* Testing error checking: ignore two reported errors *\n");
    pix1 = pixRead("weasel4.11c.png");
    pixa1 = pixaCreate(0);
    pix2 = pixCopy(NULL, pix1);
    pixClearAll(pix2);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pix2 = pixCopy(NULL, pix1);
    pixSetAll(pix2);  /* error */
    pixaAddPix(pixa1, pix2, L_INSERT);
    pix2 = pixCopy(NULL, pix1);
    pixSetAllArbitrary(pix2, 4);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pix2 = pixCopy(NULL, pix1);
    pixSetAllArbitrary(pix2, 11);  /* warning */
    pixaAddPix(pixa1, pix2, L_INSERT);

    box1 = boxCreate(20, 20, 30, 30);
    pix2 = pixCopy(NULL, pix1);
    pixClearInRect(pix2, box1);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pix2 = pixCopy(NULL, pix1);
    pixSetInRect(pix2, box1);  /* error */
    pixaAddPix(pixa1, pix2, L_INSERT);
    pix2 = pixCopy(NULL, pix1);
    pixSetInRectArbitrary(pix2, box1, 4);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pix2 = pixCopy(NULL, pix1);
    pixSetInRectArbitrary(pix2, box1, 12);  /* warning */
    pixaAddPix(pixa1, pix2, L_INSERT);
    lept_stderr("******************************************************\n");

    pix3 = pixaDisplayTiledInColumns(pixa1, 10, 1.0, 15, 2);
    pixWrite("/tmp/lept/misc/setting.png", pix3, IFF_PNG);
    pixDisplay(pix3, 500, 100);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa1);

        /* Test zlib compression in png */
        /* Note that delta may be nonzero with some libraries */
    pixs = pixRead("feyn.tif");
    for (i = 0; i < 5; i++) {
        pixSetZlibCompression(pixs, 2 * i);
        pixWrite("/tmp/lept/misc/zlibtest.png", pixs, IFF_PNG);
        size = nbytesInFile("/tmp/lept/misc/zlibtest.png");
        lept_stderr("zlib level = %d, file size = %lu, delta = %lu\n",
                    2 * i, (unsigned long)size,
                    (unsigned long)(size - zlibsize[i]));
    }
    pixDestroy(&pixs);

        /* Show sampled scaling with and without source indexing shift */
    pixs = pixCreate(3, 3, 4);
    cmap = pixcmapCreateRandom(4, 0, 0);
    for (i = 0; i < 3; i++)
        for (j = 0; j < 3; j++)
            pixSetPixel(pixs, j, i, 3 * i + j);
    pixSetColormap(pixs, cmap);
    pix1 = pixScaleBySampling(pixs, 100, 100);
    pix2 = pixScaleBySamplingWithShift(pixs, 100, 100, 0.0, 0.0);
    pixa1 = pixaCreate(2);
    pixaAddPix(pixa1, pix1, L_INSERT);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pix3 = pixaDisplayTiledInColumns(pixa1, 2, 1.0, 30, 2);
    pixWrite("/tmp/lept/misc/sampletest.png", pix3, IFF_PNG);
    pixDisplay(pix3, 1000, 100);
    pixDestroy(&pixs);
    pixDestroy(&pix3);
    pixaDestroy(&pixa1);

        /* Display differences in images with pixDisplayDiff() */
    pix1 = pixRead("feyn-fract.tif");
    pix2 = pixTranslate(NULL, pix1, 20, 0, L_BRING_IN_WHITE);
    pix3 = pixDisplayDiff(pix1, pix2, 1, 1, 0xff000000);
    pixWrite("/tmp/lept/misc/diff-1bit.png", pix3, IFF_PNG);
    pixDisplay(pix3, 100, 1000);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pix1 = pixRead("test-rgb.png");
    pix2 = pixExpandReplicate(pix1, 4);
    pix3 = pixTranslate(NULL, pix2, 1, 0, L_BRING_IN_WHITE);
    pix4 = pixDisplayDiff(pix2, pix3, 1, 10, 0xff000000);
    pixWrite("/tmp/lept/misc/diff-32bit.png", pix4, IFF_PNG);
    pixDisplay(pix4, 400, 1000);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Demonstrate read of cmap+alpha png; I/O of rgba pnm, bmp, webp */
    pix1 = pixRead("elephant-cmap-alpha.png");  /* has colormap */
    pixDisplay(pix1, 1300, 800);
    pixWrite("/tmp/lept/misc/e.pnm", pix1, IFF_PNM);
    pixWrite("/tmp/lept/misc/e.bmp", pix1, IFF_BMP);
  #if HAVE_LIBWEBP
    pixWrite("/tmp/lept/misc/e.webp", pix1, IFF_WEBP);
  #endif  /* HAVE_LIBWEBP */
    pix2 = pixRead("/tmp/lept/misc/e.pnm");
    pixEqual(pix1, pix2, &same);
    lept_stderr("png vs pnm same? (yes): %d\n", same);
    pixDestroy(&pix2);
    pix2 = pixRead("/tmp/lept/misc/e.bmp");
    pixEqual(pix1, pix2, &same);
    lept_stderr("png vs bmp same? (yes): %d\n", same);
    pixDestroy(&pix2);
  #if HAVE_LIBWEBP
    pix2 = pixRead("/tmp/lept/misc/e.webp");
    pixDisplay(pix2, 1440, 800);  /* interesting change in rgb layer */
    pixEqual(pix1, pix2, &same);
    lept_stderr("png vs webp same? (no): %d\n", same);
    pixDestroy(&pix2);
  #endif  /* HAVE_LIBWEBP */
    pixDestroy(&pix1);

    return 0;
}
