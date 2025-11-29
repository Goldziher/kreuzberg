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
 * runlengthtest.c
 *
 *    Set 1 tests the runlength and 1-component dynamic range transform.
 *    Set 2 tests the 3-component (rgb) dynamic range transform.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_float32  avediff, rmsdiff;
PIX       *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;

    if (argc != 1)
        return ERROR_INT(" Syntax:  runlengthtest", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/run");

        /* Set 1 */
    startTimer();
    pix1 = pixRead("rabi.png");
    pix2 = pixRunlengthTransform(pix1, 0, L_HORIZONTAL_RUNS, 8);
    pix3 = pixRunlengthTransform(pix1, 0, L_VERTICAL_RUNS, 8);
    pix4 = pixMinOrMax(NULL, pix2, pix3, L_CHOOSE_MIN);
    pix5 = pixMaxDynamicRange(pix4, L_LOG_SCALE);
    pix6 = pixMinOrMax(NULL, pix2, pix3, L_CHOOSE_MAX);
    pix7 = pixMaxDynamicRange(pix6, L_LOG_SCALE);
    lept_stderr("Time for set 1: %7.3f sec\n", stopTimer());
    pixDisplay(pix2, 0, 0);
    pixDisplay(pix3, 600, 0);
    pixDisplay(pix4, 1200, 0);
    pixDisplay(pix5, 1800, 0);
    pixDisplay(pix6, 1200, 0);
    pixDisplay(pix7, 1800, 0);
    pixWrite("/tmp/lept/run/pixh.png", pix2, IFF_PNG);
    pixWrite("/tmp/lept/run/pixv.png", pix3, IFF_PNG);
    pixWrite("/tmp/lept/run/pixmin.png", pix4, IFF_PNG);
    pixWrite("/tmp/lept/run/pixminlog.png", pix5, IFF_PNG);
    pixWrite("/tmp/lept/run/pixmax.png", pix6, IFF_PNG);
    pixWrite("/tmp/lept/run/pixmaxlog.png", pix7, IFF_PNG);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);

        /* Set 2 */
    startTimer();
    pix1 = pixRead("test24.jpg");
    pixWriteJpeg("/tmp/lept/run/junk24.jpg", pix1, 5, 0);
    pix2 = pixRead("/tmp/lept/run/junk24.jpg");
    pixCompareGrayOrRGB(pix1, pix2, L_COMPARE_ABS_DIFF, GPLOT_PNG,
                        NULL, &avediff, &rmsdiff, &pix3);
    lept_stderr("Ave diff = %6.3f, RMS diff = %6.3f\n", avediff, rmsdiff);
    pix4 = pixMaxDynamicRangeRGB(pix3, L_LINEAR_SCALE);
    pix5 = pixMaxDynamicRangeRGB(pix3, L_LOG_SCALE);
    lept_stderr("Time for set 2: %7.3f sec\n", stopTimer());
    pixDisplay(pix4, 0, 800);
    pixDisplay(pix5, 1000, 800);
    pixWrite("/tmp/lept/run/linear.png", pix4, IFF_PNG);
    pixWrite("/tmp/lept/run/log.png", pix5, IFF_PNG);

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);

    return 0;
}
