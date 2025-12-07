// SPDX-License-Identifier: MIT
#ifndef KREUZBERG_RUBY_UNISTD_H
#define KREUZBERG_RUBY_UNISTD_H

#ifdef _MSC_VER
#include <direct.h>
#include <io.h>
#include <process.h>
#include <stdlib.h>

#ifndef pid_t
typedef int pid_t;
#endif

#ifndef ssize_t
typedef long ssize_t;
#endif

#ifndef F_OK
#define F_OK 0
#endif
#ifndef X_OK
#define X_OK 0
#endif
#ifndef W_OK
#define W_OK 2
#endif
#ifndef R_OK
#define R_OK 4
#endif

#define access _access
#define dup2 _dup2
#define fileno _fileno
#define getpid _getpid
#define isatty _isatty
#define lseek _lseek
#define read _read
#define write _write
#define close _close
#define unlink _unlink

#else
#include_next <unistd.h>
#endif

#endif
