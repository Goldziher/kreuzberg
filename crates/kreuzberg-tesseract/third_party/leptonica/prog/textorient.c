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
 * textorient.c
 *
 *   This attempts to identify the orientation of text in the image.
 *   If text is found, it is rotated by a multiple of 90 degrees
 *   to make it right-side up.  It is not further deskewed.
 *   This works for roman mixed-case text.  It will not work if the
 *   image has all caps or all numbers.  It has not been tested on
 *   other scripts.

 *   Usage:
 *     textorient filein minupconf minratio fileout
 *
 *   You can use minupconf = 0.0, minratio = 0.0 for default values,
 *   which are:
 *       minupconf = 8.0, minratio = 2.5
 *   fileout is the output file name, without the extension, which is
 *   added here depending on the encoding chosen for the output pix.
 *
 *   Example on 1 bpp image:
 *     textorient feyn.tif 0.0 0.0 feyn.oriented
 *   which generates the file
 *     feyn.oriented.tif
 *
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define BUF_SIZE 512

LEPT_DLL extern const char *ImageFileFormatExtensions[];

int main(int    argc,
         char **argv)
{
char         buf[BUF_SIZE];
const char  *filein, *fileout;
l_int32      pixformat;
l_float32    minupconf, minratio;
PIX         *pixs, *pixd;

    if (argc != 5) {
        return ERROR_INT(
            "Syntax:  textorient filein minupconf minratio, fileout",
             __func__, 1);
    }
    filein = argv[1];
    minupconf = atof(argv[2]);
    minratio = atof(argv[3]);
    fileout = argv[4];
    setLeptDebugOK(1);

    pixs = pixRead(filein);
    pixd = pixOrientCorrect(pixs, minupconf, minratio, NULL, NULL, NULL, 1);

    pixformat = pixChooseOutputFormat(pixd);
    snprintf(buf, BUF_SIZE, "%s.%s", fileout,
             ImageFileFormatExtensions[pixformat]);
    pixWrite(buf, pixd, pixformat);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}

