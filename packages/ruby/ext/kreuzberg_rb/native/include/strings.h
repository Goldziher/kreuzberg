// SPDX-License-Identifier: MIT
#ifndef KREUZBERG_RUBY_STRINGS_H
#define KREUZBERG_RUBY_STRINGS_H

#include <string.h>

#ifdef _MSC_VER
#include <ctype.h>
#ifndef strcasecmp
#define strcasecmp _stricmp
#endif
#ifndef strncasecmp
#define strncasecmp _strnicmp
#endif
#ifndef bzero
#define bzero(ptr, size) memset((ptr), 0, (size))
#endif
#endif

#endif
