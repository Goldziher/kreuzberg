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
 * boxa1_reg.c
 *
 *    This carries out various operations on boxa, including
 *    region comparison, transforms and display.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static PIX *DisplayBoxa(BOXA  *boxa);

int main(int    argc,
         char **argv)
{
l_uint8      *data1, *data2;
l_int32       same;
size_t        size1, size2;
l_float32     diffarea, diffxor;
BOX          *box;
BOXA         *boxa1, *boxa2, *boxa3;
PIX          *pix1;
PTA          *pta;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/boxa");

        /* Make a boxa and display its contents */
    boxa1 = boxaCreate(6);
    box = boxCreate(60, 60, 40, 20);
    boxaAddBox(boxa1, box, L_INSERT);
    box = boxCreate(120, 50, 20, 50);
    boxaAddBox(boxa1, box, L_INSERT);
    box = boxCreate(50, 140, 46, 60);
    boxaAddBox(boxa1, box, L_INSERT);
    box = boxCreate(166, 130, 64, 28);
    boxaAddBox(boxa1, box, L_INSERT);
    box = boxCreate(64, 224, 44, 34);
    boxaAddBox(boxa1, box, L_INSERT);
    box = boxCreate(117, 206, 26, 74);
    boxaAddBox(boxa1, box, L_INSERT);
    pix1 = DisplayBoxa(boxa1);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixDestroy(&pix1);

    boxaCompareRegions(boxa1, boxa1, 100, &same, &diffarea, &diffxor, NULL);
    regTestCompareValues(rp, 1, same, 0.0);  /* 1 */
    regTestCompareValues(rp, 0.0, diffarea, 0.0);  /* 2 */
    regTestCompareValues(rp, 0.0, diffxor, 0.0);  /* 3 */

    boxa2 = boxaTransform(boxa1, -13, -13, 1.0, 1.0);
    boxaCompareRegions(boxa1, boxa2, 10, &same, &diffarea, &diffxor, NULL);
    regTestCompareValues(rp, 1, same, 0.0);  /* 4 */
    regTestCompareValues(rp, 0.0, diffarea, 0.0);  /* 5 */
    regTestCompareValues(rp, 0.0, diffxor, 0.0);  /* 6 */
    boxaDestroy(&boxa2);

    pta = boxaExtractCorners(boxa1, L_UPPER_LEFT);
    boxa2 = boxaShiftWithPta(boxa1, pta, 1);  /* shift */
    boxa3 = boxaShiftWithPta(boxa2, pta, -1);  /* inverse shift */
    boxaWriteMem(&data1, &size1, boxa1);
    boxaWriteMem(&data2, &size2, boxa3);
    regTestCompareStrings(rp, data1, size1, data2, size2);  /* 7 */
    ptaDestroy(&pta);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    boxaDestroy(&boxa3);
    lept_free(data1);
    lept_free(data2);

        /* Test serialized boxa I/O to and from memory */
    data1 = l_binaryRead("boxa2.ba", &size1);
    boxa1 = boxaReadMem(data1, size1);
    boxaWriteMem(&data2, &size2, boxa1);
    boxa2 = boxaReadMem(data2, size2);
    boxaWrite("/tmp/lept/boxa/boxa1.ba", boxa1);
    boxaWrite("/tmp/lept/boxa/boxa2.ba", boxa2);
    filesAreIdentical("/tmp/lept/boxa/boxa1.ba", "/tmp/lept/boxa/boxa2.ba",
                      &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 8 */
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    lept_free(data1);
    lept_free(data2);

    return regTestCleanup(rp);
}


static PIX *
DisplayBoxa(BOXA  *boxa)
{
l_int32  w, h;
BOX     *box;
PIX     *pix1, *pix2, *pix3;
PIXA    *pixa;

    pixa = pixaCreate(2);
    boxaGetExtent(boxa, &w, &h, &box);
    pix1 = pixCreate(w, h, 1);
    pixMaskBoxa(pix1, pix1, boxa, L_SET_PIXELS);
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixCreate(w, h, 32);
    pixSetAll(pix2);
    pixRenderBoxaArb(pix2, boxa, 2, 0, 255, 0);
    pixRenderBoxArb(pix2, box, 3, 255, 0, 0);
    pixaAddPix(pixa, pix2, L_INSERT);
    pix3 = pixaDisplayTiledInRows(pixa, 32, 1000, 1.0, 0, 30, 2);
    boxDestroy(&box);
    pixaDestroy(&pixa);
    return pix3;
}
