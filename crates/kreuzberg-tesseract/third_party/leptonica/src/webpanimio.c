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
 * \file webpanimio.c
 * <pre>
 *
 *    Writing animated WebP
 *          l_int32          pixaWriteWebPAnim()
 *          l_int32          pixaWriteStreamWebPAnim()
 *          l_int32          pixaWriteMemWebPAnim()
 *
 * This shim was written in 2013, shortly after Google introduced animated webp.
 * However, even though animated webp is better than gifanim by every
 * technical measure (e.g., flexibility, compression), animated webp
 * is still not well supported, even with Google apps like gmail!
 *
 * To convert from gifanim to animated webp:
 *  (1) use ImageMagick to make a set of png files:
 *      convert infile.gif -coalesce outroot_%03d.png
 *  (2) read these files into a pixa, using
 *      pixaReadFiles()
 *  (3) write out the animated webp, selecting the duration and using
 *      pixaWriteWebPAnim()
 * </pre>
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

/* -----------------------------------------------*/
#if  HAVE_LIBWEBP_ANIM   /* defined in environ.h  */
/* -----------------------------------------------*/
#include "webp/encode.h"
#include "webp/mux.h"

/*---------------------------------------------------------------------*
 *                       Writing animated WebP                         *
 *---------------------------------------------------------------------*/
/*!
 * \brief   pixaWriteWebPAnim()
 *
 * \param[in]    filename
 * \param[in]    pixa        with images of all depths; cmap OK
 * \param[in]    loopcount   [0 for infinite]
 * \param[in]    duration    in ms, for each image
 * \param[in]    quality     0 - 100 for lossy; default ~80
 * \param[in]    lossless    use 1 for lossless; 0 for lossy
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) Special top-level function allowing specification of quality.
 * </pre>
 */
l_ok
pixaWriteWebPAnim(const char  *filename,
                  PIXA        *pixa,
                  l_int32      loopcount,
                  l_int32      duration,
                  l_int32      quality,
                  l_int32      lossless)
{
l_int32  ret;
FILE    *fp;

    if (!filename)
        return ERROR_INT("filename not defined", __func__, 1);
    if (!pixa)
        return ERROR_INT("pixa not defined", __func__, 1);

    if ((fp = fopenWriteStream(filename, "wb+")) == NULL)
        return ERROR_INT_1("stream not opened", filename, __func__, 1);
    ret = pixaWriteStreamWebPAnim(fp, pixa, loopcount, duration,
                                  quality, lossless);
    fclose(fp);
    if (ret)
        return ERROR_INT_1("pixs not compressed to stream",
                           filename, __func__, 1);
    return 0;
}


/*!
 * \brief   pixaWriteStreamWebPAnim()
 *
 * \param[in]    fp          file stream
 * \param[in]    pixa        with images of all depths; cmap OK
 * \param[in]    loopcount   [0 for infinite]
 * \param[in]    duration    in ms, for each image
 * \param[in]    quality     0 - 100 for lossy; default ~80
 * \param[in]    lossless    use 1 for lossless; 0 for lossy
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) See pixWriteMemWebP() for details.
 *      (2) Use 'free', and not leptonica's 'LEPT_FREE', for all heap data
 *          that is returned from the WebP library.
 * </pre>
 */
l_ok
pixaWriteStreamWebPAnim(FILE    *fp,
                        PIXA    *pixa,
                        l_int32  loopcount,
                        l_int32  duration,
                        l_int32  quality,
                        l_int32  lossless)
{
l_uint8  *filedata;
size_t    filebytes, nbytes;

    if (!fp)
        return ERROR_INT("stream not open", __func__, 1);
    if (!pixa)
        return ERROR_INT("pixa not defined", __func__, 1);

    filedata = NULL;
    pixaWriteMemWebPAnim(&filedata, &filebytes, pixa, loopcount,
                         duration, quality, lossless);
    rewind(fp);
    if (!filedata)
        return ERROR_INT("filedata not made", __func__, 1);
    nbytes = fwrite(filedata, 1, filebytes, fp);
    free(filedata);
    if (nbytes != filebytes)
        return ERROR_INT("Write error", __func__, 1);
    return 0;
}


/*!
 * \brief   pixaWriteMemWebPAnim()
 *
 * \param[out]   pencdata    webp encoded data of pixs
 * \param[out]   pencsize    size of webp encoded data
 * \param[in]    pixa        with images of any depth, cmapped OK
 * \param[in]    loopcount   [0 for infinite]
 * \param[in]    duration    in ms, for each image
 * \param[in]    quality     0 - 100 for lossy; default ~80
 * \param[in]    lossless    use 1 for lossless; 0 for lossy
 * \return  0 if OK, 1 on error
 *
 * <pre>
 * Notes:
 *      (1) See pixWriteMemWebP() for details of webp encoding of images.
 * </pre>
 */
l_ok
pixaWriteMemWebPAnim(l_uint8  **pencdata,
                     size_t    *pencsize,
                     PIXA      *pixa,
                     l_int32    loopcount,
                     l_int32    duration,
                     l_int32    quality,
                     l_int32    lossless)
{
l_int32                 i, n, same, w, h, wpl, ret, ret_webp;
l_uint8                *data;
PIX                    *pix1, *pix2;
WebPAnimEncoder        *enc;
WebPAnimEncoderOptions  enc_options;
WebPConfig              config;
WebPData                webp_data;
WebPMux                *mux = NULL;
WebPMuxAnimParams       newparams;
WebPPicture             frame;

    if (!pencdata)
        return ERROR_INT("&encdata not defined", __func__, 1);
    *pencdata = NULL;
    if (!pencsize)
        return ERROR_INT("&encsize not defined", __func__, 1);
    *pencsize = 0;
    if (!pixa)
        return ERROR_INT("&pixa not defined", __func__, 1);
    if ((n = pixaGetCount(pixa)) == 0)
        return ERROR_INT("no images in pixa", __func__, 1);
    if (loopcount < 0) loopcount = 0;
    if (lossless == 0 && (quality < 0 || quality > 100))
        return ERROR_INT("quality not in [0 ... 100]", __func__, 1);

    pixaVerifyDimensions(pixa, &same, &w, &h);
    if (!same)
        return ERROR_INT("sizes of all pix are not the same", __func__, 1);

        /* Set up the encoder */
    if (!WebPAnimEncoderOptionsInit(&enc_options))
        return ERROR_INT("cannot initialize WebP encoder options", __func__, 1);
    if (!WebPConfigInit(&config))
        return ERROR_INT("cannot initialize WebP config", __func__, 1);
    config.lossless = lossless;
    config.quality = quality;
    if ((enc = WebPAnimEncoderNew(w, h, &enc_options)) == NULL)
        return ERROR_INT("cannot create WebP encoder", __func__, 1);

    for (i = 0; i < n; i++) {
            /* Make a frame for each image.  Convert the pix to RGBA with
             * an opaque alpha layer, and put the raster data in the frame. */
        if (!WebPPictureInit(&frame)) {
            WebPAnimEncoderDelete(enc);
            return ERROR_INT("cannot initialize WebP picture", __func__, 1);
        }
        pix1 = pixaGetPix(pixa, i, L_CLONE);
        pix2 = pixConvertTo32(pix1);
        pixSetComponentArbitrary(pix2, L_ALPHA_CHANNEL, 255);
        pixEndianByteSwap(pix2);
        data = (l_uint8 *)pixGetData(pix2);
        wpl = pixGetWpl(pix2);
        frame.width = w;
        frame.height = h;
        ret_webp = WebPPictureImportRGBA(&frame, data, 4 * wpl);
        pixDestroy(&pix1);
        pixDestroy(&pix2);
        if (!ret_webp) {
            WebPAnimEncoderDelete(enc);
            return ERROR_INT("cannot import RGBA picture", __func__, 1);
        }

            /* Add the frame data to the encoder, and clear its memory */
        ret_webp = WebPAnimEncoderAdd(enc, &frame, duration * i, &config);
        WebPPictureFree(&frame);
        if (!ret_webp) {
            WebPAnimEncoderDelete(enc);
            return ERROR_INT("cannot add frame to animation", __func__, 1);
        }
    }
        /* Add a blank frame */
    if (!WebPAnimEncoderAdd(enc, NULL, duration * i, NULL)) {
        WebPAnimEncoderDelete(enc);
        return ERROR_INT("blank frame not added to animation", __func__, 1);
    }

        /* Encode the data */
    ret_webp = WebPAnimEncoderAssemble(enc, &webp_data);
    WebPAnimEncoderDelete(enc);
    if (!ret_webp)
        return ERROR_INT("cannot assemble animation", __func__, 1);

        /* Set the loopcount if requested.  Note that when you make a mux,
         * it imports the webp_data that was previously made, including
         * the webp encoded images.  Before you re-export that data using
         * WebPMuxAssemble(), free the heap data in webp_data.  There is an
         * example for setting the loop count in the webp distribution;
         * see gif2webp.c.  */
    if (loopcount > 0) {
        mux = WebPMuxCreate(&webp_data, 1);
        if (!mux) {
            L_ERROR("could not re-mux to add loop count\n", __func__);
        } else {
            ret = WebPMuxGetAnimationParams(mux, &newparams);
            if (ret != WEBP_MUX_OK) {
                L_ERROR("failed to get loop count\n", __func__);
            } else {
                newparams.loop_count = loopcount;
                ret = WebPMuxSetAnimationParams(mux, &newparams);
                if (ret != WEBP_MUX_OK)
                    L_ERROR("failed to set loop count\n", __func__);
            }
            WebPDataClear(&webp_data);
            if (WebPMuxAssemble(mux, &webp_data) != WEBP_MUX_OK)
                L_ERROR("failed to assemble in the WebP muxer\n", __func__);
            WebPMuxDelete(mux);
        }
    }

    *pencdata = (l_uint8 *)webp_data.bytes;
    *pencsize = webp_data.size;
    L_INFO("data size = %zu\n", __func__, webp_data.size);
    return 0;
}


/* --------------------------------------------*/
#endif  /* HAVE_LIBWEBP_ANIM */
/* --------------------------------------------*/
