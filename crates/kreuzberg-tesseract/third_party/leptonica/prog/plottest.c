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
 * plottest.c
 *
 *     This tests the gplot library functions that generate
 *     the plot commands and data required for input to gnuplot.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include <math.h>

#include "allheaders.h"

    /* for GPLOT_STYLE, use one of the following set:
     *    GPLOT_LINES
     *    GPLOT_POINTS
     *    GPLOT_IMPULSE
     *    GPLOT_LINESPOINTS
     *    GPLOT_DOTS */
#define  GPLOT_STYLE   GPLOT_LINES

    /* for GPLOT_OUTPUT use one of the following set:
     *    GPLOT_PNG
     *    GPLOT_PS
     *    GPLOT_EPS
     *    GPLOT_LATEX
     */

#define  GPLOT_OUTPUT   GPLOT_PNG

int main(int    argc,
         char **argv)
{
char      *str1, *str2, *pngname;
l_int32    i;
size_t     size1, size2;
l_float32  x, y1, y2, pi;
GPLOT     *gplot1, *gplot2, *gplot3, *gplot4, *gplot5;
NUMA      *nax, *nay1, *nay2;

    if (argc != 1)
        return ERROR_INT(" Syntax:  plottest", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/plot");

        /* Generate plot data */
    nax = numaCreate(0);
    nay1 = numaCreate(0);
    nay2 = numaCreate(0);
    pi = 3.1415926535;
    for (i = 0; i < 180; i++) {
        x = (pi / 180.) * i;
        y1 = (l_float32)sin(2.4 * x);
        y2 = (l_float32)cos(2.4 * x);
        numaAddNumber(nax, x);
        numaAddNumber(nay1, y1);
        numaAddNumber(nay2, y2);
    }

        /* Show the plot */
    gplot1 = gplotCreate("/tmp/lept/plot/set1", GPLOT_OUTPUT, "Example plots",
                         "theta", "f(theta)");
    gplotAddPlot(gplot1, nax, nay1, GPLOT_STYLE, "sin (2.4 * theta)");
    gplotAddPlot(gplot1, nax, nay2, GPLOT_STYLE, "cos (2.4 * theta)");
    gplotMakeOutput(gplot1);

        /* Also save the plot to png */
    gplot1->outformat = GPLOT_PNG;
    pngname = genPathname("/tmp/lept/plot", "set1.png");
    stringReplace(&gplot1->outname, pngname);
    gplotMakeOutput(gplot1);
    l_fileDisplay("/tmp/lept/plot/set1.png", 100, 100, 1.0);
    lept_free(pngname);

        /* Test gplot serialization */
    gplotWrite("/tmp/lept/plot/plot1.gp", gplot1);
    if ((gplot2 = gplotRead("/tmp/lept/plot/plot1.gp")) == NULL)
        return ERROR_INT("gplotRead failure!", __func__, 1);
    gplotWrite("/tmp/lept/plot/plot2.gp", gplot2);

        /* Are the two written gplot files the same? */
    str1 = (char *)l_binaryRead("/tmp/lept/plot/plot1.gp", &size1);
    str2 = (char *)l_binaryRead("/tmp/lept/plot/plot2.gp", &size2);
    if (size1 != size2)
        lept_stderr("Error: size1 = %lu, size2 = %lu\n",
                    (unsigned long)size1, (unsigned long)size2);
    else
        lept_stderr("Correct: size1 = size2 = %lu\n", (unsigned long)size1);
    if (strcmp(str1, str2) != 0)
        lept_stderr("Error: str1 != str2\n");
    else
        lept_stderr("Correct: str1 == str2\n");
    lept_free(str1);
    lept_free(str2);

        /* Read from file and regenerate the plot */
    gplot3 = gplotRead("/tmp/lept/plot/plot2.gp");
    stringReplace(&gplot3->title , "Example plots regen");
    gplot3->outformat = GPLOT_PNG;
    gplotMakeOutput(gplot3);

        /* Build gplot but do not make the output formatted stuff */
    gplot4 = gplotCreate("/tmp/lept/plot/set2", GPLOT_OUTPUT,
                         "Example plots 2", "theta", "f(theta)");
    gplotAddPlot(gplot4, nax, nay1, GPLOT_STYLE, "sin (2.4 * theta)");
    gplotAddPlot(gplot4, nax, nay2, GPLOT_STYLE, "cos (2.4 * theta)");

        /* Write, read back, and generate the plot */
    gplotWrite("/tmp/lept/plot/plot4.gp", gplot4);
    if ((gplot5 = gplotRead("/tmp/lept/plot/plot4.gp")) == NULL)
        return ERROR_INT("gplotRead failure!", __func__, 1);
    gplotMakeOutput(gplot5);
    l_fileDisplay("/tmp/lept/plot/set2.png", 750, 100, 1.0);

    gplotDestroy(&gplot1);
    gplotDestroy(&gplot2);
    gplotDestroy(&gplot3);
    gplotDestroy(&gplot4);
    gplotDestroy(&gplot5);
    numaDestroy(&nax);
    numaDestroy(&nay1);
    numaDestroy(&nay2);
    return 0;
}
