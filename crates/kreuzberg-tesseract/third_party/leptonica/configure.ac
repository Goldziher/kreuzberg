AC_PREREQ([2.69])
AC_INIT([leptonica], [1.87.0])
AC_CONFIG_AUX_DIR([config])
AC_CONFIG_HEADERS([config_auto.h:config/config.h.in])
AC_CONFIG_SRCDIR([src/adaptmap.c])

AC_CONFIG_MACRO_DIR([m4])
LT_INIT

AM_INIT_AUTOMAKE
AC_LANG(C)
AC_CANONICAL_HOST

# Checks for programs.
AC_PROG_AWK
AC_PROG_CC
AC_PROG_CPP
AC_PROG_INSTALL
AC_PROG_LN_S
AC_PROG_MAKE_SET

# Checks for arguments.
AC_ARG_WITH([zlib], AS_HELP_STRING([--without-zlib], [do not include zlib support]))
AC_ARG_WITH([libpng], AS_HELP_STRING([--without-libpng], [do not include libpng support]))
AC_ARG_WITH([jpeg], AS_HELP_STRING([--without-jpeg], [do not include jpeg support]))
AC_ARG_WITH([giflib], AS_HELP_STRING([--without-giflib], [do not include giflib support]))
AC_ARG_WITH([libtiff], AS_HELP_STRING([--without-libtiff], [do not include libtiff support]))
AC_ARG_WITH([libwebp], AS_HELP_STRING([--without-libwebp], [do not include libwebp support]))
AC_ARG_WITH([libwebpmux], AS_HELP_STRING([--without-libwebpmux], [do not include libwebpmux support]))
AC_ARG_WITH([libopenjpeg], AS_HELP_STRING([--without-libopenjpeg], [do not include libopenjpeg support]))

AC_ARG_ENABLE([programs], AS_HELP_STRING([--disable-programs], [do not build additional programs]))
AM_CONDITIONAL([ENABLE_PROGRAMS], [test "x$enable_programs" != xno])

# Checks for libraries.
LT_LIB_M

AS_IF([test "x$with_zlib" = xno], [
  zlib_missing=untried
], [
  zlib_missing=

  PKG_CHECK_MODULES([ZLIB], [zlib], [
    AC_DEFINE([HAVE_LIBZ], 1, [Define to 1 if you have zlib.])
  ], [
    AC_CHECK_LIB([z], [deflate], [:], [zlib_missing=library])
    AC_CHECK_HEADER([zlib.h], [:], [zlib_missing=header])

    AS_IF([test "x$zlib_missing" = x], [
      AC_DEFINE([HAVE_LIBZ], 1, [Define to 1 if you have zlib.]) AC_SUBST([ZLIB_LIBS], [-lz])
    ], [
      AS_IF([test "x$with_zlib" = xyes], AC_MSG_ERROR([zlib support requested but $zlib_missing not found]))
    ])
  ])
])

AS_IF([test "x$with_libpng" = xno], [
  libpng_missing=untried
], [
  libpng_missing=

  PKG_CHECK_MODULES([LIBPNG], [libpng], [
    AC_DEFINE([HAVE_LIBPNG], 1, [Define to 1 if you have libpng.])
  ], [
    AC_CHECK_LIB([png], [png_read_png], [:], [libpng_missing=library], [$LIBM $ZLIB_LIBS])
    AC_CHECK_HEADER([png.h], [:], [libpng_missing=headers])

    AS_IF([test "x$libpng_missing" = x], [
      AC_DEFINE([HAVE_LIBPNG], 1, [Define to 1 if you have libpng.]) AC_SUBST([LIBPNG_LIBS], [-lpng])
    ], [
      AS_IF([test "x$with_libpng" = xyes], AC_MSG_ERROR([libpng support requested but $libpng_missing not found]))
    ])
  ])
])

AS_IF([test "x$with_jpeg" = xno], [
  jpeg_missing=untried
], [
  jpeg_missing=

  PKG_CHECK_MODULES([JPEG], [libjpeg], [
    AC_DEFINE([HAVE_LIBJPEG], 1, [Define to 1 if you have jpeg.])
  ], [
    AC_CHECK_LIB([jpeg], [jpeg_read_scanlines], [:], [jpeg_missing=library])
    AC_CHECK_HEADER([jpeglib.h], [:], [jpeg_missing=headers])

    AS_IF([test "x$jpeg_missing" = x], [
      AC_DEFINE([HAVE_LIBJPEG], 1, [Define to 1 if you have jpeg.]) AC_SUBST([JPEG_LIBS], [-ljpeg])
    ], [
      AS_IF([test "x$with_jpeg" = xyes], AC_MSG_ERROR([jpeg support requested but $jpeg_missing not found]))
    ])
  ])
])

AS_IF([test "x$with_giflib" = xno], [
  giflib_missing=untried
], [
  giflib_missing=

  AC_CHECK_LIB([gif], [DGifOpenFileHandle], [:], [giflib_missing=library])
  AC_CHECK_HEADER([gif_lib.h], [:], [giflib_missing=header])

  AS_IF([test "x$giflib_missing" = x], [
    AC_MSG_CHECKING([giflib is at least version 5.1 (but not 5.1.2)])
    AC_PREPROC_IFELSE([
      AC_LANG_SOURCE([[
#include <gif_lib.h>
#if !defined GIFLIB_MAJOR || GIFLIB_MAJOR < 5 || (GIFLIB_MAJOR == 5 && GIFLIB_MINOR == 0)
#error giflib version too old
#elif GIFLIB_MAJOR == 5 && GIFLIB_MINOR == 1 && GIFLIB_RELEASE == 2
#error giflib 5.1.2 is broken
#endif
      ]])
    ], [
      AC_MSG_RESULT([yes])
      AC_DEFINE([HAVE_LIBGIF], 1, [Define to 1 if you have giflib.]) AC_SUBST([GIFLIB_LIBS], [-lgif])
    ], [
      AC_MSG_RESULT([no])
      AS_IF([test "x$with_giflib" = xyes], AC_MSG_ERROR([giflib version too old or broken]))
      giflib_missing=old
    ])
  ], [
    AS_IF([test "x$with_giflib" = xyes], AC_MSG_ERROR([giflib support requested but $giflib_missing not found]))
  ])
])

AM_CONDITIONAL([HAVE_LIBGIF], [test "x$giflib_missing" = x])

AS_IF([test "x$with_libtiff" = xno], [
  libtiff_missing=untried
], [
  libtiff_missing=

  PKG_CHECK_MODULES([LIBTIFF], [libtiff-4], [
    AC_DEFINE([HAVE_LIBTIFF], 1, [Define to 1 if you have libtiff.])
  ], [
    AC_CHECK_LIB([tiff], [TIFFOpen], [:], [libtiff_missing=library], [$LIBM $ZLIB_LIBS $JPEG_LIBS])
    AC_CHECK_HEADER([tiff.h], [:], [libtiff_missing=headers])

    AS_IF([test "x$libtiff_missing" = x], [
      AC_DEFINE([HAVE_LIBTIFF], 1, [Define to 1 if you have libtiff.]) AC_SUBST([LIBTIFF_LIBS], [-ltiff])
    ], [
      AS_IF([test "x$with_libtiff" = xyes], AC_MSG_ERROR([libtiff support requested but $libtiff_missing not found]))
    ])
  ])
])

AS_IF([test "x$with_libwebp" = xno], [
  libwebp_missing=untried
], [
  libwebp_missing=

  PKG_CHECK_MODULES([LIBWEBP], [libwebp], [
    AC_DEFINE([HAVE_LIBWEBP], 1, [Define to 1 if you have libwebp.])
  ], [
    AC_CHECK_LIB([webp], [WebPGetInfo], [:], [libwebp_missing=library], [$LIBM])
    AC_CHECK_HEADER([webp/encode.h], [:], [libwebp_missing=headers])

    AS_IF([test "x$libwebp_missing" = x], [
      AC_DEFINE([HAVE_LIBWEBP], 1, [Define to 1 if you have libwebp.]) AC_SUBST([LIBWEBP_LIBS], [-lwebp])
    ], [
      AS_IF([test "x$with_libwebp" = xyes], AC_MSG_ERROR([libwebp support requested but $libwebp_missing not found]))
    ])
  ])
])

AM_CONDITIONAL([HAVE_LIBWEBP], [test "x$libwebp_missing" = x])

AS_IF([test "x$with_libwebpmux" = xno], [
  libwebpmux_missing=untried
], [
  libwebpmux_missing=

  PKG_CHECK_MODULES([LIBWEBPMUX], [libwebpmux >= 0.5.0], [
    AC_DEFINE([HAVE_LIBWEBP_ANIM], 1, [Define to 1 if you have libwebpmux.])
  ], [
    AC_CHECK_LIB([webpmux], [WebPAnimEncoderOptionsInit], [:], [libwebpmux_missing=library], [$LIBM])

    AS_IF([test "x$libwebpmux_missing" = x], [
      AC_DEFINE([HAVE_LIBWEBP_ANIM], 1, [Define to 1 if you have libwebpmux.]) AC_SUBST([LIBWEBPMUX_LIBS], [-lwebpmux])
    ], [
      AS_IF([test "x$with_libwebpmux" = xyes], AC_MSG_ERROR([libwebpmux support requested but $libwebpmux_missing not found]))
    ])
  ])
])

AM_CONDITIONAL([HAVE_LIBWEBP_ANIM], [test "x$libwebpmux_missing" = x])

AS_IF([test "x$with_libopenjpeg" = xno], [
  libopenjpeg_missing=untried
], [
  libopenjpeg_missing=

  PKG_CHECK_MODULES([LIBJP2K], [libopenjp2 >= 2.0.0], [
    AC_DEFINE([HAVE_LIBJP2K], 1, [Define to 1 if you have libopenjp2.])
  ], [
    AC_CHECK_LIB([openjp2], [opj_create_decompress], [:], [libopenjpeg_missing=library])
    AC_CHECK_HEADERS([openjpeg-2.3/openjpeg.h openjpeg-2.2/openjpeg.h openjpeg-2.1/openjpeg.h openjpeg-2.0/openjpeg.h],
      [AC_DEFINE_UNQUOTED([LIBJP2K_HEADER], [<$ac_header>], [Path to <openjpeg.h> header file.]) break],
      [libopenjpeg_missing=headers]
    )

    AS_IF([test "x$libopenjpeg_missing" = x], [
      AC_DEFINE([HAVE_LIBJP2K], 1, [Define to 1 if you have libopenjp2.]) AC_SUBST([LIBJP2K_LIBS], [-lopenjp2])
    ], [
      AS_IF([test "x$with_libopenjpeg" = xyes], AC_MSG_ERROR([libopenjp2 support requested but $libopenjpeg_missing not found]))
    ])
  ])
])

AM_CONDITIONAL([HAVE_LIBJP2K], [test "x$libopenjpeg_missing" = x])

# Check whether to enable debugging
AC_MSG_CHECKING([whether to enable debugging])
AC_ARG_ENABLE([debug],
  AS_HELP_STRING([--enable-debug], [turn on debugging [default=no]]))
AC_MSG_RESULT([$enable_debug])
if test x"$enable_debug" = x"yes"; then
    CFLAGS="-g -Wall -DDEBUG -pedantic -Og -g3"
fi

case "$host_os" in
  mingw32*)
  AC_SUBST([GDI_LIBS], [-lgdi32])
  CPPFLAGS="${CPPFLAGS} -D__USE_MINGW_ANSI_STDIO" ;;
esac

# Enable less verbose output when building.
m4_ifdef([AM_SILENT_RULES], [AM_SILENT_RULES([yes])])

# Checks for typedefs, structures, and compiler characteristics.
AC_TYPE_SIZE_T
AC_C_BIGENDIAN

AC_SUBST([APPLE_UNIVERSAL_BUILD], [0])
AC_SUBST([ENDIANNESS], [L_LITTLE_ENDIAN])

case "$ac_cv_c_bigendian" in
  yes) AC_SUBST([ENDIANNESS], [L_BIG_ENDIAN]) ;;
  universal) AC_SUBST([APPLE_UNIVERSAL_BUILD], [1]) ;;
esac

# Add the -Wl and --as-needed flags to gcc compiler
AC_MSG_CHECKING([whether compiler supports -Wl,--as-needed])
OLD_LDFLAGS="${LDFLAGS}"
LDFLAGS="-Wl,--as-needed ${LDFLAGS}"

AC_LINK_IFELSE([AC_LANG_PROGRAM()],
  AC_MSG_RESULT([yes]),
  LDFLAGS="${OLD_LDFLAGS}"; AC_MSG_RESULT([no])
)

# Checks for library functions.
AC_CHECK_FUNCS([fmemopen])
AC_CHECK_FUNC([fstatat])
AC_CHECK_FUNC([dirfd])

# Configuration files
AC_CONFIG_FILES([Makefile src/endianness.h src/Makefile prog/Makefile lept.pc])

# cmake configuration. Get versions from PACKAGE_VERSION.
AX_SPLIT_VERSION
AC_SUBST([VERSION_PLAIN], ["${PACKAGE_VERSION}"])
AC_SUBST([VERSION_MAJOR], [$(echo "$AX_MAJOR_VERSION" | $SED 's/^[[^0-9]]*//')])
AC_SUBST([VERSION_MINOR], [$AX_MINOR_VERSION])
AC_SUBST([VERSION_PATCH], [$(echo "$AX_POINT_VERSION" | $SED 's/^\([[0-9]][[0-9]]*\).*/\1/')])
INCLUDE_PATH=`eval echo $includedir`
AC_SUBST([INCLUDE_DIR], ["${INCLUDE_PATH}/leptonica"])
AC_SUBST([leptonica_OUTPUT_NAME], ["leptonica"])

# Create symlink
AC_PROG_LN_S

AC_OUTPUT
