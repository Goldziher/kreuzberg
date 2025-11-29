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
 *  bilateral2_reg.c
 *
 *     Regression test for bilateral (nonlinear) filtering.
 *
 *     Separable operation with intermediate images at 4x reduction.
 *     This speeds the filtering up by about 30x compared to
 *     separable operation with full resolution intermediate images.
 *     Using 4x reduction on intermediates, this runs at about
 *     3 MPix/sec, with very good quality.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void DoTestsOnImage(PIX *pixs, L_REGPARAMS *rp);

static const l_int32  ncomps = 10;

int main(int    argc,
         char **argv)
{
PIX          *pixs;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixs = pixRead("test24.jpg");
    DoTestsOnImage(pixs, rp);  /* 0 - 7 */
    pixDestroy(&pixs);

    return regTestCleanup(rp);
}


static void
DoTestsOnImage(PIX          *pixs,
               L_REGPARAMS  *rp)
{
PIX   *pix, *pixd;
PIXA  *pixa;

    pixa = pixaCreate(0);
    pix = pixBilateral(pixs, 5.0, 10.0, ncomps, 4);  /* 0 */
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 5.0, 20.0, ncomps, 4);  /* 1 */
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 5.0, 40.0, ncomps, 4);  /* 2 */
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 5.0, 60.0, ncomps, 4);  /* 3 */
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 10.0, ncomps, 4);  /* 4 */
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 20.0, ncomps, 4);  /* 5 */
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 40.0, ncomps, 4);  /* 6 */
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pix = pixBilateral(pixs, 10.0, 60.0, ncomps, 4);  /* 7 */
    regTestWritePixAndCheck(rp, pix, IFF_JFIF_JPEG);
    pixaAddPix(pixa, pix, L_INSERT);
    pixd = pixaDisplayTiledInRows(pixa, 32, 2500, 1.0, 0, 30, 2);
    pixDisplayWithTitle(pixd, 100, 100, NULL, rp->display);
    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    return;
}


