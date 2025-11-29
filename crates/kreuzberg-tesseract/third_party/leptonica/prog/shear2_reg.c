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
 *   shear2_reg.c
 *
 *    Regression test for quadratic shear, both sampled and interpolated.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void PixSave(PIX **ppixs, PIXA *pixa, l_int32 newrow,
             L_BMF *bmf, const char *textstr);


l_int32 main(int    argc,
             char **argv)
{
L_BMF        *bmf;
PIX          *pixs1, *pixs2, *pixg, *pixt, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "shear2_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    bmf = bmfCreate("./fonts", 8);
    pixs1 = pixCreate(301, 301, 32);
    pixs2 = pixCreate(601, 601, 32);
    pixSetAll(pixs1);
    pixSetAll(pixs2);
    pixRenderLineArb(pixs1, 0, 20, 300, 20, 5, 0, 0, 255);
    pixRenderLineArb(pixs1, 0, 70, 300, 70, 5, 0, 255, 0);
    pixRenderLineArb(pixs1, 0, 120, 300, 120, 5, 0, 255, 255);
    pixRenderLineArb(pixs1, 0, 170, 300, 170, 5, 255, 0, 0);
    pixRenderLineArb(pixs1, 0, 220, 300, 220, 5, 255, 0, 255);
    pixRenderLineArb(pixs1, 0, 270, 300, 270, 5, 255, 255, 0);
    pixRenderLineArb(pixs2, 0, 20, 300, 20, 5, 0, 0, 255);
    pixRenderLineArb(pixs2, 0, 70, 300, 70, 5, 0, 255, 0);
    pixRenderLineArb(pixs2, 0, 120, 300, 120, 5, 0, 255, 255);
    pixRenderLineArb(pixs2, 0, 170, 300, 170, 5, 255, 0, 0);
    pixRenderLineArb(pixs2, 0, 220, 300, 220, 5, 255, 0, 255);
    pixRenderLineArb(pixs2, 0, 270, 300, 270, 5, 255, 255, 0);

        /* Color, small pix */
    pixa = pixaCreate(0);
    pixt = pixQuadraticVShear(pixs1, L_WARP_TO_LEFT,
                                   60, -20, L_SAMPLED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 1, bmf, "sampled-left");
    pixt = pixQuadraticVShear(pixs1, L_WARP_TO_RIGHT,
                                   60, -20, L_SAMPLED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 0, bmf, "sampled-right");
    pixt = pixQuadraticVShear(pixs1, L_WARP_TO_LEFT,
                                   60, -20, L_INTERPOLATED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 1, bmf, "interpolated-left");
    pixt = pixQuadraticVShear(pixs1, L_WARP_TO_RIGHT,
                                   60, -20, L_INTERPOLATED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 0, bmf, "interpolated-right");
    pixd = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    pixDisplayWithTitle(pixd, 50, 50, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

        /* Grayscale, small pix */
    pixg = pixConvertTo8(pixs1, 0);
    pixa = pixaCreate(0);
    pixt = pixQuadraticVShear(pixg, L_WARP_TO_LEFT,
                                   60, -20, L_SAMPLED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 1, bmf, "sampled-left");
    pixt = pixQuadraticVShear(pixg, L_WARP_TO_RIGHT,
                                   60, -20, L_SAMPLED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 0, bmf, "sampled-right");
    pixt = pixQuadraticVShear(pixg, L_WARP_TO_LEFT,
                                   60, -20, L_INTERPOLATED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 1, bmf, "interpolated-left");
    pixt = pixQuadraticVShear(pixg, L_WARP_TO_RIGHT,
                                   60, -20, L_INTERPOLATED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 0, bmf, "interpolated-right");
    pixd = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    pixDisplayWithTitle(pixd, 250, 50, NULL, rp->display);
    pixDestroy(&pixg);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

        /* Color, larger pix */
    pixa = pixaCreate(0);
    pixt = pixQuadraticVShear(pixs2, L_WARP_TO_LEFT,
                              120, -40, L_SAMPLED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 1, bmf, "sampled-left");
    pixt = pixQuadraticVShear(pixs2, L_WARP_TO_RIGHT,
                              120, -40, L_SAMPLED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 0, bmf, "sampled-right");
    pixt = pixQuadraticVShear(pixs2, L_WARP_TO_LEFT,
                              120, -40, L_INTERPOLATED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 1, bmf, "interpolated-left");
    pixt = pixQuadraticVShear(pixs2, L_WARP_TO_RIGHT,
                              120, -40, L_INTERPOLATED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 0, bmf, "interpolated-right");
    pixd = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    pixDisplayWithTitle(pixd, 550, 50, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

        /* Grayscale, larger pix */
    pixg = pixConvertTo8(pixs2, 0);
    pixa = pixaCreate(0);
    pixt = pixQuadraticVShear(pixg, L_WARP_TO_LEFT,
                              60, -20, L_SAMPLED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 1, bmf, "sampled-left");
    pixt = pixQuadraticVShear(pixg, L_WARP_TO_RIGHT,
                              60, -20, L_SAMPLED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 0, bmf, "sampled-right");
    pixt = pixQuadraticVShear(pixg, L_WARP_TO_LEFT,
                              60, -20, L_INTERPOLATED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 1, bmf, "interpolated-left");
    pixt = pixQuadraticVShear(pixg, L_WARP_TO_RIGHT,
                              60, -20, L_INTERPOLATED, L_BRING_IN_WHITE);
    PixSave(&pixt, pixa, 0, bmf, "interpolated-right");
    pixd = pixaDisplayTiledInColumns(pixa, 2, 1.0, 20, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    pixDisplayWithTitle(pixd, 850, 50, NULL, rp->display);
    pixDestroy(&pixg);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);

    pixDestroy(&pixs1);
    pixDestroy(&pixs2);
    bmfDestroy(&bmf);
    return regTestCleanup(rp);
}


void
PixSave(PIX        **ppixs,
        PIXA        *pixa,
        l_int32      newrow,
        L_BMF       *bmf,
        const char  *textstr)
{
PIX  *pix1, *pix2, *pix3;

    pix1 = pixConvertTo32(*ppixs);
    pix2 = pixAddBorder(pix1, 3, 0);
    pix3 = pixAddSingleTextblock(pix2, bmf, textstr, 0xff000000, L_ADD_BELOW,
                                 NULL);
    pixaAddPix(pixa, pix3, L_INSERT);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(ppixs);
}
