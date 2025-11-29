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
 * rasterop_reg.c
 *
 *     This is a fairly rigorous test of rasterop.
 *     It demonstrates both that the results are correct
 *     with many different rop configurations, and,
 *     if done under valgrind, that no memory violations occur.
 *     We use it on an image with FG extending to the edges.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, j, w, h, same, width, height, cx, cy;
l_uint32      val;
BOX          *box;
PIX          *pix0, *pixs, *pixse, *pixd1, *pixd2;
SEL          *sel;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pix0 = pixRead("feyn-fract.tif");
    box = boxCreate(293, 37, pixGetWidth(pix0) - 691, pixGetHeight(pix0) -145);
    pixs = pixClipRectangle(pix0, box, NULL);
    boxDestroy(&box);
    if (rp->display) pixDisplay(pixs, 100, 100);

        /* Test 63 different sizes */
    for (width = 1; width <= 25; width += 3) {   /* 9 values */
	for (height = 1; height <= 25; height += 4) {  /* 7 values */

	    cx = width / 2;
	    cy = height / 2;

		/* Dilate using an actual sel */
	    sel = selCreateBrick(height, width, cy, cx, SEL_HIT);
	    pixd1 = pixDilate(NULL, pixs, sel);

		/* Dilate using a pix as a sel */
	    pixse = pixCreate(width, height, 1);
	    pixSetAll(pixse);
	    pixd2 = pixCopy(NULL, pixs);
	    w = pixGetWidth(pixs);
	    h = pixGetHeight(pixs);
	    for (i = 0; i < h; i++) {
		for (j = 0; j < w; j++) {
		    pixGetPixel(pixs, j, i, &val);
		    if (val)
			pixRasterop(pixd2, j - cx, i - cy, width, height,
				    PIX_SRC | PIX_DST, pixse, 0, 0);
		}
	    }

	    pixEqual(pixd1, pixd2, &same);
            regTestCompareValues(rp, 1, same, 0.0);  /* 0 - 62 */
	    if (same == 0)
		lept_stderr("Results differ for SE (width,height) = (%d,%d)\n",
                            width, height);

	    pixDestroy(&pixse);
	    pixDestroy(&pixd1);
	    pixDestroy(&pixd2);
	    selDestroy(&sel);
	}
    }
    pixDestroy(&pix0);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}
