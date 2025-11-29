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
 * recogsort.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"


l_int32 main(int    argc,
             char **argv)
{
char     *boxatxt;
l_int32   i;
BOXA     *boxa1, *boxa2, *boxa3;
BOXAA    *baa, *baa1;
NUMAA    *naa1;
PIX      *pixdb, *pix1, *pix2, *pix3, *pix4;
PIXA     *pixa1, *pixa2, *pixa3, *pixat;
L_RECOG  *recog;
SARRAY   *sa1;

    /* ----- Example identifying samples using training data ----- */

    setLeptDebugOK(1);
    lept_mkdir("lept/recog");

        /* Read the training data */
    pixat = pixaRead("recog/sets/train06.pa");
    recog = recogCreateFromPixa(pixat, 0, 0, 0, 128, 1);
    recogAverageSamples(recog, 0);  /* required for splitting characters */
    pix1 = pixaDisplayTiledWithText(pixat, 1500, 1.0, 10, 1, 8, 0xff000000);
    pixDisplay(pix1, 0, 0);
    pixDestroy(&pix1);
    pixaDestroy(&pixat);

        /* Read the data from all samples */
    pix1 = pixRead("recog/sets/samples06.png");
    boxatxt = pixGetText(pix1);
    lept_stderr("%s\n", boxatxt);
    boxa1 = boxaReadMem((l_uint8 *)boxatxt, strlen(boxatxt));
    pixa1 = pixaCreateFromBoxa(pix1, boxa1, 0, 0, NULL);
    pixDestroy(&pix1);  /* destroys boxa1 */

        /* Identify components in the sample data */
    pixa2 = pixaCreate(0);
    pixa3 = pixaCreate(0);
    for (i = 0; i < 9; i++) {
/*        if (i != 4) continue; */  /* dots form separate boxa */
/*        if (i != 8) continue; */  /* broken 2 in '24' */
        if (i != 8) continue;
        pix1 = pixaGetPix(pixa1, i, L_CLONE);

            /* Show the 2d box data in the sample */
        boxa2 = pixConnComp(pix1, NULL, 8);
        baa = boxaSort2d(boxa2, NULL, 6, 6, 5);
        pix2 = boxaaDisplay(pix1, baa, 3, 1, 0xff000000, 0x00ff0000, 0, 0);
        pixaAddPix(pixa3, pix2, L_INSERT);
        boxaaDestroy(&baa);
        boxaDestroy(&boxa2);

            /* Get the numbers in the sample */
        recogIdentifyMultiple(recog, pix1, 0, 0, &boxa3, NULL, &pixdb, 0);
        sa1 = recogExtractNumbers(recog, boxa3, 0.7, -1, &baa1, &naa1);
        sarrayWriteStderr(sa1);
        boxaaWriteStream(stderr, baa1);
        numaaWriteStream(stderr, naa1);
        pixaAddPix(pixa2, pixdb, L_INSERT);
/*        pixaWrite("/tmp/pixa.pa", pixa2); */
        pixDestroy(&pix1);
        boxaWriteStderr(boxa3);
        boxaDestroy(&boxa3);
        boxaaDestroy(&baa1);
        numaaDestroy(&naa1);
        sarrayDestroy(&sa1);
    }

    pix3 = pixaDisplayLinearly(pixa2, L_VERT, 1.0, 0, 20, 1, NULL);
    pixWrite("/tmp/lept/recog/pix3.png", pix3, IFF_PNG);
    pix4 = pixaDisplayTiledInRows(pixa3, 32, 1500, 1.0, 0, 20, 2);
    pixDisplay(pix4, 500, 0);
    pixWrite("/tmp/lept/recog/pix4.png", pix4, IFF_PNG);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
    pixDestroy(&pix1);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixaDestroy(&pixa1);
    boxaDestroy(&boxa1);
    recogDestroy(&recog);

    return 0;
}


