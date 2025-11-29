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
 * colorsegtest.c
 *
 *   See colorseg.c for details.
 *
 *   Just for fun, try these combinations of the 4 parameters below on
 *   the image tetons.jpg:
 *      30 20 5 10    (20 colors)
 *      40 20 7 15    (19 colors)
 *      50 12 5 12    (12 colors)
 *      50 12 3 12    (12 colors)
 *      30 13 3 13    (12 colors)
 *      30 20 3 20    (20 colors)
 *      15 20 5 15    (19 colors)
 *      80 20 3 20    (12 colors)
 *      100 15 5 15   (7 colors)
 *      100 15 2 15   (7 colors)
 *      100 15 0 15   (7 colors)
 *      30 15 0 15    (12 colors)
 *      150 15 0 15   (4 colors)
 *      150 15 2 15   (4 colors)
 *      180 6 2 6     (3 colors)
 *      180 6 0 6     (3 colors)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_int32    MAX_DIST      = 120;
static const l_int32    MAX_COLORS    = 15;
static const l_int32    SEL_SIZE      = 4;
static const l_int32    FINAL_COLORS  = 15;


int main(int    argc,
         char **argv)
{
l_int32  max_dist, max_colors, sel_size, final_colors;
PIX     *pixs, *pixd, *pixt;
char    *filein, *fileout;

    if (argc != 3 && argc != 7)
        return ERROR_INT(
            "Syntax: colorsegtest filein fileout"
            " [max_dist max_colors sel_size final_colors]\n"
            " Default values are: max_dist = 120\n"
            "                     max_colors = 15\n"
            "                     sel_size = 4\n"
            "                     final_colors = 15\n", __func__, 1);
    filein = argv[1];
    fileout = argv[2];
    if (argc == 3) {  /* use default values */
        max_dist = MAX_DIST;
        max_colors = MAX_COLORS;
        sel_size = SEL_SIZE;
        final_colors = FINAL_COLORS;
    }
    else {  /* 6 input args */
        max_dist = atoi(argv[3]);
        max_colors = atoi(argv[4]);
        sel_size = atoi(argv[5]);
        final_colors = atoi(argv[6]);
    }
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    startTimer();
    pixt = pixRemoveColormap(pixs, REMOVE_CMAP_BASED_ON_SRC);
    pixd = pixColorSegment(pixt, max_dist, max_colors, sel_size,
                           final_colors, 1);
    lept_stderr("Time to segment: %7.3f sec\n", stopTimer());
    pixWrite(fileout, pixd, IFF_PNG);

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}

