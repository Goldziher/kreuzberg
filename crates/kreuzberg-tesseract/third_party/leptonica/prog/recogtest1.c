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
 *  recogtest1.c
 *
 *     Tests the recog utility using the bootstrap number set,
 *     for both training and identification
 *
 *     An example of greedy splitting of touching characters is given.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

static const l_int32 scaledw = 0;
static const l_int32 scaledh = 40;

static const l_float32  MinScore[] = {0.6f, 0.7f, 0.9f};
static const l_int32  MinTarget[] = {4, 5, 4};
static const l_int32  MinSize[] = {3, 2, 3};

l_int32 main(int    argc,
             char **argv)
{
l_int32   i, linew, same;
BOXA     *boxat;
PIX      *pixd, *pix1, *pix2, *pixdb;
PIXA     *pixa1, *pixa2, *pixa3;
L_RECOG  *recog1, *recog2;

    if (argc != 1) {
        lept_stderr(" Syntax: recogtest1\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/digits");
    recog1 = NULL;
    recog2 = NULL;

#if 0
    linew = 5;  /* for lines */
#else
    linew = 0;  /* scanned image */
#endif

#if 1
    pixa1 = pixaRead("recog/digits/bootnum1.pa");
    recog1 = recogCreateFromPixa(pixa1, scaledw, scaledh, linew, 120, 1);
    pix1 = pixaDisplayTiledWithText(pixa1, 1400, 1.0, 10, 2, 6, 0xff000000);
    pixWrite("/tmp/lept/digits/bootnum1.png", pix1, IFF_PNG);
    pixDisplay(pix1, 800, 800);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);
#endif

#if 1
    lept_stderr("Print Stats 1\n");
    recogShowContent(stderr, recog1, 1, 1);
#endif

#if 1
    lept_stderr("AverageSamples\n");
    recogAverageSamples(recog1, 1);
    recogShowAverageTemplates(recog1);
    pix1 = pixaGetPix(recog1->pixadb_ave, 0, L_CLONE);
    pixWrite("/tmp/lept/digits/unscaled_ave.png", pix1, IFF_PNG);
    pixDestroy(&pix1);
    pix1 = pixaGetPix(recog1->pixadb_ave, 1, L_CLONE);
    pixWrite("/tmp/lept/digits/scaled_ave.png", pix1, IFF_PNG);
    pixDestroy(&pix1);
#endif

#if 1
    recogDebugAverages(recog1, 0);
    recogShowMatchesInRange(recog1, recog1->pixa_tr, 0.65, 1.0, 0);
    pixWrite("/tmp/lept/digits/match_ave1.png", recog1->pixdb_range, IFF_PNG);
    recogShowMatchesInRange(recog1, recog1->pixa_tr, 0.0, 1.0, 0);
    pixWrite("/tmp/lept/digits/match_ave2.png", recog1->pixdb_range, IFF_PNG);
#endif

#if 1
    lept_stderr("Print stats 2\n");
    recogShowContent(stderr, recog1, 2, 1);
    recogWrite("/tmp/lept/digits/rec1.rec", recog1);
    recog2 = recogRead("/tmp/lept/digits/rec1.rec");
    recogShowContent(stderr, recog2, 3, 1);
    recogWrite("/tmp/lept/digits/rec2.rec", recog2);
    filesAreIdentical("/tmp/lept/digits/rec1.rec",
                      "/tmp/lept/digits/rec2.rec", &same);
    if (!same)
        lept_stderr("Error in serialization!\n");
    recogDestroy(&recog2);
#endif

#if 1
        /* Three sets of parameters:
         *  0.6, 0.3 : removes a few poor matches
         *  0.8, 0.2 : remove many based on matching; remove some based on
         *             requiring retention of 20% of templates in each class
         *  0.9, 0.01 : remove most based on matching; saved 1 in each class */
    lept_stderr("Remove outliers\n");
    pixa2 = recogExtractPixa(recog1);
    for (i = 0; i < 3; i++) {
        pixa3 = pixaRemoveOutliers1(pixa2, MinScore[i], MinTarget[i],
                                    MinSize[i],  &pix1, &pix2);
        pixDisplay(pix1, 900, 250 * i);
        pixDisplay(pix2, 1300, 250 * i);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixaDestroy(&pixa3);
    }
    pixaDestroy(&pixa2);
#endif

#if 1
        /* Split touching characters */
    lept_stderr("Split touching\n");
    pixd = pixRead("recog/digits/page.590.png");  /* 590 or 306 */
    recogIdentifyMultiple(recog1, pixd, 0, 0, &boxat, &pixa2, &pixdb, 1);
    pixDisplay(pixdb, 800, 800);
    boxaWriteStderr(boxat);
    pix1 = pixaDisplay(pixa2, 0, 0);
    pixDisplay(pix1, 1200, 800);
    pixDestroy(&pixdb);
    pixDestroy(&pix1);
    pixDestroy(&pixd);
    pixaDestroy(&pixa2);
    boxaDestroy(&boxat);
#endif

#if 1
    lept_stderr("Reading new training set and computing averages\n");
    lept_stderr("Print stats 3\n");
    pixa1 = pixaRead("recog/sets/train03.pa");
    recog2 = recogCreateFromPixa(pixa1, 0, 40, 0, 128, 1);
    recogShowContent(stderr, recog2, 3, 1);
    recogDebugAverages(recog2, 3);
    pixWrite("/tmp/lept/digits/averages.png", recog2->pixdb_ave, IFF_PNG);
    recogShowAverageTemplates(recog2);
    pixaDestroy(&pixa1);
    recogDestroy(&recog2);
#endif

    recogDestroy(&recog1);
    recogDestroy(&recog2);
    return 0;
}
