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
 * edgetest.c
 *
 *   Regression test for sobel edge filter.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, w, h;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("test8.jpg");

        /* Test speed: about 60 Mpix/sec/GHz */
    startTimer();
    for (i = 0; i < 100; i++) {
        pix1 = pixSobelEdgeFilter(pixs, L_HORIZONTAL_EDGES);
        pix2 = pixThresholdToBinary(pix1, 60);
        pixInvert(pix2, pix2);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    pixGetDimensions(pixs, &w, &h, NULL);
    lept_stderr("Sobel edge MPix/sec: %7.3f\n", 0.0001 * w * h / stopTimer());

        /* Horiz and vert sobel edges (1 bpp) */
    pix1 = pixSobelEdgeFilter(pixs, L_HORIZONTAL_EDGES);
    pix2 = pixThresholdToBinary(pix1, 60);
    pixInvert(pix2, pix2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix2, 0, 50, "Horizontal edges", rp->display);
    pix3 = pixSobelEdgeFilter(pixs, L_VERTICAL_EDGES);
    pix4 = pixThresholdToBinary(pix3, 60);
    pixInvert(pix4, pix4);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix4, 625, 50, "Vertical edges", rp->display);
    pixOr(pix4, pix4, pix2);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix4, 1200, 50, "Horiz and vert edges", rp->display);
    pixDestroy(&pix2);
    pixDestroy(&pix4);

        /* Horizontal and vertical sobel edges (8 bpp) */
    pixMinOrMax(pix1, pix1, pix3, L_CHOOSE_MAX);
    pixInvert(pix1, pix1);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 3 */
    pixDisplayWithTitle(pix1, 0, 525, "8bpp Horiz and vert edges", rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}

