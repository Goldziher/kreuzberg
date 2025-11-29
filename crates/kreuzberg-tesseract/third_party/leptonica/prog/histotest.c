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
 * histotest.c
 *
 *    Makes histograms of grayscale and color pixels
 *    from a pix.  For RGB color, this uses
 *    rgb --> octcube indexing.
 *
 *       histotest filein sigbits
 *
 *    where the number of octcubes is 8^(sigbits)
 *
 *    For gray, sigbits is ignored.
 *
 *    Also tests pixThresholdByHisto(), sliding the histogram fully
 *    to the left and right until, in each case, all numbers are 0.
 *    This has been valgrinded, to show that no memory errors occur.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char    *filein;
l_int32  d, sigbits, i, val;
BOX     *box1;
GPLOT   *gplot;
NUMA    *na1, *na2;
PIX     *pixs, *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIXA    *pixa1;

    if (argc != 3)
        return ERROR_INT(" Syntax:  histotest filein sigbits", __func__, 1);
    filein = argv[1];
    sigbits = atoi(argv[2]);

    setLeptDebugOK(1);
    lept_mkdir("lept/histo");

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    d = pixGetDepth(pixs);
    if (d != 8 && d != 32)
        return ERROR_INT("depth not 8 or 32 bpp", __func__, 1);

    if (d == 32) {
        startTimer();
        if ((na1 = pixOctcubeHistogram(pixs, sigbits, NULL)) == NULL)
            return ERROR_INT("na1 not made", __func__, 1);
        lept_stderr("histo time = %7.3f sec\n", stopTimer());
        gplot = gplotCreate("/tmp/lept/histo/color", GPLOT_PNG,
                            "color histogram with octcube indexing",
                            "octcube index", "number of pixels in cube");
        gplotAddPlot(gplot, NULL, na1, GPLOT_LINES, "input pix");
        gplotMakeOutput(gplot);
        gplotDestroy(&gplot);
        l_fileDisplay("/tmp/lept/histo/color.png", 100, 100, 1.0);
    }
    else {
        if ((na1 = pixGetGrayHistogram(pixs, 1)) == NULL)
            return ERROR_INT("na1 not made", __func__, 1);
        numaWrite("/tmp/junk.na", na1);
        gplot = gplotCreate("/tmp/lept/histo/gray", GPLOT_PNG,
                            "grayscale histogram", "gray value",
                            "number of pixels");
        gplotSetScaling(gplot, GPLOT_LOG_SCALE_Y);
        gplotAddPlot(gplot, NULL, na1, GPLOT_LINES, "input pix");
        gplotMakeOutput(gplot);
        gplotDestroy(&gplot);
        l_fileDisplay("/tmp/lept/histo/gray.png", 100, 100, 1.0);
    }

    pixDestroy(&pixs);
    numaDestroy(&na1);

        /* Test behavior of pixThresholdByHisto() */
#if 0  /* for valgrind, use pnm instead of jpg */
    pix1 = pixRead("lyra.005.jpg");
    pixWrite("/tmp/lyra.005.pnm", pix1, IFF_PNM);
#endif
/*    pix1 = pixRead("/tmp/lyra.005.pnm"); */
    pixs = pixRead("lyra.005.jpg");
    box1 = boxCreate(0, 173, 350, 580);
    pix1 = pixClipRectangle(pixs, box1, 0);
    pix2 = pixRotateOrth(pix1, 1);
    pix3 = pixConvertTo8(pix2, 0);
    pixThresholdByHisto(pix3, 1, 0, 0, &val, &pix4, &na1, &pix5);
    lept_stderr("val = %d\n", val);
    pixa1 = pixaCreate(4);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixaAddPix(pixa1, pix3, L_INSERT);
    pixaAddPix(pixa1, pix4, L_INSERT);
    pixaAddPix(pixa1, pix5, L_INSERT);
    pix6 = pixaDisplayTiledInColumns(pixa1, 1,1.0, 25, 2);
    pixDisplay(pix6, 200, 200);
    na2 = numaCopy(na1);
    for (i = 0; i < 260; i++) {
        numaRemoveNumber(na1, 0);
        numaAddNumber(na1, 0);
        numaFindLocForThreshold(na1, 0, &val, NULL);
        lept_stderr("val = %d\n", val);
    }
    for (i = 0; i < 260; i++) {
        numaInsertNumber(na2, 0, 0);
        numaRemoveNumber(na2, 256);
        numaFindLocForThreshold(na2, 0, &val, NULL);
        lept_stderr("val = %d\n", val);
    }
    numaDestroy(&na1);
    numaDestroy(&na2);
    pixaDestroy(&pixa1);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix6);
    boxDestroy(&box1);

    return 0;
}

