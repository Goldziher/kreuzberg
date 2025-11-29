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
 *  blend3_reg.c
 *
 *    42 results: 6 input image combinations * 7 blendings
 */

#include "allheaders.h"

#define   X     140
#define   Y     40
#define   ALL   1

static PIX *BlendTest(const char *file1, const char *file2, l_float32 fract);

int main(int    argc,
         char **argv)
{
PIX          *pixt, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(6);

    pixt = BlendTest("marge.jpg", "feyn-word.tif", 0.5);
    pixaAddPix(pixa, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 0 */
    pixDisplayWithTitle(pixt, 0, 0, NULL, rp->display);

    pixt = BlendTest("marge.jpg", "weasel8.png", 0.3);
    pixaAddPix(pixa, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 1 */
    pixDisplayWithTitle(pixt, 0, 200, NULL, rp->display);

    pixt = BlendTest("marge.jpg", "weasel8.240c.png", 0.3);
    pixaAddPix(pixa, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 2 */
    pixDisplayWithTitle(pixt, 0, 400, NULL, rp->display);

    pixt = BlendTest("test8.jpg", "feyn-word.tif", 0.5);
    pixaAddPix(pixa, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 3 */
    pixDisplayWithTitle(pixt, 0, 600, NULL, rp->display);

    pixt = BlendTest("test8.jpg", "weasel8.png", 0.5);
    pixaAddPix(pixa, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 4 */
    pixDisplayWithTitle(pixt, 0, 800, NULL, rp->display);

    pixt = BlendTest("test8.jpg", "weasel8.240c.png", 0.6);
    pixaAddPix(pixa, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 5 */
    pixDisplayWithTitle(pixt, 0, 1000, NULL, rp->display);

    pixd = pixaDisplayTiledInRows(pixa, 32, 1800, 1.0, 0, 20, 2);
    pixWrite("/tmp/lept/regout/blendall.jpg", pixd, IFF_JFIF_JPEG);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);

    return regTestCleanup(rp);
}


static PIX *
BlendTest(const char  *file1,
          const char  *file2,
          l_float32    fract)
{
l_int32  d1, d2;
PIX     *pixs1, *pixs2, *pix1, *pix2, *pix3, *pix4, *pix5, *pixd;
PIXA    *pixa;

    pixs1 = pixRead(file1);
    pixs2 = pixRead(file2);
    d1 = pixGetDepth(pixs1);
    d2 = pixGetDepth(pixs2);
    pixa = pixaCreate(7);

#if ALL
    if (d1 == 1) {
        pix1 = pixBlend(pixs1, pixs2, X, Y, fract);
        pix2 = pixBlend(pix1, pixs2, X, Y + 60, fract);
        pix3 = pixBlend(pix2, pixs2, X, Y + 120, fract);
        pix4 = pixBlend(pix3, pixs2, X, Y + 180, fract);
        pix5 = pixBlend(pix4, pixs2, X, Y + 240, fract);
        pixd = pixBlend(pix5, pixs2, X, Y + 300, fract);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
        pixDestroy(&pix5);
    } else {
        pix1 = pixBlend(pixs1, pixs2, X, Y, fract);
        pix2 = pixBlend(pix1, pixs2, X + 80, Y + 80, fract);
        pix3 = pixBlend(pix2, pixs2, X + 160, Y + 160, fract);
        pix4 = pixBlend(pix3, pixs2, X + 240, Y + 240, fract);
        pix5 = pixBlend(pix4, pixs2, X + 320, Y + 320, fract);
        pixd = pixBlend(pix5, pixs2, X + 360, Y + 360, fract);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
        pixDestroy(&pix5);
    }
    pixaAddPix(pixa, pixd, L_INSERT);
#endif

#if ALL
        /* Gray blend */
    if (d2 >= 8)
        pixSnapColor(pixs2, pixs2, 0xff, 0xff, 50);
    pixd = pixBlendGray(NULL, pixs1, pixs2, 200, 100, fract,
                        L_BLEND_GRAY, 1, 0xff);
    pixBlendGray(pixd, pixd, pixs2, 200, 200, fract,
                 L_BLEND_GRAY, 1, 0xff);
    pixBlendGray(pixd, pixd, pixs2, 200, 260, fract,
                 L_BLEND_GRAY, 1, 0xff);
    pixBlendGray(pixd, pixd, pixs2, 200, 340, fract,
                 L_BLEND_GRAY, 1, 0xff);
    pixaAddPix(pixa, pixd, L_INSERT);
#endif

#if ALL  /* Gray blend (with inverse) */
    if (d2 >= 8)
        pixSnapColor(pixs2, pixs2, 0xff, 0xff, 50);
    pixd = pixBlendGray(NULL,  pixs1, pixs2, 200, 100, fract,
                 L_BLEND_GRAY_WITH_INVERSE, 1, 0xff);
    pixBlendGray(pixd, pixd, pixs2, 200, 200, fract,
                 L_BLEND_GRAY_WITH_INVERSE, 1, 0xff);
    pixBlendGray(pixd, pixd, pixs2, 200, 260, fract,
                 L_BLEND_GRAY_WITH_INVERSE, 1, 0xff);
    pixBlendGray(pixd, pixd, pixs2, 200, 340, fract,
                 L_BLEND_GRAY_WITH_INVERSE, 1, 0xff);
    pixaAddPix(pixa, pixd, L_INSERT);
#endif

#if ALL  /* Blend Gray for robustness */
    if (d2 >= 8)
        pixSnapColor(pixs2, pixs2, 0xff, 0xff, 50);
    pixd = pixBlendGrayInverse(NULL,  pixs1, pixs2, 200, 100, fract);
    pixBlendGrayInverse(pixd, pixd, pixs2, 200, 200, fract);
    pixBlendGrayInverse(pixd, pixd, pixs2, 200, 260, fract);
    pixBlendGrayInverse(pixd, pixd, pixs2, 200, 340, fract);
    pixaAddPix(pixa, pixd, L_INSERT);
#endif

#if ALL  /* Blend Gray adapted */
    if (d2 >= 8)
        pixSnapColor(pixs2, pixs2, 0xff, 0xff, 50);
    pixd = pixBlendGrayAdapt(NULL,  pixs1, pixs2, 200, 100, fract, 120);
    pixBlendGrayAdapt(pixd, pixd, pixs2, 200, 200, fract, 120);
    pixBlendGrayAdapt(pixd, pixd, pixs2, 200, 260, fract, 120);
    pixBlendGrayAdapt(pixd, pixd, pixs2, 200, 340, fract, 120);
    pixaAddPix(pixa, pixd, L_INSERT);
#endif

#if ALL  /* Blend color */
    if (d2 >= 8)
         pixSnapColor(pixs2, pixs2, 0xffffff00, 0xffffff00, 50);
    pixd = pixBlendColor(NULL, pixs1, pixs2, 200, 100, fract, 1, 0xffffff00);
    pixBlendColor(pixd, pixd, pixs2, 200, 200, fract, 1, 0xffffff00);
    pixBlendColor(pixd, pixd, pixs2, 200, 260, fract, 1, 0xffffff00);
    pixBlendColor(pixd, pixd, pixs2, 200, 340, fract, 1, 0xffffff00);
    pixaAddPix(pixa, pixd, L_INSERT);
#endif

#if ALL  /* Blend color by channel */
    if (d2 >= 8)
         pixSnapColor(pixs2, pixs2, 0xffffff00, 0xffffff00, 50);
    pixd = pixBlendColorByChannel(NULL, pixs1, pixs2, 200, 100, 1.6 * fract,
                         fract, 0.5 * fract, 1, 0xffffff00);
    pixBlendColorByChannel(pixd, pixd, pixs2, 200, 200, 1.2 * fract,
                  fract, 0.2 * fract, 1, 0xffffff00);
    pixBlendColorByChannel(pixd, pixd, pixs2, 200, 260, 1.6 * fract,
                  1.8 * fract, 0.3 * fract, 1, 0xffffff00);
    pixBlendColorByChannel(pixd, pixd, pixs2, 200, 340, 0.4 * fract,
                  1.3 * fract, 1.8 * fract, 1, 0xffffff00);
    pixaAddPix(pixa, pixd, L_INSERT);
#endif

    pixd = pixaDisplayTiledInRows(pixa, 32, 2500, 0.5, 0, 20, 2);
    pixaDestroy(&pixa);
    pixDestroy(&pixs1);
    pixDestroy(&pixs2);
    return pixd;
}

