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
 * imagetops.c
 *
 *   This generates a PostScript image, optionally rotating and setting
 *   a scaling factor for printing with maximum size on 8.5 x 11
 *   paper at 300 ppi.
 *
 *        Syntax:  imagetops <filein> <level> <fileout>
 *
 *   level (corresponding to PostScript compression level):
 *        1 for uncompressed
 *        2 for compression with g4 for 1 bpp and dct for everything else
 *        3 for compression with flate
 *
 *   The output PostScript file can be printed with lpr or lp.
 *   Examples of the invocation for lp are:
 *       lp -d <printer> <ps-file>
 *       lp -d <printer> -o ColorModel=Color <ps-file>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

static const l_float32  FILL_FACTOR = 0.95;   /* fill factor on 8.5 x 11 page */

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
l_int32    w, h, index, level;
l_float32  scale;
FILE      *fp;
PIX       *pixs, *pix1;

    if (argc != 4)
        return ERROR_INT(
            " Syntax:  imagetops <filein> <compression level> <fileout>",
            __func__, 1);
    filein = argv[1];
    level = atoi(argv[2]);
    fileout = argv[3];

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    if (level < 1 || level > 3)
        return ERROR_INT("valid levels are: 1, 2, 3", __func__, 1);

    pixGetDimensions(pixs, &w, &h, NULL);
    if (w > h) {
        pix1 = pixRotate90(pixs, 1);
        pixGetDimensions(pix1, &w, &h, NULL);
    }
    else {
        pix1 = pixClone(pixs);
    }

    scale = L_MIN(FILL_FACTOR * 2550 / w, FILL_FACTOR * 3300 / h);
    if (level == 1) {
        fp = lept_fopen(fileout, "wb+");
        pixWriteStreamPS(fp, pix1, NULL, 300, scale);
        lept_fclose(fp);
    } else {  /* levels 2 and 3 */
        index = 0;
        pixWriteCompressedToPS(pix1, fileout, (l_int32)(300. / scale),
                               level, &index);
    }

    pixDestroy(&pixs);
    pixDestroy(&pix1);
    return 0;
}
