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
 * watershed_reg.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

void DoWatershed(L_REGPARAMS *rp, PIX *pixs);

int main(int    argc,
         char **argv)
{
l_int32       i, j;
l_float32     f;
PIX          *pix1, *pix2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pix1 = pixCreate(500, 500, 8);
    pix2 = pixCreate(500, 500, 8);
    for (i = 0; i < 500; i++) {
        for (j = 0; j < 500; j++) {
            f = 128.0 + 26.3 * sin(0.0438 * (l_float32)i);
            f += 33.4 * cos(0.0712 * (l_float32)i);
            f += 18.6 * sin(0.0561 * (l_float32)j);
            f += 23.6 * cos(0.0327 * (l_float32)j);
            pixSetPixel(pix1, j, i, (l_int32)f);
            f = 128.0 + 26.3 * sin(0.0238 * (l_float32)i);
            f += 33.4 * cos(0.0312 * (l_float32)i);
            f += 18.6 * sin(0.0261 * (l_float32)j);
            f += 23.6 * cos(0.0207 * (l_float32)j);
            pixSetPixel(pix2, j, i, (l_int32)f);
        }
    }
    DoWatershed(rp, pix1);  /* 0 - 11 */
    DoWatershed(rp, pix2);  /* 12 - 23 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    return regTestCleanup(rp);
}


void
DoWatershed(L_REGPARAMS  *rp,
            PIX          *pixs)
{
l_uint8   *data;
size_t     size;
l_int32    w, h, empty;
l_uint32   redval, greenval;
L_WSHED   *wshed;
PIX       *pixc, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7, *pix8, *pix9;
PIXA      *pixa;
PTA       *pta;

        /* Find local extrema */
    pixa = pixaCreate(0);
    pixGetDimensions(pixs, &w, &h, NULL);
    regTestWritePixAndCheck(rp, pixs, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pixs, L_COPY);
    startTimer();
    pixLocalExtrema(pixs, 0, 0, &pix1, &pix2);
    lept_stderr("Time for extrema: %7.3f\n", stopTimer());
    pixSetOrClearBorder(pix1, 2, 2, 2, 2, PIX_CLR);
    composeRGBPixel(255, 0, 0, &redval);
    composeRGBPixel(0, 255, 0, &greenval);
    pixc = pixConvertTo32(pixs);
    pixPaintThroughMask(pixc, pix2, 0, 0, greenval);
    pixPaintThroughMask(pixc, pix1, 0, 0, redval);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pixc, L_INSERT);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pix1, L_COPY);

        /* Generate seeds for watershed */
    pixSelectMinInConnComp(pixs, pix1, &pta, NULL);
    pix3 = pixGenerateFromPta(pta, w, h);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */
    pixaAddPix(pixa, pix3, L_COPY);
    pix4 = pixConvertTo32(pixs);
    pixPaintThroughMask(pix4, pix3, 0, 0, greenval);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 4 */
    pixaAddPix(pixa, pix4, L_COPY);
    pix5 = pixRemoveSeededComponents(NULL, pix3, pix1, 8, 2);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pix5, L_COPY);
    pixZero(pix5, &empty);
    regTestCompareValues(rp, 1, empty, 0.0);  /* 6 */

        /* Make and display watershed */
    wshed = wshedCreate(pixs, pix3, 10, 0);
    startTimer();
    wshedApply(wshed);
    lept_stderr("Time for wshed: %7.3f\n", stopTimer());
    pix6 = pixaDisplayRandomCmap(wshed->pixad, w, h);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 7 */
    pixaAddPix(pixa, pix6, L_COPY);
    numaWriteMem(&data, &size, wshed->nalevels);
    regTestWriteDataAndCheck(rp, data, size, "na");  /* 8 */
    pix7 = wshedRenderFill(wshed);
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 9 */
    pixaAddPix(pixa, pix7, L_COPY);
    pix8 = wshedRenderColors(wshed);
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 10 */
    pixaAddPix(pixa, pix8, L_COPY);
    wshedDestroy(&wshed);

    pix9 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pix9, IFF_PNG);  /* 11 */
    pixDisplayWithTitle(pix9, 100, 100, NULL, rp->display);

    lept_free(data);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixDestroy(&pix8);
    pixDestroy(&pix9);
    pixaDestroy(&pixa);
    ptaDestroy(&pta);
}

