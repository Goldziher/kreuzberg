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
 * scaleandtile.c
 *
 *    Generates a single image tiling of all images in a directory
 *    whose filename contains a given substring.  The filenames
 *    are filtered and sorted, and read into a pixa, which is
 *    then tiled into a pix at a specified depth, and finally
 *    written out to file.
 *
 *    Input:  dirin:  directory that has image files
 *            depth (output depth: 1, 8 or 32; use 32 for RGB)
 *            width (of each tile; all pix are scaled to the same width)
 *            ncols (number of tiles in each row)
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

    /* Change these and recompile if necessary */
static const l_int32  BACKGROUND_COLOR = 0;
static const l_int32  SPACING = 25;  /* between images and on outside */
static const l_int32  BLACK_BORDER = 2;  /* surrounding each image */


int main(int    argc,
         char **argv)
{
char    *dirin, *substr, *fileout;
l_int32  depth, width, ncols;
PIX     *pixd;
PIXA    *pixa;

    if (argc != 7)
	return ERROR_INT(
	    "Syntax:  scaleandtile dirin substr depth width ncols fileout",
	    __func__, 1);
    dirin = argv[1];
    substr = argv[2];
    depth = atoi(argv[3]);
    width = atoi(argv[4]);
    ncols = atoi(argv[5]);
    fileout = argv[6];
    setLeptDebugOK(1);

        /* Avoid division by zero if ncols == 0 and require a positive value. */
    if (ncols <= 0)
        return ERROR_INT("Expected a positive value for ncols", __func__, 1);

        /* Read the specified images from file */
    if ((pixa = pixaReadFiles(dirin, substr)) == NULL)
	return ERROR_INT("safiles not made", __func__, 1);
    lept_stderr("Number of pix: %d\n", pixaGetCount(pixa));

    	/* Tile them */
    pixd = pixaDisplayTiledAndScaled(pixa, depth, width, ncols,
                                     BACKGROUND_COLOR, SPACING, BLACK_BORDER);

    if (depth < 8)
        pixWrite(fileout, pixd, IFF_PNG);
    else
        pixWrite(fileout, pixd, IFF_JFIF_JPEG);

    pixaDestroy(&pixa);
    pixDestroy(&pixd);
    return 0;
}


