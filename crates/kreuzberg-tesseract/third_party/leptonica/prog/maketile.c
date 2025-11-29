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
 * maketile.c
 *
 *    Generates a single image tiling of all images of a specific depth
 *    in a directory.  The tiled images are scaled by a specified
 *    isotropic scale factor.  One can also specify the approximate width
 *    of the output image file, and the background color that is between
 *    the tiled images.
 *
 *    Input:  dirin:  directory that has image files
 *            depth (use 32 for RGB)
 *            scale factor
 *            width (approx. width of output tiled image)
 *            background (0 for white, 1 for black)
 *            fileout:  output tiled image file
 *
 *    Note: this program is Unix only; it will not compile under cygwin.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char      *dirin, *fileout, *fname, *fullname;
l_int32    depth, width, background, i, nfiles;
l_float32  scale;
SARRAY    *safiles;
PIX       *pix, *pixt, *pixd;
PIXA      *pixa;

    if (argc != 7)
        return ERROR_INT(
            "Syntax:  maketile dirin depth scale width background fileout",
            __func__, 1);
    dirin = argv[1];
    depth = atoi(argv[2]);
    scale = atof(argv[3]);
    width = atoi(argv[4]);
    background = atoi(argv[5]);
    fileout = argv[6];
    setLeptDebugOK(1);

        /* capture the filenames in the input directory; ignore directories */
    if ((safiles = getFilenamesInDirectory(dirin)) == NULL)
        return ERROR_INT("safiles not made", __func__, 1);

            /* capture images with the requisite depth */
    nfiles = sarrayGetCount(safiles);
    pixa = pixaCreate(nfiles);
    for (i = 0; i < nfiles; i++) {
        fname = sarrayGetString(safiles, i, L_NOCOPY);
        fullname = genPathname(dirin, fname);
        pix = pixRead(fullname);
        lept_free(fullname);
        if (!pix)
            continue;
        if (pixGetDepth(pix) != depth) {
            pixDestroy(&pix);
            continue;
        }
        if (pixGetHeight(pix) > 5000) {
            lept_stderr("%s too tall\n", fname);
            continue;
        }
        pixt = pixScale(pix, scale, scale);
        pixaAddPix(pixa, pixt, L_INSERT);
        pixDestroy(&pix);
/*        lept_stderr("%d..", i); */
    }
    lept_stderr("\n");

        /* tile them */
    pixd = pixaDisplayTiled(pixa, width, background, 15);

    if (depth < 8)
      pixWrite(fileout, pixd, IFF_PNG);
    else
      pixWrite(fileout, pixd, IFF_JFIF_JPEG);

    pixaDestroy(&pixa);
    pixDestroy(&pixd);
    sarrayDestroy(&safiles);
    return 0;
}

