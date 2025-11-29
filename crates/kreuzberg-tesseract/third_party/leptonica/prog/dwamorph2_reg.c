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
 * dwamorph2_reg.c
 *
 *     Compare the timings of various binary morphological implementations.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  HALFWIDTH   3

    /* Complete set of linear brick dwa operations */
PIX *pixMorphDwa_3(PIX *pixd, PIX *pixs, l_int32 operation, char *selname);

static const l_int32  NTIMES = 20;

int main(int    argc,
         char **argv)
{
char      *selname;
l_int32    i, j, nsels, sx, sy;
l_float32  fact, time;
GPLOT     *gplot;
NUMA      *na1, *na2, *na3, *na4, *nac1, *nac2, *nac3, *nac4, *nax;
PIX       *pixs, *pixt;
PIXA      *pixa;
SEL       *sel;
SELA      *selalinear;

    if (argc != 1)
        return ERROR_INT(" Syntax: dwamorph2_reg", __func__, 1);
    setLeptDebugOK(1);

    pixs = pixRead("feyn-fract.tif");
    pixt = pixCreateTemplate(pixs);
    selalinear = selaAddDwaLinear(NULL);
    nsels = selaGetCount(selalinear);

    fact = 1000. / (l_float32)NTIMES;  /* converts to time in msec */
    na1 = numaCreate(64);
    na2 = numaCreate(64);
    na3 = numaCreate(64);
    na4 = numaCreate(64);

    lept_mkdir("lept/morph");

        /*  ---------  dilation  ----------*/

    for (i = 0; i < nsels / 2; i++)
    {
        sel = selaGetSel(selalinear, i);
        selGetParameters(sel, &sy, &sx, NULL, NULL);
        selname = selGetName(sel);
        lept_stderr(" %d .", i);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixDilate(pixt, pixs, sel);
        time = fact * stopTimer();
        numaAddNumber(na1, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixDilateCompBrick(pixt, pixs, sx, sy);
        time = fact * stopTimer();
        numaAddNumber(na2, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixMorphDwa_3(pixt, pixs, L_MORPH_DILATE, selname);
        time = fact * stopTimer();
        numaAddNumber(na3, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixDilateCompBrickDwa(pixt, pixs, sx, sy);
        time = fact * stopTimer();
        numaAddNumber(na4, time);
    }

    nax = numaMakeSequence(2, 1, nsels / 2);
    nac1 = numaWindowedMean(na1, HALFWIDTH);
    nac2 = numaWindowedMean(na2, HALFWIDTH);
    nac3 = numaWindowedMean(na3, HALFWIDTH);
    nac4 = numaWindowedMean(na4, HALFWIDTH);
    gplot = gplotCreate("/tmp/lept/morph/dilate", GPLOT_PNG,
                        "Dilation time vs sel size", "size", "time (ms)");
    gplotAddPlot(gplot, nax, nac1, GPLOT_LINES, "linear rasterop");
    gplotAddPlot(gplot, nax, nac2, GPLOT_LINES, "composite rasterop");
    gplotAddPlot(gplot, nax, nac3, GPLOT_LINES, "linear dwa");
    gplotAddPlot(gplot, nax, nac4, GPLOT_LINES, "composite dwa");
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    numaDestroy(&nac1);
    numaDestroy(&nac2);
    numaDestroy(&nac3);
    numaDestroy(&nac4);

        /*  ---------  erosion  ----------*/

    numaEmpty(na1);
    numaEmpty(na2);
    numaEmpty(na3);
    numaEmpty(na4);
    for (i = 0; i < nsels / 2; i++)
    {
        sel = selaGetSel(selalinear, i);
        selGetParameters(sel, &sy, &sx, NULL, NULL);
        selname = selGetName(sel);
        lept_stderr(" %d .", i);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixErode(pixt, pixs, sel);
        time = fact * stopTimer();
        numaAddNumber(na1, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixErodeCompBrick(pixt, pixs, sx, sy);
        time = fact * stopTimer();
        numaAddNumber(na2, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixMorphDwa_3(pixt, pixs, L_MORPH_ERODE, selname);
        time = fact * stopTimer();
        numaAddNumber(na3, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixErodeCompBrickDwa(pixt, pixs, sx, sy);
        time = fact * stopTimer();
        numaAddNumber(na4, time);
    }

    nac1 = numaWindowedMean(na1, HALFWIDTH);
    nac2 = numaWindowedMean(na2, HALFWIDTH);
    nac3 = numaWindowedMean(na3, HALFWIDTH);
    nac4 = numaWindowedMean(na4, HALFWIDTH);
    gplot = gplotCreate("/tmp/lept/morph/erode", GPLOT_PNG,
                        "Erosion time vs sel size", "size", "time (ms)");
    gplotAddPlot(gplot, nax, nac1, GPLOT_LINES, "linear rasterop");
    gplotAddPlot(gplot, nax, nac2, GPLOT_LINES, "composite rasterop");
    gplotAddPlot(gplot, nax, nac3, GPLOT_LINES, "linear dwa");
    gplotAddPlot(gplot, nax, nac4, GPLOT_LINES, "composite dwa");
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    numaDestroy(&nac1);
    numaDestroy(&nac2);
    numaDestroy(&nac3);
    numaDestroy(&nac4);

        /*  ---------  opening  ----------*/

    numaEmpty(na1);
    numaEmpty(na2);
    numaEmpty(na3);
    numaEmpty(na4);
    for (i = 0; i < nsels / 2; i++)
    {
        sel = selaGetSel(selalinear, i);
        selGetParameters(sel, &sy, &sx, NULL, NULL);
        selname = selGetName(sel);
        lept_stderr(" %d .", i);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixOpen(pixt, pixs, sel);
        time = fact * stopTimer();
        numaAddNumber(na1, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixOpenCompBrick(pixt, pixs, sx, sy);
        time = fact * stopTimer();
        numaAddNumber(na2, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixMorphDwa_3(pixt, pixs, L_MORPH_OPEN, selname);
        time = fact * stopTimer();
        numaAddNumber(na3, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixOpenCompBrickDwa(pixt, pixs, sx, sy);
        time = fact * stopTimer();
        numaAddNumber(na4, time);
    }

    nac1 = numaWindowedMean(na1, HALFWIDTH);
    nac2 = numaWindowedMean(na2, HALFWIDTH);
    nac3 = numaWindowedMean(na3, HALFWIDTH);
    nac4 = numaWindowedMean(na4, HALFWIDTH);
    gplot = gplotCreate("/tmp/lept/morph/open", GPLOT_PNG,
                        "Opening time vs sel size", "size", "time (ms)");
    gplotAddPlot(gplot, nax, nac1, GPLOT_LINES, "linear rasterop");
    gplotAddPlot(gplot, nax, nac2, GPLOT_LINES, "composite rasterop");
    gplotAddPlot(gplot, nax, nac3, GPLOT_LINES, "linear dwa");
    gplotAddPlot(gplot, nax, nac4, GPLOT_LINES, "composite dwa");
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    numaDestroy(&nac1);
    numaDestroy(&nac2);
    numaDestroy(&nac3);
    numaDestroy(&nac4);

        /*  ---------  closing  ----------*/

    numaEmpty(na1);
    numaEmpty(na2);
    numaEmpty(na3);
    numaEmpty(na4);
    for (i = 0; i < nsels / 2; i++)
    {
        sel = selaGetSel(selalinear, i);
        selGetParameters(sel, &sy, &sx, NULL, NULL);
        selname = selGetName(sel);
        lept_stderr(" %d .", i);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixClose(pixt, pixs, sel);
        time = fact * stopTimer();
        numaAddNumber(na1, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixCloseCompBrick(pixt, pixs, sx, sy);
        time = fact * stopTimer();
        numaAddNumber(na2, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixMorphDwa_3(pixt, pixs, L_MORPH_CLOSE, selname);
        time = fact * stopTimer();
        numaAddNumber(na3, time);

        startTimer();
        for (j = 0; j < NTIMES; j++)
            pixCloseCompBrickDwa(pixt, pixs, sx, sy);
        time = fact * stopTimer();
        numaAddNumber(na4, time);
    }

    nac1 = numaWindowedMean(na1, HALFWIDTH);
    nac2 = numaWindowedMean(na2, HALFWIDTH);
    nac3 = numaWindowedMean(na3, HALFWIDTH);
    nac4 = numaWindowedMean(na4, HALFWIDTH);
    gplot = gplotCreate("/tmp/lept/morph/close", GPLOT_PNG,
                        "Closing time vs sel size", "size", "time (ms)");
    gplotAddPlot(gplot, nax, nac1, GPLOT_LINES, "linear rasterop");
    gplotAddPlot(gplot, nax, nac2, GPLOT_LINES, "composite rasterop");
    gplotAddPlot(gplot, nax, nac3, GPLOT_LINES, "linear dwa");
    gplotAddPlot(gplot, nax, nac4, GPLOT_LINES, "composite dwa");
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    numaDestroy(&nac1);
    numaDestroy(&nac2);
    numaDestroy(&nac3);
    numaDestroy(&nac4);


    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    numaDestroy(&na4);
    numaDestroy(&nax);
    selaDestroy(&selalinear);
    pixDestroy(&pixt);
    pixDestroy(&pixs);

        /* Display the results together */
    pixa = pixaCreate(0);
    pixs = pixRead("/tmp/lept/morph/dilate.png");
    pixaAddPix(pixa, pixs, L_INSERT);
    pixs = pixRead("/tmp/lept/morph/erode.png");
    pixaAddPix(pixa, pixs, L_INSERT);
    pixs = pixRead("/tmp/lept/morph/open.png");
    pixaAddPix(pixa, pixs, L_INSERT);
    pixs = pixRead("/tmp/lept/morph/close.png");
    pixaAddPix(pixa, pixs, L_INSERT);
    pixt = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 40, 3);
    pixWrite("/tmp/lept/morph/timings.png", pixt, IFF_PNG);
    pixDisplay(pixt, 100, 100);
    pixDestroy(&pixt);
    pixaDestroy(&pixa);
    return 0;
}

