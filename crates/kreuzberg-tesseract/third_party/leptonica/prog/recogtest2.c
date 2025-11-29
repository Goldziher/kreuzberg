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
 *  recogtest2.c
 *
 *     Test bootstrap recognizer (BSR) to train a book-adapted
 *     recognizer (BAR), starting with unlabeled bitmaps from the book.
 *
 *     Several BSRs are used.
 *     The BAR images are taken from recog/sets/train*.pa.  We really
 *     know their classes, but pretend we don't, by erasing the labels.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

    /* Sets for training using boot recognizers */
static char  trainset1[] = "recog/sets/train04.pa";  /* partial set */
static char  trainset2[] = "recog/sets/train05.pa";  /* full set */

    /* Use scanned images or width-normalized lines */
#if 1
static const l_int32  linew = 0;  /* use scanned bitmaps */
#else
static const l_int32  linew = 5;  /* use generated lines */
#endif

l_int32 main(int    argc,
             char **argv)
{
char     *fname;
l_int32   i;
BOXA     *boxa1;
BOXAA    *baa;
NUMAA    *naa;
PIX      *pix1, *pix2, *pix3;
PIXA     *pixa1, *pixa2, *pixa3;
L_RECOG  *recogboot, *recog1;
SARRAY   *sa;

    if (argc != 1) {
        lept_stderr(" Syntax: recogtest2\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/recog");

        /* Files with 'unlabeled' templates from book */
    sa = sarrayCreate(2);
    sarrayAddString(sa, trainset1, L_COPY);
    sarrayAddString(sa, trainset2, L_COPY);

    /* ----------------------------------------------------------- */
    /*        Do operations with a simple bootstrap recognizer     */
    /* ----------------------------------------------------------- */

        /* Generate a BSR (boot-strap recog), and show the unscaled
         * and scaled versions of the templates */
    pixa1 = (PIXA *)l_bootnum_gen1();  /* from recog/digits/bootnum1.pa */
    recogboot = recogCreateFromPixa(pixa1, 0, 40, linew, 128, 1);
    recogWrite("/tmp/lept/recog/boot1.rec", recogboot);
    recogShowContent(stderr, recogboot, 1, 1);
    pixaDestroy(&pixa1);

        /* Generate a BAR (book-adapted recog) for a set of images from
         * one book.  Select a set of digit images.  These happen to
         * be labeled, so we clear the text field from each pix before
         * running it through the boot recognizer. */
    for (i = 0; i < 2; i++) {
        fname = sarrayGetString(sa, i, L_NOCOPY);
        pixa2 = pixaRead(fname);
        pixaSetText(pixa2, NULL, NULL);

            /* Train a new recognizer from the boot and unlabeled samples */
        pixa3 = recogTrainFromBoot(recogboot, pixa2, 0.65, 128, 1);
        recog1 = recogCreateFromPixa(pixa3, 0, 40, linew, 128, 1);
        recogShowContent(stderr, recog1, 2, 1);
        if (i == 0)
            recogWrite("/tmp/lept/recog/recog1.rec", recog1);
        else  /* i == 1 */
            recogWrite("/tmp/lept/recog/recog2.rec", recog1);
        pixaDestroy(&pixa2);
        pixaDestroy(&pixa3);
        recogDestroy(&recog1);
    }
    recogDestroy(&recogboot);

    /* ----------------------------------------------------------- */
    /*        Do operations with a larger bootstrap recognizer     */
    /* ----------------------------------------------------------- */

        /* Generate the boot recog, and show the unscaled and scaled
         * versions of the templates */
    recogboot = recogMakeBootDigitRecog(0, 40, linew, 1, 1);
    recogWrite("/tmp/lept/recog/boot2.rec", recogboot);
    recogShowContent(stderr, recogboot, 3, 1);

        /* Generate a BAR for a set of images from one book.
         * Select a set of digit images and erase the text field. */
    for (i = 0; i < 2; i++) {
        fname = sarrayGetString(sa, i, L_NOCOPY);
        pixa2 = pixaRead(fname);
        pixaSetText(pixa2, NULL, NULL);

            /* Train a new recognizer from the boot and unlabeled samples */
        pixa3 = recogTrainFromBoot(recogboot, pixa2, 0.65, 128, 1);
        recog1 = recogCreateFromPixa(pixa3, 0, 40, linew, 128, 1);
        recogShowContent(stderr, recog1, 4, 1);
        if (i == 0)
            recogWrite("/tmp/lept/recog/recog3.rec", recog1);
        else if (i == 1)
            recogWrite("/tmp/lept/recog/recog4.rec", recog1);
        pixaDestroy(&pixa2);
        pixaDestroy(&pixa3);
        recogDestroy(&recog1);
    }
    recogDestroy(&recogboot);
    sarrayDestroy(&sa);

#if 0
    recogShowMatchesInRange(recog, recog->pixa_tr, 0.0, 1.0, 1);
    recogShowContent(stderr, recog, 1);

        /* Now use minscore = 0.75 to remove the outliers in the BAR,
         * and show what is left. */
    lept_stderr("initial size: %d\n", recog->num_samples);
    pix1 = pix2 = NULL;
    recogRemoveOutliers1(&recog, 0.75, 5, 3, &pix1, &pix2);
    pixDisplay(pix1, 500, 0);
    pixDisplay(pix2, 500, 500);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    lept_stderr("final size: %d\n", recog->num_samples);
    recogDebugAverages(recog, 1);
    recogShowContent(stderr, recog, 1);
    recogShowMatchesInRange(recog, recog->pixa_tr, 0.75, 1.0, 1);
    pixWrite("/tmp/lept/recog/range.png", recog->pixdb_range, IFF_PNG);
#endif

    /* ----------------------------------------------------------- */
    /*      Show operation of the default bootstrap recognizer     */
    /* ----------------------------------------------------------- */

    recog1 = recogMakeBootDigitRecog(0, 40, 0, 1, 0);
    pix1 = pixRead("test-87220.59.png");
    recogIdentifyMultiple(recog1, pix1, 0, 1, &boxa1, NULL, NULL, 0);
    sa = recogExtractNumbers(recog1, boxa1, 0.75, -1, &baa, &naa);
    pixa1 = showExtractNumbers(pix1, sa, baa, naa, &pix3);
    pix2 = pixaDisplayTiledInRows(pixa1, 32, 600, 1.0, 0, 20, 2);
    pixDisplay(pix2, 0, 1000);
    pixDisplay(pix3, 600, 1000);
    pixWrite("/tmp/lept/recog/extract.png", pix3, IFF_PNG);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixaDestroy(&pixa1);
    sarrayDestroy(&sa);
    boxaDestroy(&boxa1);
    boxaaDestroy(&baa);
    numaaDestroy(&naa);
    recogDestroy(&recog1);

    return 0;
}
