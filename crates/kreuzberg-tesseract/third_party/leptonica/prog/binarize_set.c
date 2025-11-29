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
 * binarize_set.c
 *
 *   Does 5 different types of binarization for the contest.
 *
 *   Method 1.  Using local background normalization, followed by
 *              a global threshold.
 *   Method 2.  Using local background normalization, followed by
 *              Otsu on the result to get a global threshold that
 *              can be applied to the normalized image.
 *   Method 3.  Using local background normalization with two different
 *              thresholds.  For the part of the image near the text,
 *              a high threshold can be chosen, to render the text
 *              fully in black.  For the rest of the image,
 *              much of which is background, use a threshold based on
 *              the Otsu global value of the original image.
 *   Method 4.  Contrast normalization followed by Sauvola binarization.
 *   Method 5.  Contrast normalization followed by background normalization
 *              and thresholding.
 *
 *   The first 3 were submitted to a binarization contest associated
 *   with ICDAR in 2009.  The 4th and 5th work better for difficult
 *   images, such as w91frag.jpg.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  ALL    1


int main(int    argc,
         char **argv)
{
char     *infile;
l_int32   w, d, threshval;
PIX      *pixs, *pixg;
PIX      *pix1, *pix2;
PIXA     *pixa;

    if (argc != 2)
        return ERROR_INT(" Syntax: binarize_set infile", __func__, 1);
    infile = argv[1];

    setLeptDebugOK(1);
    lept_mkdir("lept/binar");

    pixa = pixaCreate(5);
    pixs = pixRead(infile);
    pixGetDimensions(pixs, &w, NULL, &d);
    pixaAddPix(pixa, pixs, L_INSERT);
    pixDisplay(pixs, 100, 0);

#if ALL
    /* 1. Standard background normalization with a global threshold.  */
    pixg = pixConvertTo8(pixs, 0);
    pix1 = pixBackgroundNorm(pixg, NULL, NULL, 10, 15, 100, 50, 255, 2, 2);
    pix2 = pixThresholdToBinary(pix1, 160);
    pixWrite("/tmp/lept/binar/binar1.png", pix2, IFF_PNG);
    pixDisplay(pix2, 100, 0);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pixg);
    pixDestroy(&pix1);
#endif

#if ALL
    /* 2. Background normalization followed by Otsu thresholding.  Otsu
     * binarization attempts to split the image into two roughly equal
     * sets of pixels, and it does a very poor job when there are large
     * amounts of dark background.  By doing a background normalization
     * first (to get the background near 255), we remove this problem.
     * Then we use a modified Otsu to estimate the best global
     * threshold on the normalized image.  */
    pixg = pixConvertTo8(pixs, 0);
    pix1 = pixOtsuThreshOnBackgroundNorm(pixg, NULL, 10, 15, 100,
                                    50, 255, 2, 2, 0.10, &threshval);
    lept_stderr("thresh val = %d\n", threshval);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixWrite("/tmp/lept/binar/binar2.png", pix1, IFF_PNG);
    pixDisplay(pix1, 100, 200);
    pixDestroy(&pixg);
#endif

#if ALL
    /* 3. Background normalization with Otsu threshold estimation and
     * masking for threshold selection.  */
    pixg = pixConvertTo8(pixs, 0);
    pix1 = pixMaskedThreshOnBackgroundNorm(pixg, NULL, 10, 15, 100,
                                           50, 2, 2, 0.10, &threshval);
    lept_stderr("thresh val = %d\n", threshval);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixWrite("/tmp/lept/binar/binar3.png", pix1, IFF_PNG);
    pixDisplay(pix1, 100, 400);
    pixDestroy(&pixg);
#endif

#if ALL
    /* 4. Contrast normalization followed by Sauvola binarization */
    pix1 = pixSauvolaOnContrastNorm(pixs, 130, NULL, NULL);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixWrite("/tmp/lept/binar/binar4.png", pix1, IFF_PNG);
    pixDisplay(pix1, 100, 600);
#endif

#if ALL
    /* 5. Contrast normalization followed by background normalization, and
     * thresholding. */
    pix1 = pixThreshOnDoubleNorm(pixs, 130);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixWrite("/tmp/lept/binar/binar5.png", pix1, IFF_PNG);
    pixDisplay(pix1, 100, 800);
#endif

    pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 30, 2);
    pixWrite("/tmp/lept/binar/binar6.png", pix1, IFF_PNG);
    pixDisplay(pix1, 1000, 0);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);

    return 0;
}
