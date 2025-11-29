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
 *  extrema_reg.c
 *
 *     Tests procedure for locating extrema (minima and maxima)
 *     of a sampled function.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_uint8      *data;
size_t        size;
l_int32       i, ival, n;
l_float32     f, val;
GPLOT        *gplot;
NUMA         *na1, *na2, *na3;
PIX          *pix1;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "extrema_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/extrema");

        /* Generate a 1D signal and plot it */
    na1 = numaCreate(500);
    for (i = 0; i < 500; i++) {
        f = 48.3 * sin(0.13 * (l_float32)i);
        f += 63.4 * cos(0.21 * (l_float32)i);
        numaAddNumber(na1, f);
    }
    gplot = gplotCreate("/tmp/lept/extrema/plot", GPLOT_PNG,
                        "Extrema test", "x", "y");
    gplotAddPlot(gplot, NULL, na1, GPLOT_LINES, "plot 1");

        /* Find the local min and max and plot them */
    na2 = numaFindExtrema(na1, 38.3, NULL);
    n = numaGetCount(na2);
    na3 = numaCreate(n);
    for (i = 0; i < n; i++) {
        numaGetIValue(na2, i, &ival);
        numaGetFValue(na1, ival, &val);
        numaAddNumber(na3, val);
    }
    gplotAddPlot(gplot, na2, na3, GPLOT_POINTS, "plot 2");
    gplotMakeOutput(gplot);

    numaWriteMem(&data, &size, na2);
    regTestWriteDataAndCheck(rp, data, size, "na");  /* 0 */
    lept_free(data);

    regTestCheckFile(rp, "/tmp/lept/extrema/plot.png");  /* 1 */
    if (rp->display) {
        pix1 = pixRead("/tmp/lept/extrema/plot.png");
        pixDisplay(pix1, 100, 100);
        pixDestroy(&pix1);
    }

    gplotDestroy(&gplot);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    return regTestCleanup(rp);
}
