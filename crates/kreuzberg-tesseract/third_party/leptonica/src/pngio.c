/*====================================================================*
 -  Copyright (C) 2001 Leptonica.  All rights reserved.
 -  Copyright (C) 2017 Milner Technologies, Inc.
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
 * \file pngio.c
 * <pre>
 *
 *    Reading png through stream
 *          PIX        *pixReadStreamPng()
 *
 *    Reading png header
 *          l_int32     readHeaderPng()
 *          l_int32     freadHeaderPng()
 *          l_int32     readHeaderMemPng()
 *
 *    Reading png metadata
 *          l_int32     fgetPngResolution()
 *          l_int32     isPngInterlaced()
 *          l_int32     fgetPngColormapInfo()
 *
 *    Writing png through stream
 *          l_int32     pixWritePng()  [ special top level ]
 *          l_int32     pixWriteStreamPng()
 *          l_int32     pixSetZlibCompression()
 *
 *    Set flag for special read mode
 *          void        l_pngSetReadStrip16To8()
 *
 *    Low-level memio utility (thanks to T. D. Hintz)
 *          static void memio_png_write_data()
 *          static void memio_png_flush()
 *          static void memio_png_read_data()
 *          static void memio_free()
 *
 *    Reading png from memory
 *          PIX        *pixReadMemPng()
 *
 *    Writing png to memory
 *          l_int32     pixWriteMemPng()
 *
 *    Documentation: libpng.txt and example.c
 *
 *    On input (decompression from file), palette color images
 *    are read into an 8 bpp Pix with a colormap, and 24 bpp
 *    3 component color images are read into a 32 bpp Pix with
 *    rgb samples.  On output (compression to file), palette color
 *    images are written as 8 bpp with the colormap, and 32 bpp
 *    full color images are written compressed as a 24 bpp,
 *    3 component color image.
 *
 *    In the following, we use these abbreviations:
 *       bps == bit/sample
 *       spp == samples/pixel
 *       bpp == bits/pixel of image in Pix (memory)
 *    where each component is referred to as a "sample".
 *
 *    For reading and writing rgb and rgba images, we read and write
 *    alpha if it exists (spp == 4) and do not read or write if
 *    it doesn't (spp == 3).  The alpha component can be 'removed'
 *    simply by setting spp to 3.  In leptonica, we make relatively
 *    little explicit use of the alpha sample.  Note that the alpha
 *    sample in the image is also called "alpha transparency",
 *    "alpha component" and "alpha layer."
 *
 *    To change the zlib compression level, use pixSetZlibCompression()
 *    before writing the file.  The default is for standard png compression.
 *    The zlib compression value can be set [0 ... 9], with
 *         0     no compression (huge files)
 *         1     fastest compression
 *         -1    default compression  (equivalent to 6 in latest version)
 *         9     best compression
 *    Note that if you are using the defined constants in zlib instead
 *    of the compression integers given above, you must include zlib.h.
 *
 *    There is global for determining the size of retained samples:
 *             var_PNG_STRIP_16_to_8
 *    and a function l_pngSetReadStrip16To8() for setting it.
 *    The default is TRUE, which causes pixRead() to strip each 16 bit
 *    sample down to 8 bps:
 *     ~ For 16 bps rgb (16 bps, 3 spp) --> 32 bpp rgb Pix
 *     ~ For 16 bps gray (16 bps, 1 spp) --> 8 bpp grayscale Pix
 *    If the variable is set to FALSE, the 16 bit gray samples
 *    are saved when read; the 16 bit rgb samples return an error.
 *    Note: results can be non-deterministic if used with
 *    multi-threaded applications.
 *
 *    Thanks to a memory buffering utility contributed by T. D. Hintz,
 *    encoding png directly into memory (and decoding from memory)
 *    is now enabled without the use of any temp files.  Unlike with webp,
 *    it is necessary to preserve the stream interface to enable writing
 *    pixa to memory.  So there are two independent but very similar
 *    implementations of png reading and writing.
 * </pre>
 */

#ifdef  HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"
#include "pix_internal.h"

/* --------------------------------------------*/
#if  HAVE_LIBPNG   /* defined in environ.h */
/* --------------------------------------------*/

#include "png.h"

#if  HAVE_LIBZ
#include "zlib.h"
#else
#define  Z_DEFAULT_COMPRESSION (-1)
#endif  /* HAVE_LIBZ */

/* ------------------ Set default for read option -------------------- */
    /* Strip 16 bpp --> 8 bpp on reading png; default is for stripping.
     * If you don't strip, you can't read the gray-alpha spp = 2 images. */
static l_int32   var_PNG_STRIP_16_TO_8 = 1;

#ifndef  NO_CONSOLE_IO
#define  DEBUG_READ     0
#define  DEBUG_WRITE    0
#endif  /* ~NO_CONSOLE_IO */


/*---------------------------------------------------------------------*
 *                     Reading png through stream                      *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixReadStreamPng()
 *
 * \param[in]    fp file stream
 * \return  pix, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) If called from pixReadStream(), the stream is positioned
 *          at the beginning of the file.
 *      (2) To do sequential reads of png format images from a stream,
 *          use pixReadStreamPng()
 *      (3) All images with alpha is converted to RGBA (spp = 4, with
 *          equal red, green and blue channels) on reading.
 *          There are three cases with alpha:
 *          (a) RGBA: spp = 4.  The alpha value is the fourth byte
 *              (aka "channel, "component") in each 4-byte pixel.
 *          (b) grayscale-with-alpha (spp = 2), where bpp = 8, and each
 *              pixel has an associated alpha (transparency) value
 *              in the second component of the image data.
 *          (c) colormap (spp = 1) with alpha in the trans palette.
 *              d = 1, 2, 4, 8.  The trans palette in writing is derived
 *              from the alpha components in the cmap.  Transparency is
 *              often associated with a white background.
 *      (4) We use the high level png interface, where the transforms are set
 *          up in advance and the header and image are read with a single
 *          call.  The more complicated interface, where the header is
 *          read first and the buffers for the raster image are user-
 *          allocated before reading the image, works for single images,
 *          but I could not get it to work properly for the successive
 *          png reads that are required by pixaReadStream().
 * </pre>
 */
PIX *
pixReadStreamPng(FILE  *fp)
{
l_uint8      byte;
l_int32      i, j, k, index, ncolors, rval, gval, bval, valid;
l_int32      wpl, d, spp, cindex, bitval, bival, quadval, tRNS;
l_uint32     png_transforms;
l_uint32    *data, *line, *ppixel;
int          num_palette, num_text, num_trans;
png_byte     bit_depth, color_type, channels;
png_uint_32  w, h, rowbytes, xres, yres;
png_bytep    rowptr, trans;
png_bytep   *row_pointers;
png_structp  png_ptr;
png_infop    info_ptr, end_info;
png_colorp   palette;
png_textp    text_ptr;  /* ptr to text_chunk */
PIX         *pix, *pix1;
PIXCMAP     *cmap;

    if (!fp)
        return (PIX *)ERROR_PTR("fp not defined", __func__, NULL);
    pix = NULL;

        /* Allocate the 3 data structures */
    if ((png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING,
                   (png_voidp)NULL, NULL, NULL)) == NULL)
        return (PIX *)ERROR_PTR("png_ptr not made", __func__, NULL);

    if ((info_ptr = png_create_info_struct(png_ptr)) == NULL) {
        png_destroy_read_struct(&png_ptr, (png_infopp)NULL, (png_infopp)NULL);
        return (PIX *)ERROR_PTR("info_ptr not made", __func__, NULL);
    }

    if ((end_info = png_create_info_struct(png_ptr)) == NULL) {
        png_destroy_read_struct(&png_ptr, &info_ptr, (png_infopp)NULL);
        return (PIX *)ERROR_PTR("end_info not made", __func__, NULL);
    }

        /* Set up png setjmp error handling */
    if (setjmp(png_jmpbuf(png_ptr))) {
        png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
        return (PIX *)ERROR_PTR("internal png error", __func__, NULL);
    }

    png_init_io(png_ptr, fp);

        /* ---------------------------------------------------------- *
         *  - Set the transforms flags.  Whatever happens here,
         *    NEVER invert 1 bpp using PNG_TRANSFORM_INVERT_MONO.
         *  - Do not use PNG_TRANSFORM_EXPAND, which would
         *    expand all images with bpp < 8 to 8 bpp.
         *  - Strip 16 --> 8 if reading 16-bit gray+alpha
         * ---------------------------------------------------------- */
        /* To strip 16 --> 8 bit depth, use PNG_TRANSFORM_STRIP_16 */
    if (var_PNG_STRIP_16_TO_8 == 1) {  /* our default */
        png_transforms = PNG_TRANSFORM_STRIP_16;
    } else {
        png_transforms = PNG_TRANSFORM_IDENTITY;
        L_INFO("not stripping 16 --> 8 in png reading\n", __func__);
    }

        /* Read it */
    png_read_png(png_ptr, info_ptr, png_transforms, NULL);

    row_pointers = png_get_rows(png_ptr, info_ptr);
    w = png_get_image_width(png_ptr, info_ptr);
    h = png_get_image_height(png_ptr, info_ptr);
    bit_depth = png_get_bit_depth(png_ptr, info_ptr);
    rowbytes = png_get_rowbytes(png_ptr, info_ptr);
    color_type = png_get_color_type(png_ptr, info_ptr);
    channels = png_get_channels(png_ptr, info_ptr);
    spp = channels;
    tRNS = png_get_valid(png_ptr, info_ptr, PNG_INFO_tRNS) ? 1 : 0;

    if (spp == 1) {
        d = bit_depth;
    } else {  /* spp == 2 (gray + alpha), spp == 3 (rgb), spp == 4 (rgba) */
        d = 4 * bit_depth;
    }

        /* Remove if/when this is implemented for all bit_depths */
    if (spp != 1 && bit_depth != 8) {
        L_ERROR("spp = %d and bps = %d != 8\n"
                "turn on 16 --> 8 stripping\n", __func__, spp, bit_depth);
        png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
        return (PIX *)ERROR_PTR("not implemented for this image",
            __func__, NULL);
    }

    cmap = NULL;
    if (color_type == PNG_COLOR_TYPE_PALETTE ||
        color_type == PNG_COLOR_MASK_PALETTE) {   /* generate a colormap */
        png_get_PLTE(png_ptr, info_ptr, &palette, &num_palette);
        cmap = pixcmapCreate(d);  /* spp == 1 */
        for (cindex = 0; cindex < num_palette; cindex++) {
            rval = palette[cindex].red;
            gval = palette[cindex].green;
            bval = palette[cindex].blue;
            pixcmapAddColor(cmap, rval, gval, bval);
        }
    }

    if ((pix = pixCreate(w, h, d)) == NULL) {
        pixcmapDestroy(&cmap);
        png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
        return (PIX *)ERROR_PTR("pix not made", __func__, NULL);
    }
    pixSetInputFormat(pix, IFF_PNG);
    wpl = pixGetWpl(pix);
    data = pixGetData(pix);
    pixSetSpp(pix, spp);
    if (pixSetColormap(pix, cmap)) {
        pixDestroy(&pix);
        return (PIX *)ERROR_PTR("invalid colormap", __func__, NULL);
    }

    if (spp == 1 && !tRNS) {  /* copy straight from buffer to pix */
        for (i = 0; i < h; i++) {
            line = data + i * wpl;
            rowptr = row_pointers[i];
            for (j = 0; j < rowbytes; j++) {
                    SET_DATA_BYTE(line, j, rowptr[j]);
            }
        }
    } else if (spp == 2) {  /* grayscale + alpha; convert to RGBA */
        L_INFO("converting (gray + alpha) ==> RGBA\n", __func__);
        for (i = 0; i < h; i++) {
            ppixel = data + i * wpl;
            rowptr = row_pointers[i];
            for (j = k = 0; j < w; j++) {
                    /* Copy gray value into r, g and b */
                SET_DATA_BYTE(ppixel, COLOR_RED, rowptr[k]);
                SET_DATA_BYTE(ppixel, COLOR_GREEN, rowptr[k]);
                SET_DATA_BYTE(ppixel, COLOR_BLUE, rowptr[k++]);
                SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL, rowptr[k++]);
                ppixel++;
            }
        }
        pixSetSpp(pix, 4);  /* we do not support 2 spp pix */
    } else if (spp == 3 || spp == 4) {
        for (i = 0; i < h; i++) {
            ppixel = data + i * wpl;
            rowptr = row_pointers[i];
            for (j = k = 0; j < w; j++) {
                SET_DATA_BYTE(ppixel, COLOR_RED, rowptr[k++]);
                SET_DATA_BYTE(ppixel, COLOR_GREEN, rowptr[k++]);
                SET_DATA_BYTE(ppixel, COLOR_BLUE, rowptr[k++]);
                if (spp == 3)  /* set to opaque; some readers are buggy */
                    SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL, 255);
                else  /* spp == 4 */
                    SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL, rowptr[k++]);
                ppixel++;
            }
        }
    }

        /* Special spp == 1 cases with transparency:
         *    (1) 8 bpp without colormap; assume full transparency
         *    (2) 1 bpp with colormap + trans array (for alpha)
         *    (3) 2 bpp with colormap + trans array (for alpha)
         *    (4) 4 bpp with colormap + trans array (for alpha)
         *    (5) 8 bpp with colormap + trans array (for alpha)
         * These all require converting to RGBA */
    if (spp == 1 && tRNS) {
        if (!cmap) {
                /* Case 1: make fully transparent RGBA image */
            L_INFO("transparency, 1 spp, no colormap, no transparency array: "
                   "convention is fully transparent image\n", __func__);
            L_INFO("converting (fully transparent 1 spp) ==> RGBA\n", __func__);
            pixDestroy(&pix);
            pix = pixCreate(w, h, 32);  /* init to alpha = 0 (transparent) */
            pixSetSpp(pix, 4);
        } else {
            L_INFO("converting (cmap + alpha) ==> RGBA\n", __func__);

                /* Grab the transparency array */
            png_get_tRNS(png_ptr, info_ptr, &trans, &num_trans, NULL);
            if (!trans) {  /* invalid png file */
                pixDestroy(&pix);
                png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
                return (PIX *)ERROR_PTR("cmap, tRNS, but no transparency array",
                                        __func__, NULL);
            }

                /* Save the cmap and destroy the pix */
            cmap = pixcmapCopy(pixGetColormap(pix));
            ncolors = pixcmapGetCount(cmap);
            pixDestroy(&pix);

                /* Start over with 32 bit RGBA */
            pix = pixCreate(w, h, 32);
            wpl = pixGetWpl(pix);
            data = pixGetData(pix);
            pixSetSpp(pix, 4);

#if DEBUG_READ
            lept_stderr("ncolors = %d, num_trans = %d\n",
                        ncolors, num_trans);
            for (i = 0; i < ncolors; i++) {
                pixcmapGetColor(cmap, i, &rval, &gval, &bval);
                if (i < num_trans) {
                    lept_stderr("(r,g,b,a) = (%d,%d,%d,%d)\n",
                                rval, gval, bval, trans[i]);
                } else {
                    lept_stderr("(r,g,b,a) = (%d,%d,%d,<<255>>)\n",
                                rval, gval, bval);
                }
            }
#endif  /* DEBUG_READ */

                /* Extract the data and convert to RGBA */
            if (d == 1) {
                    /* Case 2: 1 bpp with transparency (usually) behind white */
                L_INFO("converting 1 bpp cmap with alpha ==> RGBA\n", __func__);
                if (num_trans == 1)
                    L_INFO("num_trans = 1; second color opaque by default\n",
                           __func__);
                for (i = 0; i < h; i++) {
                    ppixel = data + i * wpl;
                    rowptr = row_pointers[i];
                    for (j = 0, index = 0; j < rowbytes; j++) {
                        byte = rowptr[j];
                        for (k = 0; k < 8 && index < w; k++, index++) {
                            bitval = (byte >> (7 - k)) & 1;
                            pixcmapGetColor(cmap, bitval, &rval, &gval, &bval);
                            composeRGBPixel(rval, gval, bval, ppixel);
                            SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL,
                                      bitval < num_trans ? trans[bitval] : 255);
                            ppixel++;
                        }
                    }
                }
            } else if (d == 2) {
                    /* Case 3: 2 bpp with cmap and associated transparency */
                L_INFO("converting 2 bpp cmap with alpha ==> RGBA\n", __func__);
                for (i = 0; i < h; i++) {
                    ppixel = data + i * wpl;
                    rowptr = row_pointers[i];
                    for (j = 0, index = 0; j < rowbytes; j++) {
                        byte = rowptr[j];
                        for (k = 0; k < 4 && index < w; k++, index++) {
                            bival = (byte >> 2 * (3 - k)) & 3;
                            pixcmapGetColor(cmap, bival, &rval, &gval, &bval);
                            composeRGBPixel(rval, gval, bval, ppixel);
                                /* Assume missing entries to be 255 (opaque)
                                 * according to the spec:
                                 * http://www.w3.org/TR/PNG/#11tRNS */
                            SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL,
                                bival < num_trans ? trans[bival] : 255);
                            ppixel++;
                        }
                    }
                }
            } else if (d == 4) {
                    /* Case 4: 4 bpp with cmap and associated transparency */
                L_INFO("converting 4 bpp cmap with alpha ==> RGBA\n", __func__);
                for (i = 0; i < h; i++) {
                    ppixel = data + i * wpl;
                    rowptr = row_pointers[i];
                    for (j = 0, index = 0; j < rowbytes; j++) {
                        byte = rowptr[j];
                        for (k = 0; k < 2 && index < w; k++, index++) {
                            quadval = (byte >> 4 * (1 - k)) & 0xf;
                            pixcmapGetColor(cmap, quadval, &rval, &gval, &bval);
                            composeRGBPixel(rval, gval, bval, ppixel);
                                /* Assume missing entries to be 255 (opaque) */
                            SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL,
                                quadval < num_trans ? trans[quadval] : 255);
                            ppixel++;
                        }
                    }
                }
            } else if (d == 8) {
                    /* Case 5: 8 bpp with cmap and associated transparency */
                L_INFO("converting 8 bpp cmap with alpha ==> RGBA\n", __func__);
                for (i = 0; i < h; i++) {
                    ppixel = data + i * wpl;
                    rowptr = row_pointers[i];
                    for (j = 0; j < w; j++) {
                        index = rowptr[j];
                        pixcmapGetColor(cmap, index, &rval, &gval, &bval);
                        composeRGBPixel(rval, gval, bval, ppixel);
                            /* Assume missing entries to be 255 (opaque) */
                        SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL,
                                      index < num_trans ? trans[index] : 255);
                        ppixel++;
                    }
                }
            } else {
                L_ERROR("spp == 1, cmap, trans array, invalid depth: %d\n",
                        __func__, d);
            }
            pixcmapDestroy(&cmap);
        }
    }

#if  DEBUG_READ
    if (cmap) {
        for (i = 0; i < 16; i++) {
            lept_stderr("[%d] = %d\n", i, ((l_uint8 *)(cmap->array))[i]);
        }
    }
#endif  /* DEBUG_READ */

        /* Final adjustments for bpp = 1.
         *   + If there is no colormap, the image must be inverted because
         *     png stores black pixels as 0.
         *   + We have already handled the case of cmapped, 1 bpp pix
         *     with transparency, where the output pix is 32 bpp RGBA.
         *     If there is no transparency but the pix has a colormap,
         *     we remove the colormap, because functions operating on
         *     1 bpp images in leptonica assume no colormap.
         *   + The colormap must be removed in such a way that the pixel
         *     values are not changed.  If the values are only black and
         *     white, we return a 1 bpp image; if gray, return an 8 bpp pix;
         *     otherwise, return a 32 bpp rgb pix.
         *
         * Note that we cannot use the PNG_TRANSFORM_INVERT_MONO flag
         * to do the inversion, because that flag (since version 1.0.9)
         * inverts 8 bpp grayscale as well, which we don't want to do.
         * (It also doesn't work if there is a colormap.)
         *
         * Note that if the input png is a 1-bit with colormap and
         * transparency, it has already been rendered as a 32 bpp,
         * spp = 4 rgba pix.
         */
    if (pixGetDepth(pix) == 1) {
        if (!cmap) {
            pixInvert(pix, pix);
        } else {
            L_INFO("removing opaque cmap from 1 bpp\n", __func__);
            pix1 = pixRemoveColormap(pix, REMOVE_CMAP_BASED_ON_SRC);
            pixDestroy(&pix);
            pix = pix1;
        }
    }

    xres = png_get_x_pixels_per_meter(png_ptr, info_ptr);
    yres = png_get_y_pixels_per_meter(png_ptr, info_ptr);
    pixSetXRes(pix, (l_int32)((l_float32)xres / 39.37 + 0.5));  /* to ppi */
    pixSetYRes(pix, (l_int32)((l_float32)yres / 39.37 + 0.5));  /* to ppi */

        /* Get the text if there is any */
    png_get_text(png_ptr, info_ptr, &text_ptr, &num_text);
    if (num_text && text_ptr)
        pixSetText(pix, text_ptr->text);

    png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);

        /* Final validity check on the colormap */
    if ((cmap = pixGetColormap(pix)) != NULL) {
        pixcmapIsValid(cmap, pix, &valid);
        if (!valid) {
            pixDestroy(&pix);
            return (PIX *)ERROR_PTR("colormap is not valid", __func__, NULL);
        }
    }

    pixSetPadBits(pix, 0);
    return pix;
}


/*---------------------------------------------------------------------*
 *                          Reading png header                         *
 *---------------------------------------------------------------------*/
/*!
 * \brief   readHeaderPng()
 *
 * \param[in]    filename
 * \param[out]   pw      [optional]
 * \param[out]   ph      [optional]
 * \param[out]   pbps    [optional]  bits/sample
 * \param[out]   pspp    [optional]  samples/pixel
 * \param[out]   piscmap [optional]
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) If there is a colormap, iscmap is returned as 1; else 0.
 *      (2) For gray+alpha, although the png records bps = 16, we
 *          consider this as two 8 bpp samples (gray and alpha).
 *          When a gray+alpha is read, it is converted to 32 bpp RGBA.
 * </pre>
 */
l_ok
readHeaderPng(const char *filename,
              l_int32    *pw,
              l_int32    *ph,
              l_int32    *pbps,
              l_int32    *pspp,
              l_int32    *piscmap)
{
l_int32  ret;
FILE    *fp;

    if (pw) *pw = 0;
    if (ph) *ph = 0;
    if (pbps) *pbps = 0;
    if (pspp) *pspp = 0;
    if (piscmap) *piscmap = 0;
    if (!filename)
        return ERROR_INT("filename not defined", __func__, 1);
    if ((fp = fopenReadStream(filename)) == NULL)
        return ERROR_INT_1("image file not found", filename, __func__, 1);
    ret = freadHeaderPng(fp, pw, ph, pbps, pspp, piscmap);
    fclose(fp);
    return ret;
}


/*!
 * \brief   freadHeaderPng()
 *
 * \param[in]    fp       file stream
 * \param[out]   pw       [optional]
 * \param[out]   ph       [optional]
 * \param[out]   pbps     [optional]  bits/sample
 * \param[out]   pspp     [optional]  samples/pixel
 * \param[out]   piscmap  [optional]
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) See readHeaderPng().  We only need the first 40 bytes in the file.
 * </pre>
 */
l_ok
freadHeaderPng(FILE     *fp,
               l_int32  *pw,
               l_int32  *ph,
               l_int32  *pbps,
               l_int32  *pspp,
               l_int32  *piscmap)
{
l_int32  nbytes, ret;
l_uint8  data[40];

    if (pw) *pw = 0;
    if (ph) *ph = 0;
    if (pbps) *pbps = 0;
    if (pspp) *pspp = 0;
    if (piscmap) *piscmap = 0;
    if (!fp)
        return ERROR_INT("stream not defined", __func__, 1);

    nbytes = fnbytesInFile(fp);
    if (nbytes < 40)
        return ERROR_INT("file too small to be png", __func__, 1);
    if (fread(data, 1, 40, fp) != 40)
        return ERROR_INT("error reading data", __func__, 1);
    ret = readHeaderMemPng(data, 40, pw, ph, pbps, pspp, piscmap);
    return ret;
}


/*!
 * \brief   readHeaderMemPng()
 *
 * \param[in]    data
 * \param[in]    size    40 bytes is sufficient
 * \param[out]   pw      [optional]
 * \param[out]   ph      [optional]
 * \param[out]   pbps    [optional]  bits/sample
 * \param[out]   pspp    [optional]  samples/pixel
 * \param[out]   piscmap [optional]  input NULL to ignore
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) See readHeaderPng().
 *      (2) png colortypes (see png.h: PNG_COLOR_TYPE_*):
 *          0:  gray; fully transparent (with tRNS) (1 spp)
 *          2:  RGB (3 spp)
 *          3:  colormap; colormap+alpha (with tRNS) (1 spp)
 *          4:  gray + alpha (2 spp)
 *          6:  RGBA (4 spp)
 *          Note:
 *            0 and 3 have the alpha information in a tRNS chunk
 *            4 and 6 have separate alpha samples with each pixel.
 * </pre>
 */
l_ok
readHeaderMemPng(const l_uint8  *data,
                 size_t          size,
                 l_int32        *pw,
                 l_int32        *ph,
                 l_int32        *pbps,
                 l_int32        *pspp,
                 l_int32        *piscmap)
{
l_uint16   twobytes;
l_uint16  *pshort;
l_int32    colortype, w, h, bps, spp;
l_uint32  *pword;

    if (pw) *pw = 0;
    if (ph) *ph = 0;
    if (pbps) *pbps = 0;
    if (pspp) *pspp = 0;
    if (piscmap) *piscmap = 0;
    if (!data)
        return ERROR_INT("data not defined", __func__, 1);
    if (size < 40)
        return ERROR_INT("size < 40", __func__, 1);

        /* Check password */
    if (data[0] != 137 || data[1] != 80 || data[2] != 78 ||
        data[3] != 71 || data[4] != 13 || data[5] != 10 ||
        data[6] != 26 || data[7] != 10)
        return ERROR_INT("not a valid png file", __func__, 1);

    pword = (l_uint32 *)data;
    pshort = (l_uint16 *)data;
    w = convertOnLittleEnd32(pword[4]);
    h = convertOnLittleEnd32(pword[5]);
    if (w < 1 || h < 1)
        return ERROR_INT("invalid w or h", __func__, 1);
    twobytes = convertOnLittleEnd16(pshort[12]); /* contains depth/sample  */
                                                 /* and the color type     */
    colortype = twobytes & 0xff;  /* color type */
    bps = twobytes >> 8;   /* bits/sample */

        /* Special case with alpha that is extracted as RGBA.
         * Note that the cmap+alpha is also extracted as RGBA,
         * but only if the tRNS chunk exists, which we can't tell
         * by this simple parser.*/
    if (colortype == 4)
        L_INFO("gray + alpha: will extract as RGBA (spp = 4)\n", __func__);

    if (colortype == 2) {  /* RGB */
        spp = 3;
    } else if (colortype == 6) {  /* RGBA */
        spp = 4;
    } else if (colortype == 4) {  /* gray + alpha */
        spp = 2;
        bps = 8;  /* both the gray and alpha are 8-bit samples */
    } else {  /* gray (0) or cmap (3) or cmap+alpha (3) */
        spp = 1;
    }
    if (bps < 1 || bps > 16) {
        L_ERROR("invalid bps = %d\n", __func__, bps);
        return 1;
    }
    if (pw) *pw = w;
    if (ph) *ph = h;
    if (pbps) *pbps = bps;
    if (pspp) *pspp = spp;
    if (piscmap) {
        if (colortype & 1)  /* palette */
            *piscmap = 1;
        else
            *piscmap = 0;
    }

    return 0;
}


/*---------------------------------------------------------------------*
 *                         Reading png metadata                        *
 *---------------------------------------------------------------------*/
/*
 *  fgetPngResolution()
 *
 *      Input:  fp (file stream opened for read)
 *              &xres, &yres (<return> resolution in ppi)
 *      Return: 0 if OK; 1 on error
 *
 *  Notes:
 *      (1) If neither resolution field is set, this is not an error;
 *          the returned resolution values are 0 (designating 'unknown').
 *      (2) Side-effect: this rewinds the stream.
 */
l_int32
fgetPngResolution(FILE     *fp,
                  l_int32  *pxres,
                  l_int32  *pyres)
{
png_uint_32  xres, yres;
png_structp  png_ptr;
png_infop    info_ptr;

    if (pxres) *pxres = 0;
    if (pyres) *pyres = 0;
    if (!fp)
        return ERROR_INT("stream not opened", __func__, 1);
    if (!pxres || !pyres)
        return ERROR_INT("&xres and &yres not both defined", __func__, 1);

       /* Make the two required structs */
    if ((png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING,
                   (png_voidp)NULL, NULL, NULL)) == NULL)
        return ERROR_INT("png_ptr not made", __func__, 1);
    if ((info_ptr = png_create_info_struct(png_ptr)) == NULL) {
        png_destroy_read_struct(&png_ptr, (png_infopp)NULL, (png_infopp)NULL);
        return ERROR_INT("info_ptr not made", __func__, 1);
    }

        /* Set up png setjmp error handling.
         * Without this, an error calls exit. */
    if (setjmp(png_jmpbuf(png_ptr))) {
        png_destroy_read_struct(&png_ptr, &info_ptr, (png_infopp)NULL);
        return ERROR_INT("internal png error", __func__, 1);
    }

        /* Read the metadata */
    rewind(fp);
    png_init_io(png_ptr, fp);
    png_read_info(png_ptr, info_ptr);

    xres = png_get_x_pixels_per_meter(png_ptr, info_ptr);
    yres = png_get_y_pixels_per_meter(png_ptr, info_ptr);
    *pxres = (l_int32)((l_float32)xres / 39.37 + 0.5);  /* to ppi */
    *pyres = (l_int32)((l_float32)yres / 39.37 + 0.5);

    png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
    rewind(fp);
    return 0;
}


/*!
 * \brief   isPngInterlaced()
 *
 * \param[in]    filename
 * \param[out]   pinterlaced 1 if interlaced png; 0 otherwise
 * \return  0 if OK, 1 on error
 */
l_ok
isPngInterlaced(const char *filename,
                l_int32    *pinterlaced)
{
l_uint8  buf[32];
FILE    *fp;

    if (!pinterlaced)
        return ERROR_INT("&interlaced not defined", __func__, 1);
    *pinterlaced = 0;
    if (!filename)
        return ERROR_INT("filename not defined", __func__, 1);

    if ((fp = fopenReadStream(filename)) == NULL)
        return ERROR_INT_1("stream not opened", filename, __func__, 1);
    if (fread(buf, 1, 32, fp) != 32) {
        fclose(fp);
        return ERROR_INT_1("data not read", filename, __func__, 1);
    }
    fclose(fp);

    *pinterlaced = (buf[28] == 0) ? 0 : 1;
    return 0;
}


/*
 * \brief   fgetPngColormapInfo()
 *
 * \param[in]    fp     file stream opened for read
 * \param[out]   pcmap  optional; use NULL to skip
 * \param[out]   ptransparency   optional; 1 if colormapped with
 *                      transparency, 0 otherwise; use NULL to skip
 * \return  0 if OK, 1 on error
 *
 *  Notes:
 *      (1) The transparency information in a png is in the tRNA array,
 *          which is separate from the colormap.  If this array exists
 *          and if any element is less than 255, there exists some
 *          transparency.
 *      (2) Side-effect: this rewinds the stream.
 */
l_ok
fgetPngColormapInfo(FILE      *fp,
                    PIXCMAP  **pcmap,
                    l_int32   *ptransparency)
{
l_int32      i, cindex, rval, gval, bval, num_palette, num_trans;
png_byte     bit_depth, color_type;
png_bytep    trans;
png_colorp   palette;
png_structp  png_ptr;
png_infop    info_ptr;

    if (pcmap) *pcmap = NULL;
    if (ptransparency) *ptransparency = 0;
    if (!pcmap && !ptransparency)
        return ERROR_INT("no output defined", __func__, 1);
    if (!fp)
        return ERROR_INT("stream not opened", __func__, 1);

       /* Make the two required structs */
    if ((png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING,
                   (png_voidp)NULL, NULL, NULL)) == NULL)
        return ERROR_INT("png_ptr not made", __func__, 1);
    if ((info_ptr = png_create_info_struct(png_ptr)) == NULL) {
        png_destroy_read_struct(&png_ptr, (png_infopp)NULL, (png_infopp)NULL);
        return ERROR_INT("info_ptr not made", __func__, 1);
    }

        /* Set up png setjmp error handling.
         * Without this, an error calls exit. */
    if (setjmp(png_jmpbuf(png_ptr))) {
        png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
        if (pcmap && *pcmap) pixcmapDestroy(pcmap);
        return ERROR_INT("internal png error", __func__, 1);
    }

        /* Read the metadata and check if there is a colormap */
    rewind(fp);
    png_init_io(png_ptr, fp);
    png_read_info(png_ptr, info_ptr);
    color_type = png_get_color_type(png_ptr, info_ptr);
    if (color_type != PNG_COLOR_TYPE_PALETTE &&
        color_type != PNG_COLOR_MASK_PALETTE) {
        png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
        return 0;
    }

        /* Optionally, read the colormap */
    if (pcmap) {
        bit_depth = png_get_bit_depth(png_ptr, info_ptr);
        png_get_PLTE(png_ptr, info_ptr, &palette, &num_palette);
        *pcmap = pixcmapCreate(bit_depth);  /* spp == 1 */
        for (cindex = 0; cindex < num_palette; cindex++) {
            rval = palette[cindex].red;
            gval = palette[cindex].green;
            bval = palette[cindex].blue;
            pixcmapAddColor(*pcmap, rval, gval, bval);
        }
    }

        /* Optionally, look for transparency.  Note that the colormap
         * has been initialized to fully opaque. */
    if (ptransparency && png_get_valid(png_ptr, info_ptr, PNG_INFO_tRNS)) {
        png_get_tRNS(png_ptr, info_ptr, &trans, &num_trans, NULL);
        if (trans) {
            for (i = 0; i < num_trans; i++) {
                if (trans[i] < 255) {  /* not fully opaque */
                    *ptransparency = 1;
                    if (pcmap) pixcmapSetAlpha(*pcmap, i, trans[i]);
                }
            }
        } else {
            L_ERROR("transparency array not returned\n", __func__);
        }
    }

    png_destroy_read_struct(&png_ptr, &info_ptr, NULL);
    rewind(fp);
    return 0;
}


/*---------------------------------------------------------------------*
 *                      Writing png through stream                     *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixWritePng()
 *
 * \param[in]    filename
 * \param[in]    pix
 * \param[in]    gamma
 * \return  0 if OK; 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Special version for writing png with a specified gamma.
 *          When using pixWrite(), no field is given for gamma.
 * </pre>
 */
l_ok
pixWritePng(const char  *filename,
            PIX         *pix,
            l_float32    gamma)
{
FILE  *fp;

    if (!pix)
        return ERROR_INT("pix not defined", __func__, 1);
    if (!filename)
        return ERROR_INT("filename not defined", __func__, 1);

    if ((fp = fopenWriteStream(filename, "wb+")) == NULL)
        return ERROR_INT_1("stream not opened", filename, __func__, 1);

    if (pixWriteStreamPng(fp, pix, gamma)) {
        fclose(fp);
        return ERROR_INT_1("pix not written to stream", filename, __func__, 1);
    }

    fclose(fp);
    return 0;
}


/*!
 * \brief   pixWriteStreamPng()
 *
 * \param[in]    fp file stream
 * \param[in]    pix
 * \param[in]    gamma use 0.0 if gamma is not defined
 * \return  0 if OK; 1 on error
 *
 * <pre>
 * Notes:
 *      (1) If called from pixWriteStream(), the stream is positioned
 *          at the beginning of the file.
 *      (2) To do sequential writes of png format images to a stream,
 *          use pixWriteStreamPng() directly.
 *      (3) gamma is an optional png chunk.  If no gamma value is to be
 *          placed into the file, use gamma = 0.0.  Otherwise, if
 *          gamma > 0.0, its value is written into the header.
 *      (4) The use of gamma in png is highly problematic.  For an illuminating
 *          discussion, see:  http://hsivonen.iki.fi/png-gamma/
 *      (5) What is the effect/meaning of gamma in the png file?  This
 *          gamma, which we can call the 'source' gamma, is the
 *          inverse of the gamma that was used in enhance.c to brighten
 *          or darken images.  The 'source' gamma is supposed to indicate
 *          the intensity mapping that was done at the time the
 *          image was captured.  Display programs typically apply a
 *          'display' gamma of 2.2 to the output, which is intended
 *          to linearize the intensity based on the response of
 *          thermionic tubes (CRTs).  Flat panel LCDs have typically
 *          been designed to give a similar response as CRTs (call it
 *          "backward compatibility").  The 'display' gamma is
 *          in some sense the inverse of the 'source' gamma.
 *          jpeg encoders attached to scanners and cameras will lighten
 *          the pixels, applying a gamma corresponding to approximately
 *          a square-root relation of output vs input:
 *                output = input^(gamma)
 *          where gamma is often set near 0.4545  (1/gamma is 2.2).
 *          This is stored in the image file.  Then if the display
 *          program reads the gamma, it will apply a display gamma,
 *          typically about 2.2; the product is 1.0, and the
 *          display program produces a linear output.  This works because
 *          the dark colors were appropriately boosted by the scanner,
 *          as described by the 'source' gamma, so they should not
 *          be further boosted by the display program.
 *      (6) As an example, with xv and display, if no gamma is stored,
 *          the program acts as if gamma were 0.4545, multiplies this by 2.2,
 *          and does a linear rendering.  Taking this as a baseline
 *          brightness, if the stored gamma is:
 *              > 0.4545, the image is rendered lighter than baseline
 *              < 0.4545, the image is rendered darker than baseline
 *          In contrast, gqview seems to ignore the gamma chunk in png.
 *      (7) The only valid pixel depths in leptonica are 1, 2, 4, 8, 16
 *          and 32.  However, it is possible, and in some cases desirable,
 *          to write out a png file using an rgb pix that has 24 bpp.
 *          For example, the open source xpdf SplashBitmap class generates
 *          24 bpp rgb images.  Consequently, we enable writing 24 bpp pix
 *          without converting it to 32 bpp first.  Caution: do not call
 *          pixSetPadBits(), because the alignment is wrong and you may
 *          erase part of the last pixel on each line.
 *      (8) If the pix has a colormap, it is written to file.  In most
 *          situations, the alpha component is 255 for each colormap entry,
 *          which is opaque and indicates that it should be ignored.
 *          However, if any alpha component is not 255, it is assumed that
 *          the alpha values are valid, and they are written to the png
 *          file in a tRNS segment.  On readback, the tRNS segment is
 *          identified, and the colormapped image with alpha is converted
 *          to a 4 spp rgba image.
 * </pre>
 */
l_ok
pixWriteStreamPng(FILE      *fp,
                  PIX       *pix,
                  l_float32  gamma)
{
char         commentstring[] = "Comment";
l_int32      i, j, k, wpl, d, spp, compval, valid;
l_int32      cmflag, opaque, max_trans, ncolors;
l_int32     *rmap, *gmap, *bmap, *amap;
l_uint32    *data, *ppixel;
png_byte     bit_depth, color_type;
png_byte     alpha[256];
png_uint_32  w, h;
png_uint_32  xres, yres;
png_bytep   *row_pointers;
png_bytep    rowbuffer;
png_structp  png_ptr;
png_infop    info_ptr;
png_colorp   palette;
PIX         *pix1;
PIXCMAP     *cmap;
char        *text;

    if (!fp)
        return ERROR_INT("stream not open", __func__, 1);
    if (!pix)
        return ERROR_INT("pix not defined", __func__, 1);

    w = pixGetWidth(pix);
    h = pixGetHeight(pix);
    d = pixGetDepth(pix);
    spp = pixGetSpp(pix);

        /* A cmap validity check should prevent low-level colormap errors. */
    if ((cmap = pixGetColormap(pix))) {
        cmflag = 1;
        pixcmapIsValid(cmap, pix, &valid);
        if (!valid)
            return ERROR_INT("colormap is not valid", __func__, 1);
    } else {
        cmflag = 0;
    }

        /* Do not set pad bits for d = 24 ! */
    if (d != 24) pixSetPadBits(pix, 0);

        /* Set the color type and bit depth. */
    if (d == 32 && spp == 4) {
        bit_depth = 8;
        color_type = PNG_COLOR_TYPE_RGBA;   /* 6 */
        cmflag = 0;  /* ignore if it exists */
    } else if (d == 24 || d == 32) {
        bit_depth = 8;
        color_type = PNG_COLOR_TYPE_RGB;   /* 2 */
        cmflag = 0;  /* ignore if it exists */
    } else {
        bit_depth = d;
        color_type = PNG_COLOR_TYPE_GRAY;  /* 0 */
    }
    if (cmflag)
        color_type = PNG_COLOR_TYPE_PALETTE;  /* 3 */

#if  DEBUG_WRITE
    lept_stderr("cmflag = %d, bit_depth = %d, color_type = %d\n",
                cmflag, bit_depth, color_type);
#endif  /* DEBUG_WRITE */

        /* Allocate the 2 png data structures */
    if ((png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING,
                   (png_voidp)NULL, NULL, NULL)) == NULL)
        return ERROR_INT("png_ptr not made", __func__, 1);
    if ((info_ptr = png_create_info_struct(png_ptr)) == NULL) {
        png_destroy_write_struct(&png_ptr, (png_infopp)NULL);
        return ERROR_INT("info_ptr not made", __func__, 1);
    }

        /* Set up png setjmp error handling */
    pix1 = NULL;
    row_pointers = NULL;
    if (setjmp(png_jmpbuf(png_ptr))) {
        png_destroy_write_struct(&png_ptr, &info_ptr);
        LEPT_FREE(row_pointers);
        pixDestroy(&pix1);
        return ERROR_INT("internal png error", __func__, 1);
    }

    png_init_io(png_ptr, fp);

        /* With best zlib compression (9), get between 1 and 10% improvement
         * over default (6), but the compression is 3 to 10 times slower.
         * Use the zlib default (6) as our default compression unless
         * pix->special falls in the range [10 ... 19]; then subtract 10
         * to get the compression value.  */
    compval = Z_DEFAULT_COMPRESSION;
    if (pix->special >= 10 && pix->special < 20)
        compval = pix->special - 10;
    png_set_compression_level(png_ptr, compval);

    png_set_IHDR(png_ptr, info_ptr, w, h, bit_depth, color_type,
                 PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                 PNG_FILTER_TYPE_BASE);

        /* Store resolution in ppm, if known */
    xres = (png_uint_32)(39.37 * (l_float32)pixGetXRes(pix) + 0.5);
    yres = (png_uint_32)(39.37 * (l_float32)pixGetYRes(pix) + 0.5);
    if ((xres == 0) || (yres == 0))
        png_set_pHYs(png_ptr, info_ptr, 0, 0, PNG_RESOLUTION_UNKNOWN);
    else
        png_set_pHYs(png_ptr, info_ptr, xres, yres, PNG_RESOLUTION_METER);

    if (cmflag) {
            /* Make and save the palette */
        ncolors = pixcmapGetCount(cmap);
        palette = (png_colorp)LEPT_CALLOC(ncolors, sizeof(png_color));
        pixcmapToArrays(cmap, &rmap, &gmap, &bmap, &amap);
        for (i = 0; i < ncolors; i++) {
            palette[i].red = (png_byte)rmap[i];
            palette[i].green = (png_byte)gmap[i];
            palette[i].blue = (png_byte)bmap[i];
            alpha[i] = (png_byte)amap[i];
        }
        LEPT_FREE(rmap);
        LEPT_FREE(gmap);
        LEPT_FREE(bmap);
        LEPT_FREE(amap);
        png_set_PLTE(png_ptr, info_ptr, palette, (int)ncolors);
        LEPT_FREE(palette);

            /* Add the tRNS chunk.  If the non-opaque colors are listed
             * first in the colormap, as in the spec, we can use that in
             * the 4th arg of png_set_tRNS.  Otherwise, transparency will
             * be lost for some colors.  To prevent that, see the comments
             * in pixcmapNonOpaqueColorsInfo(). */
        pixcmapIsOpaque(cmap, &opaque);
        if (!opaque) {  /* alpha channel has some transparency; assume valid */
            pixcmapNonOpaqueColorsInfo(cmap, NULL, &max_trans, NULL);
            png_set_tRNS(png_ptr, info_ptr, (png_bytep)alpha,
                         max_trans + 1, NULL);
        }
    }

        /* 0.4545 is treated as the default by some image
         * display programs (not gqview).  A value > 0.4545 will
         * lighten an image as displayed by xv, display, etc. */
    if (gamma > 0.0)
        png_set_gAMA(png_ptr, info_ptr, (l_float64)gamma);

    if ((text = pixGetText(pix))) {
        png_text text_chunk;
        text_chunk.compression = PNG_TEXT_COMPRESSION_NONE;
        text_chunk.key = commentstring;
        text_chunk.text = text;
        text_chunk.text_length = strlen(text);
#ifdef PNG_ITXT_SUPPORTED
        text_chunk.itxt_length = 0;
        text_chunk.lang = NULL;
        text_chunk.lang_key = NULL;
#endif
        png_set_text(png_ptr, info_ptr, &text_chunk, 1);
    }

        /* Write header and palette info */
    png_write_info(png_ptr, info_ptr);

    if ((d != 32) && (d != 24)) {  /* not rgb color */
            /* Generate a temporary pix with bytes swapped.
             * For writing a 1 bpp image as png:
             *    ~ if no colormap, invert the data, because png writes
             *      black as 0
             *    ~ if colormapped, do not invert the data; the two RGBA
             *      colors can have any value.  */
        if (d == 1 && !cmap) {
            pix1 = pixInvert(NULL, pix);
            pixEndianByteSwap(pix1);
        } else {
            pix1 = pixEndianByteSwapNew(pix);
        }
        if (!pix1) {
            png_destroy_write_struct(&png_ptr, &info_ptr);
            return ERROR_INT("pix1 not made", __func__, 1);
        }

            /* Make and assign array of image row pointers */
        row_pointers = (png_bytep *)LEPT_CALLOC(h, sizeof(png_bytep));
        wpl = pixGetWpl(pix1);
        data = pixGetData(pix1);
        for (i = 0; i < h; i++)
            row_pointers[i] = (png_bytep)(data + i * wpl);
        png_set_rows(png_ptr, info_ptr, row_pointers);

            /* Transfer the data */
        png_write_image(png_ptr, row_pointers);
        png_write_end(png_ptr, info_ptr);
        LEPT_FREE(row_pointers);
        pixDestroy(&pix1);
        png_destroy_write_struct(&png_ptr, &info_ptr);
        return 0;
    }

        /* For rgb and rgba, compose and write a row at a time */
    data = pixGetData(pix);
    wpl = pixGetWpl(pix);
    if (d == 24) {  /* See note 7 above */
        for (i = 0; i < h; i++) {
            ppixel = data + i * wpl;
            png_write_rows(png_ptr, (png_bytepp)&ppixel, 1);
        }
    } else {  /* 32 bpp rgb and rgba.  If spp = 4, write the alpha channel */
        rowbuffer = (png_bytep)LEPT_CALLOC(w, 4);
        for (i = 0; i < h; i++) {
            ppixel = data + i * wpl;
            for (j = k = 0; j < w; j++) {
                rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_RED);
                rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_GREEN);
                rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_BLUE);
                if (spp == 4)
                    rowbuffer[k++] = GET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL);
                ppixel++;
            }

            png_write_rows(png_ptr, &rowbuffer, 1);
        }
        LEPT_FREE(rowbuffer);
    }

    png_write_end(png_ptr, info_ptr);
    png_destroy_write_struct(&png_ptr, &info_ptr);
    return 0;
}


/*!
 * \brief   pixSetZlibCompression()
 *
 * \param[in]    pix
 * \param[in]    compval zlib compression value
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Valid zlib compression values are in the interval [0 ... 9],
 *          where, as defined in zlib.h:
 *            0         Z_NO_COMPRESSION
 *            1         Z_BEST_SPEED    (poorest compression)
 *            9         Z_BEST_COMPRESSION
 *          For the default value, use either of these:
 *            6         Z_DEFAULT_COMPRESSION
 *           -1         (resolves to Z_DEFAULT_COMPRESSION)
 *      (2) If you use the defined constants in zlib.h instead of the
 *          compression integers given above, you must include zlib.h.
 * </pre>
 */
l_ok
pixSetZlibCompression(PIX     *pix,
                      l_int32  compval)
{
    if (!pix)
        return ERROR_INT("pix not defined", __func__, 1);
    if (compval < 0 || compval > 9) {
        L_ERROR("Invalid zlib comp val; using default\n", __func__);
        compval = Z_DEFAULT_COMPRESSION;
    }
    pixSetSpecial(pix, 10 + compval);  /* valid range [10 ... 19] */
    return 0;
}


/*---------------------------------------------------------------------*
 *              Set flag for stripping 16 bits on reading              *
 *---------------------------------------------------------------------*/
/*!
 * \brief   l_pngSetReadStrip16To8()
 *
 * \param[in]    flag 1 for stripping 16 bpp to 8 bpp on reading;
 *                    0 for leaving 16 bpp
 * \return  void
 */
void
l_pngSetReadStrip16To8(l_int32  flag)
{
    var_PNG_STRIP_16_TO_8 = flag;
}


/*-------------------------------------------------------------------------*
 *                               Memio utility                             *
 *    libpng read/write callback replacements for performing memory I/O    *
 *                                                                         *
 *    Copyright (C) 2017 Milner Technologies, Inc.  This content is a      *
 *    component of leptonica and is provided under the terms of the        *
 *    Leptonica license.                                                   *
 *-------------------------------------------------------------------------*/

    /*! A node in a linked list of memory buffers that hold I/O content */
struct MemIOData
{
    char*       m_Buffer;  /*!< pointer to this node's I/O content           */
    l_int32     m_Count;   /*!< number of I/O content bytes read or written  */
    l_int32     m_Size;    /*!< allocated size of m_buffer                   */
    struct MemIOData  *m_Next;  /*!< pointer to the next node in the list;   */
                                /*!< zero if this is the last node           */
    struct MemIOData  *m_Last;  /*!< pointer to the last node in the linked  */
                                /*!< list.  The last node is where new       */
                                /*!< content is written.                     */
};
typedef struct MemIOData MEMIODATA;

static void memio_png_write_data(png_structp png_ptr, png_bytep data,
                                 png_size_t length);
static void memio_png_flush(MEMIODATA* pthing);
static void memio_png_read_data(png_structp png_ptr, png_bytep outBytes,
                                png_size_t byteCountToRead);
static void memio_free(MEMIODATA* pthing);

static const l_int32  MEMIO_BUFFER_SIZE = 8192;  /*! buffer alloc size */

/*
 * \brief   memio_png_write_data()
 *
 * \param[in]     png_ptr
 * \param[in]     data
 * \param[in]     len     size of array data in bytes
 *
 * <pre>
 * Notes:
 *      (1) This is a libpng callback for writing an image into a
 *          linked list of memory buffers.
 * </pre>
 */
static void
memio_png_write_data(png_structp  png_ptr,
                     png_bytep    data,
                     png_size_t   len)
{
MEMIODATA  *thing, *last;
l_int32     written = 0;
l_int32     remainingSpace, remainingToWrite;

    thing = (struct MemIOData*)png_get_io_ptr(png_ptr);
    last = (struct MemIOData*)thing->m_Last;
    if (last->m_Buffer == NULL) {
        if (len > MEMIO_BUFFER_SIZE) {
            last->m_Buffer = (char *)LEPT_MALLOC(len);
            memcpy(last->m_Buffer, data, len);
            last->m_Size = last->m_Count = len;
            return;
        }

        last->m_Buffer = (char *)LEPT_MALLOC(MEMIO_BUFFER_SIZE);
        last->m_Size = MEMIO_BUFFER_SIZE;
    }

    while (written < len) {
        if (last->m_Count == last->m_Size) {
            MEMIODATA* next = (MEMIODATA *)LEPT_MALLOC(sizeof(MEMIODATA));
            next->m_Next = NULL;
            next->m_Count = 0;
            next->m_Last = next;

            last->m_Next = next;
            last = thing->m_Last = next;

            last->m_Buffer = (char *)LEPT_MALLOC(MEMIO_BUFFER_SIZE);
            last->m_Size = MEMIO_BUFFER_SIZE;
        }

        remainingSpace = last->m_Size - last->m_Count;
        remainingToWrite = len - written;
        if (remainingSpace < remainingToWrite) {
            memcpy(last->m_Buffer + last->m_Count, data + written,
                   remainingSpace);
            written += remainingSpace;
            last->m_Count += remainingSpace;
        } else {
            memcpy(last->m_Buffer + last->m_Count, data + written,
                   remainingToWrite);
            written += remainingToWrite;
            last->m_Count += remainingToWrite;
        }
    }
}


/*
 * \brief   memio_png_flush()
 *
 * \param[in]     pthing
 *
 * <pre>
 * Notes:
 *      (1) This consolidates write buffers into a single buffer at the
 *          haed of the link list of buffers.
 * </pre>
 */
static void
memio_png_flush(MEMIODATA  *pthing)
{
l_int32     amount = 0;
l_int32     copied = 0;
MEMIODATA  *buffer = 0;
char       *data = 0;

        /* If the data is in one buffer, give the buffer to the user. */
    if (pthing->m_Next == NULL) return;

        /* Consolidate multiple buffers into one new one; add the buffer
         * sizes together. */
    amount = pthing->m_Count;
    buffer = pthing->m_Next;
    while (buffer != NULL) {
        amount += buffer->m_Count;
        buffer = buffer->m_Next;
    }

        /* Copy data to a new buffer. */
    data = (char *)LEPT_MALLOC(amount);
    memcpy(data, pthing->m_Buffer, pthing->m_Count);
    copied = pthing->m_Count;

    LEPT_FREE(pthing->m_Buffer);
    pthing->m_Buffer = NULL;

        /* Don't delete original "thing" because we don't control it. */
    buffer = pthing->m_Next;
    pthing->m_Next = NULL;
    while (buffer != NULL && copied < amount) {
        MEMIODATA* old;
        memcpy(data + copied, buffer->m_Buffer, buffer->m_Count);
        copied += buffer->m_Count;

        old = buffer;
        buffer = buffer->m_Next;

        LEPT_FREE(old->m_Buffer);
        LEPT_FREE(old);
    }

    pthing->m_Buffer = data;
    pthing->m_Count = copied;
    pthing->m_Size = amount;
    return;
}


/*
 * \brief   memio_png_read_data()
 *
 * \param[in]     png_ptr
 * \param[in]     outBytes
 * \param[in]     byteCountToRead
 *
 * <pre>
 * Notes:
 *      (1) This is a libpng callback that reads an image from a single
 *          memory buffer.
 * </pre>
 */
static void
memio_png_read_data(png_structp  png_ptr,
                    png_bytep    outBytes,
                    png_size_t   byteCountToRead)
{
MEMIODATA  *thing;

    thing = (MEMIODATA *)png_get_io_ptr(png_ptr);
    if (byteCountToRead > (thing->m_Size - thing->m_Count)) {
        png_error(png_ptr, "read error in memio_png_read_data");
    }
    memcpy(outBytes, thing->m_Buffer + thing->m_Count, byteCountToRead);
    thing->m_Count += byteCountToRead;
}


/*
 * \brief   memio_free()
 *
 * \param[in]     pthing
 *
 * <pre>
 * Notes:
 *      (1) This frees all the write buffers in the linked list.  It must
 *          be done before exiting the pixWriteMemPng().
 * </pre>
 */
static void
memio_free(MEMIODATA*  pthing)
{
MEMIODATA  *buffer, *old;

    if (pthing->m_Buffer != NULL)
        LEPT_FREE(pthing->m_Buffer);

    pthing->m_Buffer = NULL;
    buffer = pthing->m_Next;
    while (buffer != NULL) {
        old = buffer;
        buffer = buffer->m_Next;

        if (old->m_Buffer != NULL)
            LEPT_FREE(old->m_Buffer);
        LEPT_FREE(old);
    }
}


/*---------------------------------------------------------------------*
 *                       Reading png from memory                       *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixReadMemPng()
 *
 * \param[in]    filedata   png compressed data in memory
 * \param[in]    filesize   number of bytes in data
 * \return  pix, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) See pixReastreamPng().
 * </pre>
 */
PIX *
pixReadMemPng(const l_uint8  *filedata,
              size_t          filesize)
{
l_uint8      byte;
l_int32      i, j, k, index, ncolors, rval, gval, bval, valid;
l_int32      wpl, d, spp, cindex, bitval, bival, quadval, tRNS;
l_uint32     png_transforms;
l_uint32    *data, *line, *ppixel;
int          num_palette, num_text, num_trans;
png_byte     bit_depth, color_type, channels;
png_uint_32  w, h, rowbytes, xres, yres;
png_bytep    rowptr, trans;
png_bytep   *row_pointers;
png_structp  png_ptr;
png_infop    info_ptr, end_info;
png_colorp   palette;
png_textp    text_ptr;  /* ptr to text_chunk */
MEMIODATA    state;
PIX         *pix, *pix1;
PIXCMAP     *cmap;

    if (!filedata)
        return (PIX *)ERROR_PTR("filedata not defined", __func__, NULL);
    if (filesize < 1)
        return (PIX *)ERROR_PTR("invalid filesize", __func__, NULL);

    state.m_Next = 0;
    state.m_Count = 0;
    state.m_Last = &state;
    state.m_Buffer = (char*)filedata;
    state.m_Size = filesize;
    pix = NULL;

        /* Allocate the 3 data structures */
    if ((png_ptr = png_create_read_struct(PNG_LIBPNG_VER_STRING,
                   (png_voidp)NULL, NULL, NULL)) == NULL)
        return (PIX *)ERROR_PTR("png_ptr not made", __func__, NULL);

    if ((info_ptr = png_create_info_struct(png_ptr)) == NULL) {
        png_destroy_read_struct(&png_ptr, (png_infopp)NULL, (png_infopp)NULL);
        return (PIX *)ERROR_PTR("info_ptr not made", __func__, NULL);
    }

    if ((end_info = png_create_info_struct(png_ptr)) == NULL) {
        png_destroy_read_struct(&png_ptr, &info_ptr, (png_infopp)NULL);
        return (PIX *)ERROR_PTR("end_info not made", __func__, NULL);
    }

        /* Set up png setjmp error handling */
    if (setjmp(png_jmpbuf(png_ptr))) {
        png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
        return (PIX *)ERROR_PTR("internal png error", __func__, NULL);
    }

    png_set_read_fn(png_ptr, &state, memio_png_read_data);

        /* ---------------------------------------------------------- *
         *  Set the transforms flags.  Whatever happens here,
         *  NEVER invert 1 bpp using PNG_TRANSFORM_INVERT_MONO.
         *  Also, do not use PNG_TRANSFORM_EXPAND, which would
         *  expand all images with bpp < 8 to 8 bpp.
         * ---------------------------------------------------------- */
        /* To strip 16 --> 8 bit depth, use PNG_TRANSFORM_STRIP_16 */
    if (var_PNG_STRIP_16_TO_8 == 1) {  /* our default */
        png_transforms = PNG_TRANSFORM_STRIP_16;
    } else {
        png_transforms = PNG_TRANSFORM_IDENTITY;
        L_INFO("not stripping 16 --> 8 in png reading\n", __func__);
    }

        /* Read it */
    png_read_png(png_ptr, info_ptr, png_transforms, NULL);

    row_pointers = png_get_rows(png_ptr, info_ptr);
    w = png_get_image_width(png_ptr, info_ptr);
    h = png_get_image_height(png_ptr, info_ptr);
    bit_depth = png_get_bit_depth(png_ptr, info_ptr);
    rowbytes = png_get_rowbytes(png_ptr, info_ptr);
    color_type = png_get_color_type(png_ptr, info_ptr);
    channels = png_get_channels(png_ptr, info_ptr);
    spp = channels;
    tRNS = png_get_valid(png_ptr, info_ptr, PNG_INFO_tRNS) ? 1 : 0;

    if (spp == 1) {
        d = bit_depth;
    } else {  /* spp == 2 (gray + alpha), spp == 3 (rgb), spp == 4 (rgba) */
        d = 4 * bit_depth;
    }

        /* Remove if/when this is implemented for all bit_depths */
    if (spp == 3 && bit_depth != 8) {
        lept_stderr("Help: spp = 3 and depth = %d != 8\n!!", bit_depth);
        png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
        return (PIX *)ERROR_PTR("not implemented for this depth",
            __func__, NULL);
    }

    cmap = NULL;
    if (color_type == PNG_COLOR_TYPE_PALETTE ||
        color_type == PNG_COLOR_MASK_PALETTE) {   /* generate a colormap */
        png_get_PLTE(png_ptr, info_ptr, &palette, &num_palette);
        cmap = pixcmapCreate(d);  /* spp == 1 */
        for (cindex = 0; cindex < num_palette; cindex++) {
            rval = palette[cindex].red;
            gval = palette[cindex].green;
            bval = palette[cindex].blue;
            pixcmapAddColor(cmap, rval, gval, bval);
        }
    }

    if ((pix = pixCreate(w, h, d)) == NULL) {
        pixcmapDestroy(&cmap);
        png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
        pixcmapDestroy(&cmap);
        return (PIX *)ERROR_PTR("pix not made", __func__, NULL);
    }
    pixSetInputFormat(pix, IFF_PNG);
    wpl = pixGetWpl(pix);
    data = pixGetData(pix);
    pixSetSpp(pix, spp);
    if (pixSetColormap(pix, cmap)) {
        pixDestroy(&pix);
        return (PIX *)ERROR_PTR("invalid colormap", __func__, NULL);
    }

    if (spp == 1 && !tRNS) {  /* copy straight from buffer to pix */
        for (i = 0; i < h; i++) {
            line = data + i * wpl;
            rowptr = row_pointers[i];
            for (j = 0; j < rowbytes; j++) {
                    SET_DATA_BYTE(line, j, rowptr[j]);
            }
        }
    } else if (spp == 2) {  /* grayscale + alpha; convert to RGBA */
        L_INFO("converting (gray + alpha) ==> RGBA\n", __func__);
        for (i = 0; i < h; i++) {
            ppixel = data + i * wpl;
            rowptr = row_pointers[i];
            for (j = k = 0; j < w; j++) {
                    /* Copy gray value into r, g and b */
                SET_DATA_BYTE(ppixel, COLOR_RED, rowptr[k]);
                SET_DATA_BYTE(ppixel, COLOR_GREEN, rowptr[k]);
                SET_DATA_BYTE(ppixel, COLOR_BLUE, rowptr[k++]);
                SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL, rowptr[k++]);
                ppixel++;
            }
        }
        pixSetSpp(pix, 4);  /* we do not support 2 spp pix */
    } else if (spp == 3 || spp == 4) {
        for (i = 0; i < h; i++) {
            ppixel = data + i * wpl;
            rowptr = row_pointers[i];
            for (j = k = 0; j < w; j++) {
                SET_DATA_BYTE(ppixel, COLOR_RED, rowptr[k++]);
                SET_DATA_BYTE(ppixel, COLOR_GREEN, rowptr[k++]);
                SET_DATA_BYTE(ppixel, COLOR_BLUE, rowptr[k++]);
                if (spp == 4)
                    SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL, rowptr[k++]);
                ppixel++;
            }
        }
    }

        /* Special spp == 1 cases with transparency:
         *    (1) 8 bpp without colormap; assume full transparency
         *    (2) 1 bpp with colormap + trans array (for alpha)
         *    (3) 2 bpp with colormap + trans array (for alpha)
         *    (4) 4 bpp with colormap + trans array (for alpha)
         *    (5) 8 bpp with colormap + trans array (for alpha)
         * These all require converting to RGBA */
    if (spp == 1 && tRNS) {
        if (!cmap) {
                /* Case 1: make fully transparent RGBA image */
            L_INFO("transparency, 1 spp, no colormap, no transparency array: "
                   "convention is fully transparent image\n", __func__);
            L_INFO("converting (fully transparent 1 spp) ==> RGBA\n", __func__);
            pixDestroy(&pix);
            pix = pixCreate(w, h, 32);  /* init to alpha = 0 (transparent) */
            pixSetSpp(pix, 4);
        } else {
            L_INFO("converting (cmap + alpha) ==> RGBA\n", __func__);

                /* Grab the transparency array */
            png_get_tRNS(png_ptr, info_ptr, &trans, &num_trans, NULL);
            if (!trans) {  /* invalid png file */
                pixDestroy(&pix);
                png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);
                return (PIX *)ERROR_PTR("cmap, tRNS, but no transparency array",
                                        __func__, NULL);
            }

                /* Save the cmap and destroy the pix */
            cmap = pixcmapCopy(pixGetColormap(pix));
            ncolors = pixcmapGetCount(cmap);
            pixDestroy(&pix);

                /* Start over with 32 bit RGBA */
            pix = pixCreate(w, h, 32);
            wpl = pixGetWpl(pix);
            data = pixGetData(pix);
            pixSetSpp(pix, 4);

#if DEBUG_READ
            lept_stderr("ncolors = %d, num_trans = %d\n",
                        ncolors, num_trans);
            for (i = 0; i < ncolors; i++) {
                pixcmapGetColor(cmap, i, &rval, &gval, &bval);
                if (i < num_trans) {
                    lept_stderr("(r,g,b,a) = (%d,%d,%d,%d)\n",
                                rval, gval, bval, trans[i]);
                } else {
                    lept_stderr("(r,g,b,a) = (%d,%d,%d,<<255>>)\n",
                                rval, gval, bval);
                }
            }
#endif  /* DEBUG_READ */

                /* Extract the data and convert to RGBA */
            if (d == 1) {
                    /* Case 2: 1 bpp with transparency (usually) behind white */
                L_INFO("converting 1 bpp cmap with alpha ==> RGBA\n", __func__);
                if (num_trans == 1)
                    L_INFO("num_trans = 1; second color opaque by default\n",
                           __func__);
                for (i = 0; i < h; i++) {
                    ppixel = data + i * wpl;
                    rowptr = row_pointers[i];
                    for (j = 0, index = 0; j < rowbytes; j++) {
                        byte = rowptr[j];
                        for (k = 0; k < 8 && index < w; k++, index++) {
                            bitval = (byte >> (7 - k)) & 1;
                            pixcmapGetColor(cmap, bitval, &rval, &gval, &bval);
                            composeRGBPixel(rval, gval, bval, ppixel);
                            SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL,
                                      bitval < num_trans ? trans[bitval] : 255);
                            ppixel++;
                        }
                    }
                }
            } else if (d == 2) {
                    /* Case 3: 2 bpp with cmap and associated transparency */
                L_INFO("converting 2 bpp cmap with alpha ==> RGBA\n", __func__);
                for (i = 0; i < h; i++) {
                    ppixel = data + i * wpl;
                    rowptr = row_pointers[i];
                    for (j = 0, index = 0; j < rowbytes; j++) {
                        byte = rowptr[j];
                        for (k = 0; k < 4 && index < w; k++, index++) {
                            bival = (byte >> 2 * (3 - k)) & 3;
                            pixcmapGetColor(cmap, bival, &rval, &gval, &bval);
                            composeRGBPixel(rval, gval, bval, ppixel);
                                /* Assume missing entries to be 255 (opaque)
                                 * according to the spec:
                                 * http://www.w3.org/TR/PNG/#11tRNS */
                            SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL,
                                bival < num_trans ? trans[bival] : 255);
                            ppixel++;
                        }
                    }
                }
            } else if (d == 4) {
                    /* Case 4: 4 bpp with cmap and associated transparency */
                L_INFO("converting 4 bpp cmap with alpha ==> RGBA\n", __func__);
                for (i = 0; i < h; i++) {
                    ppixel = data + i * wpl;
                    rowptr = row_pointers[i];
                    for (j = 0, index = 0; j < rowbytes; j++) {
                        byte = rowptr[j];
                        for (k = 0; k < 2 && index < w; k++, index++) {
                            quadval = (byte >> 4 * (1 - k)) & 0xf;
                            pixcmapGetColor(cmap, quadval, &rval, &gval, &bval);
                            composeRGBPixel(rval, gval, bval, ppixel);
                                /* Assume missing entries to be 255 (opaque) */
                            SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL,
                                quadval < num_trans ? trans[quadval] : 255);
                            ppixel++;
                        }
                    }
                }
            } else if (d == 8) {
                    /* Case 5: 8 bpp with cmap and associated transparency */
                L_INFO("converting 8 bpp cmap with alpha ==> RGBA\n", __func__);
                for (i = 0; i < h; i++) {
                    ppixel = data + i * wpl;
                    rowptr = row_pointers[i];
                    for (j = 0; j < w; j++) {
                        index = rowptr[j];
                        pixcmapGetColor(cmap, index, &rval, &gval, &bval);
                        composeRGBPixel(rval, gval, bval, ppixel);
                            /* Assume missing entries to be 255 (opaque)
                             * according to the spec:
                             * http://www.w3.org/TR/PNG/#11tRNS */
                        SET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL,
                                      index < num_trans ? trans[index] : 255);
                        ppixel++;
                    }
                }
            } else {
                L_ERROR("spp == 1, cmap, trans array, invalid depth: %d\n",
                        __func__, d);
            }
            pixcmapDestroy(&cmap);
        }
    }

#if  DEBUG_READ
    if (cmap) {
        for (i = 0; i < 16; i++) {
            lept_stderr("[%d] = %d\n", i, ((l_uint8 *)(cmap->array))[i]);
        }
    }
#endif  /* DEBUG_READ */

        /* Final adjustments for bpp = 1.
         *   + If there is no colormap, the image must be inverted because
         *     png stores black pixels as 0.
         *   + We have already handled the case of cmapped, 1 bpp pix
         *     with transparency, where the output pix is 32 bpp RGBA.
         *     If there is no transparency but the pix has a colormap,
         *     we remove the colormap, because functions operating on
         *     1 bpp images in leptonica assume no colormap.
         *   + The colormap must be removed in such a way that the pixel
         *     values are not changed.  If the values are only black and
         *     white, we return a 1 bpp image; if gray, return an 8 bpp pix;
         *     otherwise, return a 32 bpp rgb pix.
         *
         * Note that we cannot use the PNG_TRANSFORM_INVERT_MONO flag
         * to do the inversion, because that flag (since version 1.0.9)
         * inverts 8 bpp grayscale as well, which we don't want to do.
         * (It also doesn't work if there is a colormap.)
         *
         * Note that if the input png is a 1-bit with colormap and
         * transparency, it has already been rendered as a 32 bpp,
         * spp = 4 rgba pix.
         */
    if (pixGetDepth(pix) == 1) {
        if (!cmap) {
            pixInvert(pix, pix);
        } else {
            pix1 = pixRemoveColormap(pix, REMOVE_CMAP_BASED_ON_SRC);
            pixDestroy(&pix);
            pix = pix1;
        }
    }

    xres = png_get_x_pixels_per_meter(png_ptr, info_ptr);
    yres = png_get_y_pixels_per_meter(png_ptr, info_ptr);
    pixSetXRes(pix, (l_int32)((l_float32)xres / 39.37 + 0.5));  /* to ppi */
    pixSetYRes(pix, (l_int32)((l_float32)yres / 39.37 + 0.5));  /* to ppi */

        /* Get the text if there is any */
    png_get_text(png_ptr, info_ptr, &text_ptr, &num_text);
    if (num_text && text_ptr)
        pixSetText(pix, text_ptr->text);

    png_destroy_read_struct(&png_ptr, &info_ptr, &end_info);

       /* Final validity check on the colormap */
    if ((cmap = pixGetColormap(pix)) != NULL) {
        pixcmapIsValid(cmap, pix, &valid);
        if (!valid) {
            pixDestroy(&pix);
            return (PIX *)ERROR_PTR("colormap is not valid", __func__, NULL);
        }
    }

    pixSetPadBits(pix, 0);
    return pix;
}


/*---------------------------------------------------------------------*
 *                        Writing png to memory                        *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixWriteMemPng()
 *
 * \param[out]   pfiledata     png encoded data of pix
 * \param[out]   pfilesize     size of png encoded data
 * \param[in]    pix
 * \param[in]    gamma         use 0.0 if gamma is not defined
 * \return  0 if OK; 1 on error
 *
 * <pre>
 * Notes:
 *      (1) See pixWriteStreamPng()
 * </pre>
 */
l_ok
pixWriteMemPng(l_uint8  **pfiledata,
               size_t    *pfilesize,
               PIX       *pix,
               l_float32  gamma)
{
char         commentstring[] = "Comment";
l_int32      i, j, k, wpl, d, spp, cmflag, opaque, ncolors, compval, valid;
l_int32     *rmap, *gmap, *bmap, *amap;
l_uint32    *data, *ppixel;
png_byte     bit_depth, color_type;
png_byte     alpha[256];
png_uint_32  w, h, xres, yres;
png_bytep    rowbuffer;
png_structp  png_ptr;
png_infop    info_ptr;
png_colorp   palette;
PIX         *pix1;
PIXCMAP     *cmap;
char        *text;
MEMIODATA    state;

    if (pfiledata) *pfiledata = NULL;
    if (pfilesize) *pfilesize = 0;
    if (!pfiledata)
        return ERROR_INT("&filedata not defined", __func__, 1);
    if (!pfilesize)
        return ERROR_INT("&filesize not defined", __func__, 1);
    if (!pix)
        return ERROR_INT("pix not defined", __func__, 1);

    state.m_Buffer = 0;
    state.m_Size = 0;
    state.m_Next = 0;
    state.m_Count = 0;
    state.m_Last = &state;

    w = pixGetWidth(pix);
    h = pixGetHeight(pix);
    d = pixGetDepth(pix);
    spp = pixGetSpp(pix);

        /* A cmap validity check should prevent low-level colormap errors. */
    if ((cmap = pixGetColormap(pix))) {
        cmflag = 1;
        pixcmapIsValid(cmap, pix, &valid);
        if (!valid)
            return ERROR_INT("colormap is not valid", __func__, 1);
    } else {
        cmflag = 0;
    }

        /* Do not set pad bits for d = 24 ! */
    if (d != 24) pixSetPadBits(pix, 0);

        /* Set the color type and bit depth. */
    if (d == 32 && spp == 4) {
        bit_depth = 8;
        color_type = PNG_COLOR_TYPE_RGBA;   /* 6 */
        cmflag = 0;  /* ignore if it exists */
    } else if (d == 24 || d == 32) {
        bit_depth = 8;
        color_type = PNG_COLOR_TYPE_RGB;   /* 2 */
        cmflag = 0;  /* ignore if it exists */
    } else {
        bit_depth = d;
        color_type = PNG_COLOR_TYPE_GRAY;  /* 0 */
    }
    if (cmflag)
        color_type = PNG_COLOR_TYPE_PALETTE;  /* 3 */

#if  DEBUG_WRITE
    lept_stderr("cmflag = %d, bit_depth = %d, color_type = %d\n",
                cmflag, bit_depth, color_type);
#endif  /* DEBUG_WRITE */

        /* Allocate the 2 data structures */
    if ((png_ptr = png_create_write_struct(PNG_LIBPNG_VER_STRING,
                   (png_voidp)NULL, NULL, NULL)) == NULL)
        return ERROR_INT("png_ptr not made", __func__, 1);

    if ((info_ptr = png_create_info_struct(png_ptr)) == NULL) {
        png_destroy_write_struct(&png_ptr, (png_infopp)NULL);
        return ERROR_INT("info_ptr not made", __func__, 1);
    }

        /* Set up png setjmp error handling */
    pix1 = NULL;
    if (setjmp(png_jmpbuf(png_ptr))) {
        png_destroy_write_struct(&png_ptr, &info_ptr);
        pixDestroy(&pix1);
        return ERROR_INT("internal png error", __func__, 1);
    }

    png_set_write_fn(png_ptr, &state, memio_png_write_data,
                     (png_flush_ptr)NULL);

        /* With best zlib compression (9), get between 1 and 10% improvement
         * over default (6), but the compression is 3 to 10 times slower.
         * Use the zlib default (6) as our default compression unless
         * pix->special falls in the range [10 ... 19]; then subtract 10
         * to get the compression value.  */
    compval = Z_DEFAULT_COMPRESSION;
    if (pix->special >= 10 && pix->special < 20)
        compval = pix->special - 10;
    png_set_compression_level(png_ptr, compval);

    png_set_IHDR(png_ptr, info_ptr, w, h, bit_depth, color_type,
                 PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE,
                 PNG_FILTER_TYPE_BASE);

        /* Store resolution in ppm, if known */
    xres = (png_uint_32)(39.37 * (l_float32)pixGetXRes(pix) + 0.5);
    yres = (png_uint_32)(39.37 * (l_float32)pixGetYRes(pix) + 0.5);
    if ((xres == 0) || (yres == 0))
        png_set_pHYs(png_ptr, info_ptr, 0, 0, PNG_RESOLUTION_UNKNOWN);
    else
        png_set_pHYs(png_ptr, info_ptr, xres, yres, PNG_RESOLUTION_METER);

    if (cmflag) {
            /* Make and save the palette */
        ncolors = pixcmapGetCount(cmap);
        palette = (png_colorp)LEPT_CALLOC(ncolors, sizeof(png_color));
        pixcmapToArrays(cmap, &rmap, &gmap, &bmap, &amap);
        for (i = 0; i < ncolors; i++) {
            palette[i].red = (png_byte)rmap[i];
            palette[i].green = (png_byte)gmap[i];
            palette[i].blue = (png_byte)bmap[i];
            alpha[i] = (png_byte)amap[i];
        }
        LEPT_FREE(rmap);
        LEPT_FREE(gmap);
        LEPT_FREE(bmap);
        LEPT_FREE(amap);
        png_set_PLTE(png_ptr, info_ptr, palette, (int)ncolors);
        LEPT_FREE(palette);

        pixcmapIsOpaque(cmap, &opaque);
        if (!opaque)  /* alpha channel has some transparency; assume valid */
            png_set_tRNS(png_ptr, info_ptr, (png_bytep)alpha,
                         (int)ncolors, NULL);
    }

        /* 0.4545 is treated as the default by some image
         * display programs (not gqview).  A value > 0.4545 will
         * lighten an image as displayed by xv, display, etc. */
    if (gamma > 0.0)
        png_set_gAMA(png_ptr, info_ptr, (l_float64)gamma);

    if ((text = pixGetText(pix))) {
        png_text text_chunk;
        text_chunk.compression = PNG_TEXT_COMPRESSION_NONE;
        text_chunk.key = commentstring;
        text_chunk.text = text;
        text_chunk.text_length = strlen(text);
#ifdef PNG_ITXT_SUPPORTED
        text_chunk.itxt_length = 0;
        text_chunk.lang = NULL;
        text_chunk.lang_key = NULL;
#endif
        png_set_text(png_ptr, info_ptr, &text_chunk, 1);
    }

        /* Write header and palette info */
    png_write_info(png_ptr, info_ptr);

    if ((d != 32) && (d != 24)) {  /* not rgb color */
            /* Generate a temporary pix with bytes swapped.
             * For writing a 1 bpp image as png:
             *    ~ if no colormap, invert the data, because png writes
             *      black as 0
             *    ~ if colormapped, do not invert the data; the two RGBA
             *      colors can have any value.  */
        if (d == 1 && !cmap) {
            pix1 = pixInvert(NULL, pix);
            pixEndianByteSwap(pix1);
        } else {
            pix1 = pixEndianByteSwapNew(pix);
        }
        if (!pix1) {
            png_destroy_write_struct(&png_ptr, &info_ptr);
            memio_free(&state);
            return ERROR_INT("pix1 not made", __func__, 1);
        }

            /* Transfer the data */
        wpl = pixGetWpl(pix1);
        data = pixGetData(pix1);
        for (i = 0; i < h; i++)
            png_write_row(png_ptr, (png_bytep)(data + i * wpl));
        png_write_end(png_ptr, info_ptr);

        pixDestroy(&pix1);
        png_destroy_write_struct(&png_ptr, &info_ptr);
        memio_png_flush(&state);
        *pfiledata = (l_uint8 *)state.m_Buffer;
        state.m_Buffer = 0;
        *pfilesize = state.m_Count;
        memio_free(&state);
        return 0;
    }

        /* For rgb and rgba, compose and write a row at a time */
    data = pixGetData(pix);
    wpl = pixGetWpl(pix);
    if (d == 24) {  /* See note 7 in pixWriteStreamPng() */
        for (i = 0; i < h; i++) {
            ppixel = data + i * wpl;
            png_write_rows(png_ptr, (png_bytepp)&ppixel, 1);
        }
    } else {  /* 32 bpp rgb and rgba.  If spp = 4, write the alpha channel */
        rowbuffer = (png_bytep)LEPT_CALLOC(w, 4);
        for (i = 0; i < h; i++) {
            ppixel = data + i * wpl;
            for (j = k = 0; j < w; j++) {
                rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_RED);
                rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_GREEN);
                rowbuffer[k++] = GET_DATA_BYTE(ppixel, COLOR_BLUE);
                if (spp == 4)
                    rowbuffer[k++] = GET_DATA_BYTE(ppixel, L_ALPHA_CHANNEL);
                ppixel++;
            }

            png_write_rows(png_ptr, &rowbuffer, 1);
        }
        LEPT_FREE(rowbuffer);
    }
    png_write_end(png_ptr, info_ptr);

    png_destroy_write_struct(&png_ptr, &info_ptr);
    memio_png_flush(&state);
    *pfiledata = (l_uint8 *)state.m_Buffer;
    state.m_Buffer = 0;
    *pfilesize = state.m_Count;
    memio_free(&state);
    return 0;
}

/* --------------------------------------------*/
#endif  /* HAVE_LIBPNG */
/* --------------------------------------------*/
