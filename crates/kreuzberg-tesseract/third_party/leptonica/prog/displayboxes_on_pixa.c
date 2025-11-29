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
 * displayboxes_on_pixa.c
 *
 *        displayboxes_on_pixa pixain boxaain type width pixaout display
 *
 *   where 'type' follows the enum in pix.h:
 *          0:  draw red
 *          1:  draw green
 *          2:  draw blue
 *          4:  draw rgb (sequentially)
 *          5:  draw randomly selected colors
 *   and 'display' is a boolean:
 *          0:  no display on screen
 *          1:  display the resulting pixa on the screen, with the images
 *              tiled in rows
 *
 *   This reads a pixa or a pixacomp from file and a boxaa file, draws
 *   the boxes on the appropriate images, and writes the new pixa out.
 *   No scaling is done.
 *
 *   The boxa in the input boxaa should be in 1:1 correspondence with the
 *   pix in the input pixa.  The number of boxes in each boxa is arbitrary.
 *
 *   For example, you can call this with:
 *     displayboxes_on_pixa showboxes.pac showboxes2.baa 4 2 /tmp/result.pa 1
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char    *fileout;
l_int32  width, type, display;
BOXAA   *baa;
PIX     *pix1;
PIXA    *pixa1, *pixa2;

    if (argc != 7) {
        lept_stderr("Syntax error:"
           " displaybaa_on_pixa pixain boxaain type width pixaout display\n");
        return 1;
    }
    setLeptDebugOK(1);

        /* Input file can be either pixa or pixacomp */
    pixa1 = pixaReadBoth(argv[1]);
    baa = boxaaRead(argv[2]);
    type = atoi(argv[3]);
    width = atoi(argv[4]);
    fileout = argv[5];
    display = atoi(argv[6]);

    pixa2 = pixaDisplayBoxaa(pixa1, baa, type, width);
    pixaWrite(fileout, pixa2);

    if (display) {
        pix1 = pixaDisplayTiledInRows(pixa2, 32, 1400, 1.0, 0, 10, 0);
        pixDisplay(pix1, 100, 100);
        pixDestroy(&pix1);
    }

    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    boxaaDestroy(&baa);
    return 0;
}

