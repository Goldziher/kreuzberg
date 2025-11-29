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
 * bilinear_reg.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void MakePtas(l_int32 i, PTA **pptas, PTA **pptad);

    /* Sample values.
     *    1: test with relatively large distortion
     *    2-3: invertability tests
     */
static const l_int32  x1[] =  {  32,   32,   32};
static const l_int32  y1[] =  { 150,  150,  150};
static const l_int32  x2[] =  { 520,  520,  520};
static const l_int32  y2[] =  { 150,  150,  150};
static const l_int32  x3[] =  {  32,   32,   32};
static const l_int32  y3[] =  { 612,  612,  612};
static const l_int32  x4[] =  { 520,  520,  520};
static const l_int32  y4[] =  { 612,  612,  612};

static const l_int32  xp1[] = {  32,   32,   32};
static const l_int32  yp1[] = { 150,  150,  150};
static const l_int32  xp2[] = { 520,  520,  520};
static const l_int32  yp2[] = {  44,  124,  140};
static const l_int32  xp3[] = {  32,   32,   32};
static const l_int32  yp3[] = { 612,  612,  612};
static const l_int32  xp4[] = { 520,  520,  520};
static const l_int32  yp4[] = { 694,  624,  622};

#define  ALL                        1
#define  ADDED_BORDER_PIXELS      250

int main(int    argc,
         char **argv)
{
l_int32       i;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pixd;
PIX          *pixb, *pixg, *pixc, *pixcs;
PIXA         *pixa;
PTA          *ptas, *ptad;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn.tif");
    pixg = pixScaleToGray(pixs, 0.2);
    pixDestroy(&pixs);

#if ALL
        /* Test non-invertability of sampling */
    lept_stderr("Test invertability of sampling\n");
    pixa = pixaCreate(0);
    for (i = 1; i < 3; i++) {
        pixb = pixAddBorder(pixg, ADDED_BORDER_PIXELS, 255);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixBilinearSampledPta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0,3,6 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixBilinearSampledPta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1,4,7 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS);
        pixInvert(pixd, pixd);
        pixXor(pixd, pixd, pixg);
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
    pixaDestroy(&pixa);
#endif

#if ALL
        /* Test invertability of grayscale interpolation */
    lept_stderr("Test invertability of grayscale interpolation\n");
    pixa = pixaCreate(0);
    for (i = 1; i < 3; i++) {
        pixb = pixAddBorder(pixg, ADDED_BORDER_PIXELS, 255);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixBilinearPta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 10,13 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixBilinearPta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 11,14 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS);
        pixInvert(pixd, pixd);
        pixXor(pixd, pixd, pixg);
        regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 12,15 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 0.5, 20, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 16 */
    pixDisplayWithTitle(pix1, 200, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
#endif

#if ALL
        /* Test invertability of color interpolation */
    lept_stderr("Test invertability of color interpolation\n");
    pixa = pixaCreate(0);
    pixc = pixRead("test24.jpg");
    pixcs = pixScale(pixc, 0.3, 0.3);
    for (i = 1; i < 3; i++) {
        pixb = pixAddBorder(pixcs, ADDED_BORDER_PIXELS / 2, 0xffffff00);
        MakePtas(i, &ptas, &ptad);
        pix1 = pixBilinearPta(pixb, ptad, ptas, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 17,20 */
        pixaAddPix(pixa, pix1, L_INSERT);
        pix2 = pixBilinearPta(pix1, ptas, ptad, L_BRING_IN_WHITE);
        regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 18,21 */
        pixaAddPix(pixa, pix2, L_INSERT);
        pixd = pixRemoveBorder(pix2, ADDED_BORDER_PIXELS / 2);
        pixXor(pixd, pixd, pixc);
        pixInvert(pixd, pixd);
        regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 19,22 */
        pixaAddPix(pixa, pixd, L_INSERT);
        pixDestroy(&pixb);
        ptaDestroy(&ptas);
        ptaDestroy(&ptad);
    }

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 0.5, 20, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 23 */
    pixDisplayWithTitle(pix1, 400, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pixc);
    pixDestroy(&pixcs);
    pixaDestroy(&pixa);
#endif

#if ALL
        /* Comparison between sampling and interpolated */
    lept_stderr("Compare sampling with interpolated\n");
    MakePtas(2, &ptas, &ptad);
    pixa = pixaCreate(0);

        /* Use sampled transform */
    pix1 = pixBilinearSampledPta(pixg, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 24 */
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Use interpolated transforms */
    pix2 = pixBilinearPta(pixg, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 25 */
    pixaAddPix(pixa, pix2, L_COPY);

        /* Compare the results */
    pixXor(pix2, pix2, pix1);
    pixInvert(pix2, pix2);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 26 */
    pixaAddPix(pixa, pix2, L_INSERT);

    pix1 = pixaDisplayTiledInColumns(pixa, 3, 0.5, 20, 3);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 27 */
    pixDisplayWithTitle(pix1, 600, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pixg);
    pixaDestroy(&pixa);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);
#endif

#if ALL
        /* Large distortion with inversion */
    lept_stderr("Large bilinear distortion with inversion\n");
    MakePtas(0, &ptas, &ptad);
    pixa = pixaCreate(0);
    pixs = pixRead("marge.jpg");
    pixg = pixConvertTo8(pixs, 0);

    pix1 = pixBilinearSampledPta(pixg, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 28 */
    pixaAddPix(pixa, pix1, L_INSERT);

    pix2 = pixBilinearPta(pixg, ptas, ptad, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 29 */
    pixaAddPix(pixa, pix2, L_INSERT);

    pix3 = pixBilinearSampledPta(pix1, ptad, ptas, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 30 */
    pixaAddPix(pixa, pix3, L_INSERT);

    pix4 = pixBilinearPta(pix2, ptad, ptas, L_BRING_IN_WHITE);
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 31 */
    pixaAddPix(pixa, pix4, L_INSERT);

    pix1 = pixaDisplayTiledInColumns(pixa, 4, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 32 */
    pixDisplayWithTitle(pix1, 800, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

    pixDestroy(&pixs);
    pixDestroy(&pixg);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);
#endif

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
