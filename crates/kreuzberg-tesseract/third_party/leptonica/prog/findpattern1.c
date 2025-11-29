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
 * findpattern1.c
 *
 *    findpattern1 filein patternfile fileout
 *
 *    This is setup with input parameters to generate a hit-miss
 *    Sel from 'patternfile' and use it on 'filein' at 300 ppi.
 *    For example, use char.tif of a "c" bitmap, taken from the
 *    the page image feyn.tif:
 *
 *       findpattern1 feyn.tif char.tif /tmp/result.tif
 *
 *    This shows a number of different outputs, including a magnified
 *    image of the Sel superimposed on the "c" bitmap.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "pix_internal.h"

    /* for pixGenerateSelWithRuns() */
static const l_int32  NumHorLines = 11;
static const l_int32  NumVertLines = 8;
static const l_int32  MinRunlength = 1;

    /* for pixDisplayHitMissSel() */
static const l_uint32  HitColor = 0xff880000;
static const l_uint32  MissColor = 0x00ff8800;


int main(int    argc,
         char **argv)
{
char     *filein, *fileout, *patternfile;
l_int32   w, h, i, n;
BOX      *box, *boxe;
BOXA     *boxa1, *boxa2;
PIX      *pixs, *pixp, *pixpe, *pix1, *pix2, *pix3, *pix4, *pixhmt;
PIXCMAP  *cmap;
SEL      *sel_2h, *sel;

    if (argc != 4)
        return ERROR_INT(" Syntax:  findpattern1 filein patternfile fileout",
                         __func__, 1);
    filein = argv[1];
    patternfile = argv[2];
    fileout = argv[3];

    setLeptDebugOK(1);
    lept_mkdir("lept/hmt");

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    if ((pixp = pixRead(patternfile)) == NULL)
        return ERROR_INT("pixp not made", __func__, 1);
    pixGetDimensions(pixp, &w, &h, NULL);

        /* Generate the hit-miss Sel with runs */
    sel = pixGenerateSelWithRuns(pixp, NumHorLines, NumVertLines, 0,
                                MinRunlength, 7, 7, 0, 0, &pixpe);

        /* Display the Sel two ways */
    selWriteStream(stderr, sel);
    pix1 = pixDisplayHitMissSel(pixpe, sel, 9, HitColor, MissColor);
    pixDisplay(pix1, 200, 200);
    pixWrite("/tmp/lept/hmt/pix1.png", pix1, IFF_PNG);

        /* Use the Sel to find all instances in the page */
    startTimer();
    pixhmt = pixHMT(NULL, pixs, sel);
    lept_stderr("Time to find patterns = %7.3f\n", stopTimer());

        /* Small erosion to remove noise; typically not necessary if
         * there are enough elements in the Sel */
    sel_2h = selCreateBrick(1, 2, 0, 0, SEL_HIT);
    pix2 = pixErode(NULL, pixhmt, sel_2h);

        /* Display the result visually by placing the Sel at each
         * location found */
    pix3 = pixDilate(NULL, pix2, sel);
    cmap = pixcmapCreate(1);
    pixcmapAddColor(cmap, 255, 255, 255);
    pixcmapAddColor(cmap, 255, 0, 0);
    pixSetColormap(pix3, cmap);
    pixWrite(fileout, pix3, IFF_PNG);

        /* Display output with a red outline around each located pattern */
    boxa1 = pixConnCompBB(pix2, 8);
    n = boxaGetCount(boxa1);
    boxa2 = boxaCreate(n);
    pix4 = pixConvert1To2Cmap(pixs);
    for (i = 0; i < n; i++) {
        box = boxaGetBox(boxa1, i, L_COPY);
        boxe = boxCreate(box->x - w / 2, box->y - h / 2, w + 4, h + 4);
        boxaAddBox(boxa2, boxe, L_INSERT);
        pixRenderBoxArb(pix4, boxe, 2, 255, 0, 0);
        boxDestroy(&box);
    }
    pixWrite("/tmp/lept/hmt/outline.png", pix4, IFF_PNG);
    boxaWriteStderr(boxa2);

    pixDestroy(&pixs);
    pixDestroy(&pixp);
    pixDestroy(&pixpe);
    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixDestroy(&pix3);
    pixDestroy(&pix4);
    pixDestroy(&pixhmt);
    selDestroy(&sel);
    selDestroy(&sel_2h);
    boxaDestroy(&boxa1);
    boxaDestroy(&boxa2);
    return 0;
}

