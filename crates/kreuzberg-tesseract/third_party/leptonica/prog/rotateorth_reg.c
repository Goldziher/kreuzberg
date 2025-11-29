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
 * rotateorth_reg.c
 *
 *    Regression test for all rotateorth functions
 */

#include "allheaders.h"

#define   BINARY_IMAGE        "test1.png"
#define   FOUR_BPP_IMAGE      "weasel4.8g.png"
#define   GRAYSCALE_IMAGE     "test8.jpg"
#define   COLORMAP_IMAGE      "dreyfus8.png"
#define   RGB_IMAGE           "marge.jpg"

void RotateOrthTest(PIX *pix, L_REGPARAMS *rp);


int main(int    argc,
         char **argv)
{
PIX          *pixs;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_stderr("\nTest binary image:\n");
    pixs = pixRead(BINARY_IMAGE);
    RotateOrthTest(pixs, rp);
    pixDestroy(&pixs);
    lept_stderr("\nTest 4 bpp colormapped image:\n");
    pixs = pixRead(FOUR_BPP_IMAGE);
    RotateOrthTest(pixs, rp);
    pixDestroy(&pixs);
    lept_stderr("\nTest grayscale image:\n");
    pixs = pixRead(GRAYSCALE_IMAGE);
    RotateOrthTest(pixs, rp);
    pixDestroy(&pixs);
    lept_stderr("\nTest colormap image:\n");
    pixs = pixRead(COLORMAP_IMAGE);
    RotateOrthTest(pixs, rp);
    pixDestroy(&pixs);
    lept_stderr("\nTest rgb image:\n");
    pixs = pixRead(RGB_IMAGE);
    RotateOrthTest(pixs, rp);
    pixDestroy(&pixs);

    return regTestCleanup(rp);
}


void
RotateOrthTest(PIX          *pixs,
               L_REGPARAMS  *rp)
{
l_int32   zero, count;
PIX      *pixt, *pixd;

	/* Test 4 successive 90 degree rotations */
    pixt = pixRotate90(pixs, 1);
    pixd = pixRotate90(pixt, 1);
    pixDestroy(&pixt);
    pixt = pixRotate90(pixd, 1);
    pixDestroy(&pixd);
    pixd = pixRotate90(pixt, 1);
    pixDestroy(&pixt);
    regTestComparePix(rp, pixs, pixd);
    pixXor(pixd, pixd, pixs);
    pixZero(pixd, &zero);
    if (zero) {
        lept_stderr("OK.  Four 90-degree rotations gives I\n");
    } else {
         pixCountPixels(pixd, &count, NULL);
         lept_stderr("Failure for four 90-degree rots; count = %d\n", count);
    }
    pixDestroy(&pixd);

	/* Test 2 successive 180 degree rotations */
    pixt = pixRotate180(NULL, pixs);
    pixRotate180(pixt, pixt);
    regTestComparePix(rp, pixs, pixt);
    pixXor(pixt, pixt, pixs);
    pixZero(pixt, &zero);
    if (zero) {
        lept_stderr("OK.  Two 180-degree rotations gives I\n");
    } else {
        pixCountPixels(pixt, &count, NULL);
        lept_stderr("Failure for two 180-degree rots; count = %d\n", count);
    }
    pixDestroy(&pixt);

	/* Test 2 successive LR flips */
    pixt = pixFlipLR(NULL, pixs);
    pixFlipLR(pixt, pixt);
    regTestComparePix(rp, pixs, pixt);
    pixXor(pixt, pixt, pixs);
    pixZero(pixt, &zero);
    if (zero) {
        lept_stderr("OK.  Two LR flips gives I\n");
    } else {
        pixCountPixels(pixt, &count, NULL);
        lept_stderr("Failure for two LR flips; count = %d\n", count);
    }
    pixDestroy(&pixt);

	/* Test 2 successive TB flips */
    pixt = pixFlipTB(NULL, pixs);
    pixFlipTB(pixt, pixt);
    regTestComparePix(rp, pixs, pixt);
    pixXor(pixt, pixt, pixs);
    pixZero(pixt, &zero);
    if (zero) {
        lept_stderr("OK.  Two TB flips gives I\n");
    } else {
        pixCountPixels(pixt, &count, NULL);
        lept_stderr("Failure for two TB flips; count = %d\n", count);
    }
    pixDestroy(&pixt);
    return;
}
