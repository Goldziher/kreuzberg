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
 * cornertest.c
 *
 *   e.g., use on witten.png
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   LINE_SIZE   29


int main(int    argc,
         char **argv)
{
char    *filein, *fileout;
l_int32  x, y, n, i;
PIX     *pixs;
PTA     *pta;
PTAA    *ptaa, *ptaa2, *ptaa3;

    if (argc != 3)
        return ERROR_INT(" Syntax:  cornertest filein fileout", __func__, 1);
    filein = argv[1];
    fileout = argv[2];

    setLeptDebugOK(1);
    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

        /* Clean noise in LR corner of witten.tif */
    pixSetPixel(pixs, 2252, 3051, 0);
    pixSetPixel(pixs, 2252, 3050, 0);
    pixSetPixel(pixs, 2251, 3050, 0);

    pta = pixFindCornerPixels(pixs);
    ptaWriteStream(stderr, pta, 1);

        /* Test pta and ptaa I/O */
#if 1
    ptaa = ptaaCreate(3);
    ptaaAddPta(ptaa, pta, L_COPY);
    ptaaAddPta(ptaa, pta, L_COPY);
    ptaaAddPta(ptaa, pta, L_COPY);
    ptaaWriteStream(stderr, ptaa, 1);
    ptaaWrite("/tmp/junkptaa", ptaa, 1);
    ptaa2 = ptaaRead("/tmp/junkptaa");
    ptaaWrite("/tmp/junkptaa2", ptaa2, 1);
    ptaaWrite("/tmp/junkptaa3", ptaa, 0);
    ptaa3 = ptaaRead("/tmp/junkptaa3");
    ptaaWrite("/tmp/junkptaa4", ptaa3, 0);
    ptaaDestroy(&ptaa);
    ptaaDestroy(&ptaa2);
    ptaaDestroy(&ptaa3);
#endif

        /* mark corner pixels */
    n = ptaGetCount(pta);
    for (i = 0; i < n; i++) {
        ptaGetIPt(pta, i, &x, &y);
        pixRenderLine(pixs, x - LINE_SIZE, y, x + LINE_SIZE, y, 5,
                      L_FLIP_PIXELS);
        pixRenderLine(pixs, x, y - LINE_SIZE, x, y + LINE_SIZE, 5,
                      L_FLIP_PIXELS);
    }

    pixWrite(fileout, pixs, IFF_PNG);

    pixDestroy(&pixs);
    ptaDestroy(&pta);
    ptaDestroy(&pta);
    return 0;
}

