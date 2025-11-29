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
 *  paintmask_reg.c
 *
 *    Regression test for painting through a mask onto various
 *    depth images.
 *
 *    This file shows how one can start with a 32 bpp RGB image and
 *    derive from it the following:
 *       8 bpp color, cmapped
 *       4 bpp color, cmapped
 *       2 bpp color, cmapped
 *       8 bpp gray
 *       4 bpp gray
 *       4 bpp gray, cmapped
 *       2 bpp gray
 *       2 bpp gray, cmapped
 *
 *    For each of these, pixClipMasked() is used to place a 1 bpp
 *    mask over part of the image, clip out the rectangular region
 *    supporting the mask, and paint a given color through the
 *    mask onto the result.
 *
 *    Finally we do a clip/mask operation on 1 bpp sources.
 *
 *    If you run 'paintmask_reg display', a pdf of the results is made.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
BOX          *box;
PIX          *pixs, *pixs8, *pixm, *pixt1, *pixt2, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixa = pixaCreate(0);

        /* Start with a 32 bpp image and a mask.  Use the
	 * same mask for all clip/masked operations. */
    pixs = pixRead("test24.jpg");
    pixt1 = pixRead("rabi.png");
    box = boxCreate(303, 1983, 800, 500);
    pixm = pixClipRectangle(pixt1, box, NULL);
    pixInvert(pixm, pixm);
    boxDestroy(&box);
    box = boxCreate(100, 100, 800, 500);  /* clips on pixs and derivatives */
    pixt2 = pixClipRectangle(pixs, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_JFIF_JPEG);  /* 0 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixDestroy(&pixt1);

        /* Clip 32 bpp RGB */
    pixd = pixClipMasked(pixs, pixm, 100, 100, 0x03c08000);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 1 */
    pixaAddPix(pixa, pixd, L_INSERT);

        /* Clip 8 bpp colormapped */
    pixt1 = pixMedianCutQuant(pixs, 0);
    pixt2 = pixClipRectangle(pixt1, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 0x03c08000);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 3 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pixt1);

        /* Clip 4 bpp colormapped */
    pixt1 = pixOctreeQuantNumColors(pixs, 16, 1);
    pixt2 = pixClipRectangle(pixt1, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_PNG);  /* 4 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 0x03c08000);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pixt1);

        /* Clip 2 bpp colormapped */
    pixt1 = pixMedianCutQuantGeneral(pixs, 0, 2, 4, 5, 1, 1);
    pixt2 = pixClipRectangle(pixt1, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_PNG);  /* 6 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 0x03608000);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 7 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pixt1);

        /* Clip 8 bpp gray */
    pixs8 = pixConvertRGBToLuminance(pixs);
    pixt2 = pixClipRectangle(pixs8, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_JFIF_JPEG);  /* 8 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixd = pixClipMasked(pixs8, pixm, 100, 100, 90);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);  /* 9 */
    pixaAddPix(pixa, pixd, L_INSERT);

        /* Clip 4 bpp gray */
    pixt1 = pixThresholdTo4bpp(pixs8, 16, 0);
    pixt2 = pixClipRectangle(pixt1, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_PNG);  /* 10 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 11 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 5);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 12 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 15);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 13 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pixt1);

        /* Clip 4 bpp gray, colormapped */
    pixt1 = pixThresholdTo4bpp(pixs8, 16, 1);
    pixt2 = pixClipRectangle(pixt1, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_PNG);  /* 14 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 0x55555500);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 15 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pixt1);

        /* Clip 2 bpp gray */
    pixt1 = pixThresholdTo2bpp(pixs8, 4, 0);
    pixt2 = pixClipRectangle(pixt1, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_PNG);  /* 16 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 17 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pixt1);

        /* Clip 2 bpp gray, colormapped */
    pixt1 = pixThresholdTo2bpp(pixs8, 4, 1);
    pixt2 = pixClipRectangle(pixt1, box, NULL);
    pixd = pixClipMasked(pixt1, pixm, 100, 100, 0x55555500);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 18 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pixt1);
    pixDestroy(&pixt2);

    pixDestroy(&pixm);
    pixDestroy(&pixs);
    pixDestroy(&pixs8);
    boxDestroy(&box);

        /* Finally, do the 1 bpp painting through clipped region.
         * We start with two 1 bpp text sources, use the inverse
         * of the 2nd for the mask (so we take all of the 1st
         * pixels under this mask), and for the remainder, which
         * are the fg pixels in the 2nd, we paint them black (1).
         * So this is a simple and fast blending of two 1 bpp pix. */
    pixs = pixRead("feyn.tif");
    box = boxCreate(670, 827, 800, 500);
    pixt2 = pixClipRectangle(pixs, box, NULL);
    regTestWritePixAndCheck(rp, pixt2, IFF_PNG);  /* 19 */
    pixaAddPix(pixa, pixt2, L_INSERT);
    boxDestroy(&box);
    pixt1 = pixRead("rabi.png");
    box = boxCreate(303, 1983, 800, 500);
    pixm = pixClipRectangle(pixt1, box, NULL);
    pixInvert(pixm, pixm);
    regTestWritePixAndCheck(rp, pixm, IFF_PNG);  /* 20 */
    pixaAddPix(pixa, pixm, L_INSERT);
    pixd = pixClipMasked(pixs, pixm, 670, 827, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 21 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pixs);
    pixDestroy(&pixt1);
    boxDestroy(&box);

        /* If in testing mode, make a pdf */
    if (rp->display) {
        pixaConvertToPdf(pixa, 100, 1.0, L_FLATE_ENCODE, 0,
                         "Paint through mask", "/tmp/lept/regout/paintmask.pdf");
        L_INFO("Output pdf: /tmp/lept/regout/paintmask.pdf\n", rp->testname);
    }

    pixaDestroy(&pixa);
    return regTestCleanup(rp);
}
