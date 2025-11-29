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
 *   partition_reg.c
 *
 *     Tests partitioning of white space into rectangles, with examples
 *     sorted by height and by area.  We use a maximum of 10 rectangles,
 *     with no overlap allowed.
 *
 *     This partitions 1 bpp images using boxaGetWhiteblocks().
 *     For a general testing program on arbitrary images, use partitiontest.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void TestPartition(L_REGPARAMS *rp, const char *fname, l_int32 sorttype,
                          l_int32 maxboxes, l_float32 ovlap,
                          const char *fileout, PIXA  *pixad);

int main(int    argc,
         char **argv)
{
PIX          *pix1;
PIXA         *pixad;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBTIFF)
    L_ERROR("This test requires libtiff to run.\n", __func__);
    exit(77);
#endif
#if !defined(HAVE_LIBJPEG)
    L_ERROR("This test requires libjpeg to run.\n", __func__);
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    l_pdfSetDateAndVersion(0);
    lept_mkdir("lept/part");

    pixad = pixaCreate(4);  /* only for display */
    TestPartition(rp, "test8.jpg", L_SORT_BY_HEIGHT, 20, 0.0f, "test0.pdf",
                  pixad);
    TestPartition(rp, "test8.jpg", L_SORT_BY_AREA, 20, 0.0f, "test1.pdf",
                  pixad);
    TestPartition(rp, "test8.jpg", L_SORT_BY_AREA, 20, 0.4f, "test2.pdf",
                  pixad);
    TestPartition(rp, "feyn-fract.tif", L_SORT_BY_AREA, 20, 0.0f, "test3.pdf",
                  pixad);

        /* If display requested, make a tiled image of all the results */
    if (rp->display) {
        pix1 = pixaDisplayTiledInRows(pixad, 32, 2000, 0.7f, 0, 30, 3);
        pixDisplay(pix1, 100, 100);
        pixWrite("/tmp/lept/part/tiled_result.png", pix1, IFF_PNG);
        pixDestroy(&pix1);
    }

    pixaDestroy(&pixad);
    return regTestCleanup(rp);
}


static void
TestPartition(L_REGPARAMS *rp, const char *fname, l_int32 sorttype,
              l_int32 maxboxes, l_float32 ovlap, const char *fileout,
              PIXA  *pixad)
{
char     pathout[256];
l_int32  w, h;
BOX     *box;
BOXA    *boxa1, *boxa2, *boxa3;
PIX     *pix, *pix1, *pix2, *pix3, *pix4;
PIXA    *pixa;

    pixa = pixaCreate(0);
    pix = pixRead(fname);
    pixaAddPix(pixa, pix, L_COPY);
    pix1 = pixConvertTo1(pix, 128);
    pixDilateBrick(pix1, pix1, 5, 5);
    pixaAddPix(pixa, pix1, L_COPY);
    boxa1 = pixConnComp(pix1, NULL, 4);
    pixGetDimensions(pix1, &w, &h, NULL);
    box = boxCreate(0, 0, w, h);
    startTimer();
    boxaPermuteRandom(boxa1, boxa1);
    boxa2 = boxaSelectBySize(boxa1, 500, 500, L_SELECT_IF_BOTH,
                             L_SELECT_IF_LT, NULL);
    pix2 = pixCopyWithBoxa(pix1, boxa2, L_SET_WHITE);
    pixaAddPix(pixa, pix2, L_COPY);
    boxa3 = boxaGetWhiteblocks(boxa2, box, sorttype, maxboxes, ovlap,
                               200, 0.15f, 20000);

        /* Display box outlines in random colors in a cmapped image */
    pix3 = pixDrawBoxaRandom(pix2, boxa3, 7);
    pixaAddPix(pixa, pix3, L_INSERT);

        /* Display boxes in random colors in a cmapped image */
    pix3 = pixPaintBoxaRandom(pix2, boxa3);
    pixaAddPix(pixa, pix3, L_INSERT);

        /* Make and check the output pdf file */
    snprintf(pathout, sizeof(pathout), "/tmp/lept/part/%s", fileout);
    lept_stderr("Writing to: %s\n", pathout);
    pixaConvertToPdf(pixa, 300, 1.0f, L_FLATE_ENCODE, 0, fileout, pathout);
    regTestCheckFile(rp, pathout);

        /* Display in a column */
    if (rp->display) {
        pix4 = pixaDisplayTiledInColumns(pixa, 1, 0.7f, 20, 2);
        pixaAddPix(pixad, pix4, L_INSERT);
    }

    pixDestroy(&pix);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixaDestroy(&pixa);
    boxDestroy(&box);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    boxaDestroy(&boxa3);
}
