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
 * graymorph1_reg.c
 *
 *      (1) Tests the interpreter for grayscale morphology, as
 *          given in morphseq.c
 *
 *      (2) Tests composite operations: tophat and hdome
 *
 *      (3) Tests duality for grayscale erode/dilate, open/close,
 *          and black/white tophat
 *
 *      (4) Demonstrates closing plus white tophat.  Note that this
 *          combination of operations can be quite useful.
 *
 *      (5) Demonstrates a method of doing contrast enhancement
 *          by taking 3 * pixs and subtracting from this the
 *          closing and opening of pixs.  Do this both with the
 *          basic pix accumulation functions and with the cleaner
 *          Pixacc wrapper.   Verify the results are equivalent.
 *
 *      (6) Playing around: extract the feynman diagrams from
 *          the stamp, using the tophat.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define     WSIZE              7
#define     HSIZE              7

int main(int    argc,
         char **argv)
{
char          seq[512];
l_int32       w, h;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5;
PIXA         *pixa;
PIXACC       *pacc;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("aneurisms8.jpg");
    pixa = pixaCreate(0);

    /* =========================================================== */

    /* -------- Test gray morph, including interpreter ------------ */
    pix1 = pixDilateGray(pixs, WSIZE, HSIZE);
    snprintf(seq, sizeof(seq), "D%d.%d", WSIZE, HSIZE);
    pix2 = pixGrayMorphSequence(pixs, seq, 0, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    regTestComparePix(rp, pix1, pix2);  /* 1 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix1 = pixErodeGray(pixs, WSIZE, HSIZE);
    snprintf(seq, sizeof(seq), "E%d.%d", WSIZE, HSIZE);
    pix2 = pixGrayMorphSequence(pixs, seq, 0, 100);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    regTestComparePix(rp, pix1, pix2);  /* 3 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix1 = pixOpenGray(pixs, WSIZE, HSIZE);
    snprintf(seq, sizeof(seq), "O%d.%d", WSIZE, HSIZE);
    pix2 = pixGrayMorphSequence(pixs, seq, 0, 200);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    regTestComparePix(rp, pix1, pix2);  /* 5 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix1 = pixCloseGray(pixs, WSIZE, HSIZE);
    snprintf(seq, sizeof(seq), "C%d.%d", WSIZE, HSIZE);
    pix2 = pixGrayMorphSequence(pixs, seq, 0, 300);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 6 */
    regTestComparePix(rp, pix1, pix2);  /* 7 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix1 = pixTophat(pixs, WSIZE, HSIZE, L_TOPHAT_WHITE);
    snprintf(seq, sizeof(seq), "Tw%d.%d", WSIZE, HSIZE);
    pix2 = pixGrayMorphSequence(pixs, seq, 0, 400);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 8 */
    regTestComparePix(rp, pix1, pix2);  /* 9 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix1 = pixTophat(pixs, WSIZE, HSIZE, L_TOPHAT_BLACK);
    snprintf(seq, sizeof(seq), "Tb%d.%d", WSIZE, HSIZE);
    pix2 = pixGrayMorphSequence(pixs, seq, 0, 500);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10 */
    regTestComparePix(rp, pix1, pix2);  /* 11 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    /* ------------- Test erode/dilate duality -------------- */
    pix1 = pixDilateGray(pixs, WSIZE, HSIZE);
    pix2 = pixInvert(NULL, pixs);
    pix3 = pixErodeGray(pix2, WSIZE, HSIZE);
    pixInvert(pix3, pix3);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 12 */
    regTestComparePix(rp, pix1, pix3);  /* 13 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    /* ------------- Test open/close duality -------------- */
    pix1 = pixOpenGray(pixs, WSIZE, HSIZE);
    pix2 = pixInvert(NULL, pixs);
    pix3 = pixCloseGray(pix2, WSIZE, HSIZE);
    pixInvert(pix3, pix3);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 14 */
    regTestComparePix(rp, pix1, pix3);  /* 15 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    /* ------------- Test tophat duality -------------- */
    pix1 = pixTophat(pixs, WSIZE, HSIZE, L_TOPHAT_WHITE);
    pix2 = pixInvert(NULL, pixs);
    pix3 = pixTophat(pix2, WSIZE, HSIZE, L_TOPHAT_BLACK);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 16 */
    regTestComparePix(rp, pix1, pix3);  /* 17 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    pix1 = pixGrayMorphSequence(pixs, "Tw9.5", 0, 100);
    pix2 = pixInvert(NULL, pixs);
    pix3 = pixGrayMorphSequence(pix2, "Tb9.5", 0, 300);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 18 */
    regTestComparePix(rp, pix1, pix3);  /* 19 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);


    /* ------------- Test opening/closing for large sels -------------- */
    pix1 = pixGrayMorphSequence(pixs,
            "C9.9 + C19.19 + C29.29 + C39.39 + C49.49", 0, 100);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 20 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixGrayMorphSequence(pixs,
            "O9.9 + O19.19 + O29.29 + O39.39 + O49.49", 0, 400);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 21 */
    pixaAddPix(pixa, pix1, L_INSERT);

    pix1 = pixaDisplayTiledInColumns(pixa, 4, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 22 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);

    /* =========================================================== */

    pixa = pixaCreate(0);
    /* ---------- Closing plus white tophat result ------------ *
     *            Parameters: wsize, hsize = 9, 29             *
     * ---------------------------------------------------------*/
    pix1 = pixCloseGray(pixs, 9, 9);
    pix2 = pixTophat(pix1, 9, 9, L_TOPHAT_WHITE);
    pix3 = pixGrayMorphSequence(pixs, "C9.9 + TW9.9", 0, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 23 */
    regTestComparePix(rp, pix2, pix3);  /* 24 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixMaxDynamicRange(pix2, L_LINEAR_SCALE);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 25 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    pix1 = pixCloseGray(pixs, 29, 29);
    pix2 = pixTophat(pix1, 29, 29, L_TOPHAT_WHITE);
    pix3 = pixGrayMorphSequence(pixs, "C29.29 + Tw29.29", 0, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 26 */
    regTestComparePix(rp, pix2, pix3);  /* 27 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixMaxDynamicRange(pix2, L_LINEAR_SCALE);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 28 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    /* --------- hdome with parameter height = 100 ------------*/
    pix1 = pixHDome(pixs, 100, 4);
    pix2 = pixMaxDynamicRange(pix1, L_LINEAR_SCALE);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 29 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 30 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);

    /* ----- Contrast enhancement with morph parameters 9, 9 -------*/
    pixGetDimensions(pixs, &w, &h, NULL);
    pix1 = pixInitAccumulate(w, h, 0x8000);
    pixAccumulate(pix1, pixs, L_ARITH_ADD);
    pixMultConstAccumulate(pix1, 3., 0x8000);
    pix2 = pixOpenGray(pixs, 9, 9);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 31 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixAccumulate(pix1, pix2, L_ARITH_SUBTRACT);

    pix2 = pixCloseGray(pixs, 9, 9);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 32 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixAccumulate(pix1, pix2, L_ARITH_SUBTRACT);
    pix2 = pixFinalAccumulate(pix1, 0x8000, 8);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 33 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pix1);

        /* Do the same thing with the Pixacc */
    pacc = pixaccCreate(w, h, 1);
    pixaccAdd(pacc, pixs);
    pixaccMultConst(pacc, 3.);
    pix1 = pixOpenGray(pixs, 9, 9);
    pixaccSubtract(pacc, pix1);
    pixDestroy(&pix1);
    pix1 = pixCloseGray(pixs, 9, 9);
    pixaccSubtract(pacc, pix1);
    pixDestroy(&pix1);
    pix1 = pixaccFinal(pacc, 8);
    pixaccDestroy(&pacc);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 34 */
    pixaAddPix(pixa, pix1, L_INSERT);
    regTestComparePix(rp, pix1, pix2);  /* 35 */

    pix1 = pixaDisplayTiledInColumns(pixa, 4, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 36 */
    pixDisplayWithTitle(pix1, 1100, 0, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);
    pixDestroy(&pixs);

    /* =========================================================== */

    pixa = pixaCreate(0);

    /* ----  Tophat result on feynman stamp, to extract diagrams ----- */
    pixs = pixRead("feynman-stamp.jpg");
    pixGetDimensions(pixs, &w, &h, NULL);

        /* Make output image to hold five intermediate images */
    pix1 = pixCreate(5 * w + 18, h + 6, 32);  /* composite output image */
    pixSetAllArbitrary(pix1, 0x0000ff00);  /* set to blue */

        /* Paste in the input image */
    pix2 = pixRemoveColormap(pixs, REMOVE_CMAP_TO_FULL_COLOR);
    pixRasterop(pix1, 3, 3, w, h, PIX_SRC, pix2, 0, 0);  /* 1st one */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 37 */
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Paste in the grayscale version */
    cmap = pixGetColormap(pixs);
    if (cmap)
        pix2 = pixRemoveColormap(pixs, REMOVE_CMAP_TO_GRAYSCALE);
    else
        pix2 = pixConvertRGBToGray(pixs, 0.33, 0.34, 0.33);
    pix3 = pixConvertTo32(pix2);  /* 8 --> 32 bpp */
    pixRasterop(pix1, w + 6, 3, w, h, PIX_SRC, pix3, 0, 0);  /* 2nd one */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 38 */
    pixaAddPix(pixa, pix3, L_INSERT);

         /* Paste in a log dynamic range scaled version of the white tophat */
    pix3 = pixTophat(pix2, 3, 3, L_TOPHAT_WHITE);
    pix4 = pixMaxDynamicRange(pix3, L_LOG_SCALE);
    pix5 = pixConvertTo32(pix4);
    pixRasterop(pix1, 2 * w + 9, 3, w, h, PIX_SRC, pix5, 0, 0);  /* 3rd */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 39 */
    pixaAddPix(pixa, pix5, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix4);

        /* Stretch the range and threshold to binary; paste it in */
    pix2 = pixGammaTRC(NULL, pix3, 1.0, 0, 80);
    pix4 = pixThresholdToBinary(pix2, 70);
    pix5 = pixConvertTo32(pix4);
    pixRasterop(pix1, 3 * w + 12, 3, w, h, PIX_SRC, pix5, 0, 0);  /* 4th */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 40 */
    pixaAddPix(pixa, pix5, L_INSERT);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Invert; this is the final result */
    pixInvert(pix4, pix4);
    pix5 = pixConvertTo32(pix4);
    pixRasterop(pix1, 4 * w + 15, 3, w, h, PIX_SRC, pix5, 0, 0);  /* 5th */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 41 */
    pixaAddPix(pixa, pix5, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix4);

    pix1 = pixaDisplayTiledInRows(pixa, 32, 1700, 1.0, 0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 42 */
    pixDisplayWithTitle(pix1, 0, 800, NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pix1);
    pixDestroy(&pixs);

    return regTestCleanup(rp);
}
