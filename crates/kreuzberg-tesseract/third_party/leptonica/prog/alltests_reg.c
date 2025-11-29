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
 *  alltests_reg.c
 *
 *    Tests all the reg tests:
 *
 *        alltests_reg command
 *
 *    where
 *        <command> == "generate" to make the golden files in /tmp/golden
 *        <command> == "compare" to make local files and compare with
 *                     the golden files
 *        <command> == "display" to make local files and display
 *
 *    You can also run each test individually with any one of these
 *    arguments.  Warning: if you run this with "display", a very
 *    large number of images will be displayed on the screen.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif /* HAVE_CONFIG_H */

#include <string.h>
#include "allheaders.h"

static const char *tests[] = {
                              "adaptmap_reg",
                              "adaptnorm_reg",
                              "affine_reg",
                              "alphaops_reg",
                              "alphaxform_reg",
                              "baseline_reg",
                              "bilateral2_reg",
                              "bilinear_reg",
                              "binarize_reg",
                              "binmorph1_reg",
                              "binmorph3_reg",
                              "binmorph6_reg",
                              "blackwhite_reg",
                              "blend1_reg",
                              "blend2_reg",
                              "blend3_reg",
                              "blend4_reg",
                              "blend5_reg",
                              "boxa1_reg",
                              "boxa2_reg",
                              "boxa3_reg",
                              "boxa4_reg",
                              "bytea_reg",
                              "ccbord_reg",
                              "ccthin1_reg",
                              "ccthin2_reg",
                              "checkerboard_reg",
                              "circle_reg",
                              "cmapquant_reg",
                              "colorcontent_reg",
                              "colorfill_reg",
                              "coloring_reg",
                              "colorize_reg",
                              "colormask_reg",
                              "colormorph_reg",
                              "colorquant_reg",
                              "colorseg_reg",
                              "colorspace_reg",
                              "compare_reg",
                              "compfilter_reg",
                              "conncomp_reg",
                              "conversion_reg",
                              "convolve_reg",
                              "crop_reg",
                              "dewarp_reg",
                              "distance_reg",
                              "dither_reg",
                              "dna_reg",
                              "dwamorph1_reg",
                              "edge_reg",
                              "encoding_reg",
                              "enhance_reg",
                              "equal_reg",
                              "expand_reg",
                              "extrema_reg",
                              "falsecolor_reg",
                              "fhmtauto_reg",
                         /*   "files_reg",  */
                              "findcorners_reg",
                              "findpattern1_reg",
                              "findpattern2_reg",
                              "flipdetect_reg",
                              "fpix1_reg",
                              "fpix2_reg",
                              "genfonts_reg",
#if HAVE_LIBGIF
                              "gifio_reg",
#endif  /* HAVE_LIBGIF */
                              "grayfill_reg",
                              "graymorph1_reg",
                              "graymorph2_reg",
                              "grayquant_reg",
                              "hardlight_reg",
                              "hash_reg",
                              "heap_reg",
                              "insert_reg",
                              "ioformats_reg",
                              "iomisc_reg",
                              "italic_reg",
                              "jbclass_reg",
#if HAVE_LIBJP2K
                              "jp2kio_reg",
#endif  /* HAVE_LIBJP2K */
                              "jpegio_reg",
                              "kernel_reg",
                              "label_reg",
                              "lineremoval_reg",
                              "locminmax_reg",
                              "logicops_reg",
                              "lowaccess_reg",
                              "lowsat_reg",
                              "maze_reg",
                              "mtiff_reg",
                              "multitype_reg",
                              "numa1_reg",
                              "numa2_reg",
                              "numa3_reg",
                              "nearline_reg",
                              "newspaper_reg",
                              "overlap_reg",
                              "pageseg_reg",
                              "paint_reg",
                              "paintmask_reg",
                              "partition_reg",
                              "pdfio1_reg",
                              "pdfio2_reg",
                              "pdfseg_reg",
                              "pixa1_reg",
                              "pixa2_reg",
                              "pixadisp_reg",
                              "pixcomp_reg",
                              "pixmem_reg",
                              "pixserial_reg",
                              "pngio_reg",
                              "pnmio_reg",
                              "projection_reg",
                              "projective_reg",
                              "psio_reg",
                              "psioseg_reg",
                              "pta_reg",
                              "ptra1_reg",
                              "ptra2_reg",
                              "quadtree_reg",
                              "rank_reg",
                              "rankbin_reg",
                              "rankhisto_reg",
                              "rasterop_reg",
                              "rasteropip_reg",
                              "rectangle_reg",
                              "rotate1_reg",
                              "rotate2_reg",
                              "rotateorth_reg",
                              "scale_reg",
                              "seedspread_reg",
                              "selio_reg",
                              "shear1_reg",
                              "shear2_reg",
                              "skew_reg",
                              "smallpix_reg",
                              "speckle_reg",
                              "splitcomp_reg",
                              "string_reg",
                              "subpixel_reg",
                              "texturefill_reg",
                              "threshnorm_reg",
                              "translate_reg",
                              "warper_reg",
                              "watershed_reg",
#if HAVE_LIBWEBP_ANIM
                              "webpanimio_reg",
#endif  /* HAVE_LIBWEBP_ANIM */
#if HAVE_LIBWEBP
                              "webpio_reg",
#endif  /* HAVE_LIBWEBP */
                              "wordboxes_reg",
                              "writetext_reg",
                              "xformbox_reg",
                             };

static const char *header = {"\n=======================\n"
                             "Regression Test Results\n"
                             "======================="};

int main(int    argc,
         char **argv)
{
char    *str, *results_file;
char     command[256], buf[256];
l_int32  i, ntests, dotest, nfail, ret, start, stop;
SARRAY  *sa;

    if (argc != 2)
        return ERROR_INT(" Syntax alltests_reg [generate | compare | display]",
                         __func__, 1);

    setLeptDebugOK(1);  /* required for testing */
    l_getCurrentTime(&start, NULL);
    ntests = sizeof(tests) / sizeof(char *);
    lept_stderr("Running alltests_reg:\n"
            "This currently tests %d regression test\n"
            "programs in the /prog directory.\n", ntests);

        /* Clear the output file if we're doing the set of reg tests */
    dotest = strcmp(argv[1], "compare") ? 0 : 1;
    if (dotest) {
        results_file = genPathname("/tmp/lept", "reg_results.txt");
        sa = sarrayCreate(3);
        sarrayAddString(sa, header, L_COPY);
        sarrayAddString(sa, getLeptonicaVersion(), L_INSERT);
        sarrayAddString(sa, getImagelibVersions(), L_INSERT);
        str = sarrayToString(sa, 1);
        sarrayDestroy(&sa);
        l_binaryWrite("/tmp/lept/reg_results.txt", "w", str, strlen(str));
        lept_free(str);
    }

    nfail = 0;
    for (i = 0; i < ntests; i++) {
#ifndef  _WIN32
        snprintf(command, sizeof(command) - 2, "./%s %s", tests[i], argv[1]);
#else  /* windows interprets '/' as a commandline flag */
        snprintf(command, sizeof(command) - 2, "%s %s", tests[i], argv[1]);
#endif  /* ! _WIN32 */
        ret = callSystemDebug(command);
        if (ret) {
            snprintf(buf, sizeof(buf), "Failed to complete %s\n", tests[i]);
            if (dotest) {
                l_binaryWrite("/tmp/lept/reg_results.txt", "a",
                              buf, strlen(buf));
                nfail++;
            }
            else
                lept_stderr("%s", buf);
        }
    }

    if (dotest) {
#ifndef _WIN32
        snprintf(command, sizeof(command) - 2, "cat %s", results_file);
#else
        snprintf(command, sizeof(command) - 2, "type \"%s\"", results_file);
#endif  /* !_WIN32 */
        lept_free(results_file);
        ret = system(command);
        lept_stderr("Success in %d of %d *_reg programs (output matches"
                " the \"golden\" files)\n", ntests - nfail, ntests);
    }

    l_getCurrentTime(&stop, NULL);
    lept_stderr("Time for all regression tests: %d sec\n", stop - start);
    return 0;
}
