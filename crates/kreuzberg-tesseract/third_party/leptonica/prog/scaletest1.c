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
 * scaletest1.c
 *
 *      scaletest1 filein scalex scaley fileout
 *    where
 *      scalex, scaley are floating point input
 *
 *  For d < 8 bpp, writes output in png
 *  For d >= 8 bpp, uses output file extension to determine output format.
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
l_int32    d, fmt;
l_float32  scalex, scaley;
PIX       *pixs, *pixd;

    if (argc != 5)
	return ERROR_INT(" Syntax:  scaletest1 filein scalex scaley fileout",
	                 __func__, 1);
    filein = argv[1];
    scalex = atof(argv[2]);
    scaley = atof(argv[3]);
    fileout = argv[4];
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
	return ERROR_INT("pixs not made", __func__, 1);

        /* choose type of scaling operation */
#if 1
    pixd = pixScale(pixs, scalex, scaley);
#elif 0
    pixd = pixScaleLI(pixs, scalex, scaley);
#elif 0
    pixd = pixScaleSmooth(pixs, scalex, scaley);
#elif 0
    pixd = pixScaleAreaMap(pixs, scalex, scaley);
#elif 0
    pixd = pixScaleBySampling(pixs, scalex, scaley);
#else
    pixd = pixScaleToGray(pixs, scalex);
#endif

    d = pixGetDepth(pixd);

    if (d < 8) {
        pixWrite(fileout, pixd, IFF_PNG);
    } else {
        fmt = getImpliedFileFormat(fileout);
        pixWrite(fileout, pixd, fmt);
    }

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}

