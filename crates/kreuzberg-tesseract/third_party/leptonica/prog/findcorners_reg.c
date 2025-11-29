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
 * findcorners_reg.c
 *
 *   This reg test demonstrates extraction of deskewed objects (tickets
 *   in this case) using morphological operations to identify the
 *   barcodes on each object.  The objects are separately deskewed,
 *   the barcodes are again located, and the objects are extracted.
 *
 *   We also show how to generate the HMT sela for detecting corners,
 *   and how to use it (with pixUnionOfMorphOps()) to find all the
 *   corners.  The corner Sels were constructed to find significant
 *   corners in the presence of the type of noise expected from
 *   scanned images.  The located corners are displayed by xor-ing
 *   a pattern (sel_cross) on each one.
 *
 *   When this function is called with the display argument
 *        findcorners_reg display
 *   we display some results and additionally generate the following pdfs:
 *        /tmp/lept/regout/seq_output_1.pdf  (morphological operations of
 *                                       first call to locate barcodes)
 *        /tmp/lept/regout/tickets.pdf  (deskewed result for the set of tickets)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static BOXA *LocateBarcodes(PIX *pixs, PIX **ppixd, l_int32 flag);
static SELA *GetCornerSela(L_REGPARAMS *rp);

static const char *sel_cross = "     xxx     "
                               "     xxx     "
                               "     xxx     "
                               "     xxx     "
                               "     xxx     "
                               "xxxxxxxxxxxxx"
                               "xxxxxxXxxxxxx"
                               "xxxxxxxxxxxxx"
                               "     xxx     "
                               "     xxx     "
                               "     xxx     "
                               "     xxx     "
                               "     xxx     ";

l_int32 main(int    argc,
             char **argv)
{
l_int32       i, n, flag;
l_float32     angle, conf, deg2rad;
BOX          *box1, *box2, *box3, *box4;
BOXA         *boxa, *boxa2;
PIX          *pixs, *pixd, *pix1, *pix2, *pix3;
PIXA         *pixa;
SEL          *sel;
SELA         *sela;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("tickets.tif");
    flag = (rp->display) ? -1 : 0;
    boxa = LocateBarcodes(pixs, &pixd, flag);
    regTestWritePixAndCheck(rp, pixd, IFF_TIFF_G4);  /* 0 */
    if (rp->display) boxaWriteStderr(boxa);
    n = boxaGetCount(boxa);
    deg2rad = 3.14159265 / 180.;
    pixa = pixaCreate(9);
    for (i = 0; i < n; i++) {
        box1 = boxaGetBox(boxa, i, L_CLONE);
            /* Use a larger adjustment to get entire skewed ticket */
        box2 = boxAdjustSides(NULL, box1, -266, 346, -1560, 182);
        pix1 = pixClipRectangle(pixs, box2, NULL);
            /* Deskew */
        pixFindSkew(pix1, &angle, &conf);
        pix2 = pixRotate(pix1, deg2rad * angle, L_ROTATE_SAMPLING,
                              L_BRING_IN_WHITE, 0, 0);
            /* Find the barcode again ... */
        boxa2 = LocateBarcodes(pix2, NULL, 0);
        box3 = boxaGetBox(boxa2, 0, L_CLONE);
            /* ... and adjust crop box exactly for ticket size */
        box4 = boxAdjustSides(NULL, box3, -141, 221, -1535, 157);
        pix3 = pixClipRectangle(pix2, box4, NULL);
        regTestWritePixAndCheck(rp, pix3, IFF_TIFF_G4);  /* 1 - 9 */
        if (rp->display)
            pixaAddPix(pixa, pix3, L_INSERT);
        else
            pixDestroy(&pix3);
        boxDestroy(&box1);
        boxDestroy(&box2);
        boxDestroy(&box3);
        boxDestroy(&box4);
        boxaDestroy(&boxa2);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }
    if (rp->display) {
        pixaConvertToPdf(pixa, 0, 1.0, 0, 0, "tickets",
                         "/tmp/lept/regout/tickets.pdf");
        L_INFO("Output pdf: /tmp/lept/regout/tickets.pdf\n", rp->testname);
    }
    pixaDestroy(&pixa);

        /* Downscale by 2x and locate corners */
    pix1 = pixScale(pixd, 0.5, 0.5);
    regTestWritePixAndCheck(rp, pix1, IFF_TIFF_G4);  /* 10 */
    pixDisplayWithTitle(pix1, 100, 200, NULL, rp->display);
        /* Find corners and blit a cross onto each (4 to each barcode) */
    sela = GetCornerSela(rp);
    pix2 = pixUnionOfMorphOps(pix1, sela, L_MORPH_HMT);
    sel = selCreateFromString(sel_cross, 13, 13, "sel_cross");
    pix3 = pixDilate(NULL, pix2, sel);
    pixXor(pix3, pix3, pix1);
    regTestWritePixAndCheck(rp, pix3, IFF_TIFF_G4);  /* 11 */
    pixDisplayWithTitle(pix3, 800, 200, NULL, rp->display);

    boxaDestroy(&boxa);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pixd);
    selDestroy(&sel);
    selaDestroy(&sela);
    return regTestCleanup(rp);
}


static BOXA *
LocateBarcodes(PIX     *pixs,
               PIX    **ppixd,
               l_int32  flag)
{
BOXA    *boxa1, *boxa2, *boxad;
PIX     *pix1, *pix2, *pix3;

    pix1 = pixScale(pixs, 0.5, 0.5);
    pix2 = pixMorphSequence(pix1, "o1.5 + c15.1 + o10.15 + c20.20", flag);
    boxa1 = pixConnComp(pix2, NULL, 8);
    boxa2 = boxaSelectBySize(boxa1, 300, 0, L_SELECT_WIDTH,
                                   L_SELECT_IF_GT, NULL);
    boxad = boxaTransform(boxa2, 0, 0, 2.0, 2.0);
    if (ppixd) {
        pix3 = pixSelectBySize(pix2, 300, 0, 8, L_SELECT_WIDTH,
                                 L_SELECT_IF_GT, NULL);
        *ppixd = pixScale(pix3, 2.0, 2.0);
        pixDestroy(&pix3);
    }

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    return boxad;
}


static SELA *
GetCornerSela(L_REGPARAMS  *rp)
{
PIX   *pix;
SEL   *sel;
SELA  *sela1, *sela2;

    sela1 = selaAddHitMiss(NULL);
    sela2 = selaCreate(4);
    selaFindSelByName(sela1, "sel_ulc", NULL, &sel);
    selaAddSel(sela2, sel, NULL, 1);
    selaFindSelByName(sela1, "sel_urc", NULL, &sel);
    selaAddSel(sela2, sel, NULL, 1);
    selaFindSelByName(sela1, "sel_llc", NULL, &sel);
    selaAddSel(sela2, sel, NULL, 1);
    selaFindSelByName(sela1, "sel_lrc", NULL, &sel);
    selaAddSel(sela2, sel, NULL, 1);
    selaDestroy(&sela1);
    if (rp->display) {
        pix = selaDisplayInPix(sela2, 21, 3, 10, 4);
        pixDisplay(pix, 0, 0);
        pixDestroy(&pix);
    }
    return sela2;
}
