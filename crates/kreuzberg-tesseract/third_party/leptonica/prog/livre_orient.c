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
 * livre_orient.c
 *
 *    This generates an image of the set of 4 HMT Sels that are
 *    used for counting ascenders and descenders to detect
 *    text orientation.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const char *textsel1 = "x  oo "
                              "x oOo "
                              "x  o  "
                              "x     "
                              "xxxxxx";

static const char *textsel2 = " oo  x"
                              " oOo x"
                              "  o  x"
                              "     x"
                              "xxxxxx";

static const char *textsel3 = "xxxxxx"
                              "x     "
                              "x  o  "
                              "x oOo "
                              "x  oo ";

static const char *textsel4 = "xxxxxx"
                              "     x"
                              "  o  x"
                              " oOo x"
                              " oo  x";

int main(int    argc,
         char **argv)
{
PIX   *pix1;
SEL   *sel1, *sel2, *sel3, *sel4;
SELA  *sela;

    setLeptDebugOK(1);
    lept_mkdir("lept/livre");

    sel1 = selCreateFromString(textsel1, 5, 6, NULL);
    sel2 = selCreateFromString(textsel2, 5, 6, NULL);
    sel3 = selCreateFromString(textsel3, 5, 6, NULL);
    sel4 = selCreateFromString(textsel4, 5, 6, NULL);

    sela = selaCreate(4);
    selaAddSel(sela, sel1, "textsel1", L_INSERT);
    selaAddSel(sela, sel2, "textsel2", L_INSERT);
    selaAddSel(sela, sel3, "textsel3", L_INSERT);
    selaAddSel(sela, sel4, "textsel4", L_INSERT);

    pix1 = selaDisplayInPix(sela, 28, 3, 30, 4);
    pixWrite("/tmp/lept/livre/orient.png", pix1, IFF_PNG);
    pixDisplay(pix1, 1200, 1200);

    pixDestroy(&pix1);
    selaDestroy(&sela);
    return 0;
}

