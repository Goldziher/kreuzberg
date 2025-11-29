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
 * dna_reg.c
 *
 *   Tests basic functioning of L_Dna (number array of doubles)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, nbins, ival;
l_float64     pi, angle, val, sum;
L_DNA        *da1, *da2, *da3, *da4, *da5;
L_DNAA       *daa1, *daa2;
GPLOT        *gplot;
NUMA         *na, *nahisto, *nax;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "dna_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pi = 3.1415926535;
    da1 = l_dnaCreate(50);
    for (i = 0; i < 5000; i++) {
        angle = 0.02293 * i * pi;
        val = 999. * sin(angle);
        l_dnaAddNumber(da1, val);
    }

        /* Conversion to Numa; I/O for Dna */
    na = l_dnaConvertToNuma(da1);
    da2 = numaConvertToDna(na);
    l_dnaWrite("/tmp/lept/regout/dna1.da", da1);
    l_dnaWrite("/tmp/lept/regout/dna2.da", da2);
    da3 = l_dnaRead("/tmp/lept/regout/dna2.da");
    l_dnaWrite("/tmp/lept/regout/dna3.da", da3);
    regTestCheckFile(rp, "/tmp/lept/regout/dna1.da");  /* 0 */
    regTestCheckFile(rp, "/tmp/lept/regout/dna2.da");  /* 1 */
    regTestCheckFile(rp, "/tmp/lept/regout/dna3.da");  /* 2 */
    regTestCompareFiles(rp, 1, 2);  /* 3 */

        /* I/O for Dnaa */
    daa1 = l_dnaaCreate(3);
    l_dnaaAddDna(daa1, da1, L_INSERT);
    l_dnaaAddDna(daa1, da2, L_INSERT);
    l_dnaaAddDna(daa1, da3, L_INSERT);
    l_dnaaWrite("/tmp/lept/regout/dnaa1.daa", daa1);
    daa2 = l_dnaaRead("/tmp/lept/regout/dnaa1.daa");
    l_dnaaWrite("/tmp/lept/regout/dnaa2.daa", daa2);
    regTestCheckFile(rp, "/tmp/lept/regout/dnaa1.daa");  /* 4 */
    regTestCheckFile(rp, "/tmp/lept/regout/dnaa2.daa");  /* 5 */
    regTestCompareFiles(rp, 4, 5);  /* 6 */
    l_dnaaDestroy(&daa1);
    l_dnaaDestroy(&daa2);

        /* Just for fun -- is the numa ok? */
    nahisto = numaMakeHistogramClipped(na, 12, 2000);
    nbins = numaGetCount(nahisto);
    nax = numaMakeSequence(0, 1, nbins);
    gplot = gplotCreate("/tmp/lept/regout/historoot", GPLOT_PNG,
                        "Histo example", "i", "histo[i]");
    gplotAddPlot(gplot, nax, nahisto, GPLOT_LINES, "sine");
    gplotMakeOutput(gplot);
    regTestCheckFile(rp, "/tmp/lept/regout/historoot.png");  /* 7 */
    gplotDestroy(&gplot);
    numaDestroy(&na);
    numaDestroy(&nax);
    numaDestroy(&nahisto);

        /* Handling precision of int32 in double */
    da4 = l_dnaCreate(25);
    for (i = 0; i < 1000; i++)
        l_dnaAddNumber(da4, 1928374 * i);
    l_dnaWrite("/tmp/lept/regout/dna4.da", da4);
    da5 = l_dnaRead("/tmp/lept/regout/dna4.da");
    sum = 0;
    for (i = 0; i < 1000; i++) {
        l_dnaGetIValue(da5, i, &ival);
        sum += L_ABS(ival - i * 1928374);  /* we better be adding 0 each time */
    }
    regTestCompareValues(rp, sum, 0.0, 0.0);  /* 8 */
    l_dnaDestroy(&da4);
    l_dnaDestroy(&da5);

    return regTestCleanup(rp);
}
