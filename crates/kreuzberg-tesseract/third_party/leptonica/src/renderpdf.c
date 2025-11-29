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

/*!
 * \file renderpdf.c
 * <pre>
 *
 *   Rendering pdf files using an external library
 *        l_int32     l_pdfRenderFile()
 *        l_int32     l_pdfRenderFiles()
 *
 *   Utility for rendering a set of pdf files as page images.
 *   The images are rendered for full page images at a specified
 *   resolution between 50 and 300 ppi, in the directory
 *       /tmp/lept/renderpdf/
 *
 *   An application like cleanpdf performs a sequence of:
 *   (1) rendering the pdfs into a set of images,
 *   (2) doing image processing on each image to generate new images, and
 *   (3) wrapping the new images up in a single pdf file.
 *   Typically, the processed images made by step (2) are stored compressed
 *   in memory in a PixaComp, before wrapping them up in step (3).
 *
 *   This requires the Poppler package of pdf utilities, in particular
 *   the program pdftoppm.  For non-unix systems, this requires
 *   installation of the cygwin Poppler package:
 *      https://cygwin.com/cgi-bin2/package-cat.cgi?file=x86/poppler/
 *            poppler-0.26.5-1
 *
 *   For the rasterizer, use pdftoppm:
 *      pdftoppm -r res fname outroot  ('-r res' renders output at res ppi)
 *   This works on all pdf pages, both wrapped images and pages that
 *   were made orthographically.  The default output resolution for
 *   pdftoppm is 150 ppi, but we typically use 300 ppi.  This makes large
 *   uncompressed RGB image files (e.g., a standard size RGB page image
 *   at 300 ppi is 25 MB), but it is very fast.
 *
 *   The size of the resulting images does not depend on the resolution
 *   of the images stored in the input pdf.  We compute the value of the
 *   resolution parameter (render_res) that when input to pdftoppm
 *   will generate a page-size image (612 x 792 pts) at the requested
 *   output resolution.
 *
 *   We do NOT use pdfimages:
 *      pdfimages -j fname outroot   (-j outputs jpeg if input is dct)
 *   pdfimages only works when all pages are pdf wrappers around images.
 *   Further, in some cases, it scrambles the order of the output pages
 *   and inserts extra images.

 *   By default, this function will not run, because it makes a call
 *   to system(1).  To render pdfs as a set of images in a directory,
 *   three things are required:
 *   (1) To have poppler installed.
 *   (2) To enable debug operations using setLeptDebugOK(1).
 *   (3) To link the functions that generate pdf files in the library
 *       (in pdfio1.c, pdfio2.c).
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

/* --------------------------------------------*/
#if  USE_PDFIO   /* defined in environ.h */
/* --------------------------------------------*/

/*-----------------------------------------------------------------*
 *          Rendering pdf files using an external library          *
 *-----------------------------------------------------------------*/
/*!
 * \brief   l_pdfRenderFile()
 *
 * \param[in]    filename    input pdf file
 * \param[in]    res         output resolution (0, [50 ... 300]) ppi
 * \param[out]   psaout      sarray of filenames of rasterized images
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Wrapper to l_padfRenderFiles() for a single input pdf file.
 * </pre>
 */
l_ok
l_pdfRenderFile(const char  *filename,
                l_int32      res,
                SARRAY     **psaout)
{
l_int32  ret;
SARRAY  *sain;

    if (!psaout)
        return ERROR_INT("&saout not defined", __func__, 1);
    *psaout = NULL;
    if (!filename)
        return ERROR_INT("filename not defined", __func__, 1);

    sain = sarrayCreate(1);
    sarrayAddString(sain, filename, L_COPY);
    ret = l_pdfRenderFiles(NULL, sain, res, psaout);
    sarrayDestroy(&sain);
    return ret;
}


/*!
 * \brief   l_pdfRenderFiles()
 *
 * \param[in]    dir         directory of input pdf files
 * \param[in]    sain        sarray of input pdf filenames
 * \param[in]    res         output resolution (0, [50 ... 300]) ppi
 * \param[out]   psaout      sarray of output filenames of rendered images
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Because this uses the "system" call, it is disabled by default
 *          on all platforms.  It is not supported and therefor3 disabled
 *          on iOS 11.
 *      (2) Input pdf file(s) are specified either by an input directory
 *          or an sarray with the paths.  Use the sarray if it is given;
 *          otherwise, use all files in the directory with extention "pdf",
 *          and name the rendered images in lexical order of the filenames.
 *      (3) The allowed output rendering resolutions are between 50 ppi
 *          and 300 ppi.  Typical resolutions are 150 and 300 ppi.
 *          Default input value of 0 can be used for 300 ppi resolution.
 *      (4) Images are rendered in ppm format in directory /tmp/lept/renderpdf
 *          and named in lexical order of the input filenames.  On invocation,
 *          any existing files in this directory are removed.
 *      (5) This requires pdftoppm from the Poppler package of pdf utilities.
 * </pre>
 */
l_ok
l_pdfRenderFiles(const char  *dir,
                 SARRAY      *sain,
                 l_int32      res,
                 SARRAY     **psaout)
{
char     buf[256];
char    *imagedir, *firstfile, *fname, *basename, *tail;
l_int32  i, nfiles, render_res;
SARRAY  *sa;

    if (!LeptDebugOK) {
        L_INFO("running pdftoppm is disabled; "
               "use setLeptDebugOK(1) to enable\n", __func__);
        return 0;
    }

  #ifdef OS_IOS /* iOS 11 does not support system() */
    return ERROR_INT("iOS 11 does not support system()", __func__, 0);
  #endif /* OS_IOS */

    if (!psaout)
        return ERROR_INT("&saout not defined", __func__, 1);
    *psaout = NULL;
    if (res == 0) res = 300;
    if (res < 50 || res > 300)
        return ERROR_INT("res not in range [50 ... 300]", __func__, 1);
    if (!dir && !sain)
        return ERROR_INT("neither dir or sain are defined", __func__, 1);
    if (sain) {
        sa = sarrayCopy(sain);
    } else {
        sa = getSortedPathnamesInDirectory(dir, "pdf", 0, 0);
        if (!sa)
            return ERROR_INT("no files found in dir", __func__, 1);
    }
    nfiles = sarrayGetCount(sa);

        /* Set up directory for rendered page images. */
    lept_rmdir("lept/renderpdf");
    lept_mkdir("lept/renderpdf");
    imagedir = genPathname("/tmp/lept/renderpdf", NULL);

        /* Figure out the resolution to use with the image renderer.
           This first checks the media box sizes, which give the output
           image size in printer points (1/72 inch).  The largest expected
           output image has a max dimension of about 11 inches, corresponding
           to 792 points.  At a resolution of 300 ppi, the max image size
           is then 3300.  For robustness, use the median of media box sizes.
           If the max dimension of this median is significantly larger than
           792, reduce the input resolution to the renderer. Specifically:
            * Calculate the median of the MediaBox widths and heights.
            * If the max exceeds 850, reduce the resolution so that the max
              dimension of the rendered image is 3300.  The new resolution
              input to the renderer is reduced from 300 by the factor:
                            (792 / medmax)
           If the media boxes are not found, render a page using a small
           given resolution (72) and use the max dimension to find the
           resolution, render_res, that will produce an out with
           3300 pixels in the largest dimension. */
    firstfile = sarrayGetString(sa, 0, L_NOCOPY);
    getPdfRendererResolution(firstfile, imagedir, &render_res);

        /* The input %res gives the actual resolution at which the page is
           to be rendered.  If this is less than 300 ppi, reduce render_res,
           the resolution input to pdftoppm, by the factor:
                         (res / 300)                            */
    render_res = (render_res * res) / 300;

        /* Rasterize: '-r res' renders output at res ppi
         *  pdftoppm -r res fname outroot    */
    for (i = 0; i < nfiles; i++) {
        fname = sarrayGetString(sa, i, L_NOCOPY);
        splitPathAtDirectory(fname, NULL, &tail);
        splitPathAtExtension(tail, &basename, NULL);
        snprintf(buf, sizeof(buf), "pdftoppm -r %d %s %s/%s",
                 render_res, fname, imagedir, basename);
        LEPT_FREE(tail);
        LEPT_FREE(basename);
        lept_stderr("%s\n", buf);
        callSystemDebug(buf);   /* pdftoppm */
    }
    sarrayDestroy(&sa);

        /* Generate the output array of image file names */
    *psaout = getSortedPathnamesInDirectory(imagedir, NULL, 0, 0);
    LEPT_FREE(imagedir);
    return 0;
}


/* --------------------------------------------*/
#endif  /* USE_PDFIO  */
/* --------------------------------------------*/



/* ------------------------------------------------------------------------- *
 *                       Stubs if pdf is not supported                       *
 * ------------------------------------------------------------------------- */

/* -----------------------------------------------------------------*/
#if  !USE_PDFIO
/* -----------------------------------------------------------------*/

l_ok l_pdfRenderFile(const char *filename, l_int32 res, SARRAY **psaout)
{
    return ERROR_INT("function not present", __func__, 1);
}

/* -----------------------------------------------------------*/

l_ok l_pdfRenderFiles(const char *dir, SARRAY *sain, l_int32 res,
                      SARRAY **psaout)
{
    return ERROR_INT("function not present", __func__, 1);
}

/* -----------------------------------------------------------------*/
#endif  /* !USE_PDFIO */
/* -----------------------------------------------------------------*/

