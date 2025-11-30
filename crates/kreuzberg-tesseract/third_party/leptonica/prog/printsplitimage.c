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
 * printsplitimage.c
 *
 *   Syntax:  printsplitimage filein nx ny [printer]
 *
 *        nx = number of horizontal tiles
 *        ny = number of vertical tiles
 *
 *   If printer is not specified, the only action is that the
 *   image is split into a set of tiles, and these are written
 *   out as a set of compressed (level 3) Postscript files.
 *   The images in the PostScript files are scaled to each fill
 *   an 8.5 x 11 inch page, up to the FILLING_FACTOR fraction
 *    in each direction.
 *
 *   If printer is specified, these are printed on separate pages,
 *   because some printers cannot print multi-page Postscript of images.
 *
 *   If your system does not have lpr, it likely has lp.  You can run
 *   printsplitimage to make the PostScript files, and print them with lp:
 *       lp -d <printer> /tmp/lept/split/image0.ps
 *       lp -d <printer> /tmp/lept/split/image1.ps
 *       ...
 *   To print in color, see prog/printimage.c.
 *
 *   ***************************************************************
 *   N.B.  If a printer is specified, this program invokes lpr via
 *         "system'.  It could pose a security vulnerability if used
 *         as a service in a production environment.  Consequently,
 *         this program should only be used for debug and testing.
 *   ***************************************************************
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

#define  USE_COMPRESSED    1

    /* fill factor on 8.5 x 11 inch output page */
static const l_float32   FILL_FACTOR = 0.95;

int main(int    argc,
         char **argv)
{
char        *filein, *fname, *printer;
char         buf[512];
l_int32      nx, ny, i, w, h, ws, hs, n, index;
l_float32    scale;
FILE        *fp;
PIX         *pixs, *pixt, *pixr;
PIXA        *pixa;
SARRAY      *sa;

    if (argc != 4 && argc != 5)
        return ERROR_INT(" Syntax:  printsplitimage filein nx ny [printer]",
                         __func__, 1);
    filein = argv[1];
    nx = atoi(argv[2]);
    ny = atoi(argv[3]);
    printer = (argc == 5) ? argv[4] : NULL;

    lept_stderr(
         "\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n"
         "   Warning: this program should only be used for testing,\n"
         "     and not in a production environment, because of a\n"
         "      potential vulnerability with the 'system' call.\n"
         "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n\n");

    setLeptDebugOK(1);
    lept_rmdir("lept/split");
    lept_mkdir("lept/split");

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);
    pixGetDimensions(pixs, &ws, &hs, NULL);
    if (ny * ws > nx * hs) {
        pixr = pixRotate90(pixs, 1);
        pixa = pixaSplitPix(pixr, ny, nx, 0, 0);
    } else {
        pixr = pixClone(pixs);
        pixa = pixaSplitPix(pixr, nx, ny, 0, 0);
    }
    pixDestroy(&pixr);

    n = pixaGetCount(pixa);
    sa = sarrayCreate(n);
    for (i = 0; i < n; i++) {
        pixt = pixaGetPix(pixa, i, L_CLONE);
        pixGetDimensions(pixt, &w, &h, NULL);
        scale = L_MIN(FILL_FACTOR * 2550 / w, FILL_FACTOR * 3300 / h);
        snprintf(buf, sizeof(buf), "image%d.ps", i);
        fname = genPathname("/tmp/lept/split", buf);
        lept_stderr("fname: %s\n", fname);
        sarrayAddString(sa, fname, L_INSERT);
#if USE_COMPRESSED
        index = 0;
        pixWriteCompressedToPS(pixt, fname, (l_int32)(300. / scale), 3, &index);
        index = 0;  /* write each out to a separate file */
#else  /* uncompressed, level 1 */
        fp = lept_fopen(fname, "wb+");
        pixWriteStreamPS(fp, pixt, NULL, 300, scale);
        lept_fclose(fp);
#endif  /* USE_COMPRESSED */
        pixDestroy(&pixt);
    }

    if (argc == 5) {
        for (i = 0; i < n; i++) {
            fname = sarrayGetString(sa, i, L_NOCOPY);
            snprintf(buf, sizeof(buf), "lpr -P%s %s &", printer, fname);
            callSystemDebug(buf);
        }
    }

    sarrayDestroy(&sa);
    pixaDestroy(&pixa);
    pixDestroy(&pixs);
    return 0;
}
