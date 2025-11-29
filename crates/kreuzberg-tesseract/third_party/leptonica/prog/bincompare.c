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
 * bincompare.c
 *
 *    Bitwise comparison of two binary images
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* set one of these to 1 */
#define   XOR                   1
#define   SUBTRACT_1_FROM_2     0
#define   SUBTRACT_2_FROM_1     0

int main(int    argc,
         char **argv)
{
l_int32  w, h, d, n;
char    *filein1, *filein2, *fileout;
PIX     *pixs1, *pixs2;

    if (argc != 4)
        return ERROR_INT(" Syntax:  bincompare filein1 filein2 fileout",
                         __func__, 1);
    filein1 = argv[1];
    filein2 = argv[2];
    fileout = argv[3];
    setLeptDebugOK(1);

    if ((pixs1 = pixRead(filein1)) == NULL)
        return ERROR_INT("pixs1 not made", __func__, 1);
    if ((pixs2 = pixRead(filein2)) == NULL)
        return ERROR_INT("pixs2 not made", __func__, 1);

    pixGetDimensions(pixs1, &w, &h, &d);
    if (d != 1)
        return ERROR_INT("pixs1 not binary", __func__, 1);

    pixCountPixels(pixs1, &n, NULL);
    lept_stderr("Number of fg pixels in file1 = %d\n", n);
    pixCountPixels(pixs2, &n, NULL);
    lept_stderr("Number of fg pixels in file2 = %d\n", n);

#if XOR
    lept_stderr("xor: 1 ^ 2\n");
    pixRasterop(pixs1, 0, 0, w, h, PIX_SRC ^ PIX_DST, pixs2, 0, 0);
    pixCountPixels(pixs1, &n, NULL);
    lept_stderr("Number of fg pixels in XOR = %d\n", n);
    pixWrite(fileout, pixs1, IFF_PNG);
#elif  SUBTRACT_1_FROM_2
    lept_stderr("subtract: 2 - 1\n");
    pixRasterop(pixs1, 0, 0, w, h, PIX_SRC & PIX_NOT(PIX_DST), pixs2, 0, 0);
    pixCountPixels(pixs1, &n, NULL);
    lept_stderr("Number of fg pixels in 2 - 1 = %d\n", n);
    pixWrite(fileout, pixs1, IFF_PNG);
#elif  SUBTRACT_2_FROM_1
    lept_stderr("subtract: 1 - 2\n");
    pixRasterop(pixs1, 0, 0, w, h, PIX_DST & PIX_NOT(PIX_SRC), pixs2, 0, 0);
    pixCountPixels(pixs1, &n, NULL);
    lept_stderr("Number of fg pixels in 1 - 2 = %d\n", n);
    pixWrite(fileout, pixs1, IFF_PNG);
#else
    lept_stderr("no comparison selected\n");
#endif

    return 0;
}

