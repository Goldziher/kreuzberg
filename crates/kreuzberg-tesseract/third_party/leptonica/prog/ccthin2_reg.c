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
 * ccthin2_reg.c
 *
 *   Tests:
 *   - The examples in pixThinConnectedBySet()
 *   - Use of thinning and thickening in stroke width normalization
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i;
BOX          *box;
PIX          *pixs, *pix1, *pix2;
PIXA         *pixa1, *pixa2, *pixa3, *pixa4, *pixa5;
PIXAA        *paa;
L_REGPARAMS  *rp;
SELA         *sela;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "ccthin_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Clip to foreground to see if there are any boundary
         * artifacts from thinning and thickening.  (There are not.) */
    pix1 = pixRead("feyn.tif");
    box = boxCreate(683, 799, 970, 479);
    pix2 = pixClipRectangle(pix1, box, NULL);
    pixClipToForeground(pix2, &pixs, NULL);

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxDestroy(&box);

    pixa1 = pixaCreate(0);

    sela = selaMakeThinSets(1, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(2, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(3, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(4, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(5, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(6, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 5 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(7, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 6 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(8, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 7 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(9, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_FG, sela, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 8 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(10, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_BG, sela, 5);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 9 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

    sela = selaMakeThinSets(11, 0);
    pix1 = pixThinConnectedBySet(pixs, L_THIN_BG, sela, 5);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10 */
    pixaAddPix(pixa1, pix1, L_INSERT);
    selaDestroy(&sela);

        /* Display the thinning results */
    pix1 = pixaDisplayTiledAndScaled(pixa1, 8, 500, 1, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 11 */
    if (rp->display) {
        lept_mkdir("lept/thin");
        pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
        lept_stderr("Writing to: /tmp/lept/thin/ccthin2-1.pdf");
        pixaConvertToPdf(pixa1, 0, 1.0, 0, 0, "Thin 2 Results",
                         "/tmp/lept/thin/ccthin2-1.pdf");
    }
    pixDestroy(&pix1);
    pixDestroy(&pixs);
    pixaDestroy(&pixa1);

       /* Show thinning using width normalization */
    paa = pixaaCreate(3);
    pixa1 = l_bootnum_gen3();
    pixa2 = pixaScaleToSize(pixa1, 0, 36);
    pixaaAddPixa(paa, pixa2, L_INSERT);
    pixa3 = pixaScaleToSizeRel(pixa2, -4, 0);
    pixaaAddPixa(paa, pixa3, L_INSERT);
    pixa3 = pixaScaleToSizeRel(pixa2, 4, 0);
    pixaaAddPixa(paa, pixa3, L_INSERT);
    pixa5 = pixaCreate(6);
    for (i = 0; i < 3; i++) {
        pixa3 = pixaaGetPixa(paa, i, L_CLONE);
        pix1 = pixaDisplayTiledInColumns(pixa3, 15, 1.0, 10, 1);
        regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 12, 14, 16 */
        pixaAddPix(pixa5, pix1, L_INSERT);
        pixa4 = pixaSetStrokeWidth(pixa3, 5, 1, 8);
/*        pixa4 = pixaSetStrokeWidth(pixa3, 1, 1, 8); */
        pix1 = pixaDisplayTiledInColumns(pixa4, 15, 1.0, 10, 1);
        regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 13, 15, 17 */
        pixaAddPix(pixa5, pix1, L_INSERT);
        pixaDestroy(&pixa3);
        pixaDestroy(&pixa4);
    }
    pix1 = pixaDisplayTiledInColumns(pixa5, 2, 1.0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 18 */
    if (rp->display) {
        pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
        lept_stderr("Writing to: /tmp/lept/thin/ccthin2-2.pdf");
        pixaConvertToPdf(pixa5, 0, 1.0, 0, 0, "Thin strokes",
                         "/tmp/lept/thin/ccthin2-2.pdf");
    }
    pixaaDestroy(&paa);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa5);
    pixDestroy(&pix1);

    return regTestCleanup(rp);
}


