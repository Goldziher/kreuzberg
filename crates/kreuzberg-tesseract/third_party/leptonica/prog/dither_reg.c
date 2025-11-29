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
 * dither_reg.c
 *
 *    Test dithering from 8 bpp to 1 bpp and 2 bpp.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
PIX          *pix, *pixs, *pix1, *pix2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pix = pixRead("test8.jpg");
    pixs = pixGammaTRC(NULL, pix, 1.3, 0, 255);  /* gamma of 1.3, for fun */

        /* Dither to 1 bpp */
    pix1 = pixDitherToBinary(pixs);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix1, 0, 0, NULL, rp->display);
    pixDestroy(&pix1);

         /* Dither to 2 bpp, with colormap */
    pix1 = pixDitherTo2bpp(pixs, 1);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix1, 400, 0, NULL, rp->display);

         /* Dither to 2 bpp, without colormap */
    pix2 = pixDitherTo2bpp(pixs, 0);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pix2, 800, 0, NULL, rp->display);
    regTestComparePix(rp, pix1, pix2);   /* 3 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Dither 2x upscale to 1 bpp */
    pix1 = pixScaleGray2xLIDither(pixs);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    pixDisplayWithTitle(pix1, 0, 400, NULL, rp->display);
    pixDestroy(&pix1);

        /* Dither 4x upscale to 1 bpp */
    pix1 = pixScaleGray4xLIDither(pixs);
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 5 */
    pixDisplayWithTitle(pix1, 700, 400, NULL, rp->display);
    pixDestroy(&pix1);

    pixDestroy(&pix);
    pixDestroy(&pixs);
    return regTestCleanup(rp);
}

