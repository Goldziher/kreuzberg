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
 * scaleimages.c
 *
 *   Syntax:
 *     scaleimages dirin pattern scalefactor dirout rootname [compress_type]
 *
 *        pattern: a filter on the filenames in directory 'dirin'
 *        rootname: the root of the output file names.  For example, if the
 *                  output scaled images have jpeg compression, their file
 *                  names would be:
 *                      [rootname]_001.jpg
 *                      [rootname]_002.jpg
 *                      ...
 *        compress_type: optional argument; use one of the following:
 *                          png, jpg, tiff, tiffg4, pnm, bmp, webp, jp2, gif
 *
 *   Notes:
 *       (1) If the optional 'compress_type' argument is given, this writes
 *           all output images in that format.
 *           Otherwise, each output image is written in the format implied
 *           by the extension of its input filename.
 *
 *   Example usage with jpeg compressed input files in the current directory:
 *   (a) scaleimages . jpg 0.23 /tmp/out file_
 *       Writes jpeg output files: /tmp/out/file_001.jpg, ...
 *   (b) scaleimages . jpg 0.23 /tmp/out file_ png
 *       Writes png output files: /tmp/out/file_001.png, ...
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char       outname[512];
char      *dirin, *pattern, *dirout, *rootname, *comptype, *fname, *extension;
l_int32    i, n, format;
l_float32  scalefactor;
PIX       *pix1, *pix2;
SARRAY    *sa1;

    if (argc != 6 && argc != 7)
        return ERROR_INT(" Syntax:  scaleimages dirin pattern scalefactor"
                         " dirout rootname [compress_type]", __func__, 1);
    dirin = argv[1];
    pattern = argv[2];
    scalefactor = atof(argv[3]);
    dirout = argv[4];
    rootname = argv[5];
	comptype = NULL;
    if (argc == 7)
        comptype = argv[6];

    sa1 = getSortedPathnamesInDirectory(dirin, pattern, 0, 0);
    sarrayWriteStderr(sa1);
    n = sarrayGetCount(sa1);

    for (i = 0; i < n; i++) {
        fname = sarrayGetString(sa1, i, L_NOCOPY);
        if ((pix1 = pixRead(fname)) == NULL) {
            L_ERROR("pix[%d] not read\n", __func__, i);
            continue;
        }

            /* Determine the output compression format */
        if (comptype)
            format = getFormatFromExtension(comptype);
        else
            format = getImpliedFileFormat(fname);
        lept_stderr("fname = %s, format: %d\n", fname, format);

            /* Determine the name of the output scaled image file */
        extension = (char *)getFormatExtension(format);
        snprintf(outname, sizeof(outname), "%s/%s%03d.%s",
                 dirout, rootname, i + 1, extension);
        lept_stderr("Writing %s\n", outname);

            /* Scale and output */
        pix2 = pixScale(pix1, scalefactor, scalefactor);
        pixWrite(outname, pix2, format);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
    }

    sarrayDestroy(&sa1);
    return 0;
}
