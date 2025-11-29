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
 * livre_hmt.c
 *
 *    This demonstrates use of pixGenerateSelBoundary() to
 *    generate a hit-miss Sel.
 *
 *    (1) The Sel is displayed with the hit and miss elements in color.
 *
 *    (2) We produce several 4 bpp colormapped renditions,
 *        with the matched pattern either highlighted or removed.
 *
 *    (3) For figures in the Document Image Applications chapter:
 *           fig 7:  livre_hmt 1 8
 *           fig 8:  livre_hmt 2 4
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* for pixDisplayHitMissSel() */
static const l_uint32  HitColor = 0x33aa4400;
static const l_uint32  MissColor = 0xaa44bb00;

    /* Patterns at full resolution */
static const char *patname[3] = {
    "",
    "tribune-word.png",   /* patno = 1 */
    "tribune-t.png"};     /* patno = 2 */


int main(int    argc,
         char **argv)
{
l_int32  patno, reduction, width, cols, cx, cy;
PIX     *pixs, *pixt, *pix, *pixr, *pixp, *pixsel, *pixhmt;
PIX     *pixd1, *pixd2, *pixd3, *pixd;
PIXA    *pixa;
SEL     *selhm;

    if (argc != 3)
        return ERROR_INT(" Syntax:  livre_hmt pattern reduction", __func__, 1);
    patno = atoi(argv[1]);
    reduction = atoi(argv[2]);

    setLeptDebugOK(1);
    lept_mkdir("lept/livre");
    if ((pixs = pixRead(patname[patno])) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    if (reduction != 4 && reduction != 8 && reduction != 16)
        return ERROR_INT("reduction not 4, 8 or 16", __func__, 1);

    if (reduction == 4)
        pixt = pixReduceRankBinaryCascade(pixs, 4, 4, 0, 0);
    else if (reduction == 8)
        pixt = pixReduceRankBinaryCascade(pixs, 4, 4, 2, 0);
    else  /* reduction == 16 */
        pixt = pixReduceRankBinaryCascade(pixs, 4, 4, 2, 2);

        /* Make a hit-miss sel */
    if (reduction == 4)
        selhm = pixGenerateSelBoundary(pixt, 2, 2, 20, 30, 1, 1, 0, 0, &pixp);
    else if (reduction == 8)
        selhm = pixGenerateSelBoundary(pixt, 1, 2, 6, 12, 1, 1, 0, 0, &pixp);
    else  /* reduction == 16 */
        selhm = pixGenerateSelBoundary(pixt, 1, 1, 4, 8, 0, 0, 0, 0, &pixp);

        /* Display the sel */
    pixsel = pixDisplayHitMissSel(pixp, selhm, 7, HitColor, MissColor);
    pixDisplay(pixsel, 1000, 300);
    pixWrite("/tmp/lept/livre/pixsel1", pixsel, IFF_PNG);

        /* Use the Sel to find all instances in the page */
    pix = pixRead("tribune-page-4x.png");  /* 4x reduced */
    if (reduction == 4)
        pixr = pixClone(pix);
    else if (reduction == 8)
        pixr = pixReduceRankBinaryCascade(pix, 2, 0, 0, 0);
    else  /* reduction == 16 */
        pixr = pixReduceRankBinaryCascade(pix, 2, 2, 0, 0);

    startTimer();
    pixhmt = pixHMT(NULL, pixr, selhm);
    lept_stderr("Time to find patterns = %7.3f\n", stopTimer());

        /* Color each instance at full res */
    selGetParameters(selhm, NULL, NULL, &cy, &cx);
    pixd1 = pixDisplayMatchedPattern(pixr, pixp, pixhmt,
                                     cx, cy, 0x0000ff00, 1.0, 5);
    pixWrite("/tmp/lept/livre/pixd11", pixd1, IFF_PNG);

        /* Color each instance at 0.5 scale */
    pixd2 = pixDisplayMatchedPattern(pixr, pixp, pixhmt,
                                     cx, cy, 0x0000ff00, 0.5, 5);
    pixWrite("/tmp/lept/livre/pixd12", pixd2, IFF_PNG);

        /* Remove each instance from the input image */
    pixd3 = pixCopy(NULL, pixr);
    pixRemoveMatchedPattern(pixd3, pixp, pixhmt, cx, cy, 1);
    pixWrite("/tmp/lept/livre/pixr1", pixd3, IFF_PNG);

    pixa = pixaCreate(2);
    pixaAddPix(pixa, pixs, L_CLONE);
    pixaAddPix(pixa, pixsel, L_CLONE);
    cols = (patno == 1) ? 1 : 2;
    width = (patno == 1) ? 800 : 400;
    pixd = pixaDisplayTiledAndScaled(pixa, 32, width, cols, 0, 30, 2);
    pixWrite("/tmp/lept/livre/hmt.png", pixd, IFF_PNG);
    pixDisplay(pixd, 1000, 600);

    selDestroy(&selhm);
    pixDestroy(&pixp);
    pixDestroy(&pixsel);
    pixDestroy(&pixhmt);
    pixDestroy(&pixd1);
    pixDestroy(&pixd2);
    pixDestroy(&pixd3);
    pixDestroy(&pixs);
    pixDestroy(&pix);
    pixDestroy(&pixt);
    return 0;
}
