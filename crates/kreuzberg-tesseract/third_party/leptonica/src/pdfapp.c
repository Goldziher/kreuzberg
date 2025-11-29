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
 * \file pdfapp.c
 * <pre>
 *
 *    Image processing operations on multiple images followed by wrapping
 *    them into a pdf.
 *
 *    There are two possible ways to specify the set of images:
 *    (1) an array of pathnames
 *    (2) a directory, typically with an additional pattern for selection.
 *    We use (1) because it is both simpler and more general.
 *
 *    Corresponding to each function here is:
 *    (1) the image processing function that is carried out on each image
 *    (2) a program in prog that extracts images from a pdf and calls this
 *        function with an array of their pathnames.
 *
 *    |=============================================================|
 *    |                        Important notes                      |
 *    |=============================================================|
 *    | Some of these functions require I/O libraries such as       |
 *    | libtiff, libjpeg, libpng and libz.  If you do not have      |
 *    | these libraries, some calls will fail.  For example,        |
 *    | if you do not have libtiff, you cannot write a pdf that     |
 *    | uses libtiff to encode bilevel images in tiffg4.            |
 *    |                                                             |
 *    | You can manually deactivate all pdf writing by setting      |
 *    | this in environ.h:                                          |
 *    | \code                                                       |
 *    |      #define  USE_PDFIO     0                               |
 *    | \endcode                                                    |
 *    | This will link the stub file pdfappstub.c.                  |
 *    |=============================================================|
 *
 *     The images in the pdf file can be rendered using a pdf viewer,
 *     such as evince, gv, xpdf or acroread.
 *
 *     Compression of images for prog/compresspdf
 *          l_int32          compressFilesToPdf()
 *
 *     Crop images for prog/croppdf
 *          l_int32          cropFilesToPdf()
 *
 *     Cleanup and binarization of images for prog/cleanpdf
 *          l_int32          cleanTo1bppFilesToPdf()
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"


/* --------------------------------------------*/
#if  USE_PDFIO   /* defined in environ.h */
 /* --------------------------------------------*/

/*---------------------------------------------------------------------*
 *              Compression of images for prog/compresspdf             *
 *---------------------------------------------------------------------*/
/*!
 * \brief   compressFilesToPdf()
 *
 * \param[in]    sa            sorted full pathnames of images
 * \param[in]    onebit        set to 1 to enforce 1 bpp tiffg4 encoding
 * \param[in]    savecolor     if %onebit == 1, set to 1 to save color
 * \param[in]    scalefactor   scaling factor applied to each image; > 0.0
 * \param[in]    quality       for jpeg: 0 for default (50; otherwise 25 - 95.
 * \param[in]    title         [optional] pdf title; can be null
 * \param[in]    fileout       pdf file of all images
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *    (1) This function is designed to optionally scale and compress a set of
 *        images, wrapping them in a pdf in the order given in the input %sa.
 *    (2) It does the image processing for prog/compresspdf.c.
 *    (3) Images in the output pdf are encoded with either tiffg4 or jpeg (DCT),
 *        or a mixture of them depending on parameters %onebit and %savecolor.
 *        If the resulting image is 1 bpp, it is encoded with tiffg4;
 *        otherwise, DCT (jpeg) encoding is used.
 *    (4) Parameters %onebit and %savecolor work as follows:
 *        %onebit = 0: no depth conversion, default encoding depends on depth
 *        %onebit = 1, %savecolor = 0: all images converted to 1 bpp
 *        %onebit = 1, %savecolor = 1: images without color are converted
 *           to 1 bpp; images with color have the color preserved.
 *    (5) In use, if most of the pages are 1 bpp but some have color that needs
 *        to be preserved, %onebit and %savecolor should both be 1.  This
 *        causes DCT compression of color images and tiffg4 compression
 *        of monochrome images.
 *    (6) The images will be concatenated in the order given in %sa.
 *    (7) Typically, %scalefactor <= 1.0.  It is applied to each image
 *        before encoding.  If you enter a value <= 0.0, it will be set to 1.0.
 *        The maximum allowed value is 2.0.  If the pdf is a set of low-res
 *        (say, 100 ppi) 8 bpp images, set onebit = 1 and use scalefactor = 2.0
 *        to upscale before binarizing.
 *    (8) Default jpeg %quality is 50; otherwise, quality factors between
 *        25 and 95 are enforced.
 *    (9) Page images at 300 ppi are about 8 Mpixels.  RGB(A) rasters are
 *        then about 32 MB (1 bpp images are about 1 MB).  If there are
 *        more than 25 images, store the images after processing as an
 *        array of compressed images (a Pixac); otherwise, use a Pixa.
 * </pre>
 */
l_ok
compressFilesToPdf(SARRAY      *sa,
                   l_int32      onebit,
                   l_int32      savecolor,
                   l_float32    scalefactor,
                   l_int32      quality,
                   const char  *title,
                   const char  *fileout)
{
char      *fname;
l_int32    n, i, res, processcolor;
l_int32    maxsmallset = 25;  /* max num images kept uncompressed in array */
l_float32  colorfract;
PIX       *pixs, *pix1, *pix2, *pix3, *pix4;
PIXA      *pixa1 = NULL;
PIXAC     *pixac1 = NULL;

    if (!sa)
        return ERROR_INT("sa not defined", __func__, 1);
    if (!fileout)
        return ERROR_INT("fileout not defined", __func__, 1);
    if (scalefactor <= 0) scalefactor = 1.0;
    if (scalefactor > 2.0) {
        L_WARNING("scalefactor %f too big; setting to 2.0\n", __func__,
                  scalefactor);
        scalefactor = 2.0;
    }
    if (quality <= 0) quality = 50;  /* default value */
    if (quality < 25) {
        L_WARNING("quality %d too low; setting to 25\n", __func__, quality);
        quality = 25;
    }
    if (quality > 95) {
        L_WARNING("quality %d too high; setting to 95\n", __func__, quality);
        quality = 95;
    }
    if ((n = sarrayGetCount(sa)) == 0)
        return ERROR_INT("sa is empty", __func__, 1);

    if (n <= maxsmallset)
        pixa1 = pixaCreate(n);
    else
        pixac1 = pixacompCreate(n);
    for (i = 0; i < n; i++) {
        if (i == 0)
            lept_stderr("page: ");
        else if (i % 10 == 0)
            lept_stderr("%d . ", i);
        fname = sarrayGetString(sa, i, L_NOCOPY);
        processcolor = FALSE;
        pixs = pixRead(fname);
        pix1 = pixRemoveColormap(pixs, REMOVE_CMAP_BASED_ON_SRC);
        if (!onebit) {  /* scale and save the input image */
            pix2 = pixScale(pix1, scalefactor, scalefactor);
            processcolor = TRUE;
        } else if (onebit && savecolor) {
            pixColorFraction(pix1, 40, 224, 60, 4, NULL, &colorfract);
            if (colorfract > 0.01) { /* save the color; use DCT encoding */
                processcolor = TRUE;
                pix2 = pixScale(pix1, scalefactor, scalefactor);
            }
        }
        if (!processcolor) {  /* scale, binarize and use tiffg4 encoding */
            if (pixGetDepth(pix1) != 1) {
                pix3 = pixConvertTo8(pix1, FALSE);
                if (scalefactor < 1.0 ||
                   (scalefactor > 1.0 && scalefactor < 2.0)) {
                    pix4 = pixScale(pix3, scalefactor, scalefactor);
                    pix2 = pixConvertTo1(pix4, 180);
                    pixDestroy(&pix4);
                } else if (scalefactor == 1.0) {
                    pix2 = pixConvertTo1(pix3, 180);
                } else {  /* scalefactor == 2.0 */
                    pix2 = pixScaleGray2xLIThresh(pix3, 180);
                }
                pixDestroy(&pix3);
            } else {  /* pix1 is 1 bpp */
                pix2 = pixScale(pix1, scalefactor, scalefactor);
            }
        }
        if (n <= maxsmallset) {
            pixaAddPix(pixa1, pix2, L_INSERT);
        } else {
            pixacompAddPix(pixac1, pix2, IFF_DEFAULT);
            pixDestroy(&pix2);
        }
        pixDestroy(&pixs);
        pixDestroy(&pix1);
    }

        /* Generate the pdf.  Compute the actual input resolution from
         * the pixel dimensions of the first image.  This will cause each
         * page to be printed to cover an 8.5 x 11 inch sheet of paper. */
    lept_stderr("\nWrite output to %s\n", fileout);
    if (n <= maxsmallset)
        pix1 = pixaGetPix(pixa1, 0, L_CLONE);
    else
        pix1 = pixacompGetPix(pixac1, 0);
    pixInferResolution(pix1, 11.0, &res);
    pixDestroy(&pix1);
    if (strcmp(title, "none") == 0)
        title = NULL;
    if (n <= maxsmallset) {
        pixaConvertToPdf(pixa1, res, 1.0, L_DEFAULT_ENCODE, quality,
                         title, fileout);
        pixaDestroy(&pixa1);
    } else {
        pixacompConvertToPdf(pixac1, res, 1.0, L_DEFAULT_ENCODE, quality,
                             title, fileout);
        pixacompDestroy(&pixac1);
    }
    return 0;
}


/*---------------------------------------------------------------------*
 *                       Crop images for prog/croppdf                  *
 *---------------------------------------------------------------------*/
/*!
 * \brief   cropFilesToPdf()
 *
 * \param[in]    sa            sorted full pathnames of images
 * \param[in]    lr_clear      full res pixels cleared at left and right sides
 * \param[in]    tb_clear      full res pixels cleared at top and bottom sides
 * \param[in]    edgeclean     parameter for removing edge noise (-1 to 15)
 *                             default = 0 (no removal);
 *                             15 is maximally aggressive for random noise
 *                             -1 for aggressively removing side noise
 *                             -2 to extract page embedded in black background
 * \param[in]    lr_border     full res final "added" pixels on left and right
 * \param[in]    tb_border     full res final "added" pixels on top and bottom
 * \param[in]    maxwiden      max fractional horizontal stretch allowed
 * \param[in]    printwiden    0 to skip, 1 for 8.5x11, 2 for A4
 * \param[in]    title         [optional] pdf title; can be null
 * \param[in]    fileout       pdf file of all images
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *    (1) This function is designed to optionally remove white space from
 *        around the page images, and generate a pdf that prints with
 *        foreground occupying much of the full page.
 *    (2) It does the image processing for prog/croppdf.c.
 *    (3) Images in the output pdf are 1 bpp and encoded with tiffg4.
 *    (4) See documentation in pixCropImage() for details on the processing.
 *    (5) The images will be concatenated in the order given in %safiles.
 *    (6) Output page images are at 300 ppi and are stored in memory.
 *        They are about 1 Mpixel when uncompressed.  For up to 200 pages,
 *        the images are stored uncompressed; otherwise, the stored
 *        images are compressed with tiffg4.
 * </pre>
 */
l_ok
cropFilesToPdf(SARRAY      *sa,
               l_int32      lr_clear,
               l_int32      tb_clear,
               l_int32      edgeclean,
               l_int32      lr_border,
               l_int32      tb_border,
               l_float32    maxwiden,
               l_int32      printwiden,
               const char  *title,
               const char  *fileout)
{
char      *fname;
l_int32    n, i, res;
l_int32    maxsmallset = 200;  /* max num images kept uncompressed in array */
PIX       *pixs, *pix1;
PIXA      *pixa1 = NULL;
PIXAC     *pixac1 = NULL;

    if (!sa)
        return ERROR_INT("sa not defined", __func__, 1);
    if (!fileout)
        return ERROR_INT("fileout not defined", __func__, 1);
    if ((n = sarrayGetCount(sa)) == 0)
        return ERROR_INT("sa is empty", __func__, 1);

    if (n <= maxsmallset)
        pixa1 = pixaCreate(n);
    else
        pixac1 = pixacompCreate(n);
    for (i = 0; i < n; i++) {
        if (i == 0)
            lept_stderr("page: ");
        else if (i % 10 == 0)
            lept_stderr("%d . ", i);
        fname = sarrayGetString(sa, i, L_NOCOPY);
        pixs = pixRead(fname);
        pix1 = pixCropImage(pixs, lr_clear, tb_clear, edgeclean,
                            lr_border, tb_border, maxwiden, printwiden,
                            NULL, NULL);
        pixDestroy(&pixs);
        if (!pix1) {
            L_ERROR("pix1 not made for i = %d\n", __func__, i);
            continue;
        }
        if (n <= maxsmallset)
            pixaAddPix(pixa1, pix1, L_INSERT);
        else
            pixacompAddPix(pixac1, pix1, IFF_TIFF_G4);
    }

        /* Generate the pdf.  Compute the actual input resolution from
         * the pixel dimensions of the first image.  This will cause each
         * page to be printed to cover an 8.5 x 11 inch sheet of paper. */
    lept_stderr("\nWrite output to %s\n", fileout);
    if (n <= maxsmallset)
        pix1 = pixaGetPix(pixa1, 0, L_CLONE);
    else
        pix1 = pixacompGetPix(pixac1, 0);
    pixInferResolution(pix1, 11.0, &res);
    pixDestroy(&pix1);
    if (strcmp(title, "none") == 0)
        title = NULL;
    if (n <= maxsmallset) {
        pixaConvertToPdf(pixa1, res, 1.0, L_G4_ENCODE, 0, title, fileout);
        pixaDestroy(&pixa1);
    } else {
        pixacompConvertToPdf(pixac1, res, 1.0, L_G4_ENCODE, 0, title, fileout);
        pixacompDestroy(&pixac1);
    }
    return 0;
}


/*---------------------------------------------------------------------*
 *        Cleanup and binarization of images for prog/cleanpdf         *
 *---------------------------------------------------------------------*/
/*!
 * \brief   cleanTo1bppFilesToPdf()
 *
 * \param[in]    sa            sorted full pathnames of images
 * \param[in]    res           either 300 or 600 ppi for output
 * \param[in]    contrast      vary contrast: 1 = lightest; 10 = darkest;
 *                             suggest 1 unless light features are being lost
 * \param[in]    rotation      cw by 90 degrees: {0,1,2,3} represent
 *                             0, 90, 180 and 270 degree cw rotations
 * \param[in]    opensize      opening size of structuring element for noise
 *                             removal: {0 or 1to skip; 2, 3 for opening}
 * \param[in]    title         [optional] pdf title; can be null
 * \param[in]    fileout       pdf file of all images
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *    (1) This deskews, optionally rotates and darkens, cleans background
 *        to white, binarizes and optionally removes small noise, and
 *        put the images into the pdf in the order given in %sa.
 *    (2) All images in the pdf are tiffg4 encoded.
 *    (3) For color and grayscale input, local background normalization is
 *        done to 200, and a threshold of 180 sets the maximum foreground
 *        value in the normalized image.
 *    (4) The %res parameter can be either 300 or 600 ppi.  If the input
 *        is gray or color and %res = 600, this does an interpolated 2x
 *        expansion before binarizing.
 *    (5) The %contrast parameter adjusts the binarization to avoid losing
 *        lighter input pixels.  Contrast is increased as %contrast increases
 *        from 1 to 10.
 *    (6) The #opensize parameter is the size of a square SEL used with
 *        opening to remove small speckle noise.  Allowed open sizes are 2,3.
 *        If this is to be used, try 2 before 3.
 *    (7) If there are more than 200 images, store the images after processing
 *        as an array of compressed images (a Pixac); otherwise, use a Pixa.
 * </pre>
 */
l_ok
cleanTo1bppFilesToPdf(SARRAY      *sa,
                      l_int32      res,
                      l_int32      contrast,
                      l_int32      rotation,
                      l_int32      opensize,
                      const char  *title,
                      const char  *fileout)
{
char      *fname;
l_int32    n, i, scale;
l_int32    maxsmallset = 200;  /* max num images kept uncompressed in array */
PIX       *pixs, *pix1;
PIXA      *pixa1 = NULL;
PIXAC     *pixac1 = NULL;

    if (!sa)
        return ERROR_INT("sa not defined", __func__, 1);
    if (!fileout)
        return ERROR_INT("fileout not defined", __func__, 1);
    if (res == 0) res = 300;
    if (res != 300 && res != 600) {
        L_ERROR("invalid res = %d; res must be in {0, 300, 600}\n",
                __func__, res);
        return 1;
    }
    if (contrast < 1 || contrast > 10) {
        L_ERROR("invalid contrast = %d; contrast must be in [1...10]\n",
                __func__, contrast);
        return 1;
    }
    if (rotation < 0 || rotation > 3) {
        L_ERROR("invalid rotation = %d; rotation must be in  {0,1,2,3}\n",
                __func__, rotation);
        return 1;
    }
    if (opensize > 3) {
        L_ERROR("invalid opensize = %d; opensize must be <= 3\n",
                __func__, opensize);
        return 1;
    }
    scale = (res == 300) ? 1 : 2;
    if ((n = sarrayGetCount(sa)) == 0)
        return ERROR_INT("sa is empty", __func__, 1);

    if (n <= maxsmallset)
        pixa1 = pixaCreate(n);
    else
        pixac1 = pixacompCreate(n);
    for (i = 0; i < n; i++) {
        if (i == 0)
            lept_stderr("page: ");
        else if (i % 10 == 0)
            lept_stderr("%d . ", i);
        fname = sarrayGetString(sa, i, L_NOCOPY);
        if ((pixs = pixRead(fname)) == NULL) {
            L_ERROR("pixs not read from %s\n", __func__, fname);
            continue;
        }

        pix1 = pixCleanImage(pixs, contrast, rotation, scale, opensize);
        if (n <= maxsmallset) {
            pixaAddPix(pixa1, pix1, L_INSERT);
        } else {
            pixacompAddPix(pixac1, pix1, IFF_TIFF_G4);
            pixDestroy(&pix1);
        }
        pixDestroy(&pixs);
    }

        /* Generate the pdf.  Compute the actual input resolution from
         * the pixel dimensions of the first image.  This will cause each
         * page to be printed to cover an 8.5 x 11 inch sheet of paper. */
    lept_stderr("Write output to %s\n", fileout);
    if (n <= maxsmallset)
        pix1 = pixaGetPix(pixa1, 0, L_CLONE);
    else
        pix1 = pixacompGetPix(pixac1, 0);
    pixInferResolution(pix1, 11.0, &res);
    pixDestroy(&pix1);
    if (strcmp(title, "none") == 0)
        title = NULL;

    if (n <= maxsmallset) {
        pixaConvertToPdf(pixa1, res, 1.0, L_G4_ENCODE, 0, title, fileout);
        pixaDestroy(&pixa1);
    } else {
        pixacompConvertToPdf(pixac1, res, 1.0, L_G4_ENCODE, 0, title, fileout);
        pixacompDestroy(&pixac1);
    }
    return 0;
}

/* --------------------------------------------*/
#endif  /* USE_PDFIO */
/* --------------------------------------------*/
