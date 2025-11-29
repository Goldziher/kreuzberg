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
 * scale_it.c
 *
 *    scale_it filein scalex scaley fileout lossless [sharpen antialias togray]
 *
 *    where
 *         scalex:    floating pt input
 *         scaley:    ditto
 *         lossless:  (for bpp >= 8) 1 to output jpeg; 0 to output png
 *         sharpen :  (for bpp > 1; scale factor in [0.2 ... 1.4]):
 *                    1 to sharpen; 0 not to sharpen
 *         antialias: (for bpp > 1): 1 to use area-mapping or linear
 *                    interpolation; 0 for sampling.
 *         togray:    (for bpp == 1, reduction): 1 for scale-to-gray;
 *                    0 for sampling
 *
 *    The choice of writing lossless (png) or lossy (jpeg) only applies
 *    for bpp >= 8.  Otherwise:
 *          bpp == 1 -->  tiffg4
 *          bpp == 2 -->  png
 *          bpp == 4 -->  png
 *
 *    Sharpening: no sharpening is done for scale factors < 0.2 or > 1.4.
 *    Sharpening increases the saliency of edges, making the scaled image
 *    look less fuzzy.  It is much slower than scaling without sharpening.
 *    The default is to sharpen.
 *
 *    Antialias: area-mapping and linear interpolation give higher
 *    quality results with bpp > 1.  Sampling is faster, but shows
 *    artifacts, such as pixel-sized steps in lines.  The default is
 *    to use antialiasing.
 *
 *    ScaleToGray: for bpp == 1, downscaling to gray gives a better appearance
 *    than subsampling.  The default is to scale-to-gray.
 *
 *    The defaults are all intended to improve the quality of the result.
 *    The quality can be degraded, with faster processing, by setting
 *    some of the three optional inputs to 0.
 *
 *    Note that the short form:
 *        scale_it filein scalex scaley fileout lossless
 *    is equivalent to
 *        scale_it filein scalex scaley fileout lossless 1 1 1
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char      *filein, *fileout;
l_int32    sharpen, antialias, togray, lossless, d;
l_float32  scalex, scaley;
PIX       *pixs, *pixd;

    if (argc != 6 && argc != 9)
        return ERROR_INT(
            "\n    Syntax:  scale_it filein scalex scaley fileout lossless "
                           "[sharpen antialias togray]",
            __func__, 1);

    filein = argv[1];
    scalex = atof(argv[2]);
    scaley = atof(argv[3]);
    fileout = argv[4];
    lossless = atoi(argv[5]);
    sharpen = antialias = togray = 1;
    if (argc == 9) {
        sharpen = atoi(argv[6]);
        antialias = atoi(argv[7]);
        togray = atoi(argv[8]);
    }
    if (scalex <= 0 || scaley <= 0)
        return ERROR_INT("invalid scale factor; must be > 0.0", __func__, 1);
    setLeptDebugOK(1);

    if ((pixs = pixRead(filein)) == NULL)
	return ERROR_INT("pixs not made", __func__, 1);
    d = pixGetDepth(pixs);
    if (d == 1) {
        if (togray && scalex < 1.0)
            pixd = pixScaleToGray(pixs, scalex);
        else  /* this will just scale by sampling */
            pixd = pixScaleBinary(pixs, scalex, scaley);
    } else {
        if (antialias == 0) {
            pixd = pixScaleBySampling(pixs, scalex, scaley);
        } else if (sharpen == 0) {
            pixd = pixScaleGeneral(pixs, scalex, scaley, 0.0, 0);
        } else {  /* antialias == 1, sharpen == 1 */
            pixd = pixScale(pixs, scalex, scaley);
        }
    }
    if (!pixd)
	return ERROR_INT("pixd not made", __func__, 1);

    d = pixGetDepth(pixd);
    if (d == 1)
        pixWrite(fileout, pixd, IFF_TIFF_G4);
    else if (d == 2 || d == 4)
        pixWrite(fileout, pixd, IFF_PNG);
    else {  /* d >= 8 */
        if (lossless)
            pixWrite(fileout, pixd, IFF_PNG);
        else
            pixWrite(fileout, pixd, IFF_JFIF_JPEG);
    }

    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}
