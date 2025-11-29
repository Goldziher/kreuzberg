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
 * recog_bootnum2.c
 *
 *   This shows how to convert from a pixa of digit images to
 *   a very compressed representation, including a filtering step
 *   where selected pix are removed.  This method was used to
 *   generate the recog/digits/digit*.comp.tif image mosaics.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

static const char *removeset = "4,7,9,21";

void ProcessDigits(l_int32 i);
void PixaDisplayNumbered(PIXA *pixa, const char *rootname);

l_int32 main(int    argc,
             char **argv)
{
    setLeptDebugOK(1);
    lept_mkdir("lept/digit");
    ProcessDigits(5);
    return 0;
}

/* ----------------------------------------------------- */
void ProcessDigits(l_int32  index)
{
char       rootname[8] = "digit5";
char       buf[64];
l_int32    i, nc, ns, same;
NUMA      *na1;
PIX       *pix1, *pix2, *pix3, *pix4, *pix5, *pix6;
PIXA      *pixa1, *pixa2, *pixa3;

        /* Read the unfiltered, unscaled pixa of twenty-five 5s */
    snprintf(buf, sizeof(buf), "digits/%s.orig-25.pa", rootname);
    pixa1 = pixaRead(buf);

        /* Number and show the input images */
    snprintf(buf, sizeof(buf), "/tmp/lept/digit/%s.orig-num", rootname);
    PixaDisplayNumbered(pixa1, buf);

        /* Remove some of them */
    na1 = numaCreateFromString(removeset);
    pixaRemoveSelected(pixa1, na1);
    numaDestroy(&na1);
    snprintf(buf, sizeof(buf), "/tmp/lept/digit/%s.filt.pa", rootname);
    pixaWrite(buf, pixa1);

        /* Number and show the filtered images */
    snprintf(buf, sizeof(buf), "/tmp/lept/digit/%s.filt-num", rootname);
    PixaDisplayNumbered(pixa1, buf);

        /* Extract the largest c.c., clip to the foreground,
         * and scale the result to a fixed size. */
    nc = pixaGetCount(pixa1);
    pixa2 = pixaCreate(nc);
    for (i = 0; i < nc; i++) {
        pix1 = pixaGetPix(pixa1, i, L_CLONE);
            /* A threshold of 140 gives reasonable results */
        pix2 = pixThresholdToBinary(pix1, 140);
            /* Join nearly touching pieces */
        pix3 = pixCloseSafeBrick(NULL, pix2, 5, 5);
            /* Take the largest (by area) connected component */
        pix4 = pixFilterComponentBySize(pix3, 0, L_SELECT_BY_AREA, 8, NULL);
            /* Extract the original 1 bpp pixels that have been
             * covered by the closing operation */
        pixAnd(pix4, pix4, pix2);
            /* Grab the result as an image with no surrounding whitespace */
        pixClipToForeground(pix4, &pix5, NULL);
            /* Rescale the result to the canonical size */
        pix6 = pixScaleToSize(pix5, 20, 30);
        pixaAddPix(pixa2, pix6, L_INSERT);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
        pixDestroy(&pix4);
        pixDestroy(&pix5);
    }

        /* Add the index (a "5") in the text field of each pix; save pixa2 */
    snprintf(buf, sizeof(buf), "%d", index);
    for (i = 0; i < nc; i++) {
        pix1 = pixaGetPix(pixa2, i, L_CLONE);
        pixSetText(pix1, buf);
        pixDestroy(&pix1);
    }
    snprintf(buf, sizeof(buf), "/tmp/lept/digit/%s.comp.pa", rootname);
    pixaWrite(buf, pixa2);

        /* Number and show the resulting binary templates */
    snprintf(buf, sizeof(buf), "/tmp/lept/digit/%s.comp-num", rootname);
    PixaDisplayNumbered(pixa2, buf);

        /* Save the binary templates as a packed tiling (tiff g4).
         * This is the most efficient way to represent the templates. */
    pix1 = pixaDisplayOnLattice(pixa2, 20, 30, NULL, NULL);
    pixDisplay(pix1, 1000, 500);
    snprintf(buf, sizeof(buf), "/tmp/lept/digit/%s.comp.tif", rootname);
    pixWrite(buf, pix1, IFF_TIFF_G4);

        /* The number of templates is in the pix text string; check it. */
    pix2 = pixRead(buf);
    if (sscanf(pixGetText(pix2), "n = %d", &ns) != 1)
        lept_stderr("Failed to read the number of templates!\n");
    if (ns != nc)
        lept_stderr("(stored = %d) != (actual number = %d)\n", ns, nc);

        /* Reconstruct the pixa of templates from the tiled compressed
         * image, and verify that the resulting pixa is the same.  */
    pixa3 = pixaMakeFromTiledPix(pix1, 20, 30, 0, 0, NULL);
    pixaEqual(pixa2, pixa3, 0, NULL, &same);
    if (!same)
        lept_stderr("Pixa are not the same!\n");

    pixDestroy(&pix1);
    pixDestroy(&pix2);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    pixaDestroy(&pixa3);
}


/* ----------------------------------------------------- */
void PixaDisplayNumbered(PIXA        *pixa,
                         const char  *basename)
{
char     buf[64];
l_int32  fill, color, d;
L_BMF   *bmf;
PIX     *pix1;
PIXA    *pixa1, *pixa2;

    bmf = bmfCreate(NULL, 4);
    pixaGetPixDimensions(pixa, 0, NULL, NULL, &d);
    fill = (d == 8) ? 0xff : 0;
    color = (d == 8) ? 0x00000000 : 0xffffff00;
    pixa1 = pixaAddBorderGeneral(NULL, pixa, 10, 10, 0, 0, fill);
    pixa2 = pixaAddTextNumber(pixa1, bmf, NULL, color, L_ADD_BELOW);
    snprintf(buf, sizeof(buf), "%s.pa", basename);
    pixaWrite(buf, pixa2);
    pix1 = pixaDisplayTiledInColumns(pixa2, 20, 2.5, 15, 2);
    snprintf(buf, sizeof(buf), "%s.png", basename);
    pixWrite(buf, pix1, IFF_PNG);
    pixDisplay(pix1, 500, 500);
    pixDestroy(&pix1);
    pixaDestroy(&pixa1);
    pixaDestroy(&pixa2);
    bmfDestroy(&bmf);
}

