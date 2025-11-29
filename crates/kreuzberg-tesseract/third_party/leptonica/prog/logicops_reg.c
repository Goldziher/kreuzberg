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
 * logicops_reg.c
 *
 *    Regression test for pixel-wise logical operations, both in-place and
 *    generating new images.  Implemented by rasterops.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
PIX          *pixs, *pix1, *pix2, *pix3, *pix4;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("test1.png");


        /* pixInvert */
    pix1 = pixInvert(NULL, pixs);
    pix2 = pixCreateTemplate(pixs);  /* into pixd of same size */
    pixInvert(pix2, pixs);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    regTestComparePix(rp, pix1, pix2);  /* 1 */

    pix3 = pixRead("marge.jpg");  /* into pixd of different size */
    pixInvert(pix3, pixs);
    regTestComparePix(rp, pix1, pix3);  /* 2 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    pix1 = pixOpenBrick(NULL, pixs, 1, 9);
    pix2 = pixDilateBrick(NULL, pixs, 1, 9);

        /* pixOr */
    pix3 = pixCreateTemplate(pixs);
    pixOr(pix3, pixs, pix1);  /* existing */
    pix4 = pixOr(NULL, pixs, pix1);  /* new */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 3 */
    regTestComparePix(rp, pix3, pix4);  /* 4 */
    pixCopy(pix4, pix1);
    pixOr(pix4, pix4, pixs);  /* in-place */
    regTestComparePix(rp, pix3, pix4);  /* 5 */
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pix3 = pixCreateTemplate(pixs);
    pixOr(pix3, pixs, pix2);  /* existing */
    pix4 = pixOr(NULL, pixs, pix2);  /* new */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 6 */
    regTestComparePix(rp, pix3, pix4);  /* 7 */
    pixCopy(pix4, pix2);
    pixOr(pix4, pix4, pixs);  /* in-place */
    regTestComparePix(rp, pix3, pix4);  /* 8 */
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* pixAnd */
    pix3 = pixCreateTemplate(pixs);
    pixAnd(pix3, pixs, pix1);  /* existing */
    pix4 = pixAnd(NULL, pixs, pix1);  /* new */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 9 */
    regTestComparePix(rp, pix3, pix4);  /* 10 */
    pixCopy(pix4, pix1);
    pixAnd(pix4, pix4, pixs);  /* in-place */
    regTestComparePix(rp, pix3, pix4);  /* 11 */
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pix3 = pixCreateTemplate(pixs);
    pixAnd(pix3, pixs, pix2);  /* existing */
    pix4 = pixAnd(NULL, pixs, pix2);  /* new */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 12 */
    regTestComparePix(rp, pix3, pix4);  /* 13 */
    pixCopy(pix4, pix2);
    pixAnd(pix4, pix4, pixs);  /* in-place */
    regTestComparePix(rp, pix3, pix4);  /* 14 */
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* pixXor */
    pix3 = pixCreateTemplate(pixs);
    pixXor(pix3, pixs, pix1);  /* existing */
    pix4 = pixXor(NULL, pixs, pix1);  /* new */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 15 */
    regTestComparePix(rp, pix3, pix4);  /* 16 */
    pixCopy(pix4, pix1);
    pixXor(pix4, pix4, pixs);  /* in-place */
    regTestComparePix(rp, pix3, pix4);  /* 17 */
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pix3 = pixCreateTemplate(pixs);
    pixXor(pix3, pixs, pix2);  /* existing */
    pix4 = pixXor(NULL, pixs, pix2);  /* new */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 18 */
    regTestComparePix(rp, pix3, pix4);  /* 19 */
    pixCopy(pix4, pix2);
    pixXor(pix4, pix4, pixs);  /* in-place */
    regTestComparePix(rp, pix3, pix4);  /* 20 */
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* pixSubtract */
    pix3 = pixCreateTemplate(pixs);
    pixSubtract(pix3, pixs, pix1);  /* existing */
    pix4 = pixSubtract(NULL, pixs, pix1);  /* new */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 21 */
    regTestComparePix(rp, pix3, pix4);  /* 22 */
    pixCopy(pix4, pix1);
    pixSubtract(pix4, pixs, pix4);  /* in-place */
    regTestComparePix(rp, pix3, pix4);  /* 23 */
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pix3 = pixCreateTemplate(pixs);
    pixSubtract(pix3, pixs, pix2);  /* existing */
    pix4 = pixSubtract(NULL, pixs, pix2);  /* new */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 24 */
    regTestComparePix(rp, pix3, pix4);  /* 25 */
    pixCopy(pix4, pix2);
    pixSubtract(pix4, pixs, pix4);  /* in-place */
    regTestComparePix(rp, pix3, pix4);  /* 26 */
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pix4 = pixRead("marge.jpg");
    pixSubtract(pix4, pixs, pixs);  /* subtract from itself; should be empty */
    pix3 = pixCreateTemplate(pixs);
    regTestComparePix(rp, pix3, pix4);  /* 27*/
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pixSubtract(pixs, pixs, pixs);  /* subtract from itself; should be empty */
    pix3 = pixCreateTemplate(pixs);
    regTestComparePix(rp, pix3, pixs);  /* 28*/
    pixDestroy(&pix3);

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    return regTestCleanup(rp);
}

