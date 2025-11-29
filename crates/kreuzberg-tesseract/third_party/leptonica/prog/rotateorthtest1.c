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
 * rotateorthtest1.c
 *
 *    Tests and timings for 90 and 180 degree rotations
 *        rotateorthtest1 filein fileout [direction]
 *    where
 *        direction = 1 for cw; -1 for ccw
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  NTIMES   10


int main(int    argc,
         char **argv)
{
l_int32    dir;
PIX       *pixs, *pixd, *pixt;
l_float32  pops;
char      *filein, *fileout;

    if (argc != 3 && argc != 4)
        return ERROR_INT(" Syntax:  rotateorthtest1 filein fileout [direction]",
                         __func__, 1);
    filein = argv[1];
    fileout = argv[2];
    if (argc == 4)
        dir = atoi(argv[3]);
    else
        dir = 1;
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pix not made", __func__, 1);

        /* Do a single operation */
#if 1
    pixd = pixRotate90(pixs, dir);
#elif 0
    pixd = pixRotate180(NULL, pixs);
#elif 0
    pixd = pixRotateLR(NULL, pixs);
#elif 0
    pixd = pixRotateTB(NULL, pixs);
#endif

        /* Time rotate 90, allocating & destroying each time */
#if 0
    startTimer();
    w = pixGetWidth(pixs);
    h = pixGetHeight(pixs);
    for (i = 0; i < NTIMES; i++) {
        pixd = pixRotate90(pixs, dir);
        pixDestroy(&pixd);
    }
    pops = (l_float32)(w * h * NTIMES) / stopTimer();
    lept_stderr("MPops for 90 rotation: %7.3f\n", pops / 1000000.);
    pixd = pixRotate90(pixs, dir);
#endif

        /* Time rotate 180, with no alloc/destroy */
#if 0
    startTimer();
    w = pixGetWidth(pixs);
    h = pixGetHeight(pixs);
    pixd = pixCreateTemplate(pixs);
    for (i = 0; i < NTIMES; i++)
        pixRotate180(pixd, pixs);
    pops = (l_float32)(w * h * NTIMES) / stopTimer();
    lept_stderr("MPops for 180 rotation: %7.3f\n", pops / 1000000.);
#endif


        /* Test rotate 180 not in-place */
#if 0
    pixt = pixRotate180(NULL, pixs);
    pixd = pixRotate180(NULL, pixt);
    pixEqual(pixs, pixd, &eq);
    if (eq) lept_stderr("2 rots gives I\n");
    else lept_stderr("2 rots fail to give I\n");
    pixDestroy(&pixt);
#endif

       /* Test rotate 180 in-place */
#if 0
    pixd = pixCopy(NULL, pixs);
    pixRotate180(pixd, pixd);
    pixRotate180(pixd, pixd);
    pixEqual(pixs, pixd, &eq);
    if (eq) lept_stderr("2 rots gives I\n");
    else lept_stderr("2 rots fail to give I\n");
#endif

        /* Mix rotate 180 with LR/TB */
#if 0
    pixd = pixRotate180(NULL, pixs);
    pixRotateLR(pixd, pixd);
    pixRotateTB(pixd, pixd);
    pixEqual(pixs, pixd, &eq);
    if (eq) lept_stderr("180 rot OK\n");
    else lept_stderr("180 rot error\n");
#endif

    if (pixGetDepth(pixd) < 8)
        pixWrite(fileout, pixd, IFF_PNG);
    else
        pixWrite(fileout, pixd, IFF_JFIF_JPEG);

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}

