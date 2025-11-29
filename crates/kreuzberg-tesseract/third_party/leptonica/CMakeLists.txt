#
# leptonica
#

# ##############################################################################
#
# cmake settings
#
# ##############################################################################

cmake_minimum_required(VERSION 3.10)
cmake_policy(SET CMP0054 NEW)

# In-source builds are disabled.
if("${CMAKE_CURRENT_SOURCE_DIR}" STREQUAL "${CMAKE_CURRENT_BINARY_DIR}")
  message(
    FATAL_ERROR
      "CMake generation is not possible within the source directory!"
      "\n Remove the CMakeCache.txt file and try again from another folder"
      ", e.g.:\n "
      "\n rm CMakeCache.txt"
      "\n mkdir build"
      "\n cd build"
      "\n cmake ..")
endif()

set(CMAKE_MODULE_PATH "${CMAKE_MODULE_PATH};${CMAKE_CURRENT_SOURCE_DIR}/cmake")

set(EXECUTABLE_OUTPUT_PATH "${CMAKE_CURRENT_BINARY_DIR}/bin")
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY "${EXECUTABLE_OUTPUT_PATH}")

# Use solution folders.
set_property(GLOBAL PROPERTY USE_FOLDERS ON)
set_property(GLOBAL PROPERTY PREDEFINED_TARGETS_FOLDER "CMake Targets")

# ##############################################################################
#
# project settings
#
# ##############################################################################

project(
  leptonica
  LANGUAGES C
  VERSION 1.87.0)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

# Standard installation paths
include(GNUInstallDirs)

set(MINIMUM_WEBPMUX_VERSION 0.5.0)

option(BUILD_PROG "Build utility programs" OFF)
if(WIN32)
  option(SW_BUILD "Build with sw" ON)
else()
  option(SW_BUILD "Build with sw" OFF)
endif()
if(UNIX OR MINGW)
  option(SYM_LINK "Create symlink leptonica -> lept on UNIX" OFF)
endif()
option(STRICT_CONF
       "Strict configuration - fail when the optional library is not found" OFF)
option(ENABLE_ZLIB "Enable ZLIB support" ON)
option(ENABLE_PNG "Enable PNG support" ON)
option(ENABLE_GIF "Enable GIF support" ON)
option(ENABLE_JPEG "Enable JPEG support" ON)
option(ENABLE_TIFF "Enable TIFF support" ON)
option(ENABLE_WEBP "Enable WebP support" ON)
option(ENABLE_OPENJPEG "Enable OpenJPEG support" ON)

set(leptonica_INSTALL_CMAKE_DIR
    "${CMAKE_INSTALL_LIBDIR}/cmake/leptonica"
    CACHE STRING "Install destination for CMake package files")

include(LeptonicaFunc)

if(NOT SW_BUILD)
  find_package(PkgConfig QUIET)
  set(pkgs_private "")
  set(libs_private "")

  find_and_handle_library(ZLIB ENABLE_ZLIB zlib)
  find_and_handle_library(PNG ENABLE_PNG libpng)
  find_and_handle_library(JPEG ENABLE_JPEG libjpeg)
  find_and_handle_library(TIFF ENABLE_TIFF libtiff-4)

  if(ENABLE_GIF)
    find_package(GIF 5)
    if(GIF_FOUND)
      set(libs_private "${libs_private} -lgif")
    else()
      if(STRICT_CONF)
        message(
          FATAL_ERROR
            "Could not find GIF 5 libs. Use -DENABLE_GIF=OFF to disable GIF support."
        )
      endif()
    endif()
  endif()

  if(ENABLE_WEBP)
    find_package(WebP ${MINIMUM_WEBPMUX_VERSION} CONFIG)
    set(pkgs_private "${pkgs_private} libwebp libwebpmux")
    if(NOT TARGET WebP::webp)
      find_path(WebP_INCLUDE_DIR NAMES webp/decode.h)
      find_path(WebP_MUX_INCLUDE_DIR NAMES webp/mux.h)
      find_library(WebP_LIBRARY NAMES webp)
      find_library(WebP_MUX_LIBRARY NAMES webpmux)
      if(WebP_INCLUDE_DIR
         AND WebP_MUX_INCLUDE_DIR
         AND WebP_LIBRARY)
        message(STATUS "Found non-cmake WebP: ${WebP_LIBRARY}")
        set(WebP_INCLUDE_DIR "${WebP_INCLUDE_DIR};${WebP_MUX_INCLUDE_DIR}")
        set(WebP_LIBRARIES "${WebP_LIBRARY}")
        if(WebP_MUX_LIBRARY)
          list(APPEND WebP_LIBRARIES "${WebP_MUX_LIBRARY}")
        else()
          message(STATUS "Library webpmux not found!")
        endif()
        set(WebP_FOUND 1)
      endif()
    endif()
    if(WebP_FOUND)
      set(HAVE_LIBWEBP 1)
      set(HAVE_LIBWEBP_ANIM 1)
      include_directories(${WebP_INCLUDE_DIR})
      include_directories(${WebP_MUX_INCLUDE_DIR})
      message(STATUS "Found WebP: ${WebP_LIBRARIES} (${WebP_VERSION})")
    else()
      if(STRICT_CONF)
        message(
          FATAL_ERROR
            "Could not find WebP libs. Use -DENABLE_WEBP=OFF to disable WebP support."
        )
      endif()
    endif()
  endif()

  if(ENABLE_OPENJPEG)
    find_package(OpenJPEG CONFIG)
    if(OpenJPEG_FOUND)
      set(OPENJPEG_VERSION
          "${OPENJPEG_MAJOR_VERSION}.${OPENJPEG_MINOR_VERSION}")
      if(${OPENJPEG_VERSION} VERSION_LESS "2.1")
        message(STATUS "Your libopenjp version is too old ${OPENJPEG_VERSION}")
        set(HAVE_LIBJP2K 0)
      else()
        set(pkgs_private "${pkgs_private} libopenjp2")
        set(HAVE_LIBJP2K 1)
        include_directories(${OPENJPEG_INCLUDE_DIRS})
        if(WIN32)
          if(BUILD_SHARED_LIBS)
            add_definitions(-DOPJ_EXPORTS)
          else()
            add_definitions(-DOPJ_STATIC)
          endif()
        endif()
        message(
          STATUS
            "Found OpenJPEG: ${OPENJPEG_LIBRARIES} (${OPENJPEG_VERSION}.${OPENJPEG_BUILD_VERSION})"
        )
      endif()
    else()
      if(STRICT_CONF)
        message(
          FATAL_ERROR
            "Could not find OpenJPEG libs. Use -DENABLE_OPENJPEG=OFF to disable OpenJPEG support."
        )
      endif()
    endif()
  endif()
else()
  find_package(SW REQUIRED)
  sw_add_package(
    org.sw.demo.gif org.sw.demo.jpeg org.sw.demo.glennrp.png org.sw.demo.tiff
    org.sw.demo.webmproject.webp org.sw.demo.uclouvain.openjpeg.openjp2)
  sw_execute()

  add_definitions(
    -DHAVE_LIBGIF=1
    -DHAVE_LIBJPEG=1
    -DHAVE_LIBPNG=1
    -DHAVE_LIBTIFF=1
    -DHAVE_LIBWEBP=1
    -DHAVE_LIBWEBP_ANIM=1
    -DHAVE_LIBZ=1
    -DHAVE_LIBJP2K=1)
endif()

include(CheckLibraryExists)
check_library_exists(m sin "" HAVE_LIBM)
if(HAVE_LIBM)
  set(libs_private "${libs_private} -lm")
endif(HAVE_LIBM)

if(${CMAKE_SYSTEM_NAME} MATCHES "kFreeBSD.*|DragonFly.*|FreeBSD")
  set(FREEBSD ON)
endif(${CMAKE_SYSTEM_NAME} MATCHES "kFreeBSD.*|DragonFly.*|FreeBSD")

# ##############################################################################
#
# compiler and linker
#
# ##############################################################################

if(CMAKE_C_COMPILER_ID MATCHES "Clang")
  set(CLANG 1)
endif()

if(NOT CMAKE_BUILD_TYPE)
  message(STATUS "Setting build type to 'Release' as none was specified.")
  set(CMAKE_BUILD_TYPE
      Release
      CACHE STRING "Choose the type of build." FORCE)
  set_property(CACHE CMAKE_BUILD_TYPE PROPERTY STRINGS "Debug" "Release")
else()
  message("CMAKE_BUILD_TYPE = ${CMAKE_BUILD_TYPE}")
endif()

include(CheckCCompilerFlag)

macro(CHECK_CCXX_COMPILER_FLAG flag_ result_)
  check_c_compiler_flag(${flag_} C_HAS${result_})
endmacro()

if(MSVC)
  add_definitions(-D_CRT_SECURE_NO_WARNINGS)
  add_compile_options("$<$<CONFIG:Release>:/W1;/Ox>")
  add_compile_options("$<$<CONFIG:Debug>:/W4;/DDEBUG>")
  if(NOT CLANG)
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} /MP")
  endif()
else()
  if(CMAKE_BUILD_TYPE MATCHES "[Dd][Ee][Bb][Uu][Gg]")
    set(debug_flags -Wall -Wextra -Werror=format-security -pedantic -Og -g3)
    list(APPEND debug_flags
         "-fdebug-prefix-map=${CMAKE_SOURCE_DIR}=${CMAKE_BINARY_DIR}")

    foreach(flag_ IN LISTS debug_flags)
      string(REGEX REPLACE "[^A-Za-z0-9]" "_" flag_var "${flag_}")
      check_c_compiler_flag(${flag_} ${flag_var})
      if(C_HAS${flag_var})
        set(CMAKE_C_FLAGS_DEBUG "${CMAKE_C_FLAGS_DEBUG} ${flag_}")
      endif()
    endforeach()
    add_definitions(-DDEBUG)
  endif()
endif()

# ##############################################################################
#
# configure
#
# ##############################################################################

set(AUTOCONFIG_SRC ${CMAKE_CURRENT_BINARY_DIR}/config_auto.h.in)
set(AUTOCONFIG ${CMAKE_CURRENT_BINARY_DIR}/src/config_auto.h)
if(HAVE_LIBJP2K)
  set(ADDITIONAL_INCLUDE_DIRS "${JP2K_INCLUDE_DIR}")
endif()
set(CMAKE_REQUIRED_INCLUDES
    ${CMAKE_REQUIRED_INCLUDES} "${CMAKE_PREFIX_PATH}/include"
    "${CMAKE_INSTALL_PREFIX}/include" "${ADDITIONAL_INCLUDE_DIRS}")

include(Configure)

configure_file(${AUTOCONFIG_SRC} ${AUTOCONFIG} @ONLY)

set(INCLUDE_DIR "${CMAKE_INSTALL_PREFIX}/include"
                "${CMAKE_INSTALL_PREFIX}/include/leptonica")

# ##############################################################################
#
# Show summary
#
# ##############################################################################
message(STATUS)
message(STATUS "General configuration for Leptonica ${PROJECT_VERSION}")
message(STATUS "--------------------------------------------------------")
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "Compiler: ${CMAKE_C_COMPILER_ID}")
message(
  STATUS
    "C compiler options: ${CMAKE_C_FLAGS} ${CMAKE_C_FLAGS_${CMAKE_BUILD_TYPE}}")
message(
  STATUS
    "Linker options: ${CMAKE_EXE_LINKER_FLAGS} ${CMAKE_EXE_LINKER_FLAGS_${CMAKE_BUILD_TYPE}}"
)
message(STATUS "Install directory: ${CMAKE_INSTALL_PREFIX}")
message(STATUS)
message(STATUS "Build with sw [SW_BUILD]: ${SW_BUILD}")
message(STATUS "Build utility programs [BUILD_PROG]: ${BUILD_PROG}")
if(NOT SW_BUILD)
  message(STATUS "Used ZLIB library: ${ZLIB_LIBRARIES}")
  message(STATUS "Used PNG library:  ${PNG_LIBRARIES}")
  message(STATUS "Used JPEG library: ${JPEG_LIBRARIES}")
  message(STATUS "Used OpenJPEG library: ${OPENJPEG_LIBRARIES}")
  message(STATUS "Used TIFF library: ${TIFF_LIBRARIES}")
  message(STATUS "Used GIF library:  ${GIF_LIBRARIES}")
  message(STATUS "Used WebP library: ${WebP_LIBRARIES}")
endif()
message(STATUS "--------------------------------------------------------")
message(STATUS)

# ##############################################################################
#
# build
#
# ##############################################################################

add_definitions(-DHAVE_CONFIG_H)

include_directories(${CMAKE_CURRENT_SOURCE_DIR}/src)
include_directories(${CMAKE_CURRENT_BINARY_DIR}/src)

if(BUILD_PROG)
  add_subdirectory(prog)
endif()

add_subdirectory(src)

get_target_property(leptonica_NAME leptonica NAME)
get_target_property(leptonica_OUTPUT_NAME leptonica OUTPUT_NAME)

configure_file(lept.pc.cmake ${CMAKE_CURRENT_BINARY_DIR}/lept.pc.in @ONLY)
# to resolve generator expression in OUTPUT_NAME
set(pkg_conf_name lept.pc)
if(WIN32)
  set(pkg_conf_name lept_$<CONFIG>.pc)
endif(WIN32)

file(
  GENERATE
  OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/${pkg_conf_name}
  INPUT ${CMAKE_CURRENT_BINARY_DIR}/lept.pc.in)

configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}/cmake/templates/LeptonicaConfig-version.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/LeptonicaConfig-version.cmake @ONLY)
configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}/cmake/templates/LeptonicaConfig.cmake.in
  ${CMAKE_CURRENT_BINARY_DIR}/LeptonicaConfig.cmake @ONLY)

install(FILES ${CMAKE_CURRENT_BINARY_DIR}/${pkg_conf_name}
        DESTINATION ${CMAKE_INSTALL_LIBDIR}/pkgconfig)
install(FILES ${CMAKE_CURRENT_BINARY_DIR}/LeptonicaConfig.cmake
              ${CMAKE_CURRENT_BINARY_DIR}/LeptonicaConfig-version.cmake
        DESTINATION ${leptonica_INSTALL_CMAKE_DIR})

# ##############################################################################
# uninstall target
# ##############################################################################
if(NOT TARGET uninstall)
  configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/templates/cmake_uninstall.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/cmake_uninstall.cmake" IMMEDIATE @ONLY)

  add_custom_target(
    uninstall COMMAND ${CMAKE_COMMAND} -P
                      ${CMAKE_CURRENT_BINARY_DIR}/cmake_uninstall.cmake)
endif()
# ##############################################################################
