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
 *  italic_reg.c
 *
 *     This demonstrates binary reconstruction for finding italic text.
 *     It also tests debug output of word masking.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char          opstring[32];
l_int32       size;
BOXA         *boxa1, *boxa2, *boxa3, *boxa4;
PIX          *pixs, *pixm, *pix1;
PIXA         *pixadb;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "italic_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/ital");
    pixs = pixRead("italic.png");

        /* Basic functionality with debug flag */
    pixItalicWords(pixs, NULL, NULL, &boxa1, 1);
    boxaWrite("/tmp/lept/ital/ital1.ba", boxa1);
    regTestCheckFile(rp, "/tmp/lept/ital/ital1.ba");  /* 0 */
    regTestCheckFile(rp, "/tmp/lept/ital/ital.3.pdf");  /* 1 */
    pix1 = pixRead("/tmp/lept/ital/ital.3.png");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix1, 0, 0, "Intermediate steps", rp->display);
    pixDestroy(&pix1);
    pix1 = pixRead("/tmp/lept/ital/runhisto.png");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 3 */
    pixDisplayWithTitle(pix1, 400, 0, "Histogram of white runs", rp->display);
    pixDestroy(&pix1);

        /* Generate word mask */
    pixadb = pixaCreate(5);
    pixWordMaskByDilation(pixs, NULL, &size, pixadb);
    l_pdfSetDateAndVersion(0);
    pixaConvertToPdf(pixadb, 100, 1.0, L_FLATE_ENCODE, 0, "Word Mask",
                     "/tmp/lept/ital/wordmask.pdf");
    regTestCheckFile(rp, "/tmp/lept/ital/wordmask.pdf");  /* 4 */
    pix1 = pixaDisplayTiledInColumns(pixadb, 1, 1.0, 25, 2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix1, 1400, 0, "Intermediate mask step", rp->display);
    pixDestroy(&pix1);
    pixaDestroy(&pixadb);
    L_INFO("dilation size = %d\n", rp->testname, size);
    snprintf(opstring, sizeof(opstring), "d1.5 + c%d.1", size);
    pixm = pixMorphSequence(pixs, opstring, 0);
    regTestWritePixAndCheck(rp, pixm, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pixm, 400, 550, "Word mask", rp->display);

        /* Re-run italic finder using the word mask */
    pixItalicWords(pixs, NULL, pixm, &boxa2, 1);
    boxaWrite("/tmp/lept/ital/ital2.ba", boxa2);
    regTestCheckFile(rp, "/tmp/lept/ital/ital2.ba");  /* 7 */

        /* Re-run italic finder using word mask bounding boxes */
    boxa3 = pixConnComp(pixm, NULL, 8);
    pixItalicWords(pixs, boxa3, NULL, &boxa4, 1);
    boxaWrite("/tmp/lept/ital/ital3.ba", boxa3);
    regTestCheckFile(rp, "/tmp/lept/ital/ital3.ba");  /* 8 */
    boxaWrite("/tmp/lept/ital/ital4.ba", boxa4);
    regTestCheckFile(rp, "/tmp/lept/ital/ital4.ba");  /* 9 */
    regTestCompareFiles(rp, 7, 9);  /* 10 */

    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    boxaDestroy(&boxa3);
    boxaDestroy(&boxa4);
    pixDestroy(&pixs);
    pixDestroy(&pixm);
    return regTestCleanup(rp);
}

