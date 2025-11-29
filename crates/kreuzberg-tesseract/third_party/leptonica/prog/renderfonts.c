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
 * renderfonts.c
 *
 *     This tests the font rendering functions
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define   DIRECTORY    "./fonts"

int main(int    argc,
         char **argv)
{
char    *textstr;
l_int32  width, wtext, overflow;
L_BMF   *bmf;
PIX     *pixs, *pix;

    if (argc != 1)
        return ERROR_INT("Syntax: renderfonts", __func__, 1);

    setLeptDebugOK(1);
    lept_mkdir("lept/render");

        /* Render a character of text */
    bmf = bmfCreate(DIRECTORY, 20);
    pixs = pixRead("dreyfus8.png");
    lept_stderr("n = %d\n", pixaGetCount(bmf->pixa));
    pix = pixaGetPix(bmf->pixa, 6, L_CLONE);
    pixSetMaskedGeneral(pixs, pix, 0x45, 140, 165);
    pixWrite("/tmp/lept/render/char.png", pixs, IFF_PNG);
    pixDisplay(pixs, 0, 0);
    pixDestroy(&pix);
    pixDestroy(&pixs);
    bmfDestroy(&bmf);

        /* Render a line of text */
    bmf = bmfCreate(DIRECTORY, 8);
    pixs = pixRead("marge.jpg");
    bmfGetStringWidth(bmf, "This is a funny cat!", &width);
    lept_stderr("String width: %d pixels\n", width);

    pixSetTextline(pixs, bmf, "This is a funny cat!", 0x4080ff00, 50, 250,
                   &width, &overflow);
    pixWrite("/tmp/lept/render/line.png", pixs, IFF_JFIF_JPEG);
    pixDisplay(pixs, 450, 0);
    lept_stderr("Text width = %d\n", width);
    if (overflow)
        lept_stderr("Text overflow beyond image boundary\n");
    pixDestroy(&pixs);
    bmfDestroy(&bmf);

        /* Render a block of text */
    bmf = bmfCreate(DIRECTORY, 10);
    pixs = pixRead("marge.jpg");
    textstr = stringNew("This is a cat! This is a funny cat! "
                        "This is a funny funny cat! This is a "
                        "funny funny funny cat!");

    wtext = pixGetWidth(pixs) - 70;
    pixSetTextblock(pixs, bmf, textstr, 0x90804000, 50, 50, wtext,
                    1, &overflow);
    pixWrite("/tmp/lept/render/block.png", pixs, IFF_JFIF_JPEG);
    pixDisplay(pixs, 0, 500);
    if (overflow)
        lept_stderr("Text overflow beyond image boundary\n");
    lept_free(textstr);
    pixDestroy(&pixs);
    bmfDestroy(&bmf);
    return 0;
}

