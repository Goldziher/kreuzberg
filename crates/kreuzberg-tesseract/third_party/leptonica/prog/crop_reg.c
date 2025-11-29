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
 *  crop_reg.c
 *
 *   Test:
 *   * plotting pixel profiles
 *   * undercropping from a box (i.e., with an added border)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32  mindif = 60;

static l_int32 GetLeftCut(NUMA *narl, NUMA *nart, NUMA *nait,
                   l_int32 h, l_int32 *pleft);
static l_int32 GetRightCut(NUMA *narl, NUMA *nart, NUMA *nait,
                     l_int32 h, l_int32 *pright);

const char *fnames[] = {"lyra.005.jpg", "lyra.036.jpg"};

int main(int    argc,
         char **argv)
{
l_int32       i, pageno, w, h, left, right;
BOX          *box1, *box2;
NUMA         *na1, *nar, *naro, *narl, *nart, *nai, *naio, *nait;
PIX          *pixs, *pixr, *pixg, *pixgi, *pixd, *pix1, *pix2, *pix3, *pix4;
PIXA         *pixa1, *pixa2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/crop");

        /* Calculate projection profiles through images/drawings. */
    pixa1 = pixaCreate(2);
    for (i = 0; i < 2; i++) {
        pageno = extractNumberFromFilename(fnames[i], 5, 0);
        lept_stderr("Page %d\n", pageno);
        pixs = pixRead(fnames[i]);
        pixr = pixRotate90(pixs, (pageno % 2) ? 1 : -1);
        pixg = pixConvertTo8(pixr, 0);
        pixGetDimensions(pixg, &w, &h, NULL);

            /* Get info on vertical reversal profile */
        nar = pixReversalProfile(pixg, 0.8, L_VERTICAL_LINE,
                                 0, h - 1, mindif, 1, 1);
        naro = numaOpen(nar, 11);
        pix1 = gplotSimplePix1(naro, "Reversals Opened");
        regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 0,2 */
        narl = numaLowPassIntervals(naro, 0.1, 0.0);
        nart = numaThresholdEdges(naro, 0.1, 0.5, 0.0);
        numaDestroy(&nar);
        numaDestroy(&naro);

            /* Get info on vertical intensity profile */
        pixgi = pixInvert(NULL, pixg);
        nai = pixAverageIntensityProfile(pixgi, 0.8, L_VERTICAL_LINE,
                                         0, h - 1, 1, 1);
        naio = numaOpen(nai, 11);
        pix2 = gplotSimplePix1(naio, "Intensities Opened");
        regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 1,3 */
        nait = numaThresholdEdges(naio, 0.4, 0.6, 0.0);
        numaDestroy(&nai);
        numaDestroy(&naio);

            /* Analyze profiles for left/right edges  */
        GetLeftCut(narl, nart, nait, w, &left);
        GetRightCut(narl, nart, nait, w, &right);
        if (rp->display)
            lept_stderr("left = %d, right = %d\n", left, right);

            /* Output visuals */
        pixa2 = pixaCreate(3);
        pixaAddPix(pixa2, pixr, L_INSERT);
        pixaAddPix(pixa2, pix1, L_INSERT);
        pixaAddPix(pixa2, pix2, L_INSERT);
        pixd = pixaDisplayTiledInColumns(pixa2, 2, 1.0, 25, 0);
        pixaDestroy(&pixa2);
        pixaAddPix(pixa1, pixd, L_INSERT);
        pixDisplayWithTitle(pixd, 800 * i, 100, NULL, rp->display);
        if (rp->display) {
            lept_stderr("narl:");
            numaWriteStderr(narl);
            lept_stderr("nart:");
            numaWriteStderr(nart);
            lept_stderr("nait:");
            numaWriteStderr(nait);
        }
        pixDestroy(&pixs);
        pixDestroy(&pixg);
        pixDestroy(&pixgi);
        numaDestroy(&narl);
        numaDestroy(&nart);
        numaDestroy(&nait);
    }

    lept_stderr("Writing profiles to /tmp/lept/crop/croptest.pdf\n");
    pixaConvertToPdf(pixa1, 75, 1.0, L_JPEG_ENCODE, 0, "Profiles",
                     "/tmp/lept/crop/croptest.pdf");
    pixaDestroy(&pixa1);

        /* Calculate projection profiles from text lines */
    pixs = pixRead("1555.007.jpg");
    pixGetDimensions(pixs, &w, &h, NULL);
    na1 = pixReversalProfile(pixs, 0.98, L_HORIZONTAL_LINE,
                                  0, h - 1, 40, 3, 3);
    pix1 = gplotSimplePix1(na1, "Reversals");
    numaDestroy(&na1);

    na1 = pixAverageIntensityProfile(pixs, 0.98, L_HORIZONTAL_LINE,
                                    0, h - 1, 1, 1);
    pix2 = gplotSimplePix1(na1, "Intensities");
    regTestWritePixAndCheck(rp, pix1, IFF_PNG);  /* 4 */
    regTestWritePixAndCheck(rp, pix2, IFF_PNG);  /* 5 */
    numaDestroy(&na1);
    pixa1 = pixaCreate(3);
    pixaAddPix(pixa1, pixScale(pixs, 0.5, 0.5), L_INSERT);
    pixaAddPix(pixa1, pix1, L_INSERT);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixd = pixaDisplayTiledInRows(pixa1, 32, 1000, 1.0, 0, 30, 2);
    pixWrite("/tmp/lept/crop/profiles.png", pixd, IFF_PNG);
    pixDisplayWithTitle(pixd, 0, 700, NULL, rp->display);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
    pixaDestroy(&pixa1);

        /* Test rectangle clipping with border */
    pix1 = pixRead("lyra.005.jpg");
    pix2 = pixScale(pix1, 0.5, 0.5);
    box1 = boxCreate(125, 50, 180, 230);  /* fully contained */
    pix3 = pixClipRectangleWithBorder(pix2, box1, 30, &box2);
    pixRenderBoxArb(pix2, box1, 2, 255, 0, 0);
    pixRenderBoxArb(pix3, box2, 2, 255, 0, 0);
    pixa1 = pixaCreate(2);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixaAddPix(pixa1, pix3, L_INSERT);
    pix4 = pixaDisplayTiledInColumns(pixa1, 2, 1.0, 15, 2);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 6 */
    pixDisplayWithTitle(pix4, 325, 700, NULL, rp->display);
    boxDestroy(&box1);
    boxDestroy(&box2);
    pixDestroy(&pix4);
    pixaDestroy(&pixa1);

    pix2 = pixScale(pix1, 0.5, 0.5);
    box1 = boxCreate(125, 10, 180, 270);  /* not full border */
    pix3 = pixClipRectangleWithBorder(pix2, box1, 30, &box2);
    pixRenderBoxArb(pix2, box1, 2, 255, 0, 0);
    pixRenderBoxArb(pix3, box2, 2, 255, 0, 0);
    pixa1 = pixaCreate(2);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixaAddPix(pixa1, pix3, L_INSERT);
    pix4 = pixaDisplayTiledInColumns(pixa1, 2, 1.0, 15, 2);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 7 */
    pixDisplayWithTitle(pix4, 975, 700, NULL, rp->display);
    boxDestroy(&box1);
    boxDestroy(&box2);
    pixDestroy(&pix4);
    pixaDestroy(&pixa1);

    pix2 = pixScale(pix1, 0.5, 0.5);
    box1 = boxCreate(125, 200, 180, 270);  /* not entirely within pix2 */
    pix3 = pixClipRectangleWithBorder(pix2, box1, 30, &box2);
    pixRenderBoxArb(pix2, box1, 2, 255, 0, 0);
    pixRenderBoxArb(pix3, box2, 2, 255, 0, 0);
    pixa1 = pixaCreate(2);
    pixaAddPix(pixa1, pix2, L_INSERT);
    pixaAddPix(pixa1, pix3, L_INSERT);
    pix4 = pixaDisplayTiledInColumns(pixa1, 2, 1.0, 15, 2);
    regTestWritePixAndCheck(rp, pix4, IFF_PNG);  /* 8 */
    pixDisplayWithTitle(pix4, 1600, 700, NULL, rp->display);
    boxDestroy(&box1);
    boxDestroy(&box2);
    pixDestroy(&pix4);
    pixaDestroy(&pixa1);
    pixDestroy(&pix1);

    return regTestCleanup(rp);
}


/*
 * Use these variable abbreviations:
 *
 * pap1: distance from left edge to the page
 * txt1: distance from left edge to the text
 * Identify pap1 by (a) 1st downward transition in intensity (nait).
 *                  (b) start of 1st lowpass interval (nail)
 * Identify txt1 by (a) end of 1st lowpass interval (nail)
 *                  (b) first upward transition in reversals (nart)
 *
 * pap2: distance from right edge to beginning of last upward transition,
 *       plus some extra for safety.
 * txt1: distance from right edge to the text
 * Identify pap2 by 1st downward transition in intensity.
 * Identify txt2 by (a) beginning of 1st lowpass interval from bottom
 *                  (b) last downward transition in reversals from bottom
 */
static l_int32
GetLeftCut(NUMA *narl,
           NUMA *nart,
           NUMA *nait,
           l_int32 w,
           l_int32 *pleft) {
l_int32  nrl, nrt, nit, start, end, sign, pap1, txt1, del;

    nrl = numaGetCount(narl);
    nrt = numaGetCount(nart);
    nit = numaGetCount(nait);

        /* Check for small max number of reversals or no edge */
    numaGetSpanValues(narl, 0, NULL, &end);
    if (end < 20 || nrl <= 1) {
        *pleft = 0;
        return 0;
    }

        /* Where is text and page, scanning from the left? */
    pap1 = 0;
    txt1 = 0;
    if (nrt >= 4) {  /* beginning of first upward transition */
        numaGetEdgeValues(nart, 0, &start, NULL, NULL);
        txt1 = start;
    }
    if (nit >= 4) {  /* end of first downward trans in (inverse) intensity */
        numaGetEdgeValues(nait, 0, NULL, &end, &sign);
        if (end < txt1 && sign == -1)
            pap1 = end;
        else
            pap1 = 0.5 * txt1;
    }
    del = txt1 - pap1;
    if (del > 20) {
        txt1 -= L_MIN(20, 0.5 * del);
        pap1 += L_MIN(20, 0.5 * del);
    }
    lept_stderr("txt1 = %d, pap1 = %d\n", txt1, pap1);
    *pleft = pap1;
    return 0;
}


static l_int32
GetRightCut(NUMA *narl,
            NUMA *nart,
            NUMA *nait,
            l_int32 w,
            l_int32 *pright) {
l_int32  nrt, ntrans, start, end, sign, txt2, pap2, found, trans;

    nrt = numaGetCount(nart);

        /* Check for small max number of reversals or no edge */
        /* Where is text and page, scanning from the right?  */
    ntrans = nrt / 3;
    if (ntrans > 1) {
        found = FALSE;
        for (trans = ntrans - 1; trans > 0; --trans) {
            numaGetEdgeValues(nart, trans, &start, &end, &sign);
            if (sign == -1) {  /* end of textblock */
                txt2 = end;
                found = TRUE;
            }
        }
        if (!found) {
            txt2 = w - 1;  /* take the whole thing! */
            pap2 = w - 1;
        } else {  /* found textblock; now find right side of page */
            found = FALSE;
            for (trans = ntrans - 1; trans > 0; --trans) {
                numaGetEdgeValues(nart, trans, &start, &end, &sign);
                if (sign == 1 && start > txt2) {
                    pap2 = start;  /* start of textblock on other page */
                    found = TRUE;
                }
            }
            if (!found) {  /* no text from other page */
                pap2 = w - 1;  /* refine later */
            }
        }
    } else {
        txt2 = w - 1;
        pap2 = w - 1;
    }
    lept_stderr("txt2 = %d, pap2 = %d\n", txt2, pap2);
    *pright = pap2;
    return 0;
}

