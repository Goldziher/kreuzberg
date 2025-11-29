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
 * equal_reg.c
 *
 *    Tests the pixEqual() function in many situations.
 *
 *    This also tests the quantization of grayscale and color
 *    images (to generate a colormapped image), and removal of
 *    the colormap to either RGB or grayscale.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* use this set */
#define   FEYN1            "feyn.tif"      /* 1 bpp */
#define   DREYFUS2         "dreyfus2.png"  /* 2 bpp cmapped */
#define   DREYFUS4         "dreyfus4.png"  /* 4 bpp cmapped */
#define   DREYFUS8         "dreyfus8.png"  /* 8 bpp cmapped */
#define   KAREN8           "karen8.jpg"    /* 8 bpp, not cmapped */
#define   MARGE32          "marge.jpg"     /* rgb */

int main(int    argc,
         char **argv)
{
PIX          *pixs, *pix1, *pix2, *pix3, *pix4;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/equal");

    pixs = pixRead(FEYN1);
    pixWrite("/tmp/lept/equal/junkfeyn.png", pixs, IFF_PNG);
    pix1 = pixRead("/tmp/lept/equal/junkfeyn.png");
    regTestComparePix(rp, pixs, pix1);  /* 0 */
    pixDestroy(&pixs);
    pixDestroy(&pix1);

    pixs = pixRead(DREYFUS2);
    pix1 = pixRemoveColormap(pixs, REMOVE_CMAP_BASED_ON_SRC);
    pix2 = pixRemoveColormap(pixs, REMOVE_CMAP_TO_FULL_COLOR);
    pix3 = pixOctreeQuantNumColors(pix2, 64, 1);
    pix4 = pixConvertRGBToColormap(pix2, 1);
    regTestComparePix(rp, pixs, pix1);  /* 1 */
    regTestComparePix(rp, pixs, pix2);  /* 2 */
    regTestComparePix(rp, pixs, pix3);  /* 3 */
    regTestComparePix(rp, pixs, pix4);  /* 4 */
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pixs = pixRead(DREYFUS4);
    pix1 = pixRemoveColormap(pixs, REMOVE_CMAP_BASED_ON_SRC);
    pix2 = pixRemoveColormap(pixs, REMOVE_CMAP_TO_FULL_COLOR);
    pix3 = pixOctreeQuantNumColors(pix2, 256, 1);
    pix4 = pixConvertRGBToColormap(pix2, 1);
    regTestComparePix(rp, pixs, pix1);  /* 5 */
    regTestComparePix(rp, pixs, pix2);  /* 6 */
    regTestComparePix(rp, pixs, pix3);  /* 7 */
    regTestComparePix(rp, pixs, pix4);  /* 8 */
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pixs = pixRead(DREYFUS8);
    pix1 = pixRemoveColormap(pixs, REMOVE_CMAP_BASED_ON_SRC);
    pix2 = pixRemoveColormap(pixs, REMOVE_CMAP_TO_FULL_COLOR);
    pix3 = pixConvertRGBToColormap(pix2, 1);
    regTestComparePix(rp, pixs, pix1);  /* 9 */
    regTestComparePix(rp, pixs, pix2);  /* 10 */
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);

    pixs = pixRead(KAREN8);
    pix1 = pixThresholdTo4bpp(pixs, 16, 1);
    pix2 = pixRemoveColormap(pix1, REMOVE_CMAP_BASED_ON_SRC);
    pix3 = pixRemoveColormap(pix1, REMOVE_CMAP_TO_FULL_COLOR);
    pix4 = pixConvertRGBToColormap(pix3, 1);
    regTestComparePix(rp, pix1, pix2);  /* 11 */
    regTestComparePix(rp, pix1, pix3);  /* 12 */
    regTestComparePix(rp, pix1, pix4);  /* 13 */
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    pixs = pixRead(MARGE32);
    pix1 = pixOctreeQuantNumColors(pixs, 32, 0);
    pix2 = pixRemoveColormap(pix1, REMOVE_CMAP_TO_FULL_COLOR);
    pix3 = pixConvertRGBToColormap(pix2, 1);
    pix4 = pixOctreeQuantNumColors(pix2, 64, 0);
    regTestComparePix(rp, pix1, pix2);  /* 14 */
    regTestComparePix(rp, pix1, pix3);  /* 15 */
    regTestComparePix(rp, pix1, pix4);  /* 16 */
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    return regTestCleanup(rp);
}

