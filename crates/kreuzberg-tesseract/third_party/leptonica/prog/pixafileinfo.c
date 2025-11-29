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
 * pixafileinfo.c
 *
 *   Returns information about the images in the pixa or pixacomp file
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char     buf[64];
char    *sn;
l_int32  i, n;
PIX     *pix;
PIXA    *pixa;
PIXAC   *pac;
char    *filein;

    if (argc != 2)
        return ERROR_INT(" Syntax:  pixafileinfo filein", __func__, 1);
    setLeptDebugOK(1);

        /* Input file can be either pixa or pixacomp */
    filein = argv[1];
    l_getStructStrFromFile(filein, L_STR_NAME, &sn);
    if (strcmp(sn, "Pixa") == 0) {
        if ((pixa = pixaRead(filein)) == NULL)
            return ERROR_INT("pixa not made", __func__, 1);
    } else if (strcmp(sn, "Pixacomp") == 0) {
        if ((pac = pixacompRead(filein)) == NULL)
            return ERROR_INT("pac not made", __func__, 1);
        pixa = pixaCreateFromPixacomp(pac, L_COPY);
        pixacompDestroy(&pac);
    } else {
        return ERROR_INT("invalid file type", __func__, 1);
    }

    n = pixaGetCount(pixa);
    for (i = 0; i < n; i++) {
        pix = pixaGetPix(pixa, i, L_CLONE);
        snprintf(buf, sizeof(buf), "Pix(%d)", i);
        pixPrintStreamInfo(stderr, pix, buf);
        pixDestroy(&pix);
        lept_stderr("=================================\n");
    }

    pixaDestroy(&pixa);
    return 0;
}
