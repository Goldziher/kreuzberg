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
 * livre_tophat.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
PIX   *pixs, *pixsg, *pix1, *pix2;
PIXA  *pixa;

    if (argc != 1)
	return ERROR_INT(" Syntax: livre_tophat", __func__, 1);
    setLeptDebugOK(1);

        /* Read the image in at 150 ppi. */
    pixs = pixRead("brothers.150.jpg");
    pixa = pixaCreate(0);
    pixaAddPix(pixa, pixs, L_INSERT);

    pixsg = pixConvertRGBToLuminance(pixs);

        /* Black tophat (closing - original-image) and invert */
    pix1 = pixTophat(pixsg, 15, 15, L_TOPHAT_BLACK);
    pixInvert(pix1, pix1);
    pixaAddPix(pixa, pix1, L_INSERT);

        /* Set black point at 200, white point at 245. */
    pix2 = pixGammaTRC(NULL, pix1, 1.0, 200, 245);
    pixaAddPix(pixa, pix2, L_INSERT);

        /* Generate the output image */
    lept_mkdir("lept/livre");
    lept_stderr("Writing to: /tmp/lept/livre/tophat.jpg\n");
    pix1 = pixaDisplayTiledAndScaled(pixa, 8, 350, 3, 0, 25, 2);
    pixWrite("/tmp/lept/livre/tophat.jpg", pix1, IFF_JFIF_JPEG);
    pixDisplay(pix1, 1200, 800);
    pixDestroy(&pix1);
    pixaDestroy(&pixa);
    pixDestroy(&pixsg);
    return 0;
}

