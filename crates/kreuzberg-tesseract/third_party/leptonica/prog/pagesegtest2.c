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
 * pagesegtest2.c
 *
 *   Demonstrates morphological approach to segmenting images.
 *
 *       pagesegtest2 filein fileout
 *
 *   where:
 *       filein: 1, 8 or 32 bpp page image
 *       fileout: photomask for image regions at full resolution
 *
 *   This example shows how to use the morphseq specification of
 *   a sequence of morphological and reduction/expansion operations.
 *
 *   This is much simpler than generating the structuring elements
 *   for the morph operations, specifying each of the function
 *   calls, keeping track of the intermediate images, and removing
 *   them at the end.
 *
 *   The specific sequences below tend to work ok for images scanned at
 *   about 600 ppi.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

    /* Mask at 4x reduction */
static const char *mask_sequence = "r11";
    /* Seed at 4x reduction, formed by doing a 16x reduction,
     * an opening, and finally a 4x replicative expansion. */
static const char *seed_sequence = "r1143 + o5.5+ x4";
    /* Simple dilation */
static const char *dilation_sequence = "d3.3";

#define  DFLAG     1

int main(int    argc,
         char **argv)
{
char    *filein, *fileout;
l_int32  thresh;
PIX     *pixs, *pixg, *pixb;
PIX     *pixmask4, *pixseed4, *pixsf4, *pixd4, *pixd;

    if (argc != 4)
        return ERROR_INT(" Syntax:  pagesegtest2 filein thresh fileout",
                         __func__, 1);
    filein = argv[1];
    thresh = atoi(argv[2]);
    fileout = argv[3];
    setLeptDebugOK(1);

        /* Get a 1 bpp version of the page */
    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    if (pixGetDepth(pixs) == 32)
        pixg = pixConvertRGBToGrayFast(pixs);
    else
        pixg = pixClone(pixs);
    if (pixGetDepth(pixg) == 8)
        pixb = pixThresholdToBinary(pixg, thresh);
    else
        pixb = pixClone(pixg);

        /* Make seed and mask, and fill seed into mask */
    pixseed4 = pixMorphSequence(pixb, seed_sequence, 0);
    pixmask4 = pixMorphSequence(pixb, mask_sequence, 0);
    pixsf4 = pixSeedfillBinary(NULL, pixseed4, pixmask4, 8);
    pixd4 = pixMorphSequence(pixsf4, dilation_sequence, 0);

        /* Mask at full resolution */
    pixd = pixExpandBinaryPower2(pixd4, 4);
    pixWrite(fileout, pixd, IFF_TIFF_G4);

        /* Extract non-image parts (e.g., text) at full resolution */
    pixSubtract(pixb, pixb, pixd);

    pixDisplayWithTitle(pixseed4, 400, 100, "halftone seed", DFLAG);
    pixDisplayWithTitle(pixmask4, 100, 100, "halftone seed mask", DFLAG);
    pixDisplayWithTitle(pixd4, 700, 100, "halftone mask", DFLAG);
    pixDisplayWithTitle(pixb, 1000, 100, "non-halftone", DFLAG);

#if 1
    lept_mkdir("lept/pageseg");
    pixWrite("/tmp/lept/pageseg/seed2", pixseed4, IFF_TIFF_G4);
    pixWrite("/tmp/lept/pageseg/mask2", pixmask4, IFF_TIFF_G4);
    pixWrite("/tmp/lept/pageseg/fill2", pixd4, IFF_TIFF_G4);
    pixWrite("/tmp/lept/pageseg/text2", pixb, IFF_TIFF_G4);
#endif

    pixDestroy(&pixs);
    pixDestroy(&pixg);
    pixDestroy(&pixb);
    pixDestroy(&pixseed4);
    pixDestroy(&pixmask4);
    pixDestroy(&pixsf4);
    pixDestroy(&pixd4);
    pixDestroy(&pixd);
    return 0;
}
