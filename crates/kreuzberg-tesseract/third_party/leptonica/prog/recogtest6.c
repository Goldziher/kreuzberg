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
 *  recogtest6.c
 *
 *     Another test of character splitting.  This will test both DID
 *     and greedy splitting.  To test greedy splitting, in recogident.c,
 *       #define  SPLIT_WITH_DID   0
 *
 *     The timing info is used to measure the time to split touching
 *     characters and identify them.  One set of 4 digits takes about 1 ms
 *     with DID and 7 ms with greedy splitting.  Because DID is about
 *     5x faster than greedy splitting, DID is the default that is used.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

static PIX *GetBigComponent(PIX *pixs);


l_int32 main(int    argc,
             char **argv)
{
l_int32   item, debug, i;
l_int32   example[6] = {17, 20, 21, 22, 23, 24};  /* for decoding */
BOXA     *boxa;
NUMA     *nascore;
PIX      *pix1, *pix2, *pix3, *pixdb;
PIXA     *pixa1, *pixa2;
L_RECOG  *recog;

    if (argc != 1) {
        lept_stderr(" Syntax: recogtest6\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/recog");

        /* Generate the recognizer */
    pixa1 = pixaRead("recog/sets/train01.pa");
    recog = recogCreateFromPixa(pixa1, 0, 0, 0, 128, 1);
    recogAverageSamples(recog, 0);

        /* Show the templates */
    recogDebugAverages(recog, 1);
    recogShowMatchesInRange(recog, recog->pixa_tr, 0.0, 1.0, 1);

        /* Get a set of problem images to decode */
    pixa2 = pixaRead("recog/sets/test01.pa");

        /* Decode a subset of them */
    debug = 1;
    for (i = 0; i < 6; i++) {
/*        if (i != 3) continue; */
        item = example[i];
        pix1 = pixaGetPix(pixa2, item, L_CLONE);
        pixDisplay(pix1, 100, 100);
        pix2 = GetBigComponent(pix1);
        if (debug) {
            recogIdentifyMultiple(recog, pix2, 0, 0, &boxa, NULL, &pixdb, 1);
            rchaExtract(recog->rcha, NULL, &nascore, NULL, NULL,
                        NULL, NULL, NULL);
            pixDisplay(pixdb, 300, 500);
            boxaWriteStderr(boxa);
            numaWriteStderr(nascore);
            numaDestroy(&nascore);
            pixDestroy(&pixdb);
        } else {  /* just get the timing */
            startTimer();
            recogIdentifyMultiple(recog, pix2, 0, 0, &boxa, NULL, NULL, 0);
            lept_stderr("Time: %5.3f\n", stopTimer());
        }
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        boxaDestroy(&boxa);
    }
    if (debug) {
        pix3 = pixaDisplayTiledInRows(recog->pixadb_split, 1, 200,
                                      1.0, 0, 20, 3);
        pixDisplay(pix3, 0, 0);
        pixDestroy(&pix3);
    }

    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    recogDestroy(&recog);
    return 0;
}


static PIX *
GetBigComponent(PIX  *pixs)
{
BOX  *box;
PIX  *pix1, *pixd;

    pix1 = pixMorphSequence(pixs, "c40.7 + o20.15 + d25.1", 0);
    pixClipToForeground(pix1, NULL, &box);
    pixd = pixClipRectangle(pixs, box, NULL);
    pixDestroy(&pix1);
    boxDestroy(&box);
    return pixd;
}

