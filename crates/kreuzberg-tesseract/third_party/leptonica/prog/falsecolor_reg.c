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
 * falsecolor_reg.c
 *
 *    Test false color generation from 8 and 16 bpp gray
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_float32  gamma[] = {1.0, 2.0, 3.0};

int main(int    argc,
         char **argv)
{
l_int32       i, j, val8, val16;
NUMA         *na;
PIX          *pix1, *pix8, *pix16, *pix8f, *pix16f;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pix8 = pixCreate(768, 100, 8);
    pix16 = pixCreate(768, 100, 16);
    for (i = 0; i < 100; i++) {
        for (j = 0; j < 768; j++) {
            val16 = 0xffff * j / 768;
            pixSetPixel(pix16, j, i, val16);
            val8 = 0xff * j / 768;
            pixSetPixel(pix8, j, i, val8);
        }
    }
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);   /* 0 */
    regTestWritePixAndCheck(rp, pix16, IFF_PNG);  /* 1 */

    pixa = pixaCreate(8);
    pixaAddPix(pixa, pix8, L_INSERT);
    for (i = 0; i < 3; i++) {
        pix8f = pixConvertGrayToFalseColor(pix8, gamma[i]);
        regTestWritePixAndCheck(rp, pix8f, IFF_PNG);  /* 2 - 4 */
        pixaAddPix(pixa, pix8f, L_INSERT);
    }
    pixaAddPix(pixa, pix16, L_INSERT);
    for (i = 0; i < 3; i++) {
        pix16f = pixConvertGrayToFalseColor(pix16, gamma[i]);
        regTestWritePixAndCheck(rp, pix16f, IFF_PNG);  /* 5 - 7 */
        pixaAddPix(pixa, pix16f, L_INSERT);
    }

    if (rp->display) {
            /* Use na to display them in column major order with 4 rows */
        na = numaCreate(8);
        for (i = 0; i < 8; i++)
            numaAddNumber(na, i / 4);
        pix1 = pixaDisplayTiledByIndex(pixa, na, 768, 20, 2, 6, 0xff000000);
        pixDisplay(pix1, 100, 100);
        numaDestroy(&na);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa);
    return regTestCleanup(rp);
}

