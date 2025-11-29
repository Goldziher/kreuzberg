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
 * findbinding.c
 *
 *    Here is a simple approach to find the location of the binding
 *    in an open book that is photographed.  It relies on the typical
 *    condition that the background pixels near the binding are
 *    darker than those on the rest of the page, and further, that
 *    the lightest pixels in each column parallel to the binding
 *    exhibit a large variance by column near the binding.  This is
 *    because the pixels at the binding are typically even darker
 *    than the pixels near the binding.
 *
 *    Accurate results are obtained in this example at the very low
 *    resolution of 45 ppi.  Better results can be expected at higher
 *    resolution.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32    w, h, ystart, yend, y, ymax, ymid, i, window, sum1, sum2, rankx;
l_uint32   uval;
l_float32  ave, rankval, maxvar, variance, norm, conf, angle, radangle;
NUMA      *na1;
PIX       *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;
PIXA      *pixa;

    if (argc != 1)
        return ERROR_INT(" Syntax:  findbinding", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/binding");
    pixa = pixaCreate(0);

    pix1 = pixRead("binding-example.45.jpg");
    pix2 = pixConvertTo8(pix1, 0);

        /* Find the skew angle */
    pix3 = pixConvertTo1(pix2, 150);
    pixFindSkewSweepAndSearch(pix3, &angle, &conf, 2, 2, 7.0, 1.0, 0.01);
    lept_stderr("angle = %f, conf = %f\n", angle, conf);

        /* Deskew, bringing in black pixels at the edges */
    if (L_ABS(angle) < 0.1 || conf < 1.5) {
        pix4 = pixClone(pix2);
    } else {
        radangle = 3.1416 * angle / 180.0;
        pix4 = pixRotate(pix2, radangle, L_ROTATE_AREA_MAP,
                         L_BRING_IN_BLACK, 0, 0);
    }

        /* Rotate 90 degrees to make binding horizontal */
    pix5 = pixRotateOrth(pix4, 1);

        /* Sort pixels in each row by their gray value.
         * Dark pixels on the left, light ones on the right. */
    pix6 = pixRankRowTransform(pix5);
    pixDisplay(pix5, 0, 0);
    pixDisplay(pix6, 550, 0);
    pixaAddPix(pixa, pix4, L_COPY);
    pixaAddPix(pixa, pix5, L_COPY);
    pixaAddPix(pixa, pix6, L_COPY);

        /* Make an a priori estimate of the y-interval within which the
         * binding will be found.  The search will be done in this interval. */
    pixGetDimensions(pix6, &w, &h, NULL);
    ystart = 0.25 * h;
    yend = 0.75 * h;

        /* Choose a very light rank value; close to white, which
         * corresponds to a column in pix6 near the right side. */
    rankval = 0.98;
    rankx = (l_int32)(w * rankval);

        /* Investigate variance in a small window (vertical, size = 5)
         * of the pixels in that column.  These are the %rankval
         * pixels in each raster of pix6.  Find the y-location of
         * maximum variance. */
    window = 5;
    norm = 1.0 / window;
    maxvar = 0.0;
    na1 = numaCreate(0);
    numaSetParameters(na1, ystart, 1);
    for (y = ystart; y <= yend; y++) {
        sum1 = sum2 = 0;
        for (i = 0; i < window; i++) {
            pixGetPixel(pix6, rankx, y + i, &uval);
            sum1 += uval;
            sum2 += uval * uval;
        }
        ave = norm * sum1;
        variance = norm * sum2 - ave * ave;
        numaAddNumber(na1, variance);
        ymid = y + window / 2;
        if (variance > maxvar) {
            maxvar = variance;
            ymax = ymid;
        }
    }

        /* Plot the windowed variance as a function of the y-value
         * of the window location */
    lept_stderr("maxvar = %f, ymax = %d\n", maxvar, ymax);
    gplotSimple1(na1, GPLOT_PNG, "/tmp/lept/binding/root", NULL);
    pix7 = pixRead("/tmp/lept/binding/root.png");
    pixDisplay(pix7, 0, 800);
    pixaAddPix(pixa, pix7, L_COPY);

        /* Superimpose the variance plot over the image.
         * The variance peak is at the binding. */
    pixRenderPlotFromNumaGen(&pix5, na1, L_VERTICAL_LINE, 3, w - 120, 100, 1,
                                                           0x0000ff00);
    pixDisplay(pix5, 1050, 0);
    pixaAddPix(pixa, pix5, L_COPY);

        /* Bundle the results up in a pdf */
    lept_stderr("Writing pdf output file: /tmp/lept/binding/binding.pdf\n");
    pixaConvertToPdf(pixa, 45, 1.0, 0, 0, "Binding locator",
                     "/tmp/lept/binding/binding.pdf");

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixDestroy(&pix7);
    pixaDestroy(&pixa);
    numaDestroy(&na1);
    return 0;
}
