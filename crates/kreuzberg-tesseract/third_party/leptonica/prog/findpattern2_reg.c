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
 * findpattern2_reg.c
 *
 *    This demonstrates three methods for generating hit-miss SELs from
 *    a 1 bpp image of a pattern.  Of the three, only the boundary
 *    method should be used.  The other methods are retained for comparison.
 *
 *    The SELs that are effective for each of the three methods are
 *    displayed.  For each method, one SEL is chosen and used to extract
 *    the "asterisk" patterns in the input image.
 *
 *    The removal of matched patterns by brute-force dilation is shown
 *    as a set of steps.  Not recommended because it is too expensive.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* for pixDisplayHitMissSel() */
static const l_uint32  HitColor = 0x33aa4400;
static const l_uint32  MissColor = 0xaa44bb00;

l_int32 DoPatternMatch(PIX *pixs, PIX *pixt, SEL *sel,
                       const char *fname, L_REGPARAMS *rp);


int main(int    argc,
         char **argv)
{
l_int32       i, cx, cy;
PIX          *pixs, *pixt, *pixsel, *pix1, *pix2, *pix3, *pix4;
PIXA         *pixa1;
SEL          *sel_ast, *sel1, *sel2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/pattern");

    pixs = pixRead("asterisk.png");
    pixt = pixRead("one-asterisk.png");

    /********************************************************************
     *  Generate hit-miss SELs that work effectively to identify        *
     *  the pattern in the image.  Do this for the three methods:       *
     *    - located a given distance from the boundary: this is         *
     *      the best method; always use it.                             *
     *    - located on a set of horizontal and vertical lines: this     *
     *      works over a range of parameters, but is less robust        *
     *    - random locations: the least reliable method                 *
     ********************************************************************/

         /* Boundary method is quite robust. With boundary distance of 2
          * for both hits and misses, hitskip and missskip can be
          * anything from 0 to 6. */
    pixa1 = pixaCreate(7);
    for (i = 0; i <= 6; i++) {
        sel_ast = pixGenerateSelBoundary(pixt, 2, 2, i, i, 1, 0, 1, 1, &pix1);
        pixsel = pixDisplayHitMissSel(pix1, sel_ast, 7, HitColor, MissColor);
        pixaAddPix(pixa1, pixsel, L_INSERT);
        pixDestroy(&pix1);
    }
    pix2 = pixaDisplayTiledInColumns(pixa1, 7, 1.0, 25, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 0 */
    pixDisplayWithTitle(pix2, 0, 700, NULL, rp->display);
    pixaDestroy(&pixa1);
    pixDestroy(&pix2);

         /* Run method is less robust.  With default min distance and
          * min runlength, the number of horizontal and vertical lines
          * can be chosen between 9 and 16. */
    pixa1 = pixaCreate(7);
    for (i = 9; i <= 16; i++) {
        sel_ast = pixGenerateSelWithRuns(pixt, i, i, 1, 3, 0, 0, 0, 0, &pix1);
        pixsel = pixDisplayHitMissSel(pix1, sel_ast, 7, HitColor, MissColor);
        pixaAddPix(pixa1, pixsel, L_INSERT);
        pixDestroy(&pix1);
    }
    pix2 = pixaDisplayTiledInColumns(pixa1, 8, 1.0, 25, 2);
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1 */
    pixDisplayWithTitle(pix2, 0, 850, NULL, rp->display);
    pixaDestroy(&pixa1);
    pixDestroy(&pix2);

          /* The random method is the least robust.  For this template,
           * the hit fraction must be near 0.15 and the miss fraction
           * near 0.10 */
    sel_ast = pixGenerateSelRandom(pixt, 0.15, 0.10, 1, 3, 0, 3, 3, &pix1);
    pixsel = pixDisplayHitMissSel(pix1, sel_ast, 7, HitColor, MissColor);
    regTestWritePixAndCheck(rp, pixsel, IFF_PNG);  /* 2 */
    pixDisplayWithTitle(pixsel, 0, 950, NULL, rp->display);
    pixDestroy(&pix1);
    pixDestroy(&pixsel);
    selDestroy(&sel_ast);

    /********************************************************************
     *  For each of the three methods, choose a workable sel and show   *
     *  the action on the input image.                                  *
     ********************************************************************/

    l_pdfSetDateAndVersion(0);  /* can't have date in a regression test */

        /* Choose some working skip distance for the boundary method */
    sel1 = pixGenerateSelBoundary(pixt, 2, 2, 5, 5, 1, 0, 1, 1, &pix1);
    DoPatternMatch(pixs, pix1, sel1, "/tmp/lept/pattern/match1.pdf",
                   rp);  /* 3 */
    lept_stderr("Boundary output written to /tmp/lept/pattern/match1.pdf\n");
    regTestCheckFile(rp, "/tmp/lept/pattern/match1.pdf");   /* 4 */
    selDestroy(&sel1);
    pixDestroy(&pix1);

        /* Choose some working number of horizontal and vertical lines for
         * the method of generating a HMT sel with runs */
    sel1 = pixGenerateSelWithRuns(pixt, 11, 11, 1, 3, 0, 0, 0, 0, &pix1);
    DoPatternMatch(pixs, pix1, sel1, "/tmp/lept/pattern/match2.pdf",
                   rp);  /* 5 */
    lept_stderr("Run output written to /tmp/lept/pattern/match2.pdf\n");
    regTestCheckFile(rp, "/tmp/lept/pattern/match2.pdf");   /* 6 */
    selDestroy(&sel1);
    pixDestroy(&pix1);

        /* Choose a working number for the hit and miss fractions. */
    sel1 = pixGenerateSelRandom(pixt, 0.15, 0.12, 1, 0, 0, 0, 0, &pix1);
    DoPatternMatch(pixs, pix1, sel1, "/tmp/lept/pattern/match3.pdf",
                   rp);  /* 7 */
    lept_stderr("Random output written to /tmp/lept/pattern/match3.pdf\n");
    regTestCheckFile(rp, "/tmp/lept/pattern/match3.pdf");   /* 8 */
    selDestroy(&sel1);
    pixDestroy(&pix1);

    /********************************************************************
     *  Brute-force method for removing all instances of a pattern:     *
     *  (1) Create a hit-miss SEL from an input pattern                 *
     *  (2) Do pixHMT() to find all locations that are matched          *
     *  (3) Dilate the result by the original input pattern             *
     *  (4) Dilate a little more to compensate for alignment issues     *
     *  (5) Subtract the dilated result from the input image            *
     *  Note: step (3) is very expensive for a pattern with many fg     *
     *  pixels.  If nfg is the number of fg pixels in the template,     *
     *  this dilation is doing nfg rasterops of the input image!        *
     *  To remove matching pixels efficiently, use                      *
     *       pixRemoveMatchedPattern()]                                 *
     ********************************************************************/

    sel1 = pixGenerateSelBoundary(pixt, 2, 2, 5, 5, 1, 0, 1, 1, &pix1);
    pix2 = pixHMT(NULL, pixs, sel1);
    selGetParameters(sel1, NULL, NULL, &cy, &cx);
    sel2 = selCreateFromPix(pix1, cy, cx, NULL);
    pix3 = pixDilate(NULL, pix2, sel2);
    pixDilateBrick(pix3, pix3, 4, 4);
    regTestWritePixAndCheck(rp, pix3, IFF_PNG);  /* 9 */
    pixDisplayWithTitle(pix3, 850, 450, NULL, rp->display);
    pix4 = pixSubtract(NULL, pixs, pix3);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 10 */
    pixDisplayWithTitle(pix4, 1150, 850, NULL, rp->display);
    selDestroy(&sel1);
    selDestroy(&sel2);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);

    return regTestCleanup(rp);
}


l_int32
DoPatternMatch(PIX          *pixs,
               PIX          *pixt,
               SEL          *sel,
               const char   *fname,
               L_REGPARAMS  *rp)
{
l_int32  cx, cy;
PIX     *pixsel, *pixt1, *pix1, *pix2, *pix3, *pix4;
PIXA    *pixa1;

    pixa1 = pixaCreate(7);
    pixaAddPix(pixa1, pixs, L_COPY);
    pixt1 = pixScale(pixt, 8, 8);
    pixaAddPix(pixa1, pixt1, L_INSERT);
    pixsel = pixDisplayHitMissSel(pixt, sel, 7, HitColor, MissColor);
    pixaAddPix(pixa1, pixsel, L_INSERT);

        /* Perform the HMT and show patterns that were matched */
    pix1 = pixHMT(NULL, pixs, sel);
    pixaAddPix(pixa1, pix1, L_INSERT);
    selGetParameters(sel, NULL, NULL, &cy, &cx);
    pix2 = pixDisplayMatchedPattern(pixs, pixt, pix1, cx, cy, 0x0000ff00,
                                    1.0, 5);
    pixaAddPix(pixa1, pix2, L_INSERT);

        /* Remove the matched patterns */
    pix3 = pixCopy(NULL, pixs);
    pixRemoveMatchedPattern(pix3, pixt, pix1, cx, cy, 2);
    pixaAddPix(pixa1, pix3, L_INSERT);

        /* Generate outputs */
    pix4 = pixaDisplayTiledInColumns(pixa1, 7, 1.0, 15, 2);
    pixaAddPix(pixa1, pix4, L_INSERT);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);
    pixaConvertToPdf(pixa1, 100, 1.0, L_FLATE_ENCODE, 50, NULL, fname);
    pixaDestroy(&pixa1);
    return 0;
}

