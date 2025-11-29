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
 * htmlviewer.c
 *
 *    This takes a directory of image files, optionally scales them,
 *    and generates html files to view the scaled images (and thumbnails).
 *
 *    Input:  dirin:  directory of input image files
 *            dirout: directory for output files
 *            rootname: root name for output files
 *            thumbwidth: width of thumb images, in pixels; use 0 for default
 *            viewwidth: max width of view images, in pixels; use 0 for default
 *
 *    Example:
 *         mkdir /tmp/lept/lion-in
 *         mkdir /tmp/lept/lion-out
 *         cp lion-page* /tmp/lept/lion-in
 *         htmlviewer /tmp/lept/lion-in /tmp/lept/lion-out lion 200 600
 *        ==> output:
 *            /tmp/lept/lion-out/lion.html         (main html file)
 *            /tmp/lept/lion-out/lion-links.html   (html file of links)
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

#ifdef _WIN32
#include <windows.h>   /* for CreateDirectory() */
#endif

static const l_int32  DEFAULT_THUMB_WIDTH = 120;
static const l_int32  DEFAULT_VIEW_WIDTH = 800;
static const l_int32  MIN_THUMB_WIDTH = 50;
static const l_int32  MIN_VIEW_WIDTH = 300;

static l_int32 pixHtmlViewer(const char *dirin, const char *dirout,
                             const char  *rootname, l_int32 thumbwidth,
                             l_int32 viewwidth);
static void WriteFormattedPix(const char *fname, PIX *pix);


int main(int    argc,
         char **argv)
{
char    *dirin, *dirout, *rootname;
l_int32  thumbwidth, viewwidth;

    if (argc != 6)
        return ERROR_INT(
            " Syntax:  htmlviewer dirin dirout rootname thumbwidth viewwidth",
             __func__, 1);
    dirin = argv[1];
    dirout = argv[2];
    rootname = argv[3];
    thumbwidth = atoi(argv[4]);
    viewwidth = atoi(argv[5]);
    setLeptDebugOK(1);

    pixHtmlViewer(dirin, dirout, rootname, thumbwidth, viewwidth);
    return 0;
}


/*---------------------------------------------------------------------*
 *            Generate smaller images for viewing and write html       *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixHtmlViewer()
 *
 * \param[in]    dirin      directory of input image files
 * \param[in]    dirout     directory for output files
 * \param[in]    rootname   root name for output files
 * \param[in]    thumbwidth width of thumb images in pixels; use 0 for default
 * \param[in]    viewwidth  maximum width of view images no up-scaling
 *                          in pixels; use 0 for default
 * \return  0 if OK; 1 on error
 *
 * <pre>
 * Notes:
 *      (1) The thumb and view reduced images are generated,
 *          along with two html files:
 *             <rootname>.html and <rootname>-links.html
 *      (2) The thumb and view files are named
 *             <rootname>_thumb_xxx.jpg
 *             <rootname>_view_xxx.jpg
 *          With this naming scheme, any number of input directories
 *          of images can be processed into views and thumbs
 *          and placed in the same output directory.
 * </pre>
 */
static l_int32
pixHtmlViewer(const char  *dirin,
              const char  *dirout,
              const char  *rootname,
              l_int32      thumbwidth,
              l_int32      viewwidth)
{
char      *fname, *fullname, *outname;
char      *mainname, *linkname, *linknameshort;
char      *viewfile, *thumbfile;
char      *shtml, *slink;
char       buf[512];
char       htmlstring[] = "<html>";
char       framestring[] = "</frameset></html>";
l_int32    i, nfiles, index, w, d, nimages, ret;
l_float32  factor;
PIX       *pix, *pixthumb, *pixview;
SARRAY    *safiles, *sathumbs, *saviews, *sahtml, *salink;

    if (!dirin)
        return ERROR_INT("dirin not defined", __func__, 1);
    if (!dirout)
        return ERROR_INT("dirout not defined", __func__, 1);
    if (!rootname)
        return ERROR_INT("rootname not defined", __func__, 1);

    if (thumbwidth == 0)
        thumbwidth = DEFAULT_THUMB_WIDTH;
    if (thumbwidth < MIN_THUMB_WIDTH) {
        L_WARNING("thumbwidth too small; using min value\n", __func__);
        thumbwidth = MIN_THUMB_WIDTH;
    }
    if (viewwidth == 0)
        viewwidth = DEFAULT_VIEW_WIDTH;
    if (viewwidth < MIN_VIEW_WIDTH) {
        L_WARNING("viewwidth too small; using min value\n", __func__);
        viewwidth = MIN_VIEW_WIDTH;
    }

        /* Make the output directory if it doesn't already exist */
#ifndef _WIN32
    snprintf(buf, sizeof(buf), "mkdir -p %s", dirout);
    ret = callSystemDebug(buf);
#else
    ret = CreateDirectory(dirout, NULL) ? 0 : 1;
#endif  /* !_WIN32 */
    if (ret) {
        L_ERROR("output directory %s not made\n", __func__, dirout);
        return 1;
    }

        /* Capture the filenames in the input directory */
    if ((safiles = getFilenamesInDirectory(dirin)) == NULL)
        return ERROR_INT("safiles not made", __func__, 1);

        /* Generate output text file names */
    snprintf(buf, sizeof(buf), "%s/%s.html", dirout, rootname);
    mainname = stringNew(buf);
    snprintf(buf, sizeof(buf), "%s/%s-links.html", dirout, rootname);
    linkname = stringNew(buf);
    linknameshort = stringJoin(rootname, "-links.html");

        /* Generate the thumbs and views */
    sathumbs = sarrayCreate(0);
    saviews = sarrayCreate(0);
    nfiles = sarrayGetCount(safiles);
    index = 0;
    for (i = 0; i < nfiles; i++) {
        fname = sarrayGetString(safiles, i, L_NOCOPY);
        fullname = genPathname(dirin, fname);
        lept_stderr("name: %s\n", fullname);
        if ((pix = pixRead(fullname)) == NULL) {
            lept_stderr("file %s not a readable image\n", fullname);
            lept_free(fullname);
            continue;
        }
        lept_free(fullname);

            /* Make and store the thumbnail images */
        pixGetDimensions(pix, &w, NULL, &d);
        factor = (l_float32)thumbwidth / (l_float32)w;
        pixthumb = pixScale(pix, factor, factor);
        snprintf(buf, sizeof(buf), "%s_thumb_%03d", rootname, index);
        sarrayAddString(sathumbs, buf, L_COPY);
        outname = genPathname(dirout, buf);
        WriteFormattedPix(outname, pixthumb);
        lept_free(outname);
        pixDestroy(&pixthumb);

            /* Make and store the view images */
        factor = (l_float32)viewwidth / (l_float32)w;
        if (factor >= 1.0)
            pixview = pixClone(pix);   /* no upscaling */
        else
            pixview = pixScale(pix, factor, factor);
        snprintf(buf, sizeof(buf), "%s_view_%03d", rootname, index);
        sarrayAddString(saviews, buf, L_COPY);
        outname = genPathname(dirout, buf);
        WriteFormattedPix(outname, pixview);
        lept_free(outname);
        pixDestroy(&pixview);
        pixDestroy(&pix);
        index++;
    }

        /* Generate the main html file */
    sahtml = sarrayCreate(0);
    sarrayAddString(sahtml, htmlstring, L_COPY);
    snprintf(buf, sizeof(buf), "<frameset cols=\"%d, *\">", thumbwidth + 30);
    sarrayAddString(sahtml, buf, L_COPY);
    snprintf(buf, sizeof(buf), "<frame name=\"thumbs\" src=\"%s\">",
             linknameshort);
    sarrayAddString(sahtml, buf, L_COPY);
    snprintf(buf, sizeof(buf), "<frame name=\"views\" src=\"%s\">",
            sarrayGetString(saviews, 0, L_NOCOPY));
    sarrayAddString(sahtml, buf, L_COPY);
    sarrayAddString(sahtml, framestring, L_COPY);
    shtml = sarrayToString(sahtml, 1);
    l_binaryWrite(mainname, "w", shtml, strlen(shtml));
    lept_stderr("******************************************\n"
                "Writing html file: %s\n"
                "******************************************\n", mainname);
    lept_free(shtml);
    lept_free(mainname);

        /* Generate the link html file */
    nimages = sarrayGetCount(saviews);
    lept_stderr("num. images = %d\n", nimages);
    salink = sarrayCreate(0);
    for (i = 0; i < nimages; i++) {
        viewfile = sarrayGetString(saviews, i, L_NOCOPY);
        thumbfile = sarrayGetString(sathumbs, i, L_NOCOPY);
        snprintf(buf, sizeof(buf),
                 "<a href=\"%s\" TARGET=views><img src=\"%s\"></a>",
                 viewfile, thumbfile);
        sarrayAddString(salink, buf, L_COPY);
    }
    slink = sarrayToString(salink, 1);
    l_binaryWrite(linkname, "w", slink, strlen(slink));
    lept_free(slink);
    lept_free(linkname);
    lept_free(linknameshort);
    sarrayDestroy(&safiles);
    sarrayDestroy(&sathumbs);
    sarrayDestroy(&saviews);
    sarrayDestroy(&sahtml);
    sarrayDestroy(&salink);
    return 0;
}

static void
WriteFormattedPix(const char *fname,
                  PIX        *pix)
{
l_int32  d;

    d = pixGetDepth(pix);
    if (d == 1 || pixGetColormap(pix))
        pixWrite(fname, pix, IFF_PNG);
    else
        pixWrite(fname, pix, IFF_JFIF_JPEG);
    return;
}

