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
 * convertsegfilestopdf.c
 *
 *    Converts all image files in the page directory with matching substring
 *    to a pdf.  Image regions are downscaled by the scalefactor and
 *    encoded as jpeg.  Non-image regions with depth > 1 are automatically
 *    scaled up by 2x and thresholded if the encoding type is G4;
 *    otherwise, no scaling is performed on them.  To convert all
 *    files in the page directory, use 'allfiles' for its substring.
 *    Likewise to use all files in the mask directory, use 'allfiles'
 *    for its substring.
 *
 *    A typical invocation would be something like:
 *       convertsegfilestopdf /tmp/segpages allfiles /tmp/segmasks allfiles \
 *       300 2 160 skip 0.5 [title] [output pdf]
 *    This upscales by 2x all non-image regions to 600 ppi, and downscales
 *    by 0.5 all image regions to 150 ppi.
 *
 *    If used on a set of images without segmentation data, a typical
 *    invocation would be:
 *       convertsegfilestopdf /tmp/pages allfiles skip skip \
 *       300 2 160 skip 1.0 [title] [output pdf]
 *    If the page images have depth > 1 bpp, this will upscale all pages
 *    by 2x (to 600 ppi), and then convert the images to 1 bpp.
 *    Note that 'skip' is used three times to omit all segmentation data.
 *
 *    See below for further syntax and usage.
 *
 *    Again, note that the image regions are displayed at a resolution
 *    that depends on the input resolution (res) and the scaling factor
 *    (scalefact) that is applied to the images before conversion to pdf.
 *    Internally we multiply these, so that the generated pdf will render
 *    at the same resolution as if it hadn't been scaled.  When we
 *    downscale the image regions, this:
 *       (1) reduces the size of the images.  For jpeg, downscaling
 *           reduces by square of the scale factor the 'image' segmented part.
 *       (2) regenerates the jpeg with quality = 75 after downscaling.
 *
 *    If you already have a boxaafile of the image regions, use 'skip' for
 *    maskdir.  Otherwise, this will generate the boxaa from the mask images.
 *
 *    A regression test that uses this is pdfseg_reg, which
 *    generates images and the boxaa file in /tmp/segtest/.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char      *pagedir, *pagesubstr, *maskdir, *masksubstr;
char      *title, *fileout, *boxaafile, *boxaapath;
l_int32    ret, res, type, thresh;
l_float32  scalefactor;
BOXAA     *baa;

    if (argc != 12) {
        lept_stderr(
	    " Syntax: convertsegfilestopdf dirin substr res type thresh \\ \n"
            "                       boxaafile scalefactor title fileout\n"
            "     where\n"
            "         pagedir:  input directory for image files\n"
            "         pagesubstr:  Use 'allfiles' to convert all files\n"
            "                  in the directory\n"
            "         maskdir:  input directory for mask files;\n"
            "                   use 'skip' to skip \n"
            "         masksubstr:  Use 'allfiles' to convert all files\n"
            "                  in the directory; 'skip' to skip\n"
            "         res:  Input resolution of each image;\n"
            "               assumed to all be the same\n"
            "         type: compression used for non-image regions:\n"
            "               0: default (G4 encoding)\n"
            "               1: JPEG encoding\n"
            "               2: G4 encoding\n"
            "               3: PNG encoding\n"
            "         thresh:  threshold for binarization; use 0 for default\n"
            "         boxaafile: Optional file of 'image' regions within\n"
            "                    each page.  This contains a boxa for each\n"
            "                    page, consisting of a set of regions.\n"
            "                    Use 'skip' to skip.\n"
            "         scalefactor:  Use to scale down the image regions\n"
            "         title:  Use 'none' to omit\n"
            "         fileout:  Output pdf file\n");
        return 1;
    }
    pagedir = argv[1];
    pagesubstr = argv[2];
    maskdir = argv[3];
    masksubstr = argv[4];
    res = atoi(argv[5]);
    type = atoi(argv[6]);
    thresh = atoi(argv[7]);
    boxaafile = argv[8];
    scalefactor = atof(argv[9]);
    title = argv[10];
    fileout = argv[11];

    if (!strcmp(pagesubstr, "allfiles"))
        pagesubstr = NULL;
    if (!strcmp(maskdir, "skip"))
        maskdir = NULL;
    if (!strcmp(masksubstr, "allfiles"))
        masksubstr = NULL;
    if (scalefactor <= 0.0 || scalefactor > 1.0) {
        L_WARNING("invalid scalefactor: setting to 1.0\n", __func__);
        scalefactor = 1.0;
    }
    if (type != 1 && type != 2 && type != 3)
        type = L_G4_ENCODE;
    if (thresh <= 0)
        thresh = 150;
    if (!strcmp(title, "none"))
        title = NULL;

    setLeptDebugOK(1);
    if (maskdir)  /* use this; ignore any input boxaafile */
        baa = convertNumberedMasksToBoxaa(maskdir, masksubstr, 0, 0);
    else if (strcmp(boxaafile, "skip") != 0) {  /* use the boxaafile */
        boxaapath = genPathname(boxaafile, NULL);
        baa = boxaaRead(boxaapath);
        lept_free(boxaapath);
    }
    else  /* no maskdir and no input boxaafile */
        baa = NULL;

    ret = convertSegmentedFilesToPdf(pagedir, pagesubstr, res, type, thresh,
                                     baa, 75, scalefactor, title, fileout);
    boxaaDestroy(&baa);
    return ret;
}
