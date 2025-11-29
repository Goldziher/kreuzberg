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
 *  histoduptest.c
 *
 *  This demonstrates two things:
 *  (1) Histogram method of comparing two grayscale images for similarity.
 *      High score (> 0.5) means they're likely to be the same image
 *  (2) The morphological method, based on horizontal lines, for
 *      deciding if a grayscale image is text or non-text.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"


#define  TEST1   1
#define  TEST2   1
#define  TEST3   1
#define  TEST4   1
#define  TEST5   1

l_int32 main(int    argc,
             char **argv)
{
l_uint8     *bytea1, *bytea2;
l_int32      i, j, n, maxi, maxj, istext, w1, h1, w2, h2;
l_int32      debug;
size_t       size1, size2;
l_float32    score, maxscore;
l_float32   *scores;
BOX         *box1, *box2;
NUMA        *nai;
NUMAA       *naa1, *naa2, *naa3, *naa4;
PIX         *pix1, *pix2;
PIXA        *pixa1, *pixa2, *pixa3;
PIXAC       *pac;

    if (argc != 1) {
        lept_stderr("Syntax: histoduptest\n");
        return 1;
    }

        /* Set to 1 for more output from tests 1 and 2 */
    debug = 0;

    setLeptDebugOK(1);
    lept_mkdir("lept/comp");
    pac = pixacompRead("dinos.pac");  /* resolution = 75 ppi */

#if TEST1
    /* -------------------------------------------------------------- *
     *                  Test comparison with rotation                 *
     * -------------------------------------------------------------- */
        /* Make a second set that is rotated; combine with the input set. */
    pixa1 = pixaCreateFromPixacomp(pac, L_COPY);
    pixa2 = pixaScaleBySampling(pixa1, 2.0, 2.0);  /* to resolution 150 ppi */
    n = pixaGetCount(pixa2);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa2, i, L_CLONE);
        pix2 = pixRotate(pix1, 0.06, L_ROTATE_SAMPLING, L_BRING_IN_WHITE,
                         0, 0);
        pixaAddPix(pixa2, pix2, L_INSERT);
        pixDestroy(&pix1);
    }

        /* Compare between every pair of images;
         * can also use n = 2, simthresh = 0.50.  */
    pixaComparePhotoRegionsByHisto(pixa2, 0.85, 1.3, 1, 3, 0.20,
                                   &nai, &scores, &pix1, debug);
    lept_free(scores);

        /* Show the similarity classes. */
    numaWriteStderr(nai);
    pixWrite("/tmp/lept/comp/photoclass1.jpg", pix1, IFF_JFIF_JPEG);
    lept_stderr("Writing photo classes: /tmp/lept/comp/photoclass1.jpg\n");
    numaDestroy(&nai);
    pixDestroy(&pix1);

        /* Show the scores between images as a 2d array */
    pix2 = pixRead("/tmp/lept/comp/scorearray.png");
    pixDisplay(pix2, 100, 100);
    pixDestroy(&pix2);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
#endif

#if TEST2
    /* -------------------------------------------------------------- *
     *                      Test translation                          *
     * -------------------------------------------------------------- */
        /* Make a second set that is translated; combine with the input set. */
    pixa1 = pixaCreateFromPixacomp(pac, L_COPY);
    pixa2 = pixaScaleBySampling(pixa1, 2.0, 2.0);  /* to resolution 150 ppi */
    pixa3 = pixaTranslate(pixa2, 15, -21, L_BRING_IN_WHITE);
    pixaJoin(pixa2, pixa3, 0, -1);

        /* Compare between every pair of images. */
    pixaComparePhotoRegionsByHisto(pixa2, 0.85, 1.3, 1, 3, 0.20,
                                   &nai, &scores, &pix1, debug);
    lept_free(scores);

        /* Show the similarity classes. */
    numaWriteStderr(nai);
    pixWrite("/tmp/lept/comp/photoclass2.jpg", pix1, IFF_JFIF_JPEG);
    lept_stderr("Writing photo classes: /tmp/lept/comp/photoclass2.jpg\n");
    numaDestroy(&nai);
    pixDestroy(&pix1);

        /* Show the scores between images as a 2d array */
    pix2 = pixRead("/tmp/lept/comp/scorearray.png");
    pixDisplay(pix2, 100, 100);
    pixDestroy(&pix2);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
#endif

#if TEST3
    /* -------------------------------------------------------------- *
     *                 Compare two image regions                      *
     * -------------------------------------------------------------- */
        /* Do a comparison on a pair: dinos has (5,7) and (4,10) being
         * superficially similar.  But they are far apart by this test. */
    pixa1 = pixaCreateFromPixacomp(pac, L_COPY);
    pixa2 = pixaScaleBySampling(pixa1, 2.0, 2.0);  /* to resolution 150 ppi */
    pix1 = pixaGetPix(pixa2, 5, L_CLONE);
    box1 = pixaGetBox(pixa2, 5, L_COPY);
    pix2 = pixaGetPix(pixa2, 7, L_CLONE);
    box2 = pixaGetBox(pixa2, 7, L_COPY);
    pixGenPhotoHistos(pix1, box1, 1, 1.2, 3, &naa1, &w1, &h1, 5);
    pixGenPhotoHistos(pix2, box2, 1, 1.2, 3, &naa2, &w2, &h2, 7);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    if (!naa1 || !naa2) {
        lept_stderr("Not both image; exiting\n");
        return 0;
    }
    bytea1 = l_compressGrayHistograms(naa1, w1, h1, &size1);
    bytea2 = l_compressGrayHistograms(naa2, w2, h2, &size2);
    naa3 = l_uncompressGrayHistograms(bytea1, size1, &w1, &h1);
    naa4 = l_uncompressGrayHistograms(bytea2, size2, &w2, &h2);
    lept_stderr("*******  (%d, %d), (%d, %d)  *******\n", w1, h1, w2, h2);
    pixa1 = pixaCreate(0);
        /* Set @minratio very small to allow comparison for all pairs */
    compareTilesByHisto(naa3, naa4, 0.1, w1, h1, w2, h2, &score, pixa1);
    pixaDestroy(&pixa1);
    lept_stderr("score = %5.3f\n", score);
    pixaDestroy(&pixa1);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    boxDestroy(&box1);
    boxDestroy(&box2);
    numaaDestroy(&naa1);
    numaaDestroy(&naa2);
    numaaDestroy(&naa3);
    numaaDestroy(&naa4);
    lept_free(bytea1);
    lept_free(bytea2);
#endif

#if TEST4
    /* -------------------------------------------------------------- *
     *                  Test comparison in detail                     *
     * -------------------------------------------------------------- */
    pixa1 = pixaCreateFromPixacomp(pac, L_COPY);
    n = pixaGetCount(pixa1);
    maxscore = 0.0;
    maxi = 0;
    maxj = 0;
    for (i = 0; i < n; i++) {
        lept_stderr("i = %d\n", i);
        pix1 = pixaGetPix(pixa1, i, L_CLONE);
        box1 = pixaGetBox(pixa1, i, L_COPY);
        for (j = 0; j <= i; j++) {
            pix2 = pixaGetPix(pixa1, j, L_CLONE);
            box2 = pixaGetBox(pixa1, j, L_COPY);
            pixCompareGrayByHisto(pix1, pix2, box1, box2, 0.85, 230, 1, 3,
                                  &score, 0);
            lept_stderr("Score[%d,%d] = %5.3f\n", i, j, score);
            if (i != j && score > maxscore) {
                maxscore = score;
                maxi = i;
                maxj = j;
            }
            pixDestroy(&pix2);
            boxDestroy(&box2);
        }
        pixDestroy(&pix1);
        boxDestroy(&box1);
    }
    pixaDestroy(&pixa1);
    lept_stderr("max score [%d,%d] = %5.3f\n", maxi, maxj, maxscore);
#endif

#if TEST5
    /* -------------------------------------------------------------- *
     *              Text or photo determination in detail             *
     * -------------------------------------------------------------- */
        /* Are the images photo or text?  This is the morphological
         * method, which is more accurate than the variance of gray
         * histo method.  Output to /tmp/lept/comp/isphoto1.pdf.  */
    pixa1 = pixaCreateFromPixacomp(pac, L_COPY);
    n = pixaGetCount(pixa1);
    pixa2 = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pixa3 = pixaCreate(0);
        lept_stderr("i = %d\n", i);
        pix1 = pixaGetPix(pixa1, i, L_CLONE);
        box1 = pixaGetBox(pixa1, i, L_COPY);
        lept_stderr("w = %d, h = %d\n", pixGetWidth(pix1),
                pixGetHeight(pix1));
        pixDecideIfText(pix1, box1, &istext, pixa3);
        if (istext == 1) {
            lept_stderr("This is text\n\n");
        } else if (istext == 0) {
            lept_stderr("This is a photo\n\n");
        } else {  /*  istext == -1 */
            lept_stderr("Not determined if text or photo\n\n");
        }
        if (istext == 0) {
            pix2 = pixaDisplayTiledInRows(pixa3, 32, 1000, 1.0, 0, 50, 2);
            pixDisplay(pix2, 100, 100);
            pixaAddPix(pixa2, pix2, L_INSERT);
            pixaDestroy(&pixa3);
        }
        pixDestroy(&pix1);
        boxDestroy(&box1);
    }
    lept_stderr("Writing to: /tmp/lept/comp/isphoto1.pdf\n");
    pixaConvertToPdf(pixa2, 300, 1.0, L_FLATE_ENCODE, 0, NULL,
                         "/tmp/lept/comp/isphoto1.pdf");
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
#endif

    pixacompDestroy(&pac);
    return 0;
}

