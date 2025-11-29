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
 * wordsinorder.c
 *
 *     wordsinorder dirin rootname [firstpage npages]
 *
 *         dirin:  directory of input pages
 *         rootname: used for naming the two output files (templates
 *                   and c.c. data)
 *         firstpage: <optional> 0-based; default is 0
 *         npages: <optional> use 0 for all pages; default is 0
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Input variables */
static const l_int32  MIN_WORD_WIDTH = 6;
static const l_int32  MIN_WORD_HEIGHT = 4;
static const l_int32  MAX_WORD_WIDTH = 500;
static const l_int32  MAX_WORD_HEIGHT = 100;

#define   BUF_SIZE                  512
#define   RENDER_PAGES              1

int main(int    argc,
         char **argv)
{
char      filename[BUF_SIZE];
char     *dirin, *rootname, *fname;
l_int32   i, j, w, h, firstpage, npages, nfiles, ncomp;
l_int32   index, ival, rval, gval, bval;
BOX      *box;
BOXA     *boxa;
BOXAA    *baa;
NUMA     *nai;
NUMAA    *naa;
SARRAY   *safiles;
PIX      *pixs, *pix1, *pix2, *pixd;
PIXCMAP  *cmap;

    if (argc != 3 && argc != 5)
        return ERROR_INT(
            " Syntax: wordsinorder dirin rootname [firstpage, npages]",
            __func__, 1);
    dirin = argv[1];
    rootname = argv[2];
    if (argc == 3) {
        firstpage = 0;
        npages = 0;
    }
    else {
        firstpage = atoi(argv[3]);
        npages = atoi(argv[4]);
    }
    setLeptDebugOK(1);

        /* Compute the word bounding boxes at 2x reduction, along with
         * the textlines that they are in. */
    safiles = getSortedPathnamesInDirectory(dirin, NULL, firstpage, npages);
    nfiles = sarrayGetCount(safiles);
    baa = boxaaCreate(nfiles);
    naa = numaaCreate(nfiles);
    for (i = 0; i < nfiles; i++) {
        fname = sarrayGetString(safiles, i, L_NOCOPY);
        if ((pixs = pixRead(fname)) == NULL) {
            L_WARNING("image file %d not read\n", __func__, i);
            continue;
        }
        pix1 = pixReduceRankBinary2(pixs, 1, NULL);
        pixGetWordBoxesInTextlines(pix1, MIN_WORD_WIDTH, MIN_WORD_HEIGHT,
                                   MAX_WORD_WIDTH, MAX_WORD_HEIGHT,
                                   &boxa, &nai);
        boxaaAddBoxa(baa, boxa, L_INSERT);
        numaaAddNuma(naa, nai, L_INSERT);
        pixDestroy(&pix1);

#if  RENDER_PAGES
            /* Show the results on a 2x reduced image, where each
             * word is outlined and the color of the box depends on the
             * computed textline. */
        pix1 = pixReduceRankBinary2(pixs, 2, NULL);
        pixGetDimensions(pix1, &w, &h, NULL);
        pixd = pixCreate(w, h, 8);
        cmap = pixcmapCreateRandom(8, 1, 1);  /* first color is black */
        pixSetColormap(pixd, cmap);

        pix2 = pixUnpackBinary(pix1, 8, 1);
        pixRasterop(pixd, 0, 0, w, h, PIX_SRC | PIX_DST, pix2, 0, 0);
        ncomp = boxaGetCount(boxa);
        for (j = 0; j < ncomp; j++) {
            box = boxaGetBox(boxa, j, L_CLONE);
            numaGetIValue(nai, j, &ival);
            index = 1 + (ival % 254);  /* omit black and white */
            pixcmapGetColor(cmap, index, &rval, &gval, &bval);
            pixRenderBoxArb(pixd, box, 2, rval, gval, bval);
            boxDestroy(&box);
        }

        snprintf(filename, BUF_SIZE, "%s.%05d", rootname, i);
        lept_stderr("filename: %s\n", filename);
        pixWrite(filename, pixd, IFF_PNG);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pixs);
        pixDestroy(&pixd);
#endif  /* RENDER_PAGES */
    }

    boxaaDestroy(&baa);
    numaaDestroy(&naa);
    sarrayDestroy(&safiles);
    return 0;
}

