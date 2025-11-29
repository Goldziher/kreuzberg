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
 *  dewarprules.c
 *
 *     Syntax: dewarprules select ndew
 *       where select = 0 (sudoku), 1 (graph paper)
 *             ndew = 1 (simple) or 2 (twice with rotations)
 *
 *     There are two ways to dewarp the images:
 *     (1) use dewarpBuildLineModel() to correct both vertical and
 *         horizontal disparity with 1 dew
 *     (2) use dewarpBuildPageModel() twice, correcting only for
 *         vertical disparity, with 90 degree rotations in between
 *         and at the end.
 *
 *     A challenge was presented in:
 *        http://stackoverflow.com/questions/10196198/how-to-remove-convexity-defects-in-sudoku-square/10226971#10226971
 *
 *     Solutions were given there using mathematica and opencv.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
l_int32     w, h, select, ndew;
BOXA       *boxa1;
L_DEWARP   *dew;
L_DEWARPA  *dewa;
PIX        *pixs, *pixd, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;
PIX        *pix8, *pix9, *pix10;
PIXA       *pixa1, *pixa2;

    if (argc != 3) {
        lept_stderr(" Syntax: dewarprules select ndew\n");
        return 1;
    }
    select = atoi(argv[1]);
    ndew = atoi(argv[2]);

    setLeptDebugOK(1);
    lept_mkdir("dewarp");

    if (select == 0) {
            /* Extract the basic grid from the sudoku image */
        pixs = pixRead("warped_sudoku.jpg");
        pixGetDimensions(pixs, &w, &h, NULL);
        pix1 = pixConvertTo1(pixs, 220);
        boxa1 = pixConnComp(pix1, &pixa1, 8);
        pixa2 = pixaSelectBySize(pixa1, 400, 400, L_SELECT_IF_BOTH,
                                 L_SELECT_IF_GT, NULL);
        pix2 = pixaDisplay(pixa2, w, h);  /* grid */
        pixDisplay(pix1, 600, 300);
        pixDisplay(pix2, 100, 100);
    } else {  /* select == 1 */
            /* Extract the grid from the graph paper image */
        pixs = pixRead("warped_paper.jpg");
        pixDisplay(pixs, 1500, 1000);
        pix3 = pixConvertTo8(pixs, 0);
        pix4 = pixBackgroundNormSimple(pix3, NULL, NULL);
        pix5 = pixGammaTRC(NULL, pix4, 1.0, 50, 200);
        pix1 = pixConvertTo1(pix5, 220);
        pixGetDimensions(pix1, &w, &h, NULL);
        boxa1 = pixConnComp(pix1, &pixa1, 8);
        pixa2 = pixaSelectBySize(pixa1, 400, 400, L_SELECT_IF_BOTH,
                                 L_SELECT_IF_GT, NULL);
        pix2 = pixaDisplay(pixa2, w, h);  /* grid */
        pixDisplay(pix1, 600, 300);
        pixDisplay(pix2, 600, 400);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
        pixDestroy(&pix5);
    }

    if (ndew == 1) {
    /* -------------------------------------------------------------------*
     *     Use dewarpBuildLineModel() to correct using both horizontal
     *     and vertical lines with one dew.
     * -------------------------------------------------------------------*/
        dewa = dewarpaCreate(1, 30, 1, 4, 50);
        dewarpaSetCurvatures(dewa, 500, 0, 500, 100, 100, 200);
        dewarpaUseBothArrays(dewa, 1);
        dew = dewarpCreate(pix2, 0);
        dewarpaInsertDewarp(dewa, dew);
        dewarpBuildLineModel(dew, 10, "/tmp/dewarp/sud.pdf");
        dewarpaApplyDisparity(dewa, 0, pix1, 255, 0, 0, &pix3, NULL);
        dewarpaApplyDisparity(dewa, 0, pix2, 255, 0, 0, &pix4, NULL);
        pixDisplay(pix3, 500, 100);
        pixDisplay(pix4, 600, 100);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
        dewarpaDestroy(&dewa);
    } else {
    /* -------------------------------------------------------------------*
     *     Hack: use dewarpBuildPageModel() twice, first straightening
     *     the horizontal lines, then rotating the result by 90 degrees
     *     and doing it again, and finally rotating back by -90 degrees.
     * -------------------------------------------------------------------*/
            /* Extract the horizontal lines */
        pix3 = pixMorphSequence(pix2, "d1.3 + c6.1 + o8.1", 0);
        pixDisplay(pix3, 600, 100);

            /* Correct for vertical disparity */
        dewa = dewarpaCreate(1, 30, 1, 4, 50);
        dewarpaSetCurvatures(dewa, 500, 0, 500, 100, 100, 200);
        dewarpaUseBothArrays(dewa, 0);
        dew = dewarpCreate(pix3, 0);
        dewarpaInsertDewarp(dewa, dew);
        dewarpBuildPageModel(dew, "/tmp/dewarp/sud1.pdf");
        dewarpaApplyDisparity(dewa, 0, pix1, 255, 0, 0, &pix4, NULL);
        dewarpaApplyDisparity(dewa, 0, pix2, 255, 0, 0, &pix5, NULL);
        pixDisplay(pix4, 500, 100);
        pixDisplay(pix5, 600, 100);
        dewarpaDestroy(&dewa);

            /* Rotate result 90 degrees */
        pix6 = pixRotateOrth(pix4, 1);
        pix7 = pixRotateOrth(pix5, 1);  /* grid: vertical lines now are horiz */

            /* Extract the vertical lines (which are now horizontal) */
        pix8 = pixMorphSequence(pix7, "d1.3 + c6.1 + o8.1", 0);
        pixDisplay(pix8, 600, 500);

            /* Correct for vertical (now horizontal) disparity */
        dewa = dewarpaCreate(1, 30, 1, 4, 50);
        dewarpaSetCurvatures(dewa, 500, 0, 500, 100, 100, 200);
        dewarpaUseBothArrays(dewa, 0);
        dew = dewarpCreate(pix8, 0);
        dewarpaInsertDewarp(dewa, dew);
        dewarpBuildPageModel(dew, "/tmp/dewarp/sud2.pdf");
        dewarpaApplyDisparity(dewa, 0, pix6, 255, 0, 0, &pix9, NULL);
        dewarpaApplyDisparity(dewa, 0, pix8, 255, 0, 0, &pix10, NULL);
        pixd = pixRotateOrth(pix9, 3);
        pixDisplay(pix10, 600, 300);
        pixDisplay(pixd, 600, 700);
    }

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxaDestroy(&boxa1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    return 0;
}
