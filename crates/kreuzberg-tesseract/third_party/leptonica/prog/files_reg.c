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
 *   files_reg.c
 *
 *    Regression test for lept_*() and other path utilities in utils.h
 *
 *    Some of these only work properly on unix because they explicitly
 *    use "/tmp" for string compares.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"
#include <string.h>
#ifndef _MSC_VER
#include <unistd.h>
#else
#include <direct.h>
#define getcwd _getcwd  /* fix MSVC warning */
#endif  /* !_MSC_VER */

void TestPathJoin(L_REGPARAMS *rp, const char *first, const char *second,
                  const char *result);
void TestLeptCpRm(L_REGPARAMS *rp, const char *srctail, const char *newdir,
                  const char *newtail);
void TestGenPathname(L_REGPARAMS *rp, const char *dir, const char *fname,
                     const char *result);

l_int32 main(int    argc,
             char **argv)
{
l_int32       exists;
L_REGPARAMS  *rp;

    if (regTestSetup(argc, argv, &rp))
        return 1;

    lept_stderr(" ===================================================\n");
    lept_stderr(" =================== Test pathJoin() ===============\n");
    lept_stderr(" ===================================================\n");
    TestPathJoin(rp, "/a/b//c///d//", "//e//f//g//", "/a/b/c/d/e/f/g");  /* 0 */
    TestPathJoin(rp, "/tmp/", "junk//", "/tmp/junk");  /* 1 */
    TestPathJoin(rp, "//tmp/", "junk//", "/tmp/junk");  /* 2 */
    TestPathJoin(rp, "tmp/", "//junk//", "tmp/junk");  /* 3 */
    TestPathJoin(rp, "tmp/", "junk/////", "tmp/junk");  /* 4 */
    TestPathJoin(rp, "/tmp/", "///", "/tmp");  /* 5 */
    TestPathJoin(rp, "////", NULL, "/");  /* 6 */
    TestPathJoin(rp, "//", "/junk//", "/junk");  /* 7 */
    TestPathJoin(rp, NULL, "/junk//", "/junk");  /* 8 */
    TestPathJoin(rp, NULL, "//junk//", "/junk");  /* 9 */
    TestPathJoin(rp, NULL, "junk//", "junk");  /* 10 */
    TestPathJoin(rp, NULL, "//", "/");  /* 11 */
    TestPathJoin(rp, NULL, NULL, "");  /* 12 */
    TestPathJoin(rp, "", "", "");  /* 13 */
    TestPathJoin(rp, "/", "", "/");  /* 14 */
    TestPathJoin(rp, "", "//", "/");  /* 15 */
    TestPathJoin(rp, "", "a", "a");  /* 16 */

    lept_stderr("The next 3 joins properly give error messages:\n");
    lept_stderr("join: .. + a --> NULL\n");
    pathJoin("..", "a");  /* returns NULL */
    lept_stderr("join: %s + .. --> NULL\n", "/tmp");
    pathJoin("/tmp", "..");  /* returns NULL */
    lept_stderr("join: ./ + .. --> NULL\n");
    pathJoin("./", "..");  /* returns NULL */

    lept_stderr("\n ===================================================\n");
    lept_stderr(" ======= Test lept_rmdir() and lept_mkdir()) =======\n");
    lept_stderr(" ===================================================\n");
    lept_rmdir("junkfiles");
    lept_direxists("/tmp/junkfiles", &exists);
    if (rp->display) lept_stderr("directory removed?: %d\n", !exists);
    regTestCompareValues(rp, 0, exists, 0.0);  /* 17 */

    lept_mkdir("junkfiles");
    lept_direxists("/tmp/junkfiles", &exists);
    if (rp->display) lept_stderr("directory made?: %d\n", exists);
    regTestCompareValues(rp, 1, exists, 0.0);  /* 18 */

    lept_stderr("\n ===================================================\n");
    lept_stderr(" ======= Test lept_mv(), lept_cp(), lept_rm() ======\n");
    lept_stderr(" ===================================================\n");
    TestLeptCpRm(rp, "weasel2.png", NULL, NULL);  /* 19 - 22 */
    TestLeptCpRm(rp, "weasel2.png", "junkfiles", NULL);  /* 23 - 26 */
    TestLeptCpRm(rp, "weasel2.png", NULL, "new_weasel2.png");  /* 27 - 30 */
    TestLeptCpRm(rp, "weasel2.png", "junkfiles", "new_weasel2.png"); /* 31-34 */

    lept_stderr("\n ===================================================\n");
    lept_stderr(" =============== Test genPathname() ================\n");
    lept_stderr(" ===================================================\n");
    TestGenPathname(rp, "what/", NULL, "what");  /* 35 */
    TestGenPathname(rp, "what", "abc", "what/abc");  /* 36 */
    TestGenPathname(rp, NULL, "abc/def", "abc/def");  /* 37 */
    TestGenPathname(rp, "", "abc/def", "abc/def");  /* 38 */
#ifndef _WIN32   /* unix only */
    if (getenv("TMPDIR") == NULL) {
        TestGenPathname(rp, "/tmp", NULL, "/tmp");  /* 39 */
        TestGenPathname(rp, "/tmp/", NULL, "/tmp");  /* 40 */
        TestGenPathname(rp, "/tmp/junk", NULL, "/tmp/junk");  /* 41 */
        TestGenPathname(rp, "/tmp/junk/abc", NULL, "/tmp/junk/abc");  /* 42 */
        TestGenPathname(rp, "/tmp/junk/", NULL, "/tmp/junk");  /* 43 */
        TestGenPathname(rp, "/tmp/junk", "abc", "/tmp/junk/abc");  /* 44 */
    }
#endif  /* !_WIN32 */

    return regTestCleanup(rp);
}


void TestPathJoin(L_REGPARAMS  *rp,
                  const char   *first,
                  const char   *second,
                  const char   *result)
{
char  *newfirst = NULL;
char  *newsecond = NULL;
char  *newpath = NULL;
char  *path = NULL;

    if ((path = pathJoin(first, second)) == NULL) return;
    regTestCompareStrings(rp, (l_uint8 *)result, strlen(result),
                          (l_uint8 *)path, strlen(path));

    if (first && first[0] == '\0')
        newfirst = stringNew("\"\"");
    else if (first)
        newfirst = stringNew(first);
    if (second && second[0] == '\0')
        newsecond = stringNew("\"\"");
    else if (second)
        newsecond = stringNew(second);
    if (path && path[0] == '\0')
        newpath = stringNew("\"\"");
    else if (path)
        newpath = stringNew(path);
    if (rp->display)
        lept_stderr("join: %s + %s --> %s\n", newfirst, newsecond, newpath);
    lept_free(path);
    lept_free(newfirst);
    lept_free(newsecond);
    lept_free(newpath);
    return;
}

void TestLeptCpRm(L_REGPARAMS  *rp,
                  const char   *srctail,
                  const char   *newdir,
                  const char   *newtail)
{
char     realnewdir[256], newnewdir[256];
char    *realtail, *newsrc, *fname;
l_int32  nfiles1, nfiles2, nfiles3;
SARRAY  *sa;

        /* Remove old version if it exists */
    realtail = (newtail) ? stringNew(newtail) : stringNew(srctail);
    lept_rm(newdir, realtail);
    makeTempDirname(realnewdir, 256, newdir);
    if (rp->display) {
        lept_stderr("\nInput: srctail = %s, newdir = %s, newtail = %s\n",
                    srctail, newdir, newtail);
        lept_stderr("  realnewdir = %s, realtail = %s\n", realnewdir, realtail);
    }
    sa = getFilenamesInDirectory(realnewdir);
    nfiles1 = sarrayGetCount(sa);
    sarrayDestroy(&sa);

        /* Copy */
    lept_cp(srctail, newdir, newtail, &fname);
    sa = getFilenamesInDirectory(realnewdir);
    nfiles2 = sarrayGetCount(sa);
    if (rp->display) {
        lept_stderr("  File copied to directory: %s\n", realnewdir);
        lept_stderr("  ... with this filename: %s\n", fname);
        lept_stderr("  delta files should be 1: %d\n", nfiles2 - nfiles1);
    }
    regTestCompareValues(rp, 1, nfiles2 - nfiles1, 0.0);  /* '1' */
    sarrayDestroy(&sa);
    lept_free(fname);

        /* Remove it */
    lept_rm(newdir, realtail);
    sa = getFilenamesInDirectory(realnewdir);
    nfiles2 = sarrayGetCount(sa);
    if (rp->display) {
        lept_stderr("  File removed from directory: %s\n", realnewdir);
        lept_stderr("  delta files should be 0: %d\n", nfiles2 - nfiles1);
    }
    regTestCompareValues(rp, 0, nfiles2 - nfiles1, 0.0);  /* '2' */
    sarrayDestroy(&sa);

        /* Copy it again ... */
    lept_cp(srctail, newdir, newtail, &fname);
    if (rp->display)
        lept_stderr("  File copied to: %s\n", fname);
    lept_free(fname);

        /* move it elsewhere ... */
    lept_rmdir("junko");  /* clear out this directory */
    lept_mkdir("junko");
    newsrc = pathJoin(realnewdir, realtail);
    lept_mv(newsrc, "junko", NULL, &fname);
    if (rp->display) {
        lept_stderr("  Move file at: %s\n", newsrc);
        lept_stderr("  ... to: %s\n", fname);
    }
    lept_free(fname);
    lept_free(newsrc);
    makeTempDirname(newnewdir, 256, "junko");
    if (rp->display) lept_stderr("  In this directory: %s\n", newnewdir);
    sa = getFilenamesInDirectory(newnewdir);  /* check if it landed ok */
    nfiles3 = sarrayGetCount(sa);
    if (rp->display) lept_stderr("  num files should be 1: %d\n", nfiles3);
    regTestCompareValues(rp, 1, nfiles3, 0.0);  /* '3' */
    sarrayDestroy(&sa);

        /* and verify it was removed from the original location */
    sa = getFilenamesInDirectory(realnewdir);  /* check if it was removed */
    nfiles2 = sarrayGetCount(sa);
    if (rp->display) {
        lept_stderr("  In this directory: %s\n", realnewdir);
        lept_stderr("  delta files should be 0: %d\n", nfiles2 - nfiles1);
    }
    regTestCompareValues(rp, 0, nfiles2 - nfiles1, 0.0);  /* '4' */
    sarrayDestroy(&sa);
    lept_free(realtail);
}

void TestGenPathname(L_REGPARAMS  *rp,
                     const char   *dir,
                     const char   *fname,
                     const char   *result)
{
char  expect[512], localdir[256];

    char *path = genPathname(dir, fname);
    if (!dir || dir[0] == '\0') {
        if (!getcwd(localdir, sizeof(localdir)))
            lept_stderr("bad bad bad -- no local directory!\n");
        snprintf(expect, sizeof(expect), "%s/%s", localdir, result);
#ifdef _WIN32
        convertSepCharsInPath(expect, UNIX_PATH_SEPCHAR);
#endif  /* _WIN32 */
        regTestCompareStrings(rp, (l_uint8 *)expect, strlen(expect),
                              (l_uint8 *)path, strlen(path));
    } else {
        regTestCompareStrings(rp, (l_uint8 *)result, strlen(result),
                              (l_uint8 *)path, strlen(path));
    }
    if (rp->display) {
        char  *newdir = NULL;
        if (dir && dir[0] == '\0')
            newdir = stringNew("\"\"");
        else if (dir)
            newdir = stringNew(dir);
        lept_stderr("genPathname(%s, %s) --> %s\n", newdir, fname, path);
        lept_free(newdir);
    }
    lept_free(path);
    return;
}


