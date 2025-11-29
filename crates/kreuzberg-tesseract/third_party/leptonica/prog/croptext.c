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
 *  croptext.c
 *
 *     Simple program that crops text pages to a given border
 *
 *     Syntax:
 *         croptext dirin border dirout
 *     where
 *         border = number of pixels added on each side (e.g., 50)
 *
 *     The output file name has the same tail as the input file name.
 *     If dirout is the same as dirin, you overwrite the input files.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char    *dirin, *dirout, *infile, *outfile, *tail;
l_int32  i, nfiles, border, x, y, w, h, xb, yb, wb, hb;
BOX     *box1, *box2;
BOXA    *boxa1, *boxa2;
PIX     *pixs, *pixt1, *pixd;
SARRAY  *safiles;

    if (argc != 4)
        return ERROR_INT("Syntax: croptext dirin border dirout", __func__, 1);
    dirin = argv[1];
    border = atoi(argv[2]);
    dirout = argv[3];

    setLeptDebugOK(1);
    safiles = getSortedPathnamesInDirectory(dirin, NULL, 0, 0);
    nfiles = sarrayGetCount(safiles);

    for (i = 0; i < nfiles; i++) {
        infile = sarrayGetString(safiles, i, L_NOCOPY);
        splitPathAtDirectory(infile, NULL, &tail);
        outfile = genPathname(dirout, tail);
        pixs = pixRead(infile);
        pixt1 = pixMorphSequence(pixs, "r11 + c10.40 + o5.5 + x4", 0);
        boxa1 = pixConnComp(pixt1, NULL, 8);
        if (boxaGetCount(boxa1) == 0) {
            lept_stderr("Warning: no components on page %s\n", tail);
            continue;
        }
        boxa2 = boxaSort(boxa1, L_SORT_BY_AREA, L_SORT_DECREASING, NULL);
        box1 = boxaGetBox(boxa2, 0, L_CLONE);
        boxGetGeometry(box1, &x, &y, &w, &h);
        xb = L_MAX(0, x - border);
        yb = L_MAX(0, y - border);
        wb = w + 2 * border;
        hb = h + 2 * border;
        box2 = boxCreate(xb, yb, wb, hb);
        pixd = pixClipRectangle(pixs, box2, NULL);
        pixWrite(outfile, pixd, IFF_TIFF_G4);

        pixDestroy(&pixs);
        pixDestroy(&pixt1);
        pixDestroy(&pixd);
        boxaDestroy(&boxa1);
        boxaDestroy(&boxa2);
    }

    return 0;
}

