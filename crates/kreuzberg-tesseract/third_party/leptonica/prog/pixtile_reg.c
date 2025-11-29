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
 *   pixtile_reg.c
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "pix_internal.h"

static l_int32 TestTiling(PIX *pixd, PIX *pixs, l_int32 nx, l_int32 ny,
                          l_int32 w, l_int32 h, l_int32 xoverlap,
                          l_int32 yoverlap);


int main(int    argc,
         char **argv)
{
PIX  *pixs, *pixd;

    setLeptDebugOK(1);
    pixs = pixRead("test24.jpg");
    pixd = pixCreateTemplateNoInit(pixs);

    TestTiling(pixd, pixs, 1, 1, 0, 0, 183, 83);
    TestTiling(pixd, pixs, 0, 1, 60, 0, 30, 20);
    TestTiling(pixd, pixs, 1, 0, 0, 60, 40, 40);
    TestTiling(pixd, pixs, 0, 0, 27, 31, 27, 31);
    TestTiling(pixd, pixs, 0, 0, 400, 400, 40, 20);
    TestTiling(pixd, pixs, 7, 9, 0, 0, 35, 35);
    TestTiling(pixd, pixs, 0, 0, 27, 31, 0, 0);
    TestTiling(pixd, pixs, 7, 9, 0, 0, 0, 0);

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}


l_int32
TestTiling(PIX     *pixd,
           PIX     *pixs,
           l_int32  nx,
           l_int32  ny,
           l_int32  w,
           l_int32  h,
           l_int32  xoverlap,
           l_int32  yoverlap)
{
l_int32     i, j, same;
PIX        *pixt;
PIXTILING  *pt;

    pixClearAll(pixd);
    pt = pixTilingCreate(pixs, nx, ny, w, h, xoverlap, yoverlap);
    pixTilingGetCount(pt, &nx, &ny);
    pixTilingGetSize(pt, &w, &h);
    if (pt)
        lept_stderr("nx,ny = %d,%d; w,h = %d,%d; overlap = %d,%d\n",
                    nx, ny, w, h, pt->xoverlap, pt->yoverlap);
    else
        return 1;

    for (i = 0; i < ny; i++) {
        for (j = 0; j < nx; j++) {
            pixt = pixTilingGetTile(pt, i, j);
            pixTilingPaintTile(pixd, i, j, pixt, pt);
            pixDestroy(&pixt);
        }
    }
    pixEqual(pixs, pixd, &same);
    if (same)
        lept_stderr("Tiling OK\n");
    else
        lept_stderr("Tiling ERROR !\n");

    pixTilingDestroy(&pt);
    return 0;
}
