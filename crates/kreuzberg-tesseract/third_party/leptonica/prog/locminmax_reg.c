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
 * locminmax_reg.c
 *
 *    Note: you can remove all minima that are touching the border, using:
 *             pix3 = pixRemoveBorderConnComps(pix1, 8);
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

void DoLocMinmax(L_REGPARAMS *rp, PIX *pixs, l_int32 minmax, l_int32 maxmin);

int main(int    argc,
         char **argv)
{
l_int32       i, j;
l_float32     f;
PIX          *pix1, *pix2, *pix3;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pix1 = pixCreate(500, 500, 8);
    for (i = 0; i < 500; i++) {
        for (j = 0; j < 500; j++) {
            f = 128.0 + 26.3 * sin(0.0438 * (l_float32)i);
            f += 33.4 * cos(0.0712 * (l_float32)i);
            f += 18.6 * sin(0.0561 * (l_float32)j);
            f += 23.6 * cos(0.0327 * (l_float32)j);
            pixSetPixel(pix1, j, i, (l_int32)f);
        }
    }
    pix2 = pixRead("karen8.jpg");
    pix3 = pixBlockconv(pix2, 10, 10);
    DoLocMinmax(rp, pix1, 0, 0);  /* 0 - 2 */
    DoLocMinmax(rp, pix3, 50, 100);  /* 3 - 5 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    return regTestCleanup(rp);
}

void
DoLocMinmax(L_REGPARAMS  *rp,
            PIX          *pixs,
            l_int32       minmax,
            l_int32       maxmin)
{
l_uint32  redval, greenval;
PIX      *pix1, *pix2, *pix3, *pixd;
PIXA     *pixa;

    pixa = pixaCreate(0);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pixs, L_COPY);
    pixLocalExtrema(pixs, minmax, maxmin, &pix1, &pix2);
    composeRGBPixel(255, 0, 0, &redval);
    composeRGBPixel(0, 255, 0, &greenval);
    pixd = pixConvertTo32(pixs);
    pixPaintThroughMask(pixd, pix2, 0, 0, greenval);
    pixPaintThroughMask(pixd, pix1, 0, 0, redval);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pix3 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 25, 2);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix3, 300, 0, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
}
