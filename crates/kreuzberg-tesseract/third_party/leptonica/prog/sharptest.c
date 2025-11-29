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
 * sharptest.c
 *
 *      sharptest filein smooth fract fileout
 *
 *      (1) Use smooth = 1 for 3x3 smoothing filter
 *              smooth = 2 for 5x5 smoothing filter, etc.
 *      (2) Use fract in typical range (0.2 - 0.7)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
PIX       *pixs, *pixd;
l_int32    smooth;
l_float32  fract;
char      *filein, *fileout;

    if (argc != 5)
        return ERROR_INT(" Syntax:  sharptest filein smooth fract fileout",
                         __func__, 1);
    filein = argv[1];
    smooth = atoi(argv[2]);
    fract = atof(argv[3]);
    fileout = argv[4];
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

    pixd = pixUnsharpMasking(pixs, smooth, fract);
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}

