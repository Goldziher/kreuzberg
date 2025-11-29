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
 * \file gifio.c
 * <pre>
 *
 *    Reading gif
 *          PIX            *pixReadStreamGif()
 *          PIX            *pixReadMemGif()
 *          static l_int32  gifReadFunc()
 *          static PIX     *gifToPix()
 *
 *    Writing gif
 *          l_int32         pixWriteStreamGif()
 *          l_int32         pixWriteMemGif()
 *          static l_int32  gifWriteFunc()
 *          static l_int32  pixToGif()
 *
 *    The initial version of this module was generously contribued by
 *    Antony Dovgal.
 *
 *    The functions that read and write from pix to gif-compressed memory,
 *    using gif internal functions DGifOpen() and EGifOpen() that are
 *    available in 5.1 and later, were contributed by Tobias Peirick.
 *
 *    Version information:
 *
 *    (1) This supports the gif library, version 5.1 or later, for which
 *        gif read-from-mem and write-to-mem allow these operations
 *        without writing temporary files.
 *    (2) There has never been a gif stream interface.  For versions
 *        before 5.1, it was necessary to use a file descriptor, and to
 *        generate a file stream from the low-level descriptor.  With the
 *        memory interface in 5.1 that can be used on all platforms, it
 *        is no longer necessary to use any API code with file descriptors.
 *    (3) The public interface changed with 5.0 and with 5.1, and we
 *        no longer support 4.6.1 and 5.0.
 *    (4) Version 5.1.2 came out on Jan 7, 2016.  Leptonica cannot
 *        successfully read gif files that it writes with this version;
 *        DGifSlurp() gets an internal error from an uninitialized array
 *        and returns failure.  The problem was fixed in 5.1.3.
 *
 *    Limitations:
 *
 *    (1) We do not support animated gif.  If the gif has more than one image,
 *        an error message is returned.
 *
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

/* --------------------------------------------------------------------*/
#if  HAVE_LIBGIF  || HAVE_LIBUNGIF             /* defined in environ.h */
/* --------------------------------------------------------------------*/

#include "gif_lib.h"

    /* Interface that enables low-level GIF support for reading from memory */
static PIX * gifToPix(GifFileType *gif);
    /* Interface that enables low-level GIF support for writing to memory */
static l_int32 pixToGif(PIX *pix, GifFileType *gif);

    /*! For in-memory decoding of GIF; 5.1+ */
typedef struct GifReadBuffer
{
    size_t            size;    /*!< size of buffer                           */
    size_t            pos;     /*!< position relative to beginning of buffer */
    const l_uint8    *cdata;   /*!< data in the buffer                       */
} GifReadBuffer;

    /*! Low-level callback for in-memory decoding */
static l_int32  gifReadFunc(GifFileType *gif, GifByteType *dest,
                            l_int32 bytesToRead);
    /*! Low-level callback for in-memory encoding */
static l_int32  gifWriteFunc(GifFileType *gif, const GifByteType *src,
                             l_int32 bytesToWrite);


/*---------------------------------------------------------------------*
 *                            Reading gif                              *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixReadStreamGif()
 *
 * \param[in]  fp   file stream opened for reading
 * \return  pix, or NULL on error
 */
PIX *
pixReadStreamGif(FILE  *fp)
{
l_uint8  *filedata;
size_t    filesize;
PIX      *pix;

    if (!fp)
        return (PIX *)ERROR_PTR("fp not defined", __func__, NULL);

        /* Read data into memory from file */
    rewind(fp);
    if ((filedata = l_binaryReadStream(fp, &filesize)) == NULL)
        return (PIX *)ERROR_PTR("filedata not read", __func__, NULL);

        /* Uncompress from memory */
    pix = pixReadMemGif(filedata, filesize);
    LEPT_FREE(filedata);
    if (!pix)
        L_ERROR("failed to read gif from file data\n", __func__);
    return pix;
}


/*!
 * \brief   pixReadMemGif()
 *
 * \param[in]  cdata    const; gif-encoded
 * \param[in]  size     bytes data
 * \return  pix, or NULL on error
 *
 * <pre>
 * Notes:
 *     (1) For libgif version >= 5.1, this uses the DGifOpen() buffer
 *         interface.  No temp files are required.
 *     (2) For libgif version < 5.1, it was necessary to write the compressed
 *         data to file and read it back, and we couldn't use the GNU
 *         runtime extension fmemopen() because libgif doesn't have a file
 *         stream interface.
 * </pre>
 */
PIX *
pixReadMemGif(const l_uint8  *cdata,
              size_t          size)
{
GifFileType   *gif;
GifReadBuffer  buffer;

        /* 5.1+ and not 5.1.2 */
#if (GIFLIB_MAJOR < 5 || (GIFLIB_MAJOR == 5 && GIFLIB_MINOR == 0))
    L_ERROR("Require giflib-5.1 or later\n", __func__);
    return NULL;
#endif  /* < 5.1 */
#if GIFLIB_MAJOR == 5 && GIFLIB_MINOR == 1 && GIFLIB_RELEASE == 2  /* 5.1.2 */
    L_ERROR("Can't use giflib-5.1.2; suggest 5.1.3 or later\n", __func__);
    return NULL;
#endif  /* 5.1.2 */

    if (!cdata)
        return (PIX *)ERROR_PTR("cdata not defined", __func__, NULL);

    buffer.cdata = cdata;
    buffer.size = size;
    buffer.pos = 0;
    if ((gif = DGifOpen((void*)&buffer, gifReadFunc, NULL)) == NULL)
        return (PIX *)ERROR_PTR("could not open gif stream from memory",
                                __func__, NULL);

    return gifToPix(gif);
}


static l_int32
gifReadFunc(GifFileType  *gif,
            GifByteType  *dest,
            l_int32       bytesToRead)
{
GifReadBuffer  *buffer;
l_int32         bytesRead;

    if ((buffer = (GifReadBuffer*)gif->UserData) == NULL)
        return ERROR_INT("UserData not set", __func__, -1);

    if(buffer->pos >= buffer->size || bytesToRead > buffer->size)
        return -1;

    bytesRead = (buffer->pos < buffer->size - bytesToRead)
              ? bytesToRead : buffer->size - buffer->pos;
    memcpy(dest, buffer->cdata + buffer->pos, bytesRead);
    buffer->pos += bytesRead;
    return bytesRead;
}


/*!
 * \brief   gifToPix()
 *
 * \param[in]  gif   opened gif stream
 * \return  pix, or NULL on error
 *
 * <pre>
 * Notes:
 *      (1) This decodes the pix from the compressed gif stream and
 *          closes the stream.
 *      (2) It is static so that the stream is not exposed to clients.
 *      (3) Leptonica does not support gifanim (more than 1 image in the file).
 * </pre>
 */
static PIX *
gifToPix(GifFileType  *gif)
{
l_int32          wpl, i, j, w, h, d, cindex, ncolors, valid, nimages;
l_int32          rval, gval, bval;
l_uint32        *data, *line;
PIX             *pixd;
PIXCMAP         *cmap;
ColorMapObject  *gif_cmap;
SavedImage       si;
int              giferr;

        /* Read all the data, but use only the first image found */
    if (DGifSlurp(gif) != GIF_OK) {
        DGifCloseFile(gif, &giferr);
        return (PIX *)ERROR_PTR("failed to read GIF data", __func__, NULL);
    }

    if (gif->SavedImages == NULL) {
        DGifCloseFile(gif, &giferr);
        return (PIX *)ERROR_PTR("no images found in GIF", __func__, NULL);
    }

    nimages = gif->ImageCount;
    if (nimages > 1) {
        DGifCloseFile(gif, &giferr);
        L_ERROR("There are %d images in the file; gifanim is not supported\n",
                __func__, nimages);
        return NULL;
    }

    si = gif->SavedImages[0];
    w = si.ImageDesc.Width;
    h = si.ImageDesc.Height;
    if (w <= 0 || h <= 0) {
        DGifCloseFile(gif, &giferr);
        return (PIX *)ERROR_PTR("invalid image dimensions", __func__, NULL);
    }

    if (si.RasterBits == NULL) {
        DGifCloseFile(gif, &giferr);
        return (PIX *)ERROR_PTR("no raster data in GIF", __func__, NULL);
    }

    if (si.ImageDesc.ColorMap) {
            /* private cmap for this image */
        gif_cmap = si.ImageDesc.ColorMap;
    } else if (gif->SColorMap) {
            /* global cmap for whole picture */
        gif_cmap = gif->SColorMap;
    } else {
            /* don't know where to take cmap from */
        DGifCloseFile(gif, &giferr);
        return (PIX *)ERROR_PTR("color map is missing", __func__, NULL);
    }

    ncolors = gif_cmap->ColorCount;
    if (ncolors <= 0 || ncolors > 256) {
        DGifCloseFile(gif, &giferr);
        return (PIX *)ERROR_PTR("ncolors is invalid", __func__, NULL);
    }
    if (ncolors <= 2)
        d = 1;
    else if (ncolors <= 4)
        d = 2;
    else if (ncolors <= 16)
        d = 4;
    else  /* [17 ... 256] */
        d = 8;
    cmap = pixcmapCreate(d);
    for (cindex = 0; cindex < ncolors; cindex++) {
        rval = gif_cmap->Colors[cindex].Red;
        gval = gif_cmap->Colors[cindex].Green;
        bval = gif_cmap->Colors[cindex].Blue;
        pixcmapAddColor(cmap, rval, gval, bval);
    }

    if ((pixd = pixCreate(w, h, d)) == NULL) {
        DGifCloseFile(gif, &giferr);
        pixcmapDestroy(&cmap);
        return (PIX *)ERROR_PTR("failed to allocate pixd", __func__, NULL);
    }
    pixSetInputFormat(pixd, IFF_GIF);
    pixSetColormap(pixd, cmap);
    pixcmapIsValid(cmap, pixd, &valid);
    if (!valid) {
        DGifCloseFile(gif, &giferr);
        pixDestroy(&pixd);
        pixcmapDestroy(&cmap);
        return (PIX *)ERROR_PTR("colormap is invalid", __func__, NULL);
    }

    wpl = pixGetWpl(pixd);
    data = pixGetData(pixd);
    for (i = 0; i < h; i++) {
        line = data + i * wpl;
        if (d == 1) {
            for (j = 0; j < w; j++) {
                if (si.RasterBits[i * w + j])
                    SET_DATA_BIT(line, j);
            }
        } else if (d == 2) {
            for (j = 0; j < w; j++)
                SET_DATA_DIBIT(line, j, si.RasterBits[i * w + j]);
        } else if (d == 4) {
            for (j = 0; j < w; j++)
                SET_DATA_QBIT(line, j, si.RasterBits[i * w + j]);
        } else {  /* d == 8 */
            for (j = 0; j < w; j++)
                SET_DATA_BYTE(line, j, si.RasterBits[i * w + j]);
        }
    }

    /* Versions before 5.0 required un-interlacing to restore
     * the raster lines to normal order if the image
     * had been interlaced (for viewing in a browser):
         if (gif->Image.Interlace) {
             PIX *pixdi = pixUninterlaceGIF(pixd);
             pixTransferAllData(pixd, &pixdi, 0, 0);
         }
     * This is no longer required. */

    DGifCloseFile(gif, &giferr);
    return pixd;
}


/*---------------------------------------------------------------------*
 *                            Writing gif                              *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixWriteStreamGif()
 *
 * \param[in]  fp    file stream opened for writing
 * \param[in]  pix   1, 2, 4, 8, 16 or 32 bpp
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) All output gif have colormaps.  If the pix is 32 bpp rgb,
 *          this quantizes the colors and writes out 8 bpp.
 *          If the pix is 16 bpp grayscale, it converts to 8 bpp first.
 * </pre>
 */
l_ok
pixWriteStreamGif(FILE  *fp,
                  PIX   *pix)
{
l_uint8  *filedata;
size_t    filebytes, nbytes;

    if (!fp)
        return ERROR_INT("stream not open", __func__, 1);
    if (!pix)
        return ERROR_INT("pix not defined", __func__, 1);

    pixSetPadBits(pix, 0);
    if (pixWriteMemGif(&filedata, &filebytes, pix) != 0) {
        LEPT_FREE(filedata);
        return ERROR_INT("failure to gif encode pix", __func__, 1);
    }

    rewind(fp);
    nbytes = fwrite(filedata, 1, filebytes, fp);
    LEPT_FREE(filedata);
    if (nbytes != filebytes)
        return ERROR_INT("write error", __func__, 1);
    return 0;
}


/*!
 * \brief   pixWriteMemGif()
 *
 * \param[out]   pdata data of gif compressed image
 * \param[out]   psize size of returned data
 * \param[in]    pix
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) See comments in pixReadMemGif()
 * </pre>
 */
l_ok
pixWriteMemGif(l_uint8  **pdata,
               size_t    *psize,
               PIX       *pix)
{
int           giferr;
l_int32       result;
GifFileType  *gif;
L_BBUFFER    *buffer;

        /* 5.1+ and not 5.1.2 */
#if (GIFLIB_MAJOR < 5 || (GIFLIB_MAJOR == 5 && GIFLIB_MINOR == 0))
    L_ERROR("Require giflib-5.1 or later\n", __func__);
    return 1;
#endif  /* < 5.1 */
#if GIFLIB_MAJOR == 5 && GIFLIB_MINOR == 1 && GIFLIB_RELEASE == 2  /* 5.1.2 */
    L_ERROR("Can't use giflib-5.1.2; suggest 5.1.3 or later\n", __func__);
    return 1;
#endif  /* 5.1.2 */

    if (!pdata)
        return ERROR_INT("&data not defined", __func__, 1 );
    *pdata = NULL;
    if (!psize)
        return ERROR_INT("&size not defined", __func__, 1 );
    *psize = 0;
    if (!pix)
        return ERROR_INT("&pix not defined", __func__, 1 );

    if ((buffer = bbufferCreate(NULL, 0)) == NULL)
        return ERROR_INT("failed to create buffer", __func__, 1);

    if ((gif = EGifOpen((void*)buffer, gifWriteFunc, NULL)) == NULL) {
        bbufferDestroy(&buffer);
        return ERROR_INT("failed to create GIF image handle", __func__, 1);
    }

    result = pixToGif(pix, gif);
    EGifCloseFile(gif, &giferr);

    if (result == 0) {
        *pdata = bbufferDestroyAndSaveData(&buffer, psize);
    } else {
        bbufferDestroy(&buffer);
    }
    return result;
}


static l_int32
gifWriteFunc(GifFileType        *gif,
             const GifByteType  *src,
             l_int32             bytesToWrite)
{
L_BBUFFER  *buffer;

    if ((buffer = (L_BBUFFER*)gif->UserData) == NULL)
        return ERROR_INT("UserData not set", __func__, -1);

    if(bbufferRead(buffer, (l_uint8*)src, bytesToWrite) == 0)
        return bytesToWrite;
    return 0;
}


/*!
 * \brief   pixToGif()
 *
 * \param[in]  pix    1, 2, 4, 8, 16 or 32 bpp
 * \param[in]  gif    opened gif stream
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) This encodes the pix to the gif stream. The stream is not
 *          closed by this function.
 *      (2) It is static to make this function private.
 * </pre>
 */
static l_int32
pixToGif(PIX          *pix,
         GifFileType  *gif)
{
char            *text;
l_int32          wpl, i, j, w, h, d, ncolor, rval, gval, bval, valid;
l_int32          gif_ncolor = 0;
l_uint32        *data, *line;
PIX             *pixd;
PIXCMAP         *cmap;
ColorMapObject  *gif_cmap;
GifByteType     *gif_line;

    if (!pix)
        return ERROR_INT("pix not defined", __func__, 1);
    if (!gif)
        return ERROR_INT("gif not defined", __func__, 1);

    d = pixGetDepth(pix);
    if (d == 32) {
        pixd = pixConvertRGBToColormap(pix, 1);
    } else if (d > 1) {
        pixd = pixConvertTo8(pix, TRUE);
    } else {  /* d == 1; make sure there's a colormap */
        pixd = pixClone(pix);
        if (!pixGetColormap(pixd)) {
            cmap = pixcmapCreate(1);
            pixcmapAddColor(cmap, 255, 255, 255);
            pixcmapAddColor(cmap, 0, 0, 0);
            pixSetColormap(pixd, cmap);
        }
    }

    if (!pixd)
        return ERROR_INT("failed to convert to colormapped pix", __func__, 1);
    d = pixGetDepth(pixd);
    cmap = pixGetColormap(pixd);
    if (!cmap) {
        pixDestroy(&pixd);
        return ERROR_INT("cmap is missing", __func__, 1);
    }
    pixcmapIsValid(cmap, pixd, &valid);
    if (!valid) {
        pixDestroy(&pixd);
        return ERROR_INT("colormap is not valid", __func__, 1);
    }

        /* 'Round' the number of gif colors up to a power of 2 */
    ncolor = pixcmapGetCount(cmap);
    for (i = 0; i <= 8; i++) {
        if ((1 << i) >= ncolor) {
            gif_ncolor = (1 << i);
            break;
        }
    }
    if (gif_ncolor < 1) {
        pixDestroy(&pixd);
        return ERROR_INT("number of colors is invalid", __func__, 1);
    }

        /* Save the cmap colors in a gif_cmap */
    if ((gif_cmap = GifMakeMapObject(gif_ncolor, NULL)) == NULL) {
        pixDestroy(&pixd);
        return ERROR_INT("failed to create GIF color map", __func__, 1);
    }
    for (i = 0; i < gif_ncolor; i++) {
        rval = gval = bval = 0;
        if (ncolor > 0) {
            if (pixcmapGetColor(cmap, i, &rval, &gval, &bval) != 0) {
                pixDestroy(&pixd);
                GifFreeMapObject(gif_cmap);
                return ERROR_INT("failed to get color from color map",
                                 __func__, 1);
            }
            ncolor--;
        }
        gif_cmap->Colors[i].Red = rval;
        gif_cmap->Colors[i].Green = gval;
        gif_cmap->Colors[i].Blue = bval;
    }

    pixGetDimensions(pixd, &w, &h, NULL);
    if (EGifPutScreenDesc(gif, w, h, gif_cmap->BitsPerPixel, 0, gif_cmap)
        != GIF_OK) {
        pixDestroy(&pixd);
        GifFreeMapObject(gif_cmap);
        return ERROR_INT("failed to write screen description", __func__, 1);
    }
    GifFreeMapObject(gif_cmap); /* not needed after this point */

    if (EGifPutImageDesc(gif, 0, 0, w, h, FALSE, NULL) != GIF_OK) {
        pixDestroy(&pixd);
        return ERROR_INT("failed to image screen description", __func__, 1);
    }

    data = pixGetData(pixd);
    wpl = pixGetWpl(pixd);
    if (d != 1 && d != 2 && d != 4 && d != 8) {
        pixDestroy(&pixd);
        return ERROR_INT("image depth is not in {1, 2, 4, 8}", __func__, 1);
    }

    if ((gif_line = (GifByteType *)LEPT_CALLOC(sizeof(GifByteType), w))
        == NULL) {
        pixDestroy(&pixd);
        return ERROR_INT("mem alloc fail for data line", __func__, 1);
    }

    for (i = 0; i < h; i++) {
        line = data + i * wpl;
            /* Gif's way of setting the raster line up for compression */
        for (j = 0; j < w; j++) {
            switch(d)
            {
            case 8:
                gif_line[j] = GET_DATA_BYTE(line, j);
                break;
            case 4:
                gif_line[j] = GET_DATA_QBIT(line, j);
                break;
            case 2:
                gif_line[j] = GET_DATA_DIBIT(line, j);
                break;
            case 1:
                gif_line[j] = GET_DATA_BIT(line, j);
                break;
            }
        }

            /* Compress and save the line */
        if (EGifPutLine(gif, gif_line, w) != GIF_OK) {
            LEPT_FREE(gif_line);
            pixDestroy(&pixd);
            return ERROR_INT("failed to write data line into GIF", __func__, 1);
        }
    }

        /* Write a text comment.  This must be placed after writing the
         * data (!!)  Note that because libgif does not provide a function
         * for reading comments from file, you will need another way
         * to read comments. */
    if ((text = pixGetText(pix)) != NULL) {
        if (EGifPutComment(gif, text) != GIF_OK)
            L_WARNING("gif comment not written\n", __func__);
    }

    LEPT_FREE(gif_line);
    pixDestroy(&pixd);
    return 0;
}


#if 0
/*---------------------------------------------------------------------*
 *         Removing interlacing (reference only; not used)             *
 *---------------------------------------------------------------------*/
    /* GIF supports 4-way interlacing by raster lines.
     * Before 5.0, it was necessary for leptonica to restore interlaced
     * data to normal raster order when reading to a pix. With 5.0,
     * the de-interlacing is done by the library read function.
     * It is here only as a reference. */
static const l_int32 InterlacedOffset[] = {0, 4, 2, 1};
static const l_int32 InterlacedJumps[] = {8, 8, 4, 2};

static PIX *
pixUninterlaceGIF(PIX  *pixs)
{
l_int32    w, h, d, wpl, j, k, srow, drow;
l_uint32  *datas, *datad, *lines, *lined;
PIX       *pixd;

    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, NULL);

    pixGetDimensions(pixs, &w, &h, &d);
    wpl = pixGetWpl(pixs);
    pixd = pixCreateTemplate(pixs);
    datas = pixGetData(pixs);
    datad = pixGetData(pixd);
    for (k = 0, srow = 0; k < 4; k++) {
        for (drow = InterlacedOffset[k]; drow < h;
             drow += InterlacedJumps[k], srow++) {
            lines = datas + srow * wpl;
            lined = datad + drow * wpl;
            for (j = 0; j < w; j++)
                memcpy(lined, lines, 4 * wpl);
        }
    }

    return pixd;
}
#endif


/* -----------------------------------------------------------------*/
#endif    /* HAVE_LIBGIF || HAVE_LIBUNGIF  */
