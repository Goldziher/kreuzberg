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
 *  newspaper_seg.c
 *
 *  Segmenting newspaper articles using morphology.
 *
 *  Most of the work is done at 4x reduction (approx. 75 ppi),
 *  which makes it very fast.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       w, h;
BOXA         *boxa;
PIX          *pixs, *pixt, *pix1, *pix2, *pix3, *pix4, *pix5;
PIX          *pix6, *pix7, *pix8, *pix9, *pix10, *pix11;
PIXA         *pixa1, *pixa2;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("scots-frag.tif");
    pixa1 = pixaCreate(12);

    pixt = pixScaleToGray4(pixs);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_JFIF_JPEG);  /* 0 */

        /* Rank reduce 2x */
    pix1 = pixReduceRankBinary2(pixs, 2, NULL);
    pixt = pixScale(pix1, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);

        /* Open out the vertical lines */
    pix2 = pixMorphSequence(pix1, "o1.50", 0);
    pixt = pixScale(pix2, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_TIFF_G4);  /* 1 */
    pixDisplayWithTitle(pixt, 0, 0, "open vertical lines", rp->display);

        /* Seedfill back to get those lines in their entirety */
    pix3 = pixSeedfillBinary(NULL, pix2, pix1, 8);
    pixt = pixScale(pix3, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_TIFF_G4);  /* 2 */
    pixDisplayWithTitle(pixt, 300, 0, "seedfill vertical", rp->display);

        /* Remove the vertical lines (and some of the images) */
    pixXor(pix2, pix1, pix3);
    pixt = pixScale(pix2, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_TIFF_G4);  /* 3 */
    pixDisplayWithTitle(pixt, 600, 0, "remove vertical lines", rp->display);

        /* Open out the horizontal lines */
    pix4 = pixMorphSequence(pix2, "o50.1", 0);
    pixt = pixScale(pix4, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);

        /* Seedfill back to get those lines in their entirety */
    pix5 = pixSeedfillBinary(NULL, pix4, pix2, 8);
    pixt = pixScale(pix5, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_TIFF_G4);  /* 4 */
    pixDisplayWithTitle(pixt, 900, 0, "seedfill horizontal", rp->display);

        /* Remove the horizontal lines */
    pixXor(pix4, pix2, pix5);
    pixt = pixScale(pix4, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_TIFF_G4);  /* 5 */
    pixDisplayWithTitle(pixt, 1200, 0, "remove horiz lines", rp->display);

        /* Invert and identify vertical gutters between text columns */
    pix6 = pixReduceRankBinaryCascade(pix4, 1, 1, 0, 0);
    pixInvert(pix6, pix6);
    pixt = pixScale(pix6, 2.0, 2.0);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_TIFF_G4);  /* 6 */
    pixDisplayWithTitle(pixt, 1500, 0, NULL, rp->display);
    pix7 = pixMorphSequence(pix6, "o1.50", 0);
    pixt = pixScale(pix7, 2.0, 2.0);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_TIFF_G4);  /* 7 */
    pixDisplayWithTitle(pixt, 0, 300, "vertical gutters", rp->display);
    pix8 = pixExpandBinaryPower2(pix7, 4);  /* gutter mask */
    regTestWritePixAndCheck(rp, pix8, IFF_TIFF_G4);  /* 8 */

        /* Solidify text blocks */
    pix9 = pixMorphSequence(pix4, "c50.1 + c1.10", 0);
    pixSubtract(pix9, pix9, pix8);  /* preserve gutter */
    pix10 = pixMorphSequence(pix9, "d3.3", 0);
    pixt = pixScale(pix10, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_TIFF_G4);  /* 9 */
    pixDisplayWithTitle(pixt, 300, 300, "solidify text", rp->display);

        /* Show stuff under this mask */
    pixGetDimensions(pix10, &w, &h, NULL);
    boxa = pixConnComp(pix10, &pixa2, 8);
    pix11 = pixaDisplayRandomCmap(pixa2, w, h);
    pixPaintThroughMask(pix11, pix4, 0, 0, 0);
    pixt = pixScale(pix11, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 10 */
    pixDisplayWithTitle(pixt, 600, 300, "stuff under mask1", rp->display);
    boxaDestroy(&boxa);
    pixaDestroy(&pixa2);

        /* Paint the background white */
    cmap = pixGetColormap(pix11);
    pixcmapResetColor(cmap, 0, 255, 255, 255);
    regTestWritePixAndCheck(rp, pix11, IFF_PNG);  /* 11 */
    pixt = pixScale(pix11, 0.5, 0.5);
    pixaAddPix(pixa1, pixt, L_INSERT);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pixt, 900, 300, "stuff under mask2", rp->display);
    pixaConvertToPdf(pixa1, 75, 1.0, 0, 0, "Segmentation: newspaper_reg",
                     "/tmp/lept/regout/newspaper.pdf");
    L_INFO("Output pdf: /tmp/lept/regout/newspaper.pdf\n", rp->testname);

    pixaDestroy(&pixa1);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixDestroy(&pix8);
    pixDestroy(&pix9);
    pixDestroy(&pix10);
    pixDestroy(&pix11);
    return regTestCleanup(rp);
}

