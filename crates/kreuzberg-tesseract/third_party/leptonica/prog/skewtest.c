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
 * skewtest.c
 *
 *     Tests various skew finding methods, optionally deskewing
 *     the input (binary) image.  The best version does a linear
 *     sweep followed by a binary (angle-splitting) search.
 *     The basic method is to find the vertical shear angle such
 *     that the differential variance of ON pixels between each
 *     line and it's neighbor, when summed over all lines, is
 *     maximized.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* binarization threshold */
#define   BIN_THRESHOLD         130

    /* deskew */
#define   DESKEW_REDUCTION      2      /* 1, 2 or 4 */

    /* sweep only */
#define   SWEEP_RANGE           10.    /* degrees */
#define   SWEEP_DELTA           0.2    /* degrees */
#define   SWEEP_REDUCTION       2      /* 1, 2, 4 or 8 */

    /* sweep and search */
#define   SWEEP_RANGE2          10.    /* degrees */
#define   SWEEP_DELTA2          1.     /* degrees */
#define   SWEEP_REDUCTION2      2      /* 1, 2, 4 or 8 */
#define   SEARCH_REDUCTION      2      /* 1, 2, 4 or 8 */
#define   SEARCH_MIN_DELTA      0.01   /* degrees */


int main(int    argc,
         char **argv)
{
char      *filein;
l_int32    ret;
l_float32  deg2rad;
l_float32  angle, conf, score, endscore;
PIX       *pix, *pixs, *pixd;

    if (argc != 2)
        return ERROR_INT(" Syntax:  skewtest filein", __func__, 1);
    filein = argv[1];

    setLeptDebugOK(1);
    lept_rmdir("lept/deskew");
    lept_mkdir("lept/deskew");
    pixd = NULL;
    deg2rad = 3.1415926535 / 180.;

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

        /* Find the skew angle various ways */
    pix = pixConvertTo1(pixs, BIN_THRESHOLD);
    pixWrite("/tmp/lept/deskew/binarized.tif", pix, IFF_TIFF_G4);
    pixFindSkew(pix, &angle, &conf);
    lept_stderr("pixFindSkew():\n"
                "  conf = %5.3f, angle = %7.3f degrees\n", conf, angle);

    pixFindSkewSweepAndSearchScorePivot(pix, &angle, &conf, &score,
                                        SWEEP_REDUCTION2, SEARCH_REDUCTION,
                                        0.0, SWEEP_RANGE2, SWEEP_DELTA2,
                                        SEARCH_MIN_DELTA,
                                        L_SHEAR_ABOUT_CORNER);
    lept_stderr("pixFind...Pivot(about corner):\n"
                "  conf = %5.3f, angle = %7.3f degrees, score = %.0f\n",
                conf, angle, score);

    pixFindSkewSweepAndSearchScorePivot(pix, &angle, &conf, &score,
                                        SWEEP_REDUCTION2, SEARCH_REDUCTION,
                                        0.0, SWEEP_RANGE2, SWEEP_DELTA2,
                                        SEARCH_MIN_DELTA,
                                        L_SHEAR_ABOUT_CENTER);
    lept_stderr("pixFind...Pivot(about center):\n"
                "  conf = %5.3f, angle = %7.3f degrees, score = %.0f\n",
                conf, angle, score);

        /* Use top-level */
    pixd = pixDeskew(pixs, 0);
    pixWriteImpliedFormat("/tmp/lept/deskew/result1", pixd, 0, 0);
    pixDestroy(&pix);
    pixDestroy(&pixd);

#if 1
        /* Do skew finding and rotation separately.  This fails if
         * the skew angle is outside the range. */
    pix = pixConvertTo1(pixs, BIN_THRESHOLD);
    if (pixGetDepth(pixs) == 1) {
        pixd = pixDeskew(pix, DESKEW_REDUCTION);
        pixWrite("/tmp/lept/deskew/result2", pixd, IFF_PNG);
    }
    else {
        ret = pixFindSkewSweepAndSearch(pix, &angle, &conf, SWEEP_REDUCTION2,
                                        SEARCH_REDUCTION, SWEEP_RANGE2,
                                        SWEEP_DELTA2, SEARCH_MIN_DELTA);
        if (ret)
            L_WARNING("skew angle not valid\n", __func__);
        else {
            lept_stderr("conf = %5.3f, angle = %7.3f degrees\n", conf, angle);
            if (conf > 2.5)
                pixd = pixRotate(pixs, angle * deg2rad, L_ROTATE_AREA_MAP,
                                 L_BRING_IN_WHITE, 0, 0);
            else
                pixd = pixClone(pixs);
            pixWrite("/tmp/lept/deskew/result2", pixd, IFF_PNG);
            pixDestroy(&pixd);
        }
    }
    pixDestroy(&pix);
#endif

#if 1
    pixFindSkewSweepAndSearchScore(pixs, &angle, &conf, &endscore,
                                   4, 2, 0.0, 5.0, 1.0, 0.01);
    lept_stderr("angle = %8.4f, conf = %8.4f, endscore = %.0f\n",
                angle, conf, endscore);
    startTimer();
    pixd = pixDeskew(pixs, DESKEW_REDUCTION);
    lept_stderr("Time to deskew = %7.4f sec\n", stopTimer());
    pixWrite("/tmp/lept/deskew/result3", pixd, IFF_PNG);
    pixDestroy(&pixd);
#endif

#if 1
    ret = pixFindSkew(pixs, &angle, &conf);
    lept_stderr("angle = %8.4f, conf = %8.4f\n", angle, conf);
    if (ret) {
        L_WARNING("skew angle not valid\n", __func__);
        return 1;
    }
#endif

#if 1
    ret = pixFindSkewSweep(pixs, &angle, SWEEP_REDUCTION,
                           SWEEP_RANGE, SWEEP_DELTA);
    lept_stderr("angle = %8.4f, conf = %8.4f\n", angle, conf);
    if (ret) {
        L_WARNING("skew angle not valid\n", __func__);
        return 1;
    }
#endif

#if 1
    ret = pixFindSkewSweepAndSearch(pixs, &angle, &conf,
                                    SWEEP_REDUCTION2, SEARCH_REDUCTION,
                                    SWEEP_RANGE2, SWEEP_DELTA2,
                                    SEARCH_MIN_DELTA);
    lept_stderr("angle = %8.4f, conf = %8.4f\n", angle, conf);
    if (ret) {
        L_WARNING("skew angle not valid\n", __func__);
        return 1;
    }
#endif

    pixDestroy(&pixs);
    return 0;
}
