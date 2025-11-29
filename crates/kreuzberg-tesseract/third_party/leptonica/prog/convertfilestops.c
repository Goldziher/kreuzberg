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
 * convertfilestops.c
 *
 *    Converts all files in the given directory with matching substring
 *    to a level 3 compressed PostScript file, at the specified resolution.
 *    To convert all files in the directory, use 'allfiles' for the substring.
 *
 *    See below for syntax and usage.
 *
 *    To generate a ps that scales the images to fit a standard 8.5 x 11
 *    page, use res = 0.
 *
 *    Otherwise, this will convert based on a specified input resolution.
 *    Decreasing the input resolution will cause the image to be rendered
 *    larger, and v.v.   For example, if the page was originally scanned
 *    at 400 ppi and you use 300 ppi for the resolution, the page will
 *    be rendered with larger pixels (i.e., be magnified) and you will
 *    lose a quarter of the page on the right side and a quarter
 *    at the bottom.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char    *dirin, *substr, *fileout;
l_int32  res;

    if (argc != 5) {
        lept_stderr(
            " Syntax: convertfilestops dirin substr res fileout\n"
            "     where\n"
            "         dirin:  input directory for image files\n"
            "         substr:  Use 'allfiles' to convert all files\n"
            "                  in the directory.\n"
            "         res:  Input resolution of each image;\n"
            "               assumed to all be the same\n"
            "         fileout:  Output ps file.\n");
        return 1;
    }
    dirin = argv[1];
    substr = argv[2];
    res = atoi(argv[3]);
    fileout = argv[4];

    setLeptDebugOK(1);
    if (!strcmp(substr, "allfiles"))
        substr = NULL;
    if (res != 0)
        return convertFilesToPS(dirin, substr, res, fileout);
    else
        return convertFilesFittedToPS(dirin, substr, 0.0, 0.0, fileout);
}
