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
 *   otsutest1.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

static const l_int32 NTests = 5;
static const l_int32 gaussmean1[5] = {20, 40, 60, 80, 60};
static const l_int32 gaussstdev1[5] = {10, 20, 20, 20, 30};
static const l_int32 gaussmean2[5] = {220, 200, 140, 180, 150};
static const l_float32 gaussfract1[5] = {0.2f, 0.3f, 0.1f, 0.5f, 0.3f};
static char  buf[256];

static l_int32  GenerateSplitPlot(l_int32 i);
static NUMA *MakeGaussian(l_int32 mean, l_int32 stdev, l_float32 fract);


int main(int    argc,
         char **argv)
{
l_int32  i;
PIX     *pix;
PIXA    *pixa;

    setLeptDebugOK(1);
    lept_mkdir("lept/otsu");
    for (i = 0; i < NTests; i++)
        GenerateSplitPlot(i);

       /* Read the results back in ...  */
    pixa = pixaCreate(0);
    for (i = 0; i < NTests; i++) {
        snprintf(buf, sizeof(buf), "/tmp/lept/otsu/plot.%d.png", i);
        pix = pixRead(buf);
        pixaAddPix(pixa, pix, L_INSERT);
        snprintf(buf, sizeof(buf), "/tmp/lept/otsu/plots.%d.png", i);
        pix = pixRead(buf);
        pixaAddPix(pixa, pix, L_INSERT);
    }

        /* ... and save into a tiled pix  */
    pix = pixaDisplayTiledInColumns(pixa, 2, 1.0, 25, 0);
    pixWrite("/tmp/lept/otsu/plot.png", pix, IFF_PNG);
    pixDisplay(pix, 100, 100);
    pixaDestroy(&pixa);
    pixDestroy(&pix);
    return 0;
}


static l_int32
GenerateSplitPlot(l_int32  i)
{
char       title[256];
l_int32    split;
l_float32  ave1, ave2, num1, num2, maxnum, maxscore;
GPLOT     *gplot;
NUMA      *na1, *na2, *nascore, *nax, *nay;

        /* Generate a fake histogram composed of 2 gaussians */
    na1 = MakeGaussian(gaussmean1[i], gaussstdev1[i], gaussfract1[i]);
    na2 = MakeGaussian(gaussmean2[i], gaussstdev1[i], 1.0 - gaussfract1[i]);
    numaArithOp(na1, na1, na2, L_ARITH_ADD);

        /* Otsu splitting */
    numaSplitDistribution(na1, 0.08, &split, &ave1, &ave2, &num1, &num2,
                          &nascore);
    lept_stderr("split = %d, ave1 = %6.1f, ave2 = %6.1f\n", split, ave1, ave2);
    lept_stderr("num1 = %8.0f, num2 = %8.0f\n", num1, num2);

        /* Prepare for plotting a vertical line at the split point */
    nax = numaMakeConstant(split, 2);
    numaGetMax(na1, &maxnum, NULL);
    nay = numaMakeConstant(0, 2);
    numaReplaceNumber(nay, 1, (l_int32)(0.5 * maxnum));

        /* Plot the input histogram with the split location */
    snprintf(buf, sizeof(buf), "/tmp/lept/otsu/plot.%d", i);
    snprintf(title, sizeof(title), "Plot %d", i);
    gplot = gplotCreate(buf, GPLOT_PNG,
                        "Histogram: mixture of 2 gaussians",
                        "Grayscale value", "Number of pixels");
    gplotAddPlot(gplot, NULL, na1, GPLOT_LINES, title);
    gplotAddPlot(gplot, nax, nay, GPLOT_LINES, NULL);
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    numaDestroy(&na1);
    numaDestroy(&na2);

        /* Plot the score function */
    snprintf(buf, sizeof(buf), "/tmp/lept/otsu/plots.%d", i);
    snprintf(title, sizeof(title), "Plot %d", i);
    gplot = gplotCreate(buf, GPLOT_PNG,
                        "Otsu score function for splitting",
                        "Grayscale value", "Score");
    gplotAddPlot(gplot, NULL, nascore, GPLOT_LINES, title);
    numaGetMax(nascore, &maxscore, NULL);
    numaReplaceNumber(nay, 1, maxscore);
    gplotAddPlot(gplot, nax, nay, GPLOT_LINES, NULL);
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    numaDestroy(&nax);
    numaDestroy(&nay);
    numaDestroy(&nascore);
    return 0;
}


static NUMA *
MakeGaussian(l_int32 mean, l_int32 stdev, l_float32 fract)
{
l_int32    i, total;
l_float32  norm, val;
NUMA      *na;

    na = numaMakeConstant(0.0, 256);
    norm = fract / ((l_float32)stdev * sqrt(2 * 3.14159));
    total = 0;
    for (i = 0; i < 256; i++) {
        val = norm * 1000000. * exp(-(l_float32)((i - mean) * (i - mean)) /
                                      (l_float32)(2 * stdev * stdev));
        total += (l_int32)val;
        numaSetValue(na, i, val);
    }
    lept_stderr("Total = %d\n", total);

    return na;
}

