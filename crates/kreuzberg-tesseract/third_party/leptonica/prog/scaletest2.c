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
 * scaletest2.c
 *
 *   Tests scale-to-gray, unsharp masking, smoothing, and color scaling
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   DISPLAY      0    /* set to 1 to see the results */

int main(int    argc,
         char **argv)
{
PIX     *pixs;
l_int32  d;

    if (argc != 2)
        return ERROR_INT(" Syntax:  scaletest2 filein", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/scale");

    if ((pixs = pixRead(argv[1])) == NULL)
       	return ERROR_INT("pixs not made", __func__, 1);
    d = pixGetDepth(pixs);

#if 1
        /* Integer scale-to-gray functions */
    if (d == 1) {
        PIX  *pixd;

        pixd = pixScaleToGray2(pixs);
        pixWrite("/tmp/lept/scale/s2g_2x", pixd, IFF_PNG);
        pixDestroy(&pixd);
        pixd = pixScaleToGray3(pixs);
        pixWrite("/tmp/lept/scale/s2g_3x", pixd, IFF_PNG);
        pixDestroy(&pixd);
        pixd = pixScaleToGray4(pixs);
        pixWrite("/tmp/lept/scale/s2g_4x", pixd, IFF_PNG);
        pixDestroy(&pixd);
        pixd = pixScaleToGray6(pixs);
        pixWrite("/tmp/lept/scale/s2g_6x", pixd, IFF_PNG);
        pixDestroy(&pixd);
        pixd = pixScaleToGray8(pixs);
        pixWrite("/tmp/lept/scale/s2g_8x", pixd, IFF_PNG);
        pixDestroy(&pixd);
        pixd = pixScaleToGray16(pixs);
        pixWrite("/tmp/lept/scale/s2g_16x", pixd, IFF_PNG);
        pixDestroy(&pixd);
    }
#endif

#if 1
        /* Various non-integer scale-to-gray, compared with
	 * with different ways of getting similar results */
    if (d == 1) {
        PIX  *pixt, *pixd;

        pixd = pixScaleToGray8(pixs);
        pixWrite("/tmp/lept/scale/s2g_8.png", pixd, IFF_PNG);
        pixDestroy(&pixd);

        pixd = pixScaleToGray(pixs, 0.124);
        pixWrite("/tmp/lept/scale/s2g_124.png", pixd, IFF_PNG);
        pixDestroy(&pixd);

        pixd = pixScaleToGray(pixs, 0.284);
        pixWrite("/tmp/lept/scale/s2g_284.png", pixd, IFF_PNG);
        pixDestroy(&pixd);

        pixt = pixScaleToGray4(pixs);
        pixd = pixScaleBySampling(pixt, 284./250., 284./250.);
        pixWrite("/tmp/lept/scale/s2g_284.2.png", pixd, IFF_PNG);
        pixDestroy(&pixt);
        pixDestroy(&pixd);

        pixt = pixScaleToGray4(pixs);
        pixd = pixScaleGrayLI(pixt, 284./250., 284./250.);
        pixWrite("/tmp/lept/scale/s2g_284.3.png", pixd, IFF_PNG);
        pixDestroy(&pixt);
        pixDestroy(&pixd);

        pixt = pixScaleBinary(pixs, 284./250., 284./250.);
        pixd = pixScaleToGray4(pixt);
        pixWrite("/tmp/lept/scale/s2g_284.4.png", pixd, IFF_PNG);
        pixDestroy(&pixt);
        pixDestroy(&pixd);

        pixt = pixScaleToGray4(pixs);
        pixd = pixScaleGrayLI(pixt, 0.49, 0.49);
        pixWrite("/tmp/lept/scale/s2g_42.png", pixd, IFF_PNG);
        pixDestroy(&pixt);
        pixDestroy(&pixd);

        pixt = pixScaleToGray4(pixs);
        pixd = pixScaleSmooth(pixt, 0.49, 0.49);
        pixWrite("/tmp/lept/scale/s2g_4sm.png", pixd, IFF_PNG);
        pixDestroy(&pixt);
        pixDestroy(&pixd);

        pixt = pixScaleBinary(pixs, .16/.125, .16/.125);
        pixd = pixScaleToGray8(pixt);
        pixWrite("/tmp/lept/scale/s2g_16.png", pixd, IFF_PNG);
        pixDestroy(&pixt);
        pixDestroy(&pixd);

        pixd = pixScaleToGray(pixs, .16);
        pixWrite("/tmp/lept/scale/s2g_16.2.png", pixd, IFF_PNG);
        pixDestroy(&pixd);
    }
#endif

#if 1
        /* Antialiased (smoothed) reduction, along with sharpening */
    if (d != 1) {
        PIX *pixt1, *pixt2;

        startTimer();
        pixt1 = pixScaleSmooth(pixs, 0.154, 0.154);
        lept_stderr("fast scale: %5.3f sec\n", stopTimer());
        pixDisplayWithTitle(pixt1, 0, 0, "smooth scaling", DISPLAY);
        pixWrite("/tmp/lept/scale/smooth1.png", pixt1, IFF_PNG);
        pixt2 = pixUnsharpMasking(pixt1, 1, 0.3);
        pixWrite("/tmp/lept/scale/smooth2.png", pixt2, IFF_PNG);
        pixDisplayWithTitle(pixt2, 200, 0, "sharp scaling", DISPLAY);
        pixDestroy(&pixt1);
        pixDestroy(&pixt2);
    }
#endif


#if 1
        /* Test a large range of scale-to-gray reductions */
    if (d == 1) {
        l_int32    i;
        l_float32  scale;
        PIX       *pixd;

        for (i = 2; i < 15; i++) {
            scale = 1. / (l_float32)i;
            startTimer();
            pixd = pixScaleToGray(pixs, scale);
            lept_stderr("Time for scale %7.3f: %7.3f sec\n",
            scale, stopTimer());
            pixDisplayWithTitle(pixd, 75 * i, 100, "scaletogray", DISPLAY);
            pixDestroy(&pixd);
        }
        for (i = 8; i < 14; i++) {
            scale = 1. / (l_float32)(2 * i);
            startTimer();
            pixd = pixScaleToGray(pixs, scale);
            lept_stderr("Time for scale %7.3f: %7.3f sec\n",
            scale, stopTimer());
            pixDisplayWithTitle(pixd, 100 * i, 600, "scaletogray", DISPLAY);
            pixDestroy(&pixd);
        }
    }
#endif


#if 1
        /* Test the same range of scale-to-gray mipmap reductions */
    if (d == 1) {
        l_int32    i;
        l_float32  scale;
        PIX       *pixd;

        for (i = 2; i < 15; i++) {
            scale = 1. / (l_float32)i;
            startTimer();
            pixd = pixScaleToGrayMipmap(pixs, scale);
            lept_stderr("Time for scale %7.3f: %7.3f sec\n",
            scale, stopTimer());
            pixDisplayWithTitle(pixd, 75 * i, 100, "scale mipmap", DISPLAY);
            pixDestroy(&pixd);
        }
        for (i = 8; i < 12; i++) {
            scale = 1. / (l_float32)(2 * i);
            startTimer();
            pixd = pixScaleToGrayMipmap(pixs, scale);
            lept_stderr("Time for scale %7.3f: %7.3f sec\n",
            scale, stopTimer());
            pixDisplayWithTitle(pixd, 100 * i, 600, "scale mipmap", DISPLAY);
            pixDestroy(&pixd);
        }
    }
#endif

#if 1
        /* Test several methods for antialiased reduction,
         * along with sharpening */
    if (d != 1) {
        PIX *pixt1, *pixt2, *pixt3, *pixt4, *pixt5, *pixt6, *pixt7;
        l_float32 SCALING = 0.27;
        l_int32   SIZE = 7;
        l_int32   smooth;
        l_float32 FRACT = 1.0;

        smooth = SIZE / 2;

        startTimer();
        pixt1 = pixScaleSmooth(pixs, SCALING, SCALING);
        lept_stderr("fast scale: %5.3f sec\n", stopTimer());
        pixDisplayWithTitle(pixt1, 0, 0, "smooth scaling", DISPLAY);
        pixWrite("/tmp/lept/scale/sm_1.png", pixt1, IFF_PNG);
        pixt2 = pixUnsharpMasking(pixt1, 1, 0.3);
        pixDisplayWithTitle(pixt2, 150, 0, "sharpened scaling", DISPLAY);

        startTimer();
        pixt3 = pixBlockconv(pixs, smooth, smooth);
        pixt4 = pixScaleBySampling(pixt3, SCALING, SCALING);
        lept_stderr("slow scale: %5.3f sec\n", stopTimer());
        pixDisplayWithTitle(pixt4, 200, 200, "sampled scaling", DISPLAY);
        pixWrite("/tmp/lept/scale/sm_2.png", pixt4, IFF_PNG);

        startTimer();
        pixt5 = pixUnsharpMasking(pixs, smooth, FRACT);
        pixt6 = pixBlockconv(pixt5, smooth, smooth);
        pixt7 = pixScaleBySampling(pixt6, SCALING, SCALING);
        lept_stderr("very slow scale + sharp: %5.3f sec\n", stopTimer());
        pixDisplayWithTitle(pixt7, 500, 200, "sampled scaling", DISPLAY);
        pixWrite("/tmp/lept/scale/sm_3.jpg", pixt7, IFF_JFIF_JPEG);

        pixDestroy(&pixt1);
        pixDestroy(&pixt2);
        pixDestroy(&pixt3);
        pixDestroy(&pixt4);
        pixDestroy(&pixt5);
        pixDestroy(&pixt6);
        pixDestroy(&pixt7);
    }
#endif


#if 1
        /* Test the color scaling function, comparing the
         * special case of scaling factor 2.0 with the
         * general case. */
    if (d == 32) {
        PIX    *pix1, *pix2, *pixd;
        NUMA   *nar, *nag, *nab, *naseq;
        GPLOT  *gplot;

        startTimer();
        pix1 = pixScaleColorLI(pixs, 2.00001, 2.0);
        lept_stderr(" Time with regular LI: %7.3f\n", stopTimer());
        pixWrite("/tmp/lept/scale/color1.jpg", pix1, IFF_JFIF_JPEG);
        startTimer();
        pix2 = pixScaleColorLI(pixs, 2.0, 2.0);
        lept_stderr(" Time with 2x LI: %7.3f\n", stopTimer());
        pixWrite("/tmp/lept/scale/color2.jpg", pix2, IFF_JFIF_JPEG);

        pixd = pixAbsDifference(pix1, pix2);
        pixGetColorHistogram(pixd, 1, &nar, &nag, &nab);
        naseq = numaMakeSequence(0., 1., 256);
        gplot = gplotCreate("/tmp/lept/scale/c_absdiff", GPLOT_PNG,
                            "Number vs diff", "diff", "number");
        gplotSetScaling(gplot, GPLOT_LOG_SCALE_Y);
        gplotAddPlot(gplot, naseq, nar, GPLOT_POINTS, "red");
        gplotAddPlot(gplot, naseq, nag, GPLOT_POINTS, "green");
        gplotAddPlot(gplot, naseq, nab, GPLOT_POINTS, "blue");
        gplotMakeOutput(gplot);
        l_fileDisplay("/tmp/lept/scale/c_absdiff.png", 0, 100, 1.0);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pixd);
        numaDestroy(&naseq);
        numaDestroy(&nar);
        numaDestroy(&nag);
        numaDestroy(&nab);
        gplotDestroy(&gplot);
    }
#endif


#if 1
        /* Test the gray LI scaling function, comparing the
         * special cases of scaling factor 2.0 and 4.0 with the
         * general case */
    if (d == 8 || d == 32) {
        PIX    *pixt, *pix0, *pix1, *pix2, *pixd;
        NUMA   *nagray, *naseq;
        GPLOT  *gplot;

        if (d == 8)
            pixt = pixClone(pixs);
        else
            pixt = pixConvertRGBToGray(pixs, 0.33, 0.34, 0.33);
        pix0 = pixScaleGrayLI(pixt, 0.5, 0.5);

#if 1
        startTimer();
        pix1 = pixScaleGrayLI(pix0, 2.00001, 2.0);
        lept_stderr(" Time with regular LI 2x: %7.3f\n", stopTimer());
        startTimer();
        pix2 = pixScaleGrayLI(pix0, 2.0, 2.0);
        lept_stderr(" Time with 2x LI: %7.3f\n", stopTimer());
#else
        startTimer();
        pix1 = pixScaleGrayLI(pix0, 4.00001, 4.0);
        lept_stderr(" Time with regular LI 4x: %7.3f\n", stopTimer());
        startTimer();
        pix2 = pixScaleGrayLI(pix0, 4.0, 4.0);
        lept_stderr(" Time with 2x LI: %7.3f\n", stopTimer());
#endif
        pixWrite("/tmp/lept/scale/gray1", pix1, IFF_JFIF_JPEG);
        pixWrite("/tmp/lept/scale/gray2", pix2, IFF_JFIF_JPEG);

        pixd = pixAbsDifference(pix1, pix2);
        nagray = pixGetGrayHistogram(pixd, 1);
        naseq = numaMakeSequence(0., 1., 256);
        gplot = gplotCreate("/tmp/lept/scale/g_absdiff", GPLOT_PNG,
                            "Number vs diff", "diff", "number");
        gplotSetScaling(gplot, GPLOT_LOG_SCALE_Y);
        gplotAddPlot(gplot, naseq, nagray, GPLOT_POINTS, "gray");
        gplotMakeOutput(gplot);
        l_fileDisplay("/tmp/lept/scale/g_absdiff.png", 750, 100, 1.0);
        pixDestroy(&pixt);
        pixDestroy(&pix0);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pixd);
        numaDestroy(&naseq);
        numaDestroy(&nagray);
        gplotDestroy(&gplot);
    }
#endif

    pixDestroy(&pixs);
    return 0;
}

