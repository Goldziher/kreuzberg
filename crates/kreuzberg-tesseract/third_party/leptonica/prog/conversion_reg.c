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
 * conversion_reg.c
 *
 *    Regression test for depth conversion functions,
 *    including some of the octcube quantization.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char        *errorstr;
l_int32      same, error;
PIX         *pixs1, *pixs2, *pixs4, *pixs8, *pixs16, *pixs32;
PIX         *pixc2, *pixc4, *pixc8;
PIX         *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIXA        *pixa;
PIXCMAP     *cmap;
SARRAY      *sa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs1 = pixRead("test1.png");
    pixs2 = pixRead("dreyfus2.png");
    pixc2 = pixRead("weasel2.4c.png");
    pixs4 = pixRead("weasel4.16g.png");
    pixc4 = pixRead("weasel4.11c.png");
    pixs8 = pixRead("karen8.jpg");
    pixc8 = pixRead("weasel8.240c.png");
    pixs16 = pixRead("test16.tif");
    pixs32 = pixRead("marge.jpg");
    error = FALSE;
    sa = sarrayCreate(0);

        /* Conversion: 1 bpp --> 8 bpp --> 1 bpp */
    pix1 = pixConvertTo8(pixs1, FALSE);
    pix2 = pixThreshold8(pix1, 1, 0, 0);
    regTestComparePix(rp, pixs1, pix2);  /* 0 */
    pixEqual(pixs1, pix2, &same);
    if (!same) {
        pixDisplayWithTitle(pixs1, 100, 100, "1 bpp, no cmap", 1);
        pixDisplayWithTitle(pix2, 500, 100, "1 bpp, no cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 1 bpp <==> 8 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 1 bpp <==> 8 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Conversion: 2 bpp --> 8 bpp --> 2 bpp */
        /* Conversion: 2 bpp cmap --> 8 bpp cmap --> 2 bpp cmap */
    pix1 = pixRemoveColormap(pixs2, REMOVE_CMAP_TO_GRAYSCALE);
    pix2 = pixThreshold8(pix1, 2, 4, 0);
    pix3 = pixConvertTo8(pix2, FALSE);
    pix4 = pixThreshold8(pix3, 2, 4, 0);
    regTestComparePix(rp, pix2, pix4);  /* 1 */
    pixEqual(pix2, pix4, &same);
    if (!same) {
        pixDisplayWithTitle(pix2, 100, 100, "2 bpp, no cmap", 1);
        pixDisplayWithTitle(pix4, 500, 100, "2 bpp, no cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 2 bpp <==> 8 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 2 bpp <==> 8 bpp\n");
    }
    pix5 = pixConvertTo8(pixs2, TRUE);
    pix6 = pixThreshold8(pix5, 2, 4, 1);
    regTestComparePix(rp, pixs2, pix6);  /* 2 */
    pixEqual(pixs2, pix6, &same);
    if (!same) {
        pixDisplayWithTitle(pixs2, 100, 100, "2 bpp, cmap", 1);
        pixDisplayWithTitle(pix6, 500, 100, "2 bpp, cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 2 bpp <==> 8 bpp; cmap",
                        L_COPY);
    } else {
        lept_stderr("OK: conversion 2 bpp <==> 8 bpp; cmap\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);

        /* Conversion: 4 bpp --> 8 bpp --> 4 bpp */
        /* Conversion: 4 bpp cmap --> 8 bpp cmap --> 4 bpp cmap */
    pix1 = pixRemoveColormap(pixs4, REMOVE_CMAP_TO_GRAYSCALE);
    pix2 = pixThreshold8(pix1, 4, 16, 0);
    pix3 = pixConvertTo8(pix2, FALSE);
    pix4 = pixThreshold8(pix3, 4, 16, 0);
    regTestComparePix(rp, pix2, pix4);  /* 3 */
    pixEqual(pix2, pix4, &same);
    if (!same) {
        pixDisplayWithTitle(pix2, 100, 100, "4 bpp, no cmap", 1);
        pixDisplayWithTitle(pix4, 500, 100, "4 bpp, no cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 4 bpp <==> 8 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 4 bpp <==> 8 bpp\n");
    }
    pix5 = pixConvertTo8(pixs4, TRUE);
    pix6 = pixThreshold8(pix5, 4, 16, 1);
    regTestComparePix(rp, pixs4, pix6);  /* 4 */
    pixEqual(pixs4, pix6, &same);
    if (!same) {
        pixDisplayWithTitle(pixs4, 100, 100, "4 bpp, cmap", 1);
        pixDisplayWithTitle(pix6, 500, 100, "4 bpp, cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 4 bpp <==> 8 bpp, cmap",
                        L_COPY);
    } else {
        lept_stderr("OK: conversion 4 bpp <==> 8 bpp; cmap\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);

        /* Conversion: 2 bpp cmap --> 2 bpp --> 2 bpp cmap --> 2 bpp */
    pix1 = pixRemoveColormap(pixs2, REMOVE_CMAP_TO_GRAYSCALE);
    pix2 = pixConvertGrayToColormap(pix1);
    pix3 = pixRemoveColormap(pix2, REMOVE_CMAP_TO_GRAYSCALE);
    pix4 = pixThresholdTo2bpp(pix3, 4, 1);
    regTestComparePix(rp, pix1, pix4);  /* 5 */
    pixEqual(pix1, pix4, &same);
    if (!same) {
        pixDisplayWithTitle(pixs2, 100, 100, "2 bpp, cmap", 1);
        pixDisplayWithTitle(pix4, 500, 100, "2 bpp, cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 2 bpp <==> 2 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 2 bpp <==> 2 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Conversion: 4 bpp cmap --> 4 bpp --> 4 bpp cmap --> 4 bpp */
    pix1 = pixRemoveColormap(pixs4, REMOVE_CMAP_TO_GRAYSCALE);
    pix2 = pixConvertGrayToColormap(pix1);
    pix3 = pixRemoveColormap(pix2, REMOVE_CMAP_TO_GRAYSCALE);
    pix4 = pixThresholdTo4bpp(pix3, 16, 1);
    regTestComparePix(rp, pix1, pix4);  /* 6 */
    pixEqual(pix1, pix4, &same);
    if (!same) {
        pixDisplayWithTitle(pixs4, 100, 100, "4 bpp, cmap", 1);
        pixDisplayWithTitle(pix4, 500, 100, "4 bpp, cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 4 bpp <==> 4 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 4 bpp <==> 4 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Conversion: 8 bpp --> 8 bpp cmap --> 8 bpp */
    pix1 = pixConvertTo8(pixs8, TRUE);
    pix2 = pixConvertTo8(pix1, FALSE);
    regTestComparePix(rp, pixs8, pix2);  /* 7 */
    pixEqual(pixs8, pix2, &same);
    if (!same) {
        pixDisplayWithTitle(pix1, 100, 100, "8 bpp, cmap", 1);
        pixDisplayWithTitle(pix2, 500, 100, "8 bpp, no cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 8 bpp <==> 8 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 8 bpp <==> 8 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Conversion: 2 bpp cmap --> 32 bpp --> 2 bpp cmap */
    pix1 = pixConvertTo8(pixc2, TRUE);
    pix2 = pixConvertTo32(pix1);
    pix3 = pixConvertTo32(pixc2);
    regTestComparePix(rp, pix2, pix3);  /* 8 */
    pixEqual(pix2, pix3, &same);
    if (!same) {
        pixDisplayWithTitle(pix2, 100, 100, "32 bpp", 1);
        pixDisplayWithTitle(pix3, 500, 100, "32 bpp", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 2 bpp ==> 32 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 2 bpp <==> 32 bpp\n");
    }
    cmap = pixGetColormap(pixc2);
    pix4 = pixOctcubeQuantFromCmap(pix3, cmap, 2, 4, L_EUCLIDEAN_DISTANCE);
    regTestComparePix(rp, pixc2, pix4);  /* 9 */
    pixEqual(pixc2, pix4, &same);
    if (!same) {
        pixDisplayWithTitle(pixc2, 100, 100, "4 bpp, cmap", 1);
        pixDisplayWithTitle(pix4, 500, 100, "4 bpp, cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 2 bpp <==> 32 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 2 bpp <==> 32 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Conversion: 4 bpp cmap --> 32 bpp --> 4 bpp cmap */
    pix1 = pixConvertTo8(pixc4, TRUE);
    pix2 = pixConvertTo32(pix1);
    pix3 = pixConvertTo32(pixc4);
    regTestComparePix(rp, pix2, pix3);  /* 10 */
    pixEqual(pix2, pix3, &same);
    if (!same) {
        pixDisplayWithTitle(pix2, 100, 100, "32 bpp", 1);
        pixDisplayWithTitle(pix3, 500, 100, "32 bpp", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 4 bpp ==> 32 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 4 bpp <==> 32 bpp\n");
    }
    cmap = pixGetColormap(pixc4);
    pix4 = pixOctcubeQuantFromCmap(pix3, cmap, 2, 4, L_EUCLIDEAN_DISTANCE);
    regTestComparePix(rp, pixc4, pix4);  /* 11 */
    pixEqual(pixc4, pix4, &same);
    if (!same) {
        pixDisplayWithTitle(pixc4, 100, 100, "4 bpp, cmap", 1);
        pixDisplayWithTitle(pix4, 500, 100, "4 bpp, cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 4 bpp <==> 32 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 4 bpp <==> 32 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

        /* Conversion: 8 bpp --> 32 bpp --> 8 bpp */
    pix1 = pixConvertTo32(pixs8);
    pix2 = pixConvertTo8(pix1, FALSE);
    regTestComparePix(rp, pixs8, pix2);  /* 12 */
    pixEqual(pixs8, pix2, &same);
    if (!same) {
        pixDisplayWithTitle(pixs8, 100, 100, "8 bpp", 1);
        pixDisplayWithTitle(pix2, 500, 100, "8 bpp", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 8 bpp <==> 32 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 8 bpp <==> 32 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Conversion: 8 bpp --> 16 bpp --> 8 bpp */
    pix1 = pixConvert8To16(pixs8, 8);
    pix2 = pixConvertTo8(pix1, FALSE);
    regTestComparePix(rp, pixs8, pix2);  /* 13 */
    pixEqual(pixs8, pix2, &same);
    if (!same) {
        pixDisplayWithTitle(pixs8, 100, 100, "8 bpp", 1);
        pixDisplayWithTitle(pix2, 500, 100, "8 bpp", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 8 bpp <==> 16 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 8 bpp <==> 16 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Conversion: 16 bpp --> 8 bpp --> 16 bpp */
    pix1 = pixConvert16To8(pixs16, 1);
    pix2 = pixConvertTo16(pix1);
    regTestComparePix(rp, pixs16, pix2);  /* 14 */
    pixEqual(pixs16, pix2, &same);
    if (!same) {
        pixDisplayWithTitle(pixs16, 100, 100, "16 bpp", 1);
        pixDisplayWithTitle(pix2, 500, 100, "16 bpp", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 16 bpp <==> 8 bpp", L_COPY);
    } else {
        lept_stderr("OK: conversion 16 bpp <==> 8 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Conversion: 8 bpp cmap --> 32 bpp --> 8 bpp cmap */
        /* Required to go to level 6 of octcube to get identical result */
    pix1 = pixConvertTo32(pixc8);
    cmap = pixGetColormap(pixc8);
    pix2 = pixOctcubeQuantFromCmap(pix1, cmap, 2, 6, L_EUCLIDEAN_DISTANCE);
    regTestComparePix(rp, pixc8, pix2);  /* 15 */
    pixEqual(pixc8, pix2, &same);
    if (!same) {
        pixDisplayWithTitle(pixc8, 100, 100, "8 bpp cmap", 1);
        pixDisplayWithTitle(pix2, 500, 100, "8 bpp cmap", 1);
        error = TRUE;
        sarrayAddString(sa, "conversion 8 bpp cmap <==> 32 bpp cmap",
                        L_COPY);
    } else {
        lept_stderr("OK: conversion 8 bpp <==> 32 bpp\n");
    }
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Summarize results so far */
    if (error == FALSE) {
        lept_stderr("No errors found\n");
    } else {
        errorstr = sarrayToString(sa, 1);
        lept_stderr("Errors in the following:\n %s", errorstr);
        lept_free(errorstr);
    }

        /* General onversion to 2 bpp */
    pixa = pixaCreate(0);
    pix1 = pixConvertTo2(pixs1);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 16 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo2(pixs2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 17 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo2(pixc2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 18 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo2(pixs4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 19 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo2(pixc4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 20 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo2(pixs8);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 21 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo2(pixc8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 22 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo2(pixs32);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 23 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixaDisplayTiledAndScaled(pixa, 32, 300, 4, 0, 30, 2);
    pixDisplayWithTitle(pix2, 500, 0, NULL, rp->display);
    pixDestroy(&pix2);
    pixaDestroy(&pixa);

        /* General onversion to 4 bpp */
    pixa = pixaCreate(0);
    pix1 = pixConvertTo4(pixs1);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 24 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo4(pixs2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 25 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo4(pixc2);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 26 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo4(pixs4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 27 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo4(pixc4);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 28 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo4(pixs8);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 29 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo4(pixc8);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 30 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix1 = pixConvertTo4(pixs32);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 31 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixaDisplayTiledAndScaled(pixa, 32, 300, 4, 0, 30, 2);
    pixDisplayWithTitle(pix2, 500, 750, NULL, rp->display);
    pixDestroy(&pix2);
    pixaDestroy(&pixa);

    sarrayDestroy(&sa);
    pixDestroy(&pixs1);
    pixDestroy(&pixs2);
    pixDestroy(&pixs4);
    pixDestroy(&pixc2);
    pixDestroy(&pixc4);
    pixDestroy(&pixs8);
    pixDestroy(&pixc8);
    pixDestroy(&pixs16);
    pixDestroy(&pixs32);
    return regTestCleanup(rp);
}

