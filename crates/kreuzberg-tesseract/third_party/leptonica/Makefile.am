ACLOCAL_AMFLAGS = -I m4
AUTOMAKE_OPTIONS = foreign
EXTRA_DIST = config README.html leptonica-license.txt moller52.jpg version-notes.html make-for-auto make-for-local autogen.sh Doxyfile

pkgconfigdir = $(libdir)/pkgconfig
pkgconfig_DATA = lept.pc

SUBDIRS = src prog

# The fuzzing tests are run by OSS-Fuzz https://oss-fuzz.com/,
# but can also be run locally.
# Note: -fsanitize=fuzzer currently requires the clang++ compiler.

# LIB_FUZZING_ENGINE can be overridden by the caller.
# This is used by OSS-Fuzz.
LIB_FUZZING_ENGINE ?= -fsanitize=fuzzer

FUZZERS :=
FUZZERS += adaptmap_fuzzer
FUZZERS += affine_fuzzer
FUZZERS += barcode_fuzzer
FUZZERS += baseline_fuzzer
FUZZERS += bilateral_fuzzer
FUZZERS += binarize_fuzzer
FUZZERS += blend_fuzzer
FUZZERS += boxfunc_fuzzer
FUZZERS += boxfunc3_fuzzer
FUZZERS += boxfunc4_fuzzer
FUZZERS += boxfunc5_fuzzer
FUZZERS += ccbord_fuzzer
FUZZERS += checkerboard_fuzzer
FUZZERS += classapp_fuzzer
FUZZERS += colorfill_fuzzer
FUZZERS += colorquant_fuzzer
FUZZERS += compare_fuzzer
FUZZERS += dewarp_fuzzer
FUZZERS += edge_fuzzer
FUZZERS += enhance_fuzzer
FUZZERS += fhmtgen_fuzzer
FUZZERS += finditalic_fuzzer
FUZZERS += flipdetect_fuzzer
FUZZERS += fpix2_fuzzer
FUZZERS += graphics_fuzzer
FUZZERS += graymorph_fuzzer
FUZZERS += grayquant_fuzzer
FUZZERS += jpegiostub_fuzzer
FUZZERS += kernel_fuzzer
FUZZERS += mask_fuzzer
FUZZERS += maze_fuzzer
FUZZERS += morph_fuzzer
FUZZERS += morphapp_fuzzer
FUZZERS += pageseg_fuzzer
FUZZERS += paintcmap_fuzzer
FUZZERS += pix1_fuzzer
FUZZERS += pix3_fuzzer
FUZZERS += pix4_fuzzer
FUZZERS += pixa_recog_fuzzer
FUZZERS += pixconv_fuzzer
FUZZERS += pix_orient_fuzzer
FUZZERS += pix_rotate_shear_fuzzer
FUZZERS += recog_basic_fuzzer

fuzzers: $(FUZZERS)

$(FUZZERS): all

%_fuzzer: $(top_srcdir)/prog/fuzzing/%_fuzzer.cc
	$(CXX) $(CXXFLAGS) -g $(LIB_FUZZING_ENGINE) \
          -I $(top_srcdir)/src \
          -I $(builddir)/src \
          $< \
          $(builddir)/src/.libs/libleptonica.a \
          $(GIFLIB_LIBS) \
          $(JPEG_LIBS) \
          $(LIBJP2K_LIBS) \
          $(LIBPNG_LIBS) \
          $(LIBTIFF_LIBS) \
          $(LIBWEBP_LIBS) \
          -o $@
