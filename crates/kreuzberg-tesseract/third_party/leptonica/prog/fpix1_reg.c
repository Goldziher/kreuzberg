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
 *  fpix1_reg.c
 *
 *    Regression test for a number of functions in the FPix utility.
 *    FPix allows you to do floating point operations such as
 *    convolution, with conversions to and from Pix.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

static void MakePtasAffine(l_int32 i, PTA **pptas, PTA **pptad);
static void MakePtas(l_int32 i, PTA **pptas, PTA **pptad);

static const l_int32  xs1[] =  { 300,  300, 1100,  300,   32};
static const l_int32  ys1[] =  {1200, 1200, 1200, 1250,  934};
static const l_int32  xs2[] =  {1200, 1200,  325, 1300,  487};
static const l_int32  ys2[] =  {1100, 1100, 1200, 1250,  934};
static const l_int32  xs3[] =  { 200,  200, 1200,  250,   32};
static const l_int32  ys3[] =  { 200,  200,  200,  300,   67};
static const l_int32  xs4[] =  {1200, 1200, 1100, 1250,  332};
static const l_int32  ys4[] =  { 400,  200,  200,  300,   57};

static const l_int32  xd1[] = { 300,  300, 1150,  300,   32};
static const l_int32  yd1[] = {1200, 1400, 1150, 1350,  934};
static const l_int32  xd2[] = {1100, 1400,  320, 1300,  487};
static const l_int32  yd2[] = {1000, 1500, 1300, 1200,  904};
static const l_int32  xd3[] = { 250,  200, 1310,  300,   61};
static const l_int32  yd3[] = { 200,  300,  250,  325,   83};
static const l_int32  xd4[] = {1250, 1200, 1140, 1250,  412};
static const l_int32  yd4[] = { 300,  300,  250,  350,   83};


int main(int    argc,
         char **argv)
{
l_float32     sum, sumx, sumy, diff;
L_DEWARP     *dew;
L_DEWARPA    *dewa;
FPIX         *fpixs, *fpixs2, *fpixs3, *fpixs4, *fpixg, *fpixd;
FPIX         *fpix1, *fpix2;
DPIX         *dpix, *dpix2;
L_KERNEL     *kel, *kelx, *kely;
PIX          *pixs, *pixs2, *pixs3, *pixd, *pixg, *pixb, *pixn;
PIX          *pix0, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIXA         *pixa;
PTA          *ptas, *ptad;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "fpix1_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Gaussian kernel */
    pixa = pixaCreate(0);
    kel = makeGaussianKernel(5, 5, 3.0, 4.0);
    kernelGetSum(kel, &sum);
    if (rp->display) lept_stderr("Sum for 2d gaussian kernel = %f\n", sum);
    pix0 = kernelDisplayInPix(kel, 41, 2);
    regTestWritePixAndCheck(rp, pix0, IFF_PNG);  /* 0 */
    pixaAddPix(pixa, pix0, L_INSERT);

        /* Separable gaussian kernel */
    makeGaussianKernelSep(5, 5, 3.0, 4.0, &kelx, &kely);
    kernelGetSum(kelx, &sumx);
    if (rp->display) lept_stderr("Sum for x gaussian kernel = %f\n", sumx);
    kernelGetSum(kely, &sumy);
    if (rp->display) lept_stderr("Sum for y gaussian kernel = %f\n", sumy);
    if (rp->display) lept_stderr("Sum for x * y gaussian kernel = %f\n",
                         sumx * sumy);
    pix0 = kernelDisplayInPix(kelx, 41, 2);
    regTestWritePixAndCheck(rp, pix0, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pix0, L_INSERT);
    pix0 = kernelDisplayInPix(kely, 41, 2);
    regTestWritePixAndCheck(rp, pix0, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pix0, L_INSERT);
    pix0 = pixaDisplayTiledInColumns(pixa, 4, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pix0, IFF_PNG);  /* 3 */
    pixaDestroy(&pixa);
    pixDestroy(&pix0);

        /* Use pixRasterop() to generate source image */
    pixa = pixaCreate(0);
    pixs = pixRead("test8.jpg");
    pixs2 = pixRead("karen8.jpg");
    pixRasterop(pixs, 150, 125, 150, 100, PIX_SRC, pixs2, 75, 100);
    regTestWritePixAndCheck(rp, pixs, IFF_JFIF_JPEG);  /* 4 */

        /* Convolution directly with pix */
    pix1 = pixConvolve(pixs, kel, 8, 1);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 5 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixConvolveSep(pixs, kelx, kely, 8, 1);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 6 */
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Convolution indirectly with fpix, using fpixRasterop()
         * to generate the source image. */
    fpixs = pixConvertToFPix(pixs, 3);
    fpixs2 = pixConvertToFPix(pixs2, 3);
    fpixRasterop(fpixs, 150, 125, 150, 100, fpixs2, 75, 100);
    fpix1 = fpixConvolve(fpixs, kel, 1);
    pix3 = fpixConvertToPix(fpix1, 8, L_CLIP_TO_ZERO, 1);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 7 */
    pixaAddPix(pixa, pix3, L_INSERT);
    fpix2 = fpixConvolveSep(fpixs, kelx, kely, 1);
    pix4 = fpixConvertToPix(fpix2, 8, L_CLIP_TO_ZERO, 1);
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 8 */
    pixaAddPix(pixa, pix4, L_INSERT);
    pixDestroy(&pixs2);
    fpixDestroy(&fpixs2);
    fpixDestroy(&fpix1);
    fpixDestroy(&fpix2);

        /* Comparison of results */
    if (rp->display) {
        pixCompareGray(pix1, pix2, L_COMPARE_ABS_DIFF, 0, NULL,
                       &diff, NULL, NULL);
        lept_stderr("Ave diff of pixConvolve and pixConvolveSep: %f\n", diff);
        pixCompareGray(pix3, pix4, L_COMPARE_ABS_DIFF, 0, NULL,
                       &diff, NULL, NULL);
        lept_stderr("Ave diff of fpixConvolve and fpixConvolveSep: %f\n", diff);
        pixCompareGray(pix1, pix3, L_COMPARE_ABS_DIFF, 0, NULL,
                       &diff, NULL, NULL);
        lept_stderr("Ave diff of pixConvolve and fpixConvolve: %f\n", diff);
    }
    pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 9 */
    pixaDestroy(&pixa);
    pixDestroy(&pix1);

        /* Test arithmetic operations; add in a fraction rotated by 180 */
    pixa = pixaCreate(0);
    pixs3 = pixRotate180(NULL, pixs);
    regTestWritePixAndCheck(rp, pixs3, IFF_JFIF_JPEG);  /* 10 */
    pixaAddPix(pixa, pixs3, L_INSERT);
    fpixs3 = pixConvertToFPix(pixs3, 3);
    fpixd = fpixLinearCombination(NULL, fpixs, fpixs3, 20.0, 5.0);
    fpixAddMultConstant(fpixd, 0.0, 23.174);   /* multiply up in magnitude */
    pixd = fpixDisplayMaxDynamicRange(fpixd);  /* bring back to 8 bpp */
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 11 */
    pixaAddPix(pixa, pixd, L_INSERT);
    fpixDestroy(&fpixs3);
    fpixDestroy(&fpixd);
    pixDestroy(&pixs);
    fpixDestroy(&fpixs);

        /* Display results */
    pixd = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 12 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

        /* Test some more convolutions, with sampled output. First on pix */
    pixa = pixaCreate(0);
    pixs = pixRead("1555.007.jpg");
    pixg = pixConvertTo8(pixs, 0);
    l_setConvolveSampling(5, 5);
    pix1 = pixConvolve(pixg, kel, 8, 1);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 13 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixConvolveSep(pixg, kelx, kely, 8, 1);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 14 */
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixConvolveRGB(pixs, kel);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);  /* 15 */
    pixaAddPix(pixa, pix3, L_INSERT);
    pix4 = pixConvolveRGBSep(pixs, kelx, kely);
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);  /* 16 */
    pixaAddPix(pixa, pix4, L_INSERT);

        /* Then on fpix */
    fpixg = pixConvertToFPix(pixg, 1);
    fpix1 = fpixConvolve(fpixg, kel, 1);
    pix5 = fpixConvertToPix(fpix1, 8, L_CLIP_TO_ZERO, 0);
    regTestWritePixAndCheck(rp, pix5, IFF_JFIF_JPEG);  /* 17 */
    pixaAddPix(pixa, pix5, L_INSERT);
    fpix2 = fpixConvolveSep(fpixg, kelx, kely, 1);
    pix6 = fpixConvertToPix(fpix2, 8, L_CLIP_TO_ZERO, 0);
    regTestWritePixAndCheck(rp, pix6, IFF_JFIF_JPEG);  /* 18 */
    pixaAddPix(pixa, pix6, L_INSERT);
    regTestCompareSimilarPix(rp, pix1, pix5, 2, 0.00, 0);  /* 19 */
    regTestCompareSimilarPix(rp, pix2, pix6, 2, 0.00, 0);  /* 20 */
    fpixDestroy(&fpixg);
    fpixDestroy(&fpix1);
    fpixDestroy(&fpix2);

    pixd = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 21 */
    pixDisplayWithTitle(pixd, 600, 100, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixg);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

        /* Test extension (continued and slope).
         * First, build a smooth vertical disparity array;
         * then extend and show the contours. */
    pixs = pixRead("cat.035.jpg");
    pixn = pixBackgroundNormSimple(pixs, NULL, NULL);
    pixg = pixConvertRGBToGray(pixn, 0.5, 0.3, 0.2);
    pixb = pixThresholdToBinary(pixg, 130);
    dewa = dewarpaCreate(1, 30, 1, 15, 0);
    if ((dew = dewarpCreate(pixb, 35)) == NULL) {
        rp->success = FALSE;
        L_ERROR("dew not made; tests 21-28 skipped (failed)\n", "fpix1_reg");
        return regTestCleanup(rp);
    }
    dewarpaInsertDewarp(dewa, dew);
    dewarpBuildPageModel(dew, NULL);  /* two invalid indices in ptaGetPt() */
    dewarpPopulateFullRes(dew, NULL, 0, 0);
    fpixs = dew->fullvdispar;
    fpixs2 = fpixAddContinuedBorder(fpixs, 200, 200, 100, 300);
    fpixs3 = fpixAddSlopeBorder(fpixs, 200, 200, 100, 300);
    dpix = fpixConvertToDPix(fpixs3);
    fpixs4 = dpixConvertToFPix(dpix);
    pix1 = fpixRenderContours(fpixs, 2.0, 0.2);
    pix2 = fpixRenderContours(fpixs2, 2.0, 0.2);
    pix3 = fpixRenderContours(fpixs3, 2.0, 0.2);
    pix4 = fpixRenderContours(fpixs4, 2.0, 0.2);
    pix5 = pixRead("karen8.jpg");
    dpix2 = pixConvertToDPix(pix5, 1);
    pix6 = dpixConvertToPix(dpix2, 8, L_CLIP_TO_ZERO, 0);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 22 */
    pixDisplayWithTitle(pix1, 0, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 23 */
    pixDisplayWithTitle(pix2, 470, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 24 */
    pixDisplayWithTitle(pix3, 1035, 100, NULL, rp->display);
    regTestComparePix(rp, pix3, pix4);  /* 25 */
    regTestComparePix(rp, pix5, pix6);  /* 26 */
    pixDestroy(&pixs);
    pixDestroy(&pixn);
    pixDestroy(&pixg);
    pixDestroy(&pixb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    fpixDestroy(&fpixs2);
    fpixDestroy(&fpixs3);
    fpixDestroy(&fpixs4);
    dpixDestroy(&dpix);
    dpixDestroy(&dpix2);

        /* Test affine and projective transforms on fpix */
    fpixWrite("/tmp/lept/regout/fpix1.fp", dew->fullvdispar);
    fpix1 = fpixRead("/tmp/lept/regout/fpix1.fp");
    pix1 = fpixAutoRenderContours(fpix1, 40);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 27 */
    pixDisplayWithTitle(pix1, 0, 500, NULL, rp->display);
    pixDestroy(&pix1);

    MakePtasAffine(1, &ptas, &ptad);
    fpix2 = fpixAffinePta(fpix1, ptad, ptas, 200, 0.0);
    pix2 = fpixAutoRenderContours(fpix2, 40);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 28 */
    pixDisplayWithTitle(pix2, 400, 500, NULL, rp->display);
    fpixDestroy(&fpix2);
    pixDestroy(&pix2);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);

    MakePtas(1, &ptas, &ptad);
    fpix2 = fpixProjectivePta(fpix1, ptad, ptas, 200, 0.0);
    pix3 = fpixAutoRenderContours(fpix2, 40);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 29 */
    pixDisplayWithTitle(pix3, 400, 500, NULL, rp->display);
    fpixDestroy(&fpix1);
    fpixDestroy(&fpix2);
    pixDestroy(&pix3);
    ptaDestroy(&ptas);
    ptaDestroy(&ptad);
    dewarpaDestroy(&dewa);

    kernelDestroy(&kel);
    kernelDestroy(&kelx);
    kernelDestroy(&kely);
    return regTestCleanup(rp);
}


static void
MakePtas(l_int32  i,
         PTA    **pptas,
         PTA    **pptad)
{
PTA  *ptas, *ptad;
    ptas = ptaCreate(4);
    ptaAddPt(ptas, xs1[i], ys1[i]);
    ptaAddPt(ptas, xs2[i], ys2[i]);
    ptaAddPt(ptas, xs3[i], ys3[i]);
    ptaAddPt(ptas, xs4[i], ys4[i]);
    ptad = ptaCreate(4);
    ptaAddPt(ptad, xd1[i], yd1[i]);
    ptaAddPt(ptad, xd2[i], yd2[i]);
    ptaAddPt(ptad, xd3[i], yd3[i]);
    ptaAddPt(ptad, xd4[i], yd4[i]);
    *pptas = ptas;
    *pptad = ptad;
    return;
}

static void
MakePtasAffine(l_int32  i,
               PTA    **pptas,
               PTA    **pptad)
{
PTA  *ptas, *ptad;
    ptas = ptaCreate(3);
    ptaAddPt(ptas, xs1[i], ys1[i]);
    ptaAddPt(ptas, xs2[i], ys2[i]);
    ptaAddPt(ptas, xs3[i], ys3[i]);
    ptad = ptaCreate(3);
    ptaAddPt(ptad, xd1[i], yd1[i]);
    ptaAddPt(ptad, xd2[i], yd2[i]);
    ptaAddPt(ptad, xd3[i], yd3[i]);
    *pptas = ptas;
    *pptad = ptad;
    return;
}
