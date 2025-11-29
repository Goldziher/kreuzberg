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
 *  recogtest4.c
 *
 *     Test document image decoding (DID) approach to splitting characters
 *     This tests the low-level recogDecode() function.
 *     Splitting succeeds for both with and without character height scaling.
 *
 *     But cf. recogtest5.c.  Note that recogIdentifyMultiple(), which
 *     does prefiltering and splitting before character identification,
 *     does not accept input that has been scaled.  That is because
 *     the only reason for scaling the templates is that the recognizer
 *     is a hybrid BAR/BSR, where we've used a mixture of templates from
 *     a single source and bootstrap templates from many sources.
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
char      buf[256];
l_int32   i, item;
l_int32   example[6] = {17, 20, 21, 22, 23, 24};  /* for decoding */
BOXA     *boxa;
PIX      *pix1, *pix2, *pixdb;
PIXA     *pixa1, *pixa2;
L_RECOG  *recog;

    if (argc != 1) {
        lept_stderr(" Syntax: recogtest4\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/recog");

        /* Generate the recognizer */
    pixa1 = pixaRead("recog/sets/train01.pa");
#if 1   /* scale to fixed height */
    recog = recogCreateFromPixa(pixa1, 0, 40, 0, 128, 1);
#else   /* no scaling */
    recog = recogCreateFromPixa(pixa1, 0, 0, 0, 128, 1);
#endif
    recogAverageSamples(recog, 1);
    recogWrite("/tmp/lept/recog/rec1.rec", recog);

        /* Show the templates */
    if (recogDebugAverages(recog, 1) != 0) {
        lept_stderr("Averaging failed!!\n");
        return 1;
    }
    recogShowMatchesInRange(recog, recog->pixa_tr, 0.0, 1.0, 1);

        /* Get a set of problem images to decode */
    pixa2 = pixaRead("recog/sets/test01.pa");


        /* Decode a subset of them.  It takes about 1 ms to decode a
         * 4 digit number, with both Viterbi and rescoring (debug off). */
    for (i = 0; i < 6; i++) {
/*        if (i != 3) continue; */  /* remove this comment to do all 6 */
        item = example[i];
        pix1 = pixaGetPix(pixa2, item, L_CLONE);
        pixDisplay(pix1, 100, 100);
        pix2 = GetBigComponent(pix1);
        boxa = recogDecode(recog, pix2, 2, &pixdb);
        pixDisplay(pixdb, 300, 100);
        snprintf(buf, sizeof(buf), "/tmp/lept/recog/did-%d.png", item);
        pixWrite(buf, pixdb, IFF_PNG);
        pixDestroy(&pixdb);
        boxaDestroy(&boxa);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
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


