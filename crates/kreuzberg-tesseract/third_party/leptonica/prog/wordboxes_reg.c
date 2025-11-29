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
 * wordboxes_reg.c
 *
 *   This tests:
 *     - functions that make word boxes
 *     - the function that finds the nearest box to a given box in a boxa
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void MakeWordBoxes1(PIX *pixs, l_float32 scalefact, l_int32 thresh,
                    l_int32 index, L_REGPARAMS *rp);
void MakeWordBoxes2(PIX *pixs, l_float32 scalefact, l_int32 thresh,
                    L_REGPARAMS  *rp);
void TestBoxaAdjacency(PIX *pixs, L_REGPARAMS  *rp);

#define  DO_ALL   1

int main(int    argc,
         char **argv)
{
BOX          *box1, *box2;
BOXA         *boxa1;
BOXAA        *boxaa1;
PIX          *pix1, *pix2, *pix3, *pix4;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

#if DO_ALL
        /* Make word boxes using pixWordMaskByDilation() */
    pix1 = pixRead("lucasta.150.jpg");
    MakeWordBoxes1(pix1, 1.0, 140, 0, rp);  /* 0 */
    MakeWordBoxes1(pix1, 0.6, 140, 1, rp);  /* 1 */
    pixDestroy(&pix1);
#endif

#if DO_ALL
    pix1 = pixRead("zanotti-78.jpg");
    MakeWordBoxes1(pix1, 1.0, 140, 2, rp);  /* 2 */
    MakeWordBoxes1(pix1, 0.6, 140, 3, rp);  /* 3 */
    pixDestroy(&pix1);
#endif

#if DO_ALL
    pix1 = pixRead("words.15.tif");
    MakeWordBoxes1(pix1, 1.0, 140, 4, rp);  /* 4 */
    MakeWordBoxes1(pix1, 0.6, 140, 5, rp);  /* 5 */
    pixDestroy(&pix1);
#endif

#if DO_ALL
    pix1 = pixRead("words.44.tif");
    MakeWordBoxes1(pix1, 1.0, 140, 6, rp);  /* 6 */
    MakeWordBoxes1(pix1, 0.6, 140, 7, rp);  /* 7 */
    pixDestroy(&pix1);
#endif

#if DO_ALL
        /* Make word boxes using the higher-level functions
         * pixGetWordsInTextlines() and pixGetWordBoxesInTextlines() */

    pix1 = pixRead("lucasta.150.jpg");
    MakeWordBoxes2(pix1, 0.7, 140, rp);  /* 8, 9 */
    pixDestroy(&pix1);
#endif

#if DO_ALL
    pix1 = pixRead("zanotti-78.jpg");
    MakeWordBoxes2(pix1, 0.7, 140, rp);  /* 10, 11 */
    pixDestroy(&pix1);
#endif

#if DO_ALL
        /* Test boxa adjacency function */
    pix1 = pixRead("lucasta.150.jpg");
    TestBoxaAdjacency(pix1, rp);  /* 12 - 15 */
    pixDestroy(&pix1);
#endif

#if DO_ALL
        /* Test word and character box finding */
    pix1 = pixRead("zanotti-78.jpg");
    box1 = boxCreate(0, 0, 1500, 700);
    pix2 = pixClipRectangle(pix1, box1, NULL);
    box2 = boxCreate(150, 130, 1500, 355);
    pixFindWordAndCharacterBoxes(pix2, box2, 130, &boxa1, &boxaa1,
                                 "/tmp/lept/testboxes");
    pix3 = pixRead("/tmp/lept/testboxes/words.png");
    pix4 = pixRead("/tmp/lept/testboxes/chars.png");
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 16 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 17 */
    pixDisplayWithTitle(pix3, 200, 1000, NULL, rp->display);
    pixDisplayWithTitle(pix4, 200, 100, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    boxDestroy(&box1);
    boxDestroy(&box2);
    boxaDestroy(&boxa1);
    boxaaDestroy(&boxaa1);
#endif

    return regTestCleanup(rp);
}

void
MakeWordBoxes1(PIX          *pixs,
               l_float32     scalefact,
               l_int32       thresh,
               l_int32       index,
               L_REGPARAMS  *rp)
{
BOXA  *boxa1, *boxa2;
PIX   *pix1, *pix2, *pix3, *pix4, *pix5;
PIXA  *pixa1;

    pix1 = pixScale(pixs, scalefact, scalefact);
    pix2 = pixConvertTo1(pix1, thresh);
    pixa1 = pixaCreate(3);
    pixWordMaskByDilation(pix2, &pix3, NULL, pixa1);
    pix4 = NULL;
    if (pix3) {
        boxa1 = pixConnComp(pix3, NULL, 8);
        boxa2 = boxaTransform(boxa1, 0, 0, 1.0/scalefact, 1.0/scalefact);
        pix4 = pixConvertTo32(pixs);
        pixRenderBoxaArb(pix4, boxa2, 2, 255, 0, 0);
        pix5 = pixaDisplayTiledInColumns(pixa1, 1, 1.0, 25, 2);
        pixDisplayWithTitle(pix5, 200 * index, 0, NULL, rp->display);
        boxaDestroy(&boxa1);
        boxaDestroy(&boxa2);
        pixDestroy(&pix3);
        pixDestroy(&pix5);
    }
    regTestWritePixAndCheck(rp, pix4, IFF_JFIF_JPEG);
    pixDisplayWithTitle(pix4, 200 * index, 800, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix4);
    pixaDestroy(&pixa1);
}

void
MakeWordBoxes2(PIX          *pixs,
               l_float32     scalefact,
               l_int32       thresh,
               L_REGPARAMS  *rp)
{
l_int32  default_minwidth = 10;
l_int32  default_minheight = 10;
l_int32  default_maxwidth = 400;
l_int32  default_maxheight = 70;
l_int32  minwidth, minheight, maxwidth, maxheight;
BOXA    *boxa1, *boxa2;
NUMA    *na;
PIX     *pix1, *pix2, *pix3, *pix4;
PIXA    *pixa;

    minwidth = scalefact * default_minwidth;
    minheight = scalefact * default_minheight;
    maxwidth = scalefact * default_maxwidth;
    maxheight = scalefact * default_maxheight;

        /* Get the word boxes */
    pix1 = pixScale(pixs, scalefact, scalefact);
    pix2 = pixConvertTo1(pix1, thresh);
    pixGetWordsInTextlines(pix2, minwidth, minheight,
                           maxwidth, maxheight, &boxa1, &pixa, &na);
    pixaDestroy(&pixa);
    numaDestroy(&na);
    boxa2 = boxaTransform(boxa1, 0, 0, 1.0 / scalefact, 1.0 / scalefact);
    pix3 = pixConvertTo32(pixs);
    pixRenderBoxaArb(pix3, boxa2, 2, 255, 0, 0);
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);
    pixDisplayWithTitle(pix3, 900, 0, NULL, rp->display);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);

        /* Do it again with this interface.  The result should be the same. */
    pixGetWordBoxesInTextlines(pix2, minwidth, minheight,
                               maxwidth, maxheight, &boxa1, NULL);
    boxa2 = boxaTransform(boxa1, 0, 0, 1.0 / scalefact, 1.0 / scalefact);
    pix4 = pixConvertTo32(pixs);
    pixRenderBoxaArb(pix4, boxa2, 2, 255, 0, 0);
    if (regTestComparePix(rp, pix3, pix4)) {
        L_ERROR("pix not the same", "MakeWordBoxes2");
        pixDisplayWithTitle(pix4, 1200, 0, NULL, rp->display);
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
}

void
TestBoxaAdjacency(PIX          *pixs,
                  L_REGPARAMS  *rp)
{
l_int32  i, j, k, n;
BOX     *box1, *box2;
BOXA    *boxa0, *boxa1, *boxa2;
PIX     *pix0, *pix1, *pix2, *pix3;
NUMAA   *naai, *naad;

    pix0 = pixConvertTo1(pixs, 140);

        /* Make a word mask and remove small components */
    pixWordMaskByDilation(pix0, &pix1, NULL, NULL);
    boxa0 = pixConnComp(pix1, NULL, 8);
    boxa1 = boxaSelectBySize(boxa0, 8, 8, L_SELECT_IF_BOTH,
                             L_SELECT_IF_GT, NULL);
    pix2 = pixConvertTo32(pixs);
    pixRenderBoxaArb(pix2, boxa1, 2, 255, 0, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_JFIF_JPEG);
    pixDisplayWithTitle(pix2, 600, 700, NULL, rp->display);
    pixDestroy(&pix1);

        /* Find the adjacent boxes and their distances */
    boxaFindNearestBoxes(boxa1, L_NON_NEGATIVE, 0, &naai, &naad);
    numaaWrite("/tmp/lept/regout/index.naa", naai);
    regTestCheckFile(rp, "/tmp/lept/regout/index.naa");
    numaaWrite("/tmp/lept/regout/dist.naa", naad);
    regTestCheckFile(rp, "/tmp/lept/regout/dist.naa");

        /* For a few boxes, show the (up to 4) adjacent boxes */
    n = boxaGetCount(boxa1);
    pix3 = pixConvertTo32(pixs);
    for (i = 10; i < n; i += 25) {
        box1 = boxaGetBox(boxa1, i, L_COPY);
        pixRenderBoxArb(pix3, box1, 2, 255, 0, 0);
        boxa2 = boxaCreate(4);
        for (j = 0; j < 4; j++) {
            numaaGetValue(naai, i, j, NULL, &k);
            if (k >= 0) {
                box2 = boxaGetBox(boxa1, k, L_COPY);
                boxaAddBox(boxa2, box2, L_INSERT);
            }
        }
        pixRenderBoxaArb(pix3, boxa2, 2, 0, 255, 0);
        boxDestroy(&box1);
        boxaDestroy(&boxa2);
    }
    regTestWritePixAndCheck(rp, pix3, IFF_JFIF_JPEG);
    pixDisplayWithTitle(pix3, 1100, 700, NULL, rp->display);

    pixDestroy(&pix0);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    boxaDestroy(&boxa0);
    boxaDestroy(&boxa1);
    numaaDestroy(&naai);
    numaaDestroy(&naad);
}

