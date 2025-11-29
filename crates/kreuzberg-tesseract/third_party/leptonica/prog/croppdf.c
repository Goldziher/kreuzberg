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
 * croppdf.c
 *
 *    This program concatenates all pdfs in a directory by rendering them
 *    as images, cropping each image to the foreground region with
 *    options for noise removal and margins, slightly thickens long
 *    horizontal lines (e.g., of a music staff), and to some extent
 *    scales the width to fill a printed page.  See documentation for
 *    pixCropImage() for the parameters.
 *
 *    The pdfs are concatenated in lexical order.  Each image is 1 bpp,
 *    rendered at 300 ppi, and encoded with tiffg4.
 *
 *    Syntax:
 *       croppdf basedir lrclear tbclear edgeclean lrborder tbborder
 *       maxwiden printwiden title fileout
 *
 *    Typical parameters for an invocation are:
 *       croppdf . 50 50 0 70 70 1.12 1 none <output-file-name>
 *
 *    Parameter %basedir is a directory where the input pdf files are located.
 *    The program will operate on every file in this directory with
 *    the ".pdf" extension, taking them in lexical order.
 *
 *    Parameter %lrclear and %tbclear parameters give the width of the
 *    regions at the left-right and top-bottom edges of the input image
 *    that are cleared to background as first step in the processing.
 *
 *    The %edgeclean parameter is used to remove noise that is typically
 *    near the edges of the image:
 *      -2: to extract page embedded in black background
 *      -1: aggressively removes left and right side noise
 *       0: default, no removal; use for orthographically produced images
 *       1-15: removal of random noise, where 15 is maximally aggressive
 *
 *    The suggested value for %lrborder and %tbborder is 70.
 *    Laser printers do not print foreground pixels very close to the
 *    page edges, and using a margin of 70 pixels (about 1/4" at 300 ppi)
 *    will allow all foregrounnd pixels to be printed.
 *
 *    The %maxwiden parameter allows the foreground to better fill an
 *    8.5 x 11 inch printed page.  It gives the maximum fractional horizontal
 *    stretching allowed.  Suggested values are between 1.0 and 1.15.
 *
 *    If you are not concerned with printing on paper, use the default
 *    value 0 for %printwiden to skip; 1 for 8.5 x 11 paper; 2 for A4.
 *    Widening only takes place if the ratio h/w exceeds the specified paper
 *    size by 3%, and the horizontal scaling factor will not exceed 1.20.
 *
 *    The %title is the title given to the pdf.  Use %title == "none"
 *    to omit the title.
 *
 *    The pdf output is written to %fileout.  It is advisable (but not
 *    required) to have a '.pdf' extension.
 *
 *    The first processing step is render images from the pdf as RGB
 *    at 300 ppi in ppm format, and to seve them in the directory
 *    /tmp/lept/renderpdf/.
 *
 *    We use pdftoppm to render the images at 300 pixels/inch for a
 *    full page.  The renderer uses the mediaboxes to decide how big
 *    to make the images.  If those boxes have values that are too large,
 *    the intermediate ppm images can be very large.  To prevent that,
 *    we compute the resolution to input to pdftoppm that results
 *    in RGB ppm images representing page images  at about 300 ppi.
 *    These images are about 25MB, and are written quickly because
 *    there is no compression.

 *    N.B.  This requires running pdftoppm from the Poppler package
 *          of pdf utilities.  For non-unix systems, this requires
 *          installation of the cygwin Poppler package:
 *       https://cygwin.com/cgi-bin2/package-cat.cgi?file=x86/poppler/
 *              poppler-0.26.5-1
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

l_int32 main(int    argc,
             char **argv)
{
char       buf[256];
char      *basedir, *title, *fileout;
l_int32    lrclear, tbclear, edgeclean, lrborder, tbborder;
l_int32    printwiden, render_res;
l_float32  maxwiden;
SARRAY    *safiles;

    if (argc != 11)
        return ERROR_INT(
            "Syntax: croppdf basedir lrclear tbclear edgeclean "
            "lrborder tbborder maxwiden printwiden title fileout", __func__, 1);
    basedir = argv[1];
    lrclear = atoi(argv[2]);
    tbclear = atoi(argv[3]);
    edgeclean = atoi(argv[4]);
    lrborder = atoi(argv[5]);
    tbborder = atoi(argv[6]);
    maxwiden = atof(argv[7]);
    printwiden = atoi(argv[8]);
    title = argv[9];
    fileout = argv[10];

    setLeptDebugOK(1);

       /* Render all images from pdfs */
    if (l_pdfRenderFiles(basedir, NULL, 300, &safiles))
        return ERROR_INT_1("rendering failed from basedir", basedir,
                           __func__, 1);

        /* Process each image and collect all resulting 1 bpp images
         * in memory.  If n > 200, use pixacomp instead of pixa to
         * store the images before generating the pdf.  */
    lept_stderr("cropping ...\n");
    cropFilesToPdf(safiles, lrclear, tbclear, edgeclean, lrborder, tbborder,
                   maxwiden, printwiden, title, fileout);
    sarrayDestroy(&safiles);
    return 0;
}

