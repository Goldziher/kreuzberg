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
 * cmapquant_reg.c
 *
 *   Tests quantization of rgb image to a specific colormap.
 *   Does this by starting with a grayscale image, doing a grayscale
 *   quantization with a colormap in the dest, then adding new
 *   colors, scaling (which removes the colormap), and finally
 *   re-quantizing back to the original colormap.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  LEVEL       3
#define  MIN_DEPTH   4

int main(int    argc,
         char **argv)
{
BOX          *box;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;
PIX          *pix8, *pix9;
PIXCMAP      *cmap;
L_REGPARAMS  *rp;

   if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("lucasta-frag.jpg");

        /* Convert to 4 bpp with 6 levels and a colormap */
    pix1 = pixThresholdTo4bpp(pixs, 6, 1);

        /* Color some non-white pixels, preserving antialiasing, and
         * adding these colors to the colormap */
    box = boxCreate(120, 30, 200, 200);
    pixColorGray(pix1, box, L_PAINT_DARK, 220, 0, 0, 255);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    boxDestroy(&box);

        /* Scale up by 1.5; losing the colormap */
    pix2 = pixScale(pix1, 1.5, 1.5);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);  /* 1 */
    pixDisplayWithTitle(pix2, 0, 0, NULL, rp->display);

        /* Octcube quantize using the same colormap */
    startTimer();
    cmap = pixGetColormap(pix1);
    pix3 = pixOctcubeQuantFromCmap(pix2, cmap, MIN_DEPTH,
                                   LEVEL, L_EUCLIDEAN_DISTANCE);
    lept_stderr("Time to re-quantize to cmap = %7.3f sec\n", stopTimer());
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix3, 0, 0, NULL, rp->display);

        /* Convert the quantized image to rgb */
    pix4 = pixConvertTo32(pix3);

        /* Re-quantize using median cut */
    pix5 = pixMedianCutQuant(pix4, 0);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix5, 0, 0, NULL, rp->display);

        /* Re-quantize to few colors using median cut */
    pix6 = pixFewColorsMedianCutQuantMixed(pix4, 30, 30, 100, 0, 0, 0);
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix6, 0, 0, NULL, rp->display);

        /* Octcube quantize mixed with gray */
    startTimer();
    pix7 = pixOctcubeQuantMixedWithGray(pix2, 4, 5, 5);
    lept_stderr("Time to re-quantize mixed = %7.3f sec\n", stopTimer());
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix7, 0, 0, NULL, rp->display);

        /* Fixed octcube quantization */
    startTimer();
    pix8 = pixFixedOctcubeQuant256(pix2, 0);
    lept_stderr("Time to re-quantize 256 = %7.3f sec\n", stopTimer());
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pix8, 0, 0, NULL, rp->display);

        /* Remove unused colors */
    startTimer();
    pix9 = pixCopy(NULL, pix8);
    pixRemoveUnusedColors(pix9);
    lept_stderr("Time to remove unused colors = %7.3f sec\n", stopTimer());
    regTestWritePixAndCheck(rp, pix9, IFF_PNG);  /* 7 */
    pixDisplayWithTitle(pix8, 0, 0, NULL, rp->display);

         /* Compare before and after colors removed */
    regTestComparePix(rp, pix8, pix9);  /* 8 */

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixDestroy(&pix8);
    pixDestroy(&pix9);
    return regTestCleanup(rp);
}


