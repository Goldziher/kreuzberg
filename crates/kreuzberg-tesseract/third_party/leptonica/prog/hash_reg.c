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
 *  hash_reg.c
 *
 *  Tests hashing functions for strings, points and double arrays, for both:
 *    *  ordered sets (underlying rbtree implementation for sorting)
 *    *  hashing (underlying hashmap implementation)
 *
 *  We use 64-bit hashes, which are sufficiently randomized so that
 *  you expect the probability of a collision with 10M objects to be
 *  about 10^-5.  [For n < 2^32, the collision probability goes as
 *  approximately (n / 4*10^9)^2].
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "array_internal.h"

static SARRAY *BuildShortStrings(l_int32 nchars);
static PTA *BuildPointSet(l_int32 w, l_int32 h);

const l_int32  string_set = 10967;
const l_int32  string_union = 18278;
const l_int32  string_intersection = 3656;
const l_int32  pta_set = 150000;
const l_int32  pta_union = 250000;
const l_int32  pta_intersection = 50001;
const l_int32  da_set = 48000;
const l_int32  da_union = 80000;
const l_int32  da_intersection = 16001;

l_int32 main(int    argc,
             char **argv)
{
l_uint8      *data1;
l_int32       i, n, c1, c2, c3, c4, c5, s1;
size_t        size1;
L_ASET       *set;
L_DNA        *da0, *da1, *da2, *da3, *da4, *da5;
L_HASHMAP    *hmap;
PTA          *pta0, *pta1, *pta2, *pta3;
SARRAY       *sa0, *sa1, *sa2, *sa3;
PIX          *pix1;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/hash");

        /* Test string hashing with aset */
    sa0 = BuildShortStrings(3);
    n = sarrayGetCount(sa0);
    sa1 = sarraySelectRange(sa0, 0, 0.6 * n);
    sarrayAppendRange(sa1, sa0, 0.1 * n, 0.2 * n);  /* add dups */
    sa2 = sarraySelectRange(sa0, 0.4 * n, -1);   /* overlaps sa1 */
    sarrayAppendRange(sa2, sa0, 0.7 * n, 0.8 * n);  /* add dups */
    c1 = sarrayGetCount(sa1);
    c2 = sarrayGetCount(sa2);
    set = l_asetCreateFromSarray(sa2);
    s1 = l_asetSize(set);
    regTestCompareValues(rp, string_set, s1, 0);  /* 0 */
    if (rp->display) {
        lept_stderr("String operations\n  c1 = %d, c2 = %d\n", c1, c2);
        lept_stderr("  aset: size of set without dups: %d\n", s1);
    }
    l_asetDestroy(&set);
    sarrayRemoveDupsByAset(sa2, &sa3);
    c1 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    regTestCompareValues(rp, string_set, c1, 0);  /* 1 */
    if (rp->display) lept_stderr("  aset: size without dups = %d\n", c1);
    sarrayIntersectionByAset(sa1, sa2, &sa3);
    c1 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    regTestCompareValues(rp, string_intersection, c1, 1);  /* 2 */
    if (rp->display) lept_stderr("  aset: intersection size = %d\n", c1);
    sarrayUnionByAset(sa1, sa2, &sa3);
    c1 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    regTestCompareValues(rp, string_union, c1, 0);  /* 3 */
    if (rp->display) lept_stderr("  aset: union size = %d\n", c1);

        /* Test string hashing with hashmap */
    hmap = l_hmapCreateFromSarray(sa1);
    c1 = hmap->nitems;
    regTestCompareValues(rp, string_set, c1, 0);  /* 4 */
    if (rp->display) lept_stderr("  hmap: set size without dups: %d\n", c1);
    l_hmapDestroy(&hmap);

    sarrayRemoveDupsByHmap(sa2, &sa3, NULL);
    c1 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    regTestCompareValues(rp, string_set, c1, 0);  /* 5 */
    if (rp->display) lept_stderr("  hmap: size without dups = %d\n", c1);
    sarrayIntersectionByHmap(sa1, sa2, &sa3);
    c1 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    regTestCompareValues(rp, string_intersection, c1, 1);  /* 6 */
    if (rp->display) lept_stderr("  hmap: intersection size = %d\n", c1);
    sarrayUnionByHmap(sa1, sa2, &sa3);
    c1 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    regTestCompareValues(rp, string_union, c1, 0);  /* 7 */
    if (rp->display) lept_stderr("  hmap: union size = %d\n", c1);
    sarrayDestroy(&sa3);
    sarrayDestroy(&sa0);
    sarrayDestroy(&sa1);
    sarrayDestroy(&sa2);

        /* Test point hashing with aset. */
    pta0 = BuildPointSet(500, 500);
    n = ptaGetCount(pta0);
    pta1 = ptaSelectRange(pta0, 0, 0.6 * n);
    ptaJoin(pta1, pta0, 0.1 * n, 0.2 * n);  /* add dups */
    pta2 = ptaSelectRange(pta0, 0.4 * n, -1);  /* overlap with pta1 */
    ptaJoin(pta2, pta0, 0.7 * n, 0.8 * n);  /* add dups */
    c1 = ptaGetCount(pta1);
    c2 = ptaGetCount(pta2);
    set = l_asetCreateFromPta(pta2);
    s1 = l_asetSize(set);
    l_asetDestroy(&set);
    regTestCompareValues(rp, pta_set, s1, 0);  /* 8 */
    if (rp->display) {
        lept_stderr("Pt array operations\n  c1 = %d, c2 = %d\n", c1, c2);
        lept_stderr("  aset: size of set without dups: %d\n", s1);
    }
    ptaRemoveDupsByAset(pta2, &pta3);
    c1 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    regTestCompareValues(rp, pta_set, c1, 0);  /* 9 */
    if (rp->display) lept_stderr("  aset: size without dups = %d\n", c1);
    ptaIntersectionByAset(pta1, pta2, &pta3);
    c1 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    regTestCompareValues(rp, pta_intersection, c1, 1);  /* 10 */
    if (rp->display) lept_stderr("  aset: intersection size = %d\n", c1);
    ptaUnionByAset(pta1, pta2, &pta3);
    c1 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    regTestCompareValues(rp, pta_union, c1, 0);  /* 11 */
    if (rp->display) lept_stderr("  aset: union size = %d\n", c1);

        /* Test point hashing with hashmap */
    hmap = l_hmapCreateFromPta(pta2);
    c1 = hmap->nitems;
    regTestCompareValues(rp, pta_set, c1, 0);  /* 12 */
    if (rp->display) lept_stderr("  hmap: set size without dups: %d\n", c1);
    l_hmapDestroy(&hmap);
    ptaRemoveDupsByHmap(pta2, &pta3, NULL);
    c1 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    regTestCompareValues(rp, pta_set, c1, 0);  /* 13 */
    if (rp->display) lept_stderr("  hmap: size without dups = %d\n", c1);
    ptaIntersectionByHmap(pta1, pta2, &pta3);
    c1 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    regTestCompareValues(rp, pta_intersection, c1, 1);  /* 14 */
    if (rp->display) lept_stderr("  hmap: intersection size = %d\n", c1);
    ptaUnionByHmap(pta1, pta2, &pta3);
    c1 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    regTestCompareValues(rp, pta_union, c1, 0);  /* 15 */
    if (rp->display) lept_stderr("  hmap: union size = %d\n", c1);
    ptaDestroy(&pta0);
    ptaDestroy(&pta1);
    ptaDestroy(&pta2);

        /* Test dna hashing with aset */
    da0 = l_dnaMakeSequence(0, 1, 80000);
    n = l_dnaGetCount(da0);
    for (i = 0; i < n; i++)
        da0->array[i] = -7.4 * da0->array[i];
    da1 = l_dnaSelectRange(da0, 0, 0.6 * n);
    l_dnaJoin(da1, da0, 0.1 * n, 0.2 * n);  /* add dups */
    da2 = l_dnaSelectRange(da0, 0.4 * n, -1);  /* overlap with da1 */
    l_dnaJoin(da2, da0, 0.7 * n, 0.8 * n);  /* add dups */
    c1 = l_dnaGetCount(da1);
    c2 = l_dnaGetCount(da2);
    set = l_asetCreateFromDna(da2);
    s1 = l_asetSize(set);
    l_asetDestroy(&set);
    regTestCompareValues(rp, da_set, s1, 0);  /* 16 */
    if (rp->display) {
        lept_stderr("Double array operations\n  c1 = %d, c2 = %d\n", c1, c2);
        lept_stderr("  aset: size of set without dups: %d\n", s1);
    }
    l_dnaRemoveDupsByAset(da2, &da3);
    c1 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    regTestCompareValues(rp, da_set, c1, 0);  /* 17 */
    if (rp->display) lept_stderr("  aset: size without dups = %d\n", c1);
    l_dnaIntersectionByAset(da1, da2, &da3);
    c1 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    regTestCompareValues(rp, da_intersection, c1, 1);  /* 18 */
    if (rp->display) lept_stderr("  aset: intersection size = %d\n", c1);
    l_dnaUnionByAset(da1, da2, &da3);
    c1 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    regTestCompareValues(rp, da_union, c1, 0);  /* 19 */
    if (rp->display) lept_stderr("  aset: union size = %d\n", c1);

        /* Test dna hashing with hashmap */
    hmap = l_hmapCreateFromDna(da2);
    c1 = hmap->nitems;
    regTestCompareValues(rp, da_set, c1, 0);  /* 20 */
    if (rp->display) lept_stderr("  hmap: set size without dups: %d\n", c1);
    l_hmapDestroy(&hmap);
    l_dnaRemoveDupsByHmap(da2, &da3, NULL);
    c1 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    regTestCompareValues(rp, da_set, c1, 0);  /* 21 */
    if (rp->display) lept_stderr("  hmap: size without dups = %d\n", c1);
    l_dnaIntersectionByHmap(da1, da2, &da3);
    c1 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    regTestCompareValues(rp, da_intersection, c1, 1);  /* 22 */
    if (rp->display) lept_stderr("  hmap: intersection size = %d\n", c1);
    l_dnaUnionByHmap(da1, da2, &da3);
    c1 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    regTestCompareValues(rp, da_union, c1, 0);  /* 23 */
    if (rp->display) lept_stderr("  hmap: union size = %d\n", c1);
    l_dnaDestroy(&da0);
    l_dnaDestroy(&da1);
    l_dnaDestroy(&da2);

        /* Another test of dna hashing, showing equivalence
         * of results between ordered and unordered sets. */
    da0 = l_dnaMakeSequence(0, 1, 20);
    n = l_dnaGetCount(da0);
    for (i = 0; i < n; i++)
        da0->array[i] = 3.4 * da0->array[i];
    da1 = l_dnaSelectRange(da0, 0, 0.6 * n);
    l_dnaJoin(da1, da0, 0.1 * n, 0.2 * n);  /* add dups */
    da2 = l_dnaSelectRange(da0, 0.4 * n, -1);  /* overlap with da1 */
    l_dnaJoin(da2, da0, 0.7 * n, 0.8 * n);  /* add dups */
    c1 = l_dnaGetCount(da1);
    c2 = l_dnaGetCount(da2);
    l_dnaRemoveDupsByAset(da2, &da3);
    l_dnaRemoveDupsByHmap(da2, &da4, NULL);
        /* Show the two sets da3 and da4 are identical in content,
         * because they have the same size and their intersection also
         * has the same size. */
    c3 = l_dnaGetCount(da3);
    c4 = l_dnaGetCount(da4);
    regTestCompareValues(rp, c3, c4, 0);  /* 24 */
    l_dnaIntersectionByHmap(da3, da4, &da5);
    c5 = l_dnaGetCount(da5);
    regTestCompareValues(rp, c4, c5, 0);  /* 25 */
    if (rp->display) {
        lept_stderr("\nc1 = %d, c2 = %d\n", c1, c2);
        lept_stderr("c3 = %d, c4 = %d, c5 = %d\n", c3, c4, c5);
        l_dnaWriteMem(&data1, &size1, da4);
        lept_stderr("%s", data1);
        lept_free(data1);
    }
    l_dnaDestroy(&da0);
    l_dnaDestroy(&da1);
    l_dnaDestroy(&da2);
    l_dnaDestroy(&da3);
    l_dnaDestroy(&da4);
    l_dnaDestroy(&da5);

        /* Test pixel counting operations with hashmap and ordered map */
    pix1 = pixRead("wet-day.jpg");
    pixCountRGBColorsByHash(pix1, &c1);
    pixCountRGBColors(pix1, 1, &c2);
    regTestCompareValues(rp, 42427, c1, 0);  /* 26 */
    regTestCompareValues(rp, 42427, c2, 0);  /* 27 */
    if (rp->display) {
        lept_stderr("Color count using hashmap: %d\n", c1);
        lept_stderr("Color count using aset: %d\n", c2);
    }
    pixDestroy(&pix1);

    return regTestCleanup(rp);
}

/* ------------------------------------------------------- */
static SARRAY *
BuildShortStrings(l_int32  nchars)  /* 3, 4 or 5 */
{
char      buf[64];
l_int32   i, j, k, l, m;
l_uint64  hash;
SARRAY   *sa;

    sa = sarrayCreate(1000);
    for (i = 0; i < 26; i++) {
        snprintf(buf, sizeof(buf), "%c", i + 0x61);
        sarrayAddString(sa, buf, L_COPY);
        for (j = 0; j < 26; j++) {
            snprintf(buf, sizeof(buf), "%c%c", i + 0x61, j + 0x61);
            sarrayAddString(sa, buf, L_COPY);
            for (k = 0; k < 26; k++) {
                snprintf(buf, sizeof(buf), "%c%c%c", i + 0x61, j + 0x61,
                         k + 0x61);
                sarrayAddString(sa, buf, L_COPY);
                if (nchars > 3) {
                    for (l = 0; l < 26; l++) {
                        snprintf(buf, sizeof(buf), "%c%c%c%c", i + 0x61,
                                 j + 0x61, k + 0x61, l + 0x61);
                        sarrayAddString(sa, buf, L_COPY);
                        if (nchars > 4) {
                            for (m = 0; m < 26; m++) {
                                snprintf(buf, sizeof(buf), "%c%c%c%c%c",
                                         i + 0x61, j + 0x61, k + 0x61,
                                         l + 0x61, m + 0x61);
                                sarrayAddString(sa, buf, L_COPY);
                                if (i == 17 && j == 12 && k == 4 && l == 21) {
                                    l_hashStringToUint64(buf, &hash);
                                    lept_stderr("  %llx\n", hash);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    return sa;
}

/* ------------------------------------------------------- */
static PTA *
BuildPointSet(l_int32  w, l_int32 h)
{
l_int32  i, j;
PTA     *pta;

    pta = ptaCreate(w * h);
    for (i = 0; i < h; i++) {
        for (j = 0; j < w; j++)
            ptaAddPt(pta, 316.27 * j, 243.59 * i);
    }
    return pta;
}

