#ifndef KREUZBERG_RB_MSVC_COMPAT_STRINGS_H
#define KREUZBERG_RB_MSVC_COMPAT_STRINGS_H

#include <string.h>

#if !defined(strcasecmp) && defined(_stricmp)
#define strcasecmp _stricmp
#endif

#if !defined(strncasecmp) && defined(_strnicmp)
#define strncasecmp _strnicmp
#endif

#endif
