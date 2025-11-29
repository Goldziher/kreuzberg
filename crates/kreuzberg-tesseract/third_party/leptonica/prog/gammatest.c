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
 * gammatest.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

#define  MINVAL      30
#define  MAXVAL      210

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
char       buf[512];
l_int32    iplot, same;
l_float32  gam;
l_float64  gamma[] = {.5, 1.0, 1.5, 2.0, 2.5, -1.0};
GPLOT     *gplot;
NUMA      *na, *nax;
PIX       *pixs, *pixd;

    if (argc != 4)
        return ERROR_INT(" Syntax:  gammatest filein gam fileout", __func__, 1);
    filein = argv[1];
    gam = atof(argv[2]);
    fileout = argv[3];

    setLeptDebugOK(1);
    lept_mkdir("lept/gamma");

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

    startTimer();
    pixd = pixGammaTRC(NULL, pixs, gam, MINVAL, MAXVAL);
    lept_stderr("Time for gamma: %7.3f sec\n", stopTimer());
    pixGammaTRC(pixs, pixs, gam, MINVAL, MAXVAL);
    pixEqual(pixs, pixd, &same);
    if (!same)
        lept_stderr("Error in pixGammaTRC!\n");
    pixWrite(fileout, pixs, IFF_JFIF_JPEG);
    pixDestroy(&pixs);

    na = numaGammaTRC(gam, MINVAL, MAXVAL);
    gplotSimple1(na, GPLOT_PNG, "/tmp/lept/gamma/trc", "gamma trc");
    l_fileDisplay("/tmp/lept/gamma/trc.png", 100, 100, 1.0);
    numaDestroy(&na);

        /* Plot gamma TRC maps */
    gplot = gplotCreate("/tmp/lept/gamma/corr", GPLOT_PNG,
                        "Mapping function for gamma correction",
                        "value in", "value out");
    nax = numaMakeSequence(0.0, 1.0, 256);
    for (iplot = 0; gamma[iplot] >= 0.0; iplot++) {
        na = numaGammaTRC(gamma[iplot], 30, 215);
        snprintf(buf, sizeof(buf), "gamma = %3.1f", gamma[iplot]);
        gplotAddPlot(gplot, nax, na, GPLOT_LINES, buf);
        numaDestroy(&na);
    }
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    l_fileDisplay("/tmp/lept/gamma/corr.png", 100, 100, 1.0);
    numaDestroy(&nax);
    return 0;
}

