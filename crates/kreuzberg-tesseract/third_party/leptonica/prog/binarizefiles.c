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
 * binarizefiles.c
 *
 *    Program that optionally scales and then binarizes a set of files,
 *    writing them to the specified directory in tiff-g4 format.
 *    The resolution is preserved.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include <sys/stat.h>
#include <sys/types.h>
#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
char       buf[512], dirname[256];
char      *dirin, *pattern, *subdirout, *fname, *tail, *basename;
l_int32    thresh, i, n;
l_float32  scalefactor;
PIX       *pix1, *pix2, *pix3, *pix4;
SARRAY    *sa;

    if (argc != 6) {
        lept_stderr(
            "Syntax: binarizefiles dirin pattern thresh scalefact dirout\n"
            "      dirin: input directory for image files\n"
            "      pattern: use 'allfiles' to convert all files\n"
            "               in the directory\n"
            "      thresh: 0 for adaptive; > 0 for global thresh (e.g., 128)\n"
            "      scalefactor: in (0.0 ... 4.0]; use 1.0 to prevent scaling\n"
            "      subdirout: subdirectory of /tmp for output files\n");
        return 1;
    }
    dirin = argv[1];
    pattern = argv[2];
    thresh = atoi(argv[3]);
    scalefactor = atof(argv[4]);
    subdirout = argv[5];
    if (!strcmp(pattern, "allfiles"))
              pattern = NULL;
    if (scalefactor <= 0.0 || scalefactor > 4.0) {
        L_WARNING("invalid scalefactor: setting to 1.0\n", __func__);
        scalefactor = 1.0;
    }

    setLeptDebugOK(1);

        /* Get the input filenames */
    sa = getSortedPathnamesInDirectory(dirin, pattern, 0, 0);
    sarrayWriteStderr(sa);
    n = sarrayGetCount(sa);

        /* Write the output files */
    makeTempDirname(dirname, 256, subdirout);
    lept_stderr("dirname: %s\n", dirname);
    lept_rmdir(subdirout);
    lept_mkdir(subdirout);
    for (i = 0; i < n; i++) {
        fname = sarrayGetString(sa, i, L_NOCOPY);
        if ((pix1 = pixRead(fname)) == NULL) {
            L_ERROR("file %s not read as image", __func__, fname);
            continue;
        }
        splitPathAtDirectory(fname, NULL, &tail);
        splitPathAtExtension(tail, &basename, NULL);
        snprintf(buf, sizeof(buf), "%s/%s.tif", dirname, basename);
        lept_free(tail);
        lept_free(basename);
        lept_stderr("fileout: %s\n", buf);
        if (scalefactor != 1.0)
            pix2 = pixScale(pix1, scalefactor, scalefactor);
        else
            pix2 = pixClone(pix1);
        if (thresh == 0) {
            pix4 = pixConvertTo8(pix2, 0);
            pix3 = pixAdaptThresholdToBinary(pix4, NULL, 1.0);
            pixDestroy(&pix4);
        } else {
            pix3 = pixConvertTo1(pix2, thresh);
        }
        pixWrite(buf, pix3, IFF_TIFF_G4);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        pixDestroy(&pix3);
    }
    sarrayDestroy(&sa);
    return 0;
}
