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
 * quadtreetest.c
 *
 *     This tests quadtree statistical functions
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_uint8      *data;
l_int32       i, j, w, h, error;
l_float32     val1, val2;
l_float32     val00, val10, val01, val11, valc00, valc10, valc01, valc11;
size_t        size;
PIX          *pixs, *pixg, *pix1, *pix2, *pix3, *pix4, *pix5;
FPIXA        *fpixam, *fpixav, *fpixarv;
BOXAA        *baa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "quadtree_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/quad");

        /* Test generation of quadtree regions. */
    baa = boxaaQuadtreeRegions(1000, 500, 3);
    boxaaWriteMem(&data, &size, baa);
    regTestWriteDataAndCheck(rp, data, size, "baa");  /* 0 */
    if (rp->display) boxaaWriteStream(stderr, baa);
    boxaaDestroy(&baa);
    lept_free(data);
    baa = boxaaQuadtreeRegions(1001, 501, 3);
    boxaaWriteMem(&data, &size, baa);
    regTestWriteDataAndCheck(rp, data, size, "baa");  /* 1 */
    boxaaDestroy(&baa);
    lept_free(data);

        /* Test quadtree stats generation */
    pixs = pixRead("rabi.png");
    pixg = pixScaleToGray4(pixs);
    pixDestroy(&pixs);
    pixQuadtreeMean(pixg, 8, NULL, &fpixam);
    pix1 = fpixaDisplayQuadtree(fpixam, 2, 10);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 100, 0, NULL, rp->display);
    pixQuadtreeVariance(pixg, 8, NULL, NULL, &fpixav, &fpixarv);
    pix2 = fpixaDisplayQuadtree(fpixav, 2, 10);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix2, 100, 200, NULL, rp->display);
    pix3 = fpixaDisplayQuadtree(fpixarv, 2, 10);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix3, 100, 400, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    fpixaDestroy(&fpixav);
    fpixaDestroy(&fpixarv);

        /* Compare with fixed-size tiling at a resolution corresponding
         * to the deepest level of the quadtree above */
    pix4 = pixGetAverageTiled(pixg, 5, 6, L_MEAN_ABSVAL);
    pix5 = pixExpandReplicate(pix4, 4);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix5, 800, 0, NULL, rp->display);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pix4 = pixGetAverageTiled(pixg, 5, 6, L_STANDARD_DEVIATION);
    pix5 = pixExpandReplicate(pix4, 4);
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pix5, 800, 400, NULL, rp->display);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pixg);

        /* Test quadtree parent/child access */
    error = FALSE;
    fpixaGetFPixDimensions(fpixam, 4, &w, &h);
    for (i = 0; i < w; i += 2) {
        for (j = 0; j < h; j += 2) {
            quadtreeGetParent(fpixam, 4, j, i, &val1);
            fpixaGetPixel(fpixam, 3, j / 2, i / 2, &val2);
            if (val1 != val2) error = TRUE;
        }
    }
    regTestCompareValues(rp, 0, error, 0.0);  /* 7 */
    error = FALSE;
    for (i = 0; i < w; i++) {
        for (j = 0; j < h; j++) {
            quadtreeGetChildren(fpixam, 4, j, i,
                                &val00, &val10, &val01, &val11);
            fpixaGetPixel(fpixam, 5, 2 * j, 2 * i, &valc00);
            fpixaGetPixel(fpixam, 5, 2 * j + 1, 2 * i, &valc10);
            fpixaGetPixel(fpixam, 5, 2 * j, 2 * i + 1, &valc01);
            fpixaGetPixel(fpixam, 5, 2 * j + 1, 2 * i + 1, &valc11);
            if ((val00 != valc00) || (val10 != valc10) ||
                (val01 != valc01) || (val11 != valc11))
                error = TRUE;
        }
    }
    regTestCompareValues(rp, 0, error, 0.0);  /* 8 */
    fpixaDestroy(&fpixam);

    return regTestCleanup(rp);
}
