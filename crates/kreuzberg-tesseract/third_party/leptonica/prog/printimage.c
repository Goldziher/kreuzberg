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
 * printimage.c
 *
 *   This prints an image.  It rotates and isotropically scales the image,
 *   as necessary, to get a maximum filling when printing onto an
 *   8.5 x 11 inch page.
 *
 *        Syntax:  printimage <filein> [printer, other lpr args]
 *
 *   The simplest input would be something like
 *        printimage myfile.jpg
 *   This generates the PostScript file /tmp/print_image.ps, but
 *   does not send it to a printer.
 *
 *   If you have lpr, you can specify a printer; e.g.
 *        printimage myfile.jpg myprinter
 *
 *   You can add lpr flags.  Two useful ones are:
 *   * to print more than one copy
 *       -#N        (prints N copies)
 *   * to print in color (flag is printer-dependent)
 *       -o ColorModel=Color    or
 *       -o ColorModel=CMYK
 *
 *   For example, to make 3 color copies, you might use:
 *       printimage myfile.jpg myprinter -#3 -o ColorModel=Color
 *
 *   By default, the intermediate PostScript file generated is
 *   level 3 (compressed):
 *       /tmp/print_image.ps
 *
 *   If your system does not have lpr, it likely has lp.  You can run
 *   printimage to make the PostScript file, and then print with lp:
 *       lp -d <printer> /tmp/print_image.ps
 *       lp -d <printer> -o ColorModel=Color /tmp/print_image.ps
 *   etc.
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

static const l_float32  FILL_FACTOR = 0.95;   /* fill factor on 8.5 x 11 page */

int main(int    argc,
         char **argv)
{
char      *filein, *printer, *extra, *fname;
char       buffer[512];
l_int32    i, w, h, ret, index;
l_float32  scale;
FILE      *fp;
PIX       *pixs, *pix1;
SARRAY    *sa;

    if (argc < 2)
        return ERROR_INT(
            " Syntax:  printimage <filein> [printer, other lpr args]",
            __func__, 1);
    filein = argv[1];
    printer = (argc > 2) ? argv[2] : NULL;

    lept_stderr(
         "\n!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n"
         "   Warning: this program should only be used for testing,\n"
         "     and not in a production environment, because of a\n"
         "      potential vulnerability with the 'system' call.\n"
         "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!\n\n");

    setLeptDebugOK(1);
    (void)lept_rm(NULL, "print_image.ps");

    if ((pixs = pixRead(filein)) == NULL)
        return ERROR_INT("pixs not made", __func__, 1);

    pixGetDimensions(pixs, &w, &h, NULL);
    if (w > h) {
        pix1 = pixRotate90(pixs, 1);
        pixGetDimensions(pix1, &w, &h, NULL);
    }
    else {
        pix1 = pixClone(pixs);
    }
    scale = L_MIN(FILL_FACTOR * 2550 / w, FILL_FACTOR * 3300 / h);
    fname = genPathname("/tmp", "print_image.ps");
#if USE_COMPRESSED
    index = 0;
    pixWriteCompressedToPS(pix1, fname, (l_int32)(300. / scale), 3, &index);
#else  /* uncompressed, level 1 */
    fp = lept_fopen(fname, "wb+");
    pixWriteStreamPS(fp, pix1, NULL, 300, scale);
    lept_fclose(fp);
#endif  /* USE_COMPRESSED */

        /* Optionally print it out */
    if (argc > 2) {
        extra = NULL;
        if (argc > 3) {  /* concatenate the extra args */
            sa = sarrayCreate(0);
            for (i = 3; i < argc; i++)
                sarrayAddString(sa, argv[i], L_COPY);
            extra = sarrayToString(sa, 2);
            sarrayDestroy(&sa);
        }
        if (!extra) {
            snprintf(buffer, sizeof(buffer), "lpr %s -P%s &", fname, printer);
            ret = callSystemDebug(buffer);
        } else {
            snprintf(buffer, sizeof(buffer), "lpr %s -P%s %s &",
                     fname, printer, extra);
            ret = callSystemDebug(buffer);
        }
        lept_free(extra);
    }

    lept_free(fname);
    pixDestroy(&pixs);
    pixDestroy(&pix1);
    return 0;
}
