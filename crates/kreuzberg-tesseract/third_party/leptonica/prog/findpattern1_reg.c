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
 * findpattern1_reg.c
 *
 *    This uses pixGenerateSelBoundary() to generate hit-miss Sels
 *    that are a good fit for two 1 bpp patterns:
 *       * a "T" in the banner name
 *       * the banner name ("Tribune")
 *    The sels are first displayed, showing the hit and miss elements
 *    in color.
 *
 *    The sels are then used to identify and remove the components
 *    in a page image in which they are found.  We demonstrate
 *    the ability to find these components are reductions from 4 to 16x.
 *    (16x is extreme -- don't do this at home!)  The results are displayed
 *    with the matched pattern either highlighted or removed.
 *
 *    Some of these Sels are also made by livre_hmt.c for figures
 *    in the Document Image Applications chapter.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* for pixDisplayHitMissSel() */
static const l_uint32  HitColor = 0x33aa4400;
static const l_uint32  MissColor = 0xaa44bb00;

    /* Patterns at full resolution */
static const char *patname[2] = {
    "tribune-word.png",   /* patno = 0 */
    "tribune-t.png"};     /* patno = 1 */

l_int32 GeneratePattern(l_int32 patno, l_int32 red, L_REGPARAMS  *rp);


int main(int    argc,
         char **argv)
{
l_int32      patno, red;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    for (patno = 0; patno < 2; patno++) {
        for (red = 4; red <= 16; red *= 2) {
            if (patno == 1 && red == 16) continue;
            GeneratePattern(patno, red, rp);
        }
    }

    return regTestCleanup(rp);
}


l_int32
GeneratePattern(l_int32       patno,
                l_int32       red,
                L_REGPARAMS  *rp)
{
l_int32  width, cx, cy;
PIX     *pixs, *pixt, *pix, *pixr, *pixp, *pixsel, *pixhmt;
PIX     *pixc1, *pixc2, *pixc3, *pixd;
PIXA    *pixa;
SEL     *selhm;

    if ((pixs = pixRead(patname[patno])) == NULL) {
        rp->success = FALSE;
        return ERROR_INT("pixs not made", __func__, 1);
    }

        /* Make a hit-miss sel at specified reduction factor */
    if (red == 4) {
        pixt = pixReduceRankBinaryCascade(pixs, 4, 4, 0, 0);
        selhm = pixGenerateSelBoundary(pixt, 2, 2, 20, 30, 1, 1, 0, 0, &pixp);
    }
    else if (red == 8) {
        pixt = pixReduceRankBinaryCascade(pixs, 4, 4, 2, 0);
        selhm = pixGenerateSelBoundary(pixt, 1, 2, 6, 12, 1, 1, 0, 0, &pixp);
    }
    else {  /*  red == 16 */
        pixt = pixReduceRankBinaryCascade(pixs, 4, 4, 2, 2);
        selhm = pixGenerateSelBoundary(pixt, 1, 1, 4, 8, 0, 0, 0, 0, &pixp);
    }
    pixDestroy(&pixt);

        /* Display the sel */
    pixsel = pixDisplayHitMissSel(pixp, selhm, 7, HitColor, MissColor);
    pixa = pixaCreate(2);
    pixaAddPix(pixa, pixs, L_CLONE);
    pixaAddPix(pixa, pixsel, L_CLONE);
    width = (patno == 0) ? 1200 : 400;
    pixd = pixaDisplayTiledAndScaled(pixa, 32, width, 2, 0, 30, 2);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);
    pixDisplayWithTitle(pixd, 100, 100 + 100 * (3 * patno + red / 4),
                        NULL, rp->display);
    pixaDestroy(&pixa);
    pixDestroy(&pixd);

        /* Use the sel to find all instances in the page */
    pix = pixRead("tribune-page-4x.png");  /* 4x reduced */
    if (red == 4)
        pixr = pixClone(pix);
    else if (red == 8)
        pixr = pixReduceRankBinaryCascade(pix, 2, 0, 0, 0);
    else  /* red == 16 */
        pixr = pixReduceRankBinaryCascade(pix, 2, 2, 0, 0);
    pixDestroy(&pix);

    startTimer();
    pixhmt = pixHMT(NULL, pixr, selhm);
    lept_stderr("Time to find patterns = %7.3f\n", stopTimer());

        /* Color each instance at full res */
    selGetParameters(selhm, NULL, NULL, &cy, &cx);
    pixc1 = pixDisplayMatchedPattern(pixr, pixp, pixhmt,
                                     cx, cy, 0x0000ff00, 1.0, 5);
    regTestWritePixAndCheck(rp, pixc1, IFF_PNG);
    pixDisplayWithTitle(pixc1, 500, 100, NULL, rp->display);

        /* Color each instance at 0.5 scale */
    pixc2 = pixDisplayMatchedPattern(pixr, pixp, pixhmt,
                                     cx, cy, 0x0000ff00, 0.5, 5);
    regTestWritePixAndCheck(rp, pixc2, IFF_PNG);

        /* Remove each instance from the input image */
    pixc3 = pixCopy(NULL, pixr);
    pixRemoveMatchedPattern(pixc3, pixp, pixhmt, cx, cy, 1);
    regTestWritePixAndCheck(rp, pixc3, IFF_PNG);

    selDestroy(&selhm);
    pixDestroy(&pixp);
    pixDestroy(&pixsel);
    pixDestroy(&pixhmt);
    pixDestroy(&pixc1);
    pixDestroy(&pixc2);
    pixDestroy(&pixc3);
    pixDestroy(&pixd);
    pixDestroy(&pixr);
    pixDestroy(&pixs);
    return 0;
}
