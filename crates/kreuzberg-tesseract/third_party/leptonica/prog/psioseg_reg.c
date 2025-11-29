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
 * psioseg_reg.c
 *
 *   This tests the PostScript output for images with mixed
 *   text and images, coming from source of different depths,
 *   with and without colormaps.
 *
 *   Both convertFilesFittedToPS() and convertSegmentedPagesToPS()
 *   generate a compressed PostScript file from a subset of images in
 *   a directory.  However, the latter function can also accept 1 bpp
 *   masks that delineate image (as opposed to text) regions in
 *   the corresponding page image file.  Then, for page images that
 *   are not 1 bpp, it generates mixed raster PostScript with
 *   g4 encoding for the text and jpeg ("DCT") encoding for the
 *   maining image parts.
 *
 *   N.B. Although not required for 'success' on the regression test,
 *        this program uses ps2pdf to generate the pdf output.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

int main(int    argc,
         char **argv)
{
char          buf[512];
char         *psname, *pdfname;
l_int32       w, h, wc, hc, ret;
l_float32     scalefactor;
PIX          *pixs, *pixc, *pixht, *pixtxt, *pixmfull;
PIX          *pix4c, *pix8c, *pix8g, *pix32, *pixcs, *pixcs2;
L_REGPARAMS  *rp;

#if !defined(HAVE_LIBPNG)
    L_ERROR("This test requires libpng to run.\n", "psioseg_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBJPEG)
    L_ERROR("This test requires libjpeg to run.\n", "psioseg_reg");
    exit(77);
#endif
#if !defined(HAVE_LIBTIFF)
    L_ERROR("This test requires libtiff to run.\n", "psioseg_reg");
    exit(77);
#endif

    if (regTestSetup(argc, argv, &rp))
        return 1;

#if !USE_PSIO
    lept_stderr("psio writing is not enabled\n"
                "See environ.h: #define USE_PSIO 1\n\n");
    regTestCleanup(rp);
    return 0;
#endif  /* abort */

        /* Source for generating images */
    pixs = pixRead("pageseg2.tif");   /* 1 bpp */
    pixc = pixRead("tetons.jpg");     /* 32 bpp */

        /* Get a halftone segmentation mask for pixs */
    pixGetRegionsBinary(pixs, &pixht, NULL, NULL, 0);
    pixtxt = pixSubtract(NULL, pixs, pixht);

        /* Construct a 32 bpp image in full page size, along with
         * a mask that can be used to render it. */
    pixGetDimensions(pixs, &w, &h, NULL);
    pixGetDimensions(pixc, &wc, NULL, NULL);
    scalefactor = (l_float32)w / (l_float32)wc;
    pixcs = pixScale(pixc, scalefactor, scalefactor);
    pixGetDimensions(pixcs, &wc, &hc, NULL);
    pixcs2 = pixCreate(w, h, 32);
    pixRasterop(pixcs2, 0, 0, w, hc, PIX_SRC, pixcs, 0, 0);
    pixRasterop(pixcs2, 0, hc, w, hc, PIX_SRC, pixcs, 0, 0);
    regTestWritePixAndCheck(rp, pixcs2, IFF_JFIF_JPEG);  /* 0 */
    pixmfull = pixCreate(w, h, 1);
    pixSetAll(pixmfull);  /* use as mask to render the color image */

         /* Now make a 32 bpp input image, taking text parts from the
          * page image and image parts from pixcs2. */
    pix32 = pixConvertTo32(pixtxt);
    pixCombineMasked(pix32, pixcs2, pixht);
    regTestWritePixAndCheck(rp, pix32, IFF_JFIF_JPEG);  /* 1 */

         /* Make an 8 bpp gray version */
    pix8g = pixConvertRGBToLuminance(pix32);
    regTestWritePixAndCheck(rp, pix8g, IFF_JFIF_JPEG);  /* 2 */

         /* Make an 8 bpp colormapped version */
    pix8c = pixOctreeColorQuant(pix32, 240, 0);
    regTestWritePixAndCheck(rp, pix8c, IFF_PNG);  /* 3 */

         /* Make a 4 bpp colormapped version */
    pix4c = pixOctreeQuantNumColors(pix32, 16, 4);
    regTestWritePixAndCheck(rp, pix4c, IFF_PNG);  /* 4 */

         /* Write out the files to be imaged */
    lept_mkdir("lept/psio");
    pixWrite("/tmp/lept/psio/image_001.tif", pixs, IFF_TIFF_G4);
    pixWrite("/tmp/lept/psio/image_002.tif", pixht, IFF_TIFF_G4);
    pixWrite("/tmp/lept/psio/image_003.tif", pixtxt, IFF_TIFF_G4);
    pixWrite("/tmp/lept/psio/image_004.jpg", pixcs2, IFF_JFIF_JPEG);
    pixWrite("/tmp/lept/psio/mask_004.tif", pixmfull, IFF_TIFF_G4);
    pixWrite("/tmp/lept/psio/image_005.jpg", pix32, IFF_JFIF_JPEG);
    pixWrite("/tmp/lept/psio/mask_005.tif", pixht, IFF_TIFF_G4);
    pixWrite("/tmp/lept/psio/image_006.jpg", pix8g, IFF_JFIF_JPEG);
    pixWrite("/tmp/lept/psio/mask_006.tif", pixht, IFF_TIFF_G4);
    pixWrite("/tmp/lept/psio/image_007.png", pix8c, IFF_PNG);
    pixWrite("/tmp/lept/psio/mask_007.tif", pixht, IFF_TIFF_G4);
    pixWrite("/tmp/lept/psio/image_008.png", pix4c, IFF_PNG);
    pixWrite("/tmp/lept/psio/mask_008.tif", pixht, IFF_TIFF_G4);
    pixDestroy(&pixs);
    pixDestroy(&pixc);
    pixDestroy(&pixht);
    pixDestroy(&pixtxt);
    pixDestroy(&pixcs);
    pixDestroy(&pixcs2);
    pixDestroy(&pixmfull);
    pixDestroy(&pix32);
    pixDestroy(&pix8g);
    pixDestroy(&pix8c);
    pixDestroy(&pix4c);

        /* Generate the 8 page ps */
    convertSegmentedPagesToPS("/tmp/lept/psio", "image_", 6, "/tmp/lept/psio",
                              "mask_", 5, 0, 10, 2.0, 0.15, 190,
                              "/tmp/lept/regout/psioseg.5.ps");
    regTestCheckFile(rp, "/tmp/lept/regout/psioseg.5.ps");  /* 5 */
    L_INFO("Output ps: /tmp/lept/regout/psioseg.5.ps\n", rp->testname);

        /* For convenience, also generate a pdf of this, using ps2pdf */
    psname = genPathname("/tmp/lept/regout", "psioseg.5.ps");
    pdfname = genPathname("/tmp/lept/regout", "psioseg.5.pdf");
    snprintf(buf, sizeof(buf), "ps2pdf %s %s", psname, pdfname);
    ret = callSystemDebug(buf);  /* ps2pdf */
    lept_free(psname);
    lept_free(pdfname);
    if (!ret)
        L_INFO("Output pdf: /tmp/lept/regout/psioseg.5.pdf\n", rp->testname);
    else
        L_WARNING("ps2pdf failed to generate pdf\n", rp->testname);

    return regTestCleanup(rp);
}
