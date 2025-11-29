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
 *  rotatefastalt.c
 *
 *      Alternative (slightly slower) method for rotating color images,
 *      with antialiasing.  This is here just for comparison with
 *      the better methods in the library.
 *
 *      Includes these functions:
 *          pixRotateAMColorFast2()
 *          pixShiftRGB258()
 *          rotateAMColorFastLow2()
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include <math.h>   /* required for sin and tan */
#include "allheaders.h"

static const l_float32  VERY_SMALL_ANGLE = 0.001;  /* radians; ~0.06 degrees */

static PIX *pixRotateAMColorFast2(PIX *pixs, l_float32 angle, l_uint8 grayval);
static PIX *pixShiftRGB258(PIX  *pixs);
static void rotateAMColorFastLow2(l_uint32  *datad, l_int32  w, l_int32  h,
                                  l_int32  wpld, l_uint32  *datas,
                                  l_int32  wpls, l_float32  angle,
                                  l_uint8  grayval);

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
l_float32  angle, deg2rad;
PIX       *pixs, *pixd;

    if (argc != 4)
        return ERROR_INT("Syntax:  rotatefastalt filein angle fileout",
                         __func__, 1);
    filein = argv[1];
    angle = atof(argv[2]);
    fileout = argv[3];

    setLeptDebugOK(1);
    deg2rad = 3.1415926535 / 180.;
    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not read", __func__, 1);

    startTimer();
    pixd = pixRotateAMColorFast2(pixs, deg2rad * angle, 255);
    lept_stderr("Time for rotation: %7.3f sec\n", stopTimer());
    pixWrite(fileout, pixd, IFF_JFIF_JPEG);

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}


/*!
 *  pixRotateAMColorFast2()
 *
 *      Input: pixs
 *             angle (radians; clockwise is positive)
 *             grayval (0 to bring in BLACK, 255 for WHITE)
 *      Return:  pixd, or null on error
 *
 *  Notes:
 *      - This rotates a color image about the image center.
 *        A positive angle gives a clockwise rotation.
 *      - It uses area mapping, dividing each pixel into
 *        16 subpixels.
 *      - It creates a temporary 32-bit color image.
 *      - It is slightly slower than pixRotateAMColorFast(),
 *        which uses less memory because it does not create
 *        a temporary image.
 *
 *  *** Warning: implicit assumption about RGB component ordering ***
 */
PIX *
pixRotateAMColorFast2(PIX       *pixs,
                      l_float32  angle,
                      l_uint8    grayval)
{
l_int32    w, h, wpls, wpld;
l_uint32  *datas, *datad;
PIX       *pixshft, *pixd;

    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, NULL);
    if (pixGetDepth(pixs) != 32)
        return (PIX *)ERROR_PTR("pixs must be 32 bpp", __func__, NULL);

    if (L_ABS(angle) < VERY_SMALL_ANGLE)
        return pixClone(pixs);

    if ((pixshft = pixShiftRGB258(pixs)) == NULL)
        return (PIX *)ERROR_PTR("pixshft not defined", __func__, NULL);

    w = pixGetWidth(pixshft);
    h = pixGetHeight(pixshft);
    datas = pixGetData(pixshft);
    wpls = pixGetWpl(pixshft);
    pixd = pixCreateTemplate(pixshft);
    datad = pixGetData(pixd);
    wpld = pixGetWpl(pixd);
    rotateAMColorFastLow2(datad, w, h, wpld, datas, wpls, angle, grayval);

    pixDestroy(&pixshft);
    return pixd;
}


/*!
 *  pixShiftRGB258()
 *
 *      Makes a new 32 bpp image with the R, G and B components
 *      right-shifted by 2, 5 and 8 bits, respectively.
 */
PIX *
pixShiftRGB258(PIX  *pixs)
{
l_int32    w, h, wpls, wpld, i, j;
l_uint32   word;
l_uint32  *datas, *datad, *lines, *lined;
PIX       *pixd;

    if (!pixs)
        return (PIX *)ERROR_PTR("pixs not defined", __func__, NULL);
    if (pixGetDepth(pixs) != 32)
        return (PIX *)ERROR_PTR("depth not 32 bpp", __func__, NULL);
    w = pixGetWidth(pixs);
    h = pixGetHeight(pixs);
    wpls = pixGetWpl(pixs);
    datas = pixGetData(pixs);

    if ((pixd = pixCreate(w, h, 32)) == NULL)
        return (PIX *)ERROR_PTR("pixd not made", __func__, NULL);
    wpld = pixGetWpl(pixd);
    datad = pixGetData(pixd);

    for (i = 0; i < h; i++) {
        lines = datas + i * wpls;
        lined = datad + i * wpld;
        for (j = 0; j < w; j++) {
            word = *(lines + j);
            *(lined + j) = ((word & 0xff000000) >> 2) |
                           ((word & 0x00ff0000) >> 5) |
                           ((word & 0x0000ff00) >> 8);
        }
    }

    return pixd;
}


/*!
 *  rotateAMColorFastLow2()
 *
 *  Alternative version for fast color rotation
 *
 *  *** Warning: explicit assumption about RGB component ordering ***
 */
void
rotateAMColorFastLow2(l_uint32  *datad,
                      l_int32    w,
                      l_int32    h,
                      l_int32    wpld,
                      l_uint32  *datas,
                      l_int32    wpls,
                      l_float32  angle,
                      l_uint8    grayval)
{
l_int32    i, j, xcen, ycen, wm2, hm2;
l_int32    xdif, ydif, xpm, ypm, xp, yp, xf, yf;
l_uint32   edgeval, word;
l_uint32  *pword, *lines, *lined;
l_float32  sina, cosa;

    xcen = w / 2;
    wm2 = w - 2;
    ycen = h / 2;
    hm2 = h - 2;
    sina = 4. * sin(angle);
    cosa = 4. * cos(angle);

    edgeval = (grayval << 24) | (grayval << 16) | (grayval << 8);
    for (i = 0; i < h; i++) {
        ydif = ycen - i;
        lined = datad + i * wpld;
        for (j = 0; j < w; j++) {
            xdif = xcen - j;
            xpm = (l_int32)(-xdif * cosa - ydif * sina + 0.5);
            ypm = (l_int32)(-ydif * cosa + xdif * sina + 0.5);
            xp = xcen + (xpm >> 2);
            yp = ycen + (ypm >> 2);
            xf = xpm & 0x03;
            yf = ypm & 0x03;

                /* if off the edge, write the input grayval */
            if (xp < 0 || yp < 0 || xp > wm2 || yp > hm2) {
                *(lined + j) = edgeval;
                continue;
            }

            lines = datas + yp * wpls;
            pword = lines + xp;

            switch (xf + 4 * yf)
            {
            case 0:
                word = *pword;
                *(lined + j) = ((word & 0x3fc00000) << 2) |
                               ((word & 0x0007f800) << 5) |
                               ((word & 0x000000ff) << 8);
                break;
            case 1:
                word = 3 * (*pword) + *(pword + 1);
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            case 2:
                word = *pword + *(pword + 1);
                *(lined + j) = ((word & 0x7f800000) << 1) |
                               ((word & 0x000ff000) << 4) |
                               ((word & 0x000001fe) << 7);
                break;
            case 3:
                word = *pword + 3 * (*(pword + 1));
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            case 4:
                word = 3 * (*pword) + *(pword + wpls);
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            case 5:
                word = 2 * (*pword) + *(pword + 1) + *(pword + wpls);
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            case 6:
                word = *pword + *(pword + 1);
                *(lined + j) = ((word & 0x7f800000) << 1) |
                               ((word & 0x000ff000) << 4) |
                               ((word & 0x000001fe) << 7);
                break;
            case 7:
                word = *pword + 2 * (*(pword + 1)) + *(pword + wpls + 1);
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            case 8:
                word = *pword + *(pword + wpls);
                *(lined + j) = ((word & 0x7f800000) << 1) |
                               ((word & 0x000ff000) << 4) |
                               ((word & 0x000001fe) << 7);
                break;
            case 9:
                word = *pword + *(pword + wpls);
                *(lined + j) = ((word & 0x7f800000) << 1) |
                               ((word & 0x000ff000) << 4) |
                               ((word & 0x000001fe) << 7);
                break;
            case 10:
                word = *pword + *(pword + 1) + *(pword + wpls) +
                       *(pword + wpls + 1);
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            case 11:
                word = *(pword + 1) + *(pword + wpls + 1);
                *(lined + j) = ((word & 0x7f800000) << 1) |
                               ((word & 0x000ff000) << 4) |
                               ((word & 0x000001fe) << 7);
                break;
            case 12:
                word = *pword + 3 * (*(pword + wpls));
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            case 13:
                word = *pword + 2 * (*(pword + wpls)) + *(pword + wpls + 1);
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            case 14:
                word = *(pword + wpls) + *(pword + wpls + 1);
                *(lined + j) = ((word & 0x7f800000) << 1) |
                               ((word & 0x000ff000) << 4) |
                               ((word & 0x000001fe) << 7);
                break;
            case 15:
                word = *(pword + 1) + *(pword + wpls) +
                           2 * (*(pword + wpls + 1));
                *(lined + j) = (word & 0xff000000) |
                               ((word & 0x001fe000) << 3) |
                               ((word & 0x000003fc) << 6);
                break;
            default:  /* for testing only; no interpolation, no shift */
                lept_stderr("shouldn't get here\n");
                *(lined + j) = *pword;
                break;
            }
        }
    }

    return;
}
