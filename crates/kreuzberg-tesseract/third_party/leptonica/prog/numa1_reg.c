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
 * numa1_reg.c
 *
 *   Tests:
 *     * histograms
 *     * interpolation
 *     * integration/differentiation
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <math.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, n, binsize, binstart, nbins;
l_float32     pi, val, angle, xval, yval, x0, y0, startval, fbinsize;
l_float32     minval, maxval, meanval, median, variance, rankval, rank, rmsdev;
GPLOT        *gplot;
NUMA         *na, *nahisto, *nax, *nay, *nasx, *nasy;
NUMA         *nadx, *nady, *nafx, *nafy;
PIX          *pixs, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "numa1_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/numa1");

    /* -------------------------------------------------------------------*
     *                            Histograms                              *
     * -------------------------------------------------------------------*/
    pi = 3.1415926535;
    na = numaCreate(5000);
    for (i = 0; i < 500000; i++) {
        angle = 0.02293 * i * pi;
        val = (l_float32)(999. * sin(angle));
        numaAddNumber(na, val);
    }

    nahisto = numaMakeHistogramClipped(na, 6, 2000);
    nbins = numaGetCount(nahisto);
    nax = numaMakeSequence(0, 1, nbins);
    pix1 = gplotGeneralPix2(nax, nahisto, GPLOT_LINES, "/tmp/lept/numa1/histo1",
                            "example histo 1", "i", "histo[i]");
    numaDestroy(&nax);
    numaDestroy(&nahisto);

    nahisto = numaMakeHistogram(na, 1000, &binsize, &binstart);
    nbins = numaGetCount(nahisto);
    nax = numaMakeSequence(binstart, binsize, nbins);
    lept_stderr(" binsize = %d, binstart = %d\n", binsize, binstart);
    pix2 = gplotGeneralPix2(nax, nahisto, GPLOT_LINES, "/tmp/lept/numa1/histo2",
                            "example histo 2", "i", "histo[i]");
    numaDestroy(&nax);
    numaDestroy(&nahisto);

    nahisto = numaMakeHistogram(na, 1000, &binsize, NULL);
    nbins = numaGetCount(nahisto);
    nax = numaMakeSequence(0, binsize, nbins);
    lept_stderr(" binsize = %d, binstart = %d\n", binsize, 0);
    pix3 = gplotGeneralPix2(nax, nahisto, GPLOT_LINES, "/tmp/lept/numa1/histo3",
                            "example histo 3", "i", "histo[i]");
    numaDestroy(&nax);
    numaDestroy(&nahisto);

    nahisto = numaMakeHistogramAuto(na, 1000);
    nbins = numaGetCount(nahisto);
    numaGetParameters(nahisto, &startval, &fbinsize);
    nax = numaMakeSequence(startval, fbinsize, nbins);
    lept_stderr(" binsize = %7.4f, binstart = %8.3f\n", fbinsize, startval);
    pix4 = gplotGeneralPix2(nax, nahisto, GPLOT_LINES, "/tmp/lept/numa1/histo4",
                            "example histo 4", "i", "histo[i]");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 3 */
    pixa = pixaCreate(4);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    if (rp->display) {
        pixd = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 20, 2);
        pixDisplayWithTitle(pixd, 0, 0, NULL, 1);
        pixDestroy(&pixd);
    }
    pixaDestroy(&pixa);
    numaDestroy(&nax);
    numaDestroy(&nahisto);

    numaGetStatsUsingHistogram(na, 2000, &minval, &maxval, &meanval,
                               &variance, &median, 0.80, &rankval, &nahisto);
    rmsdev = sqrt((l_float64)variance);
    numaHistogramGetRankFromVal(nahisto, rankval, &rank);
    regTestCompareValues(rp, -999.00, minval,  0.1);    /* 4 */
    regTestCompareValues(rp,  999.00, maxval,  0.1);    /* 5 */
    regTestCompareValues(rp,  0.055,  meanval, 0.001);  /* 6 */
    regTestCompareValues(rp,  0.30,   median,  0.005);  /* 7 */
    regTestCompareValues(rp,  706.41, rmsdev,  0.1);    /* 8 */
    regTestCompareValues(rp,  808.15, rankval, 0.1);    /* 9 */
    regTestCompareValues(rp,  0.800,  rank,    0.01);   /* 10 */
    if (rp->display) {
        lept_stderr("Sin histogram: \n"
                    "  min val  = %7.3f    -- should be -999.00\n"
                    "  max val  = %7.3f    -- should be  999.00\n"
                    "  mean val = %7.3f    -- should be    0.055\n"
                    "  median   = %7.3f    -- should be    0.30\n"
                    "  rmsdev   = %7.3f    -- should be  706.41\n"
                    "  rank val = %7.3f    -- should be  808.152\n"
                    "  rank     = %7.3f    -- should be    0.800\n",
                    minval, maxval, meanval, median, rmsdev, rankval, rank);
    }
    numaDestroy(&nahisto);
    numaDestroy(&na);

    /* -------------------------------------------------------------------*
     *                            Interpolation                           *
     * -------------------------------------------------------------------*/
        /* Test numaInterpolateEqxInterval() */
    pixs = pixRead("test8.jpg");
    na = pixGetGrayHistogramMasked(pixs, NULL, 0, 0, 1);
    nasy = numaGetPartialSums(na);
    pix1 = gplotGeneralPix1(nasy, GPLOT_LINES, "/tmp/lept/numa1/int1",
                            "partial sums", NULL, NULL);
    pix2 = gplotGeneralPix1(na, GPLOT_LINES, "/tmp/lept/numa1/int2",
                            "simple test", NULL, NULL);
    numaInterpolateEqxInterval(0.0, 1.0, na, L_LINEAR_INTERP,
                               0.0, 255.0, 15, &nax, &nay);
    pix3 = gplotGeneralPix2(nax, nay, GPLOT_LINES, "/tmp/lept/numa1/int3",
                            "test interpolation", "pix val", "num pix");
    numaDestroy(&na);
    numaDestroy(&nasy);
    numaDestroy(&nax);
    numaDestroy(&nay);
    pixDestroy(&pixs);

        /* Test numaInterpolateArbxInterval() */
    pixs = pixRead("test8.jpg");
    na = pixGetGrayHistogramMasked(pixs, NULL, 0, 0, 1);
    nasy = numaGetPartialSums(na);
    numaInsertNumber(nasy, 0, 0.0);
    nasx = numaMakeSequence(0.0, 1.0, 257);
    numaInterpolateArbxInterval(nasx, nasy, L_LINEAR_INTERP,
                                10.0, 250.0, 23, &nax, &nay);
    pix4 = gplotGeneralPix2(nax, nay, GPLOT_LINES, "/tmp/lept/numa1/int4",
                            "arbx interpolation", "pix val", "cum num pix");
    numaDestroy(&na);
    numaDestroy(&nasx);
    numaDestroy(&nasy);
    numaDestroy(&nax);
    numaDestroy(&nay);
    pixDestroy(&pixs);

        /* Test numaInterpolateArbxVal() */
    pixs = pixRead("test8.jpg");
    na = pixGetGrayHistogramMasked(pixs, NULL, 0, 0, 1);
    nasy = numaGetPartialSums(na);
    numaInsertNumber(nasy, 0, 0.0);
    nasx = numaMakeSequence(0.0, 1.0, 257);
    nax = numaMakeSequence(15.0, (250.0 - 15.0) / 23.0, 24);
    n = numaGetCount(nax);
    nay = numaCreate(n);
    for (i = 0; i < n; i++) {
        numaGetFValue(nax, i, &xval);
        numaInterpolateArbxVal(nasx, nasy, L_QUADRATIC_INTERP, xval, &yval);
        numaAddNumber(nay, yval);
    }
    pix5 = gplotGeneralPix2(nax, nay, GPLOT_LINES, "/tmp/lept/numa1/int5",
                            "arbx interpolation", "pix val", "cum num pix");
    numaDestroy(&na);
    numaDestroy(&nasx);
    numaDestroy(&nasy);
    numaDestroy(&nax);
    numaDestroy(&nay);
    pixDestroy(&pixs);

        /* Test interpolation */
    nasx = numaRead("testangle.na");
    nasy = numaRead("testscore.na");
    gplot = gplotCreate("/tmp/lept/numa1/int6", GPLOT_PNG, "arbx interpolation",
                        "angle", "score");
    numaInterpolateArbxInterval(nasx, nasy, L_LINEAR_INTERP,
                                -2.00, 0.0, 50, &nax, &nay);
    gplotAddPlot(gplot, nax, nay, GPLOT_LINES, "linear");
    numaDestroy(&nax);
    numaDestroy(&nay);
    numaInterpolateArbxInterval(nasx, nasy, L_QUADRATIC_INTERP,
                                -2.00, 0.0, 50, &nax, &nay);
    gplotAddPlot(gplot, nax, nay, GPLOT_LINES, "quadratic");
    numaDestroy(&nax);
    numaDestroy(&nay);
    pix6 = gplotMakeOutputPix(gplot);
    gplotDestroy(&gplot);
    gplot = gplotCreate("/tmp/lept/numa1/int7", GPLOT_PNG, "arbx interpolation",
                        "angle", "score");
    numaInterpolateArbxInterval(nasx, nasy, L_LINEAR_INTERP,
                                -1.2, -0.8, 50, &nax, &nay);
    gplotAddPlot(gplot, nax, nay, GPLOT_LINES, "quadratic");
    pix7 = gplotMakeOutputPix(gplot);
    gplotDestroy(&gplot);
    numaFitMax(nay, &yval, nax, &xval);
    if (rp->display) lept_stderr("max = %f at loc = %f\n", yval, xval);
    pixa = pixaCreate(7);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 11 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 12 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 13 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 14 */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 15 */
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 16 */
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 17 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix5, L_INSERT);
    pixaAddPix(pixa, pix6, L_INSERT);
    pixaAddPix(pixa, pix7, L_INSERT);
    if (rp->display) {
        pixd = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 20, 2);
        pixDisplayWithTitle(pixd, 300, 0, NULL, 1);
        pixDestroy(&pixd);
    }
    pixaDestroy(&pixa);
    numaDestroy(&nasx);
    numaDestroy(&nasy);
    numaDestroy(&nax);
    numaDestroy(&nay);

    /* -------------------------------------------------------------------*
     *                   Integration and differentiation                  *
     * -------------------------------------------------------------------*/
        /* Test integration and differentiation */
    nasx = numaRead("testangle.na");
    nasy = numaRead("testscore.na");
        /* ---------- Plot the derivative ---------- */
    numaDifferentiateInterval(nasx, nasy, -2.0, 0.0, 50, &nadx, &nady);
    pix1 = gplotGeneralPix2(nadx, nady, GPLOT_LINES, "/tmp/lept/numa1/diff1",
                            "derivative", "angle", "slope");
        /*  ---------- Plot the original function ----------- */
        /*  and the integral of the derivative; the two       */
        /*  should be approximately the same.                 */
    gplot = gplotCreate("/tmp/lept/numa1/diff2", GPLOT_PNG, "integ-diff",
                        "angle", "val");
    numaInterpolateArbxInterval(nasx, nasy, L_LINEAR_INTERP,
                                -2.00, 0.0, 50, &nafx, &nafy);
    gplotAddPlot(gplot, nafx, nafy, GPLOT_LINES, "function");
    n = numaGetCount(nadx);
    numaGetFValue(nafx, 0, &x0);
    numaGetFValue(nafy, 0, &y0);
    nay = numaCreate(n);
        /* (Note: this tests robustness of the integrator: we go from
         * i = 0, and choose to have only 1 point in the interpolation
         * there, which is too small and causes the function to bomb out.) */
    lept_stderr("******************************************************\n");
    lept_stderr("* Testing error checking: ignore 'npts < 2' error    *\n");
    for (i = 0; i < n; i++) {
        numaGetFValue(nadx, i, &xval);
        numaIntegrateInterval(nadx, nady, x0, xval, 2 * i + 1, &yval);
        numaAddNumber(nay, y0 + yval);
    }
    lept_stderr("******************************************************\n");
    gplotAddPlot(gplot, nafx, nay, GPLOT_LINES, "anti-derivative");
    pix2 = gplotMakeOutputPix(gplot);
    gplotDestroy(&gplot);
    pixa = pixaCreate(2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 18 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 19 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix2, L_INSERT);
    if (rp->display) {
        pixd = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 20, 2);
        pixDisplayWithTitle(pixd, 600, 0, NULL, 1);
        pixDestroy(&pixd);
    }
    pixaDestroy(&pixa);
    numaDestroy(&nasx);
    numaDestroy(&nasy);
    numaDestroy(&nafx);
    numaDestroy(&nafy);
    numaDestroy(&nadx);
    numaDestroy(&nady);
    numaDestroy(&nay);
    return regTestCleanup(rp);
}
