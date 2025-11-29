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
 *  maptest.c
 *
 *  Tests map function for RGB (uint32) keys and count (int32) values.
 *  The underlying rbtree takes 64 bit keys and values, so it also works
 *  transparently with 32 bit keys and values.
 *
 *  We take a colormapped image and use the map to accumulate a
 *  histogram of the colors, using the 32-bit rgb value as the key.
 *  The value is the number of pixels with that color that we have seen.
 *
 *  Also:
 *   * test the forward and backward iterators on the map
 *   * build an inverse colormap table using a map.
 *   * test RGB histogram and counting functions in pix4.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static L_AMAP *BuildMapHistogram(PIX *pix, l_int32 factor, l_int32 print);
static void DisplayMapHistogram(L_AMAP *m, PIXCMAP *cmap,
                                const char *rootname);
static void DisplayMapRGBHistogram(L_AMAP *m, const char *rootname);
static void TestMapIterator1(L_AMAP *m, l_int32  print);
static void TestMapIterator2(L_AMAP *m, l_int32  print);
static void TestMapIterator3(L_AMAP *m, l_int32  print);
static void TestMapIterator4(L_AMAP *m, l_int32  print);
static void TestMapIterator5(L_AMAP *m, l_int32  print);


l_int32 main(int    argc,
             char **argv)
{
l_int32    i, n, w, h, ncolors;
l_uint32   val32;
L_AMAP    *m;
NUMA      *na;
PIX       *pix;
PIXCMAP   *cmap;
RB_TYPE    key, value;
RB_TYPE   *pval;

    setLeptDebugOK(1);
    lept_mkdir("lept/map");

    pix = pixRead("weasel8.240c.png");
    pixGetDimensions(pix, &w, &h, NULL);
    lept_stderr("Image area in pixels: %d\n", w * h);
    cmap = pixGetColormap(pix);

        /* Build the histogram, stored in a map.  Then compute
         * and display the histogram as the number of pixels vs
         * the colormap index */
    m = BuildMapHistogram(pix, 1, FALSE);
    TestMapIterator1(m, FALSE);
    TestMapIterator2(m, FALSE);
    DisplayMapHistogram(m, cmap, "/tmp/lept/map/map1");
    l_amapDestroy(&m);

        /* Ditto, but just with a few pixels */
    m = BuildMapHistogram(pix, 14, TRUE);
    DisplayMapHistogram(m, cmap, "/tmp/lept/map/map2");
    l_amapDestroy(&m);

        /* Do in-order tranversals, using the iterators */
    m = BuildMapHistogram(pix, 7, FALSE);
    TestMapIterator1(m, TRUE);
    TestMapIterator2(m, TRUE);
    l_amapDestroy(&m);

        /* Do in-order tranversals, with iterators and destroying the map */
    m = BuildMapHistogram(pix, 7, FALSE);
    TestMapIterator3(m, TRUE);
    lept_free(m);
    m = BuildMapHistogram(pix, 7, FALSE);
    TestMapIterator4(m, TRUE);
    lept_free(m);

        /* Do in-order tranversals, with iterators and reversing the map */
    m = BuildMapHistogram(pix, 7, FALSE);
    TestMapIterator5(m, TRUE);
    l_amapDestroy(&m);

        /* Build a histogram the old-fashioned way */
    na = pixGetCmapHistogram(pix, 1);
    numaWrite("/tmp/lept/map/map2.na", na);
    gplotSimple1(na, GPLOT_PNG, "/tmp/lept/map/map3", NULL);
    numaDestroy(&na);

        /* Build a separate map from (rgb) --> colormap index ... */
    m = l_amapCreate(L_UINT_TYPE);
    n = pixcmapGetCount(cmap);
    for (i = 0; i < n; i++) {
        pixcmapGetColor32(cmap, i, &val32);
        key.utype = val32;
        value.itype = i;
        l_amapInsert(m, key, value);
    }
        /* ... and test the map */
    for (i = 0; i < n; i++) {
        pixcmapGetColor32(cmap, i, &val32);
        key.utype = val32;
        pval = l_amapFind(m, key);
        if (pval && (i != pval->itype))
            lept_stderr("i = %d != val = %llx\n", i, pval->itype);
    }
    l_amapDestroy(&m);
    pixDestroy(&pix);

        /* Build and display a real RGB histogram */
    pix = pixRead("wyom.jpg");
    m = pixGetColorAmapHistogram(pix, 1);
    DisplayMapRGBHistogram(m, "/tmp/lept/map/map4");
    pixNumColors(pix, 1, &ncolors);
    lept_stderr(" Using pixNumColors: %d\n", ncolors);
    pixCountRGBColors(pix, 1, &ncolors);
    lept_stderr(" Using pixCountRGBColors: %d\n", ncolors);
    l_amapDestroy(&m);
    pixDestroy(&pix);

    return 0;
}

static L_AMAP *
BuildMapHistogram(PIX     *pix,
                  l_int32  factor,
                  l_int32  print)
{
l_int32    i, j, w, h, wpl, val;
l_uint32   val32;
l_uint32  *data, *line;
L_AMAP    *m;
PIXCMAP   *cmap;
RB_TYPE    key, value;
RB_TYPE   *pval;

    lept_stderr("\n --------------- Begin building map --------------\n");
    m = l_amapCreate(L_UINT_TYPE);
    data = pixGetData(pix);
    wpl = pixGetWpl(pix);
    cmap = pixGetColormap(pix);
    pixGetDimensions(pix, &w, &h, NULL);
    for (i = 0; i < h; i += factor) {
        line = data + i * wpl;
        for (j = 0; j < w; j += factor) {
            val = GET_DATA_BYTE(line, j);
            pixcmapGetColor32(cmap, val, &val32);
            key.utype = val32;
            pval = l_amapFind(m, key);
            if (!pval)
                value.itype = 1;
            else
                value.itype = 1 + pval->itype;
            if (print)
                lept_stderr("key = %llx, val = %lld\n", key.utype, value.itype);
            l_amapInsert(m, key, value);
        }
    }
    lept_stderr("Size: %d\n", l_amapSize(m));
    if (print)
        l_rbtreePrint(stderr, m);
    lept_stderr(" ----------- End Building map -----------------\n");

    return m;
}


static void
DisplayMapHistogram(L_AMAP      *m,
                    PIXCMAP     *cmap,
                    const char  *rootname)
{
char      buf[128];
l_int32   i, n, ival;
l_uint32  val32;
NUMA     *na;
RB_TYPE   key;
RB_TYPE  *pval;

    n = pixcmapGetCount(cmap);
    na = numaCreate(n);
    for (i = 0; i < n; i++) {
        pixcmapGetColor32(cmap, i, &val32);
        key.utype = val32;
        pval = l_amapFind(m, key);
        if (pval) {
            ival = pval->itype;
            numaAddNumber(na, ival);
        }
    }
    gplotSimple1(na, GPLOT_PNG, rootname, NULL);
    snprintf(buf, sizeof(buf), "%s.png", rootname);
    l_fileDisplay(buf, 700, 0, 1.0);
    numaDestroy(&na);
    return;
}

static void
DisplayMapRGBHistogram(L_AMAP      *m,
                       const char  *rootname)
{
char          buf[128];
l_int32       ncolors, npix, ival, maxn, maxn2;
l_uint32      maxcolor;
L_AMAP_NODE  *n;
NUMA         *na;

    lept_stderr("\n --------------- Display RGB histogram ------------\n");
    na = numaCreate(0);
    ncolors = npix = 0;
    maxn = 0;
    maxcolor = 0;
    n = l_amapGetFirst(m);
    while (n) {
        ncolors++;
        ival = n->value.itype;
        if (ival > maxn) {
            maxn = ival;
            maxcolor = n->key.utype;
        }
        numaAddNumber(na, ival);
        npix += ival;
        n = l_amapGetNext(n);
    }
    lept_stderr(" Num colors = %d, Num pixels = %d\n", ncolors, npix);
    lept_stderr(" Color %x has count %d\n", maxcolor, maxn);
    maxn2 = amapGetCountForColor(m, maxcolor);
    if (maxn != maxn2)
        lept_stderr(" Error: maxn2 = %d; not equal to %d\n", maxn, maxn2);
    gplotSimple1(na, GPLOT_PNG, rootname, NULL);
    snprintf(buf, sizeof(buf), "%s.png", rootname);
    l_fileDisplay(buf, 1400, 0, 1.0);
    numaDestroy(&na);
    return;
}

static void
TestMapIterator1(L_AMAP  *m,
                 l_int32  print)  /* forward iterator; fixed tree */
{
l_int32       count, npix, ival;
l_uint32      ukey;
L_AMAP_NODE  *n;

    n = l_amapGetFirst(m);
    count = 0;
    npix = 0;
    lept_stderr("\n ---------- Begin forward iter listing -----------\n");
    while (n) {
        count++;
        ukey = n->key.utype;
        ival = n->value.itype;
        npix += ival;
        if (print)
            lept_stderr("key = %x, val = %d\n", ukey, ival);
        n = l_amapGetNext(n);
    }
    lept_stderr("Count from iterator: %d\n", count);
    lept_stderr("Number of pixels: %d\n", npix);
    lept_stderr(" ------------ End forward iter listing -----------\n");
    return;
}

static void
TestMapIterator2(L_AMAP  *m,
                 l_int32  print)  /* reverse iterator; fixed tree */
{
l_int32       count, npix, ival;
l_uint32      ukey;
L_AMAP_NODE  *n;

    n = l_amapGetLast(m);
    count = 0;
    npix = 0;
    lept_stderr("\n ---------- Begin reverse iter listing -----------\n");
    while (n) {
        count++;
        ukey = n->key.utype;
        ival = n->value.itype;
        npix += ival;
        if (print)
            lept_stderr("key = %x, val = %d\n", ukey, ival);
        n = l_amapGetPrev(n);
    }
    lept_stderr("Count from iterator: %d\n", count);
    lept_stderr("Number of pixels: %d\n", npix);
    lept_stderr(" ------------ End reverse iter listing -----------\n");
    return;
}

static void
TestMapIterator3(L_AMAP  *m,
                 l_int32  print)  /* forward iterator; delete the tree */
{
l_int32       count, npix, ival;
l_uint32      ukey;
L_AMAP_NODE  *n, *nn;

    n = l_amapGetFirst(m);
    count = 0;
    npix = 0;
    lept_stderr("\n ------ Begin forward iter; delete tree ---------\n");
    while (n) {
        nn = l_amapGetNext(n);
        count++;
        ukey = n->key.utype;
        ival = n->value.itype;
        npix += ival;
        if (print)
            lept_stderr("key = %x, val = %d\n", ukey, ival);
        l_amapDelete(m, n->key);
        n = nn;
    }
    lept_stderr("Count from iterator: %d\n", count);
    lept_stderr("Number of pixels: %d\n", npix);
    lept_stderr(" ------ End forward iter; delete tree ---------\n");
    return;
}

static void
TestMapIterator4(L_AMAP  *m,
                 l_int32  print)  /* reverse iterator; delete the tree */
{
l_int32       count, npix, ival;
l_uint32      ukey;
L_AMAP_NODE  *n, *np;

    n = l_amapGetLast(m);
    count = 0;
    npix = 0;
    lept_stderr("\n ------- Begin reverse iter; delete tree --------\n");
    while (n) {
        np = l_amapGetPrev(n);
        count++;
        ukey = n->key.utype;
        ival = n->value.itype;
        npix += ival;
        if (print)
            lept_stderr("key = %x, val = %d\n", ukey, ival);
        l_amapDelete(m, n->key);
        n = np;
    }
    lept_stderr("Count from iterator: %d\n", count);
    lept_stderr("Number of pixels: %d\n", npix);
    lept_stderr(" ------- End reverse iter; delete tree --------\n");
    return;
}

static void
TestMapIterator5(L_AMAP  *m,
                 l_int32  print)  /* reverse iterator; rebuild the tree */
{
l_int32       count, npix, ival;
l_uint32      ukey;
L_AMAP       *m2;
L_AMAP_NODE  *n, *np;

    m2 = l_amapCreate(L_UINT_TYPE);
    n = l_amapGetLast(m);
    count = npix = 0;
    lept_stderr("\n ------- Begin reverse iter; rebuild tree --------\n");
    while (n) {
        np = l_amapGetPrev(n);
        count++;
        ukey = n->key.utype;
        ival = n->value.itype;
        l_amapInsert(m2, n->key, n->value);
        npix += ival;
        if (print)
            lept_stderr("key = %x, val = %d\n", ukey, ival);
        l_amapDelete(m, n->key);
        n = np;
    }
    m->root = m2->root;
    lept_free(m2);
    lept_stderr("Count from iterator: %d\n", count);
    lept_stderr("Number of pixels: %d\n", npix);
    count = npix = 0;
    n = l_amapGetLast(m);
    while (n) {
        np = l_amapGetPrev(n);
        count++;
        ukey = n->key.utype;
        ival = n->value.itype;
        npix += ival;
        if (print)
            lept_stderr("key = %x, val = %d\n", ukey, ival);
        n = np;
    }
    lept_stderr("Count from iterator: %d\n", count);
    lept_stderr("Number of pixels: %d\n", npix);
    lept_stderr(" ------- End reverse iter; rebuild tree --------\n");
    return;
}


