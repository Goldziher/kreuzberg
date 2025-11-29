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
 *  settest.c
 *
 *  Tests set function for RGB (uint32) keys.
 *
 *  We take a colormapped image and use the set to find the unique
 *  colors in the image.  These are stored as 32-bit rgb keys.
 *  Also test the iterator on the set.
 *
 *  For a more complete set of tests, see the operations tested in maptest.c.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static L_ASET *BuildSet(PIX *pix, l_int32 factor, l_int32 print);
static void TestSetIterator(L_ASET *s, l_int32  print);


l_int32 main(int    argc,
             char **argv)
{
L_ASET  *s;
PIX     *pix;

    setLeptDebugOK(1);
    pix = pixRead("weasel8.240c.png");

        /* Build the set from all the pixels. */
    s = BuildSet(pix, 1, FALSE);
    TestSetIterator(s, FALSE);
    l_asetDestroy(&s);

        /* Ditto, but just with a few pixels */
    s = BuildSet(pix, 10, TRUE);
    TestSetIterator(s, TRUE);
    l_asetDestroy(&s);
    pixDestroy(&pix);

    pix = pixRead("marge.jpg");
    startTimer();
    s = BuildSet(pix, 1, FALSE);
    lept_stderr("Time (250K pixels): %7.3f sec\n", stopTimer());
    TestSetIterator(s, FALSE);
    l_asetDestroy(&s);
    pixDestroy(&pix);
    return 0;
}

static L_ASET *
BuildSet(PIX     *pix,
         l_int32  factor,
         l_int32  print)
{
l_int32    i, j, w, h, wpl, val;
l_uint32   val32;
l_uint32  *data, *line;
L_ASET    *s;
PIXCMAP   *cmap;
RB_TYPE    key;
RB_TYPE   *pval;

    lept_stderr("\n --------------- Begin building set --------------\n");
    s = l_asetCreate(L_UINT_TYPE);
    data = pixGetData(pix);
    wpl = pixGetWpl(pix);
    cmap = pixGetColormap(pix);
    pixGetDimensions(pix, &w, &h, NULL);
    for (i = 0; i < h; i += factor) {
        line = data + i * wpl;
        for (j = 0; j < w; j += factor) {
            if (cmap) {
                val = GET_DATA_BYTE(line, j);
                pixcmapGetColor32(cmap, val, &val32);
                key.utype = val32;
            } else {
                key.utype = line[j];
            }
            pval = l_asetFind(s, key);
            if (pval && print)
                lept_stderr("key = %llx\n", key.utype);
            l_asetInsert(s, key);
        }
    }
    lept_stderr("Size: %d\n", l_asetSize(s));
    if (print)
        l_rbtreePrint(stderr, s);
    lept_stderr(" ----------- End Building set -----------------\n");

    return s;
}

static void
TestSetIterator(L_ASET  *s,
                l_int32  print)
{
l_int32       count;
L_ASET_NODE  *n;

    n = l_asetGetFirst(s);
    count = 0;
    lept_stderr("\n --------------- Begin iter listing --------------\n");
    while (n) {
        count++;
        if (print)
#if 0
            lept_stderr("key = %x\n", n->key.utype);
#else
            lept_stderr("key = %llx\n", n->key.utype);
#endif
        n = l_asetGetNext(n);
    }
    lept_stderr("Count from iterator: %d\n", count);
    lept_stderr(" --------------- End iter listing --------------\n");
    return;
}

