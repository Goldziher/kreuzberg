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
 * digitprep1.c
 *
 *   Extract barcode digits and put in a pixaa (a resource file for
 *   readnum.c).
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  HEIGHT = 32;  /* pixels */

int main(int    argc,
         char **argv)
{
char       buf[16];
l_int32    i, n, h;
l_float32  scalefact;
BOXA      *boxa;
PIX       *pixs, *pix1, *pix2;
PIXA      *pixa, *pixas, *pixad;
PIXAA     *paa;

    if (argc != 1) {
        ERROR_INT(" Syntax: digitprep1", __func__, 1);
        return 1;
    }

    setLeptDebugOK(1);
    if ((pixs = pixRead("barcode-digits.png")) == NULL)
        return ERROR_INT("pixs not read", __func__, 1);

        /* Extract the digits and scale to HEIGHT */
    boxa = pixConnComp(pixs, &pixa, 8);
    pixas = pixaSort(pixa, L_SORT_BY_X, L_SORT_INCREASING, NULL, L_CLONE);
    n = pixaGetCount(pixas);

        /* Move the last ("0") to the first position */
    pix1 = pixaGetPix(pixas, n - 1, L_CLONE);
    pixaInsertPix(pixas, 0, pix1, NULL);
    pixaRemovePix(pixas, n);

        /* Make the output scaled pixa */
    pixad = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixas, i, L_CLONE);
        pixGetDimensions(pix1, NULL, &h, NULL);
        scalefact = HEIGHT / (l_float32)h;
        pix2 = pixScale(pix1, scalefact, scalefact);
        if (pixGetHeight(pix2) != 32)
            return ERROR_INT("height not 32!", __func__, 1);
        snprintf(buf, sizeof(buf), "%d", i);
        pixSetText(pix2, buf);
        pixaAddPix(pixad, pix2, L_INSERT);
        pixDestroy(&pix1);
    }

        /* Save in a pixaa, with 1 pix in each pixa */
    paa = pixaaCreateFromPixa(pixad, 1, L_CHOOSE_CONSECUTIVE, L_CLONE);
    pixaaWrite("/tmp/lept/barcode_digits.paa", paa);

        /* Show result */
    pix1 = pixaaDisplayByPixa(paa, 50, 1.0, 20, 20, 0);
    pixDisplay(pix1, 100, 100);
    pixDestroy(&pix1);

    pixDestroy(&pixs);
    boxaDestroy(&boxa);
    pixaDestroy(&pixa);
    pixaDestroy(&pixas);
    pixaDestroy(&pixad);
    pixaaDestroy(&paa);
    return 0;
}


