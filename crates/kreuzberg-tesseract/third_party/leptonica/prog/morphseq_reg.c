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
 * morphseq_reg.c
 *
 *    Simple regression test for binary morph sequence (interpreter),
 *    showing display mode and rejection of invalid sequence components.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  SEQUENCE1    "O1.3 + C3.1 + R22 + D2.2 + X4"
#define  SEQUENCE2    "O2.13 + C5.23 + R22 + X4"
#define  SEQUENCE3    "e3.3 + d3.3 + tw5.5"
#define  SEQUENCE4    "O3.3 + C3.3"
#define  SEQUENCE5    "O5.5 + C5.5"
#define  BAD_SEQUENCE  "O1.+D8 + E2.4 + e.4 + r25 + R + R.5 + X + x5 + y7.3"

#define  DISPLAY_SEPARATION   0   /* use 250 to get images displayed */

int main(int    argc,
         char **argv)
{
PIX  *pixs, *pixg, *pixc, *pixd;

    if (argc != 1)
        return ERROR_INT(" Syntax:  morphseq_reg", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept");
    pixs = pixRead("feyn.tif");

        /* 1 bpp */
    pixd = pixMorphSequence(pixs, SEQUENCE1, -1);
    pixDestroy(&pixd);
    pixd = pixMorphSequence(pixs, SEQUENCE1, DISPLAY_SEPARATION);
    pixWrite("/tmp/lept/morphseq1.png", pixd, IFF_PNG);
    pixDestroy(&pixd);

    pixd = pixMorphCompSequence(pixs, SEQUENCE2, -2);
    pixDestroy(&pixd);
    pixd = pixMorphCompSequence(pixs, SEQUENCE2, DISPLAY_SEPARATION);
    pixWrite("/tmp/lept/morphseq2.png", pixd, IFF_PNG);
    pixDestroy(&pixd);

    lept_stderr("\n ------------------ Error messages -----------------\n");
    lept_stderr(" ------------  DWA v23 Sel doesn't exist -----------\n");
    lept_stderr(" ---------------------------------------------------\n");
    pixd = pixMorphSequenceDwa(pixs, SEQUENCE2, -3);
    pixDestroy(&pixd);
    pixd = pixMorphSequenceDwa(pixs, SEQUENCE2, DISPLAY_SEPARATION);
    pixWrite("/tmp/lept/morphseq3.png", pixd, IFF_PNG);
    pixDestroy(&pixd);

    pixd = pixMorphCompSequenceDwa(pixs, SEQUENCE2, -4);
    pixDestroy(&pixd);
    pixd = pixMorphCompSequenceDwa(pixs, SEQUENCE2, DISPLAY_SEPARATION);
    pixWrite("/tmp/lept/morphseq4.png", pixd, IFF_PNG);
    pixDestroy(&pixd);

        /* 8 bpp */
    pixg = pixScaleToGray(pixs, 0.25);
    pixd = pixGrayMorphSequence(pixg, SEQUENCE3, -5, 150);
    pixDestroy(&pixd);
    pixd = pixGrayMorphSequence(pixg, SEQUENCE3, DISPLAY_SEPARATION, 150);
    pixWrite("/tmp/lept/morphseq5.png", pixd, IFF_PNG);
    pixDestroy(&pixd);

    pixd = pixGrayMorphSequence(pixg, SEQUENCE4, -6, 300);
    pixWrite("/tmp/lept/morphseq6.png", pixd, IFF_PNG);
    pixDestroy(&pixd);

        /* 32 bpp */
    pixc = pixRead("wyom.jpg");
    pixd = pixColorMorphSequence(pixc, SEQUENCE5, -7, 150);
    pixDestroy(&pixd);
    pixd = pixColorMorphSequence(pixc, SEQUENCE5, DISPLAY_SEPARATION, 450);
    pixWrite("/tmp/lept/morphseq7.png", pixd, IFF_PNG);
    pixDestroy(&pixc);
    pixDestroy(&pixd);

        /* Syntax error handling */
    lept_stderr("\n ----------------- Error messages ------------------\n");
    lept_stderr(" ---------------- Invalid sequence -----------------\n");
    lept_stderr(" ---------------------------------------------------\n");
    pixd = pixMorphSequence(pixs, BAD_SEQUENCE, 50);  /* fails; returns null */
    pixd = pixGrayMorphSequence(pixg, BAD_SEQUENCE, 50, 0);  /* this fails */

    pixDestroy(&pixg);
    pixDestroy(&pixs);
    return 0;
}
