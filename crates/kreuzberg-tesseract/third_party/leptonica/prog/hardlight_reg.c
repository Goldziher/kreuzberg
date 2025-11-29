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
 * hardlight_reg.c
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void TestHardlight(const char *file1, const char *file2,
                          L_REGPARAMS *rp);

int main(int    argc,
         char **argv)
{
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    TestHardlight("hardlight1_1.jpg", "hardlight1_2.jpg", rp);
    TestHardlight("hardlight2_1.jpg", "hardlight2_2.jpg", rp);
    return regTestCleanup(rp);
}

void
TestHardlight(const char   *file1,
              const char   *file2,
              L_REGPARAMS  *rp)
{
PIX    *pixs1, *pixs2, *pix1, *pix2, *pixd;
PIXA   *pixa;
PIXAA  *paa;

        /* Read in images */
    pixs1 = pixRead(file1);
    pixs2 = pixRead(file2);
    paa = pixaaCreate(0);

        /* ---------- Test not-in-place; no colormaps ----------- */
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixs1, L_COPY);
    pixaAddPix(pixa, pixs2, L_COPY);
    pixaaAddPixa(paa, pixa, L_INSERT);
    pixd = pixBlendHardLight(NULL, pixs1, pixs2, 0, 0, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 0, 9 */
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixd, L_INSERT);

    pix2 = pixConvertTo32(pixs2);
    pixd = pixBlendHardLight(NULL, pixs1, pix2, 0, 0, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 1, 10 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixDestroy(&pix2);

    pixd = pixBlendHardLight(NULL, pixs2, pixs1, 0, 0, 1.0);
    pixaAddPix(pixa, pixd, L_INSERT);
    pixaaAddPixa(paa, pixa, L_INSERT);

        /* ---------- Test not-in-place; colormaps ----------- */
    pixa = pixaCreate(0);
    pix1 = pixMedianCutQuant(pixs1, 0);
    if (pixGetDepth(pixs2) == 8)
        pix2 = pixConvertGrayToColormap8(pixs2, 8);
    else
        pix2 = pixMedianCutQuant(pixs2, 0);
    pixaAddPix(pixa, pix1, L_COPY);
    pixaAddPix(pixa, pix2, L_COPY);
    pixaaAddPixa(paa, pixa, L_INSERT);

    pixa = pixaCreate(0);
    pixd = pixBlendHardLight(NULL, pix1, pixs2, 0, 0, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 2, 11 */
    pixaAddPix(pixa, pixd, L_INSERT);

    pixd = pixBlendHardLight(NULL, pix1, pix2, 0, 0, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 3, 12 */
    pixaAddPix(pixa, pixd, L_INSERT);

    pixd = pixBlendHardLight(NULL, pix2, pix1, 0, 0, 1.0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 4, 13 */
    pixaAddPix(pixa, pixd, L_INSERT);
    pixaaAddPixa(paa, pixa, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* ---------- Test in-place; no colormaps ----------- */
    pixa = pixaCreate(0);
    pixBlendHardLight(pixs1, pixs1, pixs2, 0, 0, 1.0);
    regTestWritePixAndCheck(rp, pixs1, IFF_PNG);  /* 5, 14 */
    pixaAddPix(pixa, pixs1, L_INSERT);

    pixs1 = pixRead(file1);
    pix2 = pixConvertTo32(pixs2);
    pixBlendHardLight(pixs1, pixs1, pix2, 0, 0, 1.0);
    regTestWritePixAndCheck(rp, pixs1, IFF_PNG);  /* 6, 15 */
    pixaAddPix(pixa, pixs1, L_INSERT);
    pixDestroy(&pix2);

    pixs1 = pixRead(file1);
    pixBlendHardLight(pixs2, pixs2, pixs1, 0, 0, 1.0);
    regTestWritePixAndCheck(rp, pixs2, IFF_PNG);  /* 7, 16 */
    pixaAddPix(pixa, pixs2, L_INSERT);
    pixaaAddPixa(paa, pixa, L_INSERT);
    pixDestroy(&pixs1);

    pixd = pixaaDisplayByPixa(paa, 4, 1.0, 20, 20, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 8, 17 */
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaaDestroy(&paa);
}
