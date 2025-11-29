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
 * fpixcontours.c
 *
 *   Generates and displays an fpix as a set of contours
 *
 *   Syntax: fpixcontours filein [ncontours]
 *   Default for ncontours is 40.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

static const char *fileout = "/tmp/lept/fpix/fpixcontours.png";

int main(int    argc,
         char **argv)
{
char    *filein;
l_int32  ncontours;
FPIX    *fpix;
PIX     *pix;

    if (argc != 2 && argc != 3) {
        lept_stderr("Syntax: fpixcontours filein [ncontours]\n");
        return 1;
    }
    filein = argv[1];
    if (argc == 2)
        ncontours = 40;
    else  /* argc == 3 */
        ncontours = atoi(argv[2]);

    setLeptDebugOK(1);
    lept_mkdir("lept/fpix");

    if ((fpix = fpixRead(filein)) == NULL)
        return ERROR_INT(__func__, "fpix not read", 1);
    pix = fpixAutoRenderContours(fpix, ncontours);
    pixWrite(fileout, pix, IFF_PNG);
    pixDisplay(pix, 100, 100);

    pixDestroy(&pix);
    fpixDestroy(&fpix);
    return 0;
}
