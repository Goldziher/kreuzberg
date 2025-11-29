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
 * displaypixa.c
 *
 *        displaypixa filein fileout showtext
 *        displaypixa filein scalefact border lossless disp fileout showtext
 *
 *   where:
 *         showtext = 1 to print text in the text field of each pix below
 *                    the image; 0 to skip
 *         disp = 1 to display on the screen; 0 to skip
 *         lossless = 1 for tiff or png
 *
 *   This reads a pixa or a pixacomp from file and generates a composite of the
 *   images tiled in rows.  It also optionally displays on the screen.
 *   No scaling is done if @scalefact == 0.0 or @scalefact == 1.0.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char       buf[32];
char      *fileout, *textstr;
l_int32    n, i, same, maxd, ntext, border, lossless, display, showtext;
l_float32  scalefact;
L_BMF     *bmf;
PIX       *pix1, *pix2, *pix3, *pix4, *pixd;
PIXA      *pixa, *pixad;

    if (argc != 4 && argc != 8) {
        lept_stderr("Syntax error in displaypixa:\n"
                    "   displaypixa filein fileout showtext\n"
                    "   displaypixa filein scalefact border"
                    " lossless disp fileout showtext\n");
         return 1;
    }
    setLeptDebugOK(1);

        /* Input file can be either pixa or pixacomp */
    pixa = pixaReadBoth(argv[1]);
    pixaCountText(pixa, &ntext);

    if (argc == 4) {
        fileout = argv[2];
        showtext = atoi(argv[3]);
    }

        /* Simple specification; no output text */
    if (argc == 4 && (showtext == 0 || ntext == 0)) {  /* no text output */
        pixaVerifyDepth(pixa, &same, &maxd);
        pixd = pixaDisplayTiledInRows(pixa, maxd, 1400, 1.0, 0, 10, 0);
        pixDisplay(pixd, 100, 100);
        if (pixGetDepth(pixd) == 1)
            pixWrite(fileout, pixd, IFF_PNG);
        else
            pixWrite(fileout, pixd, IFF_JFIF_JPEG);
        pixDestroy(&pixd);
        pixaDestroy(&pixa);
        return 0;
    }

        /* Simple specification with output text */
    if (argc == 4) {  /* showtext == 1 && ntext > 0 */
        n = pixaGetCount(pixa);
        bmf = bmfCreate(NULL, 10);
        pixad = pixaCreate(n);
        for (i = 0; i < n; i++) {
            pix1 = pixaGetPix(pixa, i, L_CLONE);
            pix2 = pixConvertTo32(pix1);
            pix3 = pixAddBorderGeneral(pix2, 10, 10, 5, 5, 0xffffff00);
            textstr = pixGetText(pix1);
            if (textstr && strlen(textstr) > 0) {
                snprintf(buf, sizeof(buf), "%s", textstr);
                pix4 = pixAddSingleTextblock(pix3, bmf, buf, 0xff000000,
                                             L_ADD_BELOW, NULL);
            } else {
                pix4 = pixClone(pix3);
            }
            pixaAddPix(pixad, pix4, L_INSERT);
            pixDestroy(&pix1);
            pixDestroy(&pix2);
            pixDestroy(&pix3);
        }
        bmfDestroy(&bmf);
        pixaVerifyDepth(pixad, &same, &maxd);
        pixd = pixaDisplayTiledInRows(pixad, maxd, 1400, 1.0, 0, 10, 0);
        pixDisplay(pixd, 100, 100);
        if (pixGetDepth(pixd) == 1)
            pixWrite(fileout, pixd, IFF_PNG);
        else
            pixWrite(fileout, pixd, IFF_JFIF_JPEG);
        pixDestroy(&pixd);
        pixaDestroy(&pixa);
        pixaDestroy(&pixad);
        return 0;
    }

        /* Full specification */
    scalefact = atof(argv[2]);
    border = atoi(argv[3]);
    lossless = atoi(argv[4]);
    display = atoi(argv[5]);
    fileout = argv[6];
    showtext = atoi(argv[7]);
    if (showtext && ntext == 0)
        L_INFO("No text found in any of the pix\n", __func__);
    bmf = (showtext && ntext > 0) ?  bmfCreate(NULL, 10) : NULL;
    n = pixaGetCount(pixa);
    pixad = pixaCreate(n);
    for (i = 0; i < n; i++) {
        pix1 = pixaGetPix(pixa, i, L_CLONE);
        pix2 = pixConvertTo32(pix1);
        pix3 = pixAddBorderGeneral(pix2, 10, 10, 5, 5, 0xffffff00);
        textstr = pixGetText(pix1);
        if (bmf && textstr && strlen(textstr) > 0) {
            snprintf(buf, sizeof(buf), "%s", textstr);
            pix4 = pixAddSingleTextblock(pix3, bmf, buf, 0xff000000,
                                     L_ADD_BELOW, NULL);
        } else {
            pix4 = pixClone(pix3);
        }
        pixaAddPix(pixad, pix4, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    bmfDestroy(&bmf);

    pixaVerifyDepth(pixad, &same, &maxd);
    pixd = pixaDisplayTiledInRows(pixad, maxd, 1400, scalefact,
                                  0, 10, border);
    if (display) pixDisplay(pixd, 20, 20);
    if (pixGetDepth(pixd) == 1 || lossless)
        pixWrite(fileout, pixd, IFF_PNG);
    else
        pixWrite(fileout, pixd, IFF_JFIF_JPEG);

    pixDestroy(&pixd);
    pixaDestroy(&pixa);
    pixaDestroy(&pixad);
    return 0;
}

