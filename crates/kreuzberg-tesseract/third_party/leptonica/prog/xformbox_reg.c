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
 *   xformbox_reg.c
 *
 *      Tests ordered box transforms (rotation, scaling, translation).
 *      Also tests the various box hashing graphics operations.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Consts for second set */
static const l_int32  SHIFTX_2 = 50;
static const l_int32  SHIFTY_2 = 70;
static const l_float32  SCALEX_2 = 1.17;
static const l_float32  SCALEY_2 = 1.13;
static const l_float32  ROTATION_2 = 0.10;   /* radian */

    /* Consts for third set */
static const l_int32  SHIFTX_3 = 44;
static const l_int32  SHIFTY_3 = 39;
static const l_float32  SCALEX_3 = 0.83;
static const l_float32  SCALEY_3 = 0.78;
static const l_float32  ROTATION_3 = 0.11;   /* radian */


l_int32 RenderTransformedBoxa(PIX *pixt, BOXA *boxa, l_int32 i);


int main(int    argc,
         char **argv)
{
l_int32       i, n, ws, hs, w, h, rval, gval, bval, order;
l_float32    *mat1, *mat2, *mat3;
l_float32     matd[9];
BOX          *box, *boxt;
BOXA         *boxa, *boxat, *boxa1, *boxa2, *boxa3, *boxa4, *boxa5;
PIX          *pix, *pixs, *pixc, *pixt, *pix1, *pix2, *pix3;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ----------------------------------------------------------- *
     *                Test hash rendering in 3 modes               *
     * ----------------------------------------------------------- */
    pixs = pixRead("feyn.tif");
    box = boxCreate(461, 429, 1393, 342);
    pix1 = pixClipRectangle(pixs, box, NULL);
    boxa = pixConnComp(pix1, NULL, 8);
    n = boxaGetCount(boxa);
    pix2 = pixConvertTo8(pix1, 1);
    pix3 = pixConvertTo32(pix1);
    for (i = 0; i < n; i++) {
        boxt = boxaGetBox(boxa, i, L_CLONE);
        rval = (1413 * (i + 1)) % 256;
        gval = (4917 * (i + 1)) % 256;
        bval = (7341 * (i + 1)) % 256;
        pixRenderHashBox(pix1, boxt, 8, 2, i % 4, 1, L_SET_PIXELS);
        pixRenderHashBoxArb(pix2, boxt, 7, 2, i % 4, 1, rval, gval, bval);
        pixRenderHashBoxBlend(pix3, boxt, 7, 2, i % 4, 1,
                              rval, gval, bval, 0.5);
        boxDestroy(&boxt);
    }
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixDisplayWithTitle(pix2, 0, 300, NULL, rp->display);
    pixDisplayWithTitle(pix3, 0, 570, NULL, rp->display);
    boxaDestroy(&boxa);
    boxDestroy(&box);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    /* ----------------------------------------------------------- *
     *        Test orthogonal box rotation and hash rendering      *
     * ----------------------------------------------------------- */
    pixs = pixRead("feyn.tif");
    box = boxCreate(461, 429, 1393, 342);
    pix1 = pixClipRectangle(pixs, box, NULL);
    pixc = pixConvertTo32(pix1);
    pixGetDimensions(pix1, &w, &h, NULL);
    boxa1 = pixConnComp(pix1, NULL, 8);
    pixa = pixaCreate(4);
    for (i = 0; i < 4; i++) {
        pix2 = pixRotateOrth(pixc, i);
        boxa2 = boxaRotateOrth(boxa1, w, h, i);
        rval = (1413 * (i + 4)) % 256;
        gval = (4917 * (i + 4)) % 256;
        bval = (7341 * (i + 4)) % 256;
        pixRenderHashBoxaArb(pix2, boxa2, 10, 3, i, 1, rval, gval, bval);
        pixaAddPix(pixa, pix2, L_INSERT);
        boxaDestroy(&boxa2);
    }
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1200, 0.7, 0, 30, 3);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix3, 0, 800, NULL, rp->display);
    boxDestroy(&box);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixDestroy(&pixc);
    boxaDestroy(&boxa1);
    pixaDestroy(&pixa);

    /* ----------------------------------------------------------- *
     *    Test box transforms with either translation or scaling   *
     *    combined with rotation, using the simple 'ordered'       *
     *    function.  Show that the order of the operations does    *
     *    not matter; different hashing schemes end up in the      *
     *    identical boxes.                                         *
     * ----------------------------------------------------------- */
    pix = pixRead("feyn.tif");
    box = boxCreate(420, 360, 1500, 465);
    pixt = pixClipRectangle(pix, box, NULL);
    pixs = pixAddBorderGeneral(pixt, 0, 200, 0, 0, 0);
    boxDestroy(&box);
    pixDestroy(&pix);
    pixDestroy(&pixt);
    boxa = pixConnComp(pixs, NULL, 8);
    n = boxaGetCount(boxa);
    pixa = pixaCreate(0);

    pixt = pixConvertTo32(pixs);
    for (i = 0; i < 3; i++) {
        if (i == 0)
            order = L_TR_SC_RO;
        else if (i == 1)
            order = L_TR_RO_SC;
        else
            order = L_SC_TR_RO;
        boxat = boxaTransformOrdered(boxa, SHIFTX_2, SHIFTY_2, 1.0, 1.0,
                                     450, 250, ROTATION_2, order);
        RenderTransformedBoxa(pixt, boxat, i);
        boxaDestroy(&boxat);
    }
    pixaAddPix(pixa, pixt, L_INSERT);

    pixt = pixConvertTo32(pixs);
    for (i = 0; i < 3; i++) {
        if (i == 0)
            order = L_RO_TR_SC;
        else if (i == 1)
            order = L_RO_SC_TR;
        else
            order = L_SC_RO_TR;
        boxat = boxaTransformOrdered(boxa, SHIFTX_2, SHIFTY_2, 1.0, 1.0,
                                     450, 250, ROTATION_2, order);
        RenderTransformedBoxa(pixt, boxat, i + 4);
        boxaDestroy(&boxat);
    }
    pixaAddPix(pixa, pixt, L_INSERT);

    pixt = pixConvertTo32(pixs);
    for (i = 0; i < 3; i++) {
        if (i == 0)
            order = L_TR_SC_RO;
        else if (i == 1)
            order = L_SC_RO_TR;
        else
            order = L_SC_TR_RO;
        boxat = boxaTransformOrdered(boxa, 0, 0, SCALEX_2, SCALEY_2,
                                     450, 250, ROTATION_2, order);
        RenderTransformedBoxa(pixt, boxat, i + 8);
        boxaDestroy(&boxat);
    }
    pixaAddPix(pixa, pixt, L_INSERT);

    pixt = pixConvertTo32(pixs);
    for (i = 0; i < 3; i++) {
        if (i == 0)
            order = L_RO_TR_SC;
        else if (i == 1)
            order = L_RO_SC_TR;
        else
            order = L_TR_RO_SC;
        boxat = boxaTransformOrdered(boxa, 0, 0, SCALEX_2, SCALEY_2,
                                     450, 250, ROTATION_2, order);
        RenderTransformedBoxa(pixt, boxat, i + 12);
        boxaDestroy(&boxat);
    }
    pixaAddPix(pixa, pixt, L_INSERT);

    pixt = pixaDisplayTiledInColumns(pixa, 1, 0.5, 20, 0);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pixt, 1000, 0, NULL, rp->display);
    pixDestroy(&pixt);
    pixDestroy(&pixs);
    boxaDestroy(&boxa);
    pixaDestroy(&pixa);


    /* ----------------------------------------------------------- *
     *    Do more testing of box and pta transforms.  Show that    *
     *    resulting boxes are identical by three methods.          *
     * ----------------------------------------------------------- */
        /* Set up pix and boxa */
    pixa = pixaCreate(0);
    pix = pixRead("lucasta.1.300.tif");
    pixTranslate(pix, pix, 70, 0, L_BRING_IN_WHITE);
    pixt = pixCloseBrick(NULL, pix, 14, 5);
    pixOpenBrick(pixt, pixt, 1, 2);
    boxa = pixConnComp(pixt, NULL, 8);
    pixs = pixConvertTo32(pix);
    pixc = pixCopy(NULL, pixs);
    RenderTransformedBoxa(pixc, boxa, 113);
    pixaAddPix(pixa, pixc, L_INSERT);
    pixDestroy(&pix);
    pixDestroy(&pixt);

        /* (a) Do successive discrete operations: shift, scale, rotate */
    pix1 = pixTranslate(NULL, pixs, SHIFTX_3, SHIFTY_3, L_BRING_IN_WHITE);
    boxa1 = boxaTranslate(boxa, SHIFTX_3, SHIFTY_3);
    pixc = pixCopy(NULL, pix1);
    RenderTransformedBoxa(pixc, boxa1, 213);
    pixaAddPix(pixa, pixc, L_INSERT);

    pix2 = pixScale(pix1, SCALEX_3, SCALEY_3);
    boxa2 = boxaScale(boxa1, SCALEX_3, SCALEY_3);
    pixc = pixCopy(NULL, pix2);
    RenderTransformedBoxa(pixc, boxa2, 313);
    pixaAddPix(pixa, pixc, L_INSERT);

    pixGetDimensions(pix2, &w, &h, NULL);
    pix3 = pixRotateAM(pix2, ROTATION_3, L_BRING_IN_WHITE);
    boxa3 = boxaRotate(boxa2, w / 2, h / 2, ROTATION_3);
    pixc = pixCopy(NULL, pix3);
    RenderTransformedBoxa(pixc, boxa3, 413);
    pixaAddPix(pixa, pixc, L_INSERT);

        /* (b) Set up and use the composite transform */
    mat1 = createMatrix2dTranslate(SHIFTX_3, SHIFTY_3);
    mat2 = createMatrix2dScale(SCALEX_3, SCALEY_3);
    mat3 = createMatrix2dRotate(w / 2, h / 2, ROTATION_3);
    l_productMat3(mat3, mat2, mat1, matd, 3);
    boxa4 = boxaAffineTransform(boxa, matd);
    pixc = pixCopy(NULL, pix3);
    RenderTransformedBoxa(pixc, boxa4, 513);
    pixaAddPix(pixa, pixc, L_INSERT);

        /* (c) Use the special 'ordered' function */
    pixGetDimensions(pixs, &ws, &hs, NULL);
    boxa5 = boxaTransformOrdered(boxa, SHIFTX_3, SHIFTY_3,
                                 SCALEX_3, SCALEY_3,
                                 ws / 2, hs / 2, ROTATION_3, L_TR_SC_RO);
    pixc = pixCopy(NULL, pix3);
    RenderTransformedBoxa(pixc, boxa5, 613);
    pixaAddPix(pixa, pixc, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    boxaDestroy(&boxa3);
    boxaDestroy(&boxa4);
    boxaDestroy(&boxa5);
    lept_free(mat1);
    lept_free(mat2);
    lept_free(mat3);

    pixt = pixaDisplayTiledInColumns(pixa, 2, 0.5, 20, 0);
    regTestWritePixAndCheck(rp, pixt, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pixt, 1000, 300, NULL, rp->display);
    pixDestroy(&pixt);
    pixDestroy(&pixs);
    boxaDestroy(&boxa);
    pixaDestroy(&pixa);
    return regTestCleanup(rp);
}


l_int32
RenderTransformedBoxa(PIX    *pixt,
                      BOXA   *boxa,
                      l_int32 i)
{
l_int32  j, n, rval, gval, bval;
BOX     *box;

    n = boxaGetCount(boxa);
    rval = (1413 * i) % 256;
    gval = (4917 * i) % 256;
    bval = (7341 * i) % 256;
    for (j = 0; j < n; j++) {
        box = boxaGetBox(boxa, j, L_CLONE);
        pixRenderHashBoxArb(pixt, box, 10, 3, i % 4, 1, rval, gval, bval);
        boxDestroy(&box);
    }
    return 0;
}



