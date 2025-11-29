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
 * genfonts_reg.c
 *
 *    This regtest is for the generation of bitmap font characters that
 *    are presently used for annotating images.
 *
 *    The tiff images of bitmaps fonts, which are used as input
 *    to this generator, are supplied in Leptonica in the prog/fonts
 *    directory.  The tiff images were generated from the PostScript files
 *    in that directory, using the shell script prog/ps2tiff.  If you
 *    want to generate other fonts, modify the PostScript files
 *    and use ps2tiff.  ps2tiff uses GhostScript.
 *
 *    The input tiff images are stored either as files in prog/fonts/,
 *    or as compiled C strings in bmfdata.h.  Each image stores 94 of the
 *    95 printable characters, all in one of 9 sizes (ranging from 4 to 20
 *    points).  These are programmatically split into individual characters,
 *    and the baselines are computed for each character.  Baselines are
 *    required to properly render them.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include "bmfdata.h"

#define   NFONTS        9

const l_int32 sizes[] = {4, 6, 8, 10, 12, 14, 16, 18, 20};

int main(int    argc,
         char **argv)
{
char          buf[512];
char         *pathname, *datastr, *formstr;
l_uint8      *data1, *data2;
l_int32       i, bl1, bl2, bl3, sbytes, formbytes, fontsize, rbytes;
size_t        nbytes;
PIX          *pix1, *pix2, *pixd;
PIXA         *pixa;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "genfonts_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

    /* ------------  Generate pixa char bitmap files from file ----------- */
    lept_rmdir("lept/filefonts");
    lept_mkdir("lept/filefonts");
    for (i = 0; i < 9; i++) {
        pixaSaveFont("fonts", "/tmp/lept/filefonts", sizes[i]);
        pathname = pathJoin("/tmp/lept/filefonts", outputfonts[i]);
        pixa = pixaRead(pathname);
        if (rp->display) {
            lept_stderr("Found %d chars in font size %d\n",
                        pixaGetCount(pixa), sizes[i]);
        }
        pixd = pixaDisplayTiled(pixa, 1500, 0, 15);
        regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 0 - 8 */
        if (i == 2) pixDisplayWithTitle(pixd, 100, 0, NULL, rp->display);
        pixDestroy(&pixd);
        pixaDestroy(&pixa);
        lept_free(pathname);
    }
    lept_rmdir("lept/filefonts");

    /* ----------  Generate pixa char bitmap files from string --------- */
    lept_rmdir("lept/strfonts");
    lept_mkdir("lept/strfonts");
    for (i = 0; i < 9; i++) {
        pixaSaveFont(NULL, "/tmp/lept/strfonts", sizes[i]);
        pathname = pathJoin("/tmp/lept/strfonts", outputfonts[i]);
        pixa = pixaRead(pathname);
        if (rp->display) {
            lept_stderr("Found %d chars in font size %d\n",
                        pixaGetCount(pixa), sizes[i]);
        }
        pixd = pixaDisplayTiled(pixa, 1500, 0, 15);
        regTestWritePixAndCheck(rp, pixd, IFF_PNG);  /* 9 - 17 */
        if (i == 2) pixDisplayWithTitle(pixd, 100, 150, NULL, rp->display);
        pixDestroy(&pixd);
        pixaDestroy(&pixa);
        lept_free(pathname);
    }

    /* -----  Use pixaGetFont() and write the result out  -----*/
    lept_rmdir("lept/pafonts");
    lept_mkdir("lept/pafonts");
    for (i = 0; i < 9; i++) {
        pixa = pixaGetFont("/tmp/lept/strfonts", sizes[i], &bl1, &bl2, &bl3);
        lept_stderr("Baselines are at: %d, %d, %d\n", bl1, bl2, bl3);
        snprintf(buf, sizeof(buf), "/tmp/lept/pafonts/chars-%d.pa", sizes[i]);
        pixaWrite(buf, pixa);
        if (i == 2) {
            pixd = pixaDisplayTiled(pixa, 1500, 0, 15);
            pixDisplayWithTitle(pixd, 100, 300, NULL, rp->display);
            pixDestroy(&pixd);
        }
        pixaDestroy(&pixa);
    }
    lept_rmdir("lept/pafonts");

    /* -------  Generate 4/3 encoded ascii strings from tiff files ------ */
    lept_rmdir("lept/encfonts");
    lept_mkdir("lept/encfonts");
    for (i = 0; i < 9; i++) {
        fontsize = 2 * i + 4;
        pathname = pathJoin("fonts", inputfonts[i]);
        data1 = l_binaryRead(pathname, &nbytes);
        datastr = encodeBase64(data1, nbytes, &sbytes);
        if (rp->display)
            lept_stderr("nbytes = %lu, sbytes = %d\n",
                        (unsigned long)nbytes, sbytes);
        formstr = reformatPacked64(datastr, sbytes, 4, 72, 1, &formbytes);
        snprintf(buf, sizeof(buf), "/tmp/lept/encfonts/formstr_%d.txt",
                 fontsize);
        l_binaryWrite(buf, "w", formstr, formbytes);
        regTestCheckFile(rp, buf);  /* 18-26 */
        if (i == 8)
            pix1 = pixReadMem(data1, nbytes);  /* original */
        lept_free(data1);

        data2 = decodeBase64(datastr, sbytes, &rbytes);
        snprintf(buf, sizeof(buf), "/tmp/lept/encfonts/image_%d.tif", fontsize);
        l_binaryWrite(buf, "w", data2, rbytes);
        if (i == 8) {
            pix2 = pixReadMem(data2, rbytes);  /* encode/decode */
            regTestComparePix(rp, pix1, pix2);  /* 27 */
            pixDestroy(&pix1);
            pixDestroy(&pix2);
        }
        lept_free(data2);

        lept_free(pathname);
        lept_free(datastr);
        lept_free(formstr);
    }

    return regTestCleanup(rp);
}

