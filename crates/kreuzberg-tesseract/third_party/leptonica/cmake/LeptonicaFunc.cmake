# cmake/LeptonicaFunc.cmake

# ##############################################################################
# FUNCTION find_and_handle_library
# ##############################################################################
function(find_and_handle_library PACKAGE_NAME ENABLE_OPTION PKG_CONFIG_NAME)
  if(${ENABLE_OPTION})
    find_package(${PACKAGE_NAME})
    if(${PACKAGE_NAME}_FOUND)
      set(pkgs_private
          "${pkgs_private} ${PKG_CONFIG_NAME}"
          PARENT_SCOPE)
      # Propagate found variables to parent scope
      set(${PACKAGE_NAME}_FOUND
          ${${PACKAGE_NAME}_FOUND}
          PARENT_SCOPE)
      if(DEFINED ${PACKAGE_NAME}_LIBRARIES)
        set(${PACKAGE_NAME}_LIBRARIES
            ${${PACKAGE_NAME}_LIBRARIES}
            PARENT_SCOPE)
      endif()
      if(DEFINED ${PACKAGE_NAME}_INCLUDE_DIRS)
        set(${PACKAGE_NAME}_INCLUDE_DIRS
            ${${PACKAGE_NAME}_INCLUDE_DIRS}
            PARENT_SCOPE)
      endif()
    else()
      if(STRICT_CONF)
        message(
          FATAL_ERROR
          "Could not find ${PACKAGE_NAME} libs. Use -D${ENABLE_OPTION}=OFF to disable ${PACKAGE_NAME} support."
        )
      endif()
    endif()
  endif()
endfunction()
