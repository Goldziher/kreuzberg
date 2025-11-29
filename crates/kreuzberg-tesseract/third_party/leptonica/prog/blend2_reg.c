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
 * blend2_reg.c
 *
 *   Regression test for this function:
 *       pixBlendWithGrayMask()
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* In case libpng is not enabled */
#if defined(HAVE_LIBPNG)
static const char fname_png[64] = "/tmp/lept/regout/blend2.14.png";
#else
static const char fname_bmp[64] = "/tmp/lept/regout/blend2.14.bmp";
#endif

int main(int    argc,
         char **argv)
{
l_int32       i, j, w1, h1, w2, h2, w, h;
BOX          *box1, *box2;
PIX          *pixg, *pixs1, *pixs2, *pix1, *pix2, *pix3, *pix4, *pix5;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* --- Set up the 8 bpp blending image --- */
    pixg = pixCreate(660, 500, 8);
    for (i = 0; i < 500; i++)
        for (j = 0; j < 660; j++)
            pixSetPixel(pixg, j, i, (l_int32)(0.775 * j) % 256);

        /* --- Set up the initial color images to be blended together --- */
    pixs1 = pixRead("wyom.jpg");
    pixs2 = pixRead("fish24.jpg");
    pixGetDimensions(pixs1, &w, &h, NULL);
    pixGetDimensions(pixs1, &w1, &h1, NULL);
    pixGetDimensions(pixs2, &w2, &h2, NULL);
    h = L_MIN(h1, h2);
    w = L_MIN(w1, w2);
    box1 = boxCreate(0, 0, w, h);
    box2 = boxCreate(0, 300, 660, 500);
    pix1 = pixClipRectangle(pixs1, box1, NULL);
    pix2 = pixClipRectangle(pixs2, box2, NULL);
    pixDestroy(&pixs1);
    pixDestroy(&pixs2);
    boxDestroy(&box1);
    boxDestroy(&box2);

        /* --- Blend 2 rgb images --- */
    pixa = pixaCreate(0);
    pix3 = pixBlendWithGrayMask(pix1, pix2, pixg, 50, 50);
    pixaAddPix(pixa, pix1, L_COPY);
    pixaAddPix(pixa, pix2, L_COPY);
    pixaAddPix(pixa, pix3, L_INSERT);
    regTestWritePixAndCheck(rp, pixg, IFF_JFIF_JPEG);  /* 0 */
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 1 */
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 2 */
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 3 */

        /* --- Blend 2 grayscale images --- */
    pix3 = pixConvertRGBToLuminance(pix1);
    pix4 = pixConvertRGBToLuminance(pix2);
    pix5 = pixBlendWithGrayMask(pix3, pix4, pixg, 50, 50);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 4 */
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 5 */
    regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 6 */

        /* --- Blend a colormap image and an rgb image --- */
    pix3 = pixFixedOctcubeQuantGenRGB(pix2, 2);
    pix4 = pixBlendWithGrayMask(pix1, pix3, pixg, 50, 50);
    pixaAddPix(pixa, pix1, L_COPY);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 7 */
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 8 */

        /* --- Blend a colormap image and a grayscale image --- */
    pix3 = pixConvertRGBToLuminance(pix1);
    pix4 = pixFixedOctcubeQuantGenRGB(pix2, 2);
    pix5 = pixBlendWithGrayMask(pix3, pix4, pixg, 50, 50);
    pixaAddPix(pixa, pix3, L_COPY);
    pixaAddPix(pixa, pix4, L_COPY);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 9 */
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 10 */
    regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 11 */
    pix5 = pixBlendWithGrayMask(pix3, pix4, pixg, -100, -100);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 12 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);

    /* --------- Test png read/write with alpha channel --------- */
        /* First make pix2, using pixg as the alpha channel */
    pix1 = pixRead("fish24.jpg");
    box1 = boxCreate(0, 300, 660, 500);
    pix2 = pixClipRectangle(pix1, box1, NULL);
    boxDestroy(&box1);
    pixaAddPix(pixa, pix2, L_COPY);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 13 */
    pixSetRGBComponent(pix2, pixg, L_ALPHA_CHANNEL);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 14 */

        /* To see the alpha channel, blend with a black image */
    pix3 = pixCreate(660, 500, 32);
    pix4 = pixBlendWithGrayMask(pix3, pix2, NULL, 0, 0);
    pixaAddPix(pixa, pix4, L_INSERT);
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 15 */

        /* Read the RGBA image #14 back */
#if defined(HAVE_LIBPNG)
    pix4 = pixRead(fname_png);
#else
    pix4 = pixRead(fname_bmp);
#endif

        /* Make sure that the alpha channel image hasn't changed */
    pix5 = pixGetRGBComponent(pix4, L_ALPHA_CHANNEL);
    regTestComparePix(rp, pixg, pix5);  /* 16 */
    pixDestroy(&pix5);

        /* Blend again with a black image */
    pix5 = pixBlendWithGrayMask(pix3, pix4, NULL, 0, 0);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 17 */

        /* Blend with a white image */
    pixSetAll(pix3);
    pix5 = pixBlendWithGrayMask(pix3, pix4, NULL, 0, 0);
    pixaAddPix(pixa, pix5, L_INSERT);
    regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 18 */
    pixDestroy(&pixg);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Display results */
    pix1 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 40, 2);
    pixDisplayWithTitle(pix1, 100, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 19 */
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

    return regTestCleanup(rp);
}
