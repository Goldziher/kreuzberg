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
 *  recogtest5.c
 *
 *     Test document image decoding (DID) approach to splitting characters
 *
 *     This uses recogIdentifyMultiple() to first split the touching
 *     characters and then do the identification on the resulting
 *     single characters.  Compare with recogtest4.c.
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
l_int32   i, item;
l_int32   example[6] = {17, 20, 21, 22, 23, 24};  /* for decoding */
PIX      *pix1, *pix2, *pix3;
PIXA     *pixa1, *pixa2, *pixa3;
L_RECOG  *recog;

    if (argc != 1) {
        lept_stderr(" Syntax: recogtest5\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/recog");

        /* Generate the recognizer */
    pixa1 = pixaRead("recog/sets/train01.pa");
    recog = recogCreateFromPixa(pixa1, 0, 0, 0, 128, 1);  /* no scaling */
    recogAverageSamples(recog, 1);
    recogWrite("/tmp/lept/recog/rec1.rec", recog);

        /* Show the templates */
    recogDebugAverages(recog, 1);
    recogShowMatchesInRange(recog, recog->pixa_tr, 0.0, 1.0, 1);

        /* Get a set of problem images to decode */
    pixa2 = pixaRead("recog/sets/test01.pa");

        /* Decode a subset of them.  It takes about 2 ms to decode a
         * 4 digit number (Viterbi for splitting; identification against
         * all templates; debug off. */
    for (i = 0; i < 6; i++) {
/*        if (i != 3) continue;  */ /* remove this comment to do all 6 */
        item = example[i];
        pix1 = pixaGetPix(pixa2, item, L_CLONE);
        pixDisplay(pix1, 100, 100);
        pix2 = GetBigComponent(pix1);
        recogIdentifyMultiple(recog, pix2, 0, 0, NULL, &pixa3, NULL, 1);
        pix3 = pixaDisplayTiledInColumns(pixa3, 1, 1.0, 20, 2);
        pixDisplay(pix3, 800, 100);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixaDestroy(&pixa3);
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


