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
 *   partitiontest.c
 *
 *      partitiontest <fname> type [maxboxes  ovlap]
 *
 *  where type is:
 *      5:   L_SORT_BY_WIDTH
 *      6:   L_SORT_BY_HEIGHT
 *      7:   L_SORT_BY_MIN_DIMENSION
 *      8:   L_SORT_BY_MAX_DIMENSION
 *      9:   L_SORT_BY_PERIMETER
 *      10:  L_SORT_BY_AREA
 *
 *  This partitions the input (1 bpp) image using boxaGetWhiteblocks(),
 *  which is an elegant but flawed method in computational geometry to
 *  extract the significant rectangular white blocks in a 1 bpp image.
 *  See partition.c for details.
 *
 *  It then sorts the regions according to the selected input type,
 *  and displays the top sorted blocks in several different ways:
 *    - as outlines or solid filled regions
 *    - with random or specific colors
 *    - as an rgb or colormapped image.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char      *filename;
l_int32    w, h, type, maxboxes;
l_float32  ovlap;
BOX       *box;
BOXA      *boxa, *boxat, *boxad;
PIX       *pix, *pixs, *pix0, *pix1, *pix2;
PIXA      *pixa;

    if (argc != 3 && argc != 5)
        return ERROR_INT("syntax: partitiontest <fname> type [maxboxes ovlap]",
                         __func__, 1);
    filename = argv[1];
    type = atoi(argv[2]);

    if (type == L_SORT_BY_WIDTH) {
        lept_stderr("Sorting by width:\n");
    } else if (type == L_SORT_BY_HEIGHT) {
        lept_stderr("Sorting by height:\n");
    } else if (type == L_SORT_BY_MAX_DIMENSION) {
        lept_stderr("Sorting by maximum dimension:\n");
    } else if (type == L_SORT_BY_MIN_DIMENSION) {
        lept_stderr("Sorting by minimum dimension:\n");
    } else if (type == L_SORT_BY_PERIMETER) {
        lept_stderr("Sorting by perimeter:\n");
    } else if (type == L_SORT_BY_AREA) {
        lept_stderr("Sorting by area:\n");
    } else {
        lept_stderr("Use one of the following for 'type':\n"
                    "     5:   L_SORT_BY_WIDTH\n"
                    "     6:   L_SORT_BY_HEIGHT\n"
                    "     7:   L_SORT_BY_MIN_DIMENSION\n"
                    "     8:   L_SORT_BY_MAX_DIMENSION\n"
                    "     9:   L_SORT_BY_PERIMETER\n"
                    "    10:   L_SORT_BY_AREA\n");
        return ERROR_INT("invalid type: see source", __func__, 1);
    }
    if (argc == 5) {
        maxboxes = atoi(argv[3]);
        ovlap = atof(argv[4]);
    } else {
        maxboxes = 100;
        ovlap = 0.2;
    }

    setLeptDebugOK(1);
    pixa = pixaCreate(0);
    pix = pixRead(filename);
    pixaAddPix(pixa, pix, L_COPY);
    pixs = pixConvertTo1(pix, 128);
    pixDilateBrick(pixs, pixs, 5, 5);
    pixaAddPix(pixa, pixs, L_COPY);
    boxa = pixConnComp(pixs, NULL, 4);
    pixGetDimensions(pixs, &w, &h, NULL);
    box = boxCreate(0, 0, w, h);
    startTimer();
    boxaPermuteRandom(boxa, boxa);
    boxat = boxaSelectBySize(boxa, 500, 500, L_SELECT_IF_BOTH,
                             L_SELECT_IF_LT, NULL);
    pix0 = pixCopyWithBoxa(pixs, boxat, L_SET_WHITE);
    pixaAddPix(pixa, pix0, L_COPY);
    boxad = boxaGetWhiteblocks(boxat, box, type, maxboxes, ovlap,
                               200, 0.15, 20000);
    lept_stderr("Time: %7.3f sec\n", stopTimer());
/*    boxaWriteStderr(boxad); */

        /* Display box outlines in a single color in a cmapped image */
    pix1 = pixDrawBoxa(pix0, boxad, 7, 0xe0708000);
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Display box outlines in a single color in an RGB image */
    pix1 = pixConvertTo8(pix0, FALSE);
    pix2 = pixDrawBoxa(pix1, boxad, 7, 0x40a0c000);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pix1);

        /* Display box outlines with random colors in a cmapped image */
    pix1 = pixDrawBoxaRandom(pix0, boxad, 7);
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Display box outlines with random colors in an RGB image */
    pix1 = pixConvertTo8(pix0, FALSE);
    pix2 = pixDrawBoxaRandom(pix1, boxad, 7);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pix1);

        /* Display boxes in the same color in a cmapped image */
    pix1 = pixPaintBoxa(pix0, boxad, 0x60e0a000);
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Display boxes in the same color in an RGB image */
    pix1 = pixConvertTo8(pix0, FALSE);
    pix2 = pixPaintBoxa(pix2, boxad, 0xc030a000);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pix1);

        /* Display boxes in random colors in a cmapped image */
    pix1 = pixPaintBoxaRandom(pix0, boxad);
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Display boxes in random colors in an RGB image */
    pix1 = pixConvertTo8(pix0, FALSE);
    pix2 = pixPaintBoxaRandom(pix1, boxad);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixDestroy(&pix1);

    lept_stderr("Writing to: /tmp/lept/part/partition.pdf\n");
    lept_mkdir("lept/part");
    pixaConvertToPdf(pixa, 300, 1.0, L_FLATE_ENCODE, 0, "Partition test",
                     "/tmp/lept/part/partition.pdf");
    pixaDestroy(&pixa);

    pixDestroy(&pix);
    pixDestroy(&pixs);
    pixDestroy(&pix0);
    boxDestroy(&box);
    boxaDestroy(&boxa);
    boxaDestroy(&boxat);
    boxaDestroy(&boxad);
    return 0;
}
