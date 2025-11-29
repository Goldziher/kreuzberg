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
 * projective_reg.c
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void MakePtas(l_int32 i, PTA **pptas, PTA **pptad);

    /* Sample values.
     *    1-3: invertability tests
     *    4: comparison between sampling and sequential
     *    5: test with large distortion
     */
static const l_int32  x1[] =  { 300,  300,  300,  300,   32};
static const l_int32  y1[] =  {1200, 1200, 1250, 1250,  934};
static const l_int32  x2[] =  {1200, 1200, 1125, 1300,  487};
static const l_int32  y2[] =  {1100, 1100, 1100, 1250,  934};
static const l_int32  x3[] =  { 200,  200,  200,  250,   32};
static const l_int32  y3[] =  { 200,  200,  200,  300,   67};
static const l_int32  x4[] =  {1200, 1200, 1300, 1250,  332};
static const l_int32  y4[] =  { 400,  200,  200,  300,   57};

static const l_int32  xp1[] = { 300,  300, 1150,  300,   32};
static const l_int32  yp1[] = {1200, 1400, 1150, 1350,  934};
static const l_int32  xp2[] = {1100, 1400,  320, 1300,  487};
static const l_int32  yp2[] = {1000, 1500, 1300, 1200,  904};
static const l_int32  xp3[] = { 250,  200, 1310,  300,   61};
static const l_int32  yp3[] = { 200,  300,  250,  325,   83};
static const l_int32  xp4[] = {1250, 1200,  240, 1250,  412};
static const l_int32  yp4[] = { 300,  300,  250,  350,   83};

#define   ADDED_BORDER_PIXELS       250
#define   ALL     1


int main(int    argc,
         char **argv)
{
l_int32       i;
PIX          *pixs, *pixsc, *pixb, *pixg, *pixc, *pixcs, *pix1, *pix2, *pixd;
PIXA         *pixa;
PTA          *ptas, *ptad;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn.tif");
    pixsc = pixScale(pixs, 0.3, 0.3);

#if ALL
        /* Test invertability of sampling */
    lept_stderr("Test invertability of sampling\n");
    pixa = pixaCreate(0);
    for (i = 0; i < 3; i++) {
        pixb = pixAddBorder(pixsc, ADDED_BORDER_PIXELS, 0);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixProjectiveSampledPta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0,3,6 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixProjectiveSampledPta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1,4,7 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS);
        pixXor(pixd, pixd, pixsc);
        regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 2,5,8 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 0.5, 20, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 9 */
    pixDisplayWithTitle(pix1, 0, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pixsc);
    pixaDestroy(&pixa);
#endif

#if ALL
        /* Test invertability of interpolation on grayscale */
    lept_stderr("Test invertability of grayscale interpolation\n");
    pixa = pixaCreate(0);
    pixg = pixScaleToGray(pixs, 0.2);
    for (i = 0; i < 2; i++) {
        pixb = pixAddBorder(pixg, ADDED_BORDER_PIXELS / 2, 255);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixProjectivePta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 10,13 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixProjectivePta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 11,14 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS / 2);
        pixXor(pixd, pixd, pixg);
        pixInvert(pixd, pixd);
        regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 12,15 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 0.5, 20, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 16 */
    pixDisplayWithTitle(pix1, 300, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pixg);
    pixaDestroy(&pixa);
#endif

#if ALL
        /* Test invertability of interpolation on color */
    lept_stderr("Test invertability of color interpolation\n");
    pixa = pixaCreate(0);
    pixc = pixRead("test24.jpg");
    pixcs = pixScale(pixc, 0.3, 0.3);
    for (i = 0; i < 5; i++) {
        if (i == 2) continue;
        pixb = pixAddBorder(pixcs, ADDED_BORDER_PIXELS / 2, 0xffffff00);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixProjectivePta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 17,20,23,26 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixProjectivePta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 18,21,24,27 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS / 2);
        pixXor(pixd, pixd, pixcs);
        pixInvert(pixd, pixd);
        regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 19,22,25,28 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 0.5, 20, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 29 */
    pixDisplayWithTitle(pix1, 600, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pixc);
    pixDestroy(&pixcs);
    pixaDestroy(&pixa);
#endif

#if ALL
       /* Comparison between sampling and interpolated */
    lept_stderr("Compare sampling with interpolated\n");
    MakePtas(3, &ptas, &ptad);
    pixa = pixaCreate(0);
    pixg = pixScaleToGray(pixs, 0.2);

        /* Use sampled transform */
    pix1 = pixProjectiveSampledPta(pixg, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 30 */
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Use interpolated transforms */
    pix2 = pixProjectivePta(pixg, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 31 */
    pixaAddPix(pixa, pix2, L_COPY);

        /* Compare the results */
    pixXor(pix2, pix2, pix1);
    pixInvert(pix2, pix2);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 32 */
    pixaAddPix(pixa, pix2, L_INSERT);

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 0.5, 20, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 33 */
    pixDisplayWithTitle(pix1, 900, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pixg);
    pixaDestroy(&pixa);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);
#endif

    pixDestroy(&pixs);
    return regTestCleanup(rp);
}

static void
MakePtas(l_int32  i,
         PTA    **pptas,
         PTA    **pptad)
{

    *pptas = ptaCreate(4);
    ptaAddPt(*pptas, x1[i], y1[i]);
    ptaAddPt(*pptas, x2[i], y2[i]);
    ptaAddPt(*pptas, x3[i], y3[i]);
    ptaAddPt(*pptas, x4[i], y4[i]);
    *pptad = ptaCreate(4);
    ptaAddPt(*pptad, xp1[i], yp1[i]);
    ptaAddPt(*pptad, xp2[i], yp2[i]);
    ptaAddPt(*pptad, xp3[i], yp3[i]);
    ptaAddPt(*pptad, xp4[i], yp4[i]);
    return;
}

