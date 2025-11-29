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
 * contrasttest.c
 *
 *   This appplies @factor contrast enhancement to the input image.
 *   It also plots atan curves for different width parameters.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
char       buf[512];
l_int32    iplot;
l_float32  factor;    /* scaled width of atan curve */
l_float32  fact[] = {0.2f, 0.4f, 0.6f, 0.8f, 1.0f, -1.0f};
GPLOT     *gplot;
NUMA      *na, *nax;
PIX       *pixs;

    if (argc != 4)
        return ERROR_INT(" Syntax:  contrasttest filein factor fileout",
               __func__, 1);
    filein = argv[1];
    factor = atof(argv[2]);
    fileout = argv[3];

    setLeptDebugOK(1);
    lept_mkdir("lept/contrast");

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

    na = numaContrastTRC(factor);
    gplotSimple1(na, GPLOT_PNG, "/tmp/lept/contrast/trc1", "contrast trc");
    l_fileDisplay("/tmp/lept/contrast/trc1.png", 0, 100, 1.0);
    numaDestroy(&na);

         /* Plot contrast TRC maps */
    nax = numaMakeSequence(0.0, 1.0, 256);
    gplot = gplotCreate("/tmp/lept/contrast/trc2", GPLOT_PNG,
        "Atan mapping function for contrast enhancement",
        "value in", "value out");
    for (iplot = 0; fact[iplot] >= 0.0; iplot++) {
        na = numaContrastTRC(fact[iplot]);
        snprintf(buf, sizeof(buf), "factor = %3.1f", fact[iplot]);
        gplotAddPlot(gplot, nax, na, GPLOT_LINES, buf);
        numaDestroy(&na);
    }
    gplotMakeOutput(gplot);
    gplotDestroy(&gplot);
    l_fileDisplay("/tmp/lept/contrast/trc2.png", 600, 100, 1.0);
    numaDestroy(&nax);

        /* Apply the input contrast enhancement */
    pixContrastTRC(pixs, pixs, factor);
    pixWrite(fileout, pixs, IFF_PNG);
    pixDestroy(&pixs);
    return 0;
}

