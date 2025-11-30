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
 * blend4_reg.c
 *
 *   Regression test for this function:
 *       pixAddAlphaToBlend()
 *   Blending is done using pixBlendWithGrayMask()
 *
 *   Also, show blending of two color images using an alpha mask that
 *   varies linearly with radius from the center (which is transparent).
 */

#include "allheaders.h"
#include <math.h>

static PIX *AlphaRectangle(l_int32 w, l_int32 h, l_float32 fract);

static const char *blenders[] =
            {"feyn-word.tif", "weasel4.16c.png", "karen8.jpg"};

int main(int    argc,
         char **argv)
{
l_int32       i, w, h;
PIX          *pix0, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(0);

        /* Blending on a light image */
    pix1 = pixRead("fish24.jpg");
    pixGetDimensions(pix1, &w, &h, NULL);
    for (i = 0; i < 3; i++) {
        pix2 = pixRead(blenders[i]);
        if (i == 2) {
            pix3 = pixScale(pix2, 0.5, 0.5);
            pixDestroy(&pix2);
            pix2 = pix3;
        }
        pix3 = pixAddAlphaToBlend(pix2, 0.3, 0);
        pix4 = pixMirroredTiling(pix3, w, h);
        pix5 = pixBlendWithGrayMask(pix1, pix4, NULL, 0, 0);
        pixaAddPix(pixa, pix5, L_INSERT);
        regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 0 - 2 */
        pixDisplayWithTitle(pix5, 200 * i, 0, NULL, rp->display);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
    }
    pixDestroy(&pix1);

        /* Blending on a dark image */
    pix0 = pixRead("karen8.jpg");
    pix1 = pixScale(pix0, 2.0, 2.0);
    pixGetDimensions(pix1, &w, &h, NULL);
    for (i = 0; i < 2; i++) {
        pix2 = pixRead(blenders[i]);
        pix3 = pixAddAlphaToBlend(pix2, 0.3, 1);
        pix4 = pixMirroredTiling(pix3, w, h);
        pix5 = pixBlendWithGrayMask(pix1, pix4, NULL, 0, 0);
        pixaAddPix(pixa, pix5, L_INSERT);
        regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 3 - 4 */
        pixDisplayWithTitle(pix5, 600 + 200 * i, 0, NULL, rp->display);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
    }
    pixDestroy(&pix0);
    pixDestroy(&pix1);

        /* Blending of two color images using special mask */
    pix1 = pixRead("fish24.jpg");
    pix2 = pixRead("wyom.jpg");
    pixGetDimensions(pix2, &w, &h, NULL);
    pix3 = pixRotateOrth(pix1, 1);
    pix4 = pixScaleToSize(pix3, w, h);   /* same size as wyom.jpg */
    pix5 = AlphaRectangle(w, h, 1.0);
    pix6 = pixBlendWithGrayMask(pix4, pix2, pix5, 0, 0);
    pix7 = pixBlendWithGrayMask(pix2, pix4, pix5, 0, 0);
    pixDisplayWithTitle(pix6, 1000, 0, NULL, rp->display);
    pixDisplayWithTitle(pix7, 1000, 500, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 5 */
    regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 6 */
    regTestWritePixAndCheck(rp, pix6, IFF_JFIF_JPEG);  /* 7 */
    regTestWritePixAndCheck(rp, pix7, IFF_JFIF_JPEG);  /* 8 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix5, L_INSERT);
    pixaAddPix(pixa, pix6, L_INSERT);
    pixaAddPix(pixa, pix7, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix3);

    pixaConvertToPdf(pixa, 100, 1.0, L_JPEG_ENCODE, 0,
                     "Blendings: blend4_reg", "/tmp/lept/regout/blend.pdf");
    L_INFO("Output pdf: /tmp/lept/regout/blend.pdf\n", rp->testname);
    pixDestroy(&pix0);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

    return regTestCleanup(rp);
}


    /* Rectangular mask: opaque at center, linear change towards
     * transparency with distance from the center */
PIX *
AlphaRectangle(l_int32 w, l_int32 h, l_float32 fract)
{
l_int32    i, j, wpl, w2, h2, val;
l_float32  frdist;
l_uint32  *data, *line;
PIX       *pixd;

    pixd = pixCreate(w, h, 8);
    data = pixGetData(pixd);
    wpl = pixGetWpl(pixd);
    w2 = w / 2;
    h2 = h / 2;
    for (i = 0; i < h; i++) {
        line = data + i * wpl;
        for (j = 0; j < w; j++) {
            frdist = sqrt((h2 - i) * (h2 - i) + (w2 - j) * (w2 - j)) /
                     sqrt(w2 * w2 + h2 * h2);
            val = (l_int32)(255. * (1.0 - frdist * fract));
            SET_DATA_BYTE(line, j, val);
        }
    }

    return pixd;

}
