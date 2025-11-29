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
 * nearline_reg.c
 *
 *   Regression test for finding min or max values (and averages)
 *   near a specified line.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
l_int32       ret, i, n, similar, x1, y1, val1, val2, val3, val4;
l_float32     minave, minave2, maxave, fract;
NUMA         *na1, *na2, *na3, *na4, *na5, *na6;
NUMAA        *naa;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "nearline_reg");
    exit(77);
#endif

   if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("feyn.tif");
    pix1 = pixScaleToGray6(pixs);
    pixDisplayWithTitle(pix1, 100, 600, NULL, rp->display);

        /* Find averages of min and max along about 120 horizontal lines */
    lept_stderr("******************************************************\n");
    lept_stderr("* Testing error checking: ignore 12 error messages   *\n");
    na1 = numaCreate(0);
    na3 = numaCreate(0);
    for (y1 = 40; y1 < 575; y1 += 5) {
        ret = pixMinMaxNearLine(pix1, 20, y1, 400, y1, 5, L_SCAN_BOTH,
                                NULL, NULL, &minave, &maxave);
        if (!ret) {
            numaAddNumber(na1, (l_int32)minave);
            numaAddNumber(na3, (l_int32)maxave);
            if (rp->display)
                lept_stderr("y = %d: minave = %d, maxave = %d\n",
                            y1, (l_int32)minave, (l_int32)maxave);
        }
    }

        /* Find averages along about 120 vertical lines.  We've rotated
         * the image by 90 degrees, so the results should be nearly
         * identical to the first set.  Also generate a single-sided
         * scan (L_SCAN_NEGATIVE) for comparison with the double-sided scans. */
    pix2 = pixRotateOrth(pix1, 3);
    pixDisplayWithTitle(pix2, 600, 600, NULL, rp->display);
    na2 = numaCreate(0);
    na4 = numaCreate(0);
    na5 = numaCreate(0);
    for (x1 = 40; x1 < 575; x1 += 5) {
        ret = pixMinMaxNearLine(pix2, x1, 20, x1, 400, 5, L_SCAN_BOTH,
                                NULL, NULL, &minave, &maxave);
        pixMinMaxNearLine(pix2, x1, 20, x1, 400, 5, L_SCAN_NEGATIVE,
                          NULL, NULL, &minave2, NULL);
        if (!ret) {
            numaAddNumber(na2, (l_int32)minave);
            numaAddNumber(na4, (l_int32)maxave);
            numaAddNumber(na5, (l_int32)minave2);
            if (rp->display)
                lept_stderr("x = %d: minave = %d, minave2 = %d, maxave = %d\n",
                        x1, (l_int32)minave, (l_int32)minave2, (l_int32)maxave);
        }
    }
    lept_stderr("******************************************************\n");

    numaSimilar(na1, na2, 3.0, &similar);  /* should be TRUE */
    regTestCompareValues(rp, similar, 1, 0);  /* 0 */
    numaSimilar(na3, na4, 1.0, &similar);  /* should be TRUE */
    regTestCompareValues(rp, similar, 1, 0);  /* 1 */
    numaWrite("/tmp/lept/regout/na1.na", na1);
    numaWrite("/tmp/lept/regout/na2.na", na2);
    numaWrite("/tmp/lept/regout/na3.na", na3);
    numaWrite("/tmp/lept/regout/na4.na", na4);
    numaWrite("/tmp/lept/regout/na5.na", na5);
    regTestCheckFile(rp, "/tmp/lept/regout/na1.na");  /* 2 */
    regTestCheckFile(rp, "/tmp/lept/regout/na2.na");  /* 3 */
    regTestCheckFile(rp, "/tmp/lept/regout/na3.na");  /* 4 */
    regTestCheckFile(rp, "/tmp/lept/regout/na4.na");  /* 5 */
    regTestCheckFile(rp, "/tmp/lept/regout/na5.na");  /* 6 */

        /* Plot the average minimums for the 3 cases */
    naa = numaaCreate(3);
    numaaAddNuma(naa, na1, L_INSERT);  /* portrait, double-sided */
    numaaAddNuma(naa, na2, L_INSERT);  /* landscape, double-sided */
    numaaAddNuma(naa, na5, L_INSERT);  /* landscape, single-sided */
    gplotSimpleN(naa, GPLOT_PNG, "/tmp/lept/regout/nearline",
                 "Average minimums along lines");
    pix3 = pixRead("/tmp/lept/regout/nearline.png");
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 7 */
    pixDisplayWithTitle(pix3, 100, 100, NULL, rp->display);

    if (rp->display) {
        n = numaGetCount(na3);
        for (i = 0; i < n; i++) {
            numaGetIValue(na1, i, &val1);
            numaGetIValue(na2, i, &val2);
            numaGetIValue(na3, i, &val3);
            numaGetIValue(na4, i, &val4);
            lept_stderr("val1 = %d, val2 = %d, diff = %d; "
                        "val3 = %d, val4 = %d, diff = %d\n",
                        val1, val2, L_ABS(val1 - val2),
                        val3, val4, L_ABS(val3 - val4));
        }
    }

    numaaDestroy(&naa);
    numaDestroy(&na3);
    numaDestroy(&na4);

        /* Plot minima along a single line, with different distances */
    pixMinMaxNearLine(pix1, 20, 200, 400, 200, 2, L_SCAN_BOTH,
                      &na1, NULL, NULL, NULL);
    pixMinMaxNearLine(pix1, 20, 200, 400, 200, 5, L_SCAN_BOTH,
                      &na2, NULL, NULL, NULL);
    pixMinMaxNearLine(pix1, 20, 200, 400, 200, 15, L_SCAN_BOTH,
                      &na3, NULL, NULL, NULL);
    numaWrite("/tmp/lept/regout/na6.na", na1);
    regTestCheckFile(rp, "/tmp/lept/regout/na6.na");  /* 8 */
    n = numaGetCount(na1);
    fract = 100.0 / n;
    na4 = numaTransform(na1, 0.0, fract);
    na5 = numaTransform(na2, 0.0, fract);
    na6 = numaTransform(na3, 0.0, fract);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    na1 = numaUniformSampling(na4, 100);
    na2 = numaUniformSampling(na5, 100);
    na3 = numaUniformSampling(na6, 100);
    naa = numaaCreate(3);
    numaaAddNuma(naa, na1, L_INSERT);
    numaaAddNuma(naa, na2, L_INSERT);
    numaaAddNuma(naa, na3, L_INSERT);
    gplotSimpleN(naa, GPLOT_PNG, "/tmp/lept/regout/nearline2",
                 "Min along line");
    pix4 = pixRead("/tmp/lept/regout/nearline2.png");
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 9 */
    pixDisplayWithTitle(pix4, 800, 100, NULL, rp->display);
    numaaDestroy(&naa);
    numaDestroy(&na4);
    numaDestroy(&na5);
    numaDestroy(&na6);

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}

