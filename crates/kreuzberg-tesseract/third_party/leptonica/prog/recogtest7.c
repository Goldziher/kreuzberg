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
 *  recogtest7.c
 *
 *     Tests the boot recog utility using the bootstrap templates
 *     from the mosaics (bootnum4.pa) and from the stringcode version
 *     (bootnumgen4.c).
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

    /* All input templates are scaled to 20x30.  Here, we rescale the
     * height to 45 and let the width scale isotropically. */
static const l_int32 scaledw = 0;
static const l_int32 scaledh = 45;

l_int32 main(int    argc,
             char **argv)
{
l_int32   same;
PIX      *pix1, *pix2, *pix3;
PIXA     *pixa1, *pixa2, *pixa3;
L_RECOG  *recog1, *recog2;

    if (argc != 1) {
        lept_stderr(" Syntax: recogtest7\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/digits");
    recog1 = NULL;
    recog2 = NULL;

#if 1
    pixa1 = pixaRead("recog/digits/bootnum4.pa");
    pixa2 = pixaMakeFromTiledPixa(pixa1, 0, 0, 100);
    pixa3 = l_bootnum_gen4(100);
    pixaEqual(pixa2, pixa3, 0, NULL, &same);
    if (!same) L_ERROR("Bad!  The pixa differ!\n", __func__);
    pix1 = pixaDisplayTiledWithText(pixa1, 1400, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 100, 100);
    pix2 = pixaDisplayTiledWithText(pixa2, 1400, 1.0, 10, 2, 6, 0xff000000);
    pix3 = pixaDisplayTiledWithText(pixa3, 1400, 1.0, 10, 2, 6, 0xff000000);
    pixEqual(pix2, pix3, &same);
    if (!same) L_ERROR("Bad! The displayed pix differ!\n", __func__);
    pixWrite("/tmp/lept/digits/pix1.png", pix1, IFF_PNG);
    pixWrite("/tmp/lept/digits/bootnum4.png", pix1, IFF_PNG);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
#endif

#if 1
    lept_stderr("Show recog content\n");
    recog1 = recogCreateFromPixa(pixa3, scaledw, scaledh, 0, 120, 1);
    recogShowContent(stderr, recog1, 1, 1);
    pixaDestroy(&pixa3);
#endif

#if 1
    lept_stderr("\nShow averaged samples\n");
    recogAverageSamples(recog1, 1);
    recogShowAverageTemplates(recog1);
    pix1 = pixaGetPix(recog1->pixadb_ave, 0, L_CLONE);
    pixWrite("/tmp/lept/digits/unscaled_ave.png", pix1, IFF_PNG);
    pixDestroy(&pix1);
    pix1 = pixaGetPix(recog1->pixadb_ave, 1, L_CLONE);
    pixWrite("/tmp/lept/digits/scaled_ave.png", pix1, IFF_PNG);
    pixDestroy(&pix1);
    recogDestroy(&recog1);
#endif

#if 1
        /* Make a tiny recognizer and test it against itself */
    pixa1 = l_bootnum_gen4(5);
    pix1 = pixaDisplayTiledWithText(pixa1, 1400, 1.0, 10, 2, 6, 0xff000000);
    pixDisplay(pix1, 1000, 100);
    pixDestroy(&pix1);
    recog1 = recogCreateFromPixa(pixa1, scaledw, scaledh, 0, 120, 1);
    lept_stderr("\nShow matches against all inputs for given range\n");
    recogDebugAverages(recog1, 0);
    recogShowMatchesInRange(recog1, recog1->pixa_tr, 0.85, 1.00, 1);
    pixWrite("/tmp/lept/digits/match_input.png", recog1->pixdb_range, IFF_PNG);
    lept_stderr("\nShow best match against average template\n");
    recogShowMatchesInRange(recog1, recog1->pixa_u, 0.65, 1.00, 1);
    pixWrite("/tmp/lept/digits/match_ave.png", recog1->pixdb_range, IFF_PNG);
    pixaDestroy(&pixa1);
#endif

#if 1
    lept_stderr("\nContents of recog before write/read:\n");
    recogShowContent(stderr, recog1, 2, 1);

    lept_stderr("\nTest serialization\n");
    recogWrite("/tmp/lept/digits/rec1.rec", recog1);
    recog2 = recogRead("/tmp/lept/digits/rec1.rec");
    lept_stderr("Contents of recog after write/read:\n");
    recogShowContent(stderr, recog2, 3, 1);
    recogWrite("/tmp/lept/digits/rec2.rec", recog2);
    filesAreIdentical("/tmp/lept/digits/rec1.rec",
                      "/tmp/lept/digits/rec2.rec", &same);
    if (!same)
        lept_stderr("Error in serialization!\n");
    recogDestroy(&recog1);
    recogDestroy(&recog2);
#endif

    return 0;
}
