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
 * splitpdf.c
 *
 *   Syntax:  splitpdf filein n rootname
 *
 *        n = number of output files
 *        rootname: the root of the output file names, which are in the format:
 *                      [rootname]_001.pdf
 *                      [rootname]_002.pdf
 *                      ...
 *
 *   Notes:
 *   (1) This calls mutool to split the input file into a set of %n files
 *          mutool clean -g -g filein fileout page-range
 *   (2) It attempts to put the same number of pages in each file.
 *   (3) If the number %n of output files specified is greater than
 *       'npages', the number of pages in the input pdf file, this will
 *       write 'npages' files, with one page per file.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char    *filein, *rootname;
char     outname[512], buffer[1024];
l_int32  i, n, npages, nfiles, val, start, end;
NUMA    *naeach;

    if (argc != 4)
        return ERROR_INT(" Syntax:  splitpdf filein n rootname",
                         __func__, 1);
    filein = argv[1];
    n = atoi(argv[2]);
    rootname = argv[3];

    lept_stderr(
         "\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n"
         "   Warning: this program should only be used for testing,\n"
         "     and not in a production environment, because of a\n"
         "      potential vulnerability with the 'system' call.\n"
         "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n\n");

    getPdfPageCount(filein, &npages);
    if (npages == 0) {
        lept_stderr("Page count not found in %s\n", filein);
        return 1;
    }
    lept_stderr("Number of pages in pdf file: %d\n", npages);

        /* Decide how many pages in each output pdf file */
    naeach = numaGetUniformBinSizes(npages, n);
    nfiles = numaGetCount(naeach);  /* actual number of output files */
    lept_stderr("Number of output files: %d\n", nfiles);

        /* Split the pdf and write the files */
    start = 1;
    for (i = 0; i < nfiles; i++) {
        snprintf(outname, sizeof(outname), "%s_%03d.pdf", rootname, i + 1);
        numaGetIValue(naeach, i, &val);
        if (i == 0) {
            lept_stderr("Name of first output file: %s\n", outname);
            lept_stderr("Number of pages in first output file: %d\n", val);
        }
        end = start + val - 1;
        snprintf(buffer, sizeof(buffer),
                 "mutool clean -g -g %s %s %d-%d",
                 filein, outname, start, end);
        callSystemDebug(buffer);
        start = end + 1;
    }

    numaDestroy(&naeach);    
    return 0;
}
