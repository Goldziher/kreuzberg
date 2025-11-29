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
 *  binmorph3_reg.c
 *
 *     This is a regression test of dwa functions.  It should always
 *     be run if changes are made to the low-level morphology code.
 *
 *  Some things to note:
 *
 *  (1) This compares results for these operations:
 *         - rasterop brick (non-separable, separable)
 *         - dwa brick (separable), as implemented in morphdwa.c
 *         - dwa brick separable, but using lower-level non-separable
 *           autogen'd code.
 *
 *  (2) See in-line comments for ordinary closing and safe closing.
 *      The complication is due to the fact that the results differ
 *      for symmetric and asymmetric b.c., so we must do some
 *      fine adjustments of the border when implementing using
 *      the lower-level code directly.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

l_int32 TestAll(L_REGPARAMS *rp, PIX *pixs, l_int32 symmetric);

int main(int    argc,
         char **argv)
{
PIX          *pixs;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn-fract.tif");

    TestAll(rp, pixs, FALSE);
    TestAll(rp, pixs, TRUE);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}

l_int32
TestAll(L_REGPARAMS  *rp,
        PIX          *pixs,
        l_int32       symmetric)
{
char        *selnameh, *selnamev;
l_int32      ok, same, w, h, i, bordercolor, extraborder;
l_int32      width[3] = {21, 1, 21};
l_int32      height[3] = {1, 7, 7};
PIX         *pixref, *pix0, *pix1, *pix2, *pix3, *pix4;
SEL         *sel;
SELA        *sela;


    if (symmetric) {
        resetMorphBoundaryCondition(SYMMETRIC_MORPH_BC);
        lept_stderr("Testing with symmetric boundary conditions\n"
                    "==========================================\n");
    } else {
        resetMorphBoundaryCondition(ASYMMETRIC_MORPH_BC);
        lept_stderr("Testing with asymmetric boundary conditions\n"
                    "==========================================\n");
    }

        /* Note that with the choice of width and height, there is
         * no situation where they are both 1 (a no-op).  */
    for (i = 0; i < 3; i++) {
        w = width[i];
        h = height[i];
        sel = selCreateBrick(h, w, h / 2, w / 2, SEL_HIT);
        selnameh = NULL;
        selnamev = NULL;


            /* Get the selnames for horiz and vert */
        sela = selaAddBasic(NULL);
        if (w > 1) {
            if ((selnameh = selaGetBrickName(sela, w, 1)) == NULL) {
                selaDestroy(&sela);
                selDestroy(&sel);
                return ERROR_INT("dwa hor sel not defined", rp->testname, 1);
            }
        }
        if (h > 1) {
            if ((selnamev = selaGetBrickName(sela, 1, h)) == NULL) {
                selaDestroy(&sela);
                selDestroy(&sel);
                return ERROR_INT("dwa vert sel not defined", rp->testname, 1);
            }
        }
        lept_stderr("w = %d, h = %d, selh = %s, selv = %s\n",
                w, h, selnameh, selnamev);
        ok = TRUE;
        selaDestroy(&sela);

            /* ----------------- Dilation ----------------- */
        lept_stderr("Testing dilation\n");
        pixref = pixDilate(NULL, pixs, sel);
        pix1 = pixDilateBrickDwa(NULL, pixs, w, h);
        pixEqual(pixref, pix1, &same);
        if (!same) {
            lept_stderr("pixref != pix1 !\n"); ok = FALSE;
        }
        pixDestroy(&pix1);

        if (w > 1)
            pix1 = pixMorphDwa_1(NULL, pixs, L_MORPH_DILATE, selnameh);
        else
            pix1 = pixClone(pixs);
        if (h > 1)
            pix2 = pixMorphDwa_1(NULL, pix1, L_MORPH_DILATE, selnamev);
        else
            pix2 = pixClone(pix1);
        pixEqual(pixref, pix2, &same);
        if (!same) {
            lept_stderr("pixref != pix2 !\n"); ok = FALSE;
        }
        pixDestroy(&pix1);
        pixDestroy(&pix2);

        pix1 = pixAddBorder(pixs, 32, 0);
        if (w > 1)
            pix2 = pixFMorphopGen_1(NULL, pix1, L_MORPH_DILATE, selnameh);
        else
            pix2 = pixClone(pix1);
        if (h > 1)
            pix3 = pixFMorphopGen_1(NULL, pix2, L_MORPH_DILATE, selnamev);
        else
            pix3 = pixClone(pix2);
        pix4 = pixRemoveBorder(pix3, 32);
        pixEqual(pixref, pix4, &same);
        if (!same) {
            lept_stderr("pixref != pix4 !\n"); ok = FALSE;
        }
        pixDestroy(&pixref);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);

            /* ----------------- Erosion ----------------- */
        lept_stderr("Testing erosion\n");
        pixref = pixErode(NULL, pixs, sel);
        pix1 = pixErodeBrickDwa(NULL, pixs, w, h);
        pixEqual(pixref, pix1, &same);
        if (!same) {
            lept_stderr("pixref != pix1 !\n"); ok = FALSE;
        }
        pixDestroy(&pix1);

        if (w > 1)
            pix1 = pixMorphDwa_1(NULL, pixs, L_MORPH_ERODE, selnameh);
        else
            pix1 = pixClone(pixs);
        if (h > 1)
            pix2 = pixMorphDwa_1(NULL, pix1, L_MORPH_ERODE, selnamev);
        else
            pix2 = pixClone(pix1);
        pixEqual(pixref, pix2, &same);
        if (!same) {
            lept_stderr("pixref != pix2 !\n"); ok = FALSE;
        }
        pixDestroy(&pix1);
        pixDestroy(&pix2);

        pix1 = pixAddBorder(pixs, 32, 0);
        if (w > 1)
            pix2 = pixFMorphopGen_1(NULL, pix1, L_MORPH_ERODE, selnameh);
        else
            pix2 = pixClone(pix1);
        if (h > 1)
            pix3 = pixFMorphopGen_1(NULL, pix2, L_MORPH_ERODE, selnamev);
        else
            pix3 = pixClone(pix2);
        pix4 = pixRemoveBorder(pix3, 32);
        pixEqual(pixref, pix4, &same);
        if (!same) {
            lept_stderr("pixref != pix4 !\n"); ok = FALSE;
        }
        pixDestroy(&pixref);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);

            /* ----------------- Opening ----------------- */
        lept_stderr("Testing opening\n");
        pixref = pixOpen(NULL, pixs, sel);
        pix1 = pixOpenBrickDwa(NULL, pixs, w, h);
        pixEqual(pixref, pix1, &same);
        if (!same) {
            lept_stderr("pixref != pix1 !\n"); ok = FALSE;
        }
        pixDestroy(&pix1);

        if (h == 1)
            pix2 = pixMorphDwa_1(NULL, pixs, L_MORPH_OPEN, selnameh);
        else if (w == 1)
            pix2 = pixMorphDwa_1(NULL, pixs, L_MORPH_OPEN, selnamev);
        else {
            pix1 = pixMorphDwa_1(NULL, pixs, L_MORPH_ERODE, selnameh);
            pix2 = pixMorphDwa_1(NULL, pix1, L_MORPH_ERODE, selnamev);
            pixMorphDwa_1(pix1, pix2, L_MORPH_DILATE, selnameh);
            pixMorphDwa_1(pix2, pix1, L_MORPH_DILATE, selnamev);
            pixDestroy(&pix1);
        }
        pixEqual(pixref, pix2, &same);
        if (!same) {
            lept_stderr("pixref != pix2 !\n"); ok = FALSE;
        }
        pixDestroy(&pix2);

        pix1 = pixAddBorder(pixs, 32, 0);
        if (h == 1)
            pix3 = pixFMorphopGen_1(NULL, pix1, L_MORPH_OPEN, selnameh);
        else if (w == 1)
            pix3 = pixFMorphopGen_1(NULL, pix1, L_MORPH_OPEN, selnamev);
        else {
            pix2 = pixFMorphopGen_1(NULL, pix1, L_MORPH_ERODE, selnameh);
            pix3 = pixFMorphopGen_1(NULL, pix2, L_MORPH_ERODE, selnamev);
            pixFMorphopGen_1(pix2, pix3, L_MORPH_DILATE, selnameh);
            pixFMorphopGen_1(pix3, pix2, L_MORPH_DILATE, selnamev);
            pixDestroy(&pix2);
        }
        pix4 = pixRemoveBorder(pix3, 32);
        pixEqual(pixref, pix4, &same);
        if (!same) {
            lept_stderr("pixref != pix4 !\n"); ok = FALSE;
        }
        pixDestroy(&pixref);
        pixDestroy(&pix1);
        pixDestroy(&pix3);
        pixDestroy(&pix4);

            /* ----------------- Closing ----------------- */
        lept_stderr("Testing closing\n");
        pixref = pixClose(NULL, pixs, sel);

            /* Note: L_MORPH_CLOSE for h==1 or w==1 gives safe closing,
             * so we can't use it here. */
        if (h == 1) {
            pix1 = pixMorphDwa_1(NULL, pixs, L_MORPH_DILATE, selnameh);
            pix2 = pixMorphDwa_1(NULL, pix1, L_MORPH_ERODE, selnameh);
        }
        else if (w == 1) {
            pix1 = pixMorphDwa_1(NULL, pixs, L_MORPH_DILATE, selnamev);
            pix2 = pixMorphDwa_1(NULL, pix1, L_MORPH_ERODE, selnamev);
        }
        else {
            pix1 = pixMorphDwa_1(NULL, pixs, L_MORPH_DILATE, selnameh);
            pix2 = pixMorphDwa_1(NULL, pix1, L_MORPH_DILATE, selnamev);
            pixMorphDwa_1(pix1, pix2, L_MORPH_ERODE, selnameh);
            pixMorphDwa_1(pix2, pix1, L_MORPH_ERODE, selnamev);
        }
        pixDestroy(&pix1);
        pixEqual(pixref, pix2, &same);
        if (!same) {
            lept_stderr("pixref != pix2 !\n"); ok = FALSE;
        }
        pixDestroy(&pix2);

            /* Note: by adding only 32 pixels of border, we get
             * the normal closing operation, even when calling
             * with L_MORPH_CLOSE, because it requires 32 pixels
             * of border to be safe. */
        pix1 = pixAddBorder(pixs, 32, 0);
        if (h == 1)
            pix3 = pixFMorphopGen_1(NULL, pix1, L_MORPH_CLOSE, selnameh);
        else if (w == 1)
            pix3 = pixFMorphopGen_1(NULL, pix1, L_MORPH_CLOSE, selnamev);
        else {
            pix2 = pixFMorphopGen_1(NULL, pix1, L_MORPH_DILATE, selnameh);
            pix3 = pixFMorphopGen_1(NULL, pix2, L_MORPH_DILATE, selnamev);
            pixFMorphopGen_1(pix2, pix3, L_MORPH_ERODE, selnameh);
            pixFMorphopGen_1(pix3, pix2, L_MORPH_ERODE, selnamev);
            pixDestroy(&pix2);
        }
        pix4 = pixRemoveBorder(pix3, 32);
        pixEqual(pixref, pix4, &same);
        if (!same) {
            lept_stderr("pixref != pix4 !\n"); ok = FALSE;
        }
        pixDestroy(&pixref);
        pixDestroy(&pix1);
        pixDestroy(&pix3);
        pixDestroy(&pix4);

            /* ------------- Safe Closing ----------------- */
        lept_stderr("Testing safe closing\n");
        pixref = pixCloseSafe(NULL, pixs, sel);
        pix0 = pixCloseSafeBrick(NULL, pixs, w, h);
        pixEqual(pixref, pix0, &same);
        if (!same) {
            lept_stderr("pixref != pix0 !\n"); ok = FALSE;
        }
        pixDestroy(&pix0);

        pix1 = pixCloseBrickDwa(NULL, pixs, w, h);
        pixEqual(pixref, pix1, &same);
        if (!same) {
            lept_stderr("pixref != pix1 !\n"); ok = FALSE;
        }
        pixDestroy(&pix1);

        bordercolor = getMorphBorderPixelColor(L_MORPH_ERODE, 1);
        if (bordercolor == 0)   /* asymmetric b.c. */
            extraborder = 32;
        else   /* symmetric b.c. */
            extraborder = 0;

            /* Note: for safe closing we need 64 border pixels.
             * However, when we implement a separable Sel
             * with pixMorphDwa_*(), we must do dilation and
             * erosion explicitly, and these functions only
             * add/remove a 32-pixel border.  Thus, for that
             * case we must add an additional 32-pixel border
             * before doing the operations.  That is the reason
             * why the implementation in morphdwa.c adds the
             * 64 bit border and then uses the lower-level
             * pixFMorphopGen_*() functions. */
        if (h == 1)
            pix3 = pixMorphDwa_1(NULL, pixs, L_MORPH_CLOSE, selnameh);
        else if (w == 1)
            pix3 = pixMorphDwa_1(NULL, pixs, L_MORPH_CLOSE, selnamev);
        else {
            pix0 = pixAddBorder(pixs, extraborder, 0);
            pix1 = pixMorphDwa_1(NULL, pix0, L_MORPH_DILATE, selnameh);
            pix2 = pixMorphDwa_1(NULL, pix1, L_MORPH_DILATE, selnamev);
            pixMorphDwa_1(pix1, pix2, L_MORPH_ERODE, selnameh);
            pixMorphDwa_1(pix2, pix1, L_MORPH_ERODE, selnamev);
            pix3 = pixRemoveBorder(pix2, extraborder);
            pixDestroy(&pix0);
            pixDestroy(&pix1);
            pixDestroy(&pix2);
        }
        pixEqual(pixref, pix3, &same);
        if (!same) {
            lept_stderr("pixref != pix3 !\n"); ok = FALSE;
        }
        pixDestroy(&pix3);

        pix1 = pixAddBorder(pixs, 32 + extraborder, 0);
        if (h == 1)
            pix3 = pixFMorphopGen_1(NULL, pix1, L_MORPH_CLOSE, selnameh);
        else if (w == 1)
            pix3 = pixFMorphopGen_1(NULL, pix1, L_MORPH_CLOSE, selnamev);
        else {
            pix2 = pixFMorphopGen_1(NULL, pix1, L_MORPH_DILATE, selnameh);
            pix3 = pixFMorphopGen_1(NULL, pix2, L_MORPH_DILATE, selnamev);
            pixFMorphopGen_1(pix2, pix3, L_MORPH_ERODE, selnameh);
            pixFMorphopGen_1(pix3, pix2, L_MORPH_ERODE, selnamev);
            pixDestroy(&pix2);
        }
        pix4 = pixRemoveBorder(pix3, 32 + extraborder);
        pixEqual(pixref, pix4, &same);
        if (!same) {
            lept_stderr("pixref != pix4 !\n"); ok = FALSE;
        }
        pixDestroy(&pixref);
        pixDestroy(&pix1);
        pixDestroy(&pix3);
        pixDestroy(&pix4);

        regTestCompareValues(rp, TRUE, ok, 0);
        if (ok)
            lept_stderr("All morph tests OK!\n\n");
        selDestroy(&sel);
        lept_free(selnameh);
        lept_free(selnamev);
    }

    return 0;
}

