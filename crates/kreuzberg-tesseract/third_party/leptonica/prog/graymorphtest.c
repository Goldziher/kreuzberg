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
 * graymorphtest.c
 *
 *     Implements basic grayscale morphology; tests speed
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char    *filein, *fileout;
l_int32  wsize, hsize, w, h, d;
PIX     *pixs, *pixd;

    if (argc != 5)
        return ERROR_INT(" Syntax:  graymorphtest filein wsize hsize fileout",
                         __func__, 1);
    filein = argv[1];
    wsize = atoi(argv[2]);
    hsize = atoi(argv[3]);
    fileout = argv[4];
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);
    pixGetDimensions(pixs, &w, &h, &d);
    if (d != 8)
        return ERROR_INT("pix not 8 bpp", __func__, 1);

    /* ---------- Choose an operation ----------  */
#if 1
    pixd = pixDilateGray(pixs, wsize, hsize);
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);
#elif 0
    pixd = pixErodeGray(pixs, wsize, hsize);
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);
#elif 0
    pixd = pixOpenGray(pixs, wsize, hsize);
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);
#elif 0
    pixd = pixCloseGray(pixs, wsize, hsize);
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);
#elif 0
    pixd = pixTophat(pixs, wsize, hsize, TOPHAT_WHITE);
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);
#elif 0
    pixd = pixTophat(pixs, wsize, hsize, TOPHAT_BLACK);
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    pixDestroy(&pixd);
#endif


    /* ---------- Speed ----------  */
#if 0
    startTimer();
    pixd = pixCloseGray(pixs, wsize, hsize);
    lept_stderr(" Speed is %6.2f MPix/sec\n",
          (l_float32)(4 * w * h) / (1000000. * stopTimer()));
    pixWrite(fileout, pixd, IFF_PNG);
#endif

    pixDestroy(&pixs);
    return 0;
}


