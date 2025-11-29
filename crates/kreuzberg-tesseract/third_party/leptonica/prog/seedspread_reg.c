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
 *   seedspread_reg.c
 *
 *   Tests the seedspreading (voronoi finding & filling) function
 *   for both 4 and 8 connectivity.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  scalefact = 1.0;

int main(int    argc,
         char **argv)
{
l_int32       i, j, x, y, val;
PIX          *pixsq, *pixs, *pixc, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixsq = pixCreate(3, 3, 32);
    pixSetAllArbitrary(pixsq, 0x00ff0000);
    pixa = pixaCreate(6);

        /* Moderately dense */
    pixs = pixCreate(300, 300, 8);
    for (i = 0; i < 100; i++) {
        x = (153 * i * i * i + 59) % 299;
        y = (117 * i * i * i + 241) % 299;
        val = (97 * i + 74) % 256;
        pixSetPixel(pixs, x, y, val);
    }

    pixd = pixSeedspread(pixs, 4);  /* 4-cc */
    pixc = pixConvertTo32(pixd);
    for (i = 0; i < 100; i++) {
        x = (153 * i * i * i + 59) % 299;
        y = (117 * i * i * i + 241) % 299;
        pixRasterop(pixc, x - 1, y - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    }
    pixaAddPix(pixa, pixc, L_INSERT);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pixc, 100, 100, "4-cc", rp->display);
    pixDestroy(&pixd);

    pixd = pixSeedspread(pixs, 8);  /* 8-cc */
    pixc = pixConvertTo32(pixd);
    for (i = 0; i < 100; i++) {
        x = (153 * i * i * i + 59) % 299;
        y = (117 * i * i * i + 241) % 299;
        pixRasterop(pixc, x - 1, y - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    }
    pixaAddPix(pixa, pixc, L_INSERT);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pixc, 410, 100, "8-cc", rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixs);

        /* Regular lattice */
    pixs = pixCreate(200, 200, 8);
    for (i = 5; i <= 195; i += 10) {
        for (j = 5; j <= 195; j += 10) {
            pixSetPixel(pixs, i, j, (7 * i + 17 * j) % 255);
        }
    }
    pixd = pixSeedspread(pixs, 4);  /* 4-cc */
    pixc = pixConvertTo32(pixd);
    for (i = 5; i <= 195; i += 10) {
        for (j = 5; j <= 195; j += 10) {
            pixRasterop(pixc, j - 1, i - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
        }
    }
    pixaAddPix(pixa, pixc, L_INSERT);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pixc, 100, 430, "4-cc", rp->display);
    pixDestroy(&pixd);

    pixd = pixSeedspread(pixs, 8);  /* 8-cc */
    pixc = pixConvertTo32(pixd);
    for (i = 5; i <= 195; i += 10) {
        for (j = 5; j <= 195; j += 10) {
            pixRasterop(pixc, j - 1, i - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
        }
    }
    pixaAddPix(pixa, pixc, L_INSERT);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pixc, 310, 430, "8-cc", rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixs);

        /* Very sparse points */
    pixs = pixCreate(200, 200, 8);
    pixSetPixel(pixs, 60, 20, 90);
    pixSetPixel(pixs, 160, 40, 130);
    pixSetPixel(pixs, 80, 80, 205);
    pixSetPixel(pixs, 40, 160, 115);
    pixd = pixSeedspread(pixs, 4);  /* 4-cc */
    pixc = pixConvertTo32(pixd);
    pixRasterop(pixc, 60 - 1, 20 - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    pixRasterop(pixc, 160 - 1, 40 - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    pixRasterop(pixc, 80 - 1, 80 - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    pixRasterop(pixc, 40 - 1, 160 - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    pixaAddPix(pixa, pixc, L_INSERT);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pixc, 100, 600, "4-cc", rp->display);
    pixDestroy(&pixd);

    pixd = pixSeedspread(pixs, 8);  /* 8-cc */
    pixc = pixConvertTo32(pixd);
    pixRasterop(pixc, 60 - 1, 20 - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    pixRasterop(pixc, 160 - 1, 40 - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    pixRasterop(pixc, 80 - 1, 80 - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    pixRasterop(pixc, 40 - 1, 160 - 1, 3, 3, PIX_SRC, pixsq, 0, 0);
    pixaAddPix(pixa, pixc, L_INSERT);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pixc, 310, 660, "8-cc", rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixs);
    pixDestroy(&pixsq);

    pixd = pixaDisplayTiledInColumns(pixa, 2, scalefact, 20, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pixd, 720, 100, "Final", rp->display);

    pixaDestroy(&pixa);
    pixDestroy(&pixd);
    return regTestCleanup(rp);
}
