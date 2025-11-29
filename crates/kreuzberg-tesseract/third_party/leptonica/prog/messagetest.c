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

/* Test the message severity system. */

/* There are three parts:
 * o The first part demonstrates the message severity functionality.
 * o The second part demonstrates a combination of message severity control
 *   and redirect of output to stderr (in this case to dev null).
 * o The third part shows that the naked fprintf() is not affected by the
 *   callback handler, and the default handler is restored with NULL input.
 *
 * Notes on the message severity functionality
 * --------------------------------------------
 *
 * The program prints info, warning, and error messages at the initial
 * run-time severity, which defaults to L_SEVERITY_INFO.  Then it resets the
 * severity to the value specified by an environment variable (or failing
 * that, specified by one of 7 severity control variables) and prints three
 * more info, warning, and error messages.
 *
 * Which messages actually print depend on the compile-time definitions of the
 * MINIMUM_SEVERITY and DEFAULT_SEVERITY identifiers and the run-time
 * definition of the LEPT_MSG_SEVERITY environment variable.  For example:
 *
 *   These commands...               -->  ...print these messages
 *   ==============================       ====================================
 *   $ make
 *
 *   $ ./print                       -->  info, warn, error, info, warn, error
 *   $ LEPT_MSG_SEVERITY=0 ./print  -->  info, warn, error, info, warn, error
 *   $ LEPT_MSG_SEVERITY=5 ./print  -->  info, warn, error,             error
 *   $ LEPT_MSG_SEVERITY=6 ./print  -->  info, warn, error
 *
 *
 *   $ make clean ; make DEFINES='-D DEFAULT_SEVERITY=L_SEVERITY_WARNING'
 *
 *   $ ./print                       -->        warn, error,       warn, error
 *   $ LEPT_MSG_SEVERITY=0 ./print  -->        warn, error, info, warn, error
 *   $ LEPT_MSG_SEVERITY=5 ./print  -->        warn, error,             error
 *   $ LEPT_MSG_SEVERITY=6 ./print  -->        warn, error
 *
 *
 *   $ make clean ; make DEFINES='-D MINIMUM_SEVERITY=L_SEVERITY_WARNING'
 *
 *   $ ./print                       -->        warn, error,       warn, error
 *   $ LEPT_MSG_SEVERITY=0 ./print  -->        warn, error,       warn, error
 *   $ LEPT_MSG_SEVERITY=5 ./print  -->        warn, error,             error
 *   $ LEPT_MSG_SEVERITY=6 ./print  -->        warn, error
 *
 *
 *   $ make clean ; make DEFINES='-D NO_CONSOLE_IO'
 *
 *   $ ./print                       -->  (no messages)
 *   $ LEPT_MSG_SEVERITY=0 ./print  -->  (no messages)
 *   $ LEPT_MSG_SEVERITY=5 ./print  -->  (no messages)
 *   $ LEPT_MSG_SEVERITY=6 ./print  -->  (no messages)
 *
 * Note that in the first and second cases, code is generated to print all six
 * messages, while in the third and fourth cases, code is not generated to
 * print info or all messages, respectively.  This allows the run-time overhead
 * and code space of the print statements to be removed from the library, if
 * desired.
 */

#ifdef HAVE_CONFIG_H
#include <config_auto.h>
#endif  /* HAVE_CONFIG_H */

#include "allheaders.h"

void TestMessageControl(l_int32  severity);
void TestStderrRedirect();

    /* dev null callback for stderr redirect */
static void send_to_devnull(const char *msg) {}

int main ()
{
        /* Part 1: all output to stderr */
    lept_stderr("\nSeverity tests\n");
    TestMessageControl(L_SEVERITY_EXTERNAL);
    TestMessageControl(L_SEVERITY_INFO);
    TestMessageControl(L_SEVERITY_WARNING);
    TestMessageControl(L_SEVERITY_ERROR);
    TestMessageControl(L_SEVERITY_NONE);

        /* Part 2: test combination of severity and redirect */
    lept_stderr("\nRedirect Tests\n\n");
    setMsgSeverity(L_SEVERITY_INFO);
    TestStderrRedirect();
    setMsgSeverity(L_SEVERITY_WARNING);
    TestStderrRedirect();
    setMsgSeverity(L_SEVERITY_ERROR);
    TestStderrRedirect();
    setMsgSeverity(L_SEVERITY_NONE);
    TestStderrRedirect();

        /* Part 3: test of naked fprintf and output with callback handler.
         * All lines should print except for line 4.  */
    lept_stderr("1. text\n");
    lept_stderr("2. text\n");
    leptSetStderrHandler(send_to_devnull);
    lept_stderr("3. text\n");
    lept_stderr("4. text\n");
    leptSetStderrHandler(NULL);
    lept_stderr("5. text\n");
    lept_stderr("6. text\n");

    return 0;
}

void TestMessageControl(l_int32  severity)
{
l_int32  orig_severity;

    setMsgSeverity(DEFAULT_SEVERITY);
    fputc ('\n', stderr);

    /* Print a set of messages with the default setting */
    L_INFO    ("First message\n", "messagetest");
    L_WARNING ("First message\n", "messagetest");
    L_ERROR   ("First message\n", "messagetest");

    /* Set the run-time severity to the value specified by the
       LEPT_MSG_SEVERITY environment variable.  If the variable
       is not defined, set the run-time severity to the input value */
    orig_severity = setMsgSeverity(severity);

    /* Print messages allowed by the new severity setting */
    L_INFO    ("Second message\n", "messagetest");
    L_WARNING ("Second message\n", "messagetest");
    L_ERROR   ("Second message\n", "messagetest");
}

void TestStderrRedirect() {
PIX  *pix1;

        /* Output to stderr works */
    L_INFO("test output 1 to stderr\n", "messagetest");
    L_WARNING("test output 1 to stderr\n", "messagetest");
    L_ERROR("test output 1 to stderr\n", "messagetest");
    pix1 = pixRead("doesn't_exist");
        /* There is no "test output 2" */
    leptSetStderrHandler(send_to_devnull);
    L_INFO("test output 2 to stderr\n", "messagetest");
    L_WARNING("test output 2 to stderr\n", "messagetest");
    L_ERROR("test output 2 to stderr\n", "messagetest");
    pix1 = pixRead("doesn't_exist");
    leptSetStderrHandler(NULL);
        /* Output is restored to stderr */
    L_INFO("test output 3 to stderr\n", "messagetest");
    L_WARNING("test output 3 to stderr\n", "messagetest");
    L_ERROR("test output 3 to stderr\n", "messagetest");
    pix1 = pixRead("doesn't_exist");
    lept_stderr("---------------------------------\n");
}

