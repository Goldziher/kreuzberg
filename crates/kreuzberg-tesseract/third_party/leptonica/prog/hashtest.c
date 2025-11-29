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
 * hashtest.c
 *
 *   Allows testing of ordered and unordered set/map functions on dna,
 *   pta and strings, similar to hash_reg.c.
 *
     Use:
 *      hashtest dnasize ptasize strsize
 *   where to test each type use
 *      dnasize in [1, ... 10M]
 *      ptasize in [1, ... 5000]
 *      strsize in [3, 4, 5]
 *   and to skip each type use 0
 *
 *   For example,
 *      hashtest 0 0 4
 *   will test all 26^4 alphabetic strings of length 4.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"
#include "array_internal.h"
#include "pix_internal.h"

static SARRAY *BuildShortStrings(l_int32 nchars);
static PTA *BuildPointSet(l_int32  w, l_int32 h);

l_int32 main(int    argc,
             char **argv)
{
l_int32     dnasize, ptasize, strsize, i, n, c1, c2, c3, s1;
L_ASET     *set;
L_DNA      *da0, *da1, *da2, *da3;
L_HASHMAP  *hmap;
PTA        *pta0, *pta1, *pta2, *pta3;
SARRAY     *sa0, *sa1, *sa2, *sa3;

    if (argc != 4)
        return ERROR_INT(" Syntax:  hashtest dnasize ptasize strsize",
                         __func__, 1);
    setLeptDebugOK(1);

    dnasize = atoi(argv[1]);
    ptasize = atoi(argv[2]);
    strsize = atoi(argv[3]);
    if (dnasize < 0)
        return ERROR_INT(" dnasize < 0; must be in [0 ... 10M]", __func__, 1);
    if (dnasize > 10000000) {
        lept_stderr("very large dnasize = %d; using 10M\n", dnasize);
        dnasize = 10000000;
    }
    if (ptasize < 0)
        return ERROR_INT(" ptasize < 0; must be in [0 ... 5000]", __func__, 1);
    if (ptasize > 5000) {
        lept_stderr("very large ptasize = %d; using 5000\n", ptasize);
        ptasize = 5000;
    }
    if (strsize < 0) strsize = 0;
    if (strsize < 3 && strsize != 0) {
        lept_stderr("strsize < 3; using 3\n");
        strsize = 3;
    }
    if (strsize > 5) {
        lept_stderr("strsize > 5; using 5\n");
        strsize = 5;
    }

        /* Test dna hashing with aset */
    if (dnasize == 0) goto skipped_1;
    da0 = l_dnaMakeSequence(0, 1, dnasize);
    n = l_dnaGetCount(da0);
    lept_stderr("\n================= Dna ===================\n"
                "n = %d\n", n);
    for (i = 0; i < n; i++)
        da0->array[i] = -7.4 * da0->array[i];
    da1 = l_dnaSelectRange(da0, 0, 0.6 * n);
    l_dnaJoin(da1, da0, 0.1 * n, 0.2 * n);  /* add dups */
    da2 = l_dnaSelectRange(da0, 0.4 * n, -1);  /* overlap with da1 */
    l_dnaJoin(da2, da0, 0.7 * n, 0.8 * n);  /* add dups */
    l_dnaDestroy(&da0);

    startTimer();
    c1 = l_dnaGetCount(da1);
    c2 = l_dnaGetCount(da2);
    lept_stderr("c1 = %d, c2 = %d\n", da1->n, da2->n);
    l_dnaRemoveDupsByAset(da2, &da3);
    c1 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    l_dnaIntersectionByAset(da1, da2, &da3);
    c2 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    l_dnaUnionByAset(da1, da2, &da3);
    c3 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    lept_stderr("Dna Set Time: %7.2f sec\n", stopTimer());
    lept_stderr("Aset: set# = %d, intersection# = %d, union# = %d\n",
                c1, c2, c3);

        /* Test dna hashing with hashmap */
    c1 = l_dnaGetCount(da1);
    c2 = l_dnaGetCount(da2);
    startTimer();
    l_dnaRemoveDupsByHmap(da2, &da3, NULL);
    c1 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    l_dnaIntersectionByHmap(da1, da2, &da3);
    c2 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    lept_stderr("Dna Hash Time: %7.2f sec\n", stopTimer());
    l_dnaUnionByHmap(da1, da2, &da3);
    c3 = l_dnaGetCount(da3);
    l_dnaDestroy(&da3);
    lept_stderr("Hmap: set# = %d, intersection# = %d, union# = %d\n",
                c1, c2, c3);
    l_dnaDestroy(&da1);
    l_dnaDestroy(&da2);

        /* Test pt hashing with aset */
skipped_1:
    if (ptasize == 0) goto skipped_2;
    pta0 = BuildPointSet(ptasize, ptasize);
    n = ptaGetCount(pta0);
    lept_stderr("\n================= Pta ===================\n"
                "n = %d\n", n);
    pta1 = ptaSelectRange(pta0, 0, 0.6 * n);
    ptaJoin(pta1, pta0, 0.1 * n, 0.2 * n);  /* add dups */
    pta2 = ptaSelectRange(pta0, 0.4 * n, -1);  /* overlap with pta1 */
    ptaJoin(pta2, pta0, 0.7 * n, 0.8 * n);  /* add dups */

    c1 = ptaGetCount(pta1);
    c2 = ptaGetCount(pta2);
    lept_stderr("c1 = %d, c2 = %d\n", pta1->n, pta2->n);
    startTimer();
    ptaRemoveDupsByAset(pta2, &pta3);
    c1 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    ptaIntersectionByAset(pta1, pta2, &pta3);
    c2 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    ptaUnionByAset(pta1, pta2, &pta3);
    c3 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    lept_stderr("Pta Set Time: %7.2f sec\n", stopTimer());
    lept_stderr("Aset: set# = %d, intersection# = %d, union# = %d\n",
                c1, c2, c3);
    ptaDestroy(&pta0);

        /* Test pt hashing with hashmap */
    c1 = ptaGetCount(pta1);
    c2 = ptaGetCount(pta2);
    startTimer();
    ptaRemoveDupsByHmap(pta2, &pta3, NULL);
    c1 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    ptaIntersectionByHmap(pta1, pta2, &pta3);
    c2 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    ptaUnionByHmap(pta1, pta2, &pta3);
    c3 = ptaGetCount(pta3);
    ptaDestroy(&pta3);
    lept_stderr("Pta Hash Time: %7.2f sec\n", stopTimer());
    lept_stderr("Hmap: set# = %d, intersection# = %d, union# = %d\n",
                c1, c2, c3);
    ptaDestroy(&pta1);
    ptaDestroy(&pta2);

        /* Test string hashing with aset */
skipped_2:
    if (strsize == 0) goto skipped_3;
    sa0 = BuildShortStrings(strsize);
    n = sarrayGetCount(sa0);
    lept_stderr("\n================= Strings ===================\n"
                "n = %d\n", n);
    sa1 = sarraySelectRange(sa0, 0, 0.6 * n);
    sarrayAppendRange(sa1, sa0, 0.1 * n, 0.2 * n);  /* add dups */
    sa2 = sarraySelectRange(sa0, 0.4 * n, -1);   /* overlaps sa1 */
    sarrayAppendRange(sa2, sa0, 0.7 * n, 0.8 * n);  /* add dups */

    c1 = sarrayGetCount(sa1);
    c2 = sarrayGetCount(sa2);
    lept_stderr("c1 = %d, c2 = %d\n", sa1->n, sa2->n);
    set = l_asetCreateFromSarray(sa2);
    s1 = l_asetSize(set);
    lept_stderr("Aset: num unique: %d\n", s1);
    l_asetDestroy(&set);
    startTimer();
    sarrayRemoveDupsByAset(sa2, &sa3);
    c1 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    sarrayIntersectionByAset(sa1, sa2, &sa3);
    c2 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    sarrayUnionByAset(sa1, sa2, &sa3);
    c3 = sarrayGetCount(sa3);
    sarrayDestroy(&sa3);
    lept_stderr("String Set Time: %7.2f sec\n", stopTimer());
    lept_stderr("Aset: set# = %d, intersection# = %d, union# = %d\n",
                c1, c2, c3);

    hmap = l_hmapCreateFromSarray(sa1);
    lept_stderr("Hmap: num unique: %d\n", hmap->nitems);
    l_hmapDestroy(&hmap);

    startTimer();
    sarrayRemoveDupsByHmap(sa2, &sa3, NULL);
    c1 = sa3->n;
    sarrayDestroy(&sa3);
    sarrayIntersectionByHmap(sa1, sa2, &sa3);
    c2 = sa3->n;
    sarrayDestroy(&sa3);
    sarrayUnionByHmap(sa1, sa2, &sa3);
    c3 = sa3->n;
    sarrayDestroy(&sa3);
    lept_stderr("String Hash Time: %7.2f sec\n", stopTimer());
    lept_stderr("Hmap: set# = %d, intersection# = %d, union# = %d\n",
                c1, c2, c3);
    sarrayDestroy(&sa0);
    sarrayDestroy(&sa1);
    sarrayDestroy(&sa2);

skipped_3:
    return 0;
}


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
/*
                                if (i == 17 && j == 12 && k == 4 && l == 21) {
                                    l_hashStringToUint64(buf, &hash);
                                    lept_stderr("  %llx\n", hash);
                                }
*/
                            }
                        }
                    }
                }
            }
        }
    }

    return sa;
}

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
