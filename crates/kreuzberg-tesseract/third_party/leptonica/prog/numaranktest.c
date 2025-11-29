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
 * numaranktest.c
 *
 *    Test on 8 bpp grayscale (e.g., w91frag.jpg)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32   BIN_SIZE = 1;

int main(int    argc,
         char **argv)
{
char      *filein;
l_int32    i, j, w, h, d, sampling;
l_float32  rank, rval;
l_uint32   val;
NUMA      *na, *nah, *nar, *nav;
PIX       *pix;

    if (argc != 3)
        return ERROR_INT(" Syntax:  numaranktest filein sampling", __func__, 1);
    filein = argv[1];
    sampling = atoi(argv[2]);

    setLeptDebugOK(1);
    lept_mkdir("lept/numa");

    if ((pix = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);
    pixGetDimensions(pix, &w, &h, &d);
    if (d != 8)
        return ERROR_INT("d != 8 bpp", __func__, 1);

    na = numaCreate(0);
    for (i = 0; i < h; i += sampling) {
        for (j = 0; j < w; j += sampling) {
            pixGetPixel(pix, j, i, &val);
            numaAddNumber(na, val);
        }
    }
    nah = numaMakeHistogramClipped(na, BIN_SIZE, 255);

    nar = numaCreate(0);
    for (rval = 0.0; rval < 256.0; rval += 2.56) {
        numaHistogramGetRankFromVal(nah, rval, &rank);
        numaAddNumber(nar, rank);
    }
    gplotSimple1(nar, GPLOT_PNG, "/tmp/lept/numa/rank", "rank vs val");
    l_fileDisplay("/tmp/lept/numa/rank.png", 0, 0, 1.0);

    nav = numaCreate(0);
    for (rank = 0.0; rank <= 1.0; rank += 0.01) {
        numaHistogramGetValFromRank(nah, rank, &rval);
        numaAddNumber(nav, rval);
    }
    gplotSimple1(nav, GPLOT_PNG, "/tmp/lept/numa/val", "val vs rank");
    l_fileDisplay("/tmp/lept/numa/val.png", 750, 0, 1.0);

    pixDestroy(&pix);
    numaDestroy(&na);
    numaDestroy(&nah);
    numaDestroy(&nar);
    numaDestroy(&nav);
    return 0;
}
