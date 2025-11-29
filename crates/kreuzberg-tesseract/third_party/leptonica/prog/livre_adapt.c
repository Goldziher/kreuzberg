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
 * livre_adapt.c
 *
 * This shows two ways to normalize a document image for uneven
 * illumination.  It is somewhat more complicated than using the
 * morphological tophat.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
PIX   *pixs, *pix1, *pix2, *pix3, *pixr, *pixg, *pixb, *pixsg, *pixsm;
PIXA  *pixa;

    if (argc != 1)
        return ERROR_INT(" Syntax:  livre_adapt", __func__, 1);
    setLeptDebugOK(1);

        /* Read the image in at 150 ppi. */
    if ((pixs = pixRead("brothers.150.jpg")) == NULL)
        return ERROR_INT("pix not made", __func__, 1);
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixs, L_INSERT);

        /* Normalize for uneven illumination on RGB image */
    pixBackgroundNormRGBArraysMorph(pixs, NULL, 4, 5, 200,
                                    &pixr, &pixg, &pixb);
    pix1 = pixApplyInvBackgroundRGBMap(pixs, pixr, pixg, pixb, 4, 4);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pixr);
    pixDestroy(&pixg);
    pixDestroy(&pixb);

        /* Convert the RGB image to grayscale. */
    pixsg = pixConvertRGBToLuminance(pixs);
    pixaAddPix(pixa, pixsg, L_INSERT);

        /* Remove the text in the fg. */
    pix1 = pixCloseGray(pixsg, 25, 25);
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Smooth the bg with a convolution. */
    pixsm = pixBlockconv(pix1, 15, 15);
    pixaAddPix(pixa, pixsm, L_INSERT);

        /* Normalize for uneven illumination on gray image. */
    pixBackgroundNormGrayArrayMorph(pixsg, NULL, 4, 5, 200, &pixg);
    pix1 = pixApplyInvBackgroundGrayMap(pixsg, pixg, 4, 4);
    pixaAddPix(pixa, pix1, L_INSERT);
    pixDestroy(&pixg);

        /* Increase the dynamic range. */
    pix2 = pixGammaTRC(NULL, pix1, 1.0, 30, 180);
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Threshold to 1 bpp. */
    pix3 = pixThresholdToBinary(pix2, 120);
    pixaAddPix(pixa, pix3, L_INSERT);

            /* Generate the output image and pdf */
    lept_mkdir("lept/livre");
    lept_stderr("Writing jpg and pdf to: /tmp/lept/livre/adapt.*\n");
    pix1 = pixaDisplayTiledAndScaled(pixa, 8, 350, 4, 0, 25, 2);
    pixWrite("/tmp/lept/livre/adapt.jpg", pix1, IFF_DEFAULT);
    pixDisplay(pix1, 100, 100);
    pixaConvertToPdf(pixa, 0, 1.0, 0, 0, "Livre: adaptive thresholding",
                     "/tmp/lept/livre/adapt.pdf");
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    return 0;
}

