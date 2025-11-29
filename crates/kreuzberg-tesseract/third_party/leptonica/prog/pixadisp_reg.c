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
 *   pixadisp_reg.c
 *
 *     Regression test exercising various pixaDisplay*() functions.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

const char *files[7] = {"brev.06.75.jpg", "brev.10.75.jpg", "brev.14.75.jpg",
                        "brev.20.75.jpg", "brev.36.75.jpg", "brev.53.75.jpg",
                        "brev.56.75.jpg"};

int main(int    argc,
         char **argv)
{
l_int32       i, ws, hs, ncols;
char         *fname;
BOX          *box;
BOXA         *boxa;
PIX          *pixs, *pix32, *pix1, *pix2, *pix3, *pix4;
PIXA         *pixa, *pixa1, *pixa2, *pixa3;
SARRAY       *sa1, *sa2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(0);
    pix32 = pixRead("marge.jpg");
    pixs = pixRead("feyn.tif");
    box = boxCreate(683, 799, 970, 479);
    pix1 = pixClipRectangle(pixs, box, NULL);
    boxDestroy(&box);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Generate pixa2 from pixs and pixa3 from pix1 */
    boxa = pixConnComp(pixs, &pixa1, 8);
    pixa2 = pixaSelectBySize(pixa1, 60, 60, L_SELECT_IF_BOTH,
                             L_SELECT_IF_LTE, NULL);
    pixaDestroy(&pixa1);
    boxaDestroy(&boxa);
    boxa = pixConnComp(pix1, &pixa3, 8);
    boxaDestroy(&boxa);

        /* pixaDisplay() */
    pixGetDimensions(pixs, &ws, &hs, NULL);
    pix2 = pixaDisplay(pixa2, ws, hs);
    pixDisplayWithTitle(pix2, 0, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pixs);

        /* pixaDisplayRandomCmap() */
    pix2 = pixaDisplayRandomCmap(pixa2, ws, hs);  /* black bg */
    pixDisplayWithTitle(pix2, 200, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pix2, L_COPY);
    pixcmapResetColor(pixGetColormap(pix2), 0, 255, 255, 255);  /* white bg */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 3 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaDestroy(&pixa2);

        /* pixaDisplayOnLattice() */
    pix2 = pixaDisplayOnLattice(pixa3, 50, 50, &ncols, &boxa);
    pixDisplayWithTitle(pix2, 400, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 4 */
    pixaAddPix(pixa, pix2, L_INSERT);
    lept_stderr("Number of columns = %d; number of boxes: %d\n",
                ncols, boxaGetCount(boxa));
    boxaDestroy(&boxa);

        /* pixaDisplayUnsplit() */
    pixa1 = pixaSplitPix(pix32, 5, 7, 10, 0x0000ff00);
    pix2 = pixaDisplayUnsplit(pixa1, 5, 7, 10, 0x00ff0000);
    pixDisplayWithTitle(pix2, 600, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 5 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaDestroy(&pixa1);

        /* pixaDisplayTiled() */
    pix2 = pixaDisplayTiled(pixa3, 1000, 0, 10);
    pixDisplayWithTitle(pix2, 800, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 6 */
    pixaAddPix(pixa, pix2, L_INSERT);

        /* pixaDisplayTiledInRows() */
    pix2 = pixaDisplayTiledInRows(pixa3, 1, 1000, 1.0, 0, 10, 2);
    pixDisplayWithTitle(pix2, 1000, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 7 */
    pixaAddPix(pixa, pix2, L_INSERT);

        /* pixaDisplayTiledAndScaled() */
    pix2 = pixaDisplayTiledAndScaled(pixa3, 1, 25, 20, 0, 5, 0);
    pixDisplayWithTitle(pix2, 1200, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 8 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaDestroy(&pixa3);

    pixa1 = pixaCreate(10);
    pix2 = pixRankFilter(pix32, 8, 8, 0.5);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pix3 = pixScale(pix32, 0.5, 0.5);
    pix2 = pixRankFilter(pix3, 8, 8, 0.5);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixDestroy(&pix3);
    pix3 = pixScale(pix32, 0.25, 0.25);
    pix2 = pixRankFilter(pix3, 8, 8, 0.5);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixDestroy(&pix3);
    pix2 = pixaDisplayTiledAndScaled(pixa1, 32, 500, 3, 0, 25, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 9 */
    pixDisplayWithTitle(pix2, 1400, 100, NULL, rp->display);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaDestroy(&pixa1);
    pixDestroy(&pix32);

        /* pixaMakeFromTiledPix() and pixaDisplayOnLattice()  */
    pix1 = pixRead("sevens.tif");
    pixa1 = pixaMakeFromTiledPix(pix1, 20, 30, 0, 0, NULL);
    pix2 = pixaDisplayOnLattice(pixa1, 20, 30, NULL, NULL);
    regTestComparePix(rp, pix1, pix2);  /* 10 */
    pix3 = pixaDisplayOnLattice(pixa1, 20, 30, NULL, &boxa);
    pixa2 = pixaMakeFromTiledPix(pix3, 0, 0, 0, 0, boxa);
    pix4 = pixaDisplayOnLattice(pixa2, 20, 30, NULL, NULL);
    regTestComparePix(rp, pix2, pix4);  /* 11 */
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 12 */
    pixDisplayWithTitle(pix4, 1600, 100, NULL, rp->display);
    pixaAddPix(pixa, pixScale(pix4, 2.5, 2.5), L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    boxaDestroy(&boxa);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);

        /* pixaDisplayPairTiledInColumns */
    sa1 = sarrayCreate(7);
    for (i = 0; i < 7; i++)
        sarrayAddString(sa1, files[i], L_COPY);
    pixa1 = pixaCreate(7);
    pixa2 = pixaCreate(7);
    sa2 = sarrayCreate(7);
    for (i = 0; i < 7; i++) {
        fname = sarrayGetString(sa1, i, L_NOCOPY);
        pix1 = pixRead(fname);
        pix2 = pixConvertTo8(pix1, 0);
        pixaAddPix(pixa1, pix1, L_INSERT);
        pixaAddPix(pixa2, pix2, L_INSERT);
        sarrayAddString(sa2, fname, L_COPY);
    }
    pix1 = pixaDisplayPairTiledInColumns(pixa1, pixa2, 4, 0.5,
                                         15, 15, 2, 2, 6, 0, sa2);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 13 */
    pixaAddPix(pixa, pixScale(pix1, 2.0, 2.0), L_INSERT);
    pixDisplayWithTitle(pix1, 1800, 100, NULL, rp->display);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixDestroy(&pix1);
    sarrayDestroy(&sa1);
    sarrayDestroy(&sa2);

    if (rp->display) {
        lept_mkdir("lept/padisp");
        lept_stderr("Writing to: /tmp/lept/padisp/pixadisp.pdf\n");
        pixaConvertToPdf(pixa, 0, 1.0, L_FLATE_ENCODE, 0, "pixadisp-test",
                         "/tmp/lept/padisp/pixadisp.pdf");
        lept_stderr("Writing to: /tmp/lept/padisp/pixadisp.jpg\n");
        pix1 = pixaDisplayTiledInColumns(pixa, 2, 0.5, 30, 2);
        pixWrite("/tmp/lept/padisp/pixadisp.jpg", pix1, IFF_JFIF_JPEG);
        pixDisplay(pix1, 100, 100);
        pixDestroy(&pix1);
    }

    pixaDestroy(&pixa);
    return regTestCleanup(rp);
}
