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
 * arabic_lines.c
 *
 *   Demonstrates some segmentation techniques and display options.
 *   To see the results in one image: /tmp/lept/lineseg/result.png.
 *
 *   This demonstration shows many different operations.  However,
 *   better results may be obtained from pixExtractLines()
 *   which is a much simpler function.  See testmisc1.c for examples.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"


/* Hit-miss transform that splits lightly touching lines */
static const char *seltext = "xxxxxxx"
                             "   x   "
                             "   x   "
                             "   x   "
                             "   x   "
                             "   x   "
                             "   x   "
                             "   x   "
                             "o  X  o"
                             "   x   "
                             "   x   "
                             "   x   "
                             "   x   "
                             "   x   "
                             "   x   "
                             "   x   "
                             "xxxxxxx";

int main(int    argc,
         char **argv)
{
l_int32    w, h, d, w2, h2, i, ncols, same;
l_float32  angle, conf;
BOX       *box;
BOXA      *boxa1, *boxa2;
PIX       *pix, *pixs, *pixb, *pixb2;
PIX       *pix1, *pix2, *pix3, *pix4;
PIXA      *pixam;  /* mask with a single component over each column */
PIXA      *pixa, *pixa1, *pixa2;
PIXAA     *pixaa, *pixaa2;
SEL       *selsplit;

    if (argc != 1)
        return ERROR_INT(" Syntax:  arabic_lines", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/lineseg");
    pixa = pixaCreate(0);

        /* Binarize input */
    pixs = pixRead("arabic.png");
    pixGetDimensions(pixs, &w, &h, &d);
    pix = pixConvertTo1(pixs, 128);
    pixDestroy(&pixs);

        /* Deskew */
    pixb = pixFindSkewAndDeskew(pix, 1, &angle, &conf);
    pixDestroy(&pix);
    lept_stderr("Skew angle: %7.2f degrees; %6.2f conf\n", angle, conf);
    pixaAddPix(pixa, pixb, L_INSERT);

        /* Use full image morphology to find columns, at 2x reduction.
           This only works for very simple layouts where each column
           of text extends the full height of the input image.  */
    pixb2 = pixReduceRankBinary2(pixb, 2, NULL);
    pix1 = pixMorphCompSequence(pixb2, "c5.500 + o20.20", 0);
    boxa1 = pixConnComp(pix1, &pixam, 8);
    ncols = boxaGetCount(boxa1);
    lept_stderr("Num columns: %d\n", ncols);
    pixaAddPix(pixa, pix1, L_INSERT);
    boxaDestroy(&boxa1);

        /* Use selective region-based morphology to get the textline mask. */
    pixa2 = pixaMorphSequenceByRegion(pixb2, pixam, "c100.3 + o30.1", 0, 0);
    pixGetDimensions(pixb2, &w2, &h2, NULL);
    pix2 = pixaDisplay(pixa2, w2, h2);
    pixaAddPix(pixa, pix2, L_INSERT);
    pixaDestroy(&pixam);
    pixDestroy(&pixb2);

        /* Some of the lines may be touching, so use a HMT to split the
           lines in each column, and use a pixaa to save the results. */
    selsplit = selCreateFromString(seltext, 17, 7, "selsplit");
    pixaa = pixaaCreate(ncols);
    for (i = 0; i < ncols; i++) {
        pix2 = pixaGetPix(pixa2, i, L_CLONE);
        box = pixaGetBox(pixa2, i, L_COPY);
        pix3 = pixHMT(NULL, pix2, selsplit);
        pixXor(pix3, pix3, pix2);
        boxa2 = pixConnComp(pix3, &pixa1, 8);
        pixaaAddPixa(pixaa, pixa1, L_INSERT);
        pixaaAddBox(pixaa, box, L_INSERT);
        pix4 = pixaDisplayRandomCmap(pixa1, 0, 0);
        pixaAddPix(pixa, pix4, L_INSERT);
        lept_stderr("Num textlines in col %d: %d\n", i, boxaGetCount(boxa2));
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        boxaDestroy(&boxa2);
    }
    pixaDestroy(&pixa2);

        /* Visual output */
    pix2 = selDisplayInPix(selsplit, 31, 2);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixaDisplayTiledAndScaled(pixa, 32, 400, 3, 0, 35, 3);
    pixWrite("/tmp/lept/lineseg/result.png", pix3, IFF_PNG);
    pixDisplay(pix3, 100, 100);
    pixaDestroy(&pixa);
    pixDestroy(&pix3);
    selDestroy(&selsplit);

        /* Test pixaa I/O */
    pixaaWrite("/tmp/lept/lineseg/pixaa", pixaa);
    pixaa2 = pixaaRead("/tmp/lept/lineseg/pixaa");
    pixaaWrite("/tmp/lept/lineseg/pixaa2", pixaa2);
    filesAreIdentical("/tmp/lept/lineseg/pixaa", "/tmp/lept/lineseg/pixaa2",
                      &same);
    if (!same)
       L_ERROR("pixaa I/O failure\n", __func__);
    pixaaDestroy(&pixaa2);

        /* Test pixaa display */
    pix2 = pixaaDisplay(pixaa, w2, h2);
    pixWrite("/tmp/lept/lineseg/textlines.png", pix2, IFF_PNG);
    pixaaDestroy(&pixaa);
    pixDestroy(&pix2);

    return 0;
}

