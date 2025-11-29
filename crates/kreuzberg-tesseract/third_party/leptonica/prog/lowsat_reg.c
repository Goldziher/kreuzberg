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
 * lowsat_reg.c
 *
 *   Testing functions that identify and modify image pixels that
 *   have low saturation (i.e., are essentially gray).
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       i, j, wpl, val;
l_uint32      gray32;
l_uint32     *data, *line;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/lowsat");
    pixa = pixaCreate(0);
    pix1 = pixRead("zier.jpg");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix1, 0, 100, NULL, rp->display);
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Embed the image in a varying gray background */
    pix2 = pixCreate(400, 580, 32);
    data = pixGetData(pix2);
    wpl = pixGetWpl(pix2);
    for (i = 0; i < 580; i++) {
        line = data + i * wpl;
        val = 150 + 50 * i / 580;
        for (j = 0; j < 400; j++) {
            composeRGBPixel(val, val, val, &gray32);
            line[j] = gray32;
        }
    }
    pixRasterop(pix2, 70, 90, 270, 400, PIX_SRC, pix1, 0, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    pixaAddPix(pixa, pix2, L_COPY);
    pixDisplayWithTitle(pix2, 300, 100, NULL, rp->display);

        /* Darken the gray pixels, leaving most of the
         * the others unaffected.  */
    pix3 = pixDarkenGray(NULL, pix2, 220, 10);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 2 */
    pixaAddPix(pixa, pix3, L_COPY);
    pixDisplayWithTitle(pix3, 700, 100, "gray pixels are black", rp->display);

        /* We can also generate a mask over the gray pixels,
         * eliminating noise from very dark pixels morphologically. */
    pix4 = pixMaskOverGrayPixels(pix2, 220, 10);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 3 */
    pixaAddPix(pixa, pix4, L_INSERT);
    pixDisplayWithTitle(pix4, 1100, 100, "mask over gray pixels", rp->display);
    pix5 = pixMorphSequence(pix4, "o20.20", 0);  /* remove noise */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 4 */
    pixaAddPix(pixa, pix5, L_COPY);
    pixDisplayWithTitle(pix5, 1500, 100, "clean mask over gray", rp->display);
    pixInvert(pix5, pix5);
    pix6 = pixConvertTo32(pix5);
    pix7 = pixAddRGB(pix2, pix6);
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pix7, L_INSERT);
    pixDisplayWithTitle(pix7, 1900, 100, NULL, rp->display);

    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix5);
    pixDestroy(&pix6);
    pixaDestroy(&pixa);
    return regTestCleanup(rp);
}
