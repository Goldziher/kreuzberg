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
 * comparetest.c
 *
 *      comparetest filein1 filein2 type fileout
 *
 *    where
 *      type = {0, 1} for {abs-diff and subtraction} comparisons
 *
 *    Compares two images, using either the absolute value of the
 *    pixel differences or the difference clipped to 0.  For RGB,
 *    the differences are computed separately on each component.
 *    If one has a colormap and the other doesn't, the colormap
 *    is removed before making the comparison.
 *
 *    Warning: you usually want to use abs-diff to compare
 *    two grayscale or color images.  If you use subtraction,
 *    the result you get will depend on the order of the input images.
 *    For example, if pix2 = pixDilateGray(pix1), then every
 *    pixel in pix1 will be equal to or greater than pix2.  So if
 *    you subtract pix2 from pix1, you will get 0 for all pixels,
 *    which looks like they're the same!
 *
 *    Here's an interesting observation.  Take an image that has
 *    been jpeg compressed at a quality = 75.  If you re-compress
 *    the image, what quality factor should be used to minimize
 *    the change?  Answer:  75 (!)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32    type, comptype, d1, d2, same, first, last;
l_float32  fract, diff, rmsdiff;
char      *filein1, *filein2, *fileout;
GPLOT     *gplot;
NUMA      *na1, *na2;
PIX       *pixs1, *pixs2, *pixd;

    if (argc != 5)
        return ERROR_INT(" Syntax:  comparetest filein1 filein2 type fileout",
                         __func__, 1);
    filein1 = argv[1];
    filein2 = argv[2];
    type = atoi(argv[3]);
    pixd = NULL;
    fileout = argv[4];
    setLeptDebugOK(1);

        /* If comparing image files with 16 bps and spp > 1,
         * comment this line out to strip 16 --> 8 spp */
    l_pngSetReadStrip16To8(0);

    if ((pixs1 = pixRead(filein1)) == NULL)
        return ERROR_INT("pixs1 not made", __func__, 1);
    if ((pixs2 = pixRead(filein2)) == NULL)
        return ERROR_INT("pixs2 not made", __func__, 1);
    d1 = pixGetDepth(pixs1);
    d2 = pixGetDepth(pixs2);

    if (d1 == 1 && d2 == 1) {
        pixEqual(pixs1, pixs2, &same);
        if (same) {
            lept_stderr("Images are identical\n");
            pixd = pixCreateTemplate(pixs1);  /* write empty pix for diff */
        } else {
            if (type == 0)
                comptype = L_COMPARE_XOR;
            else
                comptype = L_COMPARE_SUBTRACT;
            pixCompareBinary(pixs1, pixs2, comptype, &fract, &pixd);
            lept_stderr("Fraction of different pixels: %10.6f\n", fract);
        }
        pixWrite(fileout, pixd, IFF_PNG);
    } else {
        if (type == 0)
            comptype = L_COMPARE_ABS_DIFF;
        else
            comptype = L_COMPARE_SUBTRACT;
        pixCompareGrayOrRGB(pixs1, pixs2, comptype, GPLOT_PNG, &same, &diff,
                            &rmsdiff, &pixd);
        if (type == 0) {
            if (same) {
                lept_stderr("Images are identical\n");
            } else {
                lept_stderr("Images differ: <diff> = %10.6f\n", diff);
                lept_stderr("               <rmsdiff> = %10.6f\n", rmsdiff);
            }
        }
        else {  /* subtraction */
            if (same) {
                lept_stderr("pixs2 strictly greater than pixs1\n");
            } else {
                lept_stderr("Images differ: <diff> = %10.6f\n", diff);
                lept_stderr("               <rmsdiff> = %10.6f\n", rmsdiff);
            }
        }
        if (d1 != 16)
            pixWrite(fileout, pixd, IFF_JFIF_JPEG);
        else
            pixWrite(fileout, pixd, IFF_PNG);

        if (d1 != 16 && !same) {
            na1 = pixCompareRankDifference(pixs1, pixs2, 1);
            if (na1) {
                numaGetNonzeroRange(na1, 0.00005, &first, &last);
                lept_stderr("Nonzero diff range: first = %d, last = %d\n",
                             first, last);
                na2 = numaClipToInterval(na1, first, last);
                gplot = gplotCreate("/tmp/lept/comp/rank", GPLOT_PNG,
                                    "Pixel Rank Difference",
                                    "pixel val difference", "rank");
                gplotAddPlot(gplot, NULL, na2, GPLOT_LINES, "rank");
                gplotMakeOutput(gplot);
                gplotDestroy(&gplot);
                l_fileDisplay("/tmp/lept/comp/rank.png", 100, 100, 1.0);
                numaDestroy(&na1);
                numaDestroy(&na2);
            }
        }
    }

    pixDestroy(&pixs1);
    pixDestroy(&pixs2);
    pixDestroy(&pixd);
    return 0;
}
