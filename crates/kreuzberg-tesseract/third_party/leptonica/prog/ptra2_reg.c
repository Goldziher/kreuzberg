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
 * ptra2_reg.c
 *
 *    Testing:
 *       - basic ptra and ptraa operations
 *       - bin sort using ptra
 *       - boxaEqual() and pixaEqual()
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void BoxaSortTest(L_REGPARAMS *rp, const char *fname, l_int32 index,
                  const char *text);
void PixaSortTest(L_REGPARAMS *rp, const char *fname, l_int32 index,
                  const char *text);

int main(int    argc,
         char **argv)
{
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "ptra2_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/ptra");

        /* 0 - 8 */
    BoxaSortTest(rp, "feyn-fract.tif", 1, "Boxa sort test on small image");
        /* 9 - 17 */
    BoxaSortTest(rp, "feyn.tif", 2, "Boxa sort test on large image");
        /* 18 - 27 */
    PixaSortTest(rp, "feyn-fract.tif", 3, "Pixa sort test on small image");
        /* 28 - 37 */
    PixaSortTest(rp, "feyn.tif", 4, "Pixa sort test on large image");
    return regTestCleanup(rp);
}


void
BoxaSortTest(L_REGPARAMS  *rp,
             const char   *fname,
             l_int32       index,
             const char   *text)
{
l_int32   i, n, m, imax, w, h, x, count, same;
BOX      *box;
BOXA     *boxa, *boxa1, *boxa2, *boxa3;
NUMA     *na, *nad1, *nad2, *nad3, *naindex;
PIX      *pixs;
L_PTRA   *pa, *pad, *paindex;
L_PTRAA  *paa;
char      buf[256];

    lept_stderr("Test %d: %s\n", index, text);
    pixs = pixRead(fname);
    boxa = pixConnComp(pixs, NULL, 8);

        /* Sort by x */
    boxa1 = boxaSort(boxa, L_SORT_BY_X, L_SORT_INCREASING, &nad1);
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/boxa1.%d.ba", index);
    boxaWrite(buf, boxa1);
    regTestCheckFile(rp, buf);  /* 0 */
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/nad1.%d.na", index);
    numaWrite(buf, nad1);
    regTestCheckFile(rp, buf);  /* 1 */

    boxa2 = boxaBinSort(boxa, L_SORT_BY_X, L_SORT_INCREASING, &nad2);
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/boxa2.%d.ba", index);
    boxaWrite(buf, boxa2);
    regTestCheckFile(rp, buf);  /* 2 */
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/nad2.%d.na", index);
    numaWrite(buf, nad2);
    regTestCheckFile(rp, buf);  /* 3 */

    boxaEqual(boxa1, boxa2, 0, NULL, &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 4 */
    if (rp->display && same)
        lept_stderr("boxa1 and boxa2 are identical\n");
    boxaEqual(boxa1, boxa2, 2, &naindex, &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 5 */
    if (rp->display && same)
        lept_stderr("boxa1 and boxa2 are same at maxdiff = 2\n");
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/naindex.%d.na", index);
    numaWrite(buf, naindex);
    regTestCheckFile(rp, buf);  /* 6 */
    numaDestroy(&naindex);
    boxaDestroy(&boxa1);
    numaDestroy(&nad1);
    numaDestroy(&nad2);

        /* Now do this stuff with ptra and ptraa */
        /* First, store the boxes in a ptraa, where each ptra contains
         * the boxes, and store the sort index in a ptra of numa */
    pixGetDimensions(pixs, &w, &h, NULL);
    paa = ptraaCreate(w);
    paindex = ptraCreate(w);
    n = boxaGetCount(boxa);
    if (rp->display) lept_stderr("n = %d\n", n);
    for (i = 0; i < n; i++) {
        box = boxaGetBox(boxa, i, L_CLONE);
        boxGetGeometry(box, &x, NULL, NULL, NULL);
        pa = ptraaGetPtra(paa, x, L_HANDLE_ONLY);
        na = (NUMA *)ptraGetPtrToItem(paindex, x);
        if (!pa) {  /* na also needs to be made */
            pa = ptraCreate(1);
            ptraaInsertPtra(paa, x, pa);
            na = numaCreate(1);
            ptraInsert(paindex, x, na, L_MIN_DOWNSHIFT);
        }
        ptraAdd(pa, box);
        numaAddNumber(na, i);
    }
    ptraGetActualCount(paindex, &count);
    if (rp->display) lept_stderr("count = %d\n", count);

        /* Flatten the ptraa to a ptra containing all the boxes
         * in sorted order, and put them in a boxa */
    pad = ptraaFlattenToPtra(paa);
    ptraaDestroy(&paa, FALSE, FALSE);
    ptraGetActualCount(pad, &m);
    if (m != n)
        lept_stderr("n(orig) = %d, m(new) = %d\n", n, m);
    boxa3 = boxaCreate(m);
    for (i = 0; i < m; i++) {
        box = (BOX *)ptraRemove(pad, i, L_NO_COMPACTION);
        boxaAddBox(boxa3, box, L_INSERT);
    }
    ptraDestroy(&pad, FALSE, FALSE);

        /* Extract the data from the ptra of Numa, putting it into
         * a single Numa */
    ptraGetMaxIndex(paindex, &imax);
    nad3 = numaCreate(0);
    if (rp->display) lept_stderr("imax = %d\n\n", imax);
    for (i = 0; i <= imax; i++) {
        na = (NUMA *)ptraRemove(paindex, i, L_NO_COMPACTION);
        numaJoin(nad3, na, 0, -1);
        numaDestroy(&na);
    }

    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/boxa3.%d.ba", index);
    boxaWrite(buf, boxa3);
    regTestCheckFile(rp, buf);  /* 7 */
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/nad3.%d.na", index);
    numaWrite(buf, nad3);
    regTestCheckFile(rp, buf);  /* 8 */

    boxaDestroy(&boxa2);
    boxaDestroy(&boxa3);
    numaDestroy(&nad3);
    ptraDestroy(&paindex, FALSE, FALSE);
    pixDestroy(&pixs);
    boxaDestroy(&boxa);
    return;
}


void
PixaSortTest(L_REGPARAMS  *rp,
             const char   *fname,
             l_int32       index,
             const char   *text)
{
l_int32  same;
BOXA    *boxa, *boxa1, *boxa2;
NUMA    *nap1, *nap2, *naindex;
PIX     *pixs;
PIXA    *pixa, *pixa1, *pixa2;
char     buf[256];

    lept_stderr("Test %d: %s\n", index, text);
    pixs = pixRead(fname);
    boxa = pixConnComp(pixs, &pixa, 8);

    pixa1 = pixaSort(pixa, L_SORT_BY_X, L_SORT_INCREASING, &nap1, L_CLONE);
    boxa1 = pixaGetBoxa(pixa1, L_CLONE);
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/bap1.%d.ba", index);
    boxaWrite(buf, boxa1);
    regTestCheckFile(rp, buf);  /* 0 */
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/nap1.%d.na", index);
    numaWrite(buf, nap1);
    regTestCheckFile(rp, buf);  /* 1 */
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/pixa1.%d.pa", index);
    pixaWrite(buf, pixa1);
    regTestCheckFile(rp, buf);  /* 2 */

    pixa2 = pixaBinSort(pixa, L_SORT_BY_X, L_SORT_INCREASING, &nap2, L_CLONE);
    boxa2 = pixaGetBoxa(pixa2, L_CLONE);
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/bap2.%d.ba", index);
    boxaWrite(buf, boxa2);
    regTestCheckFile(rp, buf);  /* 3 */
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/nap2.%d.na", index);
    numaWrite(buf, nap2);
    regTestCheckFile(rp, buf);  /* 4 */
    snprintf(buf, sizeof(buf), "/tmp/lept/ptra/pixa2.%d.pa", index);
    pixaWrite(buf, pixa2);
    regTestCheckFile(rp, buf);  /* 5 */

    boxaEqual(boxa1, boxa2, 0, &naindex, &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 6 */
    if (rp->display && same)
        lept_stderr("boxa1 and boxa2 are identical\n");
    numaDestroy(&naindex);
    boxaEqual(boxa1, boxa2, 3, &naindex, &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 7 */
    if (rp->display && same)
        lept_stderr("boxa1 and boxa2 are same at maxdiff = 2\n");
    numaDestroy(&naindex);

    pixaEqual(pixa1, pixa2, 0, &naindex, &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 8 */
    if (rp->display && same)
        lept_stderr("pixa1 and pixa2 are identical\n");
    numaDestroy(&naindex);
    pixaEqual(pixa1, pixa2, 3, &naindex, &same);
    regTestCompareValues(rp, 1, same, 0.0);  /* 9 */
    if (rp->display && same)
        lept_stderr("pixa1 and pixa2 are same at maxdiff = 2\n\n");
    numaDestroy(&naindex);

    boxaDestroy(&boxa);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    numaDestroy(&nap1);
    numaDestroy(&nap2);
    pixaDestroy(&pixa);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixDestroy(&pixs);
    return;
}
