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
 * flipdetect_reg.c
 *
 *   flipdetect_reg
 *
 *   - Tests the high-level text orientation interface
 *   - Tests 90 degree orientation of text and whether the text is
 *     mirror reversed.
 *   - Shows the typical 'confidence' outputs from functions in flipdetect.c.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32       orient, rotation;
l_float32     upconf, leftconf, conf;
PIX          *pix, *pixs, *pix1, *pix2;
PIXA         *pixa;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pix = pixRead("feyn.tif");
    pixs = pixScale(pix, 0.5, 0.5);
    pixDestroy(&pix);

        /* Test high-level interface */
    lept_stderr("\nTest high-level detection/rotation\n");
    pix1 = pixRotateOrth(pixs, 3);
    pix2 = pixOrientCorrect(pix1, 0.0, 0.0, &upconf, &leftconf,
                            &rotation, 0);
    if (rp->display)
        lept_stderr("upconf = %7.3f, leftconf = %7.3f, rotation = %d\n",
                    upconf, leftconf, rotation);
    regTestCompareValues(rp, upconf, 2.543, 0.1);  /* 0 */
    regTestCompareValues(rp, leftconf, 15.431, 0.1);  /* 1 */
    regTestCompareValues(rp, rotation, 90, 0.0);  /* 2 */
    regTestComparePix(rp, pixs, pix2);  /* 3 */
    pixDestroy(&pix1);
    pixDestroy(&pix2);

        /* Test orientation detection */
    pixa = pixaCreate(4);
    pix1 = pixCopy(NULL, pixs);
    lept_stderr("\nTest orient detection for 4 orientations\n");
    pixOrientDetect(pix1, &upconf, &leftconf, 0, 0);
    makeOrientDecision(upconf, leftconf, 0, 0, &orient, 1);
    regTestCompareValues(rp, upconf, 15.431, 0.1);  /* 4 */
    regTestCompareValues(rp, orient, 1, 0.0);  /* 5 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate90(pix1, 1);
    pix1 = pix2;
    pixOrientDetect(pix1, &upconf, &leftconf, 0, 0);
    makeOrientDecision(upconf, leftconf, 0, 0, &orient, 1);
    regTestCompareValues(rp, leftconf, -15.702, 0.1);  /* 6 */
    regTestCompareValues(rp, orient, 4, 0.0);  /* 7 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate90(pix1, 1);
    pix1 = pix2;
    pixOrientDetect(pix1, &upconf, &leftconf, 0, 0);
    makeOrientDecision(upconf, leftconf, 0, 0, &orient, 1);
    regTestCompareValues(rp, upconf, -15.702, 0.1);  /* 8 */
    regTestCompareValues(rp, orient, 3, 0.0);  /* 9 */
    pixaAddPix(pixa, pix1, L_INSERT);
    pix2 = pixRotate90(pix1, 1);
    pix1 = pix2;
    pixOrientDetect(pix1, &upconf, &leftconf, 0, 0);
    makeOrientDecision(upconf, leftconf, 0, 0, &orient, 1);
    regTestCompareValues(rp, leftconf, 15.431, 0.1);  /* 10 */
    regTestCompareValues(rp, orient, 2, 0.0);  /* 11 */
    pixaAddPix(pixa, pix1, L_INSERT);

    pix2 = pixaDisplayTiledInColumns(pixa, 2, 0.25, 20, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 12 */
    pixDisplayWithTitle(pix2, 100, 100, NULL, rp->display);
    pixDestroy(&pix2);
    pixaDestroy(&pixa);

    lept_stderr("\nTest mirror reverse detection\n");
    pixMirrorDetect(pixs, &conf, 0, rp->display);
    lept_stderr("conf = %5.3f; not mirror reversed\n", conf);
    regTestCompareValues(rp, conf, 4.128, 0.1);  /* 13 */

    pixDestroy(&pixs);
    return regTestCleanup(rp);
}
