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
 * affine_reg.c
 *
 *   Tests affine transforms, including invertability and large distortions.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void MakePtas(l_int32 i, PTA **pptas, PTA **pptad);
static l_int32 RenderHashedBoxa(PIX *pixt, BOXA *boxa, l_int32 i);


    /* Sample values.
     *    1-3: invertability tests
     *    4: comparison between sampling and sequential
     *    5: test with large distortion
     */
static const l_int32  x1[] =  { 300,  300,  300,  95,    32};
static const l_int32  y1[] =  {1200, 1200, 1250, 2821,  934};
static const l_int32  x2[] =  {1200, 1200, 1125, 1432,  487};
static const l_int32  y2[] =  {1100, 1100, 1100, 2682,  934};
static const l_int32  x3[] =  { 200,  200,  200,  232,   32};
static const l_int32  y3[] =  { 200,  200,  200,  657,   67};

static const l_int32  xp1[] = { 500,  300,  350,  117,   32};
static const l_int32  yp1[] = {1700, 1400, 1400, 2629,  934};
static const l_int32  xp2[] = {850, 1400, 1400, 1464,  487};
static const l_int32  yp2[] = {850, 1500, 1500, 2432,  804};
static const l_int32  xp3[] = { 450,  200,  400,  183,   61};
static const l_int32  yp3[] = { 300,  300,  400,  490,   83};

static const l_int32  SHIFTX = 44;
static const l_int32  SHIFTY = 39;
static const l_float32  SCALEX = 0.83;
static const l_float32  SCALEY = 0.78;
static const l_float32  ROTATION = 0.11;   /* radian */

#define   ADDED_BORDER_PIXELS       1000
#define   ALL     1


int main(int    argc,
         char **argv)
{
l_int32       i, w, h;
l_float32    *mat1, *mat2, *mat3, *mat1i, *mat2i, *mat3i, *matdinv;
l_float32     matd[9], matdi[9];
BOXA         *boxa, *boxa2;
PIX          *pix, *pixs, *pixb, *pixg, *pixc, *pixcs;
PIX          *pixd, *pix1, *pix2, *pix3;
PIXA         *pixa;
PTA          *ptas, *ptad;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pix = pixRead("feyn.tif");
    pixs = pixScale(pix, 0.22, 0.22);
    pixDestroy(&pix);

#if ALL
        /* Test invertability of sequential. */
    lept_stderr("Test invertability of sequential\n");
    pixa = pixaCreate(0);
    for (i = 0; i < 3; i++) {
        pixb = pixAddBorder(pixs, ADDED_BORDER_PIXELS, 0);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixAffineSequential(pixb, ptad, ptas, 0, 0);
        regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0,3,6 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixAffineSequential(pix1, ptas, ptad, 0, 0);
        regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1,4,7 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS);
        pixXor(pixd, pixd, pixs);
        regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 2,5,8 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 3);
    pix2 = pixScaleToGray(pix1, 0.2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 9 */
    pixDisplayWithTitle(pix2, 0, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixaDestroy(&pixa);
#endif

#if ALL
        /* Test invertability of sampling */
    lept_stderr("Test invertability of sampling\n");
    pixa = pixaCreate(0);
    for (i = 0; i < 3; i++) {
        pixb = pixAddBorder(pixs, ADDED_BORDER_PIXELS, 0);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixAffineSampledPta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 10,13,16 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixAffineSampledPta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 11,14,17 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS);
        pixXor(pixd, pixd, pixs);
        regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 12,15,18 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 3);
    pix2 = pixScaleToGray(pix1, 0.2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 19 */
    pixDisplayWithTitle(pix2, 200, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pixs);
    pixaDestroy(&pixa);
#endif

#if ALL
        /* Test invertability of interpolation on grayscale */
    lept_stderr("Test invertability of grayscale interpolation\n");
    pix = pixRead("feyn.tif");
    pixg = pixScaleToGray3(pix);
    pixDestroy(&pix);
    pixa = pixaCreate(0);
    for (i = 0; i < 3; i++) {
        pixb = pixAddBorder(pixg, ADDED_BORDER_PIXELS / 3, 255);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixAffinePta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 20,23,26 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixAffinePta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 21,24,27 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS / 3);
        pixXor(pixd, pixd, pixg);
        pixInvert(pixd, pixd);
        regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 22,25,28 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 3);
    pix2 = pixScale(pix1, 0.2, 0.2);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 29 */
    pixDisplayWithTitle(pix2, 400, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pixg);
    pixaDestroy(&pixa);
#endif

#if ALL
        /* Test invertability of interpolation on color */
    lept_stderr("Test invertability of color interpolation\n");
    pixa = pixaCreate(0);
    pixc = pixRead("test24.jpg");
    pixcs = pixScale(pixc, 0.3, 0.3);
    for (i = 0; i < 3; i++) {
        pixb = pixAddBorder(pixcs, ADDED_BORDER_PIXELS / 4, 0xffffff00);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixAffinePta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 30,33,36 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixAffinePta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 31,34,37 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS / 4);
        pixXor(pixd, pixd, pixcs);
        pixInvert(pixd, pixd);
        regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 32,35,38 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }
    pixDestroy(&pixcs);

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 3);
    pix2 = pixScale(pix1, 0.25, 0.25);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 39 */
    pixDisplayWithTitle(pix2, 600, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pixc);
    pixaDestroy(&pixa);
#endif

#if ALL
       /* Comparison between sequential and sampling */
    lept_stderr("Compare sequential with sampling\n");
    pix = pixRead("feyn.tif");
    pixs = pixScale(pix, 0.22, 0.22);
    pixDestroy(&pix);

    MakePtas(3, &ptas, &ptad);
    pixa = pixaCreate(0);

        /* Use sequential transforms */
    pix1 = pixAffineSequential(pixs, ptas, ptad,
                     ADDED_BORDER_PIXELS, ADDED_BORDER_PIXELS);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 40 */
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Use sampled transform */
    pix2 = pixAffineSampledPta(pixs, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 41 */
    pixaAddPix(pixa, pix2, L_COPY);

        /* Compare the results */
    pixXor(pix2, pix2, pix1);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 42 */
    pixaAddPix(pixa, pix2, L_INSERT);

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 1.0, 20, 3);
    pix2 = pixScale(pix1, 0.5, 0.5);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 43 */
    pixDisplayWithTitle(pix2, 800, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pixs);
    pixaDestroy(&pixa);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);
#endif


#if ALL
       /* Test with large distortion */
    lept_stderr("Test with large distortion\n");
    MakePtas(4, &ptas, &ptad);
    pixa = pixaCreate(0);
    pix = pixRead("feyn.tif");
    pixg = pixScaleToGray6(pix);
    pixDestroy(&pix);

    pix1 = pixAffineSequential(pixg, ptas, ptad, 0, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 44 */
    pixaAddPix(pixa, pix1, L_COPY);

    pix2 = pixAffineSampledPta(pixg, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 45 */
    pixaAddPix(pixa, pix2, L_COPY);

    pix3 = pixAffinePta(pixg, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 46 */
    pixaAddPix(pixa, pix3, L_INSERT);

    pixXor(pix1, pix1, pix2);
    pixInvert(pix1, pix1);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 47 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixXor(pix2, pix2, pix3);
    pixInvert(pix2, pix2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 48 */
    pixaAddPix(pixa, pix2, L_INSERT);

    pix1 = pixaDisplayTiledInColumns(pixa, 5, 1.0, 20, 3);
    pix2 = pixScale(pix1, 0.8, 0.8);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 49 */
    pixDisplayWithTitle(pix2, 1000, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pixg);
    pixaDestroy(&pixa);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);
#endif

#if ALL
        /* Set up pix and boxa */
    lept_stderr("Test affine transforms and inverses on pix and boxa\n");
    pixa = pixaCreate(0);
    pix = pixRead("lucasta.1.300.tif");
    pixTranslate(pix, pix, 70, 0, L_BRING_IN_WHITE);
    pix1 = pixCloseBrick(NULL, pix, 14, 5);
    pixOpenBrick(pix1, pix1, 1, 2);
    boxa = pixConnComp(pix1, NULL, 8);
    pixs = pixConvertTo32(pix);
    pixGetDimensions(pixs, &w, &h, NULL);
    pixc = pixCopy(NULL, pixs);
    RenderHashedBoxa(pixc, boxa, 113);
    regTestWritePixAndCheck(rp, pixc, IFF_PNG);  /* 50 */
    pixaAddPix(pixa, pixc, L_INSERT);
    pixDestroy(&pix);
    pixDestroy(&pix1);

        /* Set up an affine transform in matd, and apply it to boxa */
    mat1 = createMatrix2dTranslate(SHIFTX, SHIFTY);
    mat2 = createMatrix2dScale(SCALEX, SCALEY);
    mat3 = createMatrix2dRotate(w / 2, h / 2, ROTATION);
    l_productMat3(mat3, mat2, mat1, matd, 3);
    boxa2 = boxaAffineTransform(boxa, matd);

        /* Set up the inverse transform --> matdi */
    mat1i = createMatrix2dTranslate(-SHIFTX, -SHIFTY);
    mat2i = createMatrix2dScale(1.0/ SCALEX, 1.0 / SCALEY);
    mat3i = createMatrix2dRotate(w / 2, h / 2, -ROTATION);
    l_productMat3(mat1i, mat2i, mat3i, matdi, 3);

        /* Invert the original affine transform --> matdinv */
    affineInvertXform(matd, &matdinv);
    if (rp->display) {
        lept_stderr("  Affine transform, applied to boxa\n");
        for (i = 0; i < 9; i++) {
            if (i && (i % 3 == 0))  lept_stderr("\n");
            lept_stderr("   %7.3f ", matd[i]);
        }
        lept_stderr("\n  Inverse transform, by composing inverse parts");
        for (i = 0; i < 9; i++) {
            if (i % 3 == 0)  lept_stderr("\n");
            lept_stderr("   %7.3f ", matdi[i]);
        }
        lept_stderr("\n  Inverse transform, by inverting affine xform");
        for (i = 0; i < 6; i++) {
            if (i % 3 == 0)  lept_stderr("\n");
            lept_stderr("   %7.3f ", matdinv[i]);
        }
        lept_stderr("\n");
    }

        /* Apply the inverted affine transform --> pixs */
    pixd = pixAffine(pixs, matdinv, L_BRING_IN_WHITE);
    RenderHashedBoxa(pixd, boxa2, 513);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 51 */
    pixaAddPix(pixa, pixd, L_INSERT);

    pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 30, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 52 */
    pixDisplayWithTitle(pix1, 1200, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

    pixDestroy(&pixs);
    boxaDestroy(&boxa);
    boxaDestroy(&boxa2);
    lept_free(mat1);
    lept_free(mat2);
    lept_free(mat3);
    lept_free(mat1i);
    lept_free(mat2i);
    lept_free(mat3i);
    lept_free(matdinv);
#endif

    return regTestCleanup(rp);
}

static void
MakePtas(l_int32  i,
         PTA    **pptas,
         PTA    **pptad)
{

    *pptas = ptaCreate(3);
    ptaAddPt(*pptas, x1[i], y1[i]);
    ptaAddPt(*pptas, x2[i], y2[i]);
    ptaAddPt(*pptas, x3[i], y3[i]);
    *pptad = ptaCreate(3);
    ptaAddPt(*pptad, xp1[i], yp1[i]);
    ptaAddPt(*pptad, xp2[i], yp2[i]);
    ptaAddPt(*pptad, xp3[i], yp3[i]);
    return;
}


static l_int32
RenderHashedBoxa(PIX    *pixt,
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


