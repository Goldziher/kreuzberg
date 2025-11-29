void build(Solution &s)
{
    auto add_deps = [](auto &t)
    {
        t += "HAVE_LIBGIF"_d;
        t += "HAVE_LIBJP2K"_d;
        t += "HAVE_LIBJPEG"_d;
        t += "HAVE_LIBPNG"_d;
        t += "HAVE_LIBTIFF"_d;
        t += "HAVE_LIBWEBP"_d;
        t += "HAVE_LIBWEBP_ANIM"_d;
        t += "HAVE_LIBZ"_d;

        t += "org.sw.demo.gif"_dep;
        t += "org.sw.demo.jpeg"_dep;
        t += "org.sw.demo.uclouvain.openjpeg.openjp2"_dep;
        t += "org.sw.demo.glennrp.png"_dep;
        t += "org.sw.demo.tiff"_dep;
        t += "org.sw.demo.webmproject.webp"_dep;
    };

    auto &leptonica = s.addTarget<LibraryTarget>("danbloomberg.leptonica", "1.87.0");
    leptonica += Git("https://github.com/DanBloomberg/leptonica");

    {
        leptonica.setChecks("leptonica");
        leptonica.ApiName = "LEPT_DLL";

        leptonica +=
            "src/.*\\.c"_rr,
            "src/.*\\.h"_rr,
            "src/endianness.h.in";

        leptonica.Public +=
            "src"_id;

        add_deps(leptonica);

        leptonica += "LIBJP2K_HEADER=\"openjpeg.h\""_d;
        leptonica.Public += "HAVE_CONFIG_H"_d;

        if (leptonica.Variables["WORDS_BIGENDIAN"] == 1)
            leptonica.Variables["ENDIANNESS"] = "L_BIG_ENDIAN";
        else
            leptonica.Variables["ENDIANNESS"] = "L_LITTLE_ENDIAN";

        leptonica.Variables["APPLE_UNIVERSAL_BUILD"] = "defined (__APPLE_CC__)";

        leptonica.configureFile("src/endianness.h.in", "endianness.h");
        leptonica.writeFileOnce("config_auto.h");

        if (leptonica.getBuildSettings().TargetOS.Type == OSType::Windows ||
            leptonica.getBuildSettings().TargetOS.Type == OSType::Mingw)
            leptonica += "User32.lib"_slib, "Gdi32.lib"_slib;
    }

    auto &progs = leptonica.addDirectory("progs");
    {
        auto add_prog = [&s, &progs, &leptonica, &add_deps](const String &name, const Files &files) -> decltype(auto)
        {
            auto &t = progs.addExecutable(name);
            t.Scope = TargetScope::Test;
            t.setRootDirectory("prog");
            t += files;
            t += leptonica;
            add_deps(t);
            return t;
        };

        StringMap<Files> m_progs{
            {"adaptmap_reg", {"adaptmap_reg.c"}},
            {"adaptnorm_reg", {"adaptnorm_reg.c"}},
            {"affine_reg", {"affine_reg.c"}},
            {"alltests_reg", {"alltests_reg.c"}},
            {"alphaops_reg", {"alphaops_reg.c"}},
            {"alphaxform_reg", {"alphaxform_reg.c"}},
            {"baseline_reg", {"baseline_reg.c"}},
            {"bilateral1_reg", {"bilateral1_reg.c"}},
            {"bilateral2_reg", {"bilateral2_reg.c"}},
            {"bilinear_reg", {"bilinear_reg.c"}},
            {"binarize_reg", {"binarize_reg.c"}},
            {"binmorph1_reg", {"binmorph1_reg.c"}},
            {"binmorph2_reg", {"binmorph2_reg.c"}},
            {"binmorph3_reg", {"binmorph3_reg.c"}},
            {"binmorph4_reg", {"binmorph4_reg.c"}},
            {"binmorph5_reg", {"binmorph5_reg.c"}},
            {"binmorph6_reg", {"binmorph6_reg.c"}},
            {"blackwhite_reg", {"blackwhite_reg.c"}},
            {"blend1_reg", {"blend1_reg.c"}},
            {"blend2_reg", {"blend2_reg.c"}},
            {"blend3_reg", {"blend3_reg.c"}},
            {"blend4_reg", {"blend4_reg.c"}},
            {"blend5_reg", {"blend5_reg.c"}},
            {"boxa1_reg", {"boxa1_reg.c"}},
            {"boxa2_reg", {"boxa2_reg.c"}},
            {"boxa3_reg", {"boxa3_reg.c"}},
            {"boxa4_reg", {"boxa4_reg.c"}},
            {"bytea_reg", {"bytea_reg.c"}},
            {"ccbord_reg", {"ccbord_reg.c"}},
            {"ccthin1_reg", {"ccthin1_reg.c"}},
            {"ccthin2_reg", {"ccthin2_reg.c"}},
            {"checkerboard_reg", {"checkerboard_reg.c"}},
            {"circle_reg", {"circle_reg.c"}},
            {"cmapquant_reg", {"cmapquant_reg.c"}},
            {"colorcontent_reg", {"colorcontent_reg.c"}},
            {"colorfill_reg", {"colorfill_reg.c"}},
            {"coloring_reg", {"coloring_reg.c"}},
            {"colorize_reg", {"colorize_reg.c"}},
            {"colormask_reg", {"colormask_reg.c"}},
            {"colormorph_reg", {"colormorph_reg.c"}},
            {"colorquant_reg", {"colorquant_reg.c"}},
            {"colorseg_reg", {"colorseg_reg.c"}},
            {"colorspace_reg", {"colorspace_reg.c"}},
            {"compare_reg", {"compare_reg.c"}},
            {"compfilter_reg", {"compfilter_reg.c"}},
            {"conncomp_reg", {"conncomp_reg.c"}},
            {"conversion_reg", {"conversion_reg.c"}},
            {"convolve_reg", {"convolve_reg.c"}},
            {"crop_reg", {"crop_reg.c"}},
            {"dewarp_reg", {"dewarp_reg.c"}},
            {"distance_reg", {"distance_reg.c"}},
            {"dither_reg", {"dither_reg.c"}},
            {"dna_reg", {"dna_reg.c"}},
            {"dwamorph1_reg", {"dwamorph1_reg.c", "dwalinear.3.c", "dwalinearlow.3.c"}},
            {"dwamorph2_reg", {"dwamorph2_reg.c", "dwalinear.3.c", "dwalinearlow.3.c"}},
            {"edge_reg", {"edge_reg.c"}},
            {"encoding_reg", {"encoding_reg.c"}},
            {"enhance_reg", {"enhance_reg.c"}},
            {"equal_reg", {"equal_reg.c"}},
            {"expand_reg", {"expand_reg.c"}},
            {"extrema_reg", {"extrema_reg.c"}},
            {"falsecolor_reg", {"falsecolor_reg.c"}},
            {"fhmtauto_reg", {"fhmtauto_reg.c"}},
            {"files_reg", {"files_reg.c"}},
            {"findcorners_reg", {"findcorners_reg.c"}},
            {"findpattern1_reg", {"findpattern1_reg.c"}},
            {"findpattern2_reg", {"findpattern2_reg.c"}},
            {"flipdetect_reg", {"flipdetect_reg.c"}},
            {"fmorphauto_reg", {"fmorphauto_reg.c"}},
            {"fpix1_reg", {"fpix1_reg.c"}},
            {"fpix2_reg", {"fpix2_reg.c"}},
            {"genfonts_reg", {"genfonts_reg.c"}},
            {"gifio_reg", {"gifio_reg.c"}},
            {"grayfill_reg", {"grayfill_reg.c"}},
            {"graymorph1_reg", {"graymorph1_reg.c"}},
            {"graymorph2_reg", {"graymorph2_reg.c"}},
            {"grayquant_reg", {"grayquant_reg.c"}},
            {"hardlight_reg", {"hardlight_reg.c"}},
            {"hash_reg", {"hash_reg.c"}},
            {"heap_reg", {"heap_reg.c"}},
            {"insert_reg", {"insert_reg.c"}},
            {"ioformats_reg", {"ioformats_reg.c"}},
            {"iomisc_reg", {"iomisc_reg.c"}},
            {"italic_reg", {"italic_reg.c"}},
            {"jbclass_reg", {"jbclass_reg.c"}},
            {"jp2kio_reg", {"jp2kio_reg.c"}},
            {"jpegio_reg", {"jpegio_reg.c"}},
            {"kernel_reg", {"kernel_reg.c"}},
            {"label_reg", {"label_reg.c"}},
            {"lineremoval_reg", {"lineremoval_reg.c"}},
            {"locminmax_reg", {"locminmax_reg.c"}},
            {"logicops_reg", {"logicops_reg.c"}},
            {"lowaccess_reg", {"lowaccess_reg.c"}},
            {"lowsat_reg", {"lowsat_reg.c"}},
            {"maze_reg", {"maze_reg.c"}},
            {"mtiff_reg", {"mtiff_reg.c"}},
            {"multitype_reg", {"multitype_reg.c"}},
            {"nearline_reg", {"nearline_reg.c"}},
            {"newspaper_reg", {"newspaper_reg.c"}},
            {"numa1_reg", {"numa1_reg.c"}},
            {"numa2_reg", {"numa2_reg.c"}},
            {"numa3_reg", {"numa3_reg.c"}},
            {"overlap_reg", {"overlap_reg.c"}},
            {"pageseg_reg", {"pageseg_reg.c"}},
            {"paintmask_reg", {"paintmask_reg.c"}},
            {"paint_reg", {"paint_reg.c"}},
            {"partition_reg", {"partition_reg.c"}},
            {"pdfio1_reg", {"pdfio1_reg.c"}},
            {"pdfio2_reg", {"pdfio2_reg.c"}},
            {"pdfseg_reg", {"pdfseg_reg.c"}},
            {"pixa1_reg", {"pixa1_reg.c"}},
            {"pixa2_reg", {"pixa2_reg.c"}},
            {"pixadisp_reg", {"pixadisp_reg.c"}},
            {"pixalloc_reg", {"pixalloc_reg.c"}},
            {"pixcomp_reg", {"pixcomp_reg.c"}},
            {"pixmem_reg", {"pixmem_reg.c"}},
            {"pixserial_reg", {"pixserial_reg.c"}},
            {"pixtile_reg", {"pixtile_reg.c"}},
            {"pngio_reg", {"pngio_reg.c"}},
            {"pnmio_reg", {"pnmio_reg.c"}},
            {"projection_reg", {"projection_reg.c"}},
            {"projective_reg", {"projective_reg.c"}},
            {"psioseg_reg", {"psioseg_reg.c"}},
            {"psio_reg", {"psio_reg.c"}},
            {"pta_reg", {"pta_reg.c"}},
            {"ptra1_reg", {"ptra1_reg.c"}},
            {"ptra2_reg", {"ptra2_reg.c"}},
            {"quadtree_reg", {"quadtree_reg.c"}},
            {"rankbin_reg", {"rankbin_reg.c"}},
            {"rankhisto_reg", {"rankhisto_reg.c"}},
            {"rank_reg", {"rank_reg.c"}},
            {"rasteropip_reg", {"rasteropip_reg.c"}},
            {"rasterop_reg", {"rasterop_reg.c"}},
            {"rectangle_reg", {"rectangle_reg.c"}},
            {"rotate1_reg", {"rotate1_reg.c"}},
            {"rotate2_reg", {"rotate2_reg.c"}},
            {"scale_reg", {"scale_reg.c"}},
            {"selio_reg", {"selio_reg.c"}},
            {"shear1_reg", {"shear1_reg.c"}},
            {"shear2_reg", {"shear2_reg.c"}},
            {"skew_reg", {"skew_reg.c"}},
            {"smallpix_reg", {"smallpix_reg.c"}},
            {"smoothedge_reg", {"smoothedge_reg.c"}},
            {"speckle_reg", {"speckle_reg.c"}},
            {"splitcomp_reg", {"splitcomp_reg.c"}},
            {"string_reg", {"string_reg.c"}},
            {"subpixel_reg", {"subpixel_reg.c"}},
            {"texturefill_reg", {"texturefill_reg.c"}},
            {"threshnorm_reg", {"threshnorm_reg.c"}},
            {"translate_reg", {"translate_reg.c"}},
            {"warper_reg", {"warper_reg.c"}},
            {"watershed_reg", {"watershed_reg.c"}},
            {"webpio_reg", {"webpio_reg.c"}},
            {"webpanimio_reg", {"webpanimio_reg.c"}},
            {"wordboxes_reg", {"wordboxes_reg.c"}},
            {"writetext_reg", {"writetext_reg.c"}},
            {"xformbox_reg", {"xformbox_reg.c"}},
            {"adaptmap_dark", {"adaptmap_dark.c"}},
            {"arabic_lines", {"arabic_lines.c"}},
            {"arithtest", {"arithtest.c"}},
            {"autogentest1", {"autogentest1.c"}},
            {"autogentest2", {"autogentest2.c", "autogen.137.c"}},
            {"barcodetest", {"barcodetest.c"}},
            {"binarize_set", {"binarize_set.c"}},
            {"binarizefiles", {"binarizefiles.c"}},
            {"bincompare", {"bincompare.c"}},
            {"blendcmaptest", {"blendcmaptest.c"}},
            {"buffertest", {"buffertest.c"}},
            {"ccbordtest", {"ccbordtest.c"}},
            {"cctest1", {"cctest1.c"}},
            {"cleanpdf", {"cleanpdf.c"}},
            {"colorsegtest", {"colorsegtest.c"}},
            {"comparepages", {"comparepages.c"}},
            {"comparepixa", {"comparepixa.c"}},
            {"comparetest", {"comparetest.c"}},
            {"compresspdf", {"compresspdf.c"}},
            {"contrasttest", {"contrasttest.c"}},
            {"convertfilestopdf", {"convertfilestopdf.c"}},
            {"convertfilestops", {"convertfilestops.c"}},
            {"convertformat", {"convertformat.c"}},
            {"convertsegfilestopdf", {"convertsegfilestopdf.c"}},
            {"convertsegfilestops", {"convertsegfilestops.c"}},
            {"converttogray", {"converttogray.c"}},
            {"converttopdf", {"converttopdf.c"}},
            {"converttops", {"converttops.c"}},
            {"cornertest", {"cornertest.c"}},
            {"corrupttest", {"corrupttest.c"}},
            {"croppdf", {"croppdf.c"}},
            {"croptext", {"croptext.c"}},
            {"deskew_it", {"deskew_it.c"}},
            {"dewarprules", {"dewarprules.c"}},
            {"dewarptest1", {"dewarptest1.c"}},
            {"dewarptest2", {"dewarptest2.c"}},
            {"dewarptest3", {"dewarptest3.c"}},
            {"dewarptest4", {"dewarptest4.c"}},
            {"dewarptest5", {"dewarptest5.c"}},
            {"digitprep1", {"digitprep1.c"}},
            {"displayboxa", {"displayboxa.c"}},
            {"displayboxes_on_pixa", {"displayboxes_on_pixa.c"}},
            {"displaypix", {"displaypix.c"}},
            {"displaypixa", {"displaypixa.c"}},
            {"dwalineargen", {"dwalineargen.c"}},
            {"fcombautogen", {"fcombautogen.c"}},
            {"fhmtautogen", {"fhmtautogen.c"}},
            {"fileinfo", {"fileinfo.c"}},
            {"findbinding", {"findbinding.c"}},
            {"find_colorregions", {"find_colorregions.c"}},
            {"findpattern1", {"findpattern1.c"}},
            {"findpattern2", {"findpattern2.c"}},
            {"findpattern3", {"findpattern3.c"}},
            {"fmorphautogen", {"fmorphautogen.c"}},
            {"fpixcontours", {"fpixcontours.c"}},
            {"gammatest", {"gammatest.c"}},
            {"graphicstest", {"graphicstest.c"}},
            {"graymorphtest", {"graymorphtest.c"}},
            {"hashtest", {"hashtest.c"}},
            {"histoduptest", {"histoduptest.c"}},
            {"histotest", {"histotest.c"}},
            {"htmlviewer", {"htmlviewer.c"}},
            {"imagetops", {"imagetops.c"}},
            {"jbcorrelation", {"jbcorrelation.c"}},
            {"jbrankhaus", {"jbrankhaus.c"}},
            {"jbwords", {"jbwords.c"}},
            {"lightcolortest", {"lightcolortest.c"}},
            {"listtest", {"listtest.c"}},
            {"livre_adapt", {"livre_adapt.c"}},
            {"livre_hmt", {"livre_hmt.c"}},
            {"livre_makefigs", {"livre_makefigs.c"}},
            {"livre_orient", {"livre_orient.c"}},
            {"livre_pageseg", {"livre_pageseg.c"}},
            {"livre_seedgen", {"livre_seedgen.c"}},
            {"livre_tophat", {"livre_tophat.c"}},
            {"maketile", {"maketile.c"}},
            {"maptest", {"maptest.c"}},
            {"messagetest", {"messagetest.c"}},
            {"misctest1", {"misctest1.c"}},
            {"misctest2", {"misctest2.c"}},
            {"modifyhuesat", {"modifyhuesat.c"}},
            {"morphseq_reg", {"morphseq_reg.c"}},
            {"morphtest1", {"morphtest1.c"}},
            {"numaranktest", {"numaranktest.c"}},
            {"otsutest1", {"otsutest1.c"}},
            {"otsutest2", {"otsutest2.c"}},
            {"pagesegtest1", {"pagesegtest1.c"}},
            {"pagesegtest2", {"pagesegtest2.c"}},
            {"partifytest", {"partifytest.c"}},
            {"partitiontest", {"partitiontest.c"}},
            {"percolatetest", {"percolatetest.c"}},
            {"pixaatest", {"pixaatest.c"}},
            {"pixafileinfo", {"pixafileinfo.c"}},
            {"plottest", {"plottest.c"}},
            {"printimage", {"printimage.c"}},
            {"printsplitimage", {"printsplitimage.c"}},
            {"printtiff", {"printtiff.c"}},
            {"rasteroptest", {"rasteroptest.c"}},
            {"rbtreetest", {"rbtreetest.c"}},
            {"recog_bootnum1", {"recog_bootnum1.c"}},
            {"recog_bootnum2", {"recog_bootnum2.c"}},
            {"recog_bootnum3", {"recog_bootnum3.c"}},
            {"recogsort", {"recogsort.c"}},
            {"recogtest1", {"recogtest1.c"}},
            {"recogtest2", {"recogtest2.c"}},
            {"recogtest3", {"recogtest3.c"}},
            {"recogtest4", {"recogtest4.c"}},
            {"recogtest5", {"recogtest5.c"}},
            {"recogtest6", {"recogtest6.c"}},
            {"recogtest7", {"recogtest7.c"}},
            {"reducetest", {"reducetest.c"}},
            {"removecmap", {"removecmap.c"}},
            {"renderfonts", {"renderfonts.c"}},
            {"replacebytes", {"replacebytes.c"}},
            {"rotatefastalt", {"rotatefastalt.c"}},
            {"rotate_it", {"rotate_it.c"}},
            {"rotateorthtest1", {"rotateorthtest1.c"}},
            {"rotateorth_reg", {"rotateorth_reg.c"}},
            {"rotatetest1", {"rotatetest1.c"}},
            {"runlengthtest", {"runlengthtest.c"}},
            {"scaleandtile", {"scaleandtile.c"}},
            {"scaleimages", {"scaleimages.c"}},
            {"scale_it", {"scale_it.c"}},
            {"scaletest1", {"scaletest1.c"}},
            {"scaletest2", {"scaletest2.c"}},
            {"seedfilltest", {"seedfilltest.c"}},
            {"seedspread_reg", {"seedspread_reg.c"}},
            {"settest", {"settest.c"}},
            {"sharptest", {"sharptest.c"}},
            {"sheartest", {"sheartest.c"}},
            {"showedges", {"showedges.c"}},
            {"skewtest", {"skewtest.c"}},
            {"sorttest", {"sorttest.c"}},
            {"splitimage2pdf", {"splitimage2pdf.c"}},
            {"splitpdf", {"splitpdf.c"}},
            {"sudokutest", {"sudokutest.c"}},
            {"textorient", {"textorient.c"}},
            {"tiffpdftest", {"tiffpdftest.c"}},
            {"trctest", {"trctest.c"}},
            {"underlinetest", {"underlinetest.c"}},
            {"warpertest", {"warpertest.c"}},
            {"wordsinorder", {"wordsinorder.c"}},
            {"writemtiff", {"writemtiff.c"}},
            {"xtractprotos", {"xtractprotos.c"}},
            {"yuvtest", {"yuvtest.c"}},
        };

        for (auto &[p, files] : m_progs)
            add_prog(p, files);
    }
}
#pragma optimize("", on)

void check(Checker &c)
{
    auto &s = c.addSet("leptonica");
    s.checkFunctionExists("fmemopen");
    s.checkFunctionExists("fstatat");
    s.checkFunctionExists("dirfd");
    s.checkIncludeExists("dlfcn.h");
    s.checkIncludeExists("inttypes.h");
    s.checkIncludeExists("memory.h");
    s.checkIncludeExists("stdint.h");
    s.checkIncludeExists("stdlib.h");
    s.checkIncludeExists("strings.h");
    s.checkIncludeExists("string.h");
    s.checkIncludeExists("sys/stat.h");
    s.checkIncludeExists("sys/types.h");
    s.checkIncludeExists("unistd.h");
    s.checkTypeSize("size_t");
    s.checkTypeSize("void *");
}
