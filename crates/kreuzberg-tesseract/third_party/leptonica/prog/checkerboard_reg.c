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
 * checkerboard_reg.c
 *
 *     This tests the function that locates corners where four checkerboard
 *     squares are joined.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void LocateCheckerboardCorners(L_REGPARAMS *rp,
                               const char *fname,
                               l_int32 nsels);

int main(int    argc,
         char **argv)
{
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    LocateCheckerboardCorners(rp, "checkerboard1.tif", 2);
    LocateCheckerboardCorners(rp, "checkerboard2.tif", 4);
    return regTestCleanup(rp);
}


void
LocateCheckerboardCorners(L_REGPARAMS  *rp,
                          const char   *fname,
                          l_int32       nsels)
{
l_int32  w, h, n, i;
PIX     *pix1, *pix2, *pix3, *pix4;
PIXA    *pixa1;
PTA     *pta1;

    pix1 = pixRead(fname);
    pixa1 = pixaCreate(0);
    pixFindCheckerboardCorners(pix1, 15, 3, nsels, &pix2, &pta1, pixa1);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);    /* 0, 3 */
    pix3 = pixaDisplayTiledInColumns(pixa1, 1, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 1, 4 */
    pixDisplayWithTitle(pix3, 100 * (nsels - 2), 100, NULL, rp->display);
    pixGetDimensions(pix1, &w, &h, NULL);
    pix4 = pixGenerateFromPta(pta1, w, h);
    pixDilateBrick(pix4, pix4, 5, 5);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 2, 5 */
    pixDestroy(&pix1);

    if (rp->display) {
        n = pixaGetCount(pixa1);
        for (i = 0; i < n; i++) {
            pix1 = pixaGetPix(pixa1, i, L_CLONE);
            pixDisplay(pix1, 350 + 200 * i, 300 * (nsels - 2));
            pixDestroy(&pix1);
        }
    }
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixaDestroy(&pixa1);
    ptaDestroy(&pta1);
}
