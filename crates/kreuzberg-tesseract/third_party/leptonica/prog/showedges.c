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
 * showedges.c
 *
 *    Uses computation of half edge function, along with thresholding.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   SMOOTH_WIDTH_1       2  /* must be smaller */
#define   SMOOTH_WIDTH_2       4  /* must be larger */
#define   THRESHOLD            5  /* low works best */


int main(int    argc,
         char **argv)
{
char    *infile, *outfile;
l_int32  d;
PIX     *pixs, *pixgr, *pixb;

    if (argc != 3)
        return ERROR_INT(" Syntax: showedges infile outfile", __func__, 1);
    infile = argv[1];
    outfile = argv[2];
    setLeptDebugOK(1);

    pixs = pixRead(infile);
    d = pixGetDepth(pixs);
    if (d != 8 && d != 32)
        return ERROR_INT("d not 8 or 32 bpp", __func__, 1);

    pixgr = pixHalfEdgeByBandpass(pixs, SMOOTH_WIDTH_1, SMOOTH_WIDTH_1,
                                        SMOOTH_WIDTH_2, SMOOTH_WIDTH_2);
    pixb = pixThresholdToBinary(pixgr, THRESHOLD);
    pixInvert(pixb, pixb);
    pixWrite(outfile, pixb, IFF_PNG);
    return 0;
}

