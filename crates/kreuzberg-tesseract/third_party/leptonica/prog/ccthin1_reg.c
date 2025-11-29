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
 * ccthin1_reg.c
 *
 *   Tests the "best" cc-preserving thinning functions.
 *   Displays all the strong cc-preserving 3x3 Sels.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
BOX          *box;
PIX          *pix1, *pix2;
PIXA         *pixa;
SEL          *sel, *sel1, *sel2, *sel3;
SELA         *sela, *sela4, *sela8, *sela48;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(0);

        /* Generate and display all of the 4-cc sels */
    sela4 = sela4ccThin(NULL);
    pix1 = selaDisplayInPix(sela4, 35, 3, 15, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix1, 400, 0, NULL, rp->display);
    pixaAddPix(pixa, pix1, L_INSERT);
    selaDestroy(&sela4);

        /* Generate and display all of the 8-cc sels */
    sela8 = sela8ccThin(NULL);
    pix1 = selaDisplayInPix(sela8, 35, 3, 15, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix1, 850, 0, NULL, rp->display);
    pixaAddPix(pixa, pix1, L_INSERT);
    selaDestroy(&sela8);

        /* Generate and display all of the 4 and 8-cc preserving sels */
    sela48 = sela4and8ccThin(NULL);
    pix1 = selaDisplayInPix(sela48, 35, 3, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 1300, 0, NULL, rp->display);
    pixaAddPix(pixa, pix1, L_INSERT);
    selaDestroy(&sela48);

        /* Generate and display three of the 4-cc sels and their rotations */
    sela = sela4ccThin(NULL);
    sela4 = selaCreate(0);
    selaFindSelByName(sela, "sel_4_1", NULL, &sel);
    sel1 = selRotateOrth(sel, 1);
    sel2 = selRotateOrth(sel, 2);
    sel3 = selRotateOrth(sel, 3);
    selaAddSel(sela4, sel, NULL, L_COPY);
    selaAddSel(sela4, sel1, "sel_4_1_90", L_INSERT);
    selaAddSel(sela4, sel2, "sel_4_1_180", L_INSERT);
    selaAddSel(sela4, sel3, "sel_4_1_270", L_INSERT);
    selaFindSelByName(sela, "sel_4_2", NULL, &sel);
    sel1 = selRotateOrth(sel, 1);
    sel2 = selRotateOrth(sel, 2);
    sel3 = selRotateOrth(sel, 3);
    selaAddSel(sela4, sel, NULL, L_COPY);
    selaAddSel(sela4, sel1, "sel_4_2_90", L_INSERT);
    selaAddSel(sela4, sel2, "sel_4_2_180", L_INSERT);
    selaAddSel(sela4, sel3, "sel_4_2_270", L_INSERT);
    selaFindSelByName(sela, "sel_4_3", NULL, &sel);
    sel1 = selRotateOrth(sel, 1);
    sel2 = selRotateOrth(sel, 2);
    sel3 = selRotateOrth(sel, 3);
    selaAddSel(sela4, sel, NULL, L_COPY);
    selaAddSel(sela4, sel1, "sel_4_3_90", L_INSERT);
    selaAddSel(sela4, sel2, "sel_4_3_180", L_INSERT);
    selaAddSel(sela4, sel3, "sel_4_3_270", L_INSERT);
    pix1 = selaDisplayInPix(sela4, 35, 3, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix1, 400, 500, NULL, rp->display);
    pixaAddPix(pixa, pix1, L_INSERT);
    selaDestroy(&sela);
    selaDestroy(&sela4);

        /* Generate and display four of the 8-cc sels and their rotations */
    sela = sela8ccThin(NULL);
    sela8 = selaCreate(0);
    selaFindSelByName(sela, "sel_8_2", NULL, &sel);
    sel1 = selRotateOrth(sel, 1);
    sel2 = selRotateOrth(sel, 2);
    sel3 = selRotateOrth(sel, 3);
    selaAddSel(sela8, sel, NULL, L_COPY);
    selaAddSel(sela8, sel1, "sel_8_2_90", L_INSERT);
    selaAddSel(sela8, sel2, "sel_8_2_180", L_INSERT);
    selaAddSel(sela8, sel3, "sel_8_2_270", L_INSERT);
    selaFindSelByName(sela, "sel_8_3", NULL, &sel);
    sel1 = selRotateOrth(sel, 1);
    sel2 = selRotateOrth(sel, 2);
    sel3 = selRotateOrth(sel, 3);
    selaAddSel(sela8, sel, NULL, L_COPY);
    selaAddSel(sela8, sel1, "sel_8_3_90", L_INSERT);
    selaAddSel(sela8, sel2, "sel_8_3_180", L_INSERT);
    selaAddSel(sela8, sel3, "sel_8_3_270", L_INSERT);
    selaFindSelByName(sela, "sel_8_5", NULL, &sel);
    sel1 = selRotateOrth(sel, 1);
    sel2 = selRotateOrth(sel, 2);
    sel3 = selRotateOrth(sel, 3);
    selaAddSel(sela8, sel, NULL, L_COPY);
    selaAddSel(sela8, sel1, "sel_8_5_90", L_INSERT);
    selaAddSel(sela8, sel2, "sel_8_5_180", L_INSERT);
    selaAddSel(sela8, sel3, "sel_8_5_270", L_INSERT);
    selaFindSelByName(sela, "sel_8_6", NULL, &sel);
    sel1 = selRotateOrth(sel, 1);
    sel2 = selRotateOrth(sel, 2);
    sel3 = selRotateOrth(sel, 3);
    selaAddSel(sela8, sel, NULL, L_COPY);
    selaAddSel(sela8, sel1, "sel_8_6_90", L_INSERT);
    selaAddSel(sela8, sel2, "sel_8_6_180", L_INSERT);
    selaAddSel(sela8, sel3, "sel_8_6_270", L_INSERT);
    pix1 = selaDisplayInPix(sela8, 35, 3, 15, 4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix1, 1000, 500, NULL, rp->display);
    pixaAddPix(pixa, pix1, L_INSERT);
    selaDestroy(&sela);
    selaDestroy(&sela8);

        /* Optional display */
    if (rp->display) {
        lept_mkdir("/lept/thin");
        lept_stderr("Writing to: /tmp/lept/thin/ccthin1-1.pdf");
        pixaConvertToPdf(pixa, 0, 1.0, 0, 0, "Thin 1 Sels",
                         "/tmp/lept/thin/ccthin1-1.pdf");
    }
    pixaDestroy(&pixa);

    pixa = pixaCreate(0);

        /* Test the best 4 and 8 cc thinning */
    pix2 = pixRead("feyn.tif");
    box = boxCreate(683, 799, 970, 479);
    pix1 = pixClipRectangle(pix2, box, NULL);
    boxDestroy(&box);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix2 = pixThinConnected(pix1, L_THIN_FG, 4, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 6 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pix2 = pixThinConnected(pix1, L_THIN_BG, 4, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 7 */
    pixaAddPix(pixa, pix2, L_INSERT);

    pix2 = pixThinConnected(pix1, L_THIN_FG, 8, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 8 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pix2 = pixThinConnected(pix1, L_THIN_BG, 8, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 9 */
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Display tiled */
    pix1 = pixaDisplayTiledAndScaled(pixa, 8, 500, 1, 0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixDestroy(&pix1);
    if (rp->display) {
        lept_stderr("Writing to: /tmp/lept/thin/ccthin1-2.pdf");
        pixaConvertToPdf(pixa, 0, 1.0, 0, 0, "Thin 1 Results",
                         "/tmp/lept/thin/ccthin1-2.pdf");
    }
    pixaDestroy(&pixa);

    return regTestCleanup(rp);
}
