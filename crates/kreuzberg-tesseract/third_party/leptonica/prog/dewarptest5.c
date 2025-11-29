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
 *   dewarptest5.c
 *
 *   Tests dewarping model applied to word bounding boxes.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static l_int32 pageno = 35;
static l_int32 build_output = 0;
static l_int32 apply_output = 0;
static l_int32 map_output = 1;

l_int32 main(int    argc,
             char **argv)
{
char        buf[64];
BOXA       *boxa1, *boxa2, *boxa3, *boxa4;
L_DEWARP   *dew;
L_DEWARPA  *dewa;
PIX        *pixs, *pixn, *pixg, *pixb, *pix2, *pix3, *pix4, *pix5, *pix6;

    setLeptDebugOK(1);
    lept_mkdir("lept");

    snprintf(buf, sizeof(buf), "cat.%03d.jpg", pageno);
    pixs = pixRead(buf);
    dewa = dewarpaCreate(40, 30, 1, 15, 10);
    dewarpaUseBothArrays(dewa, 1);

        /* Normalize for varying background and binarize */
    pixn = pixBackgroundNormSimple(pixs, NULL, NULL);
    pixg = pixConvertRGBToGray(pixn, 0.5, 0.3, 0.2);
    pixb = pixThresholdToBinary(pixg, 130);
    pixDisplay(pixb, 0, 100);

        /* Build the model */
    dew = dewarpCreate(pixb, pageno);
    dewarpaInsertDewarp(dewa, dew);
    if (build_output) {
        snprintf(buf, sizeof(buf), "/tmp/lept/dewarp_build_%d.pdf", pageno);
        dewarpBuildPageModel(dew, buf);
    } else {
        dewarpBuildPageModel(dew, NULL);
    }

        /* Apply the model */
    dewarpPopulateFullRes(dew, pixg, 0, 0);
    if (apply_output) {
        snprintf(buf, sizeof(buf), "/tmp/lept/dewarp_apply_%d.pdf", pageno);
        dewarpaApplyDisparity(dewa, pageno, pixb, 200, 0, 0, &pix2, buf);
    } else {
        dewarpaApplyDisparity(dewa, pageno, pixb, 200, 0, 0, &pix2, NULL);
    }
    pixDisplay(pix2, 200, 100);

        /* Reverse direction: get the word boxes for the dewarped pix ... */
    pixGetWordBoxesInTextlines(pix2, 5, 5, 500, 100, &boxa1, NULL);
    pix3 = pixConvertTo32(pix2);
    pixRenderBoxaArb(pix3, boxa1, 2, 255, 0, 0);
    pixDisplay(pix3, 400, 100);

        /* ... and map to the word boxes for the input image */
    if (map_output) {
        snprintf(buf, sizeof(buf), "/tmp/lept/dewarp_map1_%d.pdf", pageno);
        dewarpaApplyDisparityBoxa(dewa, pageno, pix2, boxa1, 0, 0, 0, &boxa2,
                                  buf);
    } else {
        dewarpaApplyDisparityBoxa(dewa, pageno, pix2, boxa1, 0, 0, 0, &boxa2,
                                  NULL);
    }
    pix4 = pixConvertTo32(pixb);
    pixRenderBoxaArb(pix4, boxa2, 2, 0, 255, 0);
    pixDisplay(pix4, 600, 100);

        /* Forward direction: get the word boxes for the input pix ... */
    pixGetWordBoxesInTextlines(pixb, 5, 5, 500, 100, &boxa3, NULL);
    pix5 = pixConvertTo32(pixb);
    pixRenderBoxaArb(pix5, boxa3, 2, 255, 0, 0);
    pixDisplay(pix5, 800, 100);

        /* ... and map to the word boxes for the dewarped image */
    if (map_output) {
        snprintf(buf, sizeof(buf), "/tmp/lept/dewarp_map2_%d.pdf", pageno);
        dewarpaApplyDisparityBoxa(dewa, pageno, pixb, boxa3, 1, 0, 0, &boxa4,
                                  buf);
    } else {
        dewarpaApplyDisparityBoxa(dewa, pageno, pixb, boxa3, 1, 0, 0, &boxa4,
                                  NULL);
    }
    pix6 = pixConvertTo32(pix2);
    pixRenderBoxaArb(pix6, boxa4, 2, 0, 255, 0);
    pixDisplay(pix6, 1000, 100);

    dewarpaDestroy(&dewa);
    pixDestroy(&pixs);
    pixDestroy(&pixn);
    pixDestroy(&pixg);
    pixDestroy(&pixb);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    boxaDestroy(&boxa3);
    boxaDestroy(&boxa4);
    return 0;
}
