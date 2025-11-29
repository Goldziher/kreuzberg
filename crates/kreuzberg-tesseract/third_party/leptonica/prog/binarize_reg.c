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
 *  binarize_reg.c
 *
 *     Tests several methods of binarization:
 *     (1) Composite operations, including
 *         - contrast normalization and Sauvola binarization
 *         - contrast normalization followed by background normalization
 *           and thresholding.
 *     (2) Sauvola binarization with and without tiling
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

PIX *PixTest1(PIX *pixs, l_int32 size, l_float32 factor, L_REGPARAMS *rp);
PIX *PixTest2(PIX *pixs, l_int32 size, l_float32 factor, l_int32 nx,
              l_int32 ny, L_REGPARAMS *rp);
void PixTest3(PIX *pixs, l_int32 size, l_float32 factor,
              l_int32 nx, l_int32 ny, l_int32 paircount, L_REGPARAMS *rp);

int main(int    argc,
         char **argv)
{
PIX          *pixs, *pix1, *pix2;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("w91frag.jpg");

        /* Compare Sauvola binarization with and without tiles */
    pix1 = PixTest1(pixs, 7, 0.34, rp);  /* 0, 1 */
    pix2 = PixTest2(pixs, 7, 0.34, 4, 4, rp);  /* 2, 3 */
    regTestComparePix(rp, pix1, pix2);  /* 4 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* More comparisons of Sauvola with and without tiles. */
    PixTest3(pixs, 3, 0.20, 2, 3, 0, rp);  /* 5 - 9 */
    PixTest3(pixs, 6, 0.20, 100, 100, 1, rp);  /* 10 - 14 */
    PixTest3(pixs, 10, 0.40, 10, 10, 2, rp);  /* 15 - 19 */
    PixTest3(pixs, 10, 0.40, 20, 20, 3, rp);  /* 20 - 24 */
    PixTest3(pixs, 20, 0.34, 30, 30, 4, rp);  /* 25 - 29 */

        /* Contrast normalization followed by Sauvola */
    pixa = pixaCreate(0);
    pix1 = pixSauvolaOnContrastNorm(pixs, 130, NULL, NULL);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 30 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixaAddPix(pixa, pix1, L_INSERT);

       /* Contrast normalization followed by background normalization
        * and thresholding. */
    pix1 = pixThreshOnDoubleNorm(pixs, 130);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 31 */
    pixDisplayWithTitle(pix1, 850, 0, NULL, rp->display);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixaDisplayTiledInColumns(pixa, 2, 0.5, 30, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 32 */
    pixDisplayWithTitle(pix2, 0, 600, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix2);

    pixDestroy(&pixs);
    return regTestCleanup(rp);
}

PIX *PixTest1(PIX          *pixs,
              l_int32       size,
              l_float32     factor,
              L_REGPARAMS  *rp)
{
l_int32  w, h;
PIX     *pixm, *pixsd, *pixth, *pixd, *pix1;
PIXA    *pixa;

    pixm = pixsd = pixth = pixd = NULL;
    pixGetDimensions(pixs, &w, &h, NULL);

        /* Get speed */
    startTimer();
    pixSauvolaBinarize(pixs, size, factor, 1, NULL, NULL, NULL, &pixd);
    lept_stderr("\nSpeed: 1 tile,  %7.3f Mpix/sec\n",
                (w * h / 1000000.) / stopTimer());
    pixDestroy(&pixd);

        /* Get results witout tiling */
    pixSauvolaBinarize(pixs, size, factor, 1, &pixm, &pixsd, &pixth, &pixd);
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixm, L_INSERT);
    pixaAddPix(pixa, pixsd, L_INSERT);
    pixaAddPix(pixa, pixth, L_INSERT);
    pixaAddPix(pixa, pixd, L_COPY);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 30, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);
    if (rp->index < 5)
        pixDisplayWithTitle(pix1, 600, 600, NULL, rp->display);

    pixaDestroy(&pixa);
    pixDestroy(&pix1);
    return pixd;
}

PIX *PixTest2(PIX          *pixs,
              l_int32       size,
              l_float32     factor,
              l_int32       nx,
              l_int32       ny,
              L_REGPARAMS  *rp)
{
l_int32  w, h;
PIX     *pixth, *pixd, *pix1;
PIXA    *pixa;

    pixth = pixd = NULL;
    pixGetDimensions(pixs, &w, &h, NULL);

        /* Get speed */
    startTimer();
    pixSauvolaBinarizeTiled(pixs, size, factor, nx, ny, NULL, &pixd);
    lept_stderr("Speed: %d x %d tiles,  %7.3f Mpix/sec\n",
                nx, ny, (w * h / 1000000.) / stopTimer());
    pixDestroy(&pixd);

        /* Get results with tiling */
    pixSauvolaBinarizeTiled(pixs, size, factor, nx, ny, &pixth, &pixd);
    regTestWritePixAndCheck(rp, pixth, IFF_JFIF_JPEG);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    if (rp->index < 7 && rp->display) {
        pixa = pixaCreate(0);
        pixaAddPix(pixa, pixth, L_COPY);
        pixaAddPix(pixa, pixd, L_COPY);
        pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 30, 2);
        pixDisplayWithTitle(pix1, 600, 600, NULL, rp->display);
        pixDestroy(&pix1);
        pixaDestroy(&pixa);
    }

    pixDestroy(&pixth);
    return pixd;
}

void PixTest3(PIX          *pixs,
              l_int32       size,
              l_float32     factor,
              l_int32       nx,
              l_int32       ny,
              l_int32       paircount,
              L_REGPARAMS  *rp)
{
PIX  *pix1, *pix2;

        /* Compare with and without tiling */
    pix1 = PixTest1(pixs, size, factor, rp);
    pix2 = PixTest2(pixs, size, factor, nx, ny, rp);
    regTestComparePix(rp, pix1, pix2);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    return;
}
