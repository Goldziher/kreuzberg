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
 * converttopdf.c
 *
 *    Bundles all image files that are in the designated directory, with
 *    optional matching substring, into a pdf.
 *
 *    Syntax:  converttopdf dir [substring] fileout
 *
 *    The encoding type depends on the input file format:
 *      jpeg     ==>  DCT (not transcoded)
 *      jp2k     ==>  JPX (not transcoded)
 *      tiff-g4  ==>  G4
 *      png      ==>  FLATE (some are not transcoded)
 *    The default resolution is set at 300 ppi if not given in the
 *    individual images, and the images are wrapped at full resolution.
 *    No title is attached.
 *
 *    This is meant for the simplest set of input arguments.  It is
 *    very fast for jpeg, jp2k and some png.
 *    The syntax for using all files in the directory is:
 *         convertopdf <directory> <pdf_outfile>
 *    The syntax using some substring to be matched in the file names is:
 *         converttopdf <directory> <substring> <pdf_outfile>
 *    If you want something more general, use convertfilestopdf.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32  ret;
char    *dirin, *substr, *fileout;

    if (argc != 3 && argc != 4) {
        lept_stderr(
            " Syntax: converttopdf dir [substr] fileout\n"
            "         substr:  Leave this out to bundle all files\n"
            "         fileout:  Output pdf file\n");
        return 1;
    }
    dirin = argv[1];
    substr = (argc == 4) ? argv[2] : NULL;
    fileout = (argc == 4) ? argv[3] : argv[2];

    setLeptDebugOK(1);
    ret = convertUnscaledFilesToPdf(dirin, substr, "", fileout);
    return ret;
}
