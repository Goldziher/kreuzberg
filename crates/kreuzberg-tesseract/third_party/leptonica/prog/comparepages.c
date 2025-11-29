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
 * comparepages.c
 *
 *    This compares text pages using the location of word bounding boxes.
 *    The goal is to get a fast and robust determination for whether
 *    two pages are the same.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32  w, h, n, same;
BOXA    *boxa1, *boxa2;
NUMA    *nai1, *nai2;
NUMAA   *naa1, *naa2;
PIX     *pixs, *pixt, *pixb1, *pixb2;

    setLeptDebugOK(1);
    lept_mkdir("lept/comp");

    pixs = pixRead("lucasta.047.jpg");
    pixb1 = pixConvertTo1(pixs, 128);
    pixGetWordBoxesInTextlines(pixb1, 10, 10, 500, 50, &boxa1, &nai1);
    pixt = pixDrawBoxaRandom(pixs, boxa1, 2);
    pixDisplay(pixt, 100, 100);
    pixWrite("/tmp/lept/comp/pixt.png", pixt, IFF_PNG);
    naa1 = boxaExtractSortedPattern(boxa1, nai1);
    numaaWrite("/tmp/lept/comp/naa1.naa", naa1);
    n = numaaGetCount(naa1);
    lept_stderr("Number of textlines = %d\n", n);
    pixDisplay(pixb1, 300, 0);

        /* Translate */
    pixb2 = pixCreateTemplate(pixb1);
    pixGetDimensions(pixb1, &w, &h, NULL);
    pixRasterop(pixb2, 148, 133, w, h, PIX_SRC, pixb1, 0, 0);
    pixDisplay(pixb2, 600, 0);
    pixGetWordBoxesInTextlines(pixb2, 10, 10, 500, 50, &boxa2, &nai2);
    naa2 = boxaExtractSortedPattern(boxa2, nai2);
    numaaCompareImagesByBoxes(naa1, naa2, 5, 10, 150, 150, 20, 20, &same, 1);
    lept_stderr("Translation.  same?: %d\n\n", same);
    boxaDestroy(&boxa2);
    numaDestroy(&nai2);
    pixDestroy(&pixb2);
    numaaDestroy(&naa2);

        /* Aligned part is below h/3 */
    pixb2 = pixCreateTemplate(pixb1);
    pixGetDimensions(pixb1, &w, &h, NULL);
    pixRasterop(pixb2, 0, 0, w, h / 3, PIX_SRC, pixb1, 0, 2 * h / 3);
    pixRasterop(pixb2, 0, h / 3, w, 2 * h / 3, PIX_SRC, pixb1, 0, h / 3);
    pixDisplay(pixb2, 900, 0);
    pixGetWordBoxesInTextlines(pixb2, 10, 10, 500, 50, &boxa2, &nai2);
    naa2 = boxaExtractSortedPattern(boxa2, nai2);
    numaaCompareImagesByBoxes(naa1, naa2, 5, 10, 150, 150, 20, 20, &same, 1);
    lept_stderr("Aligned part below h/3.  same?: %d\n\n", same);
    boxaDestroy(&boxa2);
    numaDestroy(&nai2);
    pixDestroy(&pixb2);
    numaaDestroy(&naa2);

        /* Top and bottom switched; no aligned parts */
    pixb2 = pixCreateTemplate(pixb1);
    pixGetDimensions(pixb1, &w, &h, NULL);
    pixRasterop(pixb2, 0, 0, w, h / 3, PIX_SRC, pixb1, 0, 2 * h / 3);
    pixRasterop(pixb2, 0, h / 3, w, 2 * h / 3, PIX_SRC, pixb1, 0, 0);
    pixDisplay(pixb2, 1200, 0);
    pixGetWordBoxesInTextlines(pixb2, 10, 10, 500, 50, &boxa2, &nai2);
    naa2 = boxaExtractSortedPattern(boxa2, nai2);
    numaaCompareImagesByBoxes(naa1, naa2, 5, 10, 150, 150, 20, 20, &same, 1);
    lept_stderr("Top/Bot switched; no alignment.  Same?: %d\n", same);
    boxaDestroy(&boxa2);
    numaDestroy(&nai2);
    pixDestroy(&pixb2);
    numaaDestroy(&naa2);

    boxaDestroy(&boxa1);
    numaDestroy(&nai1);
    pixDestroy(&pixs);
    pixDestroy(&pixb1);
    pixDestroy(&pixt);
    numaaDestroy(&naa1);
    return 0;
}
