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
 * \file jpegio.c
 * <pre>
 *
 *    Read jpeg from file
 *          PIX             *pixReadJpeg()  [special top level]
 *          PIX             *pixReadStreamJpeg()
 *
 *    Read jpeg metadata from file
 *          l_int32          readHeaderJpeg()
 *          l_int32          freadHeaderJpeg()
 *          l_int32          fgetJpegResolution()
 *          l_int32          fgetJpegComment()
 *
 *    Write jpeg to file
 *          l_int32          pixWriteJpeg()  [special top level]
 *          l_int32          pixWriteStreamJpeg()
 *
 *    Read/write to memory
 *          PIX             *pixReadMemJpeg()
 *          l_int32          readHeaderMemJpeg()
 *          l_int32          readResolutionMemJpeg()
 *          l_int32          pixWriteMemJpeg()
 *
 *    Setting special flag for chroma sampling on write
 *          l_int32          pixSetChromaSampling()
 *
 *    Static system helpers
 *          static void      jpeg_error_catch_all_1()
 *          static void      jpeg_error_catch_all_2()
 *          static l_uint8   jpeg_getc()
 *          static l_int32   jpeg_comment_callback()
 *
 *    Documentation: libjpeg.doc can be found, along with all
 *    source code, at ftp://ftp.uu.net/graphics/jpeg
 *    Download and untar the file:  jpegsrc.v6b.tar.gz
 *    A good paper on jpeg can also be found there: wallace.ps.gz
 *
 *    The functions in libjpeg make it very simple to compress
 *    and decompress images.  On input (decompression from file),
 *    3 component color images can be read into either an 8 bpp Pix
 *    with a colormap or a 32 bpp Pix with RGB components.  For output
 *    (compression to file), all color Pix, whether 8 bpp with a
 *    colormap or 32 bpp, are written compressed as a set of three
 *    8 bpp (rgb) images.
 *
 *    Low-level error handling
 *    ------------------------
 *    The default behavior of the jpeg library is to call exit.
 *    This is often undesirable, and the caller should make the
 *    decision when to abort a process.  To prevent the jpeg library
 *    from calling exit(), setjmp() has been inserted into all
 *    readers and writers, and the cinfo struct has been set up so that
 *    the low-level jpeg library will call a special error handler
 *    that doesn't exit, instead of the default function error_exit().
 *
 *    To avoid race conditions and make these functions thread-safe in
 *    the rare situation where calls to two threads are simultaneously
 *    failing on bad jpegs, we insert a local copy of the jmp_buf struct
 *    into the cinfo.client_data field, and use this on longjmp.
 *    For extracting the jpeg comment, we have the added complication
 *    that the client_data field must also return the jpeg comment,
 *    and we use a different error handler.
 *
 *    How to avoid subsampling the chroma channels
 *    --------------------------------------------
 *    By default, the U,V (chroma) channels use 2x2 subsampling (aka 4.2.0).
 *    Higher quality for color, using full resolution (4.4.4) for the chroma,
 *    is obtained by setting a field in the pix before writing:
 *        pixSetChromaSampling(pix, L_NO_CHROMA_SAMPLING_JPEG);
 *    The field can be reset for default 4.2.0 subsampling with
 *        pixSetChromaSampling(pix, 0);
 *
 *    How to extract just the luminance channel in reading RGB
 *    --------------------------------------------------------
 *    For higher resolution and faster decoding of an RGB image, you
 *    can extract just the 8 bpp luminance channel, using pixReadJpeg(),
 *    where you use L_JPEG_READ_LUMINANCE for the %hint arg.
 *
 *    How to continue to read if the data is corrupted
 *    ------------------------------------------------
 *    By default, if data is corrupted we make every effort to fail
 *    to return a pix.  (Failure is not always possible with bad
 *    data, because in some situations, such as during arithmetic
 *    decoding, the low-level jpeg library will not abort or raise
 *    a warning.)  To attempt to ignore warnings and get a pix when data
 *    is corrupted, use L_JPEG_CONTINUE_WITH_BAD_DATA in the %hint arg.
 *
 *    Compressing to memory and decompressing from memory
 *    ---------------------------------------------------
 *    On systems like Windows without fmemopen() and open_memstream(),
 *    we write data to a temp file and read it back for operations
 *    between pix and compressed-data, such as pixReadMemJpeg() and
 *    pixWriteMemJpeg().
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"
#include "pix_internal.h"

/* --------------------------------------------*/
#if  HAVE_LIBJPEG   /* defined in environ.h */
/* --------------------------------------------*/

#include <setjmp.h>

    /* jconfig.h makes the error of setting
     *   #define HAVE_STDLIB_H
     * which conflicts with config_auto.h (where it is set to 1) and results
     * for some gcc compiler versions in a warning.  The conflict is harmless
     * but we suppress it by undefining the variable. */
#undef HAVE_STDLIB_H
#include "jpeglib.h"

static void jpeg_error_catch_all_1(j_common_ptr cinfo);
static void jpeg_error_catch_all_2(j_common_ptr cinfo);
static l_uint8 jpeg_getc(j_decompress_ptr cinfo);

    /* Note: 'boolean' is defined in jmorecfg.h.  We use it explicitly
     * here because for Windows where __MINGW32__ is defined,
     * the prototype for jpeg_comment_callback() is given as
     * returning a boolean.  */
static boolean jpeg_comment_callback(j_decompress_ptr cinfo);

    /* This is saved in the client_data field of cinfo, and used both
     * to retrieve the comment from its callback and to handle
     * exceptions with a longjmp. */
struct callback_data {
    jmp_buf   jmpbuf;
    l_uint8  *comment;
};

#ifndef  NO_CONSOLE_IO
#define  DEBUG_INFO      0
#endif  /* ~NO_CONSOLE_IO */


/*---------------------------------------------------------------------*
 *                 Read jpeg from file (special function)              *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixReadJpeg()
 *
 * \param[in]    filename
 * \param[in]    cmapflag   0 for no colormap in returned pix;
 *                          1 to return an 8 bpp cmapped pix if spp = 3 or 4
 * \param[in]    reduction  scaling factor: 1, 2, 4 or 8
 * \param[out]   pnwarn     [optional] number of warnings about
 *                          corrupted data
 * \param[in]    hint       a bitwise OR of L_JPEG_* values; 0 for default
 * \return  pix, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This is a special function for reading jpeg files.
 *      (2) Use this if you want the jpeg library to create
 *          an 8 bpp colormapped image.
 *      (3) Images reduced by factors of 2, 4 or 8 can be returned
 *          significantly faster than full resolution images.
 *      (4) If the jpeg data is bad, depending on the severity of the
 *          data corruption one of two things will happen:
 *          (a) 0 or more warnings are generated, or
 *          (b) the library will immediately attempt to exit. This is
 *              caught by our error handler and no pix will be returned.
 *          If data corruption causes a warning, the default action
 *          is to abort the read. The reason is that malformed jpeg
 *          data sequences exist that prevent termination of the read.
 *          To allow the decoding to continue after corrupted data is
 *          encountered, include L_JPEG_CONTINUE_WITH_BAD_DATA in %hint.
 *      (5) The possible hint values are given in the enum in imageio.h:
 *            * L_JPEG_READ_LUMINANCE
 *            * L_JPEG_CONTINUE_WITH_BAD_DATA
 *          Default (0) is to do neither, and to fail on warning of data
 *          corruption.
 * </pre>
 */
PIX *
pixReadJpeg(const char  *filename,
            l_int32      cmapflag,
            l_int32      reduction,
            l_int32     *pnwarn,
            l_int32      hint)
{
l_int32   ret;
l_uint8  *comment;
FILE     *fp;
PIX      *pix;

    if (pnwarn) *pnwarn = 0;
    if (!filename)
        return (PIX *)ERROR_PTR("filename not defined", __func__, NULL);
    if (cmapflag != 0 && cmapflag != 1)
        cmapflag = 0;  /* default */
    if (reduction != 1 && reduction != 2 && reduction != 4 && reduction != 8)
        return (PIX *)ERROR_PTR("reduction not in {1,2,4,8}", __func__, NULL);

    if ((fp = fopenReadStream(filename)) == NULL)
        return (PIX *)ERROR_PTR_1("image file not found",
                                  filename, __func__, NULL);
    pix = pixReadStreamJpeg(fp, cmapflag, reduction, pnwarn, hint);
    if (pix) {
        ret = fgetJpegComment(fp, &comment);
        if (!ret && comment)
            pixSetText(pix, (char *)comment);
        LEPT_FREE(comment);
    }
    fclose(fp);

    if (!pix)
        return (PIX *)ERROR_PTR_1("image not returned",
                                  filename, __func__, NULL);
    return pix;
}


/*!
 * \brief   pixReadStreamJpeg()
 *
 * \param[in]    fp         file stream
 * \param[in]    cmapflag   0 for no colormap in returned pix;
 *                          1 to return an 8 bpp cmapped pix if spp = 3 or 4
 * \param[in]    reduction  scaling factor: 1, 2, 4 or 8
 * \param[out]   pnwarn     [optional] number of warnings
 * \param[in]    hint       a bitwise OR of L_JPEG_* values; 0 for default
 * \return  pix, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) For usage, see pixReadJpeg().
 *      (2) The jpeg comment, if it exists, is not stored in the pix.
 * </pre>
 */
PIX *
pixReadStreamJpeg(FILE     *fp,
                  l_int32   cmapflag,
                  l_int32   reduction,
                  l_int32  *pnwarn,
                  l_int32   hint)
{
l_int32                        cyan, yellow, magenta, black, nwarn;
l_int32                        i, j, k, rval, gval, bval;
l_int32                        nlinesread, abort_on_warning;
l_int32                        w, h, wpl, spp, ncolors, cindex, ycck, cmyk;
l_uint32                      *data;
l_uint32                      *line, *ppixel;
JSAMPROW                       rowbuffer;
PIX                           *pix;
PIXCMAP                       *cmap;
struct jpeg_decompress_struct  cinfo = { 0 };
struct jpeg_error_mgr          jerr = { 0 };
jmp_buf                        jmpbuf;  /* must be local to the function */

    if (pnwarn) *pnwarn = 0;
    if (!fp)
        return (PIX *)ERROR_PTR("fp not defined", __func__, NULL);
    if (cmapflag != 0 && cmapflag != 1)
        cmapflag = 0;  /* default */
    if (reduction != 1 && reduction != 2 && reduction != 4 && reduction != 8)
        return (PIX *)ERROR_PTR("reduction not in {1,2,4,8}", __func__, NULL);

    if (BITS_IN_JSAMPLE != 8)  /* set in jmorecfg.h */
        return (PIX *)ERROR_PTR("BITS_IN_JSAMPLE != 8", __func__, NULL);

    rewind(fp);
    pix = NULL;
    rowbuffer = NULL;

        /* Modify the jpeg error handling to catch fatal errors  */
    cinfo.err = jpeg_std_error(&jerr);
    jerr.error_exit = jpeg_error_catch_all_1;
    cinfo.client_data = (void *)&jmpbuf;
    if (setjmp(jmpbuf)) {
        jpeg_destroy_decompress(&cinfo);
        pixDestroy(&pix);
        LEPT_FREE(rowbuffer);
        return (PIX *)ERROR_PTR("internal jpeg error", __func__, NULL);
    }

        /* Initialize jpeg structs for decompression */
    jpeg_create_decompress(&cinfo);
    jpeg_stdio_src(&cinfo, fp);
    jpeg_read_header(&cinfo, TRUE);
    cinfo.scale_denom = reduction;
    cinfo.scale_num = 1;
    jpeg_calc_output_dimensions(&cinfo);
    if (hint & L_JPEG_READ_LUMINANCE) {
        cinfo.out_color_space = JCS_GRAYSCALE;
        spp = 1;
        L_INFO("reading luminance channel only\n", __func__);
    } else {
        spp = cinfo.out_color_components;
    }

        /* Allocate the image and a row buffer */
    w = cinfo.output_width;
    h = cinfo.output_height;
    ycck = (cinfo.jpeg_color_space == JCS_YCCK && spp == 4 && cmapflag == 0);
    cmyk = (cinfo.jpeg_color_space == JCS_CMYK && spp == 4 && cmapflag == 0);
    if (spp != 1 && spp != 3 && !ycck && !cmyk) {
        jpeg_destroy_decompress(&cinfo);
        return (PIX *)ERROR_PTR("spp must be 1 or 3, or YCCK or CMYK",
                                __func__, NULL);
    }
    if ((spp == 3 && cmapflag == 0) || ycck || cmyk) {  /* rgb or 4 bpp color */
        rowbuffer = (JSAMPROW)LEPT_CALLOC(sizeof(JSAMPLE), (size_t)spp * w);
        pix = pixCreate(w, h, 32);
    } else {  /* 8 bpp gray or colormapped */
        rowbuffer = (JSAMPROW)LEPT_CALLOC(sizeof(JSAMPLE), w);
        pix = pixCreate(w, h, 8);
    }
    if (!rowbuffer || !pix) {
        LEPT_FREE(rowbuffer);
        rowbuffer = NULL;
        pixDestroy(&pix);
        jpeg_destroy_decompress(&cinfo);
        return (PIX *)ERROR_PTR("rowbuffer or pix not made", __func__, NULL);
    }
    pixSetInputFormat(pix, IFF_JFIF_JPEG);

        /* Initialize decompression.
         * Set up a colormap for color quantization if requested.
         * Arithmetic coding is rarely used on the jpeg data, but if it
         * is, jpeg_start_decompress() handles the decoding.
         * With corrupted encoded data, this can take an arbitrarily
         * long time, and fuzzers are finding examples.  Unfortunately,
         * there is no way to get a callback from an error in this phase. */
    if (spp == 1) {  /* Grayscale or colormapped */
        jpeg_start_decompress(&cinfo);
    } else {        /* Color; spp == 3 or YCCK or CMYK */
        if (cmapflag == 0) {   /* 24 bit color in 32 bit pix or YCCK/CMYK */
            cinfo.quantize_colors = FALSE;
            jpeg_start_decompress(&cinfo);
        } else {      /* Color quantize to 8 bits */
            cinfo.quantize_colors = TRUE;
            cinfo.desired_number_of_colors = 256;
            jpeg_start_decompress(&cinfo);

                /* Construct a pix cmap */
            cmap = pixcmapCreate(8);
            ncolors = cinfo.actual_number_of_colors;
            for (cindex = 0; cindex < ncolors; cindex++) {
                rval = cinfo.colormap[0][cindex];
                gval = cinfo.colormap[1][cindex];
                bval = cinfo.colormap[2][cindex];
                pixcmapAddColor(cmap, rval, gval, bval);
            }
            pixSetColormap(pix, cmap);
        }
    }
    wpl  = pixGetWpl(pix);
    data = pixGetData(pix);

        /* Decompress.  It appears that jpeg_read_scanlines() always
         * returns 1 when you ask for one scanline, but we test anyway.
         * During decoding of scanlines, warnings are issued if corrupted
         * data is found.  The default behavior is to abort reading
         * when a warning is encountered.  By setting the hint to have
         * the same bit set as in L_JPEG_CONTINUE_WITH_BAD_DATA, e.g.,
         *       hint = hint | L_JPEG_CONTINUE_WITH_BAD_DATA
         * reading will continue after warnings, in an attempt to return
         * the (possibly corrupted) image. */
    abort_on_warning = (hint & L_JPEG_CONTINUE_WITH_BAD_DATA) ? 0 : 1;
    for (i = 0; i < h; i++) {
        nlinesread = jpeg_read_scanlines(&cinfo, &rowbuffer, (JDIMENSION)1);
        nwarn = cinfo.err->num_warnings;
        if (nlinesread == 0 || (abort_on_warning && nwarn > 0)) {
            L_ERROR("read error at scanline %d; nwarn = %d\n",
                    __func__, i, nwarn);
            pixDestroy(&pix);
            jpeg_destroy_decompress(&cinfo);
            LEPT_FREE(rowbuffer);
            rowbuffer = NULL;
            if (pnwarn) *pnwarn = nwarn;
            return (PIX *)ERROR_PTR("bad data", __func__, NULL);
        }

            /* -- 24 bit color -- */
        if ((spp == 3 && cmapflag == 0) || ycck || cmyk) {
            ppixel = data + i * wpl;
            if (spp == 3) {
                for (j = k = 0; j < w; j++) {
                    SET_DATA_BYTE(ppixel, COLOR_RED, rowbuffer[k++]);
                    SET_DATA_BYTE(ppixel, COLOR_GREEN, rowbuffer[k++]);
                    SET_DATA_BYTE(ppixel, COLOR_BLUE, rowbuffer[k++]);
                    ppixel++;
                }
            } else {
                    /* This is a conversion from CMYK -> RGB that ignores
                       color profiles, and is invoked when the image header
                       claims to be in CMYK or YCCK colorspace.  If in YCCK,
                       libjpeg may be doing YCCK -> CMYK under the hood.
                       To understand why the colors need to be inverted on
                       read-in for the Adobe marker, see the "Special
                       color spaces" section of "Using the IJG JPEG
                       Library" by Thomas G. Lane:
                         http://www.jpegcameras.com/libjpeg/libjpeg-3.html#ss3.1
                       The non-Adobe conversion is equivalent to:
                           rval = black - black * cyan / 255
                           ...
                       The Adobe conversion is equivalent to:
                           rval = black - black * (255 - cyan) / 255
                           ...
                       Note that cyan is the complement to red, and we
                       are subtracting the complement color (weighted
                       by black) from black.  For Adobe conversions,
                       where they've already inverted the CMY but not
                       the K, we have to invert again.  The results
                       must be clipped to [0 ... 255]. */
                for (j = k = 0; j < w; j++) {
                    cyan = rowbuffer[k++];
                    magenta = rowbuffer[k++];
                    yellow = rowbuffer[k++];
                    black = rowbuffer[k++];
                    if (cinfo.saw_Adobe_marker) {
                        rval = (black * cyan) / 255;
                        gval = (black * magenta) / 255;
                        bval = (black * yellow) / 255;
                    } else {
                        rval = black * (255 - cyan) / 255;
                        gval = black * (255 - magenta) / 255;
                        bval = black * (255 - yellow) / 255;
                    }
                    rval = L_MIN(L_MAX(rval, 0), 255);
                    gval = L_MIN(L_MAX(gval, 0), 255);
                    bval = L_MIN(L_MAX(bval, 0), 255);
                    composeRGBPixel(rval, gval, bval, ppixel);
                    ppixel++;
                }
            }
        } else {    /* 8 bpp grayscale or colormapped pix */
            line = data + i * wpl;
            for (j = 0; j < w; j++)
                SET_DATA_BYTE(line, j, rowbuffer[j]);
        }
    }

        /* If the pixel density is neither 1 nor 2, it may not be defined.
         * In that case, don't set the resolution.  */
    if (cinfo.density_unit == 1) {  /* pixels per inch */
        pixSetXRes(pix, cinfo.X_density);
        pixSetYRes(pix, cinfo.Y_density);
    } else if (cinfo.density_unit == 2) {  /* pixels per centimeter */
        pixSetXRes(pix, (l_int32)((l_float32)cinfo.X_density * 2.54 + 0.5));
        pixSetYRes(pix, (l_int32)((l_float32)cinfo.Y_density * 2.54 + 0.5));
    }

    if (cinfo.output_components != spp)
        lept_stderr("output spp = %d, spp = %d\n",
                    cinfo.output_components, spp);

    jpeg_finish_decompress(&cinfo);
    jpeg_destroy_decompress(&cinfo);
    LEPT_FREE(rowbuffer);
    rowbuffer = NULL;
    if (pnwarn) *pnwarn = nwarn;
    if (nwarn > 0)
        L_WARNING("%d warning(s) of bad data\n", __func__, nwarn);
    return pix;
}


/*---------------------------------------------------------------------*
 *                     Read jpeg metadata from file                    *
 *---------------------------------------------------------------------*/
/*!
 * \brief   readHeaderJpeg()
 *
 * \param[in]    filename
 * \param[out]   pw     [optional]
 * \param[out]   ph     [optional]
 * \param[out]   pspp   [optional] samples/pixel
 * \param[out]   pycck  [optional] 1 if ycck color space; 0 otherwise
 * \param[out]   pcmyk  [optional] 1 if cmyk color space; 0 otherwise
 * \return  0 if OK, 1 on error
 */
l_ok
readHeaderJpeg(const char  *filename,
               l_int32     *pw,
               l_int32     *ph,
               l_int32     *pspp,
               l_int32     *pycck,
               l_int32     *pcmyk)
{
l_int32  ret;
FILE    *fp;

    if (pw) *pw = 0;
    if (ph) *ph = 0;
    if (pspp) *pspp = 0;
    if (pycck) *pycck = 0;
    if (pcmyk) *pcmyk = 0;
    if (!filename)
        return ERROR_INT("filename not defined", __func__, 1);
    if (!pw && !ph && !pspp && !pycck && !pcmyk)
        return ERROR_INT("no results requested", __func__, 1);

    if ((fp = fopenReadStream(filename)) == NULL)
        return ERROR_INT_1("image file not found", filename, __func__, 1);
    ret = freadHeaderJpeg(fp, pw, ph, pspp, pycck, pcmyk);
    fclose(fp);
    return ret;
}


/*!
 * \brief   freadHeaderJpeg()
 *
 * \param[in]    fp     file stream
 * \param[out]   pw     [optional]
 * \param[out]   ph     [optional]
 * \param[out]   pspp   [optional]  samples/pixel
 * \param[out]   pycck  [optional]  1 if ycck color space; 0 otherwise
 * \param[out]   pcmyk  [optional]  1 if cmyk color space; 0 otherwise
 * \return  0 if OK, 1 on error
 */
l_ok
freadHeaderJpeg(FILE     *fp,
                l_int32  *pw,
                l_int32  *ph,
                l_int32  *pspp,
                l_int32  *pycck,
                l_int32  *pcmyk)
{
l_int32                        spp, w, h;
struct jpeg_decompress_struct  cinfo = { 0 };
struct jpeg_error_mgr          jerr = { 0 };
jmp_buf                        jmpbuf;  /* must be local to the function */

    if (pw) *pw = 0;
    if (ph) *ph = 0;
    if (pspp) *pspp = 0;
    if (pycck) *pycck = 0;
    if (pcmyk) *pcmyk = 0;
    if (!fp)
        return ERROR_INT("stream not defined", __func__, 1);
    if (!pw && !ph && !pspp && !pycck && !pcmyk)
        return ERROR_INT("no results requested", __func__, 1);

    rewind(fp);

        /* Modify the jpeg error handling to catch fatal errors  */
    cinfo.err = jpeg_std_error(&jerr);
    cinfo.client_data = (void *)&jmpbuf;
    jerr.error_exit = jpeg_error_catch_all_1;
    if (setjmp(jmpbuf))
        return ERROR_INT("internal jpeg error", __func__, 1);

        /* Initialize the jpeg structs for reading the header */
    jpeg_create_decompress(&cinfo);
    jpeg_stdio_src(&cinfo, fp);
    jpeg_read_header(&cinfo, TRUE);
    jpeg_calc_output_dimensions(&cinfo);
    spp = cinfo.out_color_components;
    w = cinfo.output_width;
    h = cinfo.output_height;
    if (w < 1 || h < 1 || spp < 1 || spp > 4) {
        jpeg_destroy_decompress(&cinfo);
        rewind(fp);
        return ERROR_INT("bad jpeg image parameters", __func__, 1);
    }

    if (pspp) *pspp = spp;
    if (pw) *pw = cinfo.output_width;
    if (ph) *ph = cinfo.output_height;
    if (pycck) *pycck =
        (cinfo.jpeg_color_space == JCS_YCCK && spp == 4);
    if (pcmyk) *pcmyk =
        (cinfo.jpeg_color_space == JCS_CMYK && spp == 4);

    jpeg_destroy_decompress(&cinfo);
    rewind(fp);
    return 0;
}


/*
 * \brief   fgetJpegResolution()
 *
 * \param[in]    fp             file stream
 * \param[out]   pxres, pyres   resolutions
 * \return   0 if OK; 1 on error
 *
 * <pre>
 * Notes:
 *      (1) If neither resolution field is set, this is not an error;
 *          the returned resolution values are 0 (designating 'unknown').
 *      (2) Side-effect: this rewinds the stream.
 * </pre>
 */
l_int32
fgetJpegResolution(FILE     *fp,
                   l_int32  *pxres,
                   l_int32  *pyres)
{
struct jpeg_decompress_struct  cinfo = { 0 };
struct jpeg_error_mgr          jerr = { 0 };
jmp_buf                        jmpbuf;  /* must be local to the function */

    if (pxres) *pxres = 0;
    if (pyres) *pyres = 0;
    if (!pxres || !pyres)
        return ERROR_INT("&xres and &yres not both defined", __func__, 1);
    if (!fp)
        return ERROR_INT("stream not opened", __func__, 1);

    rewind(fp);

        /* Modify the jpeg error handling to catch fatal errors  */
    cinfo.err = jpeg_std_error(&jerr);
    cinfo.client_data = (void *)&jmpbuf;
    jerr.error_exit = jpeg_error_catch_all_1;
    if (setjmp(jmpbuf))
        return ERROR_INT("internal jpeg error", __func__, 1);

        /* Initialize the jpeg structs for reading the header */
    jpeg_create_decompress(&cinfo);
    jpeg_stdio_src(&cinfo, fp);
    jpeg_read_header(&cinfo, TRUE);

        /* It is common for the input resolution to be omitted from the
         * jpeg file.  If density_unit is not 1 or 2, simply return 0. */
    if (cinfo.density_unit == 1) {  /* pixels/inch */
        *pxres = cinfo.X_density;
        *pyres = cinfo.Y_density;
    } else if (cinfo.density_unit == 2) {  /* pixels/cm */
        *pxres = (l_int32)((l_float32)cinfo.X_density * 2.54 + 0.5);
        *pyres = (l_int32)((l_float32)cinfo.Y_density * 2.54 + 0.5);
    }

    jpeg_destroy_decompress(&cinfo);
    rewind(fp);
    return 0;
}


/*
 * \brief   fgetJpegComment()
 *
 * \param[in]    fp        file stream opened for read
 * \param[out]   pcomment  comment
 * \return   0 if OK; 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Side-effect: this rewinds the stream.
 * </pre>
 */
l_int32
fgetJpegComment(FILE      *fp,
                l_uint8  **pcomment)
{
struct jpeg_decompress_struct  cinfo = { 0 };
struct jpeg_error_mgr          jerr = { 0 };
struct callback_data           cb_data = { 0 };  /* contains local jmp_buf */

    if (!pcomment)
        return ERROR_INT("&comment not defined", __func__, 1);
    *pcomment = NULL;
    if (!fp)
        return ERROR_INT("stream not opened", __func__, 1);

    rewind(fp);

        /* Modify the jpeg error handling to catch fatal errors  */
    cinfo.err = jpeg_std_error(&jerr);
    jerr.error_exit = jpeg_error_catch_all_2;
    cb_data.comment = NULL;
    cinfo.client_data = (void *)&cb_data;
    if (setjmp(cb_data.jmpbuf)) {
        LEPT_FREE(cb_data.comment);
        return ERROR_INT("internal jpeg error", __func__, 1);
    }

        /* Initialize the jpeg structs for reading the header */
    jpeg_create_decompress(&cinfo);
    jpeg_set_marker_processor(&cinfo, JPEG_COM, jpeg_comment_callback);
    jpeg_stdio_src(&cinfo, fp);
    jpeg_read_header(&cinfo, TRUE);

        /* Save the result */
    *pcomment = cb_data.comment;
    jpeg_destroy_decompress(&cinfo);
    rewind(fp);
    return 0;
}


/*---------------------------------------------------------------------*
 *                             Writing Jpeg                            *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixWriteJpeg()
 *
 * \param[in]    filename
 * \param[in]    pix           any depth; cmap is OK
 * \param[in]    quality       1 - 100; 75 is default
 * \param[in]    progressive   0 for baseline sequential; 1 for progressive
 * \return  0 if OK; 1 on error
 */
l_ok
pixWriteJpeg(const char  *filename,
             PIX         *pix,
             l_int32      quality,
             l_int32      progressive)
{
FILE  *fp;

    if (!pix)
        return ERROR_INT("pix not defined", __func__, 1);
    if (!filename)
        return ERROR_INT("filename not defined", __func__, 1);

    if ((fp = fopenWriteStream(filename, "wb+")) == NULL)
        return ERROR_INT_1("stream not opened", filename, __func__, 1);

    if (pixWriteStreamJpeg(fp, pix, quality, progressive)) {
        fclose(fp);
        return ERROR_INT_1("pix not written to stream", filename, __func__, 1);
    }

    fclose(fp);
    return 0;
}


/*!
 * \brief   pixWriteStreamJpeg()
 *
 * \param[in]    fp           file stream
 * \param[in]    pixs         any depth; cmap is OK
 * \param[in]    quality      1 - 100; 75 is default value; 0 is also default
 * \param[in]    progressive  0 for baseline sequential; 1 for progressive
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Progressive encoding gives better compression, at the
 *          expense of slower encoding and decoding.
 *      (2) Standard chroma subsampling is 2x2 on both the U and V
 *          channels.  For highest quality, use no subsampling; this
 *          option is set by pixSetChromaSampling(pix, 0).
 *      (3) The only valid pixel depths in leptonica are 1, 2, 4, 8, 16
 *          and 32 bpp.  However, it is possible, and in some cases desirable,
 *          to write out a jpeg file using an rgb pix that has 24 bpp.
 *          This can be created by appending the raster data for a 24 bpp
 *          image (with proper scanline padding) directly to a 24 bpp
 *          pix that was created without a data array.
 *      (4) There are two compression paths in this function:
 *          * Grayscale image, no colormap: compress as 8 bpp image.
 *          * rgb full color image: copy each line into the color
 *            line buffer, and compress as three 8 bpp images.
 *      (5) Under the covers, the jpeg library transforms rgb to a
 *          luminance-chromaticity triple, each component of which is
 *          also 8 bits, and compresses that.  It uses 2 Huffman tables,
 *          a higher resolution one (with more quantization levels)
 *          for luminosity and a lower resolution one for the chromas.
 * </pre>
 */
l_ok
pixWriteStreamJpeg(FILE    *fp,
                   PIX     *pixs,
                   l_int32  quality,
                   l_int32  progressive)
{
l_int32                      xres, yres;
l_int32                      i, j, k;
l_int32                      w, h, d, wpl, spp, colorflag, rowsamples;
l_uint32                    *ppixel, *line, *data;
JSAMPROW                     rowbuffer;
PIX                         *pix;
struct jpeg_compress_struct  cinfo = { 0 };
struct jpeg_error_mgr        jerr = { 0 };
char                        *text;
jmp_buf                      jmpbuf;  /* must be local to the function */

    if (!fp)
        return ERROR_INT("stream not open", __func__, 1);
    if (!pixs)
        return ERROR_INT("pixs not defined", __func__, 1);
    if (quality <= 0) quality = 75;  /* default */
    if (quality > 100) {
        L_ERROR("invalid jpeg quality; setting to 75\n", __func__);
        quality = 75;
    }

        /* If necessary, convert the pix so that it can be jpeg compressed.
         * The colormap is removed based on the source, so if the colormap
         * has only gray colors, the image will be compressed with spp = 1. */
    pixGetDimensions(pixs, &w, &h, &d);
    pix = NULL;
    if (pixGetColormap(pixs) != NULL) {
        L_INFO("removing colormap; may be better to compress losslessly\n",
               __func__);
        pix = pixRemoveColormap(pixs, REMOVE_CMAP_BASED_ON_SRC);
    } else if (d >= 8 && d != 16) {  /* normal case; no rewrite */
        pix = pixClone(pixs);
    } else if (d < 8 || d == 16) {
        L_INFO("converting from %d to 8 bpp\n", __func__, d);
        pix = pixConvertTo8(pixs, 0);  /* 8 bpp, no cmap */
    } else {
        L_ERROR("unknown pix type with d = %d and no cmap\n", __func__, d);
        return 1;
    }
    if (!pix)
        return ERROR_INT("pix not made", __func__, 1);
    pixSetPadBits(pix, 0);

    rewind(fp);
    rowbuffer = NULL;

        /* Modify the jpeg error handling to catch fatal errors  */
    cinfo.err = jpeg_std_error(&jerr);
    cinfo.client_data = (void *)&jmpbuf;
    jerr.error_exit = jpeg_error_catch_all_1;
    if (setjmp(jmpbuf)) {
        LEPT_FREE(rowbuffer);
        pixDestroy(&pix);
        return ERROR_INT("internal jpeg error", __func__, 1);
    }

        /* Initialize the jpeg structs for compression */
    jpeg_create_compress(&cinfo);
    jpeg_stdio_dest(&cinfo, fp);
    cinfo.image_width  = w;
    cinfo.image_height = h;

        /* Set the color space and number of components */
    d = pixGetDepth(pix);
    if (d == 8) {
        colorflag = 0;    /* 8 bpp grayscale; no cmap */
        cinfo.input_components = 1;
        cinfo.in_color_space = JCS_GRAYSCALE;
    } else {  /* d == 32 || d == 24 */
        colorflag = 1;    /* rgb */
        cinfo.input_components = 3;
        cinfo.in_color_space = JCS_RGB;
    }

    jpeg_set_defaults(&cinfo);

        /* Setting optimize_coding to TRUE seems to improve compression
         * by approx 2-4 percent, and increases comp time by approx 20%. */
    cinfo.optimize_coding = FALSE;

        /* Set resolution in pixels/in (density_unit: 1 = in, 2 = cm) */
    xres = pixGetXRes(pix);
    yres = pixGetYRes(pix);
    if ((xres != 0) && (yres != 0)) {
        cinfo.density_unit = 1;  /* designates pixels per inch */
        cinfo.X_density = xres;
        cinfo.Y_density = yres;
    }

        /* Set the quality and progressive parameters */
    jpeg_set_quality(&cinfo, quality, TRUE);
    if (progressive)
        jpeg_simple_progression(&cinfo);

        /* Set the chroma subsampling parameters.  This is done in
         * YUV color space.  The Y (intensity) channel is never subsampled.
         * The standard subsampling is 2x2 on both the U and V channels.
         * Notation on this is confusing.  For a nice illustrations, see
         *   http://en.wikipedia.org/wiki/Chroma_subsampling
         * The standard subsampling is written as 4:2:0.
         * We allow high quality where there is no subsampling on the
         * chroma channels: denoted as 4:4:4.  */
    if (pixs->special == L_NO_CHROMA_SAMPLING_JPEG) {
        cinfo.comp_info[0].h_samp_factor = 1;
        cinfo.comp_info[0].v_samp_factor = 1;
        cinfo.comp_info[1].h_samp_factor = 1;
        cinfo.comp_info[1].v_samp_factor = 1;
        cinfo.comp_info[2].h_samp_factor = 1;
        cinfo.comp_info[2].v_samp_factor = 1;
    }

    jpeg_start_compress(&cinfo, TRUE);

        /* Cap the text the length limit, 65533, for JPEG_COM payload.
         * Just to be safe, subtract 100 to cover the Adobe name space.  */
    if ((text = pixGetText(pix)) != NULL) {
        if (strlen(text) > 65433) {
            L_WARNING("text is %zu bytes; clipping to 65433\n",
                   __func__, strlen(text));
            text[65433] = '\0';
        }
        jpeg_write_marker(&cinfo, JPEG_COM, (const JOCTET *)text, strlen(text));
    }

        /* Allocate row buffer */
    spp = cinfo.input_components;
    rowsamples = spp * w;
    if ((rowbuffer = (JSAMPROW)LEPT_CALLOC(sizeof(JSAMPLE), rowsamples))
        == NULL) {
        pixDestroy(&pix);
        return ERROR_INT("calloc fail for rowbuffer", __func__, 1);
    }

    data = pixGetData(pix);
    wpl  = pixGetWpl(pix);
    for (i = 0; i < h; i++) {
        line = data + i * wpl;
        if (colorflag == 0) {        /* 8 bpp gray */
            for (j = 0; j < w; j++)
                rowbuffer[j] = GET_DATA_BYTE(line, j);
        } else {  /* colorflag == 1 */
            if (d == 24) {  /* See note 3 above; special case of 24 bpp rgb */
                jpeg_write_scanlines(&cinfo, (JSAMPROW *)&line, 1);
            } else {  /* standard 32 bpp rgb */
                ppixel = line;
                for (j = k = 0; j < w; j++) {
                    rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_RED);
                    rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_GREEN);
                    rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_BLUE);
                    ppixel++;
                }
            }
        }
        if (d != 24)
            jpeg_write_scanlines(&cinfo, &rowbuffer, 1);
    }
    jpeg_finish_compress(&cinfo);

    pixDestroy(&pix);
    LEPT_FREE(rowbuffer);
    rowbuffer = NULL;
    jpeg_destroy_compress(&cinfo);
    return 0;
}


/*---------------------------------------------------------------------*
 *                         Read/write to memory                        *
 *---------------------------------------------------------------------*/

/*!
 * \brief   pixReadMemJpeg()
 *
 * \param[in]    data       const; jpeg-encoded
 * \param[in]    size       of data
 * \param[in]    cmflag     colormap flag 0 means return RGB image if color;
 *                          1 means create a colormap and return
 *                          an 8 bpp colormapped image if color
 * \param[in]    reduction  scaling factor: 1, 2, 4 or 8
 * \param[out]   pnwarn     [optional] number of warnings
 * \param[in]    hint       a bitwise OR of L_JPEG_* values; 0 for default
 * \return  pix, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) The %size byte of %data must be a null character.
 *      (2) The only hint flag so far is L_JPEG_READ_LUMINANCE,
 *          given in the enum in imageio.h.
 *      (3) See pixReadJpeg() for usage.
 * </pre>
 */
PIX *
pixReadMemJpeg(const l_uint8  *data,
               size_t          size,
               l_int32         cmflag,
               l_int32         reduction,
               l_int32        *pnwarn,
               l_int32         hint)
{
l_int32   ret;
l_uint8  *comment;
FILE     *fp;
PIX      *pix;

    if (pnwarn) *pnwarn = 0;
    if (!data)
        return (PIX *)ERROR_PTR("data not defined", __func__, NULL);

    if ((fp = fopenReadFromMemory(data, size)) == NULL)
        return (PIX *)ERROR_PTR("stream not opened", __func__, NULL);
    pix = pixReadStreamJpeg(fp, cmflag, reduction, pnwarn, hint);
    if (pix) {
        ret = fgetJpegComment(fp, &comment);
        if (!ret && comment) {
            pixSetText(pix, (char *)comment);
            LEPT_FREE(comment);
        }
    }
    fclose(fp);
    if (!pix) L_ERROR("pix not read\n", __func__);
    return pix;
}


/*!
 * \brief   readHeaderMemJpeg()
 *
 * \param[in]    data    const; jpeg-encoded
 * \param[in]    size    of data
 * \param[out]   pw      [optional] width
 * \param[out]   ph      [optional] height
 * \param[out]   pspp    [optional] samples/pixel
 * \param[out]   pycck   [optional] 1 if ycck color space; 0 otherwise
 * \param[out]   pcmyk   [optional] 1 if cmyk color space; 0 otherwise
 * \return  0 if OK, 1 on error
 */
l_ok
readHeaderMemJpeg(const l_uint8  *data,
                  size_t          size,
                  l_int32        *pw,
                  l_int32        *ph,
                  l_int32        *pspp,
                  l_int32        *pycck,
                  l_int32        *pcmyk)
{
l_int32  ret;
FILE    *fp;

    if (pw) *pw = 0;
    if (ph) *ph = 0;
    if (pspp) *pspp = 0;
    if (pycck) *pycck = 0;
    if (pcmyk) *pcmyk = 0;
    if (!data)
        return ERROR_INT("data not defined", __func__, 1);
    if (!pw && !ph && !pspp && !pycck && !pcmyk)
        return ERROR_INT("no results requested", __func__, 1);

    if ((fp = fopenReadFromMemory(data, size)) == NULL)
        return ERROR_INT("stream not opened", __func__, 1);
    ret = freadHeaderJpeg(fp, pw, ph, pspp, pycck, pcmyk);
    fclose(fp);
    return ret;
}


/*!
 * \brief   readResolutionMemJpeg()
 *
 * \param[in]   data    const; jpeg-encoded
 * \param[in]   size    of data
 * \param[out]  pxres   [optional]
 * \param[out]  pyres   [optional]
 * \return  0 if OK, 1 on error
 */
l_ok
readResolutionMemJpeg(const l_uint8  *data,
                      size_t          size,
                      l_int32        *pxres,
                      l_int32        *pyres)
{
l_int32  ret;
FILE    *fp;

    if (pxres) *pxres = 0;
    if (pyres) *pyres = 0;
    if (!data)
        return ERROR_INT("data not defined", __func__, 1);
    if (!pxres && !pyres)
        return ERROR_INT("no results requested", __func__, 1);

    if ((fp = fopenReadFromMemory(data, size)) == NULL)
        return ERROR_INT("stream not opened", __func__, 1);
    ret = fgetJpegResolution(fp, pxres, pyres);
    fclose(fp);
    return ret;
}


/*!
 * \brief   pixWriteMemJpeg()
 *
 * \param[out]   pdata        data of jpeg compressed image
 * \param[out]   psize        size of returned data
 * \param[in]    pix          any depth; cmap is OK
 * \param[in]    quality      1 - 100; 75 is default value; 0 is also default
 * \param[in]    progressive  0 for baseline sequential; 1 for progressive
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) See pixWriteStreamJpeg() for usage.  This version writes to
 *          memory instead of to a file stream.
 * </pre>
 */
l_ok
pixWriteMemJpeg(l_uint8  **pdata,
                size_t    *psize,
                PIX       *pix,
                l_int32    quality,
                l_int32    progressive)
{
l_int32  ret;
FILE    *fp;

    if (pdata) *pdata = NULL;
    if (psize) *psize = 0;
    if (!pdata)
        return ERROR_INT("&data not defined", __func__, 1 );
    if (!psize)
        return ERROR_INT("&size not defined", __func__, 1 );
    if (!pix)
        return ERROR_INT("&pix not defined", __func__, 1 );

#if HAVE_FMEMOPEN
    if ((fp = open_memstream((char **)pdata, psize)) == NULL)
        return ERROR_INT("stream not opened", __func__, 1);
    ret = pixWriteStreamJpeg(fp, pix, quality, progressive);
    fputc('\0', fp);
    fclose(fp);
    if (*psize > 0) *psize = *psize - 1;
#else
    L_INFO("no fmemopen API --> work-around: write to temp file\n", __func__);
  #ifdef _WIN32
    if ((fp = fopenWriteWinTempfile()) == NULL)
        return ERROR_INT("tmpfile stream not opened", __func__, 1);
  #else
    if ((fp = tmpfile()) == NULL)
        return ERROR_INT("tmpfile stream not opened", __func__, 1);
  #endif  /* _WIN32 */
    ret = pixWriteStreamJpeg(fp, pix, quality, progressive);
    rewind(fp);
    *pdata = l_binaryReadStream(fp, psize);
    fclose(fp);
#endif  /* HAVE_FMEMOPEN */
    return ret;
}


/*---------------------------------------------------------------------*
 *           Setting special flag for chroma sampling on write         *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixSetChromaSampling()
 *
 * \param[in]    pix
 * \param[in]    sampling    1 for subsampling; 0 for no subsampling
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) The default is for 2x2 chroma subsampling because the files are
 *          considerably smaller and the appearance is typically satisfactory.
 *          To get full resolution output in the chroma channels for
 *          jpeg writing, call this with %sampling == 0.
 * </pre>
 */
l_ok
pixSetChromaSampling(PIX     *pix,
                     l_int32  sampling)
{
    if (!pix)
        return ERROR_INT("pix not defined", __func__, 1 );
    if (sampling)
        pixSetSpecial(pix, 0);  /* default */
    else
        pixSetSpecial(pix, L_NO_CHROMA_SAMPLING_JPEG);
    return 0;
}


/*---------------------------------------------------------------------*
 *                        Static system helpers                        *
 *---------------------------------------------------------------------*/
/*!
 * \brief   jpeg_error_catch_all_1()
 *
 *  Notes:
 *      (1) The default jpeg error_exit() kills the process, but we
 *          never want a call to leptonica to kill a process.  If you
 *          do want this behavior, remove the calls to these error handlers.
 *      (2) This is used where cinfo->client_data holds only jmpbuf.
 */
static void
jpeg_error_catch_all_1(j_common_ptr cinfo)
{
    jmp_buf *pjmpbuf = (jmp_buf *)cinfo->client_data;
    (*cinfo->err->output_message) (cinfo);
    jpeg_destroy(cinfo);
    longjmp(*pjmpbuf, 1);
}

/*!
 * \brief   jpeg_error_catch_all_2()
 *
 *  Notes:
 *      (1) This is used where cinfo->client_data needs to hold both
 *          the jmpbuf and the jpeg comment data.
 *      (2) On error, the comment data will be freed by the caller.
 */
static void
jpeg_error_catch_all_2(j_common_ptr cinfo)
{
struct callback_data  *pcb_data;

    pcb_data = (struct callback_data *)cinfo->client_data;
    (*cinfo->err->output_message) (cinfo);
    jpeg_destroy(cinfo);
    longjmp(pcb_data->jmpbuf, 1);
}

/* This function was borrowed from libjpeg */
static l_uint8
jpeg_getc(j_decompress_ptr cinfo)
{
struct jpeg_source_mgr *datasrc;

    datasrc = cinfo->src;
    if (datasrc->bytes_in_buffer == 0) {
        if (! (*datasrc->fill_input_buffer) (cinfo)) {
            return 0;
        }
    }
    datasrc->bytes_in_buffer--;
    return GETJOCTET(*datasrc->next_input_byte++);
}

/*!
 * \brief   jpeg_comment_callback()
 *
 *  Notes:
 *      (1) This is used to read the jpeg comment (JPEG_COM).
 *          See the note above the declaration for why it returns
 *          a "boolean".
 */
static boolean
jpeg_comment_callback(j_decompress_ptr cinfo)
{
l_int32                length, i;
l_uint8               *comment;
struct callback_data  *pcb_data;

        /* Get the size of the comment */
    length = jpeg_getc(cinfo) << 8;
    length += jpeg_getc(cinfo);
    length -= 2;
    if (length <= 0)
        return 1;

        /* Extract the comment from the file */
    if ((comment = (l_uint8 *)LEPT_CALLOC(length + 1, sizeof(l_uint8))) == NULL)
        return 0;
    for (i = 0; i < length; i++)
        comment[i] = jpeg_getc(cinfo);

        /* Save the comment and return */
    pcb_data = (struct callback_data *)cinfo->client_data;
    if (pcb_data->comment) {  /* clear before overwriting previous comment */
        LEPT_FREE(pcb_data->comment);
        pcb_data->comment = NULL;
    }
    pcb_data->comment = comment;
    return 1;
}

/* --------------------------------------------*/
#endif  /* HAVE_LIBJPEG */
/* --------------------------------------------*/
