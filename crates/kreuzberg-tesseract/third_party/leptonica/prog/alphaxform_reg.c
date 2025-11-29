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
 * alphaxform_reg.c
 *
 *   This tests the alpha blending functions when used with various
 *   transforms (scaling, rotation, affine, projective, bilinear).
 *
 *   It also tests the version that are wrapped in a gamma transform,
 *   which is a technique for getting a truer color on transforming,
 *   because it undoes the gamma that has been applied to an image
 *   before transforming and then re-applying the gamma transform.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void MakePtas(l_int32 i, l_int32 npts, PTA **pptas, PTA **pptad);

static const l_int32  x1[] =  {  300,   300,   300,    95,   32 };
static const l_int32  y1[] =  { 1200,  1200,  1250,  2821,  934 };
static const l_int32  x2[] =  { 1200,  1200,  1125,  1432,  487 };
static const l_int32  y2[] =  { 1100,  1100,  1100,  2682,  934 };
static const l_int32  x3[] =  {  200,   200,   200,   232,   32 };
static const l_int32  y3[] =  {  200,   200,   200,   657,   67 };
static const l_int32  x4[] =  { 1200,  1200,  1125,  1432,  487 };
static const l_int32  y4[] =  {  200,   200,   200,   242,   84 };

static const l_int32  xp1[] = {  500,   300,   350,   117,   32 };
static const l_int32  yp1[] = { 1700,  1400,  1100,  2629,  934 };
static const l_int32  xp2[] = {  850,   400,  1100,  1664,  487 };
static const l_int32  yp2[] = {  850,   500,  1300,  2432,  804 };
static const l_int32  xp3[] = {  450,   200,   400,   183,   61 };
static const l_int32  yp3[] = {  300,   300,   400,   490,   83 };
static const l_int32  xp4[] = {  850,   1000, 1100,  1664,  487 };
static const l_int32  yp4[] = {  350,    350,  400,   532,  114 };

int main(int    argc,
         char **argv)
{
PIX          *pixs2, *pixs3, *pixb1, *pixb2, *pixb3;
PIX          *pixr2, *pixr3;
PIX          *pixc1, *pixc2, *pixc3, *pixcs1, *pixcs2, *pixcs3;
PIX          *pixd, *pixt1, *pixt2, *pixt3;
PTA          *ptas1, *ptas2, *ptas3, *ptad1, *ptad2, *ptad3;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixc1 = pixRead("test24.jpg");
    pixc2 = pixRead("wyom.jpg");
    pixc3 = pixRead("marge.jpg");

        /* Test alpha blend scaling */
    pixd = pixCreate(900, 400, 32);
    pixSetAll(pixd);
    pixs2 = pixScaleWithAlpha(pixc2, 0.5, 0.5, NULL, 0.3);
    pixs3 = pixScaleWithAlpha(pixc3, 0.4, 0.4, NULL, 0.7);
    pixb1 = pixBlendWithGrayMask(pixd, pixs3, NULL, 100, 100);
    pixb2 = pixBlendWithGrayMask(pixb1, pixs2, NULL, 300, 130);
    pixb3 = pixBlendWithGrayMask(pixb2, pixs3, NULL, 600, 160);
    regTestWritePixAndCheck(rp, pixb3, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pixb3, 900, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixs2);
    pixDestroy(&pixs3);
    pixDestroy(&pixb1);
    pixDestroy(&pixb2);
    pixDestroy(&pixb3);

        /* Test alpha blend rotation */
    pixd = pixCreate(1200, 800, 32);
    pixSetAll(pixd);
    pixr3 = pixRotateWithAlpha(pixc3, -0.3, NULL, 1.0);
    pixr2 = pixRotateWithAlpha(pixc2, +0.3, NULL, 1.0);
    pixb3 = pixBlendWithGrayMask(pixd, pixr3, NULL, 100, 100);
    pixb2 = pixBlendWithGrayMask(pixb3, pixr2, NULL, 400, 100);
    regTestWritePixAndCheck(rp, pixb2, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pixb2, 500, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixr3);
    pixDestroy(&pixr2);
    pixDestroy(&pixb3);
    pixDestroy(&pixb2);

    pixcs1 = pixScale(pixc1, 0.35, 0.35);
    pixcs2 = pixScale(pixc2, 0.55, 0.55);
    pixcs3 = pixScale(pixc3, 0.65, 0.65);

        /* Test alpha blend affine */
    pixd = pixCreate(800, 900, 32);
    pixSetAll(pixd);
    MakePtas(2, 3, &ptas1, &ptad1);
    MakePtas(4, 3, &ptas2, &ptad2);
    MakePtas(3, 3, &ptas3, &ptad3);
    pixt1 = pixAffinePtaWithAlpha(pixcs1, ptad1, ptas1, NULL, 1.0, 300);
    pixt2 = pixAffinePtaWithAlpha(pixcs2, ptad2, ptas2, NULL, 0.8, 400);
    pixt3 = pixAffinePtaWithAlpha(pixcs3, ptad3, ptas3, NULL, 0.7, 300);
    pixb1 = pixBlendWithGrayMask(pixd, pixt1, NULL, -250, 20);
    pixb2 = pixBlendWithGrayMask(pixb1, pixt2, NULL, -150, -250);
    pixb3 = pixBlendWithGrayMask(pixb2, pixt3, NULL, -100, 220);
    regTestWritePixAndCheck(rp, pixb3, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pixb3, 100, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixb1);
    pixDestroy(&pixb2);
    pixDestroy(&pixb3);
    pixDestroy(&pixt1);
    pixDestroy(&pixt2);
    pixDestroy(&pixt3);
    ptaDestroy(&ptas1);
    ptaDestroy(&ptas2);
    ptaDestroy(&ptas3);
    ptaDestroy(&ptad1);
    ptaDestroy(&ptad2);
    ptaDestroy(&ptad3);

        /* Test alpha blend projective */
    pixd = pixCreate(900, 900, 32);
    pixSetAll(pixd);
    MakePtas(2, 4, &ptas1, &ptad1);
    MakePtas(4, 4, &ptas2, &ptad2);
    MakePtas(3, 4, &ptas3, &ptad3);
    pixt1 = pixProjectivePtaWithAlpha(pixcs1, ptad1, ptas1, NULL, 1.0, 300);
    pixt2 = pixProjectivePtaWithAlpha(pixcs2, ptad2, ptas2, NULL, 0.8, 400);
    pixt3 = pixProjectivePtaWithAlpha(pixcs3, ptad3, ptas3, NULL, 0.7, 400);
    pixb1 = pixBlendWithGrayMask(pixd, pixt1, NULL, -150, 20);
    pixb2 = pixBlendWithGrayMask(pixb1, pixt2, NULL, -50, -250);
    pixb3 = pixBlendWithGrayMask(pixb2, pixt3, NULL, -100, 220);
    regTestWritePixAndCheck(rp, pixb3, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pixb3, 300, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixb1);
    pixDestroy(&pixb2);
    pixDestroy(&pixb3);
    pixDestroy(&pixt1);
    pixDestroy(&pixt2);
    pixDestroy(&pixt3);
    ptaDestroy(&ptas1);
    ptaDestroy(&ptas2);
    ptaDestroy(&ptas3);
    ptaDestroy(&ptad1);
    ptaDestroy(&ptad2);
    ptaDestroy(&ptad3);

        /* Test alpha blend bilinear */
    pixd = pixCreate(900, 900, 32);
    pixSetAll(pixd);
    MakePtas(2, 4, &ptas1, &ptad1);
    MakePtas(4, 4, &ptas2, &ptad2);
    MakePtas(3, 4, &ptas3, &ptad3);
    pixt1 = pixBilinearPtaWithAlpha(pixcs1, ptad1, ptas1, NULL, 1.0, 300);
    pixt2 = pixBilinearPtaWithAlpha(pixcs2, ptad2, ptas2, NULL, 0.8, 400);
    pixt3 = pixBilinearPtaWithAlpha(pixcs3, ptad3, ptas3, NULL, 0.7, 400);
    pixb1 = pixBlendWithGrayMask(pixd, pixt1, NULL, -150, 20);
    pixb2 = pixBlendWithGrayMask(pixb1, pixt2, NULL, -50, -250);
    pixb3 = pixBlendWithGrayMask(pixb2, pixt3, NULL, -100, 220);
    regTestWritePixAndCheck(rp, pixb3, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pixb3, 500, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixDestroy(&pixb1);
    pixDestroy(&pixb2);
    pixDestroy(&pixb3);
    pixDestroy(&pixt1);
    pixDestroy(&pixt2);
    pixDestroy(&pixt3);
    ptaDestroy(&ptas1);
    ptaDestroy(&ptas2);
    ptaDestroy(&ptas3);
    ptaDestroy(&ptad1);
    ptaDestroy(&ptad2);
    ptaDestroy(&ptad3);

    pixDestroy(&pixc1);
    pixDestroy(&pixc2);
    pixDestroy(&pixc3);
    pixDestroy(&pixcs1);
    pixDestroy(&pixcs2);
    pixDestroy(&pixcs3);
    return regTestCleanup(rp);
}


static void
MakePtas(l_int32  i,
         l_int32  npts,  /* 3 or 4 */
         PTA    **pptas,
         PTA    **pptad)
{

    *pptas = ptaCreate(npts);
    ptaAddPt(*pptas, x1[i], y1[i]);
    ptaAddPt(*pptas, x2[i], y2[i]);
    ptaAddPt(*pptas, x3[i], y3[i]);
    if (npts == 4) ptaAddPt(*pptas, x4[i], y4[i]);
    *pptad = ptaCreate(npts);
    ptaAddPt(*pptad, xp1[i], yp1[i]);
    ptaAddPt(*pptad, xp2[i], yp2[i]);
    ptaAddPt(*pptad, xp3[i], yp3[i]);
    if (npts == 4) ptaAddPt(*pptad, xp4[i], yp4[i]);
    return;
}
