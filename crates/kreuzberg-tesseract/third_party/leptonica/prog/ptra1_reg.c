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
 * ptra1_reg.c
 *
 *    Testing basic ptra operations
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static void MakePtrasFromPixa(PIXA *pixa, L_PTRA **ppapix, L_PTRA **ppabox,
                              l_int32 copyflag);
static PIXA *ReconstructPixa1(L_REGPARAMS *rp, L_PTRA *papix, L_PTRA *pabox);
static PIXA *ReconstructPixa2(L_REGPARAMS *rp, L_PTRA *papix, L_PTRA *pabox);
static PIX *SaveResult(PIXA *pixac, PIXA **ppixa, l_int32 w, l_int32 h,
                       l_int32 newline);

int main(int    argc,
         char **argv)
{
l_int32       i, n, w, h, nactual, imax;
BOX          *box;
BOXA         *boxa;
PIX          *pixs, *pixd, *pix;
PIXA         *pixas, *pixa1, *pixa2, *pixac1, *pixac2;
L_PTRA       *papix, *pabox, *papix2, *pabox2;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    pixac1 = pixaCreate(0);
    pixac2 = pixaCreate(0);

    pixs = pixRead("lucasta.1.300.tif");
    pixGetDimensions(pixs, &w, &h, NULL);
    boxa = pixConnComp(pixs, &pixas, 8);
    pixDestroy(&pixs);
    boxaDestroy(&boxa);
    n = pixaGetCount(pixas);

        /* Fill ptras with clones and reconstruct */
    if (rp->display)
        lept_stderr("Fill with clones and reconstruct\n");
    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    pixa1 = ReconstructPixa1(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    pixa2 = ReconstructPixa2(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 0 */
    pixDestroy(&pixd);
    pixd = SaveResult(pixac2, &pixa2, w, h, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 1 */
    pixDestroy(&pixd);

        /* Remove every other one for the first half;
         * with compaction at each removal */
    if (rp->display)
        lept_stderr("Remove every other in 1st half, with compaction\n");
    MakePtrasFromPixa(pixas, &papix, &pabox, L_COPY);
    for (i = 0; i < n / 2; i++) {
        if (i % 2 == 0) {
            pix = (PIX *)ptraRemove(papix, i, L_COMPACTION);
            box = (BOX *)ptraRemove(pabox, i, L_COMPACTION);
            pixDestroy(&pix);
            boxDestroy(&box);
        }
    }
    pixa1 = ReconstructPixa1(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 2 */
    pixDestroy(&pixd);

    MakePtrasFromPixa(pixas, &papix, &pabox, L_COPY);
    for (i = 0; i < n / 2; i++) {
        if (i % 2 == 0) {
            pix = (PIX *)ptraRemove(papix, i, L_COMPACTION);
            box = (BOX *)ptraRemove(pabox, i, L_COMPACTION);
            pixDestroy(&pix);
            boxDestroy(&box);
        }
    }
    pixa2 = ReconstructPixa2(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac2, &pixa2, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 3 */
    pixDestroy(&pixd);

        /* Remove every other one for the entire set,
         * but without compaction at each removal */
    if (rp->display)
        lept_stderr("Remove every other in 1st half, "
                    "without & then with compaction\n");
    MakePtrasFromPixa(pixas, &papix, &pabox, L_COPY);
    for (i = 0; i < n; i++) {
        if (i % 2 == 0) {
            pix = (PIX *)ptraRemove(papix, i, L_NO_COMPACTION);
            box = (BOX *)ptraRemove(pabox, i, L_NO_COMPACTION);
            pixDestroy(&pix);
            boxDestroy(&box);
        }
    }
    ptraCompactArray(papix);  /* now do the compaction */
    ptraCompactArray(pabox);
    pixa1 = ReconstructPixa1(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 4 */
    pixDestroy(&pixd);

    MakePtrasFromPixa(pixas, &papix, &pabox, L_COPY);
    for (i = 0; i < n; i++) {
        if (i % 2 == 0) {
            pix = (PIX *)ptraRemove(papix, i, L_NO_COMPACTION);
            box = (BOX *)ptraRemove(pabox, i, L_NO_COMPACTION);
            pixDestroy(&pix);
            boxDestroy(&box);
        }
    }
    ptraCompactArray(papix);  /* now do the compaction */
    ptraCompactArray(pabox);
    pixa2 = ReconstructPixa2(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac2, &pixa2, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 5 */
    pixDestroy(&pixd);

        /* Fill ptras using insert at head, and reconstruct */
    if (rp->display)
        lept_stderr("Insert at head and reconstruct\n");
    papix = ptraCreate(n);
    pabox = ptraCreate(n);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixas, i, L_CLONE);
        box = pixaGetBox(pixas, i, L_CLONE);
        ptraInsert(papix, 0, pix, L_MIN_DOWNSHIFT);
        ptraInsert(pabox, 0, box, L_FULL_DOWNSHIFT);
    }
    pixa1 = ReconstructPixa1(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 6 */
    pixDestroy(&pixd);

    papix = ptraCreate(n);
    pabox = ptraCreate(n);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixas, i, L_CLONE);
        box = pixaGetBox(pixas, i, L_CLONE);
        ptraInsert(papix, 0, pix, L_MIN_DOWNSHIFT);
        ptraInsert(pabox, 0, box, L_FULL_DOWNSHIFT);
    }
    pixa2 = ReconstructPixa2(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac2, &pixa2, w, h, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 7 */
    pixDestroy(&pixd);

        /* Reverse the arrays by swapping */
    if (rp->display)
        lept_stderr("Reverse by swapping\n");
    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    for (i = 0; i < n / 2; i++) {
        ptraSwap(papix, i, n - i - 1);
        ptraSwap(pabox, i, n - i - 1);
    }
    ptraCompactArray(papix);  /* already compact; shouldn't do anything */
    ptraCompactArray(pabox);
    pixa1 = ReconstructPixa1(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 8 */
    pixDestroy(&pixd);

    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    for (i = 0; i < n / 2; i++) {
        ptraSwap(papix, i, n - i - 1);
        ptraSwap(pabox, i, n - i - 1);
    }
    ptraCompactArray(papix);  /* already compact; shouldn't do anything */
    ptraCompactArray(pabox);
    pixa2 = ReconstructPixa2(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac2, &pixa2, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 9 */
    pixDestroy(&pixd);

        /* Remove at the top of the array and push the hole to the end
         * by neighbor swapping (!).  This is O(n^2), so it's not a
         * recommended way to copy a ptra. [joke]  */
    if (rp->display)
        lept_stderr(
                "Remove at top, pushing hole to end by swapping -- O(n^2)\n");
    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    papix2 = ptraCreate(0);
    pabox2 = ptraCreate(0);
    while (1) {
        ptraGetActualCount(papix, &nactual);
        if (nactual == 0) break;
        ptraGetMaxIndex(papix, &imax);
        pix = (PIX *)ptraRemove(papix, 0, L_NO_COMPACTION);
        box = (BOX *)ptraRemove(pabox, 0, L_NO_COMPACTION);
        ptraAdd(papix2, pix);
        ptraAdd(pabox2, box);
        for (i = 1; i <= imax; i++) {
           ptraSwap(papix, i - 1, i);
           ptraSwap(pabox, i - 1, i);
        }
    }
    ptraCompactArray(papix);  /* should be empty */
    ptraCompactArray(pabox);  /* ditto */
    pixa1 = ReconstructPixa1(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 1);  /* nothing there */
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 10 */
    pixDestroy(&pixd);

    pixa1 = ReconstructPixa1(rp, papix2, pabox2);
    ptraDestroy(&papix2, 0, 1);
    ptraDestroy(&pabox2, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 11 */
    pixDestroy(&pixd);

        /* Remove and insert one position above, allowing minimum downshift.
         * If you specify L_AUTO_DOWNSHIFT, because there is only 1 hole,
         * it will do a full downshift at each insert.  This is a
         * situation where the heuristic (expected number of holes)
         * fails to do the optimal thing. */
    if (rp->display)
        lept_stderr("Remove and insert one position above (min downshift)\n");
    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    for (i = 1; i < n; i++) {
        pix = (PIX *)ptraRemove(papix, i, L_NO_COMPACTION);
        box = (BOX *)ptraRemove(pabox, i, L_NO_COMPACTION);
        ptraInsert(papix, i - 1, pix, L_MIN_DOWNSHIFT);
        ptraInsert(pabox, i - 1, box, L_MIN_DOWNSHIFT);
    }
    pixa1 = ReconstructPixa1(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 12 */
    pixDestroy(&pixd);

    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    for (i = 1; i < n; i++) {
        pix = (PIX *)ptraRemove(papix, i, L_NO_COMPACTION);
        box = (BOX *)ptraRemove(pabox, i, L_NO_COMPACTION);
        ptraInsert(papix, i - 1, pix, L_MIN_DOWNSHIFT);
        ptraInsert(pabox, i - 1, box, L_MIN_DOWNSHIFT);
    }
    pixa2 = ReconstructPixa2(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac2, &pixa2, w, h, 1);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 13 */
    pixDestroy(&pixd);

        /* Remove and insert one position above, but this time
         * forcing a full downshift at each step.  */
    if (rp->display)
        lept_stderr("Remove and insert one position above (full downshift)\n");
    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    for (i = 1; i < n; i++) {
        pix = (PIX *)ptraRemove(papix, i, L_NO_COMPACTION);
        box = (BOX *)ptraRemove(pabox, i, L_NO_COMPACTION);
        ptraInsert(papix, i - 1, pix, L_AUTO_DOWNSHIFT);
        ptraInsert(pabox, i - 1, box, L_AUTO_DOWNSHIFT);
    }
    pixa1 = ReconstructPixa1(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac1, &pixa1, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 14 */
    pixDestroy(&pixd);

    MakePtrasFromPixa(pixas, &papix, &pabox, L_CLONE);
    for (i = 1; i < n; i++) {
        pix = (PIX *)ptraRemove(papix, i, L_NO_COMPACTION);
        box = (BOX *)ptraRemove(pabox, i, L_NO_COMPACTION);
        ptraInsert(papix, i - 1, pix, L_AUTO_DOWNSHIFT);
        ptraInsert(pabox, i - 1, box, L_AUTO_DOWNSHIFT);
    }
    pixa2 = ReconstructPixa2(rp, papix, pabox);
    ptraDestroy(&papix, 0, 1);
    ptraDestroy(&pabox, 0, 1);
    pixd = SaveResult(pixac2, &pixa2, w, h, 0);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);   /* 15 */
    pixDestroy(&pixd);

    pixd = pixaDisplayTiledInColumns(pixac1, 10, 0.5, 15, 2);
    pixDisplayWithTitle(pixd, 0, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 16 */
    pixDestroy(&pixd);
    pixd = pixaDisplayTiledInColumns(pixac2, 10, 0.5, 15, 2);
    pixDisplayWithTitle(pixd, 800, 100, NULL, rp->display);
    regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 17 */
    pixDestroy(&pixd);
    pixaDestroy(&pixac1);
    pixaDestroy(&pixac2);
    pixaDestroy(&pixas);

    return regTestCleanup(rp);
}


static void
MakePtrasFromPixa(PIXA     *pixa,
                  L_PTRA  **ppapix,
                  L_PTRA  **ppabox,
                  l_int32   copyflag)
{
l_int32  i, n;
BOX     *box;
PIX     *pix;
L_PTRA  *papix, *pabox;

    n = pixaGetCount(pixa);
    papix = ptraCreate(n);
    pabox = ptraCreate(n);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixa, i, copyflag);
        box = pixaGetBox(pixa, i, copyflag);
        ptraAdd(papix, pix);
        ptraAdd(pabox, box);
    }

    *ppapix = papix;
    *ppabox = pabox;
    return;
}


    /* Reconstruction without compaction */
static PIXA *
ReconstructPixa1(L_REGPARAMS  *rp,
                 L_PTRA       *papix,
                 L_PTRA       *pabox)
{
l_int32  i, imax, nactual;
BOX     *box;
PIX     *pix;
PIXA    *pixat;

    ptraGetMaxIndex(papix, &imax);
    ptraGetActualCount(papix, &nactual);
    if (rp->display)
        lept_stderr("Before removal:  imax = %4d, actual = %4d\n",
                    imax, nactual);

    pixat = pixaCreate(imax + 1);
    for (i = 0; i <= imax; i++) {
        pix = (PIX *)ptraRemove(papix, i, L_NO_COMPACTION);
        box = (BOX *)ptraRemove(pabox, i, L_NO_COMPACTION);
        if (pix) pixaAddPix(pixat, pix, L_INSERT);
        if (box) pixaAddBox(pixat, box, L_INSERT);
    }

    ptraGetMaxIndex(papix, &imax);
    ptraGetActualCount(papix, &nactual);
    if (rp->display)
        lept_stderr("After removal:   imax = %4d, actual = %4d\n\n",
                    imax, nactual);

    return pixat;
}


    /* Reconstruction with compaction */
static PIXA *
ReconstructPixa2(L_REGPARAMS  *rp,
                 L_PTRA       *papix,
                 L_PTRA       *pabox)
{
l_int32  i, imax, nactual;
BOX     *box;
PIX     *pix;
PIXA    *pixat;

    ptraGetMaxIndex(papix, &imax);
    ptraGetActualCount(papix, &nactual);
    if (rp->display)
        lept_stderr("Before removal:    imax = %4d, actual = %4d\n",
                    imax, nactual);

        /* Remove half */
    pixat = pixaCreate(imax + 1);
    for (i = 0; i <= imax; i++) {
        if (i % 2 == 0) {
            pix = (PIX *)ptraRemove(papix, i, L_NO_COMPACTION);
            box = (BOX *)ptraRemove(pabox, i, L_NO_COMPACTION);
            if (pix) pixaAddPix(pixat, pix, L_INSERT);
            if (box) pixaAddBox(pixat, box, L_INSERT);
        }
    }

        /* Compact */
    ptraGetMaxIndex(papix, &imax);
    ptraGetActualCount(papix, &nactual);
    if (rp->display)
        lept_stderr("Before compaction: imax = %4d, actual = %4d\n",
                    imax, nactual);
    ptraCompactArray(papix);
    ptraCompactArray(pabox);
    ptraGetMaxIndex(papix, &imax);
    ptraGetActualCount(papix, &nactual);
    if (rp->display)
        lept_stderr("After compaction:  imax = %4d, actual = %4d\n",
                    imax, nactual);

        /* Remove the rest (and test compaction with removal) */
    while (1) {
        ptraGetActualCount(papix, &nactual);
        if (nactual == 0) break;

        pix = (PIX *)ptraRemove(papix, 0, L_COMPACTION);
        box = (BOX *)ptraRemove(pabox, 0, L_COMPACTION);
        pixaAddPix(pixat, pix, L_INSERT);
        pixaAddBox(pixat, box, L_INSERT);
    }

    ptraGetMaxIndex(papix, &imax);
    ptraGetActualCount(papix, &nactual);
    if (rp->display)
        lept_stderr("After removal:     imax = %4d, actual = %4d\n\n",
                    imax, nactual);

    return pixat;
}


PIX *
SaveResult(PIXA   *pixac,
           PIXA  **ppixa,
           l_int32  w,
           l_int32  h,
           l_int32  newline)
{
PIX   *pixd;

    pixd = pixaDisplay(*ppixa, w, h);
    pixaAddPix(pixac, pixd, L_COPY);
    pixaDestroy(ppixa);
    return pixd;
}
