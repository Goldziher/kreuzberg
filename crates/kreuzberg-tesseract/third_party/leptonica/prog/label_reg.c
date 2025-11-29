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
 *   label_reg.c
 *
 *    Regression test for earthmover distance and these labelling operations:
 *         Connected component labelling
 *         Connected component area labelling
 *         Color coded transform of 1 bpp images
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void FindEMD(PIX *pix1, PIX *pix2, l_float32 *pdistr, l_float32 *pdistg,
             l_float32 *pdistb);

l_int32 main(int    argc,
             char **argv)
{
l_float32     dist, distr, distg, distb;
NUMA         *na1, *na2;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Test earthmover distance: extreme example */
    lept_stderr("Test earthmover distance\n");
    na1 = numaMakeConstant(0, 201);
    na2 = numaMakeConstant(0, 201);
    numaSetValue(na1, 0, 100);
    numaSetValue(na2, 200, 100);
    numaEarthMoverDistance(na1, na2, &dist);
    regTestCompareValues(rp, 200.0, dist, 0.0001);  /* 0 */
    numaDestroy(&na1);
    numaDestroy(&na2);

        /* Test connected component labelling */
    lept_stderr("Test c.c. labelling\n");
    pix1 = pixRead("feyn-fract.tif");
    pix2 = pixConnCompTransform(pix1, 8, 8);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix2, 0, 0, NULL, rp->display);
    pix3 = pixConnCompTransform(pix1, 8, 16);
    pix4 = pixConvert16To8(pix3, L_LS_BYTE);
    regTestCompareSimilarPix(rp, pix2, pix4, 3, 0.001, 0);  /* 2 */
    pix5 = pixConnCompTransform(pix1, 8, 32);
    pix6 = pixConvert32To8(pix5, L_LS_TWO_BYTES, L_LS_BYTE);
    regTestCompareSimilarPix(rp, pix2, pix6, 3, 0.001, 0);  /* 3 */
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);

        /* Test connected component area labelling */
    lept_stderr("Test c.c. area labelling\n");
    pix2 = pixConnCompAreaTransform(pix1, 8);
    pix3 = pixConvert32To8(pix2, L_LS_TWO_BYTES, L_CLIP_TO_FF);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix3, 0, 350, NULL, rp->display);
    pixMultConstantGray(pix2, 0.3);
    pix4 = pixConvert32To8(pix2, L_LS_TWO_BYTES, L_CLIP_TO_FF);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix4, 0, 700, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Test color transform: 4-fold symmetry */
    lept_stderr("Test color transform: 4-fold symmetry\n");
    pix1 = pixRead("form1.tif");
    pix2 = pixRotateOrth(pix1, 1);
    pix3 = pixRotateOrth(pix1, 2);
    pix4 = pixRotateOrth(pix1, 3);
    pix5 = pixLocToColorTransform(pix1);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 6 */
    pix6 = pixLocToColorTransform(pix2);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 7 */
    FindEMD(pix5, pix6, &distr, &distg, &distb);
    regTestCompareValues(rp, 0.12, distr, 0.01);  /* 8 */
    regTestCompareValues(rp, 0.00, distg, 0.01);  /* 9 */
    regTestCompareValues(rp, 0.00, distb, 0.01);  /* 10 */
    lept_stderr("90 deg rotation: dist (r,g,b) = (%5.2f, %5.2f, %5.2f)\n",
                distr, distg, distb);
    pixDestroy(&pix6);
    pix6 = pixLocToColorTransform(pix3);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 11 */
    FindEMD(pix5, pix6, &distr, &distg, &distb);
    regTestCompareValues(rp, 0.12, distr, 0.01);  /* 12 */
    regTestCompareValues(rp, 0.09, distg, 0.01);  /* 13 */
    regTestCompareValues(rp, 0.00, distb, 0.01);  /* 14 */
    lept_stderr("180 deg rotation: dist (r,g,b) = (%5.2f, %5.2f, %5.2f)\n",
                distr, distg, distb);
    pixDestroy(&pix6);
    pix6 = pixLocToColorTransform(pix4);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 15 */
    FindEMD(pix5, pix6, &distr, &distg, &distb);
    regTestCompareValues(rp, 0.00, distr, 0.01);  /* 16 */
    regTestCompareValues(rp, 0.09, distg, 0.01);  /* 17 */
    regTestCompareValues(rp, 0.00, distb, 0.01);  /* 18 */
    lept_stderr("270 deg rotation: dist (r,g,b) = (%5.2f, %5.2f, %5.2f)\n",
                distr, distg, distb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);

        /* Test color transform: same form with translation */
    lept_stderr("Test color transform with translation\n");
    pix1 = pixRead("form1.tif");
    pix2 = pixLocToColorTransform(pix1);
    pixDisplayWithTitle(pix2, 0, 0, NULL, rp->display);
    pixTranslate(pix1, pix1, 10, 10, L_BRING_IN_WHITE);
    pix3 = pixLocToColorTransform(pix1);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 19 */
    pixDisplayWithTitle(pix3, 470, 0, NULL, rp->display);
    FindEMD(pix2, pix3, &distr, &distg, &distb);
    regTestCompareValues(rp, 1.76, distr, 0.01);  /* 20 */
    regTestCompareValues(rp, 2.65, distg, 0.01);  /* 21 */
    regTestCompareValues(rp, 2.03, distb, 0.01);  /* 22 */
    lept_stderr("Translation dist (r,g,b) = (%5.2f, %5.2f, %5.2f)\n",
                distr, distg, distb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Test color transform: same form with small rotation */
    lept_stderr("Test color transform with small rotation\n");
    pix1 = pixRead("form1.tif");
    pix2 = pixLocToColorTransform(pix1);
    pixRotateShearCenterIP(pix1, 0.1, L_BRING_IN_WHITE);
    pix3 = pixLocToColorTransform(pix1);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 23 */
    pixDisplayWithTitle(pix3, 880, 0, NULL, rp->display);
    FindEMD(pix2, pix3, &distr, &distg, &distb);
    regTestCompareValues(rp, 1.50, distr, 0.01);  /* 24 */
    regTestCompareValues(rp, 1.71, distg, 0.01);  /* 25 */
    regTestCompareValues(rp, 1.42, distb, 0.01);  /* 26 */
    lept_stderr("Rotation dist (r,g,b) = (%5.2f, %5.2f, %5.2f)\n",
                distr, distg, distb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

        /* Test color transform: 2 different forms */
    lept_stderr("Test color transform (2 forms)\n");
    pix1 = pixRead("form1.tif");
    pix2 = pixLocToColorTransform(pix1);
    pixDisplayWithTitle(pix2, 0, 600, NULL, rp->display);
    pix3 = pixRead("form2.tif");
    pix4 = pixLocToColorTransform(pix3);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 27 */
    pixDisplayWithTitle(pix4, 470, 600, NULL, rp->display);
    FindEMD(pix2, pix4, &distr, &distg, &distb);
    regTestCompareValues(rp, 6.10, distr, 0.02);  /* 28 */
    regTestCompareValues(rp, 11.13, distg, 0.01);  /* 29 */
    regTestCompareValues(rp, 10.53, distb, 0.01);  /* 30 */
    lept_stderr("Different forms: dist (r,g,b) = (%5.2f, %5.2f, %5.2f)\n",
                distr, distg, distb);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    return regTestCleanup(rp);
}


void FindEMD(PIX *pix1, PIX *pix2,
             l_float32 *pdistr, l_float32 *pdistg, l_float32 *pdistb)
{
NUMA  *nar1, *nar2, *nag1, *nag2, *nab1, *nab2;

    pixGetColorHistogram(pix1, 1, &nar1, &nag1, &nab1);
    pixGetColorHistogram(pix2, 1, &nar2, &nag2, &nab2);
    numaEarthMoverDistance(nar1, nar2, pdistr);
    numaEarthMoverDistance(nag1, nag2, pdistg);
    numaEarthMoverDistance(nab1, nab2, pdistb);
    numaDestroy(&nar1);
    numaDestroy(&nar2);
    numaDestroy(&nag1);
    numaDestroy(&nag2);
    numaDestroy(&nab1);
    numaDestroy(&nab2);
    return;
}

