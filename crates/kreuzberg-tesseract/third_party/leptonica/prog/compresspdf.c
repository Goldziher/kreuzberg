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
 * compresspdf.c
 *
 *    This program concatenates all pdfs in a directory by rendering them
 *    as images, optionally scaling the images, and generating an output pdf.
 *    The pdfs are taken in lexical order.  Pages are encoded with either
 *    tiffg4 or jpeg (DCT), or a mixture of them depending on input parameters
 *    and page color content.  For DCT encoding, the jpeg quality factor
 *    can be used to trade off the size of the resulting pdf against
 *    the image quality.
 *
 *    If the pages are monochrome (black and white), use of the %onebit
 *    flag will achieve better compression with less distortion.
 *    If most of the pages are black and white, but some have color that
 *    needs to be saved, input parameters %onebit and %savecolor should
 *    be both set to 1.  Then the pages with color are compressed with DCT
 *    and the monochrome pages are compressed with tiffg4.
 *
 *    Scaling of the pdf pages (which are often images) is done in two
 *    steps.  In the first, the pdf is converted to a set of rasterized
 *    images, where the resolution of the rasters %imres can be either
 *    150 ppi or 300 ppi.  In the second step, %scalefactor is used to
 *    scale each of these images down or up, with a maximum upscaling
 *    of 2.0.
 *
 *    If the images in the pdf are low-resolution grayscale, they can be
 *    upscaled 2x and binarized to make a readable and better compressed pdf.
 *    For example, an Internet Archive book pdf with 8 bpp images at a
 *    resolution of about 120 ppi, can be converted to 240 ppi, 1 bpp with
 *    a size reduction of about 40%.  For that, use: %onebit = 1,
 *    savecolor = 1 (if there are color images), scalefactor = 2.0.
 *
 *      Syntax:
 *       compresspdf basedir imres scalefactor onebit savecolor
 *                   quality title fileout
 *
 *    The %basedir is a directory where the input pdf files are located.
 *    The program will operate on every file in this directory with
 *    the ".pdf" extension.
 *
 *    The %imres is the desired resolution of the rasterization from the
 *    pdf page to a page image.  Two choices are allowed: 150 and 300 ppi.
 *    Use 0 for default (150 ppi).  The actual resolution used by the
 *    renderer depends on the page image size and is computed internally.
 *    We limit the maximum resolution to 300 ppi because these images are
 *    RGB uncompressed and are large: 6.3 MB for 150 ppi and 25 MB for 300 ppi.
 *
 *    The %scalefactor is the scaling applied to the rasterized images, in
 *    order to produce the images stored in the output pdf.  To reduce
 *    the size of the generated pdf, a %scalefactor < 1.0 can be used to
 *    downscale the rasterized image.  If %scalefactor = 0.0, the default
 *    value of 1.0 will be used.  The maximum allowed value for %scalefactor
 *    is 2.0.  For normal text on images scanned at 300 ppi, a 2x reduction
 *    (%scalefactor = 0.5) may be satisfactory.  Internally, we compute
 *    an output resolution for the pdf that will cause it to print
 *    11 inches high, based on the height in pixels of the first image
 *    in the set.
 *
 *    As the first step in processing, images are saved in the directory
 *    /tmp/lept/renderpdf/, as RGB in ppm format, and at the resolution
 *    specified by %imres.  If the %onebit flag is 0, these will be
 *    encoded in the output pdf using DCT.  To force the images to be
 *    1 bpp with tiffg4 encoding, use %onebit = 1.
 *
 *    The %savecolor flag is ignored unless %onebit is 1.  In that case,
 *    if %savecolor is 1, each image is tested for color content, and if
 *    even a relatively small amount is found, the image will be encoded
 *    with DCT instead of tiffg4.
 *
 *    The %quality is the jpeg output quality factor for images stored
 *    with DCT encoding in the pdf.  Use 0 for the default value (50),
 *    which is satisfactory for many purposes.  Use 75 for standard
 *    jpeq quality; 85-95 is very high quality.  Allowed values are
 *    between 25 and 95.
 *
 *    The %title is the title given to the pdf.  Use %title == "none"
 *    to omit the title.
 *
 *    The pdf output is written to %fileout.  It is advisable (but not
 *    required) to have a '.pdf' extension.
 *
 *    As the first step in processing, images are saved in the directory
 *    /tmp/lept/renderpdf/, as RGB at either 150 or 300 ppi in ppm format.
 *    Each image about 6MB at 150 ppi, or 25MB at 300 ppi.
 *
 *    We use pdftoppm to render the images at (typically) 150 pixels/inch
 *    for a full page, when scalefactor = 1.0.  The renderer uses the
 *    mediaboxes to decide how big to make the images.  If those boxes
 *    have values that are too large, the intermediate ppm images can
 *    be very large.  To prevent that, we compute the resolution to input
 *    to pdftoppm that results in RGB ppm images representing page images
 *    at about 150 ppi (when scalefactor = 1.0).  These images are about
 *    6MB, but are written quickly because there is no compression.
 *
 *    N.B.  This requires running pdftoppm from the Poppler package
 *          of pdf utilities  For non-unix systems, this requires
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
l_int32    imres, render_res, onebit, savecolor, quality;
l_float32  scalefactor;
SARRAY    *safiles;

    if (argc != 9)
        return ERROR_INT(
            "Syntax: compresspdf basedir imres scalefactor "
                     "onebit savecolor quality title fileout", __func__, 1);
    basedir = argv[1];
    imres = atoi(argv[2]);
    scalefactor = atof(argv[3]);
    onebit = atoi(argv[4]);  /* set to 1 to enforce 1 bpp tiffg4 encoding */
    savecolor = atoi(argv[5]);  /* if onebit == 1, set to 1 to save color */
    quality = atoi(argv[6]);  /* jpeg quality */
    title = argv[7];
    fileout = argv[8];
    if (imres <= 0) imres = 150;  /* default value */
    if (imres != 150 && imres != 300) {
        L_WARNING("imres = %d must be 150 or 300; setting to 150\n",
                  __func__, imres);
        imres = 150;
    }
    if (scalefactor <= 0.0) scalefactor = 1.0;
    if (scalefactor > 2.0) {
        L_WARNING("scalefactor %f too big; setting to 2.0\n", __func__,
                  scalefactor);
        scalefactor = 2.0;
    }
    if (quality <= 0) quality = 50;  /* default value */
    if (quality < 25) {
        L_WARNING("quality = %d is too low; setting to 25\n",
                  __func__, quality);
        quality = 25;
    }
    if (quality > 95) {
        L_WARNING("quality = %d is too high; setting to 95\n",
                  __func__, quality);
        quality = 95;
    }
    setLeptDebugOK(1);

        /* Render all images from pdfs */
    if (l_pdfRenderFiles(basedir, NULL, imres, &safiles))
        return ERROR_INT_1("rendering failed from basedir", basedir,
                           __func__, 1);

        /* Optionally binarize, then scale and collect all images in memory.
         * If n > 100, use pixacomp instead of pixa to store everything
         * before generating the pdf.
         * When using the onebit option, It is important to binarize
         * the images in leptonica.  We do not let 'pdftoppm -mono' do
         * the binarization, because it will apply error-diffusion
         * dithering to gray and color images. */
    lept_stderr("compressing ...\n");
    compressFilesToPdf(safiles, onebit, savecolor, scalefactor, quality,
                       title, fileout);
    sarrayDestroy(&safiles);
    return 0;
}
