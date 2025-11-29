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
 * graphicstest.c
 *
 *  e.g.:   graphicstest fish24.jpg junkout
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char    *filein, *fileout;
l_int32  d;
BOX     *box1, *box2, *box3, *box4;
BOXA    *boxa;
PIX     *pixs, *pix1;
PTA     *pta;

    if (argc != 3)
        return ERROR_INT(" Syntax: graphicstest filein fileout", __func__, 1);
    filein = argv[1];
    fileout = argv[2];
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT(" Syntax: pixs not made", __func__, 1);
    d = pixGetDepth(pixs);
    if (d <= 8)
        pix1 = pixConvertTo32(pixs);
    else
        pix1 = pixClone(pixs);

        /* Paint on RGB */
    pixRenderLineArb(pix1, 450, 20, 850, 320, 5, 200, 50, 125);
    pixRenderLineArb(pix1, 30, 40, 440, 40, 5, 100, 200, 25);
    pixRenderLineBlend(pix1, 30, 60, 440, 70, 5, 115, 200, 120, 0.3);
    pixRenderLineBlend(pix1, 30, 600, 440, 670, 9, 215, 115, 30, 0.5);
    pixRenderLineBlend(pix1, 130, 700, 540, 770, 9, 255, 255, 250, 0.4);
    pixRenderLineBlend(pix1, 130, 800, 540, 870, 9, 0, 0, 0, 0.4);
    box1 = boxCreate(70, 80, 300, 245);
    box2 = boxCreate(470, 180, 150, 205);
    box3 = boxCreate(520, 220, 160, 220);
    box4 = boxCreate(570, 260, 160, 220);
    boxa = boxaCreate(3);
    boxaAddBox(boxa, box2, L_INSERT);
    boxaAddBox(boxa, box3, L_INSERT);
    boxaAddBox(boxa, box4, L_INSERT);
    pixRenderBoxArb(pix1, box1, 3, 200, 200, 25);
    pixRenderBoxaBlend(pix1, boxa, 17, 200, 200, 25, 0.4, 1);
    pta = ptaCreate(5);
    ptaAddPt(pta, 250, 300);
    ptaAddPt(pta, 350, 450);
    ptaAddPt(pta, 400, 600);
    ptaAddPt(pta, 212, 512);
    ptaAddPt(pta, 180, 375);
    pixRenderPolylineBlend(pix1, pta, 17, 25, 200, 200, 0.5, 1, 1);
    pixWrite(fileout, pix1, IFF_JFIF_JPEG);
    pixDisplay(pix1, 200, 200);

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    boxDestroy(&box1);
    boxaDestroy(&boxa);
    ptaDestroy(&pta);
    pixDestroy(&pixs);
    return 0;
}


