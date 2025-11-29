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
 *  recogtest3.c
 *
 *     Test padding of book-adapted recognizer (BAR) using templates
 *     from a bootstrap recognizer (BSR) to identify unlabeled samples
 *     from the book.
 *
 *     Terminology note:
 *        templates: labeled character images that can be inserted
 *                   into a recognizer.
 *        samples: unlabeled character images that must be labeled by
 *                 a recognizer before they can be used as templates.
 *
 *     This demonstrates the following operations:
 *     (1) Making a BAR from labeled book templates (as a pixa).
 *     (2) Making a hybrid BAR/BSR from scaled templates in the BAR,
 *         supplemented with similarly scaled bootstrap templates for those
 *         classes where the BAR templates are either missing or not
 *         of sufficient quantity.
 *     (3) Using the BAR/BSR to label unlabeled book sampless.
 *     (4) Adding the pixa of the original set of labeled book
 *         templates to the pixa of the newly labeled templates, and
 *         making a BAR from the joined pixa.  The BAR would then
 *         work to identify unscaled samples from the book.
 *     (5) Removing outliers from the BAR.
 *
 *     Note that if this final BAR were not to have a sufficient number
 *     of templates in each class, it can again be augmented with BSR
 *     templates, and the hybrid BAR/BSR would be the final recognizer
 *     that is used to identify unknown (scaled) samples.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"


l_int32 main(int    argc,
             char **argv)
{
char      *text;
l_int32    histo[10];
l_int32    i, n, ival, same;
PIX       *pix1, *pix2;
PIXA      *pixa1, *pixa2, *pixa3, *pixa4;
L_RECOG   *recog1, *recog2, *recog3;

    if (argc != 1) {
        lept_stderr(" Syntax: recogtest3\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/recog");

        /* Read templates and split them into two sets.  Use one to
         * make a BAR recog that needs padding; use the other with a
         * hybrid BAR/BSR to make more labeled templates to augment
         * the BAR */
    pixa1 = pixaRead("recog/sets/train05.pa");
    pixa2 = pixaCreate(0);   /* to generate a small BAR */
    pixa3 = pixaCreate(0);   /* for templates to be labeled and
                              * added to the BAR */
    n = pixaGetCount(pixa1);
    for (i = 0; i < 10; i++)
        histo[i] = 0;
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa1, i, L_COPY);
        text = pixGetText(pix1);
        ival = text[0] - '0';
            /* remove all 4's, and all but 2 7's and 9's */
        if (ival == 4 || (ival == 7 && histo[7] == 2) ||
            (ival == 9 && histo[9] == 2)) {
            pixaAddPix(pixa3, pix1, L_INSERT);
        } else {
            pixaAddPix(pixa2, pix1, L_INSERT);
            histo[ival]++;
        }
    }
    pix1 = pixaDisplayTiledWithText(pixa3, 1500, 1.0, 15, 2, 6, 0xff000000);
    pixDisplay(pix1, 500, 0);
    pixDestroy(&pix1);

        /* Make a BAR from the small set */
    recog1 = recogCreateFromPixa(pixa2, 0, 40, 0, 128, 1);
    recogShowContent(stderr, recog1, 0, 1);

        /* Pad with BSR templates to make a hybrid BAR/BSR */
    recogPadDigitTrainingSet(&recog1, 40, 0);
    recogShowContent(stderr, recog1, 1, 1);

        /* Use the BAR/BSR to label the left-over templates from the book */
    pixa4 = recogTrainFromBoot(recog1, pixa3, 0.75, 128, 1);

        /* Join the two sets */
    pixaJoin(pixa1, pixa4, 0, 0);
    pixaDestroy(&pixa4);

        /* Make a new BAR that uses unscaled templates.
         * This now has all the templates from pixa1, before deletions */
    recog2 = recogCreateFromPixa(pixa1, 0, 0, 5, 128, 1);
    recogShowContent(stderr, recog2, 2, 1);

        /* Test recog serialization */
    recogWrite("/tmp/lept/recog/recog2.rec", recog2);
    recog3 = recogRead("/tmp/lept/recog/recog2.rec");
    recogWrite("/tmp/lept/recog/recog3.rec", recog3);
    filesAreIdentical("/tmp/lept/recog/recog2.rec",
                      "/tmp/lept/recog/recog3.rec", &same);
    if (!same)
        lept_stderr("Error in serialization!\n");
    recogDestroy(&recog3);

        /* Remove outliers: method 1 */
    pixa4 = pixaRemoveOutliers1(pixa1, 0.8, 4, 3, &pix1, &pix2);
    pixDisplay(pix1, 500, 0);
    pixDisplay(pix2, 500, 500);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    recog3 = recogCreateFromPixa(pixa4, 0, 0, 0, 128, 1);
    recogShowContent(stderr, recog3, 3, 1);
    pixaDestroy(&pixa4);
    recogDestroy(&recog3);

        /* Relabel a few templates to put them in the wrong classes */
    pix1 = pixaGetPix(pixa1, 7, L_CLONE);
    pixSetText(pix1, "4");
    pixDestroy(&pix1);
    pix1 = pixaGetPix(pixa1, 38, L_CLONE);
    pixSetText(pix1, "9");
    pixDestroy(&pix1);
    pix1 = pixaGetPix(pixa1, 61, L_CLONE);
    pixSetText(pix1, "2");
    pixDestroy(&pix1);

        /* Remove outliers: method 2 */
    pixa4 = pixaRemoveOutliers2(pixa1, 0.65, 3, &pix1, &pix2);
    pixDisplay(pix1, 900, 0);
    pixDisplay(pix2, 900, 500);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    recog3 = recogCreateFromPixa(pixa4, 0, 0, 0, 128, 1);
    recogShowContent(stderr, recog3, 3, 1);
    pixaDestroy(&pixa4);
    recogDestroy(&recog3);

    recogDestroy(&recog1);
    recogDestroy(&recog2);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
    return 0;
}
