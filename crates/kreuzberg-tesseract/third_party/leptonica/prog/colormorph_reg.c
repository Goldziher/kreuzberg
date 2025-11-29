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
 *  colormorph_reg.c
 *
 *  Regression test for simple color morphological operations
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  SIZE = 7;

int main(int    argc,
         char **argv)
{
char          buf[256];
PIX          *pixs, *pix1, *pix2;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("wyom.jpg");
    pixa = pixaCreate(0);

    pix1 = pixColorMorph(pixs, L_MORPH_DILATE, SIZE, SIZE);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 0 */
    snprintf(buf, sizeof(buf), "d%d.%d", SIZE, SIZE);
    pix2 = pixColorMorphSequence(pixs, buf, 0, 0);
    regTestComparePix(rp, pix1, pix2);  /* 1 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix1 = pixColorMorph(pixs, L_MORPH_ERODE, SIZE, SIZE);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 2 */
    snprintf(buf, sizeof(buf), "e%d.%d", SIZE, SIZE);
    pix2 = pixColorMorphSequence(pixs, buf, 0, 0);
    regTestComparePix(rp, pix1, pix2);  /* 3 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix1 = pixColorMorph(pixs, L_MORPH_OPEN, SIZE, SIZE);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 4 */
    snprintf(buf, sizeof(buf), "o%d.%d", SIZE, SIZE);
    pix2 = pixColorMorphSequence(pixs, buf, 0, 0);
    regTestComparePix(rp, pix1, pix2);  /* 5 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    pix1 = pixColorMorph(pixs, L_MORPH_CLOSE, SIZE, SIZE);
    regTestWritePixAndCheck(rp, pix1, IFF_JFIF_JPEG);  /* 6 */
    snprintf(buf, sizeof(buf), "c%d.%d", SIZE, SIZE);
    pix2 = pixColorMorphSequence(pixs, buf, 0, 0);
    regTestComparePix(rp, pix1, pix2);  /* 7 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pix2);

    if (rp->display) {
        lept_mkdir("lept/cmorph");
        lept_stderr("Writing to: /tmp/lept/cmorph/colormorph.pdf\n");
        pixaConvertToPdf(pixa, 0, 1.0, L_FLATE_ENCODE, 0, "colormorph-test",
                         "/tmp/lept/cmorph/colormorph.pdf");
        lept_stderr("Writing to: /tmp/lept/cmorph/colormorph.jpg\n");
        pix1 = pixaDisplayTiledInColumns(pixa, 2, 1.0, 30, 2);
        pixWrite("/tmp/lept/cmorph/colormorph.jpg", pix1, IFF_JFIF_JPEG);
        pixDisplay(pix1, 100, 100);
        pixDestroy(&pix1);
    }
    pixaDestroy(&pixa);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}
