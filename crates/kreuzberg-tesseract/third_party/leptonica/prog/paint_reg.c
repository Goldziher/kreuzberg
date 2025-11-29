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
 * paint_reg.c
 *
 *   Regression test for:
 *      (1) painting on images of various types and depths.
 *      (2) painting through masks (test by reconstructing cmapped image)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "pix_internal.h"

static PIX * ReconstructByValue(L_REGPARAMS *rp, const char *fname);
static PIX * FakeReconstructByBand(L_REGPARAMS *rp, const char *fname);


int main(int    argc,
         char **argv)
{
l_int32       index;
l_uint32      val32;
BOX          *box, *box1, *box2, *box3, *box4, *box5;
BOXA         *boxa;
L_KERNEL     *kel;
PIX          *pixs, *pixg, *pixb, *pixd, *pixt, *pix1, *pix2, *pix3, *pix4;
PIXA         *pixa;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(0);

        /* Color non-white pixels on RGB */
    pixs = pixRead("lucasta-frag.jpg");
    pixt = pixConvert8To32(pixs);
    box = boxCreate(120, 30, 200, 200);
    pixColorGray(pixt, box, L_PAINT_DARK, 220, 0, 0, 255);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 0 */
    pixaAddPix(pixa, pixt, L_COPY);
    pixColorGray(pixt, NULL, L_PAINT_DARK, 220, 255, 100, 100);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 1 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);

        /* Color non-white pixels on colormap */
    pixt = pixThresholdTo4bpp(pixs, 6, 1);
    box = boxCreate(120, 30, 200, 200);
    pixColorGray(pixt, box, L_PAINT_DARK, 220, 0, 0, 255);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pixt, L_COPY);
    pixColorGray(pixt, NULL, L_PAINT_DARK, 220, 255, 100, 100);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 3 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);

        /* Color non-black pixels on RGB */
    pixt = pixConvert8To32(pixs);
    box = boxCreate(120, 30, 200, 200);
    pixColorGray(pixt, box, L_PAINT_LIGHT, 20, 0, 0, 255);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 4 */
    pixaAddPix(pixa, pixt, L_COPY);
    pixColorGray(pixt, NULL, L_PAINT_LIGHT, 80, 255, 100, 100);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);

        /* Color non-black pixels on colormap */
    pixt = pixThresholdTo4bpp(pixs, 6, 1);
    box = boxCreate(120, 30, 200, 200);
    pixColorGray(pixt, box, L_PAINT_LIGHT, 20, 0, 0, 255);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 6 */
    pixaAddPix(pixa, pixt, L_COPY);
    pixColorGray(pixt, NULL, L_PAINT_LIGHT, 20, 255, 100, 100);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 7 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);

        /* Add highlight color to RGB */
    pixt = pixConvert8To32(pixs);
    box = boxCreate(507, 5, 385, 45);
    pixg = pixClipRectangle(pixs, box, NULL);
    pixb = pixThresholdToBinary(pixg, 180);
    pixInvert(pixb, pixb);
    composeRGBPixel(50, 0, 250, &val32);
    pixPaintThroughMask(pixt, pixb, box->x, box->y, val32);
    boxDestroy(&box);
    pixDestroy(&pixg);
    pixDestroy(&pixb);
    box = boxCreate(236, 107, 262, 40);
    pixg = pixClipRectangle(pixs, box, NULL);
    pixb = pixThresholdToBinary(pixg, 180);
    pixInvert(pixb, pixb);
    composeRGBPixel(250, 0, 50, &val32);
    pixPaintThroughMask(pixt, pixb, box->x, box->y, val32);
    boxDestroy(&box);
    pixDestroy(&pixg);
    pixDestroy(&pixb);
    box = boxCreate(222, 208, 247, 43);
    pixg = pixClipRectangle(pixs, box, NULL);
    pixb = pixThresholdToBinary(pixg, 180);
    pixInvert(pixb, pixb);
    composeRGBPixel(60, 250, 60, &val32);
    pixPaintThroughMask(pixt, pixb, box->x, box->y, val32);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 8 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);
    pixDestroy(&pixg);
    pixDestroy(&pixb);

        /* Add highlight color to colormap */
    pixt = pixThresholdTo4bpp(pixs, 5, 1);
    cmap = pixGetColormap(pixt);
    pixcmapGetIndex(cmap, 255, 255, 255, &index);
    box = boxCreate(507, 5, 385, 45);
    pixSetSelectCmap(pixt, box, index, 50, 0, 250);
    boxDestroy(&box);
    box = boxCreate(236, 107, 262, 40);
    pixSetSelectCmap(pixt, box, index, 250, 0, 50);
    boxDestroy(&box);
    box = boxCreate(222, 208, 247, 43);
    pixSetSelectCmap(pixt, box, index, 60, 250, 60);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 9 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);

        /* Paint lines on RGB */
    pixt = pixConvert8To32(pixs);
    pixRenderLineArb(pixt, 450, 20, 850, 320, 5, 200, 50, 125);
    pixRenderLineArb(pixt, 30, 40, 440, 40, 5, 100, 200, 25);
    box = boxCreate(70, 80, 300, 245);
    pixRenderBoxArb(pixt, box, 3, 200, 200, 25);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 10 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);

        /* Paint lines on colormap */
    pixt = pixThresholdTo4bpp(pixs, 5, 1);
    pixRenderLineArb(pixt, 450, 20, 850, 320, 5, 200, 50, 125);
    pixRenderLineArb(pixt, 30, 40, 440, 40, 5, 100, 200, 25);
    box = boxCreate(70, 80, 300, 245);
    pixRenderBoxArb(pixt, box, 3, 200, 200, 25);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 11 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);

        /* Blend lines on RGB */
    pixt = pixConvert8To32(pixs);
    pixRenderLineBlend(pixt, 450, 20, 850, 320, 5, 200, 50, 125, 0.35);
    pixRenderLineBlend(pixt, 30, 40, 440, 40, 5, 100, 200, 25, 0.35);
    box = boxCreate(70, 80, 300, 245);
    pixRenderBoxBlend(pixt, box, 3, 200, 200, 25, 0.6);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 12 */
    pixaAddPix(pixa, pixt, L_INSERT);
    boxDestroy(&box);

        /* Colorize gray on cmapped image. */
    pix1 = pixRead("lucasta.150.jpg");
    pix2 = pixThresholdTo4bpp(pix1, 7, 1);
    box1 = boxCreate(73, 206, 140, 27);
    pixColorGrayCmap(pix2, box1, L_PAINT_LIGHT, 130, 207, 43);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 13 */
    pixaAddPix(pixa, pix2, L_COPY);
    if (rp->display)
        pixPrintStreamInfo(stderr, pix2, "One box added");

    box2 = boxCreate(255, 404, 197, 25);
    pixColorGrayCmap(pix2, box2, L_PAINT_LIGHT, 230, 67, 119);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 14 */
    pixaAddPix(pixa, pix2, L_COPY);
    if (rp->display)
        pixPrintStreamInfo(stderr, pix2, "Two boxes added");

    box3 = boxCreate(122, 756, 224, 22);
    pixColorGrayCmap(pix2, box3, L_PAINT_DARK, 230, 67, 119);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 15 */
    pixaAddPix(pixa, pix2, L_COPY);
    if (rp->display)
        pixPrintStreamInfo(stderr, pix2, "Three boxes added");

    box4 = boxCreate(11, 780, 147, 22);
    pixColorGrayCmap(pix2, box4, L_PAINT_LIGHT, 70, 137, 229);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 16 */
    pixaAddPix(pixa, pix2, L_COPY);
    if (rp->display)
        pixPrintStreamInfo(stderr, pix2, "Four boxes added");

    box5 = boxCreate(163, 605, 78, 22);
    pixColorGrayCmap(pix2, box5, L_PAINT_LIGHT, 70, 137, 229);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 17 */
    pixaAddPix(pixa, pix2, L_INSERT);
    if (rp->display)
        pixPrintStreamInfo(stderr, pix2, "Five boxes added");
    pixDestroy(&pix1);
    boxDestroy(&box1);
    boxDestroy(&box2);
    boxDestroy(&box3);
    boxDestroy(&box4);
    boxDestroy(&box5);
    pixDestroy(&pixs);

        /* Make a gray image and identify the fg pixels (val > 230) */
    pixs = pixRead("feyn-fract.tif");
    pix1 = pixConvertTo8(pixs, 0);
    kel = makeGaussianKernel(2, 2, 1.5, 1.0);
    pix2 = pixConvolve(pix1, kel, 8, 1);
    pix3 = pixThresholdToBinary(pix2, 230);
    boxa = pixConnComp(pix3, NULL, 8);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    kernelDestroy(&kel);

        /* Color the individual components in the gray image */
    pix4 = pixColorGrayRegions(pix2, boxa, L_PAINT_DARK, 230, 255, 0, 0);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 18 */
    pixaAddPix(pixa, pix4, L_INSERT);
    pixDisplayWithTitle(pix4, 0, 0, NULL, rp->display);

        /* Threshold to 10 levels of gray */
    pix3 = pixThresholdOn8bpp(pix2, 10, 1);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 19 */
    pixaAddPix(pixa, pix3, L_COPY);

        /* Color the individual components in the cmapped image */
    pix4 = pixColorGrayRegions(pix3, boxa, L_PAINT_DARK, 230, 255, 0, 0);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 20 */
    pixaAddPix(pixa, pix4, L_INSERT);
    pixDisplayWithTitle(pix4, 0, 100, NULL, rp->display);
    boxaDestroy(&boxa);

        /* Color the entire gray image (not component-wise) */
    pixColorGray(pix2, NULL, L_PAINT_DARK, 230, 255, 0, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 21 */
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Color the entire cmapped image (not component-wise) */
    pixColorGray(pix3, NULL, L_PAINT_DARK, 230, 255, 0, 0);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 22 */
    pixaAddPix(pixa, pix3, L_INSERT);

        /* Reconstruct cmapped images */
    pixd = ReconstructByValue(rp, "weasel2.4c.png");
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 23 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixd = ReconstructByValue(rp, "weasel4.11c.png");
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 24 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixd = ReconstructByValue(rp, "weasel8.240c.png");
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 25 */
    pixaAddPix(pixa, pixd, L_INSERT);

        /* Fake reconstruct cmapped images, with one color into a band */
    pixd = FakeReconstructByBand(rp, "weasel2.4c.png");
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 26 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixd = FakeReconstructByBand(rp, "weasel4.11c.png");
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 27 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixd = FakeReconstructByBand(rp, "weasel8.240c.png");
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 28 */
    pixaAddPix(pixa, pixd, L_INSERT);

        /* If in testing mode, make a pdf */
    if (rp->display) {
        pixaConvertToPdf(pixa, 100, 1.0, L_FLATE_ENCODE, 0,
                         "Colorize and paint", "/tmp/lept/regout/paint.pdf");
        L_INFO("Output pdf: /tmp/lept/regout/paint.pdf\n", rp->testname);
    }

    pixaDestroy(&pixa);
    return regTestCleanup(rp);
}


static PIX *
ReconstructByValue(L_REGPARAMS  *rp,
                   const char   *fname)
{
l_int32   i, n, rval, gval, bval;
PIX      *pixs, *pixm, *pixd;
PIXCMAP  *cmap;

    pixs = pixRead(fname);
    cmap = pixGetColormap(pixs);
    n = pixcmapGetCount(cmap);
    pixd = pixCreateTemplate(pixs);
    for (i = 0; i < n; i++) {
        pixm = pixGenerateMaskByValue(pixs, i, 1);
        pixcmapGetColor(cmap, i, &rval, &gval, &bval);
        pixSetMaskedCmap(pixd, pixm, 0, 0, rval, gval, bval);
        pixDestroy(&pixm);
    }

    regTestComparePix(rp, pixs, pixd);
    pixDestroy(&pixs);
    return pixd;
}

static PIX *
FakeReconstructByBand(L_REGPARAMS  *rp,
                      const char   *fname)
{
l_int32   i, jlow, jup, n, nbands;
l_int32   rval1, gval1, bval1, rval2, gval2, bval2, rval, gval, bval;
PIX      *pixs, *pixm, *pixd;
PIXCMAP  *cmaps, *cmapd;

    pixs = pixRead(fname);
    cmaps = pixGetColormap(pixs);
    n = pixcmapGetCount(cmaps);
    nbands = (n + 1) / 2;
    pixd = pixCreateTemplate(pixs);
    cmapd = pixcmapCreate(pixGetDepth(pixs));
    pixSetColormap(pixd, cmapd);
    for (i = 0; i < nbands; i++) {
        jlow = 2 * i;
        jup = L_MIN(jlow + 1, n - 1);
        pixm = pixGenerateMaskByBand(pixs, jlow, jup, 1, 1);

            /* Get average color in the band */
        pixcmapGetColor(cmaps, jlow, &rval1, &gval1, &bval1);
        pixcmapGetColor(cmaps, jup, &rval2, &gval2, &bval2);
        rval = (rval1 + rval2) / 2;
        gval = (gval1 + gval2) / 2;
        bval = (bval1 + bval2) / 2;

        pixcmapAddColor(cmapd, rval, gval, bval);
        pixSetMaskedCmap(pixd, pixm, 0, 0, rval, gval, bval);
        pixDestroy(&pixm);
    }

    pixDestroy(&pixs);
    return pixd;
}
