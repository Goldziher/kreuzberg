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
 * corrupttest.c
 *
 *     Excises or permutes a given fraction of bytes, starting from
 *     a specified location.  The parameters @loc and @size are fractions
 *     of the entire file (between 0.0 and 1.0).
 *
 *     Syntax:  corrupttest <file> <deletion> [loc size]
 *
 *        where <deletion> == 1 means that bytes are deleted
 *              <deletion> == 0 means that random bytes are substituted
 *
 *     Use: "fuzz testing" jpeg, png, tiff, bmp, webp and pnm reading,
 *          under corruption by either random byte substitution or
 *          deletion of part of the compressed file.
 *
 *     For example,
 *          corrupttest rabi.png 0 0.0001 0.0001
 *     which tests read functions on rabi.png after 23 bytes (0.01%)
 *     starting at byte 23 have been randomly permuted, emits the following:
 *      > Info in fileCorruptByMutation: Randomizing 23 bytes at location 23
 *      > libpng error: IHDR: CRC error
 *      > Error in pixReadMemPng: internal png error
 *      > Error in pixReadStream: png: no pix returned
 *      > Error in pixRead: pix not read
 *      > libpng error: IHDR: CRC error
 *      > Error in fgetPngResolution: internal png error
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "string.h"
#include "allheaders.h"

static const char *corruptfile = "/tmp/lept/corrupt/badfile";

int main(int     argc,
         char  **argv)
{
size_t     filesize;
l_float32  loc, size;
l_float32  coeff1[15], coeff2[25];
l_int32    i, j, w, xres, yres, format, ret, nwarn, hint, deletion, show;
l_uint8   *comment, *filedata;
char      *filein;
size_t     nbytes;
FILE      *fp;
PIX       *pix;

    if (argc != 3 && argc != 5)
        return ERROR_INT("syntax: corrupttest filein deletion [loc size]",
        __func__, 1);
    filein = argv[1];
    deletion = atoi(argv[2]);
    findFileFormat(filein, &format);
    nbytes = nbytesInFile(filein);
    lept_stderr("file size: %lu bytes\n", (unsigned long)nbytes);

    setLeptDebugOK(1);
    lept_mkdir("lept/corrupt");

    hint = 0;
    if (argc == 5) {  /* Single test */
        loc = atof(argv[3]);
        size = atof(argv[4]);
        if (deletion == TRUE) {
            fileCorruptByDeletion(filein, loc, size, corruptfile);
        } else {  /* mutation */
            fileCorruptByMutation(filein, loc, size, corruptfile);
        }
        if ((fp = lept_fopen(corruptfile, "rb")) == NULL)
            return ERROR_INT_1("stream not opened", corruptfile, __func__, 1);
        if (format == IFF_JFIF_JPEG) {
            if ((pix = pixReadJpeg(corruptfile, 0, 1, &nwarn, hint)) != NULL) {
                pixDisplay(pix, 100, 100);
                pixDestroy(&pix);
            }
            freadHeaderJpeg(fp, &w, NULL, NULL, NULL, NULL);
            fgetJpegResolution(fp, &xres, &yres);
            ret = fgetJpegComment(fp, &comment);
            if (!ret && comment) {
                lept_stderr("comment: %s\n", comment);
                lept_free(comment);
            }
        } else if (format == IFF_PNG) {
            if ((pix = pixRead(corruptfile)) != NULL) {
                pixDisplay(pix, 100, 100);
                pixDestroy(&pix);
            }
            freadHeaderPng(fp, &w, NULL, NULL, NULL, NULL);
            fgetPngResolution(fp, &xres, &yres);
        } else if (format == IFF_WEBP) {
            if ((pix = pixRead(corruptfile)) != NULL) {
                pixDisplay(pix, 100, 100);
                pixDestroy(&pix);
            }
            readHeaderWebP(corruptfile, &w, NULL, NULL);
        } else if (format == IFF_PNM) {
            if ((pix = pixRead(corruptfile)) != NULL) {
                pixDisplay(pix, 100, 100);
                pixDestroy(&pix);
            }
            freadHeaderPnm(fp, &w, NULL, NULL, NULL, NULL, NULL);
        }
        lept_fclose(fp);
        return 0;
    }

        /* Generate coefficients so that the size of the mangled
         * or deleted data can range from 0.001% to 1% of the file,
         * and the location of deleted data ranges from 0.001%
         * to 90% of the file. */
    for (i = 0; i < 15; i++) {
        if (i < 5) coeff1[i] = 0.00001;
        else if (i < 10) coeff1[i] = 0.0001;
        else coeff1[i] = 0.001;
    }
    for (i = 0; i < 25; i++) {
        if (i < 5) coeff2[i] = 0.00001;
        else if (i < 10) coeff2[i] = 0.0001;
        else if (i < 15) coeff2[i] = 0.001;
        else if (i < 20) coeff2[i] = 0.01;
        else coeff2[i] = 0.1;
    }

        /* Multiple test (argc == 3) */
    show = TRUE;
    for (i = 0; i < 25; i++) {
        loc = coeff2[i] * (2 * (i % 5) + 1);
        for (j = 0; j < 15; j++) {
            size = coeff1[j] * (2 * (j % 5) + 1);

                /* Write corrupt file */
            if (deletion == TRUE) {
                fileCorruptByDeletion(filein, loc, size, corruptfile);
            } else {
                fileCorruptByMutation(filein, loc, size, corruptfile);
            }

                /* Attempt to read the file */
            pix = NULL;
            if (format == IFF_JFIF_JPEG) {
                /* The pix is usually returned as long as the header
                 * information is not damaged.
                 * We expect nwarn > 0 (typically 1) for nearly every
                 * corrupted image.  In the situation where only a few
                 * bytes are removed, a corrupted image will occasionally
                 * have nwarn == 0 even though it's visually defective.  */
                pix = pixReadJpeg(corruptfile, 0, 1, &nwarn, 0);
                if (pix && nwarn != 1 && deletion == TRUE) {
                    lept_stderr("nwarn[%d,%d] = %d\n", j, i, nwarn);
                    if (show) pixDisplay(pix, 20 * i, 30 * j);
                    show = FALSE;
                }
            } else if (format == IFF_PNG) {
                pix = pixRead(corruptfile);
                if (pix) {
                    lept_stderr("pix[%d,%d] is read\n", j, i);
                    if (show) pixDisplay(pix, 20 * i, 30 * j);
                    show = FALSE;
                }
                pixDestroy(&pix);
                filedata = l_binaryRead(corruptfile, &filesize);
                pix = pixReadMemPng(filedata, filesize);
                lept_free(filedata);
            } else if (L_FORMAT_IS_TIFF(format)) {
                /* A corrupted pix is often returned, as long as the
                 * header is not damaged, so we do not display them.  */
                pix = pixRead(corruptfile);
                if (pix) lept_stderr("pix[%d,%d] is read\n", j, i);
                pixDestroy(&pix);
                filedata = l_binaryRead(corruptfile, &filesize);
                pix = pixReadMemTiff(filedata, filesize, 0);
                if (!pix) lept_stderr("no pix!\n");
                lept_free(filedata);
            } else if (format == IFF_BMP) {
                /* A corrupted pix is always returned if the header is
                 * not damaged, so we do not display them.  */
                pix = pixRead(corruptfile);
                if (pix) lept_stderr("pix[%d,%d] is read\n", j, i);
                pixDestroy(&pix);
                filedata = l_binaryRead(corruptfile, &filesize);
                pix = pixReadMemBmp(filedata, filesize);
                lept_free(filedata);
            } else if (format == IFF_WEBP) {
                /* A corrupted pix is always returned if the header is
                 * not damaged, so we do not display them.  */
                pix = pixRead(corruptfile);
                if (pix) lept_stderr("pix[%d,%d] is read\n", j, i);
                pixDestroy(&pix);
                filedata = l_binaryRead(corruptfile, &filesize);
                pix = pixReadMemWebP(filedata, filesize);
                lept_free(filedata);
            } else if (format == IFF_PNM) {
                /* A corrupted pix is always returned if the header is
                 * not damaged, so we do not display them.  */
                pix = pixRead(corruptfile);
                if (pix) lept_stderr("pix[%d,%d] is read\n", j, i);
                pixDestroy(&pix);
                filedata = l_binaryRead(corruptfile, &filesize);
                pix = pixReadMemPnm(filedata, filesize);
                lept_free(filedata);
            } else {
                lept_stderr("Format %d unknown\n", format);
                continue;
            }

                /* Effect of 1% byte mangling from interior of data stream */
            if (pix && j == 14 && i == 10 && deletion == FALSE)
                pixDisplay(pix, 0, 0);
            pixDestroy(&pix);

                /* Attempt to read the header and the resolution */
            fp = lept_fopen(corruptfile, "rb");
            if (format == IFF_JFIF_JPEG) {
                freadHeaderJpeg(fp, &w, NULL, NULL, NULL, NULL);
                if (fgetJpegResolution(fp, &xres, &yres) == 0)
                    lept_stderr("w = %d, xres = %d, yres = %d\n",
                    w, xres, yres);
            } else if (format == IFF_PNG)  {
                freadHeaderPng(fp, &w, NULL, NULL, NULL, NULL);
                if (fgetPngResolution(fp, &xres, &yres) == 0)
                    lept_stderr("w = %d, xres = %d, yres = %d\n",
                    w, xres, yres);
            } else if (L_FORMAT_IS_TIFF(format)) {
                freadHeaderTiff(fp, 0, &w, NULL, NULL, NULL, NULL, NULL, NULL);
                getTiffResolution(fp, &xres, &yres);
                lept_stderr("w = %d, xres = %d, yres = %d\n",
                        w, xres, yres);
            } else if (format == IFF_WEBP)  {
                if (readHeaderWebP(corruptfile, &w, NULL, NULL) == 0)
                    lept_stderr("w = %d\n", w);
            } else if (format == IFF_PNM)  {
                if (freadHeaderPnm(fp, &w, NULL, NULL, NULL, NULL, NULL) == 0)
                    lept_stderr("w = %d\n", w);
            }
            lept_fclose(fp);
        }
    }
    return 0;
}
