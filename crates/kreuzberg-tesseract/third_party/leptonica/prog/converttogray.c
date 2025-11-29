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
 * converttogray.c
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
          char **argv)
{
char    *filein;
char    *fileout = NULL;
l_int32  d, same;
PIX     *pixs, *pixd, *pix1, *pix2, *pix3, *pix4;

    if (argc != 2 && argc != 3)
        return ERROR_INT(" Syntax:  converttogray filein [fileout]",
                         __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/gray");

    filein = argv[1];
    if (argc == 3) fileout = argv[2];
    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

    if (fileout) {
        pixd = pixConvertRGBToGray(pixs, 0.33, 0.34, 0.33);
        pixWrite(fileout, pixd, IFF_PNG);
        pixDestroy(&pixs);
        pixDestroy(&pixd);
        return 0;
    }

    d = pixGetDepth(pixs);
    if (d == 2) {
        pix1 = pixConvert2To8(pixs, 0x00, 0x55, 0xaa, 0xff, TRUE);
        pix2 = pixConvert2To8(pixs, 0x00, 0x55, 0xaa, 0xff, FALSE);
        pixEqual(pix1, pix2, &same);
        if (same)
            lept_stderr("images are the same\n");
        else
            lept_stderr("images are different!\n");
        pixWrite("/tmp/lept/gray/pix1.png", pix1, IFF_PNG);
        pixWrite("/tmp/lept/gray/pix2.png", pix2, IFF_PNG);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixSetColormap(pixs, NULL);
        pix3 = pixConvert2To8(pixs, 0x00, 0x55, 0xaa, 0xff, TRUE);
        pix4 = pixConvert2To8(pixs, 0x00, 0x55, 0xaa, 0xff, FALSE);
        pixEqual(pix3, pix4, &same);
        if (same)
            lept_stderr("images are the same\n");
        else
            lept_stderr("images are different!\n");
        pixWrite("/tmp/lept/gray/pix3.png", pix3, IFF_PNG);
        pixWrite("/tmp/lept/gray/pix4.png", pix4, IFF_PNG);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
    } else if (d == 4) {
        pix1 = pixConvert4To8(pixs, TRUE);
        pix2 = pixConvert4To8(pixs, FALSE);
        pixEqual(pix1, pix2, &same);
        if (same)
            lept_stderr("images are the same\n");
        else
            lept_stderr("images are different!\n");
        pixWrite("/tmp/lept/gray/pix1.png", pix1, IFF_PNG);
        pixWrite("/tmp/lept/gray/pix2.png", pix2, IFF_PNG);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixSetColormap(pixs, NULL);
        pix3 = pixConvert4To8(pixs, TRUE);
        pix4 = pixConvert4To8(pixs, FALSE);
        pixEqual(pix3, pix4, &same);
        if (same)
            lept_stderr("images are the same\n");
        else
            lept_stderr("images are different!\n");
        pixWrite("/tmp/lept/gray/pix3.png", pix3, IFF_PNG);
        pixWrite("/tmp/lept/gray/pix4.png", pix4, IFF_PNG);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
    } else {
        L_INFO("only converts 2 and 4 bpp; d = %d\n", __func__, d);
    }

    pixDestroy(&pixs);
    return 0;
}

