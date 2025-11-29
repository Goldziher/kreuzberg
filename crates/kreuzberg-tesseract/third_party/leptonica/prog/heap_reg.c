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
 * heap_reg.c
 *
 *   Tests the heap utility.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

struct HeapElement {
    l_float32  distance;
    l_int32    x;
    l_int32    y;
};
typedef struct HeapElement  HEAPEL;

static const l_int32  NELEM = 50;

NUMA *ExtractNumaFromHeap(L_HEAP  *lh);


int main(int    argc,
         char **argv)
{
l_uint8      *data;
l_int32       i;
size_t        size;
l_float32     frand, fval;
HEAPEL       *item;
NUMA         *na1, *na2, *na3, *na4, *na5;
L_HEAP       *lh;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_mkdir("lept/heap");

        /* Make a numa of random numbers */
    na1 = numaCreate(5);
    for (i = 0; i < NELEM; i++) {
        frand = (l_float32)rand() / (l_float32)RAND_MAX;
        numaAddNumber(na1, frand);
    }
    numaWriteMem(&data, &size, na1);
    regTestWriteDataAndCheck(rp, data, size, "na");  /* 0 */
    lept_free(data);

        /* Make an array of HEAPELs with the same numbers */
    lh = lheapCreate(5, L_SORT_INCREASING);
    for (i = 0; i < NELEM; i++) {
        numaGetFValue(na1, i, &fval);
        item = (HEAPEL *)lept_calloc(1, sizeof(HEAPEL));
        item->distance = fval;
        lheapAdd(lh, item);
    }

        /* Re-sort for strict order */
    lheapSortStrictOrder(lh);
    na2 = ExtractNumaFromHeap(lh);
    numaWriteMem(&data, &size, na2);
    regTestWriteDataAndCheck(rp, data, size, "na");  /* 1 */
    lept_free(data);

        /* Switch the direction and re-sort strict order */
    lh->direction = L_SORT_DECREASING;
    lheapSortStrictOrder(lh);
    na3 = ExtractNumaFromHeap(lh);
    numaWriteMem(&data, &size, na3);
    regTestWriteDataAndCheck(rp, data, size, "na");  /* 2 */
    lept_free(data);

        /* Switch direction again and re-sort strict sort */
    lh->direction = L_SORT_INCREASING;
    lheapSortStrictOrder(lh);
    na4 = ExtractNumaFromHeap(lh);
    numaWriteMem(&data, &size, na4);
    regTestWriteDataAndCheck(rp, data, size, "na");  /* 3 */
    lept_free(data);

        /* Switch direction again and re-sort strict sort */
    lh->direction = L_SORT_DECREASING;
    lheapSortStrictOrder(lh);
    na5 = ExtractNumaFromHeap(lh);
    numaWriteMem(&data, &size, na5);
    regTestWriteDataAndCheck(rp, data, size, "na");  /* 4 */
    lept_free(data);

    regTestCompareFiles(rp, 1, 3);  /* 5 */
    regTestCompareFiles(rp, 2, 4);  /* 6 */

        /* Remove the elements, one at a time */
    for (i = 0; lheapGetCount(lh) > 0; i++) {
        item = (HEAPEL *)lheapRemove(lh);
        if (rp->display)
           lept_stderr("item %d: %f\n", i, item->distance);
        lept_free(item);
    }

    lheapDestroy(&lh, 1);
    numaDestroy(&na1);
    numaDestroy(&na2);
    numaDestroy(&na3);
    numaDestroy(&na4);
    numaDestroy(&na5);
    return regTestCleanup(rp);
}


    /* This just uses the heap array.  It will only be
       ordered if the heap is in strict ordering.  */
NUMA *
ExtractNumaFromHeap(L_HEAP  *lh)
{
l_int32  i, n;
HEAPEL  *item;
NUMA    *na;

    n = lheapGetCount(lh);
    na = numaCreate(0);
    for (i = 0; i < n; i++) {
        item = (HEAPEL *)lh->array[i];
        numaAddNumber(na, item->distance);
    }
    return na;
}
