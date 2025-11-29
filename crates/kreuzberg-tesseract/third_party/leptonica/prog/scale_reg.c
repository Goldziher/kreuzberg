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
 * scale_reg.c
 *
 *      This tests a number of scaling operations, through the pixScale()
 *      interface.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const char *image[10] = {"feyn-fract.tif",   /* 1 bpp */
                                "weasel2.png",      /* 2 bpp; no cmap */
                                "weasel2.4c.png",   /* 2 bpp; cmap */
                                "weasel4.png",      /* 4 bpp; no cmap */
                                "weasel4.16c.png",  /* 4 bpp; cmap */
                                "weasel8.png",      /* 8 bpp; no cmap */
                                "weasel8.240c.png", /* 8 bpp; cmap */
                                "test16.png",       /* 16 bpp rgb */
                                "marge.jpg",        /* 32 bpp rgb */
                                "test24.jpg"};      /* 32 bpp rgb */


static const l_int32    SPACE = 30;
static const l_int32    WIDTH = 300;
static const l_float32  FACTOR[5] = {2.3f, 1.5f, 1.1f, 0.6f, 0.3f};

static void AddScaledImages(PIXA *pixa, const char *fname, l_int32 width);
static void PixaSaveDisplay(PIXA *pixa, L_REGPARAMS *rp);
static void TestSmoothScaling(const char *fname, L_REGPARAMS *rp);

int main(int    argc,
         char **argv)
{
l_int32       i;
PIX          *pixs, *pixc;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Test 1 bpp */
    lept_stderr("\n-------------- Testing 1 bpp ----------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[0]);
    pixc = pixScale(pixs, 0.32, 0.32);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pixc, L_INSERT);

    pixc = pixScaleToGray3(pixs);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pixc, L_INSERT);

    pixc = pixScaleToGray4(pixs);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pixc, L_INSERT);

    pixc = pixScaleToGray6(pixs);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 3 */
    pixaAddPix(pixa, pixc, L_INSERT);

    pixc = pixScaleToGray8(pixs);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 4 */
    pixaAddPix(pixa, pixc, L_INSERT);

    pixc = pixScaleToGray16(pixs);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 6 */

    for (i = 1; i < 10; i++) {
        pixa = pixaCreate(0);
        AddScaledImages(pixa, image[i], WIDTH);
        PixaSaveDisplay(pixa, rp);  /* 7 - 15 */
    }
    pixDestroy(&pixs);

        /* Test 2 bpp without colormap */
    lept_stderr("\n-------------- Testing 2 bpp without cmap ----------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[1]);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixc = pixScale(pixs, 2.25, 2.25);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 16 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.85, 0.85);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 17 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.65, 0.65);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 18 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 19 */

        /* Test 2 bpp with colormap */
    lept_stderr("\n-------------- Testing 2 bpp with cmap ----------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[2]);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixc = pixScale(pixs, 2.25, 2.25);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 20 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.85, 0.85);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 21 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.65, 0.65);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 22 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 23 */

        /* Test 4 bpp without colormap */
    lept_stderr("\n-------------- Testing 4 bpp without cmap ----------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[3]);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixc = pixScale(pixs, 1.72, 1.72);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 24 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.85, 0.85);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 25 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.65, 0.65);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 26 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 27 */

        /* Test 4 bpp with colormap */
    lept_stderr("\n-------------- Testing 4 bpp with cmap ----------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[4]);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixc = pixScale(pixs, 1.72, 1.72);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 28 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.85, 0.85);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 29 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.65, 0.65);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 30 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 31 */

        /* Test 8 bpp without colormap */
    lept_stderr("\n-------------- Testing 8 bpp without cmap ----------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[5]);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixc = pixScale(pixs, 1.92, 1.92);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 32 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.85, 0.85);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 33 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.65, 0.65);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 34 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixs = pixRead("graytext.png");
    pixc = pixScaleToSize(pixs, 0, 32);  /* uses fast unsharp masking */
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 35 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 36 */
    pixDestroy(&pixs);

        /* Test 8 bpp with colormap */
    lept_stderr("\n-------------- Testing 8 bpp with cmap ----------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[6]);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixc = pixScale(pixs, 1.92, 1.92);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 37 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.85, 0.85);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 38 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.65, 0.65);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 39 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 40 */

        /* Test 16 bpp */
    lept_stderr("\n-------------- Testing 16 bpp ------------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[7]);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixc = pixScale(pixs, 1.92, 1.92);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 41 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.85, 0.85);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 42 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.65, 0.65);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 43 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 44 */

        /* Test 32 bpp */
    lept_stderr("\n-------------- Testing 32 bpp ------------\n");
    pixa = pixaCreate(0);
    pixs = pixRead(image[8]);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixc = pixScale(pixs, 1.42, 1.42);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 45 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.85, 0.85);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 46 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixc = pixScale(pixs, 0.65, 0.65);
    regTestWritePixAndCheck(rp, pixc, IFF_JFIF_JPEG);  /* 47 */
    pixaAddPix(pixa, pixc, L_INSERT);
    PixaSaveDisplay(pixa, rp);  /* 48 */

        /* Test 32 bpp low-pass filtered smooth scaling */
    TestSmoothScaling("test24.jpg", rp);  /* 49 */
    return regTestCleanup(rp);
}

static void
AddScaledImages(PIXA         *pixa,
                const char   *fname,
                l_int32       width)
{
l_int32    i, w;
l_float32  scalefactor;
PIX       *pixs, *pix1, *pix2, *pix3;

    pixs = pixRead(fname);
    w = pixGetWidth(pixs);
    for (i = 0; i < 5; i++) {
        scalefactor = (l_float32)width / (FACTOR[i] * (l_float32)w);
        pix1 = pixScale(pixs, FACTOR[i], FACTOR[i]);
        pix2 = pixScale(pix1, scalefactor, scalefactor);
        pix3 = pixConvertTo32(pix2);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    pixDestroy(&pixs);
}

static void
PixaSaveDisplay(PIXA *pixa, L_REGPARAMS *rp)
{
PIX  *pixd;

    pixd = pixaDisplayTiledInRows(pixa, 32, 3000, 1.0, 0, SPACE, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
}

static void
TestSmoothScaling(const char *fname, L_REGPARAMS *rp)
{
l_int32    i;
l_float32  scale, upscale;
PIX       *pix1, *pix2, *pix3;
PIXA      *pixa;

    pix1 = pixRead(fname);
    pixa = pixaCreate(12);
    scale = 0.5;
    for (i = 0; i < 12; i++) {
        scale *= 0.7;
        upscale = 0.25 / scale;
        if (rp->display) lept_stderr("scale = %5.3f\n", scale);
        pix2 = pixScaleSmooth(pix1, scale, scale);
        pix3 = pixScale(pix2, upscale, upscale);
        pixaAddPix(pixa, pix3, L_INSERT);
        pixDestroy(&pix2);
    }
    pix2 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 10, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);
    pixDisplayWithTitle(pix2, 0, 300, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
}

