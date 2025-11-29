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
 * convertformat.c
 *
 *   Converts an image file from one format to another.
 *
 *   Syntax: convertformat filein fileout [format]
 *
 *      where format is one of these:
 *
 *         BMP
 *         JPEG  (only applicable for 8 bpp or rgb; if not, transcode to png)
 *         PNG
 *         TIFF
 *         TIFF_G4  (only applicable for 1 bpp; if not, transcode to png)
 *         PNM
 *         GIF
 *         WEBP
 *         JP2  (only available for 8 bpp or rgb; if not, transcode to png)
 *
 *   The output format can be chosen either explicitly with the %format
 *   arg, or implicitly using the extension of @fileout:
 *
 *         BMP       .bmp
 *         JPEG      .jpg
 *         PNG       .png
 *         TIFF       (zip compressed: use explicitly with %format arg)
 *         TIFFG4    .tif
 *         PNM       .pnm
 *         GIF       .gif
 *         WEBP      .webp
 *         JP2       .jp2
 *
 *   If spp == 4 (rgba), the alpha component is removed, using a white backing.
 *
 *   If the requested output format does not support the image type,
 *   the image is written in png format, with filename extension 'png'.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char        *filein, *fileout, *base, *ext;
const char  *formatstr;
l_int32      format, d, change;
PIX         *pixs, *pix1;

    if (argc != 3 && argc != 4) {
        lept_stderr("Syntax: convertformat filein fileout [format]\n"
                    "Either specify a format from one of these:\n"
                    "  BMP, JPEG, PNG, TIFF, TIFFG4, PNM, GIF, WEBP, JP2\n"
                    "Or specify the extensions to the output file:\n"
                    "  bmp, jpg, png, tif, pnm, gif, webp, jp2\n");
        return 1;
    }
    filein = argv[1];
    fileout = argv[2];

    if (argc == 3) {
        splitPathAtExtension(fileout, NULL, &ext);
        if (!strcmp(ext, ".bmp"))
            format = IFF_BMP;
        else if (!strcmp(ext, ".jpg"))
            format = IFF_JFIF_JPEG;
        else if (!strcmp(ext, ".png"))
            format = IFF_PNG;
        else if (!strcmp(ext, ".tif"))  /* requesting g4-tiff binary comp */
            format = IFF_TIFF_G4;
        else if (!strcmp(ext, ".pnm"))
            format = IFF_PNM;
        else if (!strcmp(ext, ".gif"))
            format = IFF_GIF;
        else if (!strcmp(ext, ".webp"))
            format = IFF_WEBP;
        else if (!strcmp(ext, ".jp2"))
            format = IFF_JP2;
        else {
            return ERROR_INT(
                "Valid extensions: bmp, jpg, png, tif, pnm, gif, webp, jp2",
                __func__, 1);
        }
        lept_free(ext);
    }
    else {
        formatstr = argv[3];
        if (!strcmp(formatstr, "BMP"))
            format = IFF_BMP;
        else if (!strcmp(formatstr, "JPEG"))
            format = IFF_JFIF_JPEG;
        else if (!strcmp(formatstr, "PNG"))
            format = IFF_PNG;
        else if (!strcmp(formatstr, "TIFF"))
            format = IFF_TIFF_ZIP;  /* zip compressed tif */
        else if (!strcmp(formatstr, "TIFFG4"))
            format = IFF_TIFF_G4;
        else if (!strcmp(formatstr, "PNM"))
            format = IFF_PNM;
        else if (!strcmp(formatstr, "GIF"))
            format = IFF_GIF;
        else if (!strcmp(formatstr, "WEBP"))
            format = IFF_WEBP;
        else if (!strcmp(formatstr, "JP2"))
            format = IFF_JP2;
        else {
            return ERROR_INT(
                "Valid formats: BMP, JPEG, PNG, TIFF, TIFFG4, PNM, "
                "GIF, WEBP, JP2",
                __func__, 1);
        }
    }

    setLeptDebugOK(1);
    if ((pixs = pixRead(filein)) == NULL) {
        L_ERROR("read fail for %s\n", __func__, filein);
        return 1;
    }
    pix1 = pixRemoveAlpha(pixs);
    pixDestroy(&pixs);
    pixs = pix1;

        /* Change output format if necessary */
    change = FALSE;
    d = pixGetDepth(pixs);
    if (d != 1 && format == IFF_TIFF_G4) {
        L_WARNING("can't convert to tiff_g4; converting to png\n", __func__);
        change = TRUE;
    }
    if (d < 8) {
        switch(format)
        {
        case IFF_JFIF_JPEG:
            L_WARNING("can't convert to jpeg; converting to png\n", __func__);
            change = TRUE;
            break;
        case IFF_WEBP:
            L_WARNING("can't convert to webp; converting to png\n", __func__);
            change = TRUE;
            break;
        case IFF_JP2:
            L_WARNING("can't convert to jp2; converting to png\n", __func__);
            change = TRUE;
            break;
        }
    }
    if (change) {
        splitPathAtExtension(fileout, &base, &ext);
        fileout = stringJoin(base, ".png");
        format = IFF_PNG;
    }

    pixWrite(fileout, pixs, format);
    return 0;
}
