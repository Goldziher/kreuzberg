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
 *  recog_bootnum3.c
 *
 *  This does two things.
 *
 * (1) It makes recog/digits/bootnum4.pa, a pixa of 100 samples
 *     from each of the 10 digits.  These are stored as 10 mosaics
 *     where the 100 samples are packed in 20x30 pixel tiles.
 *
 * (2) It generates the code that is able to generate a pixa with
 *     any number from 1 to 100 of samples for each digit.  This
 *     new pixa has one pix for each sample (the tiled pix in the
 *     input pixa have been split out), so it can have up to 1000 pix.
 *     The compressed string of data and the code for deserializing
 *     it are auto-generated with the stringcode utility.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
char        buf[64];
l_int32     i;
PIX        *pix1, *pix2;
PIXA       *pixa1, *pixa2;
L_STRCODE  *strc;

    if (argc != 1) {
        lept_stderr(" Syntax: recog_bootnum3\n");
        return 1;
    }

    setLeptDebugOK(1);
    lept_mkdir("lept/digit");

        /* Make a pixa of the first 100 samples for each digit.
         * This will be saved to recog/digits/bootnum4.pa.  */
    pixa1 = pixaCreate(10);
    for (i = 0; i < 10; i++) {
        snprintf(buf, sizeof(buf), "recog/digits/digit%d.comp.tif", i);
        pix1 = pixRead(buf);
        pixa2 = pixaMakeFromTiledPix(pix1, 20, 30, 0, 100, NULL);
        pix2 = pixaDisplayOnLattice(pixa2, 20, 30, NULL, NULL);
        pixaAddPix(pixa1, pix2, L_INSERT);
        pixDestroy(&pix1);
        pixaDestroy(&pixa2);
    }
        /* Write it out (and copy to recog/digits/bootnum4.pa) */
    pixaWrite("/tmp/lept/digit/bootnum4.pa", pixa1);
    pixaDestroy(&pixa1);

        /* Generate the stringcode in two files for this pixa.
         * Both files are then assempled into the source file
         * bootnumgen4.c, which is compiled into the library.  */
    strc = strcodeCreate(212);   // arbitrary integer
    strcodeGenerate(strc, "/tmp/lept/digit/bootnum4.pa", "PIXA");
    strcodeFinalize(&strc, ".");
    return 0;
}
