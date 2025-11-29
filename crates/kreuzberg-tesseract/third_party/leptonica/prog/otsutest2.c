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
 *   otsutest2.c
 *
 *   This demonstrates the usefulness of the modified version of Otsu
 *   for thresholding an image that doesn't have a well-defined
 *   background color.
 *
 *   Standard Otsu binarization is done with scorefract = 0.0, which
 *   returns the threshold at the maximum value of the score.  However.
 *   this value is up on the shoulder of the background, and its
 *   use causes some of the dark background to be binarized as foreground.
 *
 *   Using the modified Otsu with scorefract = 0.1 returns a threshold
 *   at the lowest value of this histogram such that the score
 *   is at least 0.9 times the maximum value of the score.  This allows
 *   the threshold to be taken in the histogram minimum between
 *   the fg and bg peaks, producing a much cleaner binarization.
 */


#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"


int main(int    argc,
         char **argv)
{
char       textstr[256];
l_int32    i, thresh, fgval, bgval;
l_float32  scorefract;
L_BMF     *bmf;
PIX       *pixs, *pixb, *pixg, *pixp, *pix1, *pix2, *pix3;
PIXA      *pixa1, *pixad;

    setLeptDebugOK(1);
    lept_mkdir("lept/otsu");

    pixs = pixRead("1555.007.jpg");
    pixg = pixConvertTo8(pixs, 0);
    bmf = bmfCreate(NULL, 8);
    pixad = pixaCreate(0);
    for (i = 0; i < 3; i++) {
        pixa1 = pixaCreate(2);
        scorefract = 0.1 * i;

            /* Get a 1 bpp version; use a single tile */
        pixOtsuAdaptiveThreshold(pixg, 2000, 2000, 0, 0, scorefract,
                                 NULL, &pixb);
        pixaAddPix(pixa1, pixg, L_COPY);
        pixaAddPix(pixa1, pixb, L_INSERT);

            /* Show the histogram of gray values and the split location */
        pixSplitDistributionFgBg(pixg, scorefract, 1,
                                 &thresh, &fgval, &bgval, &pixp);
        lept_stderr("thresh = %d, fgval = %d, bgval = %d\n",
                    thresh, fgval, bgval);
        pixaAddPix(pixa1, pixp, L_INSERT);

            /* Join these together and add some text */
        pix1 = pixaDisplayTiledInColumns(pixa1, 3, 1.0, 20, 2);
        snprintf(textstr, sizeof(textstr),
             "Scorefract = %3.1f ........... Thresh = %d", scorefract, thresh);
        pix2 = pixAddSingleTextblock(pix1, bmf, textstr, 0x00ff0000,
                                     L_ADD_BELOW, NULL);

            /* Save and display the result */
        pixaAddPix(pixad, pix2, L_INSERT);
        snprintf(textstr, sizeof(textstr), "/tmp/lept/otsu/%03d.png", i);
        pixWrite(textstr, pix2, IFF_PNG);
        pixDisplay(pix2, 100, 100);
        pixDestroy(&pix1);
        pixaDestroy(&pixa1);
    }

        /* Use a smaller tile for Otsu */
    for (i = 0; i < 2; i++) {
        scorefract = 0.1 * i;
        pixOtsuAdaptiveThreshold(pixg, 300, 300, 0, 0, scorefract,
                                 NULL, &pixb);
        pix1 = pixAddBlackOrWhiteBorder(pixb, 2, 2, 2, 2, L_GET_BLACK_VAL);
        pix2 = pixScale(pix1, 0.5, 0.5);
        snprintf(textstr, sizeof(textstr),
             "Scorefract = %3.1f", scorefract);
        pix3 = pixAddSingleTextblock(pix2, bmf, textstr, 1,
                                     L_ADD_BELOW, NULL);
        pixaAddPix(pixad, pix3, L_INSERT);
        pixDestroy(&pixb);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }

    lept_stderr("Writing to: /tmp/lept/otsu/result1.pdf\n");
    pixaConvertToPdf(pixad, 75, 1.0, 0, 0, "Otsu thresholding",
                     "/tmp/lept/otsu/result1.pdf");
    bmfDestroy(&bmf);
    pixDestroy(&pixs);
    pixDestroy(&pixg);
    pixaDestroy(&pixad);
    return 0;
}
