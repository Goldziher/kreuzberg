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
 * rotate_it.c
 *
 *    rotate_it filein angle fileout [type incolor]
 *
 *    where:
 *         angle: in degrees; use 90, 180, 270 for orthogonal rotation
 *         type: "areamap", "shear", "sampling"
 *         incolor:  "black", "white"
 *
 *    If 'type' and 'incolor' are omitted, by default we use:
 *         type: sampling for 1 bpp; areamap for bpp > 1
 *         incolor: white
 *
 *    If angle is in {90.0, 180.0, 270.0}, this does an orthogonal
 *    rotation. Args 'type' and 'incolor' can be omitted.
 *
 *    This writes the output file in the same encoded format as
 *    the input file.  If the input file is jpeg, the output file
 *    is written with default quality factor 75.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

int main(int    argc,
         char **argv)
{
l_int32    icolor, itype, format, quads;
l_float32  angle, deg2rad, anglerad;
char      *filein, *fileout, *type, *incolor;
PIX       *pixs, *pixd;

    if (argc != 4 && argc != 6)
        return ERROR_INT(
            "\n    Syntax:  rotate_it filein angle fileout [type incolor]",
            __func__, 1);
    filein = argv[1];
    angle = atof(argv[2]);
    fileout = argv[3];
    if (argc == 6) {
        type = argv[4];
        incolor = argv[5];
    }

    setLeptDebugOK(1);
    deg2rad = 3.1415926535 / 180.;

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    format = pixGetInputFormat(pixs);
    if (format == IFF_UNKNOWN) format = IFF_PNG;

    if (angle == 90.0 || angle == 180.0 || angle == 270.0) {
        quads = (l_int32)((angle + 0.5) / 90.0);
        pixd = pixRotateOrth(pixs, quads);
        pixWrite(fileout, pixd, format);
        pixDestroy(&pixs);
        pixDestroy(&pixd);
        return 0;
    }

    anglerad = deg2rad * angle;
    icolor = L_BRING_IN_WHITE;
    itype = L_ROTATE_AREA_MAP;
    if (argc == 6) {
        icolor = (!strcmp(incolor, "white")) ? L_BRING_IN_WHITE
                                            : L_BRING_IN_BLACK;
        if (!strcmp(type, "areamap"))
            itype = L_ROTATE_AREA_MAP;
        else if (!strcmp(type, "shear"))
            itype = L_ROTATE_SHEAR;
        else
            itype = L_ROTATE_SAMPLING;
    }

    pixd = pixRotate(pixs, anglerad, itype, icolor, 0, 0);
    pixWrite(fileout, pixd, format);
    pixDestroy(&pixs);
    pixDestroy(&pixd);
    return 0;
}
