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
 * binmorph6_reg.c
 *
 *    Miscellaneous morphological operations.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
BOX          *box1;
PIX          *pix1, *pix2, *pix3, *pix4, *pix5, *pix6, *pix7, *pix8;
PIXA         *pixa;
SEL          *sel;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

        /* Test making sel from a pix */
    pixa = pixaCreate(10);
    pix1 = pixRead("feyn-fract.tif");
    box1 = boxCreate(507, 65, 60, 36);
    pix2 = pixClipRectangle(pix1, box1, NULL);
    sel = selCreateFromPix(pix2, 6, 6, "life");  /* 610 hits */

        /* Note how the closing tries to put the negative
         * of the sel, inverted spatially, in the background.  */
    pix3 = pixDilate(NULL, pix1, sel);  /* note the small holes */
    pix4 = pixOpen(NULL, pix1, sel);  /* just the sel */
    pix5 = pixCloseSafe(NULL, pix1, sel);  /* expands small holes in dilate */
    pix6 = pixSubtract(NULL, pix3, pix1);
    pix7 = pixSubtract(NULL, pix1, pix5);  /* no pixels because closing
                                            * is extensive */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 0 */
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 1 */
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 2 */
    regTestWritePixAndCheck(rp, pix5, IFF_PNG);  /* 3 */
    regTestWritePixAndCheck(rp, pix6, IFF_PNG);  /* 4 */
    regTestWritePixAndCheck(rp, pix7, IFF_PNG);  /* 5 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixaAddPix(pixa, pix4, L_INSERT);
    pixaAddPix(pixa, pix5, L_INSERT);
    pixaAddPix(pixa, pix6, L_INSERT);
    pixaAddPix(pixa, pix7, L_INSERT);

    pix8 = pixaDisplayTiledInColumns(pixa, 2, 0.75, 20, 2);
    regTestWritePixAndCheck(rp, pix8, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pix8, 100, 0, NULL, rp->display);
    pixDestroy(&pix2);
    pixDestroy(&pix8);
    pixaDestroy(&pixa);
    boxDestroy(&box1);
    selDestroy(&sel);
    return regTestCleanup(rp);
}

