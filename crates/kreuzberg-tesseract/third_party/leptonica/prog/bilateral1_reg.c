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
 *  bilateral1_reg.c
 *
 *     Regression test for bilateral (nonlinear) filtering, with both:
 *      (1) Separable results with full res intermediate images
 *      (2) Exact results
 *     This test takes about 30 seconds, so it is not included
 *     in the alltests_reg set.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void DoTestsOnImage(PIX *pixs, L_REGPARAMS *rp, l_int32 width);

static const l_int32  ncomps = 10;

int main(int    argc,
         char **argv)
{
PIX          *pixs;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("rock.png");
    DoTestsOnImage(pixs, rp, 2000);  /* 0 - 16 */
    pixDestroy(&pixs);

    pixs = pixRead("church.png");
    DoTestsOnImage(pixs, rp, 1500);  /* 17 - 33 */
    pixDestroy(&pixs);

    pixs = pixRead("color-wheel-hue.jpg");
    DoTestsOnImage(pixs, rp, 1000);  /* 34 - 50 */
    pixDestroy(&pixs);

    return regTestCleanup(rp);
}


static void
DoTestsOnImage(PIX          *pixs,
               L_REGPARAMS  *rp,
               l_int32       width)
{
PIX   *pix, *pixd;
PIXA  *pixa;

    pixa = pixaCreate(0);
    pix = pixBilateral(pixs, 5.0, 10.0, ncomps, 1);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 5.0, 20.0, ncomps, 1);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 5.0, 40.0, ncomps, 1);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 5.0, 60.0, ncomps, 1);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 10.0, ncomps, 1);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 20.0, ncomps, 1);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 40.0, ncomps, 1);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 60.0, ncomps, 1);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 10.0, ncomps, 2);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 20.0, ncomps, 2);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 40.0, ncomps, 2);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 60.0, ncomps, 2);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBlockBilateralExact(pixs, 10.0, 10.0);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBlockBilateralExact(pixs, 10.0, 20.0);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBlockBilateralExact(pixs, 10.0, 40.0);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBlockBilateralExact(pixs, 10.0, 60.0);
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pixd = pixaDisplayTiledInRows(pixa, 32, width, 1.0, 0, 30, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_JFIF_JPEG);
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    return;
}

