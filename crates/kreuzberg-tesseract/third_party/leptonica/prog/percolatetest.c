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
 *  percolatetest.c
 *
 *  This tests the code that keeps track of connected components as
 *  pixels are added (randomly, here) to a pix.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static PIX *PixDisplayWithColormap(PIX *pixs, l_int32 repl);


l_int32 main(int    argc,
             char **argv)
{
l_int32  i, x, y, ncc, npta;
NUMA    *na1;
PIX     *pixs, *pix1, *pix2, *pix3;
PIXA    *pixa;
PTAA    *ptaa;

    if (argc != 1) {
        lept_stderr(" Syntax: percolatetest\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/perc");

        /* Fill in a tiny pix; 4 connected */
    pixa = pixaCreate(0);
    pixs = pixCreate(5, 5, 1);
    pixConnCompIncrInit(pixs, 4, &pix1, &ptaa, &ncc);
    lept_stderr("ncc = %d, npta = %d\n", ncc, ptaaGetCount(ptaa));
    srand(26);
    for (i = 0; i < 50; i++) {
        pixGetRandomPixel(pix1, NULL, &x, &y);
        pixConnCompIncrAdd(pix1, ptaa, &ncc, x, y, 2);
        npta = ptaaGetCount(ptaa);
        lept_stderr("x,y = (%d,%d), num c.c. = %d, num pta = %d\n",
                    x, y, ncc, npta);
        pix2 = PixDisplayWithColormap(pix1, 20);
        pixaAddPix(pixa, pix2, L_INSERT);
    }
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 30, 2);
    pixDisplay(pix3, 0, 0);
    pixWrite("/tmp/lept/perc/file1.png", pix3, IFF_PNG);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    ptaaDestroy(&ptaa);

        /* Fill in a tiny pix; 8 connected */
    pixa = pixaCreate(0);
    pixs = pixCreate(5, 5, 1);
    pixConnCompIncrInit(pixs, 8, &pix1, &ptaa, &ncc);
    lept_stderr("ncc = %d, npta = %d\n", ncc, ptaaGetCount(ptaa));
    srand(26);
    for (i = 0; i < 50; i++) {
        pixGetRandomPixel(pix1, NULL, &x, &y);
        pixConnCompIncrAdd(pix1, ptaa, &ncc, x, y, 2);
        npta = ptaaGetCount(ptaa);
        lept_stderr("x,y = (%d,%d), num c.c. = %d, num pta = %d\n",
                    x, y, ncc, npta);
        pix2 = PixDisplayWithColormap(pix1, 20);
        pixaAddPix(pixa, pix2, L_INSERT);
    }
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 30, 2);
    pixDisplay(pix3, 0, 560);
    pixWrite("/tmp/lept/perc/file2.png", pix3, IFF_PNG);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    ptaaDestroy(&ptaa);

        /* Fill in a small pix; 4 connected */
    na1 = numaCreate(700);
    pixa = pixaCreate(0);
    pixs = pixCreate(20, 20, 1);
    pixConnCompIncrInit(pixs, 4, &pix1, &ptaa, &ncc);
    lept_stderr("ncc = %d, npta = %d\n", ncc, ptaaGetCount(ptaa));
    srand(26);
    for (i = 0; i < 700; i++) {
        pixGetRandomPixel(pix1, NULL, &x, &y);
        pixConnCompIncrAdd(pix1, ptaa, &ncc, x, y, 2);
        numaAddNumber(na1, ncc);
        npta = ptaaGetCount(ptaa);
        if (i < 100) {
            lept_stderr("x,y = (%d,%d), num c.c. = %d, num pta = %d\n",
                        x, y, ncc, npta);
        }
        if (i % 30 == 1) {
            pix2 = PixDisplayWithColormap(pix1, 5);
            pixaAddPix(pixa, pix2, L_INSERT);
        }
    }
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 30, 2);
    pixDisplay(pix3, 0, 0);
    pixWrite("/tmp/lept/perc/file3.png", pix3, IFF_PNG);
    gplotSimple1(na1, GPLOT_PNG, "/tmp/lept/plot1",
                 "Number of components: 4 cc");
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    ptaaDestroy(&ptaa);
    numaDestroy(&na1);

        /* Fill in a small pix; 8 connected */
    na1 = numaCreate(700);
    pixa = pixaCreate(0);
    pixs = pixCreate(20, 20, 1);
    pixConnCompIncrInit(pixs, 8, &pix1, &ptaa, &ncc);
    lept_stderr("ncc = %d, npta = %d\n", ncc, ptaaGetCount(ptaa));
    srand(26);
    for (i = 0; i < 700; i++) {
        pixGetRandomPixel(pix1, NULL, &x, &y);
        pixConnCompIncrAdd(pix1, ptaa, &ncc, x, y, 2);
        numaAddNumber(na1, ncc);
        npta = ptaaGetCount(ptaa);
        if (i < 100) {
             lept_stderr("x,y = (%d,%d), num c.c. = %d, num pta = %d\n",
                         x, y, ncc, npta);
        }
        if (i % 30 == 1) {
            pix2 = PixDisplayWithColormap(pix1, 5);
            pixaAddPix(pixa, pix2, L_INSERT);
        }
    }
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 30, 2);
    pixDisplay(pix3, 0, 360);
    pixWrite("/tmp/lept/perc/file4.png", pix3, IFF_PNG);
    gplotSimple1(na1, GPLOT_PNG, "/tmp/lept/plot2",
                 "Number of components: 8 cc");
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    ptaaDestroy(&ptaa);
    numaDestroy(&na1);

        /* Fill in a larger pix; 4 connected */
    pixa = pixaCreate(0);
    na1 = numaCreate(20000);
    pixs = pixCreate(195, 56, 1);
    pixConnCompIncrInit(pixs, 4, &pix1, &ptaa, &ncc);
    lept_stderr("ncc = %d, npta = %d\n", ncc, ptaaGetCount(ptaa));
    srand(26);
    for (i = 0; i < 20000; i++) {
        pixGetRandomPixel(pix1, NULL, &x, &y);
        pixConnCompIncrAdd(pix1, ptaa, &ncc, x, y, 3);
        npta = ptaaGetCount(ptaa);
        numaAddNumber(na1, ncc);
        if (i % 500 == 1) {
            pix2 = PixDisplayWithColormap(pix1, 3);
            pixaAddPix(pixa, pix2, L_INSERT);
            lept_stderr("x,y = (%d,%d), num c.c. = %d, num pta = %d\n",
                        x, y, ncc, npta);
        }
    }
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 30, 2);
    pixDisplay(pix3, 0, 0);
    pixWrite("/tmp/lept/perc/file5.png", pix3, IFF_PNG);
    gplotSimple1(na1, GPLOT_PNG, "/tmp/lept/plot3",
                 "Number of components: 4 connected");
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    ptaaDestroy(&ptaa);
    numaDestroy(&na1);

        /* Fill in a larger pix; 8 connected */
    pixa = pixaCreate(0);
    na1 = numaCreate(20000);
    pixs = pixCreate(195, 56, 1);
    pixConnCompIncrInit(pixs, 8, &pix1, &ptaa, &ncc);
    srand(26);
    lept_stderr("ncc = %d, npta = %d\n", ncc, ptaaGetCount(ptaa));
    for (i = 0; i < 20000; i++) {
        pixGetRandomPixel(pix1, NULL, &x, &y);
        pixConnCompIncrAdd(pix1, ptaa, &ncc, x, y, 3);
        npta = ptaaGetCount(ptaa);
        numaAddNumber(na1, ncc);
        if (i % 500 == 1) {
            pix2 = PixDisplayWithColormap(pix1, 3);
            pixaAddPix(pixa, pix2, L_INSERT);
            lept_stderr("x,y = (%d,%d), num c.c. = %d, num pta = %d\n",
                        x, y, ncc, npta);
        }
    }
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 30, 2);
    pixDisplay(pix3, 340, 0);
    pixWrite("/tmp/lept/perc/file6.png", pix3, IFF_PNG);
    gplotSimple1(na1, GPLOT_PNG, "/tmp/lept/plot4",
                 "Number of components: 8 connected");
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    ptaaDestroy(&ptaa);
    numaDestroy(&na1);

        /* Fill in a larger pix; 8 connected; init with feyn-word.tif */
    pixa = pixaCreate(0);
    na1 = numaCreate(20000);
    pixs = pixRead("feyn-word.tif");
    pixConnCompIncrInit(pixs, 8, &pix1, &ptaa, &ncc);
    srand(26);
    lept_stderr("ncc = %d, npta = %d\n", ncc, ptaaGetCount(ptaa));
    for (i = 0; i < 20000; i++) {
        pixGetRandomPixel(pix1, NULL, &x, &y);
        pixConnCompIncrAdd(pix1, ptaa, &ncc, x, y, 3);
        npta = ptaaGetCount(ptaa);
        numaAddNumber(na1, ncc);
        if (i % 500 == 1) {
            pix2 = PixDisplayWithColormap(pix1, 3);
            pixaAddPix(pixa, pix2, L_INSERT);
            lept_stderr("x,y = (%d,%d), num c.c. = %d, num pta = %d\n",
                        x, y, ncc, npta);
        }
    }
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1500, 1.0, 0, 30, 2);
    pixDisplay(pix3, 0, 0);
    pixWrite("/tmp/lept/perc/file7.png", pix3, IFF_PNG);
    gplotSimple1(na1, GPLOT_PNG, "/tmp/lept/plot5",
                 "Number of components: 8 connected");
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    ptaaDestroy(&ptaa);
    numaDestroy(&na1);

        /* Do 10M pixel adds on an 8M pixel image!
         * This gets it down to about 385 8-connected components.
         * With 18.3M pixel adds, you finally arrive at 1 component.
         * Speed: this does about 1.3M pixel adds/sec.  Most of the time
         * is used to write the 280MB plot data file and then generate
         * the plot (percolate-4cc.png, percolate-8cc.png). */
#if 0
    pixs = pixRead("feyn.tif");
//    pixs = pixCreate(2500, 3200, 1);
    na1 = numaCreate(10000000);
    pixConnCompIncrInit(pixs, 4, &pix1, &ptaa, &ncc);
    pix2 = PixDisplayWithColormap(pix1, 1);
    pixDisplay(pix2, 0, 0);
    pixDestroy(&pix2);
    lept_stderr("ncc = %d, npta = %d\n", ncc, ptaaGetCount(ptaa));
    lept_stderr("Now add 10M points: this takes about 7 seconds!\n");
    for (i = 0; i < 10000000; i++) {
        pixGetRandomPixel(pix1, NULL, &x, &y);
        pixConnCompIncrAdd(pix1, ptaa, &ncc, x, y, 0);
        numaAddNumber(na1, ncc);
    }

    lept_stderr("Plot the 10M points: this takes about 20 seconds\n");
    gplotSimple1(na1, GPLOT_PNG, "/tmp/lept/plot6",
                 "Number of components: 4 connected, 8 million pixels");
    pix3 = pixRead("/tmp/lept/plot6.png");
    pixDisplay(pix3, 500, 0);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixaDestroy(&pixa);
    ptaaDestroy(&ptaa);
    numaDestroy(&na1);
#endif
    return 0;
}


    /* This displays a pix where the pixel values are 32 bit labels
     * using a random colormap whose index is the LSB of each pixel value,
     * and where white is at index 0. */
PIX *
PixDisplayWithColormap(PIX *pixs, l_int32 repl)
{
PIX      *pix1, *pixd;
PIXCMAP  *cmap;

   cmap = pixcmapCreateRandom(8, 0, 0);
   pixcmapResetColor(cmap, 0, 255, 255, 255);
   pix1 = pixConvert32To8(pixs, L_LS_TWO_BYTES, L_LS_BYTE);
   pixd = pixExpandReplicate(pix1, repl);
   pixSetColormap(pixd, cmap);
   pixDestroy(&pix1);
   return pixd;
}

