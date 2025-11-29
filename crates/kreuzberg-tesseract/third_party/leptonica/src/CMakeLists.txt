#
# leptonica
#

# ##############################################################################

# ##############################################################################
# SHARED LIBRARY leptonica
# ##############################################################################

file(GLOB hdr "*.h")
file(GLOB src "*.c")

string(REPLACE "-O3" "-O2" CMAKE_C_FLAGS_RELEASE "${CMAKE_C_FLAGS_RELEASE}")

add_library(leptonica ${src} ${hdr})
set_target_properties(
  leptonica
  PROPERTIES
    VERSION 6.0.0
    SOVERSION 6
    OUTPUT_NAME
    "leptonica$<$<BOOL:${MSVC}>:-${PROJECT_VERSION}$<$<CONFIG:DEBUG>:d>>")

if(MINGW)
  set_target_properties(
    leptonica PROPERTIES SUFFIX
                         "-${PROJECT_VERSION}${CMAKE_SHARED_LIBRARY_SUFFIX}")
endif(MINGW)

if(BUILD_SHARED_LIBS)
  target_compile_definitions(leptonica PRIVATE -DLIBLEPT_EXPORTS)
endif()

if(GIF_LIBRARIES)
  target_include_directories(leptonica PRIVATE ${GIF_INCLUDE_DIR})
  target_link_libraries(leptonica PRIVATE ${GIF_LIBRARIES})
endif()
if(JPEG_LIBRARIES)
  target_include_directories(leptonica PRIVATE ${JPEG_INCLUDE_DIR})
  target_link_libraries(leptonica PRIVATE ${JPEG_LIBRARIES})
endif()
if(OPENJPEG_LIBRARIES)
  # openjp2 < 2.5.0 does not sets the INTERFACE_INCLUDE_DIRECTORIES property
  target_include_directories(leptonica PRIVATE ${OPENJPEG_INCLUDE_DIRS})
  target_link_libraries(leptonica PRIVATE openjp2)
endif()
if(PNG_LIBRARIES)
  target_include_directories(leptonica PRIVATE ${PNG_INCLUDE_DIRS})
  target_link_libraries(leptonica PRIVATE ${PNG_LIBRARIES})
endif()
if(TIFF_LIBRARIES)
  target_include_directories(leptonica PRIVATE ${TIFF_INCLUDE_DIR})
  target_link_libraries(leptonica PRIVATE ${TIFF_LIBRARIES})
endif()
if(WebP_LIBRARIES)
  if(TARGET WebP::webp)
    target_link_libraries(leptonica PRIVATE WebP::webp WebP::libwebpmux)
  else()
    target_link_libraries(leptonica PRIVATE ${WebP_LIBRARIES})
    target_include_directories(leptonica PRIVATE ${WebP_INCLUDE_DIR})
  endif()
endif()
if(ZLIB_LIBRARIES)
  target_include_directories(leptonica PRIVATE ${ZLIB_INCLUDE_DIRS})
  target_link_libraries(leptonica PRIVATE ${ZLIB_LIBRARIES})
endif()

if(UNIX)
  target_link_libraries(leptonica PRIVATE m)
  set(lib ${CMAKE_SHARED_LIBRARY_PREFIX})
  set(dll ${CMAKE_SHARED_LIBRARY_SUFFIX})
  set(old_target ${lib}lept${dll})
  if(SYM_LINK)
    add_custom_target(
      lept ALL
      ${CMAKE_COMMAND} -E create_symlink "$<TARGET_FILE:leptonica>"
      ${old_target}
      DEPENDS ${PROJECT_NAME}
      COMMENT "Creating lept symlink")
    install(FILES ${CMAKE_BINARY_DIR}/src/${old_target} DESTINATION ${CMAKE_INSTALL_LIBDIR})
  endif(SYM_LINK)
endif()

if(MSVC)
  target_link_libraries(leptonica PRIVATE user32.lib gdi32.lib)
endif()

if(NOT SW_BUILD)
  export(TARGETS leptonica FILE ${CMAKE_BINARY_DIR}/LeptonicaTargets.cmake)
else()
  target_link_libraries(
    leptonica
    PRIVATE org.sw.demo.gif org.sw.demo.jpeg org.sw.demo.glennrp.png
            org.sw.demo.tiff org.sw.demo.webmproject.webp
            org.sw.demo.uclouvain.openjpeg.openjp2)
  file(WRITE ${CMAKE_BINARY_DIR}/LeptonicaTargets.cmake
       "include(${CMAKE_BINARY_DIR}/cppan.cmake)\n")
  export(
    TARGETS leptonica
    APPEND
    FILE ${CMAKE_BINARY_DIR}/LeptonicaTargets.cmake)
endif()

install(
  TARGETS leptonica
  EXPORT LeptonicaTargets
  RUNTIME DESTINATION ${CMAKE_INSTALL_BINDIR}
  LIBRARY DESTINATION ${CMAKE_INSTALL_LIBDIR}
  ARCHIVE DESTINATION ${CMAKE_INSTALL_LIBDIR})
install(EXPORT LeptonicaTargets DESTINATION ${leptonica_INSTALL_CMAKE_DIR})
install(FILES ${hdr} ${CMAKE_BINARY_DIR}/src/endianness.h
        DESTINATION include/leptonica)

# ##############################################################################
