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
 *  fpix2_reg.c
 *
 *    Regression test for FPix:
 *       - rotation by multiples of 90 degrees
 *       - adding borders of various types
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
FPIX         *fpix1, *fpix2, *fpix3, *fpix4;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7, *pix8;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Test orthogonal rotations */
    pix1 = pixRead("marge.jpg");
    pix2 = pixConvertTo8(pix1, 0);
    fpix1 = pixConvertToFPix(pix2, 1);

    fpix2 = fpixRotateOrth(fpix1, 1);
    pix3 = fpixConvertToPix(fpix2, 8, L_CLIP_TO_ZERO, 0);
    pix4 = pixRotateOrth(pix2, 1);
    regTestComparePix(rp, pix3, pix4);  /* 0 */
    pixDisplayWithTitle(pix3, 100, 100, NULL, rp->display);

    fpix3 = fpixRotateOrth(fpix1, 2);
    pix5 = fpixConvertToPix(fpix3, 8, L_CLIP_TO_ZERO, 0);
    pix6 = pixRotateOrth(pix2, 2);
    regTestComparePix(rp, pix5, pix6);  /* 1 */
    pixDisplayWithTitle(pix5, 560, 100, NULL, rp->display);

    fpix4 = fpixRotateOrth(fpix1, 3);
    pix7 = fpixConvertToPix(fpix4, 8, L_CLIP_TO_ZERO, 0);
    pix8 = pixRotateOrth(pix2, 3);
    regTestComparePix(rp, pix7, pix8);  /* 2 */
    pixDisplayWithTitle(pix7, 1170, 100, NULL, rp->display);
    pixDisplayWithTitle(pix2, 560, 580, NULL, rp->display);

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixDestroy(&pix8);
    fpixDestroy(&fpix1);
    fpixDestroy(&fpix2);
    fpixDestroy(&fpix3);
    fpixDestroy(&fpix4);

        /* Test adding various borders */
    pix1 = pixRead("marge.jpg");
    pix2 = pixConvertTo8(pix1, 0);
    fpix1 = pixConvertToFPix(pix2, 1);

    fpix2 = fpixAddMirroredBorder(fpix1, 21, 21, 25, 25);
    pix3 = fpixConvertToPix(fpix2, 8, L_CLIP_TO_ZERO, 0);
    pix4 = pixAddMirroredBorder(pix2, 21, 21, 25, 25);
    regTestComparePix(rp, pix3, pix4);  /* 3 */
    pixDisplayWithTitle(pix3, 100, 1000, NULL, rp->display);

    fpix3 = fpixAddContinuedBorder(fpix1, 21, 21, 25, 25);
    pix5 = fpixConvertToPix(fpix3, 8, L_CLIP_TO_ZERO, 0);
    pix6 = pixAddContinuedBorder(pix2, 21, 21, 25, 25);
    regTestComparePix(rp, pix5, pix6);  /* 4 */
    pixDisplayWithTitle(pix5, 750, 1000, NULL, rp->display);

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    fpixDestroy(&fpix1);
    fpixDestroy(&fpix2);
    fpixDestroy(&fpix3);
    return regTestCleanup(rp);
}

